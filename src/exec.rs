//! Execution of commands through the system shell.

use std::process::Command;

use crate::error::Error;

/// What a command wrote to its standard streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Text written to standard output.
    pub stdout: String,
    /// Text written to standard error.
    pub stderr: String,
}

impl CommandOutput {
    /// Both streams joined in the order they were produced.
    ///
    /// Standard output comes first, then standard error, with nothing between
    /// them, mirroring how the shell reports them.
    pub fn combined(&self) -> String {
        let mut combined = String::with_capacity(self.stdout.len() + self.stderr.len());
        combined.push_str(&self.stdout);
        combined.push_str(&self.stderr);
        combined
    }
}

/// Runs `command` through `sh -c` and captures what it writes.
///
/// The exit status of the command is not reported: larry reports what the
/// command wrote, not how it ended.
pub fn run(command: &str) -> Result<CommandOutput, Error> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|source| Error::Spawn {
            command: command.to_string(),
            source,
        })?;

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_standard_output() {
        let output = run("printf hello").expect("command runs");

        assert_eq!(output.stdout, "hello");
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn captures_standard_error() {
        let output = run("printf oops >&2").expect("command runs");

        assert_eq!(output.stdout, "");
        assert_eq!(output.stderr, "oops");
    }

    #[test]
    fn combined_output_keeps_both_streams() {
        let output = run("printf out; printf err >&2").expect("command runs");

        assert_eq!(output.combined(), "outerr");
    }

    #[test]
    fn a_failing_command_still_produces_output() {
        let output = run("printf partial; exit 3").expect("command runs");

        assert_eq!(output.stdout, "partial");
    }
}
