//! Orchestration of a plan: stdin capture, command execution, streaming.

use std::io::{self, IsTerminal, Read, Write};
use std::thread::sleep;
use std::time::Duration;

use crate::cli::{Invocation, Plan, Repeat};
use crate::error::Error;
use crate::exec;
use crate::output::Emitter;
use crate::section::Section;

/// Runs the plan, capturing piped input and writing sections to `out`.
pub fn run(plan: &Plan, out: &mut dyn Write) -> Result<(), Error> {
    let stdin = capture_stdin()?;
    execute(plan, stdin, out, &mut |duration| sleep(duration))
}

/// Runs the plan with the given piped input, pacing endless repetitions.
///
/// `pace` is called between two consecutive repetitions of an endless
/// invocation, never before the first one.
pub fn execute(
    plan: &Plan,
    stdin: Option<String>,
    out: &mut dyn Write,
    pace: &mut dyn FnMut(Duration),
) -> Result<(), Error> {
    let mut emitter = Emitter::new(out);

    if let Some(text) = stdin {
        emitter
            .write(&Section::Stdin { text })
            .map_err(Error::Write)?;
    }

    for invocation in &plan.invocations {
        run_invocation(invocation, &mut emitter, pace)?;
    }

    Ok(())
}

fn run_invocation(
    invocation: &Invocation,
    emitter: &mut Emitter<'_>,
    pace: &mut dyn FnMut(Duration),
) -> Result<(), Error> {
    match invocation.repeat {
        Repeat::Count(count) => {
            for _ in 0..count.value() {
                emit_command(invocation, emitter)?;
            }
        }
        Repeat::Forever { interval } => loop {
            emit_command(invocation, emitter)?;
            pace(interval);
        },
    }

    Ok(())
}

fn emit_command(invocation: &Invocation, emitter: &mut Emitter<'_>) -> Result<(), Error> {
    let output = exec::run(&invocation.command)?;

    let section = Section::Command {
        command: invocation.command.clone(),
        output: output.combined().trim().to_string(),
    };

    emitter.write(&section).map_err(Error::Write)
}

/// Reads piped standard input, if there is any.
///
/// Returns `None` when standard input is a terminal or carries nothing but
/// whitespace.
pub fn capture_stdin() -> Result<Option<String>, Error> {
    let stdin = io::stdin();

    if stdin.is_terminal() {
        return Ok(None);
    }

    let mut buffer = Vec::new();
    stdin.lock().read_to_end(&mut buffer).map_err(Error::Read)?;

    Ok(non_empty_trimmed(&buffer))
}

fn non_empty_trimmed(buffer: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(buffer);
    let trimmed = text.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Repeat, RepeatCount};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Emit,
        Pace(Duration),
    }

    #[derive(Clone)]
    struct Recorder {
        events: Rc<RefCell<Vec<Event>>>,
        written: Rc<RefCell<String>>,
        emitted: Rc<RefCell<usize>>,
        limit: usize,
    }

    impl Recorder {
        fn new(limit: usize) -> Self {
            Recorder {
                events: Rc::new(RefCell::new(Vec::new())),
                written: Rc::new(RefCell::new(String::new())),
                emitted: Rc::new(RefCell::new(0)),
                limit,
            }
        }

        fn emits(&self) -> usize {
            *self.emitted.borrow()
        }

        fn written(&self) -> String {
            self.written.borrow().clone()
        }
    }

    impl Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let text = std::str::from_utf8(buf).expect("valid utf-8");
            self.written.borrow_mut().push_str(text);

            if text.contains("[section]") {
                let mut emitted = self.emitted.borrow_mut();

                if *emitted >= self.limit {
                    return Err(io::Error::other("recorder limit reached"));
                }

                *emitted += 1;
                self.events.borrow_mut().push(Event::Emit);
            }

            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn plan(invocations: Vec<Invocation>) -> Plan {
        Plan { invocations }
    }

    fn invocation(command: &str, repeat: Repeat) -> Invocation {
        Invocation {
            command: command.to_string(),
            repeat,
        }
    }

    #[test]
    fn a_counted_invocation_emits_one_section_per_repetition() {
        let mut recorder = Recorder::new(100);
        let plan = plan(vec![invocation(
            "printf hello",
            Repeat::Count(RepeatCount::new(3).expect("valid count")),
        )]);

        execute(&plan, None, &mut recorder, &mut |_| {}).expect("run succeeds");

        assert_eq!(recorder.emits(), 3);
    }

    #[test]
    fn an_endless_invocation_paces_between_repetitions() {
        let mut recorder = Recorder::new(3);
        let events = Rc::clone(&recorder.events);
        let interval = Duration::from_millis(10);
        let plan = plan(vec![invocation(
            "printf hello",
            Repeat::Forever { interval },
        )]);

        let result = execute(&plan, None, &mut recorder, &mut |duration| {
            events.borrow_mut().push(Event::Pace(duration));
        });

        assert!(result.is_err(), "the recorder stops the endless loop");
        assert_eq!(
            *recorder.events.borrow(),
            vec![
                Event::Emit,
                Event::Pace(interval),
                Event::Emit,
                Event::Pace(interval),
                Event::Emit,
                Event::Pace(interval),
            ]
        );
    }

    #[test]
    fn piped_input_becomes_a_section_before_the_commands() {
        let mut recorder = Recorder::new(100);
        let plan = plan(vec![invocation(
            "printf hello",
            Repeat::Count(RepeatCount::ONE),
        )]);

        execute(
            &plan,
            Some("payload".to_string()),
            &mut recorder,
            &mut |_| {},
        )
        .expect("run succeeds");

        assert_eq!(
            recorder.written(),
            "[section]\n[stdin]\npayload\n[end section]\n\
             \n[section]\n[command]\nprintf hello\n[output]\nhello\n[end section]\n"
        );
    }

    #[test]
    fn whitespace_only_input_produces_no_section() {
        assert_eq!(non_empty_trimmed(b"  \n\t "), None);
    }

    #[test]
    fn input_is_trimmed() {
        assert_eq!(
            non_empty_trimmed(b"  payload\n"),
            Some("payload".to_string())
        );
    }

    #[test]
    fn invalid_utf8_input_is_kept_as_replacement_characters() {
        assert_eq!(
            non_empty_trimmed(b"caf\xff"),
            Some("caf\u{fffd}".to_string())
        );
    }
}
