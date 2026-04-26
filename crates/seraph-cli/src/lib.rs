#![forbid(unsafe_code)]

use std::env;

#[derive(Debug, PartialEq, Eq)]
pub struct CommandPlan {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandPlan {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }
}

pub fn python_module_plan(args: Vec<String>) -> CommandPlan {
    let python = env::var("PYTHON_BIN").unwrap_or_else(|_| "python3".to_string());
    let mut full_args = vec!["-m".to_string(), "seraph_rag.cli".to_string()];
    full_args.extend(args);
    CommandPlan::new(python, full_args)
}

fn repo_script_plan(script_name: &str, args: Vec<String>) -> CommandPlan {
    let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("repo root")
        .join("scripts")
        .join(script_name);
    let mut full_args = vec![script_path.display().to_string()];
    full_args.extend(args);
    CommandPlan::new("bash", full_args)
}

pub fn phase2_index_plan(knowledge: &str, vectordb: &str) -> CommandPlan {
    python_module_plan(vec![
        "index".to_string(),
        "--knowledge".to_string(),
        knowledge.to_string(),
        "--vectordb".to_string(),
        vectordb.to_string(),
    ])
}

pub fn phase2_graph_plan(knowledge: &str, graph: &str) -> CommandPlan {
    python_module_plan(vec![
        "graph".to_string(),
        "--knowledge".to_string(),
        knowledge.to_string(),
        "--graph".to_string(),
        graph.to_string(),
    ])
}

pub fn phase2_targets_plan(graph: &str) -> CommandPlan {
    python_module_plan(vec![
        "targets".to_string(),
        "--graph".to_string(),
        graph.to_string(),
    ])
}

pub fn phase2_retrieve_plan(
    knowledge: &str,
    graph: &str,
    vectordb: &str,
    round: &str,
    output: &str,
    target_api_id: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "retrieve".to_string(),
        "--knowledge".to_string(),
        knowledge.to_string(),
        "--graph".to_string(),
        graph.to_string(),
        "--vectordb".to_string(),
        vectordb.to_string(),
        "--round".to_string(),
        round.to_string(),
        "--output".to_string(),
        output.to_string(),
    ];
    if let Some(target) = target_api_id {
        args.push("--target-api-id".to_string());
        args.push(target.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_harness_prompt_plan(
    context: &str,
    output: &str,
    variants: &str,
    style: &str,
) -> CommandPlan {
    python_module_plan(vec![
        "harness-prompt".to_string(),
        "--context".to_string(),
        context.to_string(),
        "--output".to_string(),
        output.to_string(),
        "--variants".to_string(),
        variants.to_string(),
        "--style".to_string(),
        style.to_string(),
    ])
}

pub fn phase3_harness_write_plan(
    prompt: &str,
    response: &str,
    output_dir: &str,
    round: &str,
) -> CommandPlan {
    python_module_plan(vec![
        "harness-write".to_string(),
        "--prompt".to_string(),
        prompt.to_string(),
        "--response".to_string(),
        response.to_string(),
        "--output-dir".to_string(),
        output_dir.to_string(),
        "--round".to_string(),
        round.to_string(),
    ])
}

pub fn phase3_compile_check_plan(
    harness_glob: &str,
    report_dir: &str,
    round: &str,
    command_template: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "compile-check".to_string(),
        "--harness-glob".to_string(),
        harness_glob.to_string(),
        "--report-dir".to_string(),
        report_dir.to_string(),
        "--round".to_string(),
        round.to_string(),
    ];
    if let Some(template) = command_template {
        args.push("--command-template".to_string());
        args.push(template.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_smoke_run_plan(
    compile_index: &str,
    fix_loop_index: Option<&str>,
    report_dir: &str,
    round: &str,
    command_template: &str,
) -> CommandPlan {
    let mut args = vec![
        "smoke-run".to_string(),
        "--compile-index".to_string(),
        compile_index.to_string(),
        "--report-dir".to_string(),
        report_dir.to_string(),
        "--round".to_string(),
        round.to_string(),
        "--command-template".to_string(),
        command_template.to_string(),
    ];
    if let Some(index) = fix_loop_index {
        args.push("--fix-loop-index".to_string());
        args.push(index.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_runtime_diagnose_plan(
    context: &str,
    smoke_index: &str,
    output_dir: &str,
    round: &str,
    command_template: &str,
) -> CommandPlan {
    python_module_plan(vec![
        "runtime-diagnose".to_string(),
        "--context".to_string(),
        context.to_string(),
        "--smoke-index".to_string(),
        smoke_index.to_string(),
        "--output-dir".to_string(),
        output_dir.to_string(),
        "--round".to_string(),
        round.to_string(),
        "--command-template".to_string(),
        command_template.to_string(),
    ])
}

pub fn phase3_fixer_bundle_plan(
    compile_index: &str,
    context: &str,
    output_dir: &str,
) -> CommandPlan {
    python_module_plan(vec![
        "fixer-bundle".to_string(),
        "--compile-index".to_string(),
        compile_index.to_string(),
        "--context".to_string(),
        context.to_string(),
        "--output-dir".to_string(),
        output_dir.to_string(),
    ])
}

pub fn phase3_fixer_write_plan(
    request: &str,
    response: &str,
    output_dir: &str,
    attempt: &str,
) -> CommandPlan {
    python_module_plan(vec![
        "fixer-write".to_string(),
        "--request".to_string(),
        request.to_string(),
        "--response".to_string(),
        response.to_string(),
        "--output-dir".to_string(),
        output_dir.to_string(),
        "--attempt".to_string(),
        attempt.to_string(),
    ])
}

pub fn phase3_fix_once_plan(
    request: &str,
    response: &str,
    output_dir: &str,
    report_dir: &str,
    attempt: &str,
    command_template: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "fix-once".to_string(),
        "--request".to_string(),
        request.to_string(),
        "--response".to_string(),
        response.to_string(),
        "--output-dir".to_string(),
        output_dir.to_string(),
        "--report-dir".to_string(),
        report_dir.to_string(),
        "--attempt".to_string(),
        attempt.to_string(),
    ];
    if let Some(template) = command_template {
        args.push("--command-template".to_string());
        args.push(template.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_fix_loop_plan(
    request: &str,
    responses_dir: &str,
    output_dir: &str,
    report_dir: &str,
    max_attempts: &str,
    command_template: Option<&str>,
    response_command_template: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "fix-loop".to_string(),
        "--request".to_string(),
        request.to_string(),
        "--responses-dir".to_string(),
        responses_dir.to_string(),
        "--output-dir".to_string(),
        output_dir.to_string(),
        "--report-dir".to_string(),
        report_dir.to_string(),
        "--max-attempts".to_string(),
        max_attempts.to_string(),
    ];
    if let Some(template) = command_template {
        args.push("--command-template".to_string());
        args.push(template.to_string());
    }
    if let Some(template) = response_command_template {
        args.push("--response-command-template".to_string());
        args.push(template.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_fix_loop_batch_plan(
    request_glob: &str,
    responses_dir: &str,
    output_dir: &str,
    report_dir: &str,
    index_output: &str,
    max_attempts: &str,
    command_template: Option<&str>,
    response_command_template: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "fix-loop-batch".to_string(),
        "--request-glob".to_string(),
        request_glob.to_string(),
        "--responses-dir".to_string(),
        responses_dir.to_string(),
        "--output-dir".to_string(),
        output_dir.to_string(),
        "--report-dir".to_string(),
        report_dir.to_string(),
        "--index-output".to_string(),
        index_output.to_string(),
        "--max-attempts".to_string(),
        max_attempts.to_string(),
    ];
    if let Some(template) = command_template {
        args.push("--command-template".to_string());
        args.push(template.to_string());
    }
    if let Some(template) = response_command_template {
        args.push("--response-command-template".to_string());
        args.push(template.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_fix_acceptance_plan(
    fix_loop_index: &str,
    smoke_index: Option<&str>,
    output: &str,
) -> CommandPlan {
    let mut args = vec![
        "fix-acceptance".to_string(),
        "--fix-loop-index".to_string(),
        fix_loop_index.to_string(),
        "--output".to_string(),
        output.to_string(),
    ];
    if let Some(smoke_index) = smoke_index {
        args.push("--smoke-index".to_string());
        args.push(smoke_index.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_fix_acceptance_write_plan(
    index: &str,
    harness: &str,
    status: &str,
    reason: &str,
    source: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "fix-acceptance-write".to_string(),
        "--index".to_string(),
        index.to_string(),
        "--harness".to_string(),
        harness.to_string(),
        "--status".to_string(),
        status.to_string(),
        "--reason".to_string(),
        reason.to_string(),
    ];
    if let Some(source) = source {
        args.push("--source".to_string());
        args.push(source.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_fix_acceptance_write_batch_plan(
    index: &str,
    decisions: &str,
    source: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "fix-acceptance-write-batch".to_string(),
        "--index".to_string(),
        index.to_string(),
        "--decisions".to_string(),
        decisions.to_string(),
    ];
    if let Some(source) = source {
        args.push("--source".to_string());
        args.push(source.to_string());
    }
    python_module_plan(args)
}

pub fn phase3_model_response_plan(
    input: &str,
    output: &str,
    command_template: &str,
    attempt: Option<&str>,
) -> CommandPlan {
    let mut args = vec![
        "model-response".to_string(),
        "--input".to_string(),
        input.to_string(),
        "--output".to_string(),
        output.to_string(),
        "--command-template".to_string(),
        command_template.to_string(),
    ];
    if let Some(attempt) = attempt {
        args.push("--attempt".to_string());
        args.push(attempt.to_string());
    }
    python_module_plan(args)
}

#[allow(clippy::too_many_arguments)]
pub fn phase3_afl_bootstrap_plan(
    workspace_dir: &str,
    harness: &str,
    input_mode: Option<&str>,
    corpus_dir: Option<&str>,
    findings_dir: Option<&str>,
    afl_target_dir: Option<&str>,
    release: bool,
    build_only: bool,
) -> CommandPlan {
    let mut args = vec![
        "--workspace-dir".to_string(),
        workspace_dir.to_string(),
        "--harness".to_string(),
        harness.to_string(),
    ];
    if let Some(value) = input_mode {
        args.push("--input-mode".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = corpus_dir {
        args.push("--corpus-dir".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = findings_dir {
        args.push("--findings-dir".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = afl_target_dir {
        args.push("--afl-target-dir".to_string());
        args.push(value.to_string());
    }
    if release {
        args.push("--release".to_string());
    }
    if build_only {
        args.push("--build-only".to_string());
    }
    repo_script_plan("bootstrap-fuzz-target.sh", args)
}

pub fn quote_for_display(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.' | ':' | '='))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

pub fn display_command(plan: &CommandPlan) -> String {
    std::iter::once(quote_for_display(&plan.program))
        .chain(plan.args.iter().map(|arg| quote_for_display(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase2_index_plan_invokes_python_sub_cli() {
        let plan = phase2_index_plan("workspace/knowledge.json", "workspace/vectordb");

        assert_eq!(plan.program, "python3");
        assert_eq!(
            plan.args,
            vec![
                "-m",
                "seraph_rag.cli",
                "index",
                "--knowledge",
                "workspace/knowledge.json",
                "--vectordb",
                "workspace/vectordb",
            ]
        );
    }

    #[test]
    fn phase3_compile_check_plan_accepts_optional_command_template() {
        let plan = phase3_compile_check_plan(
            "workspace/fuzz/harness_001_*.rs",
            "workspace/reports",
            "1",
            Some("rustc {harness}"),
        );

        assert_eq!(plan.args[1], "seraph_rag.cli");
        assert!(plan.args.contains(&"compile-check".to_string()));
        assert!(plan.args.contains(&"--command-template".to_string()));
        assert!(plan.args.contains(&"rustc {harness}".to_string()));
    }

    #[test]
    fn phase3_smoke_run_plan_uses_python_sub_cli() {
        let plan = phase3_smoke_run_plan(
            "workspace/reports/compile_001_index.json",
            Some("workspace/reports/fix_loop_001_index.json"),
            "workspace/reports",
            "1",
            "python3 -c 'import sys; sys.exit(0)'",
        );

        assert!(plan.args.contains(&"smoke-run".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/compile_001_index.json".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/fix_loop_001_index.json".to_string()));
    }

    #[test]
    fn phase3_runtime_diagnose_plan_uses_python_sub_cli() {
        let plan = phase3_runtime_diagnose_plan(
            "workspace/contexts/rag_target_001.md",
            "workspace/reports/smoke_001_index.json",
            "workspace/reports",
            "1",
            "python3 fake_runtime.py --input {input} --output {output}",
        );

        assert!(plan.args.contains(&"runtime-diagnose".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/contexts/rag_target_001.md".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/smoke_001_index.json".to_string()));
    }

    #[test]
    fn display_command_quotes_spaces() {
        let plan = CommandPlan::new(
            "python3",
            vec!["--command-template".into(), "rustc {harness}".into()],
        );

        assert_eq!(
            display_command(&plan),
            "python3 --command-template 'rustc {harness}'"
        );
    }

    #[test]
    fn phase3_fix_loop_plan_accepts_optional_command_template() {
        let plan = phase3_fix_loop_plan(
            "workspace/fixes/fix_request_001_01.json",
            "workspace/fixes",
            "workspace/fuzz",
            "workspace/reports",
            "3",
            Some("rustc {harness}"),
            Some("python3 fake_fixer.py --input {input} --attempt {attempt}"),
        );

        assert!(plan.args.contains(&"fix-loop".to_string()));
        assert!(plan.args.contains(&"--responses-dir".to_string()));
        assert!(plan.args.contains(&"workspace/fixes".to_string()));
        assert!(plan.args.contains(&"--max-attempts".to_string()));
        assert!(plan.args.contains(&"3".to_string()));
        assert!(plan.args.contains(&"rustc {harness}".to_string()));
        assert!(plan
            .args
            .contains(&"python3 fake_fixer.py --input {input} --attempt {attempt}".to_string()));
    }

    #[test]
    fn phase3_fix_loop_batch_plan_accepts_optional_command_template() {
        let plan = phase3_fix_loop_batch_plan(
            "workspace/fixes/fix_request_001_*.json",
            "workspace/fixes",
            "workspace/fuzz",
            "workspace/reports",
            "workspace/reports/fix_loop_001_index.json",
            "3",
            Some("rustc {harness}"),
            Some("python3 fake_fixer.py --input {input} --attempt {attempt}"),
        );

        assert!(plan.args.contains(&"fix-loop-batch".to_string()));
        assert!(plan.args.contains(&"--request-glob".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/fix_loop_001_index.json".to_string()));
        assert!(plan.args.contains(&"--max-attempts".to_string()));
        assert!(plan.args.contains(&"rustc {harness}".to_string()));
        assert!(plan
            .args
            .contains(&"python3 fake_fixer.py --input {input} --attempt {attempt}".to_string()));
    }

    #[test]
    fn phase3_afl_bootstrap_plan_uses_script_entrypoint() {
        let plan = phase3_afl_bootstrap_plan(
            "workspace",
            "fuzz/harness_001_01.rs",
            Some("stdin"),
            None,
            None,
            None,
            false,
            true,
        );

        assert_eq!(plan.program, "bash");
        assert!(plan.args[0].contains("scripts/bootstrap-fuzz-target.sh"));
        assert!(plan.args.contains(&"--workspace-dir".to_string()));
        assert!(plan.args.contains(&"workspace".to_string()));
        assert!(plan.args.contains(&"--harness".to_string()));
        assert!(plan.args.contains(&"fuzz/harness_001_01.rs".to_string()));
        assert!(plan.args.contains(&"--input-mode".to_string()));
        assert!(plan.args.contains(&"stdin".to_string()));
        assert!(plan.args.contains(&"--build-only".to_string()));
    }

    #[test]
    fn phase3_fix_acceptance_plan_uses_python_sub_cli() {
        let plan = phase3_fix_acceptance_plan(
            "workspace/reports/fix_loop_001_index.json",
            Some("workspace/reports/smoke_001_index.json"),
            "workspace/reports/fix_acceptance_001_index.json",
        );

        assert!(plan.args.contains(&"fix-acceptance".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/fix_loop_001_index.json".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/fix_acceptance_001_index.json".to_string()));
    }

    #[test]
    fn phase3_fix_acceptance_write_plan_uses_python_sub_cli() {
        let plan = phase3_fix_acceptance_write_plan(
            "workspace/reports/fix_acceptance_001_index.json",
            "workspace/fuzz/harness_001_01_fixed_01.rs",
            "accepted",
            "manual_triage_ok",
            Some("manual"),
        );

        assert!(plan.args.contains(&"fix-acceptance-write".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/fix_acceptance_001_index.json".to_string()));
        assert!(plan.args.contains(&"accepted".to_string()));
    }

    #[test]
    fn phase3_fix_acceptance_write_batch_plan_uses_python_sub_cli() {
        let plan = phase3_fix_acceptance_write_batch_plan(
            "workspace/reports/fix_acceptance_001_index.json",
            "workspace/reports/fix_acceptance_001_decisions.json",
            Some("manual"),
        );

        assert!(plan
            .args
            .contains(&"fix-acceptance-write-batch".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/fix_acceptance_001_index.json".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/reports/fix_acceptance_001_decisions.json".to_string()));
    }

    #[test]
    fn phase3_model_response_plan_uses_python_sub_cli() {
        let plan = phase3_model_response_plan(
            "workspace/prompts/harness_prompt_001.json",
            "workspace/prompts/llm_response_001.md",
            "python3 fake_model.py --input {input}",
            None,
        );

        assert!(plan.args.contains(&"model-response".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/prompts/harness_prompt_001.json".to_string()));
        assert!(plan
            .args
            .contains(&"workspace/prompts/llm_response_001.md".to_string()));
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RunLayout {
    pub round_padded: String,
    pub knowledge_out: String,
    pub vectordb: String,
    pub graph: String,
    pub context: String,
    pub coverage: String,
    pub prompt: String,
    pub llm_response: String,
    pub fuzz_dir: String,
    pub harness_glob: String,
    pub report_dir: String,
    pub compile_index: String,
    pub smoke_index: String,
    pub runtime_error_index: String,
    pub acceptance_index: String,
    pub fix_dir: String,
    pub fix_request_glob: String,
    pub fix_loop_index: String,
}

pub fn run_layout(workspace_dir: &str, round: u32) -> RunLayout {
    let round_padded = format!("{round:03}");
    RunLayout {
        round_padded: round_padded.clone(),
        knowledge_out: format!("{workspace_dir}/knowledge.json"),
        vectordb: format!("{workspace_dir}/vectordb"),
        graph: format!("{workspace_dir}/graph.pkl"),
        context: format!("{workspace_dir}/contexts/rag_target_{round_padded}.md"),
        coverage: format!("{workspace_dir}/coverage.json"),
        prompt: format!("{workspace_dir}/prompts/harness_prompt_{round_padded}.json"),
        llm_response: format!("{workspace_dir}/prompts/llm_response_{round_padded}.md"),
        fuzz_dir: format!("{workspace_dir}/fuzz"),
        harness_glob: format!("{workspace_dir}/fuzz/harness_{round_padded}_*.rs"),
        report_dir: format!("{workspace_dir}/reports"),
        compile_index: format!("{workspace_dir}/reports/compile_{round_padded}_index.json"),
        smoke_index: format!("{workspace_dir}/reports/smoke_{round_padded}_index.json"),
        runtime_error_index: format!(
            "{workspace_dir}/reports/runtime_error_{round_padded}_index.json"
        ),
        acceptance_index: format!(
            "{workspace_dir}/reports/fix_acceptance_{round_padded}_index.json"
        ),
        fix_dir: format!("{workspace_dir}/fixes"),
        fix_request_glob: format!("{workspace_dir}/fixes/fix_request_{round_padded}_*.json"),
        fix_loop_index: format!("{workspace_dir}/reports/fix_loop_{round_padded}_index.json"),
    }
}

#[cfg(test)]
mod run_layout_tests {
    use super::*;

    #[test]
    fn run_layout_uses_padded_round_paths() {
        let layout = run_layout("/tmp/work", 7);

        assert_eq!(layout.round_padded, "007");
        assert_eq!(layout.knowledge_out, "/tmp/work/knowledge.json");
        assert_eq!(layout.context, "/tmp/work/contexts/rag_target_007.md");
        assert_eq!(layout.coverage, "/tmp/work/coverage.json");
        assert_eq!(layout.prompt, "/tmp/work/prompts/harness_prompt_007.json");
        assert_eq!(layout.llm_response, "/tmp/work/prompts/llm_response_007.md");
        assert_eq!(layout.harness_glob, "/tmp/work/fuzz/harness_007_*.rs");
        assert_eq!(
            layout.compile_index,
            "/tmp/work/reports/compile_007_index.json"
        );
        assert_eq!(layout.smoke_index, "/tmp/work/reports/smoke_007_index.json");
        assert_eq!(
            layout.runtime_error_index,
            "/tmp/work/reports/runtime_error_007_index.json"
        );
        assert_eq!(
            layout.acceptance_index,
            "/tmp/work/reports/fix_acceptance_007_index.json"
        );
        assert_eq!(
            layout.fix_request_glob,
            "/tmp/work/fixes/fix_request_007_*.json"
        );
        assert_eq!(
            layout.fix_loop_index,
            "/tmp/work/reports/fix_loop_007_index.json"
        );
    }
}
