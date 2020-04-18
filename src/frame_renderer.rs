//! Terminal frame renderer
//!
//! This module contains the functions that take a terminal frame and create a rendered image
//! of the terminal at that frame.

#[cfg(not(any(feature = "backend-svg", feature = "backend-fontkit")))]
compile_error!("You must specify either the `backend-svg` or `backend-fontkit` features");

#[cfg(feature = "backend-svg")]
mod svg;
#[cfg(feature = "backend-svg")]
pub(crate) use self::svg::render_frame_to_png;

#[cfg(feature = "backend-fontkit")]
mod fontkit;
#[cfg(feature = "backend-fontkit")]
pub(crate) use fontkit::render_frame_to_png;

/// Return hex formatted version of a terminal color
///
/// Returns `None` if it is the default color
fn parse_color(color: vt100::Color) -> Option<String> {
    use vt100::Color;
    match color {
        Color::Default => None,
        // pallet source: http://chriskempson.com/projects/base16/
        // TODO: Custom pallets
        Color::Idx(i) => match i {
            0 => Some("#181818".into()),
            1 => Some("#ab4642".into()),
            2 => Some("#a1b56c".into()),
            3 => Some("#f7ca88".into()),
            4 => Some("#7cafc2".into()),
            5 => Some("#ba8baf".into()),
            6 => Some("#86c1b9".into()),
            7 => Some("#d8d8d8".into()),
            8 => Some("#585858".into()),
            9 => Some("#ab4642".into()),
            10 => Some("#a1b56C".into()),
            11 => Some("#f7ca88".into()),
            12 => Some("#7cafc2".into()),
            13 => Some("#ba8baf".into()),
            14 => Some("#86c1b9".into()),
            15 => Some("#f8f8f8".into()),
            other => {
                let (r, g, b) = ansi_colours::rgb_from_ansi256(other);
                Some(format!("#{}", base16::encode_lower(&[r, g, b])))
            }
        },
        Color::Rgb(r, g, b) => Some(format!("#{}", base16::encode_lower(&[r, g, b]))),
    }
}
