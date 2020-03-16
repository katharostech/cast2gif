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
    #[error("{0}")]
    AsciinemaError(#[from] cast_parser::AsciinemaError),
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

pub fn convert_to_gif<R: Read, W: Write>(reader: R, writer: W) -> Result<(), Error> {
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
            let frame = frame_renderer::render_frame_to_png(&frame);
            if let Err(e) = s.send(frame) {
                log::error!("Could not send frame over channel: {}", e)
            }
        });
        
        // Fake
        // break;
    }

    // Drop the unused sender ( to avoid blocking the receiver )
    drop(sender);

    // Collect the frame results
    let mut rendered_frames = Vec::with_capacity(frame_count);
    while let Ok(frame) = receiver.recv() {
        rendered_frames.push(frame);
    }

    dbg!(rendered_frames);

    Ok(())
}
