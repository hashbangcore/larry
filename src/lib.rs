//! Larry captures piped input and command output as labeled sections.
//!
//! The command line is parsed once into a [`Plan`], which the [`app`] module
//! then executes, streaming one [`Section`] per command execution.

pub mod app;
pub mod cli;
pub mod error;
pub mod exec;
pub mod output;
pub mod section;

pub use cli::{Action, Invocation, Plan, Repeat, RepeatCount};
pub use error::{Error, UsageError};
pub use section::Section;

use std::env;
use std::io::{self, Write};

/// Runs larry with the given arguments.
pub fn run<I, T>(args: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    match cli::parse(args)? {
        Action::Help => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(cli::HELP.as_bytes()).map_err(Error::Write)
        }
        Action::Version => {
            let mut stdout = io::stdout().lock();
            writeln!(stdout, "larry {}", env!("CARGO_PKG_VERSION")).map_err(Error::Write)
        }
        Action::Run(plan) => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            app::run(&plan, &mut handle)
        }
    }
}
