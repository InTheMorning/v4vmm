fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return if v4vmm::app::run_app() {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }

    #[cfg(debug_assertions)]
    if args.first().is_some_and(|arg| arg == "startup-fixture") {
        return match v4vmm::startup::fixture::run_cli(&args[1..]) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Error: {error:#}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    if let Err(error) = v4vmm::cli::run(&args) {
        eprintln!("Error: {error:#}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
