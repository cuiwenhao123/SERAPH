# Phase 3 Automatic Runtime Diagnosis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the active Phase 3 acceptance/review path with an automatic compile-fix → smoke-run → runtime-diagnosis loop that writes `runtime_error.json` and counts compile-successful runtime-reaching harnesses as covered.

**Architecture:** Keep the existing RAG retrieval, harness generation, compile-check, and fix-loop structure. Add one new Python runtime diagnosis module plus one new public `seraph-cli phase3 runtime-diagnose` command, then rewire `seraph-cli run` and `s3-coverage` so runtime diagnosis replaces acceptance as the active post-smoke artifact.

**Tech Stack:** Rust workspace crates (`seraph-cli`, `seraph-types`, `s3-coverage`), Python package `rag/seraph_rag`, pytest, Rust integration tests, provider-agnostic command templates for LLM calls.

---

## File Structure

- Create: `rag/seraph_rag/runtime_diagnose.py` — builds diagnosis prompts from smoke failures, calls the model provider, writes per-harness and round-level `runtime_error` reports.
- Modify: `rag/seraph_rag/cli.py` — add `runtime-diagnose` subcommand.
- Create: `rag/tests/test_runtime_diagnose.py` — unit tests for runtime diagnosis artifact generation and classification.
- Modify: `rag/tests/test_cli.py` — CLI tests for `runtime-diagnose`.
- Modify: `crates/seraph-cli/src/lib.rs` — add `phase3_runtime_diagnose_plan` and `RunLayout.runtime_error_index`.
- Modify: `crates/seraph-cli/src/main.rs` — add the new command parser and rewire `run` to use runtime diagnosis instead of acceptance.
- Modify: `crates/seraph-cli/tests/run_dry_run.rs` — dry-run coverage for the new command and unified run ordering.
- Modify: `crates/seraph-cli/tests/run_smoke.rs` — end-to-end runtime diagnosis integration using fake model/smoke commands.
- Modify: `crates/seraph-types/src/coverage.rs` — extend `HarnessRecord` with runtime diagnosis fields while keeping legacy review fields for backward compatibility.
- Modify: `crates/seraph-types/tests/schema_roundtrip.rs` — roundtrip tests for the new fields.
- Modify: `crates/s3-coverage/src/lib.rs` — ingest `runtime_error` instead of acceptance in the active Phase 3 path.
- Modify: `crates/s3-coverage/tests/phase3_coverage.rs` — target-level and harness-level assertions for runtime diagnosis semantics.
- Modify: `docs/architecture/phase3-harness-generation.md` — document the new active flow and downgrade acceptance to compatibility-only.
- Modify: `crates/seraph-cli/README.md` — update public CLI examples and deployment guidance.

---

### Task 1: Add Python Runtime Diagnosis Artifact

**Files:**
- Create: `rag/seraph_rag/runtime_diagnose.py`
- Create: `rag/tests/test_runtime_diagnose.py`
- Modify: `rag/seraph_rag/cli.py`
- Modify: `rag/tests/test_cli.py`

- [ ] **Step 1: Write the failing runtime diagnosis unit tests**

Create `rag/tests/test_runtime_diagnose.py`:

```python
import json
from pathlib import Path

from seraph_rag.runtime_diagnose import run_runtime_diagnosis


def test_run_runtime_diagnosis_writes_detail_and_index(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    reports_dir = tmp_path / "reports"
    smoke_index_path = reports_dir / "smoke_001_index.json"
    harness_path = tmp_path / "fuzz" / "harness_001_02_fixed_01.rs"
    output_dir = reports_dir
    harness_path.parent.mkdir(parents=True)
    reports_dir.mkdir(parents=True)
    harness_path.write_text("fn main() {}\n", encoding="utf-8")
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    (reports_dir / "smoke_001_02_fixed_01.json").write_text(
        json.dumps(
            {
                "harness": str(harness_path),
                "status": "bug",
                "classification": "panic_detected",
                "exit_code": 101,
                "stdout": "",
                "stderr": "thread panicked at bad input",
            }
        ),
        encoding="utf-8",
    )
    smoke_index_path.write_text(
        json.dumps(
            {
                "round": 1,
                "status": "bug",
                "reports": [
                    {
                        "harness": str(harness_path),
                        "report": str(reports_dir / "smoke_001_02_fixed_01.json"),
                        "status": "bug",
                        "classification": "panic_detected",
                        "exit_code": 101,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    result = run_runtime_diagnosis(
        context_path,
        smoke_index_path,
        output_dir,
        round_no=1,
        command_template="python3 -c \"import json; print(json.dumps({'classification':'panic_or_crash','summary':'panic after reaching target','is_bug':False}))\"",
    )

    assert result["status"] == "runtime_error"
    assert result["report_count"] == 1
    assert result["bug_count"] == 0
    assert result["reports"][0]["classification"] == "panic_or_crash"


def test_run_runtime_diagnosis_marks_asan_as_bug(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    reports_dir = tmp_path / "reports"
    smoke_index_path = reports_dir / "smoke_001_index.json"
    harness_path = tmp_path / "fuzz" / "harness_001_01.rs"
    harness_path.parent.mkdir(parents=True)
    reports_dir.mkdir(parents=True)
    harness_path.write_text("fn main() {}\n", encoding="utf-8")
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    (reports_dir / "smoke_001_01.json").write_text(
        json.dumps(
            {
                "harness": str(harness_path),
                "status": "bug",
                "classification": "nonzero_exit",
                "exit_code": 1,
                "stdout": "",
                "stderr": "==1==ERROR: AddressSanitizer: heap-buffer-overflow",
            }
        ),
        encoding="utf-8",
    )
    smoke_index_path.write_text(
        json.dumps(
            {
                "round": 1,
                "status": "bug",
                "reports": [
                    {
                        "harness": str(harness_path),
                        "report": str(reports_dir / "smoke_001_01.json"),
                        "status": "bug",
                        "classification": "nonzero_exit",
                        "exit_code": 1,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    result = run_runtime_diagnosis(
        context_path,
        smoke_index_path,
        reports_dir,
        round_no=1,
        command_template="python3 -c \"import json; print(json.dumps({'classification':'asan_bug','summary':'asan hit inside target','is_bug':True}))\"",
    )

    assert result["status"] == "bug"
    assert result["bug_count"] == 1
    assert result["reports"][0]["bug"] is True
```

- [ ] **Step 2: Run the Python tests to verify they fail**

Run: `cd rag && pytest -q tests/test_runtime_diagnose.py -q`
Expected: FAIL with `ModuleNotFoundError` or `cannot import name 'run_runtime_diagnosis'`.

- [ ] **Step 3: Implement the runtime diagnosis module**

Create `rag/seraph_rag/runtime_diagnose.py`:

```python
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Union

from seraph_rag.model_provider import write_model_response


def run_runtime_diagnosis(
    context_path: Union[str, Path],
    smoke_index_path: Union[str, Path],
    output_dir: Union[str, Path],
    round_no: int,
    command_template: str,
) -> Dict[str, Any]:
    target_api_id = _extract_target_api_id(Path(context_path))
    smoke_index = json.loads(Path(smoke_index_path).read_text(encoding="utf-8"))
    out_dir = Path(output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    reports: List[Dict[str, Any]] = []
    bug_count = 0

    for smoke_ref in smoke_index.get("reports", []):
        if smoke_ref.get("status") == "ok":
            continue
        smoke_report = json.loads(Path(smoke_ref["report"]).read_text(encoding="utf-8"))
        harness_path = Path(smoke_report["harness"])
        request_path = out_dir / f"runtime_request_{round_no:03d}_{_suffix_from_harness(harness_path, round_no)}.json"
        response_path = out_dir / f"runtime_response_{round_no:03d}_{_suffix_from_harness(harness_path, round_no)}.json"
        request_path.write_text(
            json.dumps(_build_prompt(target_api_id, harness_path, smoke_report), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        write_model_response(request_path, response_path, command_template)
        diagnosis = json.loads(response_path.read_text(encoding="utf-8"))
        detail = {
            "version": "seraph.phase3.runtime_error.v1",
            "round": round_no,
            "variant": _variant_from_harness(harness_path),
            "attempt": _attempt_from_harness(harness_path),
            "target_api_id": target_api_id,
            "harness": str(harness_path),
            "compile_report": None,
            "smoke_report": smoke_ref["report"],
            "request": str(request_path),
            "response": str(response_path),
            "status": "bug" if diagnosis.get("is_bug") else "runtime_error",
            "classification": diagnosis["classification"],
            "summary": diagnosis["summary"],
            "evidence": {
                "exit_code": smoke_report.get("exit_code"),
                "stdout": smoke_report.get("stdout", "")[:400],
                "stderr": smoke_report.get("stderr", "")[:400],
            },
            "bug": bool(diagnosis.get("is_bug")),
        }
        detail_path = out_dir / f"runtime_error_{round_no:03d}_{_suffix_from_harness(harness_path, round_no)}.json"
        detail_path.write_text(json.dumps(detail, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        reports.append(
            {
                "harness": detail["harness"],
                "report": str(detail_path),
                "status": detail["status"],
                "classification": detail["classification"],
                "bug": detail["bug"],
            }
        )
        if detail["bug"]:
            bug_count += 1

    index = {
        "version": "seraph.phase3.runtime_error_index.v1",
        "round": round_no,
        "status": "bug" if bug_count else ("runtime_error" if reports else "ok"),
        "report_count": len(reports),
        "bug_count": bug_count,
        "reports": reports,
    }
    index_path = out_dir / f"runtime_error_{round_no:03d}_index.json"
    index_path.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return index


def _extract_target_api_id(context_path: Path) -> str:
    for line in context_path.read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("- api_id:"):
            return line.split(":", 1)[1].strip()
    raise ValueError("context missing target api_id")


def _build_prompt(target_api_id: str, harness_path: Path, smoke_report: Dict[str, Any]) -> Dict[str, Any]:
    harness_text = harness_path.read_text(encoding="utf-8")
    return {
        "target_api_id": target_api_id,
        "harness": harness_text,
        "smoke_status": smoke_report.get("status"),
        "smoke_classification": smoke_report.get("classification"),
        "exit_code": smoke_report.get("exit_code"),
        "stdout": smoke_report.get("stdout", ""),
        "stderr": smoke_report.get("stderr", ""),
        "task": "Summarize the runtime issue. Return JSON with keys classification, summary, is_bug.",
    }


def _variant_from_harness(harness_path: Path) -> int:
    parts = harness_path.stem.split("_")
    return int(parts[2])


def _attempt_from_harness(harness_path: Path) -> int | None:
    parts = harness_path.stem.split("_")
    if "fixed" in parts:
        return int(parts[-1])
    return None


def _suffix_from_harness(harness_path: Path, round_no: int) -> str:
    prefix = f"harness_{round_no:03d}_"
    stem = harness_path.stem
    return stem[len(prefix):] if stem.startswith(prefix) else stem
```

- [ ] **Step 4: Wire the Python CLI and CLI test**

Modify `rag/seraph_rag/cli.py` to add the subcommand:

```python
    runtime_diagnose_parser = subparsers.add_parser("runtime-diagnose")
    runtime_diagnose_parser.add_argument("--context", required=True)
    runtime_diagnose_parser.add_argument("--smoke-index", required=True)
    runtime_diagnose_parser.add_argument("--output-dir", required=True)
    runtime_diagnose_parser.add_argument("--round", type=int, required=True)
    runtime_diagnose_parser.add_argument("--command-template", required=True)
```

And dispatch it:

```python
    if args.command == "runtime-diagnose":
        from seraph_rag.runtime_diagnose import run_runtime_diagnosis

        run_runtime_diagnosis(
            args.context,
            args.smoke_index,
            args.output_dir,
            round_no=args.round,
            command_template=args.command_template,
        )
        return
```

Append to `rag/tests/test_cli.py`:

```python
def test_cli_runtime_diagnose_writes_index(tmp_path):
    reports_dir = tmp_path / "reports"
    context_path = tmp_path / "rag_target_001.md"
    harness_path = tmp_path / "fuzz" / "harness_001_01.rs"
    smoke_index_path = reports_dir / "smoke_001_index.json"
    smoke_report_path = reports_dir / "smoke_001_01.json"
    reports_dir.mkdir(parents=True)
    harness_path.parent.mkdir(parents=True)
    harness_path.write_text("fn main() {}\n", encoding="utf-8")
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    smoke_report_path.write_text(json.dumps({"harness": str(harness_path), "status": "bug", "classification": "panic_detected", "exit_code": 101, "stdout": "", "stderr": "panicked at bad input"}), encoding="utf-8")
    smoke_index_path.write_text(json.dumps({"round": 1, "status": "bug", "reports": [{"harness": str(harness_path), "report": str(smoke_report_path), "status": "bug", "classification": "panic_detected", "exit_code": 101}]}), encoding="utf-8")

    main([
        "runtime-diagnose",
        "--context", str(context_path),
        "--smoke-index", str(smoke_index_path),
        "--output-dir", str(reports_dir),
        "--round", "1",
        "--command-template", "python3 -c \"import json; print(json.dumps({'classification':'panic_or_crash','summary':'panic after target','is_bug':False}))\" > {output}",
    ])

    report = json.loads((reports_dir / "runtime_error_001_index.json").read_text(encoding="utf-8"))
    assert report["status"] == "runtime_error"
    assert report["report_count"] == 1
```

- [ ] **Step 5: Run the Python tests to verify they pass**

Run: `cd rag && pytest -q tests/test_runtime_diagnose.py tests/test_cli.py -k runtime_diagnose`
Expected: PASS for the new runtime diagnosis tests.

- [ ] **Step 6: Commit**

```bash
git add rag/seraph_rag/runtime_diagnose.py rag/seraph_rag/cli.py rag/tests/test_runtime_diagnose.py rag/tests/test_cli.py
git commit -m "feat: add phase3 runtime diagnosis artifacts"
```

---

### Task 2: Add Rust CLI Planning and Unified Run Wiring

**Files:**
- Modify: `crates/seraph-cli/src/lib.rs`
- Modify: `crates/seraph-cli/src/main.rs`
- Modify: `crates/seraph-cli/tests/run_dry_run.rs`
- Modify: `crates/seraph-cli/tests/run_smoke.rs`

- [ ] **Step 1: Write the failing Rust CLI tests**

Append to `crates/seraph-cli/tests/run_dry_run.rs`:

```rust
#[test]
fn runtime_diagnose_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = std::process::Command::new(binary)
        .args([
            "phase3",
            "runtime-diagnose",
            "--context",
            "workspace/contexts/rag_target_001.md",
            "--smoke-index",
            "workspace/reports/smoke_001_index.json",
            "--output-dir",
            "workspace/reports",
            "--round",
            "1",
            "--command-template",
            "python3 fake_runtime_diag.py --input {input} --output {output}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli runtime-diagnose"));
    assert!(stdout.contains("--smoke-index workspace/reports/smoke_001_index.json"));
}
```

Append to `crates/seraph-cli/tests/run_smoke.rs`:

```rust
#[test]
fn run_smoke_failure_still_writes_validated_coverage_and_runtime_error_index() {
    let repo = repo_root();
    let workspace = temp_dir("run-runtime-diagnose");
    let model_script = workspace.join("fake_model.py");
    let runtime_diag_script = workspace.join("fake_runtime_diag.py");
    std::fs::write(
        &model_script,
        r#"import json, sys
print('fn main() { println!("SERAPH_STEP_ENTER:1:api::fixture::danger"); println!("SERAPH_STEP_OK:1:api::fixture::danger"); }')
"#,
    )
    .expect("write model script");
    std::fs::write(
        &runtime_diag_script,
        r#"import json
print(json.dumps({"classification": "panic_or_crash", "summary": "panic after target", "is_bug": False}))
"#,
    )
    .expect("write runtime diag script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = std::process::Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().unwrap(),
            "--round",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(0)'",
            "--smoke-command",
            "python3 -c 'import sys; print(""panicked at smoke"", file=sys.stderr); sys.exit(101)'",
            "--runtime-model-command",
            &format!("python3 {}", runtime_diag_script.display()),
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let coverage = std::fs::read_to_string(workspace.join("coverage.json")).expect("read coverage");
    let runtime_error_index = std::fs::read_to_string(workspace.join("reports/runtime_error_001_index.json")).expect("read runtime error index");
    assert!(coverage.contains("\"validated\""));
    assert!(coverage.contains("panic_or_crash"));
    assert!(runtime_error_index.contains("panic_or_crash"));
}
```

- [ ] **Step 2: Run the Rust tests to verify they fail**

Run: `cargo test -p seraph-cli runtime_diagnose`
Expected: FAIL with `unknown phase3 command: runtime-diagnose` and missing runtime diagnosis artifacts.

- [ ] **Step 3: Add the Rust command plan API**

Modify `crates/seraph-cli/src/lib.rs`:

```rust
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
```

Add the unit test in `crates/seraph-cli/src/lib.rs`:

```rust
#[test]
fn phase3_runtime_diagnose_plan_uses_python_sub_cli() {
    let plan = phase3_runtime_diagnose_plan(
        "workspace/contexts/rag_target_001.md",
        "workspace/reports/smoke_001_index.json",
        "workspace/reports",
        "1",
        "python3 fake_runtime_diag.py --input {input} --output {output}",
    );

    assert!(plan.args.contains(&"runtime-diagnose".to_string()));
    assert!(plan.args.contains(&"workspace/reports/smoke_001_index.json".to_string()));
}
```

- [ ] **Step 4: Rewire `seraph-cli` parser, layout, and unified run**

Modify `crates/seraph-cli/src/main.rs` parser branch:

```rust
        Some("runtime-diagnose") => Ok(phase3_runtime_diagnose_plan(
            required_value(args, "--context")?,
            required_value(args, "--smoke-index")?,
            required_value(args, "--output-dir")?,
            required_value(args, "--round")?,
            required_value(args, "--command-template")?,
        )),
```

Modify `crates/seraph-cli/src/lib.rs` `RunLayout` to add:

```rust
    pub runtime_error_index: String,
```

and initialize it in `run_layout`:

```rust
        runtime_error_index: format!("{workspace_dir}/reports/runtime_error_{round_padded}_index.json"),
```

Then update the `run_layout` unit test to assert the new path.

In `run_pipeline`, add runtime diagnosis fallback selection and remove active acceptance orchestration:

```rust
    let runtime_model_command = optional_value(args, "--runtime-model-command")
        .or(fix_model_command)
        .or(model_command);
    let runtime_diagnose = smoke_run && runtime_model_command.is_some();
```

Invoke it after smoke-run:

```rust
    if let (Some(command), true) = (runtime_model_command, runtime_diagnose) {
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
```

And pass `runtime_error_index` into coverage instead of acceptance:

```rust
                if runtime_diagnose {
                    Some(Path::new(&layout.runtime_error_index))
                } else {
                    None
                },
```

Also update usage text to include:

```text
  seraph-cli phase3 runtime-diagnose --context <path> --smoke-index <path> --output-dir <path> --round <n> --command-template <cmd> [--dry-run]
  seraph-cli run ... [--runtime-model-command <cmd>] ...
```

- [ ] **Step 5: Run the Rust tests to verify they pass**

Run: `cargo test -p seraph-cli`
Expected: PASS for `run_dry_run`, `run_smoke`, and the new runtime diagnosis tests.

- [ ] **Step 6: Commit**

```bash
git add crates/seraph-cli/src/lib.rs crates/seraph-cli/src/main.rs crates/seraph-cli/tests/run_dry_run.rs crates/seraph-cli/tests/run_smoke.rs
git commit -m "feat: wire runtime diagnosis into seraph cli"
```

---

### Task 3: Replace Acceptance Ingestion with Runtime Error Ingestion in Coverage

**Files:**
- Modify: `crates/seraph-types/src/coverage.rs`
- Modify: `crates/seraph-types/tests/schema_roundtrip.rs`
- Modify: `crates/s3-coverage/src/lib.rs`
- Modify: `crates/s3-coverage/tests/phase3_coverage.rs`

- [ ] **Step 1: Write the failing coverage tests**

Append to `crates/s3-coverage/tests/phase3_coverage.rs`:

```rust
#[test]
fn write_coverage_runtime_error_keeps_target_validated() {
    let root = temp_dir("coverage-runtime-error");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("reports/compile_001_index.json");
    let smoke_index_path = root.join("reports/smoke_001_index.json");
    let runtime_error_index_path = root.join("reports/runtime_error_001_index.json");

    std::fs::create_dir_all(root.join("reports")).expect("reports dir");
    std::fs::write(&context_path, "## Target API\n- api_id: api::fixture::danger\n").expect("write context");
    std::fs::write(&compile_index_path, "{\"round\":1,\"status\":\"ok\",\"reports\":[]}\n").expect("write compile index");
    std::fs::write(&smoke_index_path, "{\"round\":1,\"status\":\"bug\",\"reports\":[]}\n").expect("write smoke index");
    std::fs::write(
        &runtime_error_index_path,
        "{\"round\":1,\"status\":\"runtime_error\",\"reports\":[{\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/runtime_error_001_01.json\",\"status\":\"runtime_error\",\"classification\":\"panic_or_crash\",\"bug\":false}]}\n",
    )
    .expect("write runtime error index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        Some(&smoke_index_path),
        Some(&runtime_error_index_path),
    )
    .expect("update coverage");

    assert_eq!(update.status, seraph_types::ApiCoverageStatus::Validated);
    assert!(update.state.found_bugs.is_empty());
}

#[test]
fn write_coverage_runtime_error_asan_marks_found_bug() {
    let root = temp_dir("coverage-runtime-asan");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("reports/compile_001_index.json");
    let runtime_error_index_path = root.join("reports/runtime_error_001_index.json");

    std::fs::create_dir_all(root.join("reports")).expect("reports dir");
    std::fs::write(&context_path, "## Target API\n- api_id: api::fixture::danger\n").expect("write context");
    std::fs::write(&compile_index_path, "{\"round\":1,\"status\":\"ok\",\"reports\":[]}\n").expect("write compile index");
    std::fs::write(
        &runtime_error_index_path,
        "{\"round\":1,\"status\":\"bug\",\"reports\":[{\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/runtime_error_001_01.json\",\"status\":\"bug\",\"classification\":\"asan_bug\",\"bug\":true}]}\n",
    )
    .expect("write runtime error index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        Some(&runtime_error_index_path),
    )
    .expect("update coverage");

    assert_eq!(update.state.found_bugs, vec!["api::fixture::danger"]);
}
```

- [ ] **Step 2: Run the coverage tests to verify they fail**

Run: `cargo test -p s3-coverage phase3_coverage`
Expected: FAIL because coverage still interprets the sixth input as acceptance and still ties runtime semantics to review state.

- [ ] **Step 3: Extend the coverage schema minimally**

Modify `crates/seraph-types/src/coverage.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessRecord {
    pub round: u32,
    pub sub_index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    pub api_ids: Vec<ApiId>,
    pub status: ApiCoverageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_classification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_error_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_error_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bug: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_id: Option<ScenarioId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping_id: Option<MappingId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<PlanId>,
}
```

Append to `crates/seraph-types/tests/schema_roundtrip.rs`:

```rust
#[test]
fn harness_record_roundtrip_supports_runtime_error_fields() {
    let record = seraph_types::HarnessRecord {
        round: 1,
        sub_index: 2,
        attempt: Some(1),
        api_ids: vec![seraph_types::ApiId::from("api::fixture::danger")],
        status: seraph_types::ApiCoverageStatus::Validated,
        harness_path: Some("workspace/fuzz/harness_001_02_fixed_01.rs".into()),
        compile_report_path: Some("workspace/reports/compile_001_02_fixed_01.json".into()),
        smoke_report_path: Some("workspace/reports/smoke_001_02_fixed_01.json".into()),
        runtime_status: Some("runtime_error".into()),
        runtime_classification: Some("panic_or_crash".into()),
        runtime_error_report_path: Some("workspace/reports/runtime_error_001_02_fixed_01.json".into()),
        runtime_error_summary: Some("panic after target".into()),
        bug: Some(false),
        review_status: None,
        review_reason: None,
        review_report_path: None,
        scenario_id: None,
        mapping_id: None,
        plan_id: None,
    };
    let encoded = serde_json::to_string(&record).expect("serialize");
    let decoded: seraph_types::HarnessRecord = serde_json::from_str(&encoded).expect("deserialize");
    assert_eq!(decoded.runtime_error_summary.as_deref(), Some("panic after target"));
}
```

- [ ] **Step 4: Replace acceptance ingestion with runtime error ingestion**

Modify `crates/s3-coverage/src/lib.rs` function signature and logic:

```rust
pub fn write_coverage_from_phase3(
    coverage_path: &Path,
    context_path: &Path,
    compile_index_path: Option<&Path>,
    fix_loop_index_path: Option<&Path>,
    smoke_index_path: Option<&Path>,
    runtime_error_index_path: Option<&Path>,
) -> Result<CoverageUpdateResult, String> {
```

Use runtime diagnosis as the active runtime source:

```rust
fn determine_runtime_outcome(
    smoke_index_path: Option<&Path>,
    runtime_error_index_path: Option<&Path>,
) -> Result<RuntimeOutcome, String> {
    if let Some(path) = runtime_error_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        let mut has_runtime_error = false;
        if let Some(reports) = value.get("reports").and_then(Value::as_array) {
            for report in reports {
                if report.get("bug").and_then(Value::as_bool) == Some(true)
                    || report.get("classification").and_then(Value::as_str) == Some("asan_bug")
                {
                    return Ok(RuntimeOutcome::FoundBug);
                }
                has_runtime_error = true;
            }
        }
        return Ok(if has_runtime_error {
            RuntimeOutcome::Accepted
        } else {
            RuntimeOutcome::None
        });
    }
    let Some(path) = smoke_index_path.filter(|path| path.exists()) else {
        return Ok(RuntimeOutcome::None);
    };
    let value = read_json(path)?;
    if value.get("status").and_then(Value::as_str) == Some("ok") {
        Ok(RuntimeOutcome::Accepted)
    } else {
        Ok(RuntimeOutcome::Accepted)
    }
}
```

And upsert runtime diagnosis onto harness records:

```rust
fn upsert_runtime_error_record(
    state: &mut CoverageState,
    target_api_id: &ApiId,
    entry: &Value,
) -> Result<(), String> {
    let Some(harness_path) = entry.get("harness").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(identity) = parse_harness_identity(harness_path) else {
        return Ok(());
    };
    let key = harness_key(&identity);
    let record = state
        .harnesses
        .entry(key)
        .or_insert_with(|| default_harness_record(&identity, target_api_id));
    ensure_api_id_present(&mut record.api_ids, target_api_id);
    record.harness_path = Some(harness_path.to_string());
    record.runtime_status = entry.get("status").and_then(Value::as_str).map(ToOwned::to_owned);
    record.runtime_classification = entry
        .get("classification")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    record.runtime_error_summary = entry.get("summary").and_then(Value::as_str).map(ToOwned::to_owned);
    record.runtime_error_report_path = entry.get("report").and_then(Value::as_str).map(ToOwned::to_owned);
    record.bug = entry.get("bug").and_then(Value::as_bool);
    Ok(())
}
```

- [ ] **Step 5: Run the coverage and schema tests to verify they pass**

Run: `cargo test -p seraph-types && cargo test -p s3-coverage`
Expected: PASS for schema roundtrip and Phase 3 coverage semantics.

- [ ] **Step 6: Commit**

```bash
git add crates/seraph-types/src/coverage.rs crates/seraph-types/tests/schema_roundtrip.rs crates/s3-coverage/src/lib.rs crates/s3-coverage/tests/phase3_coverage.rs
git commit -m "feat: ingest runtime diagnosis into coverage"
```

---

### Task 4: Downgrade Acceptance From Active Docs and Active Run Path

**Files:**
- Modify: `docs/architecture/phase3-harness-generation.md`
- Modify: `crates/seraph-cli/README.md`
- Modify: `semantic_harness_agent_design_v5.md`

- [ ] **Step 1: Write the failing documentation search check**

Run: `rg -n "fix-acceptance|manual review|needs_review" docs/architecture/phase3-harness-generation.md crates/seraph-cli/README.md semantic_harness_agent_design_v5.md`
Expected: matches show acceptance and review still presented as active Phase 3 flow.

- [ ] **Step 2: Update the architecture doc**

In `docs/architecture/phase3-harness-generation.md`, replace the active I/O table rows with:

```md
| Smoke run | `seraph-cli phase3 smoke-run` | successful compile/fix-loop harnesses | `workspace/reports/smoke_RRR_SS*.json`, `smoke_RRR_index.json` |
| Runtime diagnose | `seraph-cli phase3 runtime-diagnose` | failed smoke reports plus target context | `workspace/reports/runtime_error_RRR_SS*.json`, `runtime_error_RRR_index.json` |
| Fixer bundle | `seraph-cli phase3 fixer-bundle` | compile index, RAG context, failing harnesses | `workspace/fixes/fix_request_RRR_SS.json` |
```

And add a compatibility note:

```md
`fix-acceptance*` commands remain in the codebase temporarily for backward compatibility, but they are no longer part of the active Phase 3 orchestration path.
```

- [ ] **Step 3: Update the public CLI README**

In `crates/seraph-cli/README.md`, replace the acceptance examples with:

```md
cargo run -p seraph-cli -- phase3 runtime-diagnose \
  --context workspace/contexts/rag_target_001.md \
  --smoke-index workspace/reports/smoke_001_index.json \
  --output-dir workspace/reports \
  --round 1 \
  --command-template 'python3 scripts/fake_runtime_diag.py --input {input} --output {output}'
```

And describe the unified behavior:

```md
When `--smoke-command` is enabled, `run` always executes smoke for compile-successful harnesses. If smoke reports any failure and a runtime diagnosis command is available, `run` writes `runtime_error_RRR_index.json` and still counts the target as covered.
```

- [ ] **Step 4: Re-run the documentation search and verify the new active wording**

Run: `rg -n "fix-acceptance|manual review" docs/architecture/phase3-harness-generation.md crates/seraph-cli/README.md semantic_harness_agent_design_v5.md`
Expected: only compatibility notes remain; active flow references `runtime-diagnose`.

- [ ] **Step 5: Commit**

```bash
git add docs/architecture/phase3-harness-generation.md crates/seraph-cli/README.md semantic_harness_agent_design_v5.md
git commit -m "docs: switch active phase3 flow to runtime diagnosis"
```

---

### Task 5: Full Verification and Migration Guardrails

**Files:**
- Modify: `crates/seraph-cli/tests/run_smoke.rs`
- Modify: `crates/s3-coverage/tests/phase3_coverage.rs`
- Modify: `rag/tests/test_runtime_diagnose.py`
- Modify: `rag/tests/test_cli.py`

- [ ] **Step 1: Add the ASan-path end-to-end verification**

Append to `crates/seraph-cli/tests/run_smoke.rs`:

```rust
#[test]
fn run_runtime_diagnose_asan_marks_found_bug() {
    let repo = repo_root();
    let workspace = temp_dir("run-runtime-diagnose-asan");
    let model_script = workspace.join("fake_model.py");
    let runtime_diag_script = workspace.join("fake_runtime_diag.py");
    std::fs::write(&model_script, "print('fn main() { println!(\"SERAPH_STEP_ENTER:1:api::fixture::danger\"); println!(\"SERAPH_STEP_OK:1:api::fixture::danger\"); }')\n").expect("write model");
    std::fs::write(&runtime_diag_script, "import json\nprint(json.dumps({'classification':'asan_bug','summary':'heap overflow in target','is_bug':True}))\n").expect("write diag");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = std::process::Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge", "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir", workspace.to_str().unwrap(),
            "--round", "1",
            "--model-command", &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command", "python3 -c 'import sys; sys.exit(0)'",
            "--smoke-command", "python3 -c 'import sys; print(""==1==ERROR: AddressSanitizer: heap-buffer-overflow"", file=sys.stderr); sys.exit(1)'",
            "--runtime-model-command", &format!("python3 {}", runtime_diag_script.display()),
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let coverage = std::fs::read_to_string(workspace.join("coverage.json")).expect("coverage");
    assert!(coverage.contains("\"found_bugs\": ["));
}
```

- [ ] **Step 2: Run the focused end-to-end suites**

Run: `cargo test -p seraph-cli run_smoke && cargo test -p s3-coverage phase3_coverage && cd rag && pytest -q tests/test_runtime_diagnose.py tests/test_cli.py`
Expected: PASS for runtime diagnosis compile/smoke/coverage paths.

- [ ] **Step 3: Run the full verification suite**

Run: `cargo fmt --all && cargo test --workspace && cd rag && pytest -q`
Expected: PASS across Rust workspace and Python package.

- [ ] **Step 4: Migration safety search**

Run: `rg -n "fix-acceptance|needs_review|manual review" crates/seraph-cli crates/s3-coverage rag docs/architecture`
Expected: compatibility-only hits remain; no active-flow command description should depend on acceptance.

- [ ] **Step 5: Commit**

```bash
git add crates/seraph-cli/tests/run_smoke.rs crates/s3-coverage/tests/phase3_coverage.rs rag/tests/test_runtime_diagnose.py rag/tests/test_cli.py
git commit -m "test: verify runtime diagnosis phase3 flow"
```
