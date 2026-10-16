fn main() {
    if let Err(error) = lapis_agent::run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
