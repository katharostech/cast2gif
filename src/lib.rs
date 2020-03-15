use thiserror::Error;

use std::io::{Read, Write};

pub(crate) mod cast_parser;
pub(crate) mod frame_renderer;
pub(crate) mod types;

#[cfg(feature = "cli")]
pub mod cli;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),
    #[error("{0}")]
    AsciinemaError(#[from] cast_parser::AsciinemaError),
}

pub fn convert_to_gif<R: Read, W: Write>(reader: R, writer: W) -> Result<(), Error> {
    // Create iterator over terminal frames
    let frames = cast_parser::TerminalFrameIter::new(reader)?;

    // Create channel for getting rendered frames
    let (sender, receiver) = crossbeam_channel::unbounded();

    // For each frame
    for frame in frames {
        let frame = frame?;

        // Spawn a thread to render the frame
        let s = sender.clone();
        rayon::spawn(move || {
            frame_renderer::render_frame(frame, s);
        });
    }

    // Drop the unused sender
    drop(sender);

    while let Ok(data) = receiver.recv() {
        dbg!(data);
    }

    Ok(())
}
