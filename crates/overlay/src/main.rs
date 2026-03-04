mod app;

fn main() {
    if let Err(err) = app::run() {
        eprintln!("kwybars-overlay failed: {err}");
        std::process::exit(1);
    }
}
