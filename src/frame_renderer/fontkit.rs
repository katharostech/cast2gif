//! The SVG-based implementation of the frame renderer

use font_kit::{
    canvas::{Canvas, Format, RasterizationOptions},
    hinting::HintingOptions,
    loaders::freetype::Font,
    metrics::Metrics
};
use imgref::{Img, ImgVec};
use lazy_static::lazy_static;
use pathfinder_geometry::{
    transform2d::Transform2F,
    vector::{Vector2F, Vector2I},
};
use rgb::{RGBA, RGBA8};

use std::iter::FromIterator;
use std::sync::Arc;

use super::parse_color;
use crate::types::*;

lazy_static! {
    static ref FONT_DATA: Arc<Vec<u8>> = Arc::new(
        Vec::from_iter(
            include_bytes!("./fontkit/Hack-Regular.ttf")
            .iter()
            .map(Clone::clone)
        )
    );
    static ref FONT_METRICS: Metrics = FONT.with(|f| f.metrics());
}

thread_local! {
    // TODO clone the arc instead of cloning the iterator every time
    static FONT: Font = Font::from_bytes(FONT_DATA.clone(), 0).expect("Could not load font");
}

pub(crate) fn render_frame_to_png(frame: TerminalFrame) -> RgbaFrame {
    flame!(guard "Render Frame To PNG");

    flame!(start "Init Values");
    let font_size = 13f32; // TODO make configurable font size
    let (rows, cols) = frame.screen.size();
    let background_color: RGBA8 = RGBA::new(0, 0, 0, 255);

    // Glyph rendering config
    lazy_static! {
        static ref TRANS: Transform2F = Transform2F::default();
        // TODO check hinting settings ( None might be faster with no difference in rendering )
        static ref HINTING_OPTS: HintingOptions = HintingOptions::Vertical(5.);
        static ref FORMAT: Format = Format::A8;
        static ref RASTER_OPTS: RasterizationOptions = RasterizationOptions::GrayscaleAa;
    }

    // Get font height and width
    let raster_rect = FONT
        .with(|f| {
            f.raster_bounds(
                f.glyph_for_char('A').expect("TODO"),
                font_size,
                *TRANS,
                *HINTING_OPTS,
                *RASTER_OPTS,
            )
        })
        .expect("TODO");
    let font_width = raster_rect.width();
    let font_height = ((FONT_METRICS.ascent - FONT_METRICS.descent) / FONT_METRICS.units_per_em as f32 * font_size).ceil() as i32;
    let height = (rows as i32 * font_height) as usize;
    let width = (cols as i32 * font_width) as usize;

    // Image to render to
    let pixel_count = width * height;
    let mut pixels: Vec<RGBA8> = Vec::with_capacity(pixel_count);
    for _ in 0..pixel_count {
        pixels.push(background_color);
    }
    let mut image: ImgVec<RGBA8> = Img::new(pixels, width, height);
    let mut canvas = Canvas::new(Vector2I::new(width as _, height as _), *FORMAT);
    let _cursor_position = frame.screen.cursor_position();

    flame!(end "Init Values");

    flame!(start "Render Cells");
    for row in 0..rows {
        for col in 0..cols {
            let cell = frame.screen.cell(row, col).expect("Error indexing cell");
            let ypos = row as i32 * font_height + raster_rect.height() / 2;
            let xpos = col as i32 * font_width;

            if cell.has_contents() {
                let cell_char: char = cell.contents().parse().expect("Invalid char in cell");
                let glyph_id = FONT
                    .with(|f| f.glyph_for_char(cell_char))
                    .expect("Could not find glyph for char");

                FONT.with(|f| {
                    f.rasterize_glyph(
                        &mut canvas,
                        glyph_id,
                        font_size as f32,
                        Transform2F::from_translation(Vector2F::new(xpos as f32, ypos as f32)),
                        *HINTING_OPTS,
                        *RASTER_OPTS,
                    )
                })
                .expect("TODO");
            }
        }
    }
    flame!(end "Render Cells");

    flame!(start "Create Image");
    for y in 0..height {
        // flame!(guard "Write Pixel");
        let (row_start, row_end) = (y as usize * canvas.stride, (y + 1) as usize * canvas.stride);
        let row = &canvas.pixels[row_start..row_end];
        for x in 0..width {
            let a = row[x as usize];
            image[(x, y)] = RGBA8::new(a, a, a, 255);
        }
    }
    flame!(end "Create Image");

    
    #[cfg(feature = "flamegraph")]
    flame::dump_html(&mut std::fs::File::create("fontkitrender-flamegraph.gitignore.html").unwrap()).unwrap();

    RgbaFrame {
        time: frame.time,
        index: frame.index,
        image,
    }
}
