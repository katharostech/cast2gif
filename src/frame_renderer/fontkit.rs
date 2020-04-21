//! The SVG-based implementation of the frame renderer

use font_kit::{
    canvas::{Canvas, Format, RasterizationOptions},
    hinting::HintingOptions,
    loaders::freetype::Font,
    metrics::Metrics,
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
    static ref FONT_DATA: Arc<Vec<u8>> = Arc::new(Vec::from_iter(
        include_bytes!("./fontkit/Hack-Regular.ttf")
            .iter()
            .map(Clone::clone)
    ));
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
    // TODO: Configurable background color
    const DEFAULT_BG_COLOR: RGBA8 = RGBA::new(0, 0, 0, 255);

    // Glyph rendering config
    lazy_static! {
        // static ref TRANS: Transform2F = Transform2F::default();
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
                Transform2F::default(),
                *HINTING_OPTS,
                *RASTER_OPTS,
            )
        })
        .expect("TODO");
    let font_width = raster_rect.width();
    let font_height = ((FONT_METRICS.ascent - FONT_METRICS.descent)
        / FONT_METRICS.units_per_em as f32
        * font_size)
        .ceil() as i32;
    let font_height_offset = (font_height - raster_rect.height()) / 2;
    let font_transform = Transform2F::from_translation(Vector2F::new(0., -font_height_offset as f32));
    
    let height = (rows as i32 * font_height) as usize;
    let width = (cols as i32 * font_width) as usize;

    // Image to render to
    let pixel_count = width * height;
    let mut pixels: Vec<RGBA8> = Vec::with_capacity(pixel_count);
    for _ in 0..pixel_count {
        pixels.push(DEFAULT_BG_COLOR);
    }
    let mut image: ImgVec<RGBA8> = Img::new(pixels, width, height);
    // TODO: Render cursor position
    let _cursor_position = frame.screen.cursor_position();

    flame!(end "Init Values");

    flame!(start "Render Cells");
    for row in 0..rows {
        for col in 0..cols {
            let cell = frame.screen.cell(row, col).expect("Error indexing cell");
            let ypos = row as i32 * font_height;
            let xpos = col as i32 * font_width;
            let mut subimg = image.sub_image_mut(
                xpos as usize,
                ypos as usize,
                font_width as usize,
                font_height as usize,
            );

            // Fill background color
            let background_color;
            if let Some((r, g, b)) = parse_color(cell.bgcolor()) {
                background_color = RGBA8::new(r, g, b, 255);
                for pixel in subimg.pixels_mut() {
                    *pixel = background_color;
                }
            } else {
                background_color = DEFAULT_BG_COLOR;
            }

            if cell.has_contents() {
                use palette::{Blend, Pixel, LinSrgba};
                let mut canvas = Canvas::new(Vector2I::new(font_width, font_height), *FORMAT);
                let cell_char: char = cell.contents().parse().expect("Invalid char in cell");
                let glyph_id = FONT
                    .with(|f| f.glyph_for_char(cell_char))
                    .expect("Could not find glyph for char");

                FONT.with(|f| {
                    f.rasterize_glyph(
                        &mut canvas,
                        glyph_id,
                        font_size as f32,
                        Transform2F::from_translation(-raster_rect.origin().to_f32()) * font_transform,
                        *HINTING_OPTS,
                        *RASTER_OPTS,
                    )
                })
                .expect("TODO");

                let (r, g, b) = if let Some((r, g, b)) = parse_color(cell.fgcolor()) {
                    (r, g, b)
                } else {
                    (255, 255, 255)
                };

                // Alpha `a` over `b`: component wize: a + b * (255 - alpha)
                for y in 0..font_height {
                    let (row_start, row_end) =
                        (y as usize * canvas.stride, (y + 1) as usize * canvas.stride);
                    let row = &canvas.pixels[row_start..row_end];
                    for x in 0..font_width {
                        let alpha = row[x as usize];
                        let bg: LinSrgba<f32> = LinSrgba::from_raw(&[
                            background_color.r,
                            background_color.g,
                            background_color.b,
                            255,
                        ]).into_format();
                        let fg: LinSrgba<f32> = LinSrgba::from_raw(&[r, g, b, alpha]).into_format();
                        let out: [u8; 4] = fg.over(bg).into_format().into_raw();
                        subimg[(x as usize, y as usize)] = RGBA8::new(out[0], out[1], out[2], 255);
                    }
                }
            }
        }
    }
    flame!(end "Render Cells");

    flame!(start "Create Image");
    // for y in 0..height {
    //     // flame!(guard "Write Pixel");
    //     let (row_start, row_end) = (y as usize * canvas.stride, (y + 1) as usize * canvas.stride);
    //     let row = &canvas.pixels[row_start..row_end];
    //     for x in 0..width {
    //         let a = row[x as usize];
    //         image[(x, y)] = RGBA8::new(a, a, a, 255);
    //     }
    // }
    flame!(end "Create Image");

    #[cfg(feature = "flamegraph")]
    flame::dump_html(
        &mut std::fs::File::create("fontkitrender-flamegraph.gitignore.html").unwrap(),
    )
    .unwrap();

    RgbaFrame {
        time: frame.time,
        index: frame.index,
        image,
    }
}
