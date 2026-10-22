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

pub use cli::{Invocation, Plan, Repeat, RepeatCount};
pub use error::Error;
pub use section::Section;

use std::ffi::OsString;
use std::io;

/// Runs larry with the given arguments.
///
/// The arguments include the program name, as they do in `argv`.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), Error> {
    let plan = cli::parse(args)?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    app::run(&plan, &mut handle)
}
