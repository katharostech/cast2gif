# Cast2Gif

A tool to convert [Asciinema](https://github.com/asciinema/asciinema) cast file to Gif, animated PNG, or SVG files *without* using Electron or a web browser.

> **Warning:** Right now this is very work-in-progress, though the concept and backing technology seem to be succesfully proved out so far. Discussion is happening on a [forum topic](https://users.rust-lang.org/t/writing-an-asciinema-to-gif-tool/39450/15?u=zicklag) on the Rust forum.

## Example

The best image I've gotten so far, a recording of cast2gif itself:

![example](./doc/example1.gif)

## Known Issues

There is an issue with flickering on very fast updating terminal UIs as shown in the example above. This is a rather tricky thing to solve, for but I've got an idea that _might_ fix it. :wink:

## Features to Add

Here are some features to get in as time permits, ordered by importance:

- Support for changing the output resolution/font-size
- Automated builds for Windows, Mac, and Linux so users don't have to build it themselves
- Support for custom SVG templates to allow you to style the output
- Support for rendering animated PNGs
- Support for rendering animated SVGs


## Building and Running

To build you must have Rust installed. You can install it with [Rustup](https://rustup.rs/). Then Run

    cargo build --release

After that the `cast2gif` program will be in the `target/release` folder.

Run `cast2gif --help` to get the usage instructions:

> **Note:** Sorry for the tease in the help message, `png` and `svg` image output types aren't done yet!

```
cast2gif 0.1.0
Zicklag <zicklag@katharostech.com>
Renders Asciinema .cast files as gif, svg, or animated png.

USAGE:
    cast2gif [FLAGS] [OPTIONS] <cast_file> <out_file>

FLAGS:
    -f, --force      Overwrite existing output file
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -F, --format <format>
            The file format to render to. This will be automatically determined
            from the file extension if not specified. [possible values:
            gif, svg, png]
    -i, --frame-interval <frame_interval>
            The interval at which frames from the recording are rendered [default:
            0.1]

ARGS:
    <cast_file>    The asciinema .cast file to render
    <out_file>     The file to render to
```