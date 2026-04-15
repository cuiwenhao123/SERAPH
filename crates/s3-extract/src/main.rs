#![forbid(unsafe_code)]

use s3_extract::{build_minimal_knowledge, extract_knowledge_from_manifest, write_knowledge_json};
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
    let mut manifest_path: Option<String> = None;
    let mut output: Option<String> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--crate" => {
                crate_name = args.next();
            }
            "--manifest-path" => {
                manifest_path = args.next();
            }
            "--output" => {
                output = args.next();
            }
            _ => {
                return Err(format!("unknown argument: {arg}"));
            }
        }
    }

    let output = output.ok_or_else(|| "missing required argument: --output <path>".to_owned())?;

    let knowledge = if let Some(manifest_path) = manifest_path {
        extract_knowledge_from_manifest(&manifest_path)
            .map_err(|err| format!("failed to extract knowledge from manifest: {err}"))?
    } else if let Some(crate_name) = crate_name {
        build_minimal_knowledge(&crate_name)
    } else {
        return Err(
            "missing required argument: --manifest-path <path> or --crate <name>".to_owned(),
        );
    };

    write_knowledge_json(&output, &knowledge)
        .map_err(|err| format!("failed to write knowledge JSON: {err}"))?;
    Ok(())
}
