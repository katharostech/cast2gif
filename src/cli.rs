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
            "The program has encountered a critical internal error and will now exit. \
             This is a bug. Please report it on our issue tracker:\n\n\
                 https://github.com/katharostech/cast2gif/issues"
        ));

        std::process::exit(1);
    })
    .expect("Panic while handling panic");
}

fn execute_cli() -> anyhow::Result<()> {
    log::trace!("Hello trace");
    log::debug!("Hello debug");
    log::info!("Hello info");
    log::warn!("Hello warn");
    log::error!("Hello error");

    Ok(())
}
