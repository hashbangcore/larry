//! Parsing of command line arguments into an executable plan.

use std::num::NonZeroU8;
use std::time::Duration;

use crate::error::UsageError;

/// Largest number of repetitions accepted by `-r`.
pub const MAX_REPEAT: u8 = 100;

/// Pause between repetitions when `-r 0` selects an endless loop.
pub const INFINITE_INTERVAL: Duration = Duration::from_secs(5);

/// Usage text shown by `-h`.
pub const HELP: &str = concat!(
    "Version ",
    env!("CARGO_PKG_VERSION"),
    "\n",
    "\n",
    "Usage: larry [OPTIONS] [COMMAND]...\n",
    "\n",
    "Capture piped stdin and execute commands, producing structured output\n",
    "with [section] markers.\n",
    "\n",
    "Arguments:\n",
    "  [COMMAND]...    Commands to execute via sh -c\n",
    "\n",
    "Options:\n",
    "  -r <N>          Repeat next command N times\n",
    "                  default: 1, max: 100, 0 = infinite loop with 5s delay\n",
    "\n",
    "  -v              Show version and exit\n",
    "\n",
    "  -h              Show this help message and exit\n",
    "\n",
    "Each command and each repetition produces its own [section] in the output.\n",
    "Use quotes to pass flags to commands: larry \"ls -la\"\n",
    "\n",
);

/// What the command line asked larry to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Print the usage text.
    Help,
    /// Print the version.
    Version,
    /// Execute the planned commands.
    Run(Plan),
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

/// Parses command line arguments into an [`Action`].
///
/// `-h` and `-v` are honoured wherever they appear, and take precedence over
/// any other argument.
pub fn parse<I, T>(args: I) -> Result<Action, UsageError>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();

    for arg in &args {
        match arg.as_str() {
            "-h" => return Ok(Action::Help),
            "-v" => return Ok(Action::Version),
            _ => {}
        }
    }

    let mut invocations = Vec::new();
    let mut pending: Option<Repeat> = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        if arg == "-r" {
            index += 1;
            let raw = args.get(index).ok_or(UsageError::MissingRepeatCount)?;
            let count: usize = raw
                .parse()
                .map_err(|_| UsageError::InvalidRepeatCount { value: raw.clone() })?;

            pending = Some(if count == 0 {
                Repeat::Forever {
                    interval: INFINITE_INTERVAL,
                }
            } else {
                match u8::try_from(count).ok().and_then(RepeatCount::new) {
                    Some(count) => Repeat::Count(count),
                    None => {
                        return Err(UsageError::RepeatCountTooHigh {
                            count,
                            max: MAX_REPEAT,
                        });
                    }
                }
            });

            index += 1;
            continue;
        }

        if arg.starts_with('-') {
            return Err(UsageError::UnknownFlag { flag: arg.clone() });
        }

        invocations.push(Invocation {
            command: arg.clone(),
            repeat: pending.take().unwrap_or(Repeat::Count(RepeatCount::ONE)),
        });

        index += 1;
    }

    if pending.is_some() {
        return Err(UsageError::MissingCommand);
    }

    Ok(Action::Run(Plan { invocations }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(args: &[&str]) -> Plan {
        match parse(args.iter().copied()).expect("valid arguments") {
            Action::Run(plan) => plan,
            other => panic!("expected a run plan, got {other:?}"),
        }
    }

    fn usage(args: &[&str]) -> UsageError {
        parse(args.iter().copied()).expect_err("expected a usage error")
    }

    #[test]
    fn no_arguments_produce_an_empty_plan() {
        assert_eq!(
            plan(&[]),
            Plan {
                invocations: vec![]
            }
        );
    }

    #[test]
    fn a_command_without_repeat_runs_once() {
        let plan = plan(&["pwd"]);

        assert_eq!(
            plan.invocations,
            vec![Invocation {
                command: "pwd".to_string(),
                repeat: Repeat::Count(RepeatCount::ONE),
            }]
        );
    }

    #[test]
    fn commands_keep_their_order() {
        let plan = plan(&["pwd", "whoami"]);

        let commands: Vec<&str> = plan
            .invocations
            .iter()
            .map(|invocation| invocation.command.as_str())
            .collect();

        assert_eq!(commands, ["pwd", "whoami"]);
    }

    #[test]
    fn repeat_applies_to_the_following_command_only() {
        let plan = plan(&["-r", "3", "pwd", "whoami"]);

        assert_eq!(
            plan.invocations[0].repeat,
            Repeat::Count(RepeatCount::new(3).unwrap())
        );
        assert_eq!(plan.invocations[1].repeat, Repeat::Count(RepeatCount::ONE));
    }

    #[test]
    fn repeat_zero_selects_an_endless_loop() {
        let plan = plan(&["-r", "0", "date"]);

        assert_eq!(
            plan.invocations[0].repeat,
            Repeat::Forever {
                interval: INFINITE_INTERVAL
            }
        );
    }

    #[test]
    fn the_maximum_count_is_accepted() {
        let plan = plan(&["-r", "100", "pwd"]);

        assert_eq!(
            plan.invocations[0].repeat,
            Repeat::Count(RepeatCount::new(100).unwrap())
        );
    }

    #[test]
    fn a_count_above_the_maximum_is_rejected() {
        assert_eq!(
            usage(&["-r", "101", "pwd"]),
            UsageError::RepeatCountTooHigh {
                count: 101,
                max: MAX_REPEAT
            }
        );
    }

    #[test]
    fn a_non_numeric_count_is_rejected() {
        assert_eq!(
            usage(&["-r", "abc", "pwd"]),
            UsageError::InvalidRepeatCount {
                value: "abc".to_string()
            }
        );
    }

    #[test]
    fn repeat_without_a_count_is_rejected() {
        assert_eq!(usage(&["-r"]), UsageError::MissingRepeatCount);
    }

    #[test]
    fn repeat_without_a_command_is_rejected() {
        assert_eq!(usage(&["-r", "2"]), UsageError::MissingCommand);
    }

    #[test]
    fn unknown_flags_are_rejected() {
        assert_eq!(
            usage(&["--bogus"]),
            UsageError::UnknownFlag {
                flag: "--bogus".to_string()
            }
        );
    }

    #[test]
    fn a_bare_dash_is_an_unknown_flag() {
        assert_eq!(
            usage(&["-"]),
            UsageError::UnknownFlag {
                flag: "-".to_string()
            }
        );
    }

    #[test]
    fn help_wins_over_every_other_argument() {
        assert_eq!(parse(["pwd", "-h"]).unwrap(), Action::Help);
        assert_eq!(parse(["--bogus", "-h"]).unwrap(), Action::Help);
    }

    #[test]
    fn version_wins_over_every_other_argument() {
        assert_eq!(parse(["pwd", "-v"]).unwrap(), Action::Version);
        assert_eq!(parse(["--bogus", "-v"]).unwrap(), Action::Version);
    }

    #[test]
    fn the_first_of_help_and_version_in_argument_order_wins() {
        assert_eq!(parse(["-h", "-v"]).unwrap(), Action::Help);
        assert_eq!(parse(["-v", "-h"]).unwrap(), Action::Version);
    }
}
