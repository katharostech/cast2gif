use gif::SetParameter;
use lazy_static::lazy_static;
use thiserror::Error;

use rgb::ComponentBytes;
use std::io::{Read, Write};
use std::{
    convert::TryInto,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

#[macro_use]
pub(crate) mod macros;
pub(crate) mod cast_parser;
pub(crate) mod frame_renderer;
pub(crate) mod types;

use cast_parser::AsciinemaError;
pub use types::*;

#[cfg(feature = "cli")]
pub mod cli;

/// A Cast2Gif Error
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),
    #[error("Asciinema error: {0}")]
    AsciinemaError(#[from] AsciinemaError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Image error: {0}")]
    ImageError(#[from] ImageError),
}

/// An error with the image
#[derive(Error, Debug)]
pub enum ImageError {
    #[error("Invalid image {0}, could not convert to unsigned 32-bit integer: {1}")]
    InvalidDimension(ImageDimension, usize),
}

#[derive(Debug)]
pub enum ImageDimension {
    Width,
    Height,
}

impl std::fmt::Display for ImageDimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageDimension::Width => write!(f, "width"),
            ImageDimension::Height => write!(f, "height"),
        }
    }
}

lazy_static! {
    /// Indicates whether or not we have initialize the rayon thread pool yet.
    static ref THREAD_POOL_INITIALIZED: AtomicBool = AtomicBool::new(false);
}

/// Initialize the rayon thread pool
fn configure_thread_pool() {
    // If the thread pool is uninitialized
    if !THREAD_POOL_INITIALIZED.load(SeqCst) {
        // Configure and build the global rayon thread pool
        rayon::ThreadPoolBuilder::new()
            // Configure the panic handler
            .panic_handler(|_| {
                log::error!(concat!(
                    "A worker thread has crashed. This is a bug. Please report this on the our ",
                    "issue tracker\n\n",
                    "    https://github.com/katharostech/cast2gif/issues"
                ));
            })
            .build_global()
            .expect("Rayon pool can only be initialized once");

        // Indicate thread pool is initialized
        THREAD_POOL_INITIALIZED.store(false, SeqCst);
    }
}

fn progress_thread<C: CastProgressHandler>(
    progress_reciever: flume::Receiver<ProgressCmd>,
    mut progress_handler: C,
) {
    // Setup initial progress
    let mut progress = CastRenderProgress::default();

    // Handle incomming commands
    for cmd in progress_reciever {
        match cmd {
            ProgressCmd::IncrementCount => progress.count += 1,
            ProgressCmd::IncrementRasterProgress => progress.raster_progress += 1,
            ProgressCmd::IncrementSequenceProgress => progress.sequence_progress += 1,
        }
        progress_handler.update_progress(&progress);
    }
}

fn png_raster_thread<Fi>(
    frames: Fi,
    progress_sender: flume::Sender<ProgressCmd>,
    frame_sender: flume::Sender<RgbaFrame>,
) where
    Fi: IntoIterator<Item = Result<TerminalFrame, AsciinemaError>>,
{
    // For each frame
    for frame in frames {
        // Unwrap frame result
        let frame = frame.expect("TODO");

        // Increment frame count
        progress_sender
            .send(ProgressCmd::IncrementCount)
            .expect("TODO");

        // Spawn a thread to render the frame
        let fs = frame_sender.clone();
        let ps = progress_sender.clone();
        rayon::spawn(move || {
            let frame = frame_renderer::render_frame_to_png(frame);
            fs.send(frame).expect("TODO");
            ps.send(ProgressCmd::IncrementRasterProgress).expect("TODO");
        });
    }
}

/// An iterator over an iterator of frames that makes sure the frames come in the right order
struct OrderedFrameIter<I: Iterator<Item = RgbaFrame>> {
    buffer: Vec<RgbaFrame>,
    frames: I,
    current_frame: u64,
}

impl<I: Iterator<Item = RgbaFrame>> OrderedFrameIter<I> {
    fn new(frame_iter: I) -> Self {
        Self {
            buffer: Vec::new(),
            frames: frame_iter,
            current_frame: 0,
        }
    }
}

impl<I: Iterator<Item = RgbaFrame>> std::iter::Iterator for OrderedFrameIter<I> {
    type Item = RgbaFrame;

    fn next(&mut self) -> Option<Self::Item> {
        // See if any of the frames in the buffer are the next frame
        let mut next_frame_buffer_index = None;
        for (index, frame) in self.buffer.iter().enumerate() {
            // If this frame is the next frame in the list
            if frame.index == self.current_frame {
                // Record its inex
                next_frame_buffer_index = Some(index);
                // And exit the loop
                break;
            }
        }

        // Grab the next frame out of the buffer if one was found
        let next_frame = next_frame_buffer_index.map(|i| self.buffer.remove(i));

        // If we have found a frame in the buffer
        let ret = if let Some(frame) = next_frame {
            // Return the frame
            Some(frame)

        // If we don't have a buffered frame
        } else {
            // Loop through the next frames until we find the next one
            loop {
                if let Some(frame) = self.frames.next() {
                    // If this is the next frame
                    if frame.index == self.current_frame {
                        // Return it
                        break Some(frame);
                    } else {
                        // Push it to the buffer
                        self.buffer.push(frame);
                    }
                } else {
                    break None;
                }
            }
        };

        self.current_frame += 1;
        ret
    }
}

fn sequence_gif<W: Write>(
    frame_receiver: flume::Receiver<RgbaFrame>,
    progress_sender: flume::Sender<ProgressCmd>,
    file_writer: W,
) -> Result<(), Error> {
    // Get the first frame so we have a reference for the image height and width
    let first_frame = frame_receiver
        .recv()
        .expect("TODO: Got a gif with no frames?");

    // Get width and height for the image
    let width = first_frame.image.width();
    let height = first_frame.image.height();

    let try_to_u16 = |x: usize, dim| {
        x.try_into()
            .map_err(|_| ImageError::InvalidDimension(dim, x))
    };

    use ImageDimension::{Height, Width};

    // Create the gif encoder
    let mut encoder = gif::Encoder::new(
        file_writer,
        try_to_u16(width, Width)?,
        try_to_u16(height, Height)?,
        &[],
    )?;

    encoder.set(gif::Repeat::Infinite)?;


    // Loop through the frames
    let mut last_frame_time = 0f32;
    for frame in OrderedFrameIter::new(std::iter::once(first_frame).chain(frame_receiver)) {
        // Send sequence progress
        progress_sender
            .send(ProgressCmd::IncrementSequenceProgress)
            .ok();

        let (mut data, width, height) = frame.image.into_contiguous_buf();
        let pixels = data.as_bytes_mut();

        let mut gif_frame = gif::Frame::from_rgba_speed(
            try_to_u16(width, Width)?,
            try_to_u16(height, Height)?,
            pixels,
            30,
        );

        let dt = frame.time - last_frame_time;

        // if dt < 1. {
        //     continue;
        // }

        gif_frame.delay = (dt / 10.).round() as u16;

        last_frame_time = frame.time;

        // Add frame to gif
        encoder.write_frame(&gif_frame)?;
    }

    Ok(())
}

/// Convert a asciinema cast file to a gif image
///
/// Provide the asciinema cast file as a reader of the cast file and the image will be output to
/// the writer.
pub fn convert_to_gif_with_progress<R, W, C>(
    reader: R,
    writer: W,
    update_progress: C,
) -> Result<(), Error>
where
    R: Read + Send + 'static,
    W: Write + Send,
    C: CastProgressHandler + 'static,
{
    // Configure the rayon thread pool
    configure_thread_pool();

    // Create the progress thread and channel
    let (progress_sender, progress_receiver) = flume::unbounded();
    rayon::spawn(move || progress_thread(progress_receiver, update_progress));

    // Create channel for getting rendered frames
    let (raster_sender, raster_receiver) = flume::unbounded();

    // Create iterator over terminal frames
    let term_frames = cast_parser::TerminalFrameIter::new(reader).expect("TODO");

    // Spawn the png rasterizer thread
    let ps = progress_sender.clone();
    rayon::spawn(move || png_raster_thread(term_frames, ps, raster_sender));

    // Buffered writer
    let buf = std::io::BufWriter::new(writer);

    // Start sequencing the gif
    sequence_gif(raster_receiver, progress_sender, buf)?;

    Ok(())
}

pub fn convert_to_gif<R, W>(reader: R, writer: W) -> Result<(), Error>
where
    R: Read + Send + 'static,
    W: Write + Send,
{
    convert_to_gif_with_progress(reader, writer, NullProgressHandler)
}
