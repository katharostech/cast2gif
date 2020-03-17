use imgref::ImgVec;
use rgb::RGBA8;

/// A terminal frame
#[derive(Debug, Clone)]
pub(crate) struct TerminalFrame {
    /// The index of the frame in the animation
    pub index: u64,
    /// The time the frame occurrs in the animation timeline
    pub time: f32,
    /// The terminal screen state at this frame
    pub screen: vt100::Screen,
}

/// An SVG render of a terminal frame
#[derive(Debug, Clone)]
pub(crate) struct SvgFrame {
    /// The index of the frame in the animation
    pub index: u64,
    /// The time the frame occurrs in the animation timeline
    pub time: f32,
    /// The SVG for the frame
    pub doc: svg::Document,
    /// The height of the SVG document in pixels
    pub height: u16,
    /// The width of the SVG document in pixels
    pub width: u16,
}

/// An SVG render of a terminal frame
#[derive(Debug, Clone)]
pub(crate) struct RgbaFrame {
    /// The index of the frame in the animation
    pub index: u64,
    /// The time the frame occurrs in the animation timeline
    pub time: f32,
    /// The RGBA image for the frame
    pub image: ImgVec<RGBA8>,
}

/// The progress of a cast render job
#[derive(Default, Debug, Clone)]
pub struct CastRenderProgress {
    /// The total number of frames to render
    pub count: u64,
    /// The progress of the terminal frame rasterization
    pub raster_progress: u64,
    /// The progress of the video sequencing
    pub sequence_progress: u64,
}

/// This types is used as a "command" to the progress thread to increment the progress
#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum ProgressCmd {
    IncrementCount,
    IncrementRasterProgress,
    IncrementSequenceProgress,
}

/// The trait for a progress handler
pub trait CastProgressHandler: Send {
    fn update_progress(&mut self, progress: &CastRenderProgress);
}

pub struct NullProgressHandler;

impl CastProgressHandler for NullProgressHandler {
    fn update_progress(&mut self, _progress: &CastRenderProgress) {}
}
