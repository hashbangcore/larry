//! Error types for argument validation and system interaction.

use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::process::ExitCode;

/// A failure that stopped larry.
#[derive(Debug)]
pub enum Error {
    /// The command line could not be understood.
    Usage(clap::Error),
    /// A command could not be started.
    Spawn {
        /// The command that failed to start.
        command: String,
        /// The underlying cause.
        source: io::Error,
    },
    /// Standard input could not be read.
    Read(io::Error),
    /// Output could not be written.
    Write(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Usage(source) => write!(f, "{source}"),
            Error::Spawn { command, source } => {
                write!(f, "failed to run command '{command}': {source}")
            }
            Error::Read(source) => write!(f, "failed to read stdin: {source}"),
            Error::Write(source) => write!(f, "failed to write output: {source}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Usage(source) => Some(source),
            Error::Spawn { source, .. } => Some(source),
            Error::Read(source) | Error::Write(source) => Some(source),
        }
    }
}

impl From<clap::Error> for Error {
    fn from(source: clap::Error) -> Self {
        Error::Usage(source)
    }
}

impl Error {
    /// Reports whether the output stream was closed by the reader.
    ///
    /// A closed pipe ends the run quietly, the way other command line tools
    /// behave when their output is piped into `head`.
    pub fn is_broken_pipe(&self) -> bool {
        matches!(self, Error::Write(source) if source.kind() == io::ErrorKind::BrokenPipe)
    }

    /// Writes the error to the stream that fits it and returns the exit code.
    ///
    /// Usage errors carry their own rendering, so they are printed as clap
    /// formatted them. Help and version output are not failures and leave
    /// the run successful.
    pub fn report(&self) -> ExitCode {
        match self {
            Error::Usage(report) => {
                let _ = report.print();

                if report.use_stderr() {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Error::Spawn { .. } | Error::Read(_) | Error::Write(_) => {
                if self.is_broken_pipe() {
                    return ExitCode::SUCCESS;
                }

                eprintln!("error: {self}");
                ExitCode::FAILURE
            }
        }
    }
}
