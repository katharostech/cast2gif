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
    /// The index
    next_index: u64,
    /// The interval between frames
    interval: f32,
    /// The time stamp of the last frame
    last_frame_time: f32,
    /// If we have determined that we need to render some extra frames, we need to serve these
    /// first instead of the true next frame in the animation.
    next_frames: Vec<TerminalFrame>,
    /// The parser instance used to emulate the terminal
    parser: vt100::Parser,
    /// The buffered line reader over the Asciinema recording file
    lines: std::io::Lines<BufReader<R>>,
}

impl<R: Read> TerminalFrameIter<R> {
    pub fn new(reader: R, interval: f32) -> Result<Self, AsciinemaError> {
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
            next_index: 0,
            last_frame_time: 0.0,
            interval,
            parser: vt100::Parser::new(metadata.height, metadata.width, 0 /* scrollback */),
            next_frames: vec![],
            lines,
        })
    }
}

impl<R: Read> Iterator for TerminalFrameIter<R> {
    type Item = Result<TerminalFrame, AsciinemaError>;

    fn next(&mut self) -> Option<Self::Item> {
        // If there is a next frame already cached
        if let Some(next_frame) = self.next_frames.pop() {
            // Increment next index
            self.next_index += 1;

            // Return that frame instead
            return Some(Ok(next_frame));
        }

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

                // Get the current frame index and increment the next frame index
                let current_index = self.next_index;
                self.next_index += 1;
                // Get the diff between the frames time and the last frame time
                let frame_time_diff = frame.time - self.last_frame_time;

                // If the difference between this frame and the last frame is greater than the
                // interval
                if frame_time_diff >= self.interval {
                    // Keep this frame and set this as the last frame time
                    self.last_frame_time = frame.time;

                    let mut filler_frame = None;
                    // For every interval's time that this frame time is greater than the last frame
                    // we need to add a filler duplicate frame, to keep the frame rate consistant.
                    for i in 0..((frame_time_diff / self.interval).floor() as i32) {
                        // The first frame we store so that we can render that next
                        if i == 0 {
                            filler_frame = Some(TerminalFrame {
                                index: current_index,
                                time: frame.time,
                                screen: self.parser.screen().clone(),
                            });
                        // For the other filler frames, we add them to the upcomming frame list
                        } else {
                            self.next_frames.push(TerminalFrame {
                                index: current_index + i as u64,
                                time: frame.time + i as f32 * self.interval,
                                screen: self.parser.screen().clone(),
                            });
                        }
                    }

                    // If there is a filler frame, render that one instead
                    if let Some(filler_frame) = filler_frame {
                        break Some(Ok(filler_frame));
                    }

                // If it has not been greater than the interval
                } else {
                    // Discard this frame and grab the next one
                    continue;
                }

                break Some(Ok(TerminalFrame {
                    index: current_index,
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
