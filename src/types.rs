#[derive(Debug, Clone)]
pub(crate) struct TerminalFrame {
    pub time: f32,
    pub screen: vt100::Screen,
}
