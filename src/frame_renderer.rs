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

/// Return (r, g b) u8 tuple formatted version of a terminal color
///
/// Returns `None` if it is the default color
fn parse_color(color: vt100::Color) -> Option<(u8, u8, u8)> {
    use vt100::Color;
    match color {
        Color::Default => None,
        // pallet source: http://chriskempson.com/projects/base16/
        // TODO: Custom pallets
        Color::Idx(i) => match i {
            0 => Some((24, 24, 24)),
            1 => Some((171, 70, 66)),
            2 => Some((161, 181, 108)),
            3 => Some((247, 202, 136)),
            4 => Some((124, 175, 194)),
            5 => Some((186, 139, 175)),
            6 => Some((134, 193, 185)),
            7 => Some((216, 216, 216)),
            8 => Some((88, 88, 88)),
            9 => Some((171, 70, 66)),
            10 => Some((161, 181, 108)),
            11 => Some((247, 202, 136)),
            12 => Some((124, 175, 194)),
            13 => Some((186, 139, 175)),
            14 => Some((134, 193, 185)),
            15 => Some((248, 248, 248)),
            other => Some(ansi_colours::rgb_from_ansi256(other)),
        },
        Color::Rgb(r, g, b) => Some((r, g, b)),
    }
}
