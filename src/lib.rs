use lazy_static::lazy_static;
use thiserror::Error;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

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
    #[error("Gif error: {0}")]
    GifError(#[from] gifski::Error),
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

fn gif_sequencer_thread(
    frame_receiver: flume::Receiver<RgbaFrame>,
    mut gif_collector: gifski::Collector,
) {
    for frame in frame_receiver {
        // Add frame to gif
        gif_collector
            // TODO: avoid `as`
            .add_frame_rgba(
                frame.index as usize,
                frame.image,
                (frame.time * 0.01) as f64,
            )
            .expect("TODO");
    }
}

/// Convert a asciinema cast file to a gif image
///
/// Provide the asciinema cast file as a reader of the cast file and the image will be output to
/// the writer.
pub fn convert_to_gif_with_progress<R, W, C>(
    reader: R,
    writer: W,
    interval: f32,
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
    let term_frames = cast_parser::TerminalFrameIter::new(reader, interval).expect("TODO");

    // Spawn the png rasterizer thread
    let ps = progress_sender.clone();
    rayon::spawn(move || png_raster_thread(term_frames, ps, raster_sender));

    // Create gifski gif encoder
    let (collector, gif_writer) = gifski::new(gifski::Settings {
        width: None,
        height: None,
        quality: 100,
        once: false,
        fast: false,
    })
    .expect("TODO");

    // Spawn the gif sequencer thread
    // NOTE: Even though we are handing the rasterized images to the gif collector
    // in a separate thread, the gif *writer* seems to write sequentially. Also because
    // Our frame index doesn't start at zero ( kind of a bug? ) it waits until all of the
    // frames have been set before sequencing. In practice this is not actually an issue
    // because we pretty much saturate the CPU while rasterizing anyway and it isn't faster
    // to try to sequence at the same time anyway.
    rayon::spawn(move || gif_sequencer_thread(raster_receiver, collector));

    // Write out the recieved gif
    let buf = std::io::BufWriter::new(writer);
    let mut progress_handler = GifWriterProgressHandler::new(progress_sender);
    gif_writer.write(buf, &mut progress_handler).expect("TODO");

    Ok(())
}

struct GifWriterProgressHandler {
    progress_sender: flume::Sender<ProgressCmd>,
}

impl GifWriterProgressHandler {
    fn new(progress_sender: flume::Sender<ProgressCmd>) -> Self {
        Self { progress_sender }
    }
}

impl gifski::progress::ProgressReporter for GifWriterProgressHandler {
    fn increase(&mut self) -> bool {
        self.progress_sender
            .send(ProgressCmd::IncrementSequenceProgress)
            .expect("TODO");

        true
    }

    fn done(&mut self, _msg: &str) {}
}

pub fn convert_to_gif<R, W>(reader: R, writer: W, interval: f32) -> Result<(), Error>
where
    R: Read + Send + 'static,
    W: Write + Send,
{
    convert_to_gif_with_progress(reader, writer, interval, NullProgressHandler)
}
