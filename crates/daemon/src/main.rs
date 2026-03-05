fn main() {
    if let Err(err) = kwybars_daemon::run() {
        eprintln!("kwybars-daemon failed: {err}");
        std::process::exit(1);
    }
}
