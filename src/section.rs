//! The labeled blocks that make up larry's output.

use std::fmt;

/// A block of output, labeled by where it came from.
///
/// The command text is borrowed from the plan that produced it, so a command
/// repeated many times does not copy its own text over and over.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Section<'cmd> {
    /// Text captured from standard input.
    Stdin {
        /// The captured text.
        text: String,
    },
    /// The result of running a command.
    Command {
        /// The command that ran, verbatim.
        command: &'cmd str,
        /// Everything the command wrote.
        output: String,
    },
}

impl fmt::Display for Section<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Section::Stdin { text } => {
                write!(f, "[section]\n[stdin]\n{text}\n[end section]")
            }
            Section::Command { command, output } => {
                write!(
                    f,
                    "[section]\n[command]\n{command}\n[output]\n{output}\n[end section]"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stdin_section_labels_its_text() {
        let section = Section::Stdin {
            text: "payload".to_string(),
        };

        assert_eq!(
            section.to_string(),
            "[section]\n[stdin]\npayload\n[end section]"
        );
    }

    #[test]
    fn a_command_section_labels_its_command_and_output() {
        let section = Section::Command {
            command: "printf hello",
            output: "hello".to_string(),
        };

        assert_eq!(
            section.to_string(),
            "[section]\n[command]\nprintf hello\n[output]\nhello\n[end section]"
        );
    }

    #[test]
    fn an_empty_output_keeps_its_own_line() {
        let section = Section::Command {
            command: "true",
            output: String::new(),
        };

        assert_eq!(
            section.to_string(),
            "[section]\n[command]\ntrue\n[output]\n\n[end section]"
        );
    }
}
