#![forbid(unsafe_code)]

use s3_model::{build_models_from_knowledge, read_knowledge_json, write_models_json};
use std::env;
use std::process;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = args.next(),
            "--output" => output = args.next(),
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let input = input.ok_or_else(|| "missing required argument: --input <path>".to_owned())?;
    let output = output.ok_or_else(|| "missing required argument: --output <path>".to_owned())?;

    let knowledge = read_knowledge_json(input)?;
    let models = build_models_from_knowledge(&knowledge)?;
    write_models_json(output, &models)
}
