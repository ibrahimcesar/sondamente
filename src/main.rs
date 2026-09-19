use std::process::ExitCode;

use sondamente::{load, spec_hash, validate};

const USAGE: &str = "\
sondamente: preregistered probes for philosophy-of-mind claims about LLMs

USAGE:
    sondamente validate <SPEC.yaml>...   Check probe specs (exit 1 if any has errors)
    sondamente hash <SPEC.yaml>          Print the spec's preregistration hash
    sondamente --help | --version
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("validate") if args.len() > 1 => cmd_validate(&args[1..]),
        Some("hash") if args.len() == 2 => cmd_hash(&args[1]),
        Some("-h" | "--help") => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some("-V" | "--version") => {
            println!("sondamente {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn cmd_validate(paths: &[String]) -> ExitCode {
    let mut failed = false;
    let mut unreadable = false;
    for path in paths {
        let spec = match load(path) {
            Ok(spec) => spec,
            Err(e) => {
                eprintln!("{path}: {e}");
                unreadable = true;
                continue;
            }
        };
        let report = validate(&spec);
        for diagnostic in &report.diagnostics {
            println!("{path}: {diagnostic}");
        }
        let errors = report.errors().count();
        let warnings = report.warnings().count();
        if errors > 0 {
            failed = true;
            println!("{path}: FAILED ({errors} error(s), {warnings} warning(s))");
        } else {
            println!("{path}: ok ({warnings} warning(s))");
        }
    }
    if unreadable {
        ExitCode::from(2)
    } else if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_hash(path: &str) -> ExitCode {
    match load(path) {
        Ok(spec) => {
            println!("{}", spec_hash(&spec));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            ExitCode::from(2)
        }
    }
}
