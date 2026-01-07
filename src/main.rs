use std::process;

fn main() {
    if let Err(e) = rusk::cli::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
