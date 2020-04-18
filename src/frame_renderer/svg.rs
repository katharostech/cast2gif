/// The SVG-based implementation of the frame renderer
use rgb::{AsPixels, RGBA8};

use crate::types::*;

use super::parse_color;

fn render_frame_to_svg(frame: &TerminalFrame) -> SvgFrame {
    use svg::{
        node::{
            element::{Rectangle, Text},
            Text as TextNode,
        },
        Document,
    };

    // Set the size of the terminal cells
    // TODO: Make this dynamic based on the font and font-size
    let font_size = 10;
    let cell_width = 6;
    let cell_height = font_size;

    // Get the size of the terminal screen
    let (rows, cols) = frame.screen.size();
    let doc_height = rows * cell_height;
    let doc_width = cols * cell_width;

    // Create the svg document
    let mut doc = Document::new()
        .set("viewbox", (0, 0, doc_width, doc_height))
        .set("height", doc_height)
        .set("width", doc_width);

    // TODO: Allow custom
    let background_color = "#000000";
    let foreground_color = "#ffffff";

    // Draw the terminal background
    doc = doc.add(
        Rectangle::new()
            .set(
                "style",
                format!(
                    "fill:{bgcolor};fill-opacity:1;stroke:none",
                    bgcolor = background_color
                ),
            )
            .set("x", "0")
            .set("y", "0")
            .set("width", doc_width)
            .set("height", doc_height),
    );

    // Iterate through each cell
    for row in 0..rows {
        for col in 0..cols {
            // Get the cell
            let cell = frame.screen.cell(row, col).unwrap_or_else(|| {
                panic!(
                    "Missing cell at position ({}, {}) in frame at {}",
                    row, col, frame.time
                )
            });

            // If the cell has a background color
            if let Some(bg_color) = parse_color(cell.bgcolor()) {
                doc = doc.add(
                    Rectangle::new()
                        .set("x", (col * cell_width).to_string())
                        .set("y", (row * cell_height).to_string())
                        .set("width", cell_width.to_string())
                        .set("height", cell_height.to_string())
                        .set(
                            "style",
                            format!(
                                "fill:{bgcolor};fill-opacity:1;stroke:none",
                                bgcolor = bg_color
                            ),
                        ),
                );
            }
            // If the cell is not empty
            let contents = cell.contents();
            if contents != "" && contents != " " {
                let text_color =
                    parse_color(cell.fgcolor()).unwrap_or_else(|| foreground_color.into());
                // Add the cell's text to the SVG
                doc = doc.add(
                    Text::new()
                        .add(TextNode::new(contents))
                        .set("x", (col * cell_width).to_string())
                        .set(
                            "y",
                            ((row + 1) * cell_height - 3/* TODO: Fix for text position */)
                                .to_string(),
                        )
                        .set("width", cell_width.to_string())
                        .set("height", cell_height.to_string())
                        .set(
                            "style",
                            format!(
                                "font-size: {font_size}px; \
                                font-family: monospace; \
                                fill: {color};",
                                // font = font_family,
                                font_size = font_size,
                                color = text_color,
                            ),
                        ),
                );
            }
        }
    }

    // std::fs::create_dir_all("out-svg.gitignore").expect("TODO");
    // svg::save(format!("out-svg.gitignore/{}.svg", frame.time), &doc).expect("TODO");

    SvgFrame {
        index: frame.index,
        time: frame.time,
        doc,
        width: doc_width,
        height: doc_height,
    }
}

pub(crate) fn render_frame_to_png(frame: TerminalFrame) -> RgbaFrame {
    use resvg::prelude::*;
    // Get the SVG render of the frame
    let svg_doc = render_frame_to_svg(&frame);

    let opt = resvg::Options::default();
    let rtree = usvg::Tree::from_str(&svg_doc.doc.to_string(), &opt.usvg).expect("TODO");
    let backend = resvg::default_backend();
    let mut img = backend.render_to_image(&rtree, &opt).expect("TODO");

    // std::fs::create_dir_all("out-png.gitignore").expect("TODO");
    // img.save_png(&std::path::PathBuf::from(format!(
    //     "out-png.gitignore/{}.png",
    //     frame.time
    // )));

    // Collect image
    let rgba8_pixels = img.make_rgba_vec();
    let rgba8_pixels: Vec<RGBA8> = rgba8_pixels
        .as_slice()
        .as_pixels()
        .iter()
        .map(Clone::clone)
        .collect();

    RgbaFrame {
        time: frame.time,
        index: frame.index,
        image: imgref::Img::new(
            rgba8_pixels,
            // TODO: avoid using `as`
            svg_doc.width as usize,
            svg_doc.height as usize,
        ),
    }
}
