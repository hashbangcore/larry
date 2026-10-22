//! End-to-end tests over the compiled `larry` binary.

use std::io::{Read, Write};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Runs larry with the given arguments and no piped input.
fn larry(args: &[&str]) -> Output {
    run(args, None)
}

/// Runs larry with the given arguments, optionally piping input into it.
fn run(args: &[&str], input: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_larry"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("larry starts");

    if let Some(input) = input {
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(input.as_bytes())
            .expect("input is written");
    }

    child.wait_with_output().expect("larry finishes")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

#[test]
fn help_describes_the_options() {
    let output = larry(&["-h"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: larry [OPTIONS] [COMMAND]..."));
    assert!(stdout(&output).contains("--repeat <N>"));
}

#[test]
fn version_reports_the_package_version() {
    let output = larry(&["-v"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        format!("larry {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn a_single_command_produces_one_section() {
    let output = larry(&["printf hello"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "[section]\n[command]\nprintf hello\n[output]\nhello\n[end section]\n"
    );
}

#[test]
fn several_commands_produce_separated_sections() {
    let output = larry(&["printf one", "printf two"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "[section]\n[command]\nprintf one\n[output]\none\n[end section]\n\
         \n[section]\n[command]\nprintf two\n[output]\ntwo\n[end section]\n"
    );
}

#[test]
fn piped_input_becomes_a_section_before_the_commands() {
    let output = run(&["printf hello"], Some("payload\n"));

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "[section]\n[stdin]\npayload\n[end section]\n\
         \n[section]\n[command]\nprintf hello\n[output]\nhello\n[end section]\n"
    );
}

#[test]
fn empty_piped_input_produces_no_stdin_section() {
    let output = run(&["printf hello"], Some(""));

    assert!(output.status.success());
    assert!(!stdout(&output).contains("[stdin]"));
}

#[test]
fn whitespace_only_input_produces_no_stdin_section() {
    let output = run(&["printf hello"], Some("  \n\t "));

    assert!(output.status.success());
    assert!(!stdout(&output).contains("[stdin]"));
}

#[test]
fn repeat_runs_the_command_the_requested_number_of_times() {
    let output = larry(&["-r", "3", "printf hello"]);

    assert!(output.status.success());
    assert_eq!(stdout(&output).matches("[section]").count(), 3);
}

#[test]
fn repeat_applies_to_the_command_that_follows_it() {
    let output = larry(&["-r", "2", "printf one", "printf two"]);

    assert!(output.status.success());
    assert_eq!(stdout(&output).matches("[command]\nprintf one").count(), 2);
    assert_eq!(stdout(&output).matches("[command]\nprintf two").count(), 1);
}

#[test]
fn a_repeat_may_follow_its_command() {
    let output = larry(&["printf hello", "-r", "3"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output).matches("[command]\nprintf hello").count(),
        3
    );
}

#[test]
fn a_repeat_may_surround_its_command() {
    let output = larry(&["-r", "3", "printf hello"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output).matches("[command]\nprintf hello").count(),
        3
    );
}

#[test]
fn repeat_counts_can_be_mixed() {
    let output = larry(&["-r", "2", "printf one", "-r", "3", "printf two"]);

    assert!(output.status.success());
    assert_eq!(stdout(&output).matches("[command]\nprintf one").count(), 2);
    assert_eq!(stdout(&output).matches("[command]\nprintf two").count(), 3);
}

#[test]
fn a_command_that_fails_still_produces_a_section() {
    let output = larry(&["exit 3"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "[section]\n[command]\nexit 3\n[output]\n\n[end section]\n"
    );
}

#[test]
fn standard_error_is_reported_after_standard_output() {
    let output = larry(&["printf out; printf err >&2"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "[section]\n[command]\nprintf out; printf err >&2\n[output]\nouterr\n[end section]\n"
    );
}

#[test]
fn surrounding_whitespace_is_trimmed_from_the_output() {
    let output = larry(&["printf '  spaced  \\n'"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "[section]\n[command]\nprintf '  spaced  \\n'\n[output]\nspaced\n[end section]\n"
    );
}

#[test]
fn the_command_keeps_its_original_spacing() {
    let output = larry(&["printf 'a b'"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("[command]\nprintf 'a b'\n"));
}

#[test]
fn an_invalid_repeat_count_fails_with_a_clear_error() {
    let output = larry(&["-r", "abc", "printf hello"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid value 'abc'"));
    assert!(stderr(&output).contains("invalid repeat count 'abc'"));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_repeat_count_above_the_maximum_fails_with_a_clear_error() {
    let output = larry(&["-r", "101", "printf hello"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("repeat count must not exceed 100"));
    assert_eq!(stdout(&output), "");
}

#[test]
fn repeat_without_a_count_fails_with_a_clear_error() {
    let output = larry(&["-r"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("a value is required"));
    assert_eq!(stdout(&output), "");
}

#[test]
fn repeat_without_a_command_fails_with_a_clear_error() {
    let output = larry(&["-r", "2"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("-r requires a command"));
    assert_eq!(stdout(&output), "");
}

#[test]
fn an_unknown_flag_fails_with_a_clear_error() {
    let output = larry(&["--bogus"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("unexpected argument '--bogus'"));
    assert_eq!(stdout(&output), "");
}

#[test]
fn help_is_printed_on_request() {
    let output = larry(&["-h"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: larry"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn version_is_printed_on_request() {
    let output = larry(&["-v"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        format!("larry {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn no_arguments_produce_no_output() {
    let output = larry(&[]);

    assert!(output.status.success());
    assert_eq!(stdout(&output), "");
}

#[test]
fn an_endless_repetition_keeps_emitting_sections() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_larry"))
        .args(["-r", "0", "printf hello"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("larry starts");

    let mut pipe = child.stdout.take().expect("stdout is piped");
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let mut seen = Vec::new();
        let mut chunk = [0; 256];

        loop {
            match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    seen.extend_from_slice(&chunk[..read]);

                    if String::from_utf8_lossy(&seen).contains("[end section]") {
                        break;
                    }
                }
            }
        }

        let _ = sender.send(seen);
    });

    let seen = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("the endless repetition emits a section");

    let _ = child.kill();
    let _ = child.wait();

    assert!(String::from_utf8_lossy(&seen).contains("[command]\nprintf hello"));
}
