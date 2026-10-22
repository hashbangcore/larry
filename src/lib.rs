//! Larry captures piped input and command output as labeled sections.
//!
//! The command line is parsed once into a [`Plan`], which is then executed,
//! streaming one [`Section`] per command execution.
//!
//! # Examples
//!
//! ```
//! use larry::Section;
//!
//! let section = Section::Stdin {
//!     text: "payload".to_string(),
//! };
//!
//! assert_eq!(
//!     section.to_string(),
//!     "[section]\n[stdin]\npayload\n[end section]"
//! );
//! ```

mod app;
mod cli;
mod error;
mod exec;
mod output;
mod section;

pub use cli::{InvalidRepeat, Invocation, MAX_REPEAT, Plan, Repeat, RepeatCount};
pub use error::Error;
pub use section::Section;

use std::ffi::OsString;
use std::io;

/// Runs larry with the given arguments.
///
/// The arguments include the program name, as they do in `argv`.
///
/// # Errors
///
/// Returns an error when the arguments cannot be understood, when a command
/// cannot be started, or when input or output cannot be read or written.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), Error> {
    let plan = cli::parse(args)?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    app::run(&plan, &mut handle)
}
