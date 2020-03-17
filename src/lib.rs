use lazy_static::lazy_static;
use thiserror::Error;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

pub(crate) mod cast_parser;
pub(crate) mod frame_renderer;
pub(crate) mod sequencer;
pub(crate) mod types;

#[cfg(feature = "cli")]
pub mod cli;

/// A Cast2Gif Error
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),
    #[error("Asciinema error: {0}")]
    AsciinemaError(#[from] cast_parser::AsciinemaError),
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

/// The progress of a cast render job
pub struct CastRenderProgress {
    /// The progress of the terminal frame rasterization
    pub raster_progress: Progress,
    /// The progress of the video sequencing
    pub sequence_progress: Progress,
}

// impl gifski::progress::ProgressReporter for CastRenderProgress {
//     fn
// }

/// The progress of a job
pub struct Progress {
    /// The number that represents "done"
    pub count: u64,
    /// The current progress
    pub progress: u64,
}

/// Convert a asciinema cast file to a gif image
///
/// Provide the asciinema cast file as a reader of the cast file and the image will be output to
/// the writer.
pub fn convert_to_gif_with_progress<R, W: Send, C>(
    reader: R,
    writer: W,
    mut update_progress: C,
) -> Result<(), Error>
where
    R: Read,
    W: Write,
    C: FnMut(&CastRenderProgress) + Send,
{
    // Configure the rayon thread pool
    configure_thread_pool();

    // Create iterator over terminal frames
    let term_frames = cast_parser::TerminalFrameIter::new(reader)?;

    // Create channel for getting rendered frames
    let (sender, receiver) = crossbeam_channel::unbounded();

    // For each frame
    let mut frame_count = 0;
    for frame in term_frames {
        // Unwrap frame result
        let frame = frame?;
        // Increment frame count
        frame_count += 1;
        // Spawn a thread to render the frame
        let s = sender.clone();
        rayon::spawn(move || {
            let frame = frame_renderer::render_frame_to_png(frame);
            if let Err(e) = s.send(frame) {
                log::error!("Could not send frame over channel: {}", e)
            }
        });
    }

    let mut progress = CastRenderProgress {
        raster_progress: Progress {
            count: frame_count,
            progress: 0,
        },
        sequence_progress: Progress {
            count: frame_count,
            progress: 0,
        },
    };

    update_progress(&progress);

    // Drop the unused sender ( to avoid blocking the receiver )
    drop(sender);

    // Collect the frame results
    let mut rendered_frames =
        Vec::with_capacity(frame_count as usize /* TODO: don't use as */);

    while let Ok(frame) = receiver.recv() {
        progress.raster_progress.progress += 1;
        update_progress(&progress);

        rendered_frames.push(frame);
    }

    rayon::scope(move |s| {
        // Create gif encoder
        let (mut collector, gif_writer) = gifski::new(gifski::Settings {
            width: None,
            height: None,
            quality: 100,
            once: false,
            fast: false,
        })
        .expect("TODO");

        // Write gif to file as it is being processed
        s.spawn(|_| {
            gif_writer
                .write(writer, &mut gifski::progress::NoProgress {})
                .expect("TODO");
        });

        for (i, (frame, img)) in rendered_frames.drain(0..).enumerate() {
            collector
                .add_frame_rgba(i, img, frame.time.into())
                .expect("TODO");
            progress.sequence_progress.progress += 1;
            update_progress(&progress);
        }

        drop(collector);
    });

    Ok(())
}

pub fn convert_to_gif<R, W>(reader: R, writer: W) -> Result<(), Error>
where
    R: Read,
    W: Write + Send,
{
    convert_to_gif_with_progress(reader, writer, |_| ())
}
