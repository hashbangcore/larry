use std::env;
use std::io::{self, IsTerminal, Read, Write};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

fn format_section(cmd: &str) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .expect("Command execution failed.");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined_output = if !stderr.is_empty() {
        format!("{}{}", stdout, stderr)
    } else {
        stdout.to_string()
    };

    format!(
        "[section]\n[command]\n{}\n[output]\n{}\n[end section]",
        cmd,
        combined_output.trim()
    )
}

fn print_help() {
    println!("Version {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage: larry [OPTIONS] [COMMAND]...");
    println!();
    println!("Capture piped stdin and execute commands, producing structured output");
    println!("with [section] markers.");
    println!();
    println!("Arguments:");
    println!("  [COMMAND]...    Commands to execute via sh -c");
    println!();
    println!("Options:");
    println!("  -r <N>          Repeat next command N times");
    println!("                  default: 1, max: 100, 0 = infinite loop with 5s delay");
    println!();
    println!("  -v              Show version and exit");
    println!();
    println!("  -h              Show this help message and exit");
    println!();
    println!("Each command and each repetition produces its own [section] in the output.");
    println!("Use quotes to pass flags to commands: larry \"ls -la\"");
    println!();
}

fn check_args(args: &[String]) {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-r" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: -r requires a repeat count");
                std::process::exit(1);
            }
            let n: usize = match args[i].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("error: -r requires a valid number, got '{}'", args[i]);
                    std::process::exit(1);
                }
            };
            if n > 100 {
                eprintln!("error: -r maximum is 100, got '{}'", n);
                std::process::exit(1);
            }
            i += 1;
            if i >= args.len() {
                eprintln!("error: -r requires a command");
                std::process::exit(1);
            }
            continue;
        }
        if args[i].starts_with('-') {
            eprintln!(
                "error: unknown flag '{}'. Use 'larry -h' for usage.",
                args[i]
            );
            std::process::exit(1);
        }
        i += 1;
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    for arg in &args {
        if arg == "-h" {
            print_help();
            return;
        }
        if arg == "-v" {
            println!("larry {}", env!("CARGO_PKG_VERSION"));
            return;
        }
    }

    check_args(&args);

    let mut results = Vec::new();

    if !io::stdin().is_terminal() {
        let mut stdin_input = String::new();
        io::stdin().read_to_string(&mut stdin_input).unwrap();
        if !stdin_input.trim().is_empty() {
            results.push(format!(
                "[section]\n[stdin]\n{}\n[end section]",
                stdin_input.trim()
            ));
        }
    }

    let mut i = 0;
    let mut repeat: usize = 1;

    while i < args.len() {
        if args[i] == "-r" {
            i += 1;
            repeat = args[i].parse().unwrap();
            i += 1;
            continue;
        }

        let cmd = &args[i];

        if repeat == 0 {
            loop {
                println!("{}", format_section(cmd));
                io::stdout().flush().unwrap();
                sleep(Duration::from_secs(5));
            }
        }

        for _ in 0..repeat {
            results.push(format_section(cmd));
        }

        repeat = 1;
        i += 1;
    }

    println!("{}", results.join("\n\n"));
}
