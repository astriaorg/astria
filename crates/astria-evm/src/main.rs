fn main() {
    if let Err(err) = symphony::cli::run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
