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
    let font_transform =
        Transform2F::from_translation(Vector2F::new(0., -font_height_offset as f32));

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

            let cell_bg_color = parse_color(cell.bgcolor())
                .map(|x| RGBA::new(x.0, x.1, x.2, 255))
                .unwrap_or(DEFAULT_BG_COLOR);
            let cell_fg_color = parse_color(cell.fgcolor())
                .map(|x| RGBA::new(x.0, x.1, x.2, 255))
                .unwrap_or(RGBA::new(255, 255, 255, 255));

            let real_bg_color;
            let real_fg_color;
            if frame.screen.cursor_position() == (row, col) {
                real_fg_color = cell_bg_color;
                real_bg_color = cell_fg_color;
            } else {
                real_bg_color = cell_bg_color;
                real_fg_color = cell_fg_color;
            }

            if real_bg_color != DEFAULT_BG_COLOR {
                for pixel in subimg.pixels_mut() {
                    *pixel = real_bg_color;
                }
            }

            if cell.has_contents() {
                use palette::{Blend, LinSrgba, Pixel};
                let mut canvas = Canvas::new(Vector2I::new(font_width, font_height), *FORMAT);
                let contents = cell.contents();
                if contents == "" {
                    break;
                }
                let cell_char: char = contents.parse().expect("Could not parse char");

                // TODO: We currently use `.` as a fallback char, but we should use a better one and maybe pick a
                // font that supports all the characters used in the TUI-rs demo.
                let glyph_id = FONT.with(|f| {
                    f.glyph_for_char(cell_char)
                        .unwrap_or_else(|| f.glyph_for_char('.').expect("TODO"))
                });

                FONT.with(|f| {
                    f.rasterize_glyph(
                        &mut canvas,
                        glyph_id,
                        font_size as f32,
                        Transform2F::from_translation(-raster_rect.origin().to_f32())
                            * font_transform,
                        *HINTING_OPTS,
                        *RASTER_OPTS,
                    )
                })
                .expect("TODO");

                // Alpha `a` over `b`: component wize: a + b * (255 - alpha)
                for y in 0..font_height {
                    let (row_start, row_end) =
                        (y as usize * canvas.stride, (y + 1) as usize * canvas.stride);
                    let row = &canvas.pixels[row_start..row_end];
                    for x in 0..font_width {
                        let alpha = row[x as usize];
                        let bg: LinSrgba<f32> = LinSrgba::from_raw(&[
                            real_bg_color.r,
                            real_bg_color.g,
                            real_bg_color.b,
                            255,
                        ])
                        .into_format();
                        let fg: LinSrgba<f32> = LinSrgba::from_raw(&[
                            real_fg_color.r,
                            real_fg_color.g,
                            real_fg_color.b,
                            alpha,
                        ])
                        .into_format();
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
