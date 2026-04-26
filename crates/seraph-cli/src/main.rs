#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use s3_coverage::write_coverage_from_phase3;
use seraph_cli::{
    display_command, phase2_graph_plan, phase2_index_plan, phase2_retrieve_plan,
    phase2_targets_plan, phase3_afl_bootstrap_plan, phase3_compile_check_plan,
    phase3_fix_acceptance_plan, phase3_fix_acceptance_write_batch_plan,
    phase3_fix_acceptance_write_plan, phase3_fix_loop_batch_plan, phase3_fix_loop_plan,
    phase3_fix_once_plan, phase3_fixer_bundle_plan, phase3_fixer_write_plan,
    phase3_harness_prompt_plan, phase3_harness_write_plan, phase3_model_response_plan,
    phase3_runtime_diagnose_plan, phase3_smoke_run_plan, run_layout, CommandPlan,
};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScheduledTarget {
    round: u32,
    api_id: Option<String>,
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_usage();
        return Ok(());
    }
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let filtered: Vec<String> = args.into_iter().filter(|arg| arg != "--dry-run").collect();
    if filtered.first().map(String::as_str) == Some("run") {
        return run_pipeline(&filtered[1..], dry_run);
    }
    let acceptance_write_sync = parse_fix_acceptance_sync(&filtered)?;
    let plan = parse_plan(&filtered)?;
    if dry_run {
        println!("{}", display_command(&plan));
        if let Some(sync) = acceptance_write_sync {
            println!(
                "# coverage sync -> {} (context {})",
                sync.coverage, sync.context
            );
        }
        return Ok(());
    }
    execute(plan)?;
    if let Some(sync) = acceptance_write_sync {
        let coverage = write_coverage_from_phase3(
            Path::new(&sync.coverage),
            Path::new(&sync.context),
            None,
            None,
            None,
            Some(Path::new(&sync.acceptance_index)),
        )?;
        println!(
            "Coverage state written to {} ({:?})",
            sync.coverage, coverage.status
        );
    }
    Ok(())
}

fn run_pipeline(args: &[String], dry_run: bool) -> Result<(), String> {
    let workspace = optional_value(args, "--workspace-dir").unwrap_or("workspace");
    let start_round = optional_value(args, "--round")
        .unwrap_or("1")
        .parse::<u32>()
        .map_err(|_| "--round must be an integer".to_string())?;
    let knowledge_arg = optional_value(args, "--knowledge");
    let manifest_arg = optional_value(args, "--manifest-path");
    if knowledge_arg.is_some() == manifest_arg.is_some() {
        return Err("choose exactly one input: --knowledge or --manifest-path".to_string());
    }
    let phase3_prompt = has_flag(args, "--phase3-prompt")
        || optional_value(args, "--llm-response").is_some()
        || optional_value(args, "--model-command").is_some();
    let fix_loop = has_flag(args, "--fix-loop");
    let fixer_bundle = has_flag(args, "--fixer-bundle") || fix_loop;
    let smoke_command = optional_value(args, "--smoke-command");
    let smoke_run = smoke_command.is_some();
    let compile_check = has_flag(args, "--compile-check") || fixer_bundle || smoke_run;
    let variants = optional_value(args, "--variants").unwrap_or("3");
    let phase3_style = optional_value(args, "--phase3-style").unwrap_or("aflpp");
    let target_api_id = optional_value(args, "--target-api-id");
    let llm_response = optional_value(args, "--llm-response");
    let model_command = optional_value(args, "--model-command");
    let compile_command = optional_value(args, "--compile-command");
    let fix_max_attempts = optional_value(args, "--fix-max-attempts").unwrap_or("2");
    let layout = run_layout(workspace, start_round);
    let fix_responses_dir = optional_value(args, "--fix-responses-dir").unwrap_or(&layout.fix_dir);
    let fix_model_command = optional_value(args, "--fix-model-command");
    let runtime_model_command = optional_value(args, "--runtime-model-command");
    let afl_bootstrap = has_flag(args, "--afl-bootstrap");
    let afl_harness = optional_value(args, "--afl-harness");
    let afl_input_mode = optional_value(args, "--afl-input-mode");
    let afl_corpus_dir = optional_value(args, "--afl-corpus-dir");
    let afl_findings_dir = optional_value(args, "--afl-findings-dir");
    let afl_target_dir = optional_value(args, "--afl-target-dir");
    let afl_release = has_flag(args, "--afl-release");
    let afl_build_only = has_flag(args, "--afl-build-only");
    let batch_mode = target_api_id.is_none();
    let effective_llm_response =
        llm_response.or_else(|| model_command.map(|_| layout.llm_response.as_str()));
    let runtime_diagnose_command = runtime_model_command
        .or(fix_model_command)
        .or(model_command);
    let runtime_diagnose = smoke_run && runtime_diagnose_command.is_some();
    let knowledge = knowledge_arg.unwrap_or(&layout.knowledge_out);

    if batch_mode && llm_response.is_some() {
        return Err(
            "--llm-response is single-target only; use --model-command for batch runs or pass --target-api-id".to_string(),
        );
    }
    if batch_mode && afl_bootstrap && afl_harness.is_none() {
        return Err(
            "--afl-bootstrap without --afl-harness is ambiguous in batch mode; pass --target-api-id or --afl-harness".to_string(),
        );
    }

    if !dry_run {
        create_run_dirs(
            &layout,
            phase3_prompt,
            effective_llm_response.is_some(),
            fix_loop,
            compile_check,
            fixer_bundle,
        )?;
    }

    if let Some(manifest) = manifest_arg {
        run_or_print(
            CommandPlan::new(
                "cargo",
                vec![
                    "run".into(),
                    "-p".into(),
                    "s3-extract".into(),
                    "--".into(),
                    "--manifest-path".into(),
                    manifest.into(),
                    "--output".into(),
                    layout.knowledge_out.clone(),
                ],
            ),
            dry_run,
        )?;
    }

    if !dry_run {
        if let Some(crate_config_path) = maybe_write_crate_config(knowledge, workspace)? {
            println!("Phase 3 crate config written to {}", crate_config_path);
        }
    }

    run_or_print(phase2_index_plan(knowledge, &layout.vectordb), dry_run)?;
    run_or_print(phase2_graph_plan(knowledge, &layout.graph), dry_run)?;
    let scheduled_targets = resolve_scheduled_targets(
        &layout.graph,
        start_round,
        target_api_id,
        dry_run,
    )?;

    if dry_run && batch_mode {
        println!(
            "# batch mode -> commands below show the first per-target template; real run iterates all ranked targets starting from round {}",
            start_round
        );
    }

    for scheduled in &scheduled_targets {
        let round = scheduled.round;
        let layout = run_layout(workspace, round);
        let effective_llm_response = llm_response.or_else(|| model_command.map(|_| layout.llm_response.as_str()));

        run_or_print(
            phase2_retrieve_plan(
                knowledge,
                &layout.graph,
                &layout.vectordb,
                &round.to_string(),
                &layout.context,
                scheduled.api_id.as_deref(),
            ),
            dry_run,
        )?;

        if phase3_prompt {
            run_or_print(
                phase3_harness_prompt_plan(&layout.context, &layout.prompt, variants, phase3_style),
                dry_run,
            )?;
        }
        if let Some(command) = model_command {
            run_or_print(
                phase3_model_response_plan(&layout.prompt, &layout.llm_response, command, None),
                dry_run,
            )?;
        }
        if let Some(response) = effective_llm_response {
            run_or_print(
                phase3_harness_write_plan(
                    &layout.prompt,
                    response,
                    &layout.fuzz_dir,
                    &round.to_string(),
                ),
                dry_run,
            )?;
        }
        if compile_check {
            run_or_print(
                phase3_compile_check_plan(
                    &layout.harness_glob,
                    &layout.report_dir,
                    &round.to_string(),
                    compile_command,
                ),
                dry_run,
            )?;
        }
        if fixer_bundle {
            run_or_print(
                phase3_fixer_bundle_plan(&layout.compile_index, &layout.context, &layout.fix_dir),
                dry_run,
            )?;
        }
        if fix_loop {
            run_or_print(
                phase3_fix_loop_batch_plan(
                    &layout.fix_request_glob,
                    fix_responses_dir,
                    &layout.fuzz_dir,
                    &layout.report_dir,
                    &layout.fix_loop_index,
                    fix_max_attempts,
                    compile_command,
                    fix_model_command,
                ),
                dry_run,
            )?;
        }
        if let Some(command) = smoke_command {
            run_or_print(
                phase3_smoke_run_plan(
                    &layout.compile_index,
                    if fix_loop {
                        Some(&layout.fix_loop_index)
                    } else {
                        None
                    },
                    &layout.report_dir,
                    &round.to_string(),
                    command,
                ),
                dry_run,
            )?;
        }
        if let Some(command) = runtime_diagnose_command.filter(|_| runtime_diagnose) {
            run_or_print(
                phase3_runtime_diagnose_plan(
                    &layout.context,
                    &layout.smoke_index,
                    &layout.report_dir,
                    &round.to_string(),
                    command,
                ),
                dry_run,
            )?;
        }

        if !dry_run {
            println!("RAG context written to {}", layout.context);
            if phase3_prompt {
                println!("Phase 3 harness prompt written to {}", layout.prompt);
            }
            if model_command.is_some() {
                println!("Phase 3 model response written to {}", layout.llm_response);
            }
            if effective_llm_response.is_some() {
                println!("Phase 3 harness files written under {}", layout.fuzz_dir);
            }
            if compile_check {
                println!(
                    "Phase 3 compile report index written to {}",
                    layout.compile_index
                );
            }
            if fixer_bundle {
                println!("Phase 3 fixer requests written under {}", layout.fix_dir);
            }
            if fix_loop {
                println!(
                    "Phase 3 fix-loop index written to {}",
                    layout.fix_loop_index
                );
            }
            if smoke_run {
                println!("Phase 3 smoke index written to {}", layout.smoke_index);
            }
            if runtime_diagnose {
                println!(
                    "Phase 3 runtime diagnosis index written to {}",
                    layout.runtime_error_index
                );
            }
            if phase3_prompt || compile_check || fix_loop {
                let coverage = write_coverage_from_phase3(
                    Path::new(&layout.coverage),
                    Path::new(&layout.context),
                    if compile_check {
                        Some(Path::new(&layout.compile_index))
                    } else {
                        None
                    },
                    if fix_loop {
                        Some(Path::new(&layout.fix_loop_index))
                    } else {
                        None
                    },
                    if smoke_run {
                        Some(Path::new(&layout.smoke_index))
                    } else {
                        None
                    },
                    if runtime_diagnose {
                        Some(Path::new(&layout.runtime_error_index))
                    } else {
                        None
                    },
                )?;
                println!(
                    "Coverage state written to {} ({:?})",
                    layout.coverage, coverage.status
                );
            }
        }
    }

    if !dry_run && afl_bootstrap {
        let layout = run_layout(workspace, start_round);
        let selected_harness = select_afl_harness(&layout, afl_harness).map_err(|message| {
            format!("failed to select harness for AFL bootstrap: {message}")
        })?;
        println!(
            "Phase 3 AFL bootstrap harness selected: {}",
            selected_harness
        );
        execute(phase3_afl_bootstrap_plan(
            workspace,
            &selected_harness,
            afl_input_mode,
            afl_corpus_dir,
            afl_findings_dir,
            afl_target_dir,
            afl_release,
            afl_build_only,
        ))?;
    } else if afl_bootstrap {
        if let Some(harness) = afl_harness {
            run_or_print(
                phase3_afl_bootstrap_plan(
                    workspace,
                    harness,
                    afl_input_mode,
                    afl_corpus_dir,
                    afl_findings_dir,
                    afl_target_dir,
                    afl_release,
                    afl_build_only,
                ),
                true,
            )?;
        } else {
            println!(
                "# afl-bootstrap -> runtime auto-select from successful fixed/original harness"
            );
        }
    }
    Ok(())
}

fn resolve_scheduled_targets(
    graph_path: &str,
    start_round: u32,
    target_api_id: Option<&str>,
    dry_run: bool,
) -> Result<Vec<ScheduledTarget>, String> {
    if let Some(target_api_id) = target_api_id {
        return Ok(vec![ScheduledTarget {
            round: start_round,
            api_id: Some(target_api_id.to_string()),
        }]);
    }

    if dry_run {
        return Ok(vec![ScheduledTarget {
            round: start_round,
            api_id: None,
        }]);
    }

    let (stdout, _) = execute_capture(phase2_targets_plan(graph_path))?;
    let mut scheduled = Vec::new();
    for (index, api_id) in parse_ranked_target_ids(&stdout).into_iter().enumerate() {
        let round = (index as u32) + 1;
        if round < start_round {
            continue;
        }
        scheduled.push(ScheduledTarget {
            round,
            api_id: Some(api_id),
        });
    }

    if scheduled.is_empty() {
        return Err(format!(
            "round {} requested target index {}, but no ranked unsafe targets remain",
            start_round, start_round
        ));
    }

    Ok(scheduled)
}

fn parse_ranked_target_ids(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.split('\t').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn create_run_dirs(
    layout: &seraph_cli::RunLayout,
    phase3_prompt: bool,
    has_llm_response: bool,
    has_fix_loop: bool,
    compile_check: bool,
    fixer_bundle: bool,
) -> Result<(), String> {
    fs::create_dir_all(
        layout
            .context
            .rsplit_once('/')
            .map(|pair| pair.0)
            .unwrap_or("."),
    )
    .map_err(|error| format!("failed to create context directory: {error}"))?;
    if phase3_prompt {
        fs::create_dir_all(
            layout
                .prompt
                .rsplit_once('/')
                .map(|pair| pair.0)
                .unwrap_or("."),
        )
        .map_err(|error| format!("failed to create prompt directory: {error}"))?;
    }
    if has_llm_response || has_fix_loop {
        fs::create_dir_all(&layout.fuzz_dir)
            .map_err(|error| format!("failed to create fuzz directory: {error}"))?;
    }
    if compile_check {
        fs::create_dir_all(&layout.report_dir)
            .map_err(|error| format!("failed to create report directory: {error}"))?;
    }
    if fixer_bundle {
        fs::create_dir_all(&layout.fix_dir)
            .map_err(|error| format!("failed to create fix directory: {error}"))?;
    }
    Ok(())
}

fn run_or_print(plan: CommandPlan, dry_run: bool) -> Result<(), String> {
    if dry_run {
        println!("{}", display_command(&plan));
        Ok(())
    } else {
        execute(plan)
    }
}

fn parse_plan(args: &[String]) -> Result<CommandPlan, String> {
    match args.first().map(String::as_str) {
        Some("phase2") => parse_phase2(&args[1..]),
        Some("phase3") => parse_phase3(&args[1..]),
        Some(other) => Err(format!("unknown command: {other}")),
        None => Err("missing command".to_string()),
    }
}

fn parse_phase2(args: &[String]) -> Result<CommandPlan, String> {
    match args.first().map(String::as_str) {
        Some("index") => Ok(phase2_index_plan(
            required_value(args, "--knowledge")?,
            required_value(args, "--vectordb")?,
        )),
        Some("graph") => Ok(phase2_graph_plan(
            required_value(args, "--knowledge")?,
            required_value(args, "--graph")?,
        )),
        Some("targets") => Ok(phase2_targets_plan(required_value(args, "--graph")?)),
        Some("retrieve") => Ok(phase2_retrieve_plan(
            required_value(args, "--knowledge")?,
            required_value(args, "--graph")?,
            required_value(args, "--vectordb")?,
            optional_value(args, "--round").unwrap_or("1"),
            required_value(args, "--output")?,
            optional_value(args, "--target-api-id"),
        )),
        Some(other) => Err(format!("unknown phase2 command: {other}")),
        None => Err("missing phase2 command".to_string()),
    }
}

fn parse_phase3(args: &[String]) -> Result<CommandPlan, String> {
    match args.first().map(String::as_str) {
        Some("harness-prompt") => Ok(phase3_harness_prompt_plan(
            required_value(args, "--context")?,
            required_value(args, "--output")?,
            optional_value(args, "--variants").unwrap_or("3"),
            optional_value(args, "--style").unwrap_or("aflpp"),
        )),
        Some("harness-write") => Ok(phase3_harness_write_plan(
            required_value(args, "--prompt")?,
            required_value(args, "--response")?,
            required_value(args, "--output-dir")?,
            required_value(args, "--round")?,
        )),
        Some("compile-check") => Ok(phase3_compile_check_plan(
            required_value(args, "--harness-glob")?,
            required_value(args, "--report-dir")?,
            required_value(args, "--round")?,
            optional_value(args, "--command-template"),
        )),
        Some("smoke-run") => Ok(phase3_smoke_run_plan(
            required_value(args, "--compile-index")?,
            optional_value(args, "--fix-loop-index"),
            required_value(args, "--report-dir")?,
            required_value(args, "--round")?,
            required_value(args, "--command-template")?,
        )),
        Some("runtime-diagnose") => Ok(phase3_runtime_diagnose_plan(
            required_value(args, "--context")?,
            required_value(args, "--smoke-index")?,
            required_value(args, "--output-dir")?,
            required_value(args, "--round")?,
            required_value(args, "--command-template")?,
        )),
        Some("fixer-bundle") => Ok(phase3_fixer_bundle_plan(
            required_value(args, "--compile-index")?,
            required_value(args, "--context")?,
            required_value(args, "--output-dir")?,
        )),
        Some("fixer-write") => Ok(phase3_fixer_write_plan(
            required_value(args, "--request")?,
            required_value(args, "--response")?,
            required_value(args, "--output-dir")?,
            required_value(args, "--attempt")?,
        )),
        Some("fix-once") => Ok(phase3_fix_once_plan(
            required_value(args, "--request")?,
            required_value(args, "--response")?,
            required_value(args, "--output-dir")?,
            required_value(args, "--report-dir")?,
            required_value(args, "--attempt")?,
            optional_value(args, "--command-template"),
        )),
        Some("fix-loop") => Ok(phase3_fix_loop_plan(
            required_value(args, "--request")?,
            required_value(args, "--responses-dir")?,
            required_value(args, "--output-dir")?,
            required_value(args, "--report-dir")?,
            required_value(args, "--max-attempts")?,
            optional_value(args, "--command-template"),
            optional_value(args, "--response-command-template"),
        )),
        Some("fix-loop-batch") => Ok(phase3_fix_loop_batch_plan(
            required_value(args, "--request-glob")?,
            required_value(args, "--responses-dir")?,
            required_value(args, "--output-dir")?,
            required_value(args, "--report-dir")?,
            required_value(args, "--index-output")?,
            required_value(args, "--max-attempts")?,
            optional_value(args, "--command-template"),
            optional_value(args, "--response-command-template"),
        )),
        Some("fix-acceptance") => Ok(phase3_fix_acceptance_plan(
            required_value(args, "--fix-loop-index")?,
            optional_value(args, "--smoke-index"),
            required_value(args, "--output")?,
        )),
        Some("fix-acceptance-write") => Ok(phase3_fix_acceptance_write_plan(
            required_value(args, "--index")?,
            required_value(args, "--harness")?,
            required_value(args, "--status")?,
            required_value(args, "--reason")?,
            optional_value(args, "--source"),
        )),
        Some("fix-acceptance-write-batch") => Ok(phase3_fix_acceptance_write_batch_plan(
            required_value(args, "--index")?,
            required_value(args, "--decisions")?,
            optional_value(args, "--source"),
        )),
        Some("model-response") => Ok(phase3_model_response_plan(
            required_value(args, "--input")?,
            required_value(args, "--output")?,
            required_value(args, "--command-template")?,
            optional_value(args, "--attempt"),
        )),
        Some("afl-bootstrap") => Ok(phase3_afl_bootstrap_plan(
            required_value(args, "--workspace-dir")?,
            required_value(args, "--harness")?,
            optional_value(args, "--input-mode"),
            optional_value(args, "--corpus-dir"),
            optional_value(args, "--findings-dir"),
            optional_value(args, "--afl-target-dir"),
            has_flag(args, "--release"),
            has_flag(args, "--build-only"),
        )),
        Some(other) => Err(format!("unknown phase3 command: {other}")),
        None => Err("missing phase3 command".to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixAcceptanceSync {
    acceptance_index: String,
    coverage: String,
    context: String,
}

fn parse_fix_acceptance_sync(args: &[String]) -> Result<Option<FixAcceptanceSync>, String> {
    if !matches!(
        (
            args.first().map(String::as_str),
            args.get(1).map(String::as_str)
        ),
        (
            Some("phase3"),
            Some("fix-acceptance-write" | "fix-acceptance-write-batch")
        )
    ) {
        return Ok(None);
    }

    let coverage = optional_value(args, "--coverage");
    let context = optional_value(args, "--context");

    if coverage.is_none() && context.is_none() {
        return Ok(None);
    }

    let Some(coverage) = coverage else {
        return Err(
            "--coverage and --context must be provided together for acceptance sync".to_string(),
        );
    };
    let Some(context) = context else {
        return Err(
            "--coverage and --context must be provided together for acceptance sync".to_string(),
        );
    };

    Ok(Some(FixAcceptanceSync {
        acceptance_index: required_value(args, "--index")?.to_string(),
        coverage: coverage.to_string(),
        context: context.to_string(),
    }))
}

fn required_value<'a>(args: &'a [String], flag: &str) -> Result<&'a str, String> {
    optional_value(args, flag).ok_or_else(|| format!("missing required argument: {flag}"))
}

fn optional_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].as_str())
}

fn execute(plan: CommandPlan) -> Result<(), String> {
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    maybe_inject_pythonpath(&mut command, &plan)?;
    let status = command
        .status()
        .map_err(|error| format!("failed to execute {}: {error}", plan.program))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command exited with status {status}"))
    }
}

fn execute_capture(plan: CommandPlan) -> Result<(String, String), String> {
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    maybe_inject_pythonpath(&mut command, &plan)?;
    let output = command
        .output()
        .map_err(|error| format!("failed to execute {}: {error}", plan.program))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !stdout.is_empty() {
        print!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    if output.status.success() {
        Ok((stdout, stderr))
    } else {
        Err(format!("command exited with status {}", output.status))
    }
}

fn maybe_inject_pythonpath(command: &mut Command, plan: &CommandPlan) -> Result<(), String> {
    if !invokes_seraph_python_module(plan) {
        return Ok(());
    }
    let rag_dir = repo_rag_dir();
    let mut paths: Vec<PathBuf> = env::var_os("PYTHONPATH")
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default();
    if !paths.iter().any(|path| path == &rag_dir) {
        paths.push(rag_dir);
    }
    let joined = env::join_paths(paths)
        .map_err(|error| format!("failed to join PYTHONPATH for seraph_rag: {error}"))?;
    command.env("PYTHONPATH", joined);
    Ok(())
}

fn invokes_seraph_python_module(plan: &CommandPlan) -> bool {
    plan.args.len() >= 2 && plan.args[0] == "-m" && plan.args[1] == "seraph_rag.cli"
}

fn repo_rag_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("repo root")
        .join("rag")
}

fn select_afl_harness(
    layout: &seraph_cli::RunLayout,
    override_harness: Option<&str>,
) -> Result<String, String> {
    if let Some(harness) = override_harness {
        return Ok(harness.to_string());
    }

    let fix_loop_path = Path::new(&layout.fix_loop_index);
    if fix_loop_path.exists() {
        if let Some(harness) = first_successful_fixed_harness(fix_loop_path)? {
            return Ok(harness);
        }
    }

    let compile_index_path = Path::new(&layout.compile_index);
    if compile_index_path.exists() {
        if let Some(harness) = first_successful_compile_harness(compile_index_path)? {
            return Ok(harness);
        }
    }

    Err("no successful compile/fix harness available".to_string())
}

fn first_successful_fixed_harness(index_path: &Path) -> Result<Option<String>, String> {
    let payload = read_json(index_path)?;
    let Some(loops) = payload.get("loops").and_then(Value::as_array) else {
        return Ok(None);
    };

    for loop_entry in loops {
        if loop_entry.get("status").and_then(Value::as_str) != Some("ok") {
            continue;
        }
        let successful_attempt = loop_entry.get("successful_attempt").and_then(Value::as_u64);
        let Some(attempts) = fixed_loop_attempts(loop_entry)? else {
            continue;
        };
        for attempt in attempts {
            if attempt.get("status").and_then(Value::as_str) != Some("ok") {
                continue;
            }
            if let Some(expected) = successful_attempt {
                if attempt.get("attempt").and_then(Value::as_u64) != Some(expected) {
                    continue;
                }
            }
            if let Some(harness) = attempt.get("harness").and_then(Value::as_str) {
                return Ok(Some(harness.to_string()));
            }
        }
    }

    Ok(None)
}

fn fixed_loop_attempts(loop_entry: &Value) -> Result<Option<Vec<Value>>, String> {
    if let Some(attempts) = loop_entry.get("attempts").and_then(Value::as_array) {
        return Ok(Some(attempts.clone()));
    }
    let Some(report_path) = loop_entry.get("report").and_then(Value::as_str) else {
        return Ok(None);
    };
    let report = read_json(Path::new(report_path))?;
    let Some(attempts) = report.get("attempts").and_then(Value::as_array) else {
        return Ok(None);
    };
    Ok(Some(attempts.clone()))
}

fn first_successful_compile_harness(index_path: &Path) -> Result<Option<String>, String> {
    let payload = read_json(index_path)?;
    let Some(reports) = payload.get("reports").and_then(Value::as_array) else {
        return Ok(None);
    };
    for report in reports {
        if report.get("status").and_then(Value::as_str) != Some("ok") {
            continue;
        }
        if let Some(harness) = report.get("harness").and_then(Value::as_str) {
            return Ok(Some(harness.to_string()));
        }
    }
    Ok(None)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn maybe_write_crate_config(
    knowledge_path: &str,
    workspace_dir: &str,
) -> Result<Option<String>, String> {
    let payload = read_json(Path::new(knowledge_path))?;
    let Some(crate_meta) = payload.get("crate_meta") else {
        return Ok(None);
    };
    let manifest_path = crate_meta
        .get("manifest_path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let package_name = crate_meta
        .get("package_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let crate_import_name = crate_meta
        .get("crate_import_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    if manifest_path.is_empty() || package_name.is_empty() || crate_import_name.is_empty() {
        return Ok(None);
    }

    let Some(crate_dir) = Path::new(manifest_path).parent() else {
        return Ok(None);
    };

    let config = serde_json::json!({
        "crate_dir": crate_dir.to_string_lossy(),
        "package_name": package_name,
        "crate_import_name": crate_import_name,
    });
    let config_path = Path::new(workspace_dir).join("crate_config.json");
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&config)
            .map_err(|error| format!("failed to encode crate config JSON: {error}"))?
            + "\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", config_path.display()))?;
    Ok(Some(config_path.display().to_string()))
}

fn print_usage() {
    println!(
        "SERAPH unified CLI\n\n\
Usage:\n\
  seraph-cli phase2 index --knowledge <path> --vectordb <path> [--dry-run]\n\
  seraph-cli phase2 graph --knowledge <path> --graph <path> [--dry-run]\n\
  seraph-cli phase2 targets --graph <path> [--dry-run]\n\
  seraph-cli phase2 retrieve --knowledge <path> --graph <path> --vectordb <path> --output <path> [--round <n>] [--target-api-id <api_id>] [--dry-run]\n\
  seraph-cli phase3 harness-prompt --context <path> --output <path> [--variants <n>] [--style <aflpp>] [--dry-run]\n\
  seraph-cli phase3 harness-write --prompt <path> --response <path> --output-dir <path> --round <n> [--dry-run]\n\
  seraph-cli phase3 compile-check --harness-glob <glob> --report-dir <path> --round <n> [--command-template <cmd>] [--dry-run]\n\
  seraph-cli phase3 runtime-diagnose --context <path> --smoke-index <path> --output-dir <path> --round <n> --command-template <cmd> [--dry-run]\n\
  seraph-cli phase3 fixer-bundle --compile-index <path> --context <path> --output-dir <path> [--dry-run]
  seraph-cli phase3 fixer-write --request <path> --response <path> --output-dir <path> --attempt <n> [--dry-run]
  seraph-cli phase3 fix-once --request <path> --response <path> --output-dir <path> --report-dir <path> --attempt <n> [--command-template <cmd>] [--dry-run]
  seraph-cli phase3 fix-loop --request <path> --responses-dir <path> --output-dir <path> --report-dir <path> --max-attempts <n> [--command-template <cmd>] [--response-command-template <cmd>] [--dry-run]
  seraph-cli phase3 fix-loop-batch --request-glob <glob> --responses-dir <path> --output-dir <path> --report-dir <path> --index-output <path> --max-attempts <n> [--command-template <cmd>] [--response-command-template <cmd>] [--dry-run]
  seraph-cli phase3 fix-acceptance --fix-loop-index <path> [--smoke-index <path>] --output <path> [--dry-run]
  seraph-cli phase3 fix-acceptance-write --index <path> --harness <path> --status <accepted|needs_review|bug> --reason <text> [--source <text>] [--coverage <path> --context <path>] [--dry-run]
  seraph-cli phase3 fix-acceptance-write-batch --index <path> --decisions <path> [--source <text>] [--coverage <path> --context <path>] [--dry-run]
  seraph-cli phase3 model-response --input <path> --output <path> --command-template <cmd> [--attempt <n>] [--dry-run]
  seraph-cli phase3 afl-bootstrap --workspace-dir <path> --harness <path> [--input-mode <file|stdin>] [--corpus-dir <path>] [--findings-dir <path>] [--afl-target-dir <path>] [--release] [--build-only] [--dry-run]
  seraph-cli run (--knowledge <path> | --manifest-path <path>) [--workspace-dir <path>] [--round <n>] [--target-api-id <api_id>] [--phase3-prompt] [--variants <n>] [--phase3-style <aflpp>] [--llm-response <path>] [--model-command <cmd>] [--compile-check] [--compile-command <cmd>] [--fixer-bundle] [--fix-loop] [--fix-max-attempts <n>] [--fix-responses-dir <path>] [--fix-model-command <cmd>] [--smoke-command <cmd>] [--runtime-model-command <cmd>] [--afl-bootstrap] [--afl-harness <path>] [--afl-input-mode <file|stdin>] [--afl-corpus-dir <path>] [--afl-findings-dir <path>] [--afl-target-dir <path>] [--afl-release] [--afl-build-only] [--dry-run]
\
Notes:\n\
  - `run` without `--target-api-id` iterates all ranked unsafe targets starting from `--round`.\n\
  - `--llm-response` is single-target only; pair it with `--target-api-id`.\n\
  - `--afl-bootstrap` without `--afl-harness` is single-target only."
    );
}
