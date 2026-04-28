# AFL++ Merged Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement case-level one-shot screening plus crate-level merged AFL++ harness generation, then build and launch a DeepSurf-style paired ASan + CmpLog campaign from the passing cases.

**Architecture:** Keep SERAPH's first-stage LLM flow target-centric and repairable by generating `run_case(input: &[u8])` case modules, then let tool-generated wrappers handle compile-check and smoke-run. After screening, synthesize one merged Cargo target per crate with a selector-based dispatcher and an `afl::fuzz!` entrypoint, then upgrade the AFL bootstrap script and CLI orchestration to build regular, ASan, and CmpLog binaries and launch two cooperating AFL++ jobs.

**Tech Stack:** Python (`rag/seraph_rag`), pytest, Rust (`crates/seraph-cli`), shell (`scripts/bootstrap-fuzz-target.sh`), AFL++

---

## File Structure

- Create: `rag/seraph_rag/case_harness.py`
  - Render deterministic one-shot wrapper `main.rs` files and Cargo manifests around model-authored case modules.
- Create: `rag/seraph_rag/merge_harnesses.py`
  - Select passing cases from compile/smoke/fix reports, render merged `registry.rs` and `main.rs`, write merge reports, and create the merged Cargo project.
- Create: `rag/tests/test_merge_harnesses.py`
  - Lock merge selection, merged source rendering, and merge report behavior.
- Modify: `rag/seraph_rag/harness_prompt.py`
  - Change the generation contract from standalone `fn main()` binaries to `pub fn run_case(input: &[u8])` case modules.
- Modify: `rag/seraph_rag/harness_codegen.py`
  - Validate the new `run_case` case shape and reject old `fn main()`-style responses.
- Modify: `rag/seraph_rag/real_crate_runner.py`
  - Replace direct `src/main.rs` copying with wrapper-aware case projects and merged-target projects.
- Modify: `rag/seraph_rag/cli.py`
  - Add `merge-harnesses` and expose the merged-harness synthesis path.
- Modify: `rag/tests/test_harness_prompt.py`
  - Lock the new prompt wording and case-generation contract.
- Modify: `rag/tests/test_harness_codegen.py`
  - Lock the new `run_case` validation rules.
- Modify: `rag/tests/test_real_crate_runner.py`
  - Prove case projects now contain `case_impl.rs` plus a generated wrapper `main.rs`.
- Modify: `rag/tests/test_smoke_run.py`
  - Reuse compile/fix/smoke indexes for merge eligibility tests.
- Modify: `crates/seraph-cli/src/lib.rs`
  - Add `phase3_merge_harnesses_plan`, extend `phase3_afl_bootstrap_plan`, and extend `RunLayout`.
- Modify: `crates/seraph-cli/src/main.rs`
  - Insert merge before AFL bootstrap and wire merged-target paths into the unified `run` flow.
- Modify: `crates/seraph-cli/tests/bootstrap_fuzz_target.rs`
  - Replace single-harness dry-run assertions with merged-manifest regular/ASan/CmpLog dry-run assertions.
- Modify: `crates/seraph-cli/tests/run_dry_run.rs`
  - Assert dry-run output now includes `merge-harnesses` and merged AFL bootstrap commands.
- Modify: `crates/seraph-cli/tests/run_smoke.rs`
  - Validate the end-to-end fake workflow: case screening, merge, and AFL build-only bootstrap.
- Modify: `scripts/bootstrap-fuzz-target.sh`
  - Build regular, ASan, and CmpLog merged binaries and render or launch paired AFL++ commands with `-M`, `-S`, and `-c`.
- Modify: `README.md`
  - Update the active `aflpp` description to the two-stage one-shot-screening plus merged AFL++ model.
- Modify: `crates/seraph-cli/README.md`
  - Update CLI examples for `merge-harnesses` and the merged `afl-bootstrap` flow.
- Modify: `docs/architecture/phase3-harness-generation.md`
  - Document the new case-module contract, merge gate, and paired AFL++ campaign.

### Task 1: Change the Prompt and Codegen Contract to `run_case`

**Files:**
- Modify: `rag/tests/test_harness_prompt.py`
- Modify: `rag/tests/test_harness_codegen.py`
- Modify: `rag/seraph_rag/harness_prompt.py`
- Modify: `rag/seraph_rag/harness_codegen.py`

- [ ] **Step 1: Write the failing tests for the new case-module contract**

```python
def test_build_prompt_bundle_requests_run_case_modules():
    context = """# SERAPH Rust Harness Context

## Crate Facts
- crate_name: fixture
- crate_import_name: fixture
- target_crate_kind: library

## Target API
- api_id: api::fixture::Buffer::get_unchecked
- path: fixture::Buffer::get_unchecked
- signature: unsafe fn get_unchecked(&self, index: usize) -> u8
"""

    bundle = build_prompt_bundle(context, variants=2)

    assert "Output only Rust code blocks, one harness variant per code block." in bundle["system_prompt"]
    assert "Emit a Rust case module, not a full executable." in bundle["system_prompt"]
    assert "Define `pub fn run_case(input: &[u8])` in every variant." in bundle["system_prompt"]
    assert "Do not generate `fn main()`." in bundle["system_prompt"]
    assert "Keep all setup and the target call inside `run_case`." in bundle["user_prompt"]
    assert "Return only Rust code blocks, one case module per code block" in bundle["user_prompt"]


def test_write_harnesses_rejects_main_style_variants(tmp_path):
    prompt_path = tmp_path / "harness_prompt_004.json"
    response_path = tmp_path / "llm_response.md"
    output_dir = tmp_path / "fuzz"
    prompt_path.write_text(
        json.dumps({"target_api_id": "api::fixture::Buffer::get_unchecked"}),
        encoding="utf-8",
    )
    response_path.write_text(
        """```rust
fn main() {
    println!("SERAPH_STEP_ENTER:1:api::fixture::Buffer::get_unchecked");
    println!("SERAPH_STEP_OK:1:api::fixture::Buffer::get_unchecked");
}
```""",
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="run_case"):
        write_harnesses_from_response(prompt_path, response_path, output_dir, round_no=4)
```

- [ ] **Step 2: Run the prompt/codegen tests and verify they fail on the old `fn main()` contract**

Run: `PYTHONPATH=rag pytest rag/tests/test_harness_prompt.py rag/tests/test_harness_codegen.py -q`
Expected: FAIL because the prompt still asks for a normal Rust binary with `fn main()` and `write_harnesses_from_response()` still accepts main-style code blocks.

- [ ] **Step 3: Implement the new prompt wording and `run_case` validation**

```python
_RUN_CASE_RE = re.compile(
    r"(?m)^pub\s+fn\s+run_case\s*\(\s*input\s*:\s*&\[\s*u8\s*\]\s*\)"
)


def _build_system_prompt(style: str) -> str:
    if style != DEFAULT_HARNESS_STYLE:
        raise ValueError("unsupported harness style: {}".format(style))
    return """You are SERAPH's Rust fuzz harness generation expert.

Your job is to generate fact-grounded Rust case modules from a structured SERAPH context.

Primary goals, in order:
1. Real target reachability: every variant must truly call the Target API.
2. Factual correctness: use only crate APIs, types, traits, enum variants, module paths, and setup facts explicitly present in the context.
3. Rust compile realism: treat Compile-Time Facts as authoritative and keep the code compile-fixable.
4. Diversity: when the context supports it, vary setup, input shaping, boundary selection, or state progression across variants.

Output contract:
- Output only Rust code blocks, one harness variant per code block.
- Emit a Rust case module, not a full executable.
- Define `pub fn run_case(input: &[u8])` in every variant.
- Call the Target API in every variant.
- Preserve exact `SERAPH_STEP_ENTER:<step_no>:<api_id>` and `SERAPH_STEP_OK:<step_no>:<api_id>` markers around each successful target call.
- Use the crate import name specified in the context.
- Do not generate `fn main()`, `afl::fuzz!`, or crate-level registry code.
""".strip()


def _ensure_case_contract(source: str, target_api_id: str) -> None:
    if "fn main" in source:
        raise ValueError("generated case must define `run_case`, not `fn main`")
    if not _RUN_CASE_RE.search(source):
        raise ValueError("generated case missing `pub fn run_case(input: &[u8])`")
    _ensure_target_markers(source, target_api_id)


def build_prompt_bundle(
    rag_context: str,
    variants: int = 3,
    style: str = DEFAULT_HARNESS_STYLE,
) -> Dict[str, Any]:
    normalized_style = _normalize_style(style)
    target_api_id = extract_target_api_id(rag_context)
    user_prompt = """Generate {variants} Rust case variants for the SERAPH target below.

Requirements:
- Every variant must call the Target API.
- Define `pub fn run_case(input: &[u8])` in every variant.
- Keep all setup and the target call inside `run_case`.
- Prefer `Related APIs` as the main construction pool.
- Use `Known Reachable Paths` as validated anchors when helpful, but do not copy them mechanically.
- Treat `Compile-Time Facts` as authoritative.
- Return only Rust code blocks, one case module per code block, with no prose outside the code blocks.

Target API id: {target_api_id}
Harness style: {style}
Requested variants: {variants}

{rag_context}
""".format(
        variants=variants,
        target_api_id=target_api_id,
        style=normalized_style,
        rag_context=rag_context.rstrip(),
    )
    return {
        "version": PROMPT_VERSION,
        "target_api_id": target_api_id,
        "style": normalized_style,
        "variants": variants,
        "system_prompt": _build_system_prompt(normalized_style),
        "user_prompt": user_prompt,
    }
```

- [ ] **Step 4: Run the updated Python tests and verify they pass**

Run: `PYTHONPATH=rag pytest rag/tests/test_harness_prompt.py rag/tests/test_harness_codegen.py -q`
Expected: PASS

- [ ] **Step 5: Commit the case-contract slice**

```bash
git add rag/tests/test_harness_prompt.py rag/tests/test_harness_codegen.py rag/seraph_rag/harness_prompt.py rag/seraph_rag/harness_codegen.py
git commit -m "feat: switch rust harness generation to run_case modules"
```

### Task 2: Generate Deterministic One-Shot Wrappers Around Case Modules

**Files:**
- Create: `rag/seraph_rag/case_harness.py`
- Modify: `rag/seraph_rag/real_crate_runner.py`
- Modify: `rag/tests/test_real_crate_runner.py`

- [ ] **Step 1: Add failing tests for wrapper-aware Cargo project generation**

```python
def test_ensure_cargo_project_writes_case_impl_and_wrapper_main(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    harness = fuzz_dir / "harness_001_01.rs"
    fuzz_dir.mkdir(parents=True)
    harness.write_text(
        "pub fn run_case(input: &[u8]) { let _ = input; }\n",
        encoding="utf-8",
    )
    (workspace / "crate_config.json").write_text(
        json.dumps(
            {
                "crate_dir": "/tmp/target-crate",
                "package_name": "moonfire-ffmpeg",
                "crate_import_name": "moonfire_ffmpeg",
            }
        ),
        encoding="utf-8",
    )

    project_dir = ensure_cargo_project(harness)

    case_impl = (project_dir / "src/case_impl.rs").read_text(encoding="utf-8")
    main_rs = (project_dir / "src/main.rs").read_text(encoding="utf-8")
    assert "pub fn run_case(input: &[u8])" in case_impl
    assert "mod case_impl;" in main_rs
    assert "case_impl::run_case(&data);" in main_rs


def test_compile_harness_builds_wrapper_binary(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    harness = fuzz_dir / "harness_001_02.rs"
    fuzz_dir.mkdir(parents=True)
    harness.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
    (workspace / "crate_config.json").write_text(
        json.dumps(
            {
                "crate_dir": "/tmp/target-crate",
                "package_name": "moonfire-ffmpeg",
                "crate_import_name": "moonfire_ffmpeg",
            }
        ),
        encoding="utf-8",
    )
    captured = {}

    class Result:
        returncode = 0
        stdout = "ok"
        stderr = ""

    def fake_run(command, **kwargs):
        captured["command"] = command
        captured["kwargs"] = kwargs
        return Result()

    monkeypatch.setattr("seraph_rag.real_crate_runner.subprocess.run", fake_run)

    result = compile_harness(harness)

    assert result.returncode == 0
    assert captured["command"][:3] == ["cargo", "build", "--manifest-path"]
    assert (workspace / "_cargo_projects" / "harness_001_02" / "src" / "case_impl.rs").exists()
```

- [ ] **Step 2: Run the real-crate-runner tests and verify they fail on direct `main.rs` copying**

Run: `PYTHONPATH=rag pytest rag/tests/test_real_crate_runner.py -q`
Expected: FAIL because `ensure_cargo_project()` still copies the harness directly into `src/main.rs` and never writes `src/case_impl.rs`.

- [ ] **Step 3: Create `case_harness.py` and make `real_crate_runner.py` use it**

```python
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class CaseProject:
    project_dir: Path
    manifest_path: Path
    main_rs_path: Path
    case_impl_path: Path


def render_case_wrapper_main() -> str:
    return """mod case_impl;

use std::fs;
use std::io::{self, Read};

fn read_input() -> Vec<u8> {
    if let Some(path) = std::env::args().nth(1) {
        return fs::read(path).unwrap_or_default();
    }
    let mut data = Vec::new();
    let _ = io::stdin().read_to_end(&mut data);
    data
}

fn main() {
    let data = read_input();
    case_impl::run_case(&data);
}
"""


def write_case_project(project_dir: Path, harness: Path, manifest_text: str) -> CaseProject:
    src_dir = project_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = project_dir / "Cargo.toml"
    case_impl_path = src_dir / "case_impl.rs"
    main_rs_path = src_dir / "main.rs"
    manifest_path.write_text(manifest_text, encoding="utf-8")
    case_impl_path.write_text(harness.read_text(encoding="utf-8"), encoding="utf-8")
    main_rs_path.write_text(render_case_wrapper_main(), encoding="utf-8")
    return CaseProject(project_dir, manifest_path, main_rs_path, case_impl_path)
```

- [ ] **Step 4: Run the real-crate-runner tests again and verify they pass**

Run: `PYTHONPATH=rag pytest rag/tests/test_real_crate_runner.py -q`
Expected: PASS

- [ ] **Step 5: Commit the wrapper-generation slice**

```bash
git add rag/seraph_rag/case_harness.py rag/seraph_rag/real_crate_runner.py rag/tests/test_real_crate_runner.py
git commit -m "feat: generate one-shot wrapper binaries for case harnesses"
```

### Task 3: Merge Only Passing Cases into a Crate-Level AFL++ Target

**Files:**
- Create: `rag/seraph_rag/merge_harnesses.py`
- Create: `rag/tests/test_merge_harnesses.py`
- Modify: `rag/seraph_rag/cli.py`
- Modify: `rag/tests/test_smoke_run.py`
- Modify: `rag/tests/test_cli.py`

- [ ] **Step 1: Add failing tests for merge selection and merged source generation**

```python
def test_write_merged_harnesses_selects_only_smoke_ok_cases(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    reports_dir = workspace / "reports"
    fuzz_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    ok_case = fuzz_dir / "harness_001_01.rs"
    bug_case = fuzz_dir / "harness_001_02.rs"
    ok_case.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
    bug_case.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")

    (reports_dir / "compile_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "ok",
                "reports": [
                    {"harness": str(ok_case), "status": "ok", "exit_code": 0},
                    {"harness": str(bug_case), "status": "ok", "exit_code": 0},
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "bug",
                "reports": [
                    {"harness": str(ok_case), "status": "ok", "classification": "completed", "exit_code": 0},
                    {"harness": str(bug_case), "status": "bug", "classification": "nonzero_exit", "exit_code": 1},
                ],
            }
        ),
        encoding="utf-8",
    )

    report = write_merged_harnesses(
        workspace_dir=workspace,
        round_no=1,
        crate_name="fixture",
        crate_import_name="fixture",
    )

    selected_cases = (workspace / "fuzz" / "fixture" / "merged" / "selected_cases.txt").read_text(encoding="utf-8")
    main_rs = (workspace / "fuzz" / "fixture" / "merged" / "main.rs").read_text(encoding="utf-8")
    assert "harness_001_01.rs" in selected_cases
    assert "harness_001_02.rs" not in selected_cases
    assert "afl::fuzz!" in main_rs
    assert "dispatch(&data);" in main_rs


def test_merge_harnesses_cli_writes_merge_report(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    reports_dir = workspace / "reports"
    fuzz_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)
    ok_case = fuzz_dir / "harness_001_01.rs"
    ok_case.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
    (workspace / "crate_config.json").write_text(
        json.dumps(
            {
                "crate_dir": "/tmp/target-crate",
                "package_name": "fixture",
                "crate_import_name": "fixture",
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "ok",
                "reports": [
                    {"harness": str(ok_case), "status": "ok", "exit_code": 0},
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "ok",
                "reports": [
                    {"harness": str(ok_case), "status": "ok", "classification": "completed", "exit_code": 0},
                ],
            }
        ),
        encoding="utf-8",
    )

    main(
        [
            "merge-harnesses",
            "--workspace-dir",
            str(workspace),
            "--round",
            "1",
        ]
    )

    report = json.loads((reports_dir / "merge_fixture.json").read_text(encoding="utf-8"))
    assert report["crate_name"] == "fixture"
    assert report["selected_cases"] == [str(ok_case)]
```

- [ ] **Step 2: Run the merge-focused tests and verify they fail because there is no merge stage yet**

Run: `PYTHONPATH=rag pytest rag/tests/test_merge_harnesses.py rag/tests/test_cli.py rag/tests/test_smoke_run.py -q`
Expected: FAIL because `seraph_rag.cli` has no `merge-harnesses` subcommand and no merged harness writer exists.

- [ ] **Step 3: Implement merge selection, merged source synthesis, and the CLI subcommand**

```python
def render_merged_main(case_count: int) -> str:
    return """mod registry;

use std::fs;
use std::io::{self, Read};

fn read_input() -> Vec<u8> {
    if let Some(path) = std::env::args().nth(1) {
        return fs::read(path).unwrap_or_default();
    }
    let mut data = Vec::new();
    let _ = io::stdin().read_to_end(&mut data);
    data
}

fn dispatch(data: &[u8]) {
    if registry::CASES.is_empty() {
        return;
    }
    let (selector_bytes, payload) = if data.len() >= 2 {
        data.split_at(2)
    } else {
        (&data[..], &[][..])
    };
    let selector = selector_bytes.iter().fold(0usize, |acc, byte| (acc << 8) | (*byte as usize));
    let case = registry::CASES[selector % registry::CASES.len()];
    (case.run)(payload);
}

#[cfg(feature = "seraph_afl")]
fn main() {
    afl::fuzz!(|data: &[u8]| {
        dispatch(data);
    });
}

#[cfg(not(feature = "seraph_afl"))]
fn main() {
    let data = read_input();
    dispatch(&data);
}
"""


def select_merge_cases(compile_index: Path, smoke_index: Path) -> tuple[list[Path], dict[str, str]]:
    compile_reports = {
        entry["harness"]: entry["status"]
        for entry in json.loads(compile_index.read_text(encoding="utf-8")).get("reports", [])
    }
    smoke_reports = {
        entry["harness"]: entry["status"]
        for entry in json.loads(smoke_index.read_text(encoding="utf-8")).get("reports", [])
    }
    selected: list[Path] = []
    excluded: dict[str, str] = {}
    for harness, compile_status in compile_reports.items():
        smoke_status = smoke_reports.get(harness)
        if compile_status != "ok":
            excluded[harness] = "compile_failed"
        elif smoke_status != "ok":
            excluded[harness] = "smoke_failed_or_missing"
        else:
            selected.append(Path(harness))
    return selected, excluded
```

- [ ] **Step 4: Run the merge Python tests and verify they pass**

Run: `PYTHONPATH=rag pytest rag/tests/test_merge_harnesses.py rag/tests/test_cli.py rag/tests/test_smoke_run.py -q`
Expected: PASS

- [ ] **Step 5: Commit the merge stage**

```bash
git add rag/seraph_rag/merge_harnesses.py rag/seraph_rag/cli.py rag/tests/test_merge_harnesses.py rag/tests/test_smoke_run.py rag/tests/test_cli.py
git commit -m "feat: merge passing case harnesses into afl targets"
```

### Task 4: Upgrade the Rust CLI and AFL Bootstrap to the Merged-Target Model

**Files:**
- Modify: `crates/seraph-cli/src/lib.rs`
- Modify: `crates/seraph-cli/src/main.rs`
- Modify: `crates/seraph-cli/tests/bootstrap_fuzz_target.rs`
- Modify: `crates/seraph-cli/tests/run_dry_run.rs`
- Modify: `crates/seraph-cli/tests/run_smoke.rs`
- Modify: `scripts/bootstrap-fuzz-target.sh`

- [ ] **Step 1: Add failing Rust tests for merge planning and merged AFL dry-run rendering**

```rust
#[test]
fn phase3_merge_harnesses_plan_uses_python_sub_cli() {
    let plan = phase3_merge_harnesses_plan("workspace", "1");

    assert_eq!(plan.program, "python3");
    assert!(plan.args.contains(&"merge-harnesses".to_string()));
    assert!(plan.args.contains(&"--workspace-dir".to_string()));
    assert!(plan.args.contains(&"workspace".to_string()));
    assert!(plan.args.contains(&"--round".to_string()));
    assert!(plan.args.contains(&"1".to_string()));
}

#[test]
fn phase3_afl_bootstrap_plan_targets_merged_manifest() {
    let plan = phase3_afl_bootstrap_plan(
        "workspace",
        "workspace/reports/merge_fixture.json",
        Some("stdin"),
        None,
        None,
        None,
        false,
        true,
    );

    assert_eq!(plan.program, "bash");
    assert!(plan.args[0].contains("scripts/bootstrap-fuzz-target.sh"));
    assert!(plan.args.contains(&"--merge-report".to_string()));
    assert!(plan.args.contains(&"workspace/reports/merge_fixture.json".to_string()));
    assert!(plan.args.contains(&"--build-only".to_string()));
}
```

```rust
#[test]
fn bootstrap_fuzz_target_dry_run_prints_regular_asan_and_cmplog_commands() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-merged");
    let merged_dir = workspace.join("fuzz/fixture/merged");
    let project_dir = workspace.join("_cargo_projects/merged_fixture");
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&merged_dir).expect("create merged dir");
    fs::create_dir_all(&src_dir).expect("create project src");
    fs::write(project_dir.join("Cargo.toml"), "[package]\nname = \"merged_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").expect("write Cargo.toml");
    fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("write main.rs");
    fs::write(
        workspace.join("reports/merge_fixture.json"),
        format!(
            "{{\"crate_name\":\"fixture\",\"manifest_path\":\"{}\",\"target_name\":\"merged_fixture\"}}",
            project_dir.join("Cargo.toml").display()
        ),
    )
    .expect("write merge report");

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--merge-report",
            workspace.join("reports/merge_fixture.json").to_str().expect("merge report str"),
            "--dry-run",
        ])
        .output()
        .expect("run bootstrap script");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("AFL_USE_ASAN=1"));
    assert!(stdout.contains("AFL_LLVM_CMPLOG=1"));
    assert!(stdout.contains("-M asan_main"));
    assert!(stdout.contains("-S cmplog_aux"));
    assert!(stdout.contains(" -c "));
}
```

- [ ] **Step 2: Run the Rust tests and verify they fail on the old single-harness bootstrap model**

Run: `cargo test -p seraph-cli phase3_merge_harnesses_plan_uses_python_sub_cli -- --nocapture`
Expected: FAIL because there is no `phase3_merge_harnesses_plan()`.

Run: `cargo test -p seraph-cli phase3_afl_bootstrap_plan_targets_merged_manifest -- --nocapture`
Expected: FAIL because `phase3_afl_bootstrap_plan()` still expects `--harness`.

Run: `cargo test -p seraph-cli --test bootstrap_fuzz_target bootstrap_fuzz_target_dry_run_prints_regular_asan_and_cmplog_commands -- --nocapture`
Expected: FAIL because there is no `phase3_merge_harnesses_plan()`, `phase3_afl_bootstrap_plan()` still expects `--harness`, and the script only renders one `cargo afl build` plus one `afl-fuzz` command.

- [ ] **Step 3: Implement the new Rust plans, run pipeline wiring, and merged AFL script**

```rust
pub fn phase3_merge_harnesses_plan(workspace_dir: &str, round: &str) -> CommandPlan {
    python_module_plan(vec![
        "merge-harnesses".to_string(),
        "--workspace-dir".to_string(),
        workspace_dir.to_string(),
        "--round".to_string(),
        round.to_string(),
    ])
}

pub fn phase3_afl_bootstrap_plan(
    workspace_dir: &str,
    merge_report: &str,
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
        "--merge-report".to_string(),
        merge_report.to_string(),
    ];
    if let Some(value) = input_mode {
        args.push("--input-mode".to_string());
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
```

```bash
regular_target_dir="${afl_target_dir:-$workspace_dir/_afl_target/regular}"
asan_target_dir="${workspace_dir}/_afl_target/asan"
cmplog_target_dir="${workspace_dir}/_afl_target/cmplog"

build_regular_cmd=(env "CARGO_TARGET_DIR=$regular_target_dir" "${cargo_afl_cmd[@]}" build --manifest-path "$manifest_path" --features seraph_afl)
build_asan_cmd=(env "CARGO_TARGET_DIR=$asan_target_dir" AFL_USE_ASAN=1 "${cargo_afl_cmd[@]}" build --manifest-path "$manifest_path" --features seraph_afl)
build_cmplog_cmd=(env "CARGO_TARGET_DIR=$cmplog_target_dir" AFL_LLVM_CMPLOG=1 "${cargo_afl_cmd[@]}" build --manifest-path "$manifest_path" --features seraph_afl)

asan_fuzz_cmd=("${afl_fuzz_cmd[@]}" -M asan_main -i "$corpus_dir" -o "$findings_dir" -- "$asan_binary" "@@")
cmplog_fuzz_cmd=("${afl_fuzz_cmd[@]}" -S cmplog_aux -i "$corpus_dir" -o "$findings_dir" -c "$cmplog_binary" -- "$regular_binary" "@@")
```

- [ ] **Step 4: Run the CLI and script tests again and verify they pass**

Run: `cargo test -p seraph-cli phase3_merge_harnesses_plan_uses_python_sub_cli -- --nocapture`
Expected: PASS

Run: `cargo test -p seraph-cli phase3_afl_bootstrap_plan_targets_merged_manifest -- --nocapture`
Expected: PASS

Run: `cargo test -p seraph-cli --test bootstrap_fuzz_target bootstrap_fuzz_target_dry_run_prints_regular_asan_and_cmplog_commands -- --nocapture`
Expected: PASS

Run: `cargo test -p seraph-cli --test run_dry_run run_dry_run_prints_afl_bootstrap_command -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the merged AFL orchestration slice**

```bash
git add crates/seraph-cli/src/lib.rs crates/seraph-cli/src/main.rs crates/seraph-cli/tests/bootstrap_fuzz_target.rs crates/seraph-cli/tests/run_dry_run.rs crates/seraph-cli/tests/run_smoke.rs scripts/bootstrap-fuzz-target.sh
git commit -m "feat: bootstrap merged afl campaigns with asan and cmplog"
```

### Task 5: Update Documentation and Lock the End-to-End Flow

**Files:**
- Modify: `README.md`
- Modify: `crates/seraph-cli/README.md`
- Modify: `docs/architecture/phase3-harness-generation.md`
- Modify: `crates/seraph-cli/tests/run_dry_run.rs`

- [ ] **Step 1: Add a failing end-to-end dry-run assertion for the new merge stage**

```rust
#[test]
fn run_dry_run_prints_merge_harnesses_before_afl_bootstrap() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--llm-response",
            "/tmp/seraph-llm-response.md",
            "--compile-check",
            "--smoke-command",
            "python3 -c \"import sys; sys.exit(0)\"",
            "--afl-bootstrap",
            "--afl-build-only",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli smoke-run"));
    assert!(stdout.contains("python3 -m seraph_rag.cli merge-harnesses"));
    assert!(stdout.contains("bootstrap-fuzz-target.sh"));
}
```

- [ ] **Step 2: Run the targeted docs-flow integration tests and verify they fail before the last docs/flow updates**

Run: `cargo test -p seraph-cli --test run_dry_run run_dry_run_prints_merge_harnesses_before_afl_bootstrap -- --nocapture`
Expected: FAIL because the current `run` flow jumps from `smoke-run` directly to `afl-bootstrap` and does not print `merge-harnesses`.

- [ ] **Step 3: Update the docs and the final flow examples**

```markdown
- Phase 3 active path is:
  retrieve -> harness prompt -> model response -> case write -> compile-check -> fix-loop -> smoke-run -> merge-harnesses -> AFL bootstrap.
- First-round case generation emits `pub fn run_case(input: &[u8])` modules, not standalone AFL or libFuzzer entrypoints.
- Only compile- and smoke-successful cases are merged into the final crate-level AFL++ target.
- The merged target is built three ways for fuzzing:
  - regular AFL instrumentation
  - ASan instrumentation (`AFL_USE_ASAN=1`)
  - CmpLog helper instrumentation (`AFL_LLVM_CMPLOG=1`)
- The DeepSurf-style launcher uses two cooperating AFL++ jobs:
  - primary ASan job on the ASan binary
  - secondary CmpLog-assisted job on the regular binary with `-c <cmplog_binary>`
```

- [ ] **Step 4: Run the focused Python and Rust verification suite**

Run: `PYTHONPATH=rag pytest rag/tests/test_harness_prompt.py rag/tests/test_harness_codegen.py rag/tests/test_real_crate_runner.py rag/tests/test_merge_harnesses.py rag/tests/test_cli.py rag/tests/test_smoke_run.py -q`
Expected: PASS

Run: `cargo test -p seraph-cli phase3_merge_harnesses_plan_uses_python_sub_cli -- --nocapture`
Expected: PASS

Run: `cargo test -p seraph-cli phase3_afl_bootstrap_plan_targets_merged_manifest -- --nocapture`
Expected: PASS

Run: `cargo test -p seraph-cli --test bootstrap_fuzz_target bootstrap_fuzz_target_dry_run_prints_regular_asan_and_cmplog_commands -- --nocapture`
Expected: PASS

Run: `cargo test -p seraph-cli --test run_dry_run run_dry_run_prints_merge_harnesses_before_afl_bootstrap -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the docs and verification slice**

```bash
git add README.md crates/seraph-cli/README.md docs/architecture/phase3-harness-generation.md crates/seraph-cli/tests/run_dry_run.rs
git commit -m "docs: describe merged aflpp harness workflow"
```

## Self-Review Checklist

- Spec coverage:
  - case-module generation contract is covered by Task 1
  - one-shot wrapper screening is covered by Task 2
  - merge selection and merged source synthesis are covered by Task 3
  - regular/ASan/CmpLog AFL orchestration is covered by Task 4
  - docs and end-to-end dry-run coverage are covered by Task 5
- Placeholder scan:
  - no placeholder markers or “similar to Task N” instructions remain
  - every code-changing step includes concrete function names or command snippets
- Type and naming consistency:
  - `run_case(input: &[u8])` is the only case entrypoint name used in all tasks
  - `merge-harnesses` is the only merge CLI subcommand name used in all tasks
  - `phase3_merge_harnesses_plan()` and `phase3_afl_bootstrap_plan(..., merge_report, ...)` are used consistently throughout
