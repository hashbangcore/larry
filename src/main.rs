use std::env;
use std::io::{self, IsTerminal, Read};
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut results = Vec::new();

    if !io::stdin().is_terminal() {
        let mut stdin_input = String::new();
        io::stdin().read_to_string(&mut stdin_input).unwrap();
        if !stdin_input.trim().is_empty() {
            results.push(format!("[section]\n[stdin]\n{}\n[end section]", stdin_input.trim()));
        }
    }

    for cmd_str in args {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd_str)
            .output()
            .expect("Command execution failed.");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let combined_output = if !stderr.is_empty() {
            format!("{}{}", stdout, stderr)
        } else {
            stdout.to_string()
        };

        results.push(format!(
            "[section]\n[command]\n{}\n[output]\n{}\n[end section]",
            cmd_str,
            combined_output.trim()
        ));
    }

    println!("{}", results.join("\n\n"));
}
