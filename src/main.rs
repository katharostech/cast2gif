fn main() {
    #[cfg(feature = "cli")]
    cast2gif::cli::run();

    #[cfg(not(feature = "cli"))]
    println!("Must be built with the \"cli\" feature.");
}
