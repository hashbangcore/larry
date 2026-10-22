//! Parsing of command line arguments into an executable plan.

use std::ffi::OsString;
use std::num::NonZeroU8;
use std::str::FromStr;
use std::time::Duration;

use clap::{ArgAction, CommandFactory, Parser};
use thiserror::Error;

/// Largest number of repetitions accepted by `-r`.
pub const MAX_REPEAT: u8 = 100;

/// Pause between repetitions when `-r 0` selects an endless loop.
pub(crate) const INFINITE_INTERVAL: Duration = Duration::from_secs(5);

/// Command line interface of larry.
#[derive(Debug, Parser)]
#[command(
    name = "larry",
    version,
    about = "Capture piped stdin and execute commands, producing structured output with [section] markers.",
    long_about = "Capture piped stdin and execute commands, producing structured output\nwith [section] markers.",
    disable_version_flag = true
)]
pub(crate) struct Cli {
    /// How many times to repeat the corresponding command.
    ///
    /// Defaults to 1, accepts up to 100, and 0 repeats the command forever
    /// with a 5 second pause between runs.
    #[arg(
        short = 'r',
        long = "repeat",
        value_name = "N",
        action = ArgAction::Append,
        value_parser = Repeat::from_str
    )]
    repeat: Vec<Repeat>,

    /// Commands to execute via sh -c.
    #[arg(value_name = "COMMAND")]
    command: Vec<String>,

    /// Show version and exit.
    #[arg(short = 'v', long = "version", action = ArgAction::Version)]
    version: Option<bool>,
}

/// The commands to execute, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Commands to run, each with its repetition count.
    pub invocations: Vec<Invocation>,
}

/// A command and how many times it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The command string, passed to `sh -c` verbatim.
    pub command: String,
    /// How many times the command runs.
    pub repeat: Repeat,
}

/// How many times an invocation runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Repeat {
    /// A fixed number of runs, back to back.
    Count(RepeatCount),
    /// Endless runs, pausing between each one.
    Forever {
        /// Pause between two consecutive runs.
        interval: Duration,
    },
}

/// A repetition count between 1 and [`MAX_REPEAT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeatCount(NonZeroU8);

impl RepeatCount {
    /// Largest accepted count.
    pub const MAX: u8 = MAX_REPEAT;

    /// A single run.
    pub const ONE: RepeatCount = RepeatCount(NonZeroU8::MIN);

    /// The numeric value of the count.
    #[must_use = "the count is the result of the conversion"]
    pub fn value(self) -> u8 {
        self.0.get()
    }
}

impl TryFrom<u8> for RepeatCount {
    type Error = InvalidRepeat;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match NonZeroU8::new(value) {
            Some(value) if value.get() <= Self::MAX => Ok(RepeatCount(value)),
            Some(_) => Err(InvalidRepeat::AboveMaximum { max: Self::MAX }),
            None => Err(InvalidRepeat::BelowMinimum { min: 1 }),
        }
    }
}

/// Parses command line arguments into a [`Plan`].
///
/// Repeat counts and commands are paired by position, so `-r` may appear
/// before or after the command it applies to.
///
/// # Errors
///
/// Returns an error when the arguments cannot be understood, when a repeat
/// count is not a number between 0 and 100, or when there are more repeat
/// counts than commands.
pub(crate) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Plan, clap::Error> {
    let cli = Cli::try_parse_from(args)?;
    cli.into_plan()
}

impl Cli {
    /// Pairs the repeat counts with the commands.
    fn into_plan(self) -> Result<Plan, clap::Error> {
        let mut repeats = self.repeat.into_iter();
        let mut invocations = Vec::with_capacity(self.command.len());

        for command in self.command {
            let repeat = repeats.next().unwrap_or(Repeat::Count(RepeatCount::ONE));

            invocations.push(Invocation { command, repeat });
        }

        if repeats.next().is_some() {
            return Err(Cli::command().error(
                clap::error::ErrorKind::InvalidValue,
                "-r requires a command",
            ));
        }

        Ok(Plan { invocations })
    }
}

impl FromStr for Repeat {
    type Err = InvalidRepeat;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let value: u32 = raw.parse().map_err(|_| InvalidRepeat::NotANumber {
            value: raw.to_string(),
        })?;

        if value == 0 {
            return Ok(Repeat::Forever {
                interval: INFINITE_INTERVAL,
            });
        }

        let count =
            u8::try_from(value).map_err(|_| InvalidRepeat::AboveMaximum { max: MAX_REPEAT })?;

        RepeatCount::try_from(count).map(Repeat::Count)
    }
}

/// A repeat count that could not be understood.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidRepeat {
    /// The value was not a number.
    #[error("invalid repeat count '{value}'")]
    NotANumber {
        /// The offending value.
        value: String,
    },
    /// The value was below the smallest accepted count.
    #[error("repeat count must be at least {min}")]
    BelowMinimum {
        /// The smallest accepted count.
        min: u8,
    },
    /// The value was above the largest accepted count.
    #[error("repeat count must not exceed {max}")]
    AboveMaximum {
        /// The largest accepted count.
        max: u8,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repeat_count_becomes_a_counted_repetition() {
        assert_eq!(
            "3".parse::<Repeat>().expect("valid count"),
            Repeat::Count(RepeatCount::try_from(3).expect("valid count"))
        );
    }

    #[test]
    fn a_zero_count_becomes_an_endless_loop() {
        assert_eq!(
            "0".parse::<Repeat>().expect("valid count"),
            Repeat::Forever {
                interval: INFINITE_INTERVAL
            }
        );
    }

    #[test]
    fn a_count_above_the_maximum_is_rejected() {
        assert!(matches!(
            "101".parse::<Repeat>(),
            Err(InvalidRepeat::AboveMaximum { max: 100 })
        ));
    }

    #[test]
    fn a_non_numeric_count_is_rejected() {
        assert!(matches!(
            "abc".parse::<Repeat>(),
            Err(InvalidRepeat::NotANumber { value }) if value == "abc"
        ));
    }

    #[test]
    fn a_negative_count_is_rejected() {
        assert!(matches!(
            "-1".parse::<Repeat>(),
            Err(InvalidRepeat::NotANumber { value }) if value == "-1"
        ));
    }

    #[test]
    fn the_largest_accepted_count_is_accepted() {
        assert_eq!(
            RepeatCount::try_from(100).expect("valid count").value(),
            100
        );
    }

    #[test]
    fn a_count_of_zero_is_rejected() {
        assert!(matches!(
            RepeatCount::try_from(0),
            Err(InvalidRepeat::BelowMinimum { min: 1 })
        ));
    }

    #[test]
    fn a_count_above_the_maximum_is_rejected_by_conversion() {
        assert!(matches!(
            RepeatCount::try_from(101),
            Err(InvalidRepeat::AboveMaximum { max: 100 })
        ));
    }

    #[test]
    fn a_repeat_applies_to_the_command_in_the_same_position() {
        let cli = Cli {
            repeat: vec![
                Repeat::Count(RepeatCount::try_from(4).expect("valid count")),
                Repeat::Count(RepeatCount::try_from(2).expect("valid count")),
            ],
            command: vec!["pwd".to_string(), "whoami".to_string()],
            version: None,
        };

        let plan = cli.into_plan().expect("valid plan");

        assert_eq!(
            plan.invocations,
            vec![
                Invocation {
                    command: "pwd".to_string(),
                    repeat: Repeat::Count(RepeatCount::try_from(4).expect("valid count")),
                },
                Invocation {
                    command: "whoami".to_string(),
                    repeat: Repeat::Count(RepeatCount::try_from(2).expect("valid count")),
                },
            ]
        );
    }

    #[test]
    fn a_command_without_a_repeat_runs_once() {
        let cli = Cli {
            repeat: Vec::new(),
            command: vec!["pwd".to_string()],
            version: None,
        };

        let plan = cli.into_plan().expect("valid plan");

        assert_eq!(plan.invocations[0].repeat, Repeat::Count(RepeatCount::ONE));
    }

    #[test]
    fn more_repeats_than_commands_is_rejected() {
        let cli = Cli {
            repeat: vec![
                Repeat::Count(RepeatCount::try_from(2).expect("valid count")),
                Repeat::Count(RepeatCount::try_from(3).expect("valid count")),
            ],
            command: vec!["pwd".to_string()],
            version: None,
        };

        assert!(cli.into_plan().is_err());
    }
}
