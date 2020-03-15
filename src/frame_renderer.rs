use crossbeam_channel::Sender;

use crate::types::TerminalFrame;

pub(crate) fn render_frame(frame: TerminalFrame, sender: Sender<f32>) {
    if let Err(e) = sender.send(frame.time) {
        log::error!("Could not send value over channel: {}", e);
    };
}
