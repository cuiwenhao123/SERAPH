# Selector-Safe Seeds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify SERAPH's merged-harness AFL++ bootstrap flow so every standard merged target automatically gets one selector-safe seed per selected small case and `bootstrap-fuzz-target.sh` consumes that corpus by default.

**Architecture:** Keep selector-safe seed generation on the merge side because `write_merged_harnesses()` already owns the authoritative `selected_cases` list. Extend the merge report with corpus metadata, generate the default corpus under `workspace/afl/<target_name>/corpus`, and make `bootstrap-fuzz-target.sh` prefer that merge-provided corpus unless the caller explicitly overrides `--corpus-dir`.

**Tech Stack:** Python (`rag/seraph_rag`), pytest, shell (`scripts/bootstrap-fuzz-target.sh`), Rust integration tests (`crates/seraph-cli/tests`)

---

## File Structure

- Modify: `rag/seraph_rag/merge_harnesses.py`
  - Generate selector-safe seeds, emit merge report `v2` corpus fields, and write `corpus_meta/summary.json`.
- Modify: `rag/tests/test_merge_harnesses.py`
  - Lock seed generation, byte layout, metadata schema, and case-count guardrails.
- Modify: `scripts/bootstrap-fuzz-target.sh`
  - Read `default_corpus_dir` from the merge report when `--corpus-dir` is not explicitly supplied, while keeping the empty-directory `seed.bin` fallback.
- Modify: `crates/seraph-cli/tests/bootstrap_fuzz_target.rs`
  - Lock dry-run behavior for merge-provided corpora, explicit overrides, and backward-compatible fallback.
- Modify: `scripts/README.md`
  - Document that standard merged-harness bootstrap now starts from selector-safe seeds by default.
- Modify: `crates/seraph-cli/README.md`
  - Update the merged-harness `afl-bootstrap` explanation to reflect merge-generated default corpora.

## Task 1: Generate Selector-Safe Corpora During Merge

**Files:**
- Modify: `rag/tests/test_merge_harnesses.py`
- Modify: `rag/seraph_rag/merge_harnesses.py`

- [ ] **Step 1: Write failing tests for selector-safe seed files and merge-report metadata**

```python
import json

import pytest

from seraph_rag.merge_harnesses import write_merged_harnesses


def test_write_merged_harnesses_generates_selector_safe_seed_files(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    reports_dir = workspace / "reports"
    fuzz_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    case_a = fuzz_dir / "harness_001_01.rs"
    case_b = fuzz_dir / "harness_001_02.rs"
    case_a.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
    case_b.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
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
                    {"harness": str(case_a), "status": "ok", "exit_code": 0},
                    {"harness": str(case_b), "status": "ok", "exit_code": 0},
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
                    {"harness": str(case_a), "status": "ok", "classification": "completed", "exit_code": 0},
                    {"harness": str(case_b), "status": "ok", "classification": "completed", "exit_code": 0},
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

    corpus_dir = workspace / "afl" / "merged_fixture" / "corpus"
    summary_path = workspace / "afl" / "merged_fixture" / "corpus_meta" / "summary.json"
    seed0 = corpus_dir / "selector_0000.bin"
    seed1 = corpus_dir / "selector_0001.bin"
    summary = json.loads(summary_path.read_text(encoding="utf-8"))

    assert report["version"] == "seraph.phase3.merge_harnesses.v2"
    assert report["selected_case_count"] == 2
    assert report["safe_seed_count"] == 2
    assert report["default_corpus_dir"] == str(corpus_dir)
    assert report["default_seed_files"] == [str(seed0), str(seed1)]
    assert seed0.read_bytes() == b"\x00\x00"
    assert seed1.read_bytes() == b"\x00\x01"
    assert summary["selected_case_count"] == 2
    assert summary["safe_seed_count"] == 2
    assert summary["crash_seed_count"] == 0
    assert summary["seed_files"] == [str(seed0), str(seed1)]


def test_write_merged_harnesses_rejects_more_than_u16_cases(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    reports_dir = workspace / "reports"
    fuzz_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)
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

    reports = []
    for index in range(65537):
        harness = fuzz_dir / "harness_{:05d}.rs".format(index)
        harness.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
        reports.append({"harness": str(harness), "status": "ok", "exit_code": 0})

    (reports_dir / "compile_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": reports}),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "ok",
                "reports": [
                    {**entry, "classification": "completed"}
                    for entry in reports
                ],
            }
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="selector-safe seed limit"):
        write_merged_harnesses(
            workspace_dir=workspace,
            round_no=1,
            crate_name="fixture",
            crate_import_name="fixture",
        )
```

- [ ] **Step 2: Run the merge tests to verify they fail**

Run: `PYTHONPATH=rag pytest rag/tests/test_merge_harnesses.py -q`
Expected: FAIL because `write_merged_harnesses()` currently writes only merged source files and a `v1` report; it does not create selector seeds, corpus metadata, or a case-count guard.

- [ ] **Step 3: Implement selector-safe seed generation and merge-report `v2` fields**

```python
def _selector_seed_bytes(selector: int) -> bytes:
    return selector.to_bytes(2, byteorder="big", signed=False)


def _write_selector_safe_corpus(
    workspace: Path,
    target_name: str,
    manifest_path: Path,
    merge_report_path: Path,
    selected_cases: List[Path],
) -> Dict[str, object]:
    selected_case_count = len(selected_cases)
    if selected_case_count > 0x10000:
        raise ValueError(
            "selector-safe seed limit exceeded: {} selected cases > 65536".format(
                selected_case_count
            )
        )

    corpus_dir = workspace / "afl" / target_name / "corpus"
    meta_dir = workspace / "afl" / target_name / "corpus_meta"
    corpus_dir.mkdir(parents=True, exist_ok=True)
    meta_dir.mkdir(parents=True, exist_ok=True)

    seed_files: List[str] = []
    for selector in range(selected_case_count):
        seed_path = corpus_dir / "selector_{:04d}.bin".format(selector)
        seed_path.write_bytes(_selector_seed_bytes(selector))
        seed_files.append(str(seed_path))

    summary = {
        "version": "seraph.phase3.selector_safe_corpus.v1",
        "target_name": target_name,
        "manifest_path": str(manifest_path),
        "merge_report": str(merge_report_path),
        "corpus_dir": str(corpus_dir),
        "meta_dir": str(meta_dir),
        "selected_case_count": selected_case_count,
        "safe_seed_count": selected_case_count,
        "crash_seed_count": 0,
        "seed_files": seed_files,
    }
    (meta_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return {
        "selected_case_count": selected_case_count,
        "safe_seed_count": selected_case_count,
        "default_corpus_dir": str(corpus_dir),
        "default_corpus_meta_dir": str(meta_dir),
        "default_seed_files": seed_files,
    }
```

Update `write_merged_harnesses()` so it:

- computes `report_path` before the final report payload
- calls `_write_selector_safe_corpus(...)`
- upgrades `version` to `seraph.phase3.merge_harnesses.v2`
- merges the returned corpus fields into the final report

- [ ] **Step 4: Run the merge tests again and verify they pass**

Run: `PYTHONPATH=rag pytest rag/tests/test_merge_harnesses.py -q`
Expected: PASS

- [ ] **Step 5: Commit the merge-side seed generation slice**

```bash
git add rag/tests/test_merge_harnesses.py rag/seraph_rag/merge_harnesses.py
git commit -m "feat: generate selector-safe corpora for merged harnesses"
```

## Task 2: Make AFL Bootstrap Prefer the Merge-Generated Corpus

**Files:**
- Modify: `crates/seraph-cli/tests/bootstrap_fuzz_target.rs`
- Modify: `scripts/bootstrap-fuzz-target.sh`

- [ ] **Step 1: Write failing dry-run tests for default corpus selection, override behavior, and backward-compatible fallback**

```rust
#[test]
fn bootstrap_fuzz_target_dry_run_prefers_merge_report_default_corpus_dir() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-selector-default");
    let merge_report = write_minimal_workspace(&workspace);
    let selector_corpus = workspace.join("prebuilt-corpus");
    fs::create_dir_all(&selector_corpus).expect("create selector corpus");
    fs::write(selector_corpus.join("selector_0000.bin"), [0_u8, 0_u8]).expect("write seed");

    fs::write(
        &merge_report,
        format!(
            "{{\"crate_name\":\"fixture\",\"manifest_path\":\"{}\",\"target_name\":\"merged_fixture\",\"default_corpus_dir\":\"{}\"}}",
            workspace.join("_cargo_projects/merged_fixture/Cargo.toml").display(),
            selector_corpus.display()
        ),
    )
    .expect("rewrite merge report");

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--merge-report",
            merge_report.to_str().expect("merge report str"),
            "--dry-run",
        ])
        .output()
        .expect("run bootstrap script");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(selector_corpus.to_str().expect("selector corpus str")));
}


#[test]
fn bootstrap_fuzz_target_dry_run_preserves_explicit_corpus_override() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-selector-override");
    let merge_report = write_minimal_workspace(&workspace);
    let explicit_corpus = workspace.join("manual-corpus");
    fs::create_dir_all(&explicit_corpus).expect("create manual corpus");

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--merge-report",
            merge_report.to_str().expect("merge report str"),
            "--corpus-dir",
            explicit_corpus.to_str().expect("corpus str"),
            "--dry-run",
        ])
        .output()
        .expect("run bootstrap script");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(explicit_corpus.to_str().expect("corpus str")));
}


#[test]
fn bootstrap_fuzz_target_dry_run_falls_back_without_default_corpus_dir() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-selector-legacy");
    let merge_report = write_minimal_workspace(&workspace);

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--merge-report",
            merge_report.to_str().expect("merge report str"),
            "--dry-run",
        ])
        .output()
        .expect("run bootstrap script");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(
        workspace
            .join("afl/merged_fixture/corpus")
            .to_str()
            .expect("fallback corpus str")
    ));
}
```

- [ ] **Step 2: Run the bootstrap dry-run tests to verify they fail**

Run: `cargo test -p seraph-cli --test bootstrap_fuzz_target -- --nocapture`
Expected: FAIL because `bootstrap-fuzz-target.sh` currently ignores `default_corpus_dir` in the merge report and always computes the default corpus path as `workspace/afl/<target_name>/corpus`.

- [ ] **Step 3: Update `bootstrap-fuzz-target.sh` to honor merge-provided default corpora**

```bash
read -r manifest_path target_name default_corpus_dir < <(
  python3 - "$merge_report_path" <<'PY'
import json
import sys

payload = json.load(open(sys.argv[1], "r", encoding="utf-8"))
print(
    "\t".join(
        [
            payload["manifest_path"],
            payload["target_name"],
            payload.get("default_corpus_dir", ""),
        ]
    )
)
PY
)

campaign_root="$workspace_dir/afl/$target_name"
if [[ -z "$corpus_dir" ]]; then
  if [[ -n "$default_corpus_dir" ]]; then
    corpus_dir="$default_corpus_dir"
  else
    corpus_dir="$campaign_root/corpus"
  fi
fi
findings_dir="${findings_dir:-$campaign_root/findings}"
```

Do not remove the existing empty-directory fallback:

```bash
mkdir -p "$corpus_dir"
if ! find "$corpus_dir" -type f -print -quit | grep -q .; then
  head -c 64 /dev/zero > "$corpus_dir/seed.bin"
fi
```

- [ ] **Step 4: Run the bootstrap dry-run tests again and verify they pass**

Run: `cargo test -p seraph-cli --test bootstrap_fuzz_target -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the bootstrap-selection slice**

```bash
git add crates/seraph-cli/tests/bootstrap_fuzz_target.rs scripts/bootstrap-fuzz-target.sh
git commit -m "feat: use merge-generated selector-safe corpora by default"
```

## Task 3: Document the Unified Default and Run Cross-Stack Verification

**Files:**
- Modify: `scripts/README.md`
- Modify: `crates/seraph-cli/README.md`

- [ ] **Step 1: Add a small documentation regression by describing the new default-corpus behavior**

```md
- `merge-harnesses` now generates a selector-safe default corpus under `workspace/afl/<target_name>/corpus`.
- `bootstrap-fuzz-target.sh` uses that merge-generated corpus automatically unless `--corpus-dir` is explicitly passed.
- The zeroed fallback `seed.bin` is now only a fallback for empty or manually supplied corpora.
```

- [ ] **Step 2: Update the docs to match the implemented behavior**

In `scripts/README.md`, update the `bootstrap-fuzz-target.sh` section so it no longer implies the standard path begins from an empty corpus.

In `crates/seraph-cli/README.md`, update the merged-harness `afl-bootstrap` description to explain that:

- merge creates selector-safe seeds
- bootstrap consumes them automatically
- explicit `--afl-corpus-dir` still overrides the default

- [ ] **Step 3: Run the targeted verification suites**

Run: `PYTHONPATH=rag pytest rag/tests/test_merge_harnesses.py -q`
Expected: PASS

Run: `cargo test -p seraph-cli --test bootstrap_fuzz_target -- --nocapture`
Expected: PASS

Run: `PYTHONPATH=rag pytest rag/tests/test_merge_harnesses.py rag/tests/test_cli.py -q`
Expected: PASS

- [ ] **Step 4: Run one manual dry-run check against the repository script output**

Run:

```bash
bash scripts/bootstrap-fuzz-target.sh \
  --workspace-dir /tmp/seraph-selector-safe-demo \
  --merge-report /tmp/seraph-selector-safe-demo/reports/merge_fixture.json \
  --dry-run
```

Expected: the rendered `asan_fuzz=` and `cmplog_fuzz=` commands point at the merge-generated `default_corpus_dir` when that field exists in the report.

- [ ] **Step 5: Commit the docs-and-verification slice**

```bash
git add scripts/README.md crates/seraph-cli/README.md
git commit -m "docs: describe selector-safe merged-harness corpora"
```

## Self-Review

- Spec coverage:
  - merge-time selector seed generation: covered by Task 1
  - merge report `v2` corpus fields: covered by Task 1
  - `corpus_meta/summary.json`: covered by Task 1
  - bootstrap default corpus resolution and explicit override precedence: covered by Task 2
  - backward-compatible fallback: covered by Task 2
  - docs and verification: covered by Task 3
- Placeholder scan:
  - no `TODO`, `TBD`, or “similar to Task N” placeholders remain
- Type consistency:
  - plan consistently uses `default_corpus_dir`, `default_corpus_meta_dir`, `default_seed_files`, `selected_case_count`, and `safe_seed_count`
