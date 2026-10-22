//! End-to-end tests over the compiled `larry` binary.

use std::io::Write;
use std::process::{Command, Output, Stdio};

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
    assert!(stdout(&output).contains("-r <N>"));
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
    assert_eq!(
        stderr(&output),
        "error: -r requires a valid number, got 'abc'\n"
    );
    assert!(stdout(&output).is_empty());
}

#[test]
fn a_repeat_count_above_the_maximum_fails_with_a_clear_error() {
    let output = larry(&["-r", "101", "printf hello"]);

    assert!(!output.status.success());
    assert_eq!(stderr(&output), "error: -r maximum is 100, got '101'\n");
    assert!(stdout(&output).is_empty());
}

#[test]
fn repeat_without_a_count_fails_with_a_clear_error() {
    let output = larry(&["-r"]);

    assert!(!output.status.success());
    assert_eq!(stderr(&output), "error: -r requires a repeat count\n");
}

#[test]
fn repeat_without_a_command_fails_with_a_clear_error() {
    let output = larry(&["-r", "2"]);

    assert!(!output.status.success());
    assert_eq!(stderr(&output), "error: -r requires a command\n");
}

#[test]
fn an_unknown_flag_fails_with_a_clear_error() {
    let output = larry(&["--bogus"]);

    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "error: unknown flag '--bogus'. Use 'larry -h' for usage.\n"
    );
}

#[test]
fn help_wins_over_an_invalid_repeat_count() {
    let output = larry(&["-r", "abc", "-h"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: larry"));
}

#[test]
fn no_arguments_produce_no_output() {
    let output = larry(&[]);

    assert!(output.status.success());
    assert!(stdout(&output).is_empty());
}
