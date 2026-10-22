use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match larry::run(env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => error.report(),
    }
}
