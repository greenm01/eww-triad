use std::process::ExitCode;

fn main() -> ExitCode {
    match eww_triad::cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("eww-triad: {err}");
            ExitCode::FAILURE
        }
    }
}
