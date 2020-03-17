use anyhow::{format_err, Context};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::path::Path;

mod logging;

pub fn run() {
    // Enable colored backtraces
    #[cfg(feature = "better-panic")]
    better_panic::Settings::auto().lineno_suffix(true).install();

    // Initialize logger
    env_logger::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(logging::formatter)
        .init();

    std::panic::catch_unwind(|| {
        // run program and report any errors
        if let Err(e) = execute_cli() {
            log::error!("{:?}", e);
            std::process::exit(1);
        }
    })
    // Catch any panics and print an error message. This will appear after the message given by
    // better backtrace.
    // TODO: Replace all uses of the concat macro for wrapping strings with backslash escapes
    .or_else(|_| -> Result<(), ()> {
        log::error!(concat!(
            "The program has encountered a critical internal error and will now exit. ",
            "This is a bug. Please report it on our issue tracker:\n\n",
            "    https://github.com/katharostech/cast2gif/issues"
        ));

        std::process::exit(1);
    })
    .expect("Panic while handling panic");
}

enum OutputFormat {
    Gif,
    Png,
    Svg,
}

fn execute_cli() -> anyhow::Result<()> {
    use clap::{crate_authors, crate_version, App, AppSettings, Arg};
    #[rustfmt::skip]
    let args = App::new("cast2gif")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Renders Asciinema .cast files as gif, svg, or animated png.")
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(Arg::with_name("cast_file")
            .help("The asciinema .cast file to render")
            .required(true))
        .arg(Arg::with_name("out_file")
            .help("The file to render to")
            .required(true))
        .arg(Arg::with_name("format")
            .long("format")
            .short("F")
            .help("The file format to render to. This will be automatically determined from the \
                   file extension if not specified.")
            .takes_value(true)
            .possible_values(&["gif", "svg", "png"]))
        .arg(Arg::with_name("force")
            .long("force")
            .short("f")
            .help("Overwrite existing output file"))
        .get_matches();

    // Load cast file
    let cast_file_path = args
        .value_of("cast_file")
        .expect("Missing required argument: cast_file");
    let cast_file = std::fs::OpenOptions::new()
        .read(true)
        .open(cast_file_path)
        .context(format!("Could not open cast file: {}", cast_file_path))?;

    // Get output path
    let out_file_path = Path::new(
        args.value_of("out_file")
            .expect("Missing required argument: out_file"),
    );

    // Make sure out path doesn't exist
    if out_file_path.exists() && !args.is_present("force") {
        return Err(format_err!(
            "Output file already exists: {}",
            out_file_path.to_string_lossy()
        ));
    }

    // Open out file
    let out_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_file_path)
        .context(format!(
            "Could not open output file: {}",
            out_file_path.to_string_lossy()
        ))?;

    let format = match args.value_of("format") {
        // Guess format from file extension
        None => {
            let warn_message = "Could not detect output format from file extension, assuming gif \
                                format. Use --format to specify otherwise.";
            if let Some(ext) = out_file_path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                match ext.as_str() {
                    "gif" => OutputFormat::Gif,
                    "svg" => OutputFormat::Svg,
                    "png" => OutputFormat::Png,
                    _ => {
                        log::warn!("{}", warn_message);
                        OutputFormat::Gif
                    }
                }
            } else {
                log::warn!("{}", warn_message);
                OutputFormat::Gif
            }
        }
        // Use seleted output format
        Some("gif") => OutputFormat::Gif,
        Some("svg") => OutputFormat::Svg,
        Some("png") => OutputFormat::Png,
        Some(other) => panic!("Invalid option to --format: {}", other),
    };

    // Create the progress bars
    let multi = MultiProgress::new();
    let template =
        "{prefix:12} [{elapsed_precise:.dim}]: {wide_bar:.green/white} {pos:>7}/{len:7} ( {eta_precise:.dim} )";
    let raster_progress =
        multi.add(ProgressBar::new(0).with_style(ProgressStyle::default_bar().template(template)));
    let sequence_progress =
        multi.add(ProgressBar::new(0).with_style(ProgressStyle::default_bar().template(template)));

    match format {
        OutputFormat::Gif => {
            std::thread::spawn(move || {
                crate::convert_to_gif_with_progress(&cast_file, &out_file, |progress| {
                    macro_rules! handle_progress {
                        ($x:expr, $p:expr, $message:expr) => {
                            $x.set_length($p.count);
                            if $x.position() > 0 {
                                $x.set_prefix($message);
                            } else if $x.is_finished() {
                                $x.set_prefix("Done")
                            } else {
                                $x.set_prefix("Waiting")
                            }
                            $x.set_position($p.progress);

                            if $x.is_finished() {
                                $x.finish();
                            }
                        };
                    };

                    handle_progress!(raster_progress, progress.raster_progress, "Rasterizing");
                    handle_progress!(sequence_progress, progress.sequence_progress, "Sequencing");
                })
                .expect("TODO");
            });
        }
        _ => unimplemented!(),
    }

    multi.join_and_clear().expect("TODO");

    Ok(())
}
