use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match larry::run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.is_broken_pipe() => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
