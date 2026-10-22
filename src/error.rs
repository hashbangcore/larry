//! Error types for argument validation and system interaction.

use std::error::Error as StdError;
use std::fmt;
use std::io;

/// A problem with the command line that larry was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    /// `-r` was not followed by a count.
    MissingRepeatCount,
    /// The value after `-r` was not a number.
    InvalidRepeatCount {
        /// The offending value.
        value: String,
    },
    /// The value after `-r` was above the allowed maximum.
    RepeatCountTooHigh {
        /// The requested count.
        count: usize,
        /// The largest allowed count.
        max: u8,
    },
    /// `-r` was not followed by a command to repeat.
    MissingCommand,
    /// An argument looked like a flag but is not known.
    UnknownFlag {
        /// The offending flag.
        flag: String,
    },
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsageError::MissingRepeatCount => write!(f, "-r requires a repeat count"),
            UsageError::InvalidRepeatCount { value } => {
                write!(f, "-r requires a valid number, got '{value}'")
            }
            UsageError::RepeatCountTooHigh { count, max } => {
                write!(f, "-r maximum is {max}, got '{count}'")
            }
            UsageError::MissingCommand => write!(f, "-r requires a command"),
            UsageError::UnknownFlag { flag } => {
                write!(f, "unknown flag '{flag}'. Use 'larry -h' for usage.")
            }
        }
    }
}

impl StdError for UsageError {}

/// A failure that stopped larry.
#[derive(Debug)]
pub enum Error {
    /// The command line could not be understood.
    Usage(UsageError),
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
            Error::Usage(usage) => write!(f, "{usage}"),
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
            Error::Usage(usage) => Some(usage),
            Error::Spawn { source, .. } => Some(source),
            Error::Read(source) | Error::Write(source) => Some(source),
        }
    }
}

impl From<UsageError> for Error {
    fn from(usage: UsageError) -> Self {
        Error::Usage(usage)
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
}
