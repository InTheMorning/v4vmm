fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        v4vmm::app::run_app();
        return;
    }

    if let Err(error) = v4vmm::cli::run(&args) {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}
