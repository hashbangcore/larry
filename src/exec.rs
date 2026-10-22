//! Execution of commands through the system shell.

use std::process::Command;

use crate::error::Error;

/// What a command wrote to its standard streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandOutput {
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
    pub(crate) fn combined(&self) -> String {
        let mut combined = String::with_capacity(self.stdout.len() + self.stderr.len());
        combined.push_str(&self.stdout);
        combined.push_str(&self.stderr);
        combined
    }

    /// Both streams joined, without the whitespace that surrounds them.
    ///
    /// The whitespace is cut in place, so the text is copied once at most.
    pub(crate) fn combined_trimmed(&self) -> String {
        let mut combined = self.combined();

        let start = combined.len() - combined.trim_start().len();
        let end = combined.trim_end().len();

        if start == 0 && end == combined.len() {
            return combined;
        }

        let _ = combined.drain(..start);
        combined.truncate(end - start);

        combined
    }
}

/// Runs `command` through `sh -c` and captures what it writes.
///
/// The exit status of the command is not reported: larry reports what the
/// command wrote, not how it ended.
///
/// # Errors
///
/// Returns an error when the command cannot be started.
pub(crate) fn run(command: &str) -> Result<CommandOutput, Error> {
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
    fn combined_output_is_trimmed() {
        let output = run("printf '  spaced  \\n'").expect("command runs");

        assert_eq!(output.combined_trimmed(), "spaced");
    }

    #[test]
    fn trimmed_output_keeps_the_whitespace_inside() {
        let output = run("printf 'a b'").expect("command runs");

        assert_eq!(output.combined_trimmed(), "a b");
    }

    #[test]
    fn trimming_does_not_join_the_streams() {
        let output = run("printf 'out\\n'; printf 'err\\n' >&2").expect("command runs");

        assert_eq!(output.combined_trimmed(), "out\nerr");
    }

    #[test]
    fn a_failing_command_still_produces_output() {
        let output = run("printf partial; exit 3").expect("command runs");

        assert_eq!(output.stdout, "partial");
    }
}
