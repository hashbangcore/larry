//! Streaming of sections to the output.

use std::io::{self, Write};

use crate::section::Section;

/// Writes sections one after another, separated by a blank line.
pub struct Emitter<'a> {
    out: &'a mut dyn Write,
    wrote_any: bool,
}

impl<'a> Emitter<'a> {
    /// Creates an emitter writing to `out`.
    pub fn new(out: &'a mut dyn Write) -> Self {
        Emitter {
            out,
            wrote_any: false,
        }
    }

    /// Writes one section and flushes it.
    pub fn write(&mut self, section: &Section) -> io::Result<()> {
        if self.wrote_any {
            self.out.write_all(b"\n")?;
        }
        self.wrote_any = true;

        write!(self.out, "{section}")?;
        self.out.write_all(b"\n")?;
        self.out.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit(sections: &[Section]) -> String {
        let mut buffer = Vec::new();
        let mut emitter = Emitter::new(&mut buffer);

        for section in sections {
            emitter.write(section).expect("write succeeds");
        }

        String::from_utf8(buffer).expect("valid utf-8")
    }

    #[test]
    fn no_sections_produce_no_output() {
        assert_eq!(emit(&[]), "");
    }

    #[test]
    fn a_single_section_ends_with_a_newline() {
        let section = Section::Stdin {
            text: "payload".to_string(),
        };

        assert_eq!(
            emit(&[section]),
            "[section]\n[stdin]\npayload\n[end section]\n"
        );
    }

    #[test]
    fn consecutive_sections_are_separated_by_a_blank_line() {
        let first = Section::Stdin {
            text: "in".to_string(),
        };
        let second = Section::Command {
            command: "pwd".to_string(),
            output: "/tmp".to_string(),
        };

        assert_eq!(
            emit(&[first, second]),
            "[section]\n[stdin]\nin\n[end section]\n\
             \n[section]\n[command]\npwd\n[output]\n/tmp\n[end section]\n"
        );
    }
}
