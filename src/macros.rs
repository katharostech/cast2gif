macro_rules! flame {
    (start $message:literal) => {
        #[cfg(feature = "flamegraph")]
        flame::start($message);
    };
    (end $message:literal) => {
        #[cfg(feature = "flamegraph")]
        flame::end($message);
    };
    (note $message:literal) => {
        #[cfg(feature = "flamegraph")]
        flame::note($message, None);
    };
    (guard $message:literal) => {
        #[cfg(feature = "flamegraph")]
        let _g = flame::start_guard($message);
    };
}