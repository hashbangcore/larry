//! Error types for argument validation and system interaction.

use std::io;
use std::process::ExitCode;

use thiserror::Error;

/// A failure that stopped larry.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The command line could not be understood.
    #[error(transparent)]
    Usage(#[from] clap::Error),
    /// A command could not be started.
    #[error("failed to run command '{command}': {source}")]
    Spawn {
        /// The command that failed to start.
        command: String,
        /// The underlying cause.
        #[source]
        source: io::Error,
    },
    /// Standard input could not be read.
    #[error("failed to read stdin: {0}")]
    Read(#[source] io::Error),
    /// Output could not be written.
    #[error("failed to write output: {0}")]
    Write(#[source] io::Error),
}

impl Error {
    /// Reports whether the output stream was closed by the reader.
    ///
    /// A closed pipe ends the run quietly, the way other command line tools
    /// behave when their output is piped into `head`.
    #[must_use = "the broken pipe state decides how the run ends"]
    pub fn is_broken_pipe(&self) -> bool {
        matches!(self, Error::Write(source) if source.kind() == io::ErrorKind::BrokenPipe)
    }

    /// Writes the error to the stream that fits it and returns the exit code.
    ///
    /// Usage errors carry their own rendering. Help and version output go to
    /// stdout and leave the run successful.
    #[must_use = "the exit code of a failed run must not be discarded"]
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
