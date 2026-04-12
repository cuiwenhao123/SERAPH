#![forbid(unsafe_code)]

use s3_extract::{build_minimal_knowledge, write_knowledge_json};
use std::env;
use std::process;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut crate_name: Option<String> = None;
    let mut output: Option<String> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--crate" => {
                crate_name = args.next();
            }
            "--output" => {
                output = args.next();
            }
            _ => {
                return Err(format!("unknown argument: {arg}"));
            }
        }
    }

    let crate_name = crate_name.ok_or_else(|| "missing required argument: --crate <name>".to_owned())?;
    let output = output.ok_or_else(|| "missing required argument: --output <path>".to_owned())?;

    let knowledge = build_minimal_knowledge(&crate_name);
    write_knowledge_json(&output, &knowledge)
        .map_err(|err| format!("failed to write knowledge JSON: {err}"))?;
    Ok(())
}
