//! Parsing of command line arguments into an executable plan.

use std::num::NonZeroU8;
use std::time::Duration;

use clap::{ArgAction, CommandFactory, Parser};

/// Largest number of repetitions accepted by `-r`.
pub const MAX_REPEAT: u8 = 100;

/// Pause between repetitions when `-r 0` selects an endless loop.
pub const INFINITE_INTERVAL: Duration = Duration::from_secs(5);

/// Command line interface of larry.
#[derive(Debug, Parser)]
#[command(
    name = "larry",
    version,
    about = "Capture piped stdin and execute commands, producing structured output with [section] markers.",
    long_about = "Capture piped stdin and execute commands, producing structured output\nwith [section] markers.",
    disable_version_flag = true
)]
pub struct Cli {
    /// How many times to repeat the corresponding command.
    ///
    /// Defaults to 1, accepts up to 100, and 0 repeats the command forever
    /// with a 5 second pause between runs.
    #[arg(
        short = 'r',
        long = "repeat",
        value_name = "N",
        action = ArgAction::Append,
        value_parser = parse_repeat
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

    /// Builds a count, rejecting zero and anything above [`RepeatCount::MAX`].
    pub fn new(value: u8) -> Option<Self> {
        let value = NonZeroU8::new(value).filter(|value| value.get() <= Self::MAX)?;
        Some(RepeatCount(value))
    }

    /// The numeric value of the count.
    pub fn value(self) -> u8 {
        self.0.get()
    }
}

/// Parses command line arguments into a [`Plan`].
///
/// Repeat counts and commands are paired in the order they were given, so
/// `-r` may appear before or after the command it applies to.
pub fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Plan, clap::Error> {
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

/// Parses a `-r` value into a [`Repeat`].
fn parse_repeat(raw: &str) -> Result<Repeat, String> {
    let value: u32 = raw
        .parse()
        .map_err(|_| format!("invalid repeat count '{raw}'"))?;

    if value == 0 {
        return Ok(Repeat::Forever {
            interval: INFINITE_INTERVAL,
        });
    }

    u8::try_from(value)
        .ok()
        .and_then(RepeatCount::new)
        .map(Repeat::Count)
        .ok_or_else(|| format!("repeat count must not exceed {MAX_REPEAT}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repeat_count_becomes_a_counted_repetition() {
        assert_eq!(
            parse_repeat("3").expect("valid count"),
            Repeat::Count(RepeatCount::new(3).expect("valid count"))
        );
    }

    #[test]
    fn a_zero_count_becomes_an_endless_loop() {
        assert_eq!(
            parse_repeat("0").expect("valid count"),
            Repeat::Forever {
                interval: INFINITE_INTERVAL
            }
        );
    }

    #[test]
    fn a_count_above_the_maximum_is_rejected() {
        assert_eq!(
            parse_repeat("101"),
            Err("repeat count must not exceed 100".to_string())
        );
    }

    #[test]
    fn a_non_numeric_count_is_rejected() {
        assert_eq!(
            parse_repeat("abc"),
            Err("invalid repeat count 'abc'".to_string())
        );
    }

    #[test]
    fn a_negative_count_is_rejected() {
        assert_eq!(
            parse_repeat("-1"),
            Err("invalid repeat count '-1'".to_string())
        );
    }

    #[test]
    fn a_repeat_applies_to_the_command_in_the_same_position() {
        let cli = Cli {
            repeat: vec![
                Repeat::Count(RepeatCount::new(4).expect("valid count")),
                Repeat::Count(RepeatCount::new(2).expect("valid count")),
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
                    repeat: Repeat::Count(RepeatCount::new(4).expect("valid count")),
                },
                Invocation {
                    command: "whoami".to_string(),
                    repeat: Repeat::Count(RepeatCount::new(2).expect("valid count")),
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
                Repeat::Count(RepeatCount::new(2).expect("valid count")),
                Repeat::Count(RepeatCount::new(3).expect("valid count")),
            ],
            command: vec!["pwd".to_string()],
            version: None,
        };

        assert!(cli.into_plan().is_err());
    }
}
