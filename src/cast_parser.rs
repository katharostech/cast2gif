//! The asciinema cast file parser
//!
//! This module contains the code that parses the asciinema cast into a set of terminal screen
//! states.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::iter::Iterator;

use crate::types::TerminalFrame;

/// An asciinema error
#[derive(Error, Debug)]
pub enum AsciinemaError {
    #[error("Could not parse Asciinema cast: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Could not parse Asciinema cast: {0}")]
    GenericParserError(String),
    #[error("IO Error while parsing Asciinema cast: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Only asciinema file version 2 is supported, got version: {0}")]
    InvalidVersion(u16),
}

/// An asciinema cast
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AsciinemaCast {
    /// Asciinema file metadata
    metadata: AsciinemaCastMeta,
    /// Asciinema frames
    frames: Vec<AsciinemaFrame>,
}

/// Asciinema cast file metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AsciinemaCastMeta {
    version: u16,
    width: u16,
    height: u16,
    timestamp: i32,
    env: HashMap<String, String>,
}

/// A frame from the asciinema recording
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AsciinemaFrame {
    time: f32,
    /// TODO: not actually sure what this field is for
    command: String,
    output: String,
}

/// A frame from the asciinema recording. This has unnamed fields to be compatible with the actual
/// JSON representation of the frame.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AsciinemaFrameRaw(f32, String, String);

/// An iterator over terminal frames in a asciinema cast file reader
///
/// Each item in the iterator represents the state of the screen at that frame in the asciinema
/// cast.
pub(crate) struct TerminalFrameIter<R: Read> {
    parser: vt100::Parser,
    lines: std::io::Lines<BufReader<R>>,
}

impl<R: Read> TerminalFrameIter<R> {
    pub fn new(reader: R) -> Result<Self, AsciinemaError> {
        // Buffer read
        let buf_reader = BufReader::new(reader);
        // Split file by lines
        let mut lines = buf_reader.lines();

        let metadata_line = lines.next().ok_or_else(|| {
            AsciinemaError::GenericParserError("Missing cast metadata line".into())
        })??;

        // Parse metadata
        let metadata: AsciinemaCastMeta = serde_json::from_str(&metadata_line)?;

        // Validate metadata version
        if metadata.version != 2 {
            return Err(AsciinemaError::InvalidVersion(metadata.version));
        }

        // Create iterator
        Ok(TerminalFrameIter {
            parser: vt100::Parser::new(metadata.height, metadata.width, 0 /* scrollback */),
            lines,
        })
    }
}

impl<R: Read> Iterator for TerminalFrameIter<R> {
    type Item = Result<TerminalFrame, AsciinemaError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Get the next line from our reader
            let line = self.lines.next();

            // If there is another line
            if let Some(line) = line {
                let line = match line {
                    // Extract line if OK
                    Ok(l) => l,
                    // Return IO error
                    Err(e) => break Some(Err(AsciinemaError::IoError(e))),
                };

                // Skip this line if it is empty
                if line == "" {
                    continue;
                }

                // Parse raw frame
                let frame: AsciinemaFrameRaw = match serde_json::from_str(&line) {
                    // Extract frame
                    Ok(frame) => frame,
                    // Return parser error
                    Err(_) => {
                        break Some(Err(AsciinemaError::GenericParserError(format!(
                            "Error parsing asciinema frame: {}",
                            line
                        ))))
                    }
                };

                // Restructucuture frame for readability
                let frame = AsciinemaFrame {
                    time: frame.0,
                    command: frame.1,
                    output: frame.2,
                };

                // TODO: I don't know what other items might be in the second item of the record array,
                // but so far I've only seen "o".
                if frame.command != "o" {
                    let error_message = format!(
                        "Cast2Gif doesn't yet understand asciinema files with \
                        something other than `o` in the second item of the record \
                        array. Please open an issue for this: {}",
                        line
                    );
                    return Some(Err(AsciinemaError::GenericParserError(error_message)));
                }

                // Process the terminal input
                self.parser.process(frame.output.as_bytes());

                // Return the next terminal frame
                break Some(Ok(TerminalFrame {
                    time: frame.time,
                    screen: self.parser.screen().clone(),
                }));

            // If there isn't another line
            } else {
                break None;
            }
        }
    }
}
