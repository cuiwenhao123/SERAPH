use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates")
        .parent()
        .expect("repo")
        .to_path_buf()
}

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("seraph-{name}-{unique}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_batch_knowledge(path: &Path) {
    let payload = serde_json::json!({
        "crate_meta": {
            "package_name": "batch_fixture",
            "lib_target_name": "batch_fixture",
            "crate_import_name": "batch_fixture",
            "version": "0.1.0",
            "edition": "2021",
            "rust_version": null,
            "repository": null,
            "manifest_path": "/tmp/batch_fixture/Cargo.toml",
            "lib_rs_path": "/tmp/batch_fixture/src/lib.rs",
            "default_features": [],
            "cargo_description": "Batch fixture crate",
            "root_docs": "Batch fixture docs.",
            "root_doc_sections": {
                "summary": "Batch fixture docs.",
                "panics": "",
                "errors": "",
                "safety": "",
                "examples": ""
            }
        },
        "modules": [
            {
                "module_id": "mod::batch_fixture",
                "name": "batch_fixture",
                "canonical_path": "batch_fixture",
                "public_paths": ["batch_fixture"],
                "parent_module_id": null,
                "code_ref": {"file": "src/lib.rs", "line": 1, "column": 1},
                "docs": "",
                "doc_sections": {"summary": "", "panics": "", "errors": "", "safety": "", "examples": ""}
            }
        ],
        "types": [
            {
                "type_id": "type::batch_fixture::Buffer",
                "name": "Buffer",
                "canonical_path": "batch_fixture::Buffer",
                "public_paths": ["batch_fixture::Buffer"],
                "public_anchor_module_id": "mod::batch_fixture",
                "code_ref": {"file": "src/lib.rs", "line": 3, "column": 1},
                "docs": "Buffer type.",
                "doc_sections": {"summary": "Buffer type.", "panics": "", "errors": "", "safety": "", "examples": ""},
                "kind": "struct",
                "generic_params": [],
                "where_clauses": [],
                "is_non_exhaustive": false,
                "fields": [],
                "variants": [],
                "has_hidden_fields": true,
                "has_hidden_variants": false
            },
            {
                "type_id": "type::batch_fixture::Cursor",
                "name": "Cursor",
                "canonical_path": "batch_fixture::Cursor",
                "public_paths": ["batch_fixture::Cursor"],
                "public_anchor_module_id": "mod::batch_fixture",
                "code_ref": {"file": "src/lib.rs", "line": 20, "column": 1},
                "docs": "Cursor type.",
                "doc_sections": {"summary": "Cursor type.", "panics": "", "errors": "", "safety": "", "examples": ""},
                "kind": "struct",
                "generic_params": [],
                "where_clauses": [],
                "is_non_exhaustive": false,
                "fields": [],
                "variants": [],
                "has_hidden_fields": true,
                "has_hidden_variants": false
            }
        ],
        "apis": [
            {
                "api_id": "api::batch_fixture::Buffer::new",
                "name": "new",
                "canonical_path": "batch_fixture::Buffer::new",
                "public_paths": ["batch_fixture::Buffer::new"],
                "module_id": "mod::batch_fixture",
                "owner_type_id": "type::batch_fixture::Buffer",
                "code_ref": {"file": "src/lib.rs", "line": 7, "column": 5},
                "docs": "Creates a buffer.",
                "doc_sections": {"summary": "Creates a buffer.", "panics": "", "errors": "", "safety": "", "examples": ""},
                "signature": "pub fn new() -> Buffer",
                "signature_text": "pub fn new() -> Buffer",
                "receiver": null,
                "return_type": "Buffer",
                "arg_types": [],
                "generic_params": [],
                "where_clauses": [],
                "api_kind": "constructor",
                "is_unsafe": false,
                "contains_unsafe_block": false
            },
            {
                "api_id": "api::batch_fixture::Buffer::get_unchecked",
                "name": "get_unchecked",
                "canonical_path": "batch_fixture::Buffer::get_unchecked",
                "public_paths": ["batch_fixture::Buffer::get_unchecked"],
                "module_id": "mod::batch_fixture",
                "owner_type_id": "type::batch_fixture::Buffer",
                "code_ref": {"file": "src/lib.rs", "line": 11, "column": 5},
                "docs": "Reads without checks.",
                "doc_sections": {"summary": "Reads without checks.", "panics": "", "errors": "", "safety": "index must be in bounds", "examples": ""},
                "signature": "pub unsafe fn get_unchecked(&self, index: usize) -> u8",
                "signature_text": "pub unsafe fn get_unchecked(&self, index: usize) -> u8",
                "receiver": "&self",
                "return_type": "u8",
                "arg_types": ["usize"],
                "generic_params": [],
                "where_clauses": [],
                "api_kind": "method",
                "is_unsafe": true,
                "contains_unsafe_block": false
            },
            {
                "api_id": "api::batch_fixture::Cursor::new",
                "name": "new",
                "canonical_path": "batch_fixture::Cursor::new",
                "public_paths": ["batch_fixture::Cursor::new"],
                "module_id": "mod::batch_fixture",
                "owner_type_id": "type::batch_fixture::Cursor",
                "code_ref": {"file": "src/lib.rs", "line": 24, "column": 5},
                "docs": "Creates a cursor.",
                "doc_sections": {"summary": "Creates a cursor.", "panics": "", "errors": "", "safety": "", "examples": ""},
                "signature": "pub fn new() -> Cursor",
                "signature_text": "pub fn new() -> Cursor",
                "receiver": null,
                "return_type": "Cursor",
                "arg_types": [],
                "generic_params": [],
                "where_clauses": [],
                "api_kind": "constructor",
                "is_unsafe": false,
                "contains_unsafe_block": false
            },
            {
                "api_id": "api::batch_fixture::Cursor::advance",
                "name": "advance",
                "canonical_path": "batch_fixture::Cursor::advance",
                "public_paths": ["batch_fixture::Cursor::advance"],
                "module_id": "mod::batch_fixture",
                "owner_type_id": "type::batch_fixture::Cursor",
                "code_ref": {"file": "src/lib.rs", "line": 28, "column": 5},
                "docs": "Advances cursor using internal pointer math.",
                "doc_sections": {"summary": "Advances cursor using internal pointer math.", "panics": "", "errors": "", "safety": "", "examples": ""},
                "signature": "pub fn advance(&mut self, amount: usize)",
                "signature_text": "pub fn advance(&mut self, amount: usize)",
                "receiver": "&mut self",
                "return_type": "()",
                "arg_types": ["usize"],
                "generic_params": [],
                "where_clauses": [],
                "api_kind": "method",
                "is_unsafe": false,
                "contains_unsafe_block": true
            }
        ],
        "symbols": [],
        "trait_registry": [],
        "trait_impl_registry": [],
        "examples": [],
        "risk_facts": {
            "unsafe_functions": ["api::batch_fixture::Buffer::get_unchecked"],
            "unsafe_blocks": ["api::batch_fixture::Cursor::advance"],
            "ffi_functions": [],
            "panic_sites": []
        }
    });
    fs::write(
        path,
        serde_json::to_string_pretty(&payload).expect("encode batch fixture") + "\n",
    )
    .expect("write batch fixture");
}

#[test]
fn run_smoke_writes_validated_coverage_json() {
    let repo = repo_root();
    let workspace = temp_dir("run-smoke");
    let model_script = workspace.join("fake_model.py");
    fs::write(
        &model_script,
        r#"import json, sys
prompt = json.load(open(sys.argv[1], "r", encoding="utf-8"))
target = prompt["target_api_id"]
if target.endswith("get_unchecked"):
    print(f'''
struct Buffer;
impl Buffer {{
    unsafe fn get_unchecked(&self, index: usize) -> u8 {{
        index as u8
    }}
}}

fn main() {{
    let buffer = Buffer;
    println!("SERAPH_STEP_ENTER:1:{target}");
    let _ = unsafe {{ buffer.get_unchecked(0) }};
    println!("SERAPH_STEP_OK:1:{target}");
}}
''')
elif target.endswith("advance"):
    print(f'''
struct Cursor;
impl Cursor {{
    fn advance(&mut self, amount: usize) {{
        let _ = amount;
    }}
}}

fn main() {{
    let mut cursor = Cursor;
    println!("SERAPH_STEP_ENTER:1:{target}");
    cursor.advance(1);
    println!("SERAPH_STEP_OK:1:{target}");
}}
''')
else:
    raise SystemExit(target)
"#,
    )
    .expect("write model script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(0)'",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let coverage_path = workspace.join("coverage.json");
    assert!(coverage_path.exists());
    let coverage = fs::read_to_string(&coverage_path).expect("read coverage");
    assert!(coverage.contains("\"validated\""));
    assert!(coverage.contains("\"coverage_rate\": 1.0"));
}

#[test]
fn run_executes_all_targets_by_default() {
    let repo = repo_root();
    let workspace = temp_dir("run-batch-all-targets");
    let knowledge_path = workspace.join("batch_knowledge.json");
    let model_script = workspace.join("fake_model.py");
    write_batch_knowledge(&knowledge_path);
    fs::write(
        &model_script,
        r#"import json, sys
prompt = json.load(open(sys.argv[1], "r", encoding="utf-8"))
target = prompt["target_api_id"]
if target.endswith("get_unchecked"):
    print(f'''
struct Buffer;
impl Buffer {{
    unsafe fn get_unchecked(&self, index: usize) -> u8 {{
        index as u8
    }}
}}

fn main() {{
    let buffer = Buffer;
    println!("SERAPH_STEP_ENTER:1:{target}");
    let _ = unsafe {{ buffer.get_unchecked(0) }};
    println!("SERAPH_STEP_OK:1:{target}");
}}
''')
elif target.endswith("advance"):
    print(f'''
struct Cursor;
impl Cursor {{
    fn advance(&mut self, amount: usize) {{
        let _ = amount;
    }}
}}

fn main() {{
    let mut cursor = Cursor;
    println!("SERAPH_STEP_ENTER:1:{target}");
    cursor.advance(1);
    println!("SERAPH_STEP_OK:1:{target}");
}}
''')
else:
    raise SystemExit(target)
"#,
    )
    .expect("write model script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            knowledge_path.to_str().expect("knowledge str"),
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--variants",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(0)'",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let coverage: Value = serde_json::from_str(
        &fs::read_to_string(workspace.join("coverage.json")).expect("read coverage"),
    )
    .expect("parse coverage");
    let total_api_ids = coverage["total_api_ids"]
        .as_array()
        .expect("total_api_ids array");
    assert_eq!(total_api_ids.len(), 2);
    assert_eq!(coverage["coverage_rate"], serde_json::json!(1.0));
    assert_eq!(
        coverage["api_status"]["api::batch_fixture::Buffer::get_unchecked"],
        "validated"
    );
    assert_eq!(
        coverage["api_status"]["api::batch_fixture::Cursor::advance"],
        "validated"
    );

    let context_001 =
        fs::read_to_string(workspace.join("contexts/rag_target_001.md")).expect("read context 1");
    let context_002 =
        fs::read_to_string(workspace.join("contexts/rag_target_002.md")).expect("read context 2");
    assert!(context_001.contains("api::batch_fixture::Buffer::get_unchecked"));
    assert!(context_002.contains("api::batch_fixture::Cursor::advance"));

    assert!(workspace.join("reports/compile_001_index.json").exists());
    assert!(workspace.join("reports/compile_002_index.json").exists());
    assert!(workspace.join("fuzz/harness_001_01.rs").exists());
    assert!(workspace.join("fuzz/harness_002_01.rs").exists());
}

#[test]
fn run_pipeline_injects_pythonpath_for_python_subcli() {
    let repo = repo_root();
    let workspace = temp_dir("run-pythonpath");
    let model_script = workspace.join("fake_model.py");
    fs::write(
        &model_script,
        r#"import json, sys
prompt = json.load(open(sys.argv[1], "r", encoding="utf-8"))
target = prompt["target_api_id"]
print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); }}')
"#,
    )
    .expect("write model script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env_remove("PYTHONPATH")
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(0)'",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(workspace.join("coverage.json").exists());
}

#[test]
fn run_smoke_writes_found_bug_to_coverage_json() {
    let repo = repo_root();
    let workspace = temp_dir("run-smoke-bug");
    let model_script = workspace.join("fake_model.py");
    let runtime_script = workspace.join("fake_runtime.py");
    fs::write(
        &model_script,
        r#"import json, sys
prompt = json.load(open(sys.argv[1], "r", encoding="utf-8"))
target = prompt["target_api_id"]
print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); }}')
"#,
    )
    .expect("write model script");
    fs::write(
        &runtime_script,
        r#"import json, sys
payload = json.load(open(sys.argv[1], "r", encoding="utf-8"))
print(json.dumps({
    "classification": "asan_bug",
    "summary": f"asan while reaching {payload['target_api_id']}",
    "is_bug": True,
}))
"#,
    )
    .expect("write runtime script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(0)'",
            "--smoke-command",
            "python3 -c 'import sys; print(\"==1==ERROR: AddressSanitizer: heap-buffer-overflow\", file=sys.stderr); sys.exit(1)'",
            "--runtime-model-command",
            &format!("python3 {} {{input}}", runtime_script.display()),
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let coverage_path = workspace.join("coverage.json");
    let coverage = fs::read_to_string(&coverage_path).expect("read coverage");
    assert!(coverage.contains("\"found_bugs\": ["));
    assert!(coverage.contains("fn::fixture_crate::Buffer::get_unchecked"));
    let runtime_error_index =
        fs::read_to_string(workspace.join("reports/runtime_error_001_index.json"))
            .expect("read runtime error index");
    assert!(runtime_error_index.contains("asan_bug"));
}

#[test]
fn run_smoke_failure_still_writes_validated_coverage_and_runtime_error_index() {
    let repo = repo_root();
    let workspace = temp_dir("run-smoke-runtime-error");
    let model_script = workspace.join("fake_model.py");
    let runtime_script = workspace.join("fake_runtime.py");
    fs::write(
        &model_script,
        r#"import json, sys
prompt = json.load(open(sys.argv[1], "r", encoding="utf-8"))
target = prompt["target_api_id"]
print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); }}')
"#,
    )
    .expect("write model script");
    fs::write(
        &runtime_script,
        r#"import json, sys
payload = json.load(open(sys.argv[1], "r", encoding="utf-8"))
print(json.dumps({
    "classification": "panic_or_crash",
    "summary": f"panic after reaching {payload['target_api_id']}",
    "is_bug": False,
}))
"#,
    )
    .expect("write runtime script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(0)'",
            "--smoke-command",
            "python3 -c 'import sys; print(\"panicked at smoke\", file=sys.stderr); sys.exit(101)'",
            "--runtime-model-command",
            &format!("python3 {} {{input}}", runtime_script.display()),
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let coverage = fs::read_to_string(workspace.join("coverage.json")).expect("read coverage");
    assert!(coverage.contains("\"validated\""));
    assert!(coverage.contains("\"found_bugs\": []"));
    let runtime_error_index =
        fs::read_to_string(workspace.join("reports/runtime_error_001_index.json"))
            .expect("read runtime error index");
    assert!(runtime_error_index.contains("panic_or_crash"));
}

#[test]
fn run_rerun_overwrites_previous_validated_coverage_with_latest_attempted_result() {
    let repo = repo_root();
    let workspace = temp_dir("run-rerun-overwrite");
    let model_script = workspace.join("fake_model.py");
    fs::write(
        &model_script,
        r#"import json, sys
prompt = json.load(open(sys.argv[1], "r", encoding="utf-8"))
target = prompt["target_api_id"]
print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); }}')
"#,
    )
    .expect("write model script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let first = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(0)'",
        ])
        .output()
        .expect("run first seraph-cli");

    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            "python3 -c 'import sys; sys.exit(2)'",
        ])
        .output()
        .expect("run second seraph-cli");

    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let coverage: Value = serde_json::from_str(
        &fs::read_to_string(workspace.join("coverage.json")).expect("read coverage"),
    )
    .expect("parse coverage");

    assert_eq!(
        coverage["api_status"]["fn::fixture_crate::Buffer::get_unchecked"],
        "attempted"
    );
    assert_eq!(coverage["covered_api_ids"], serde_json::json!([]));
    assert_eq!(
        coverage["failed_attempts"]["fn::fixture_crate::Buffer::get_unchecked"]["last_reason"],
        "compile_failed"
    );
    assert_eq!(coverage["coverage_rate"], 0.0);
}

#[test]
fn run_fix_loop_smokes_successful_fixed_harnesses() {
    let repo = repo_root();
    let workspace = temp_dir("run-smoke-fixed");
    let model_script = workspace.join("fake_model.py");
    let compile_script = workspace.join("fake_compile.py");
    let smoke_script = workspace.join("fake_smoke.py");
    fs::write(
        &model_script,
        r#"import json, sys
payload = json.load(open(sys.argv[1], "r", encoding="utf-8"))
version = payload["version"]
target = payload.get("target_api_id", "fn::fixture_crate::Buffer::get_unchecked")
if version == "seraph.phase3.prompt.v1":
    print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); BROKEN }}')
elif version == "seraph.phase3.compile_fixer_request.v1":
    print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); }}')
else:
    raise SystemExit(version)
"#,
    )
    .expect("write model script");
    fs::write(
        &compile_script,
        r#"from pathlib import Path
import sys
import re

harness = Path(sys.argv[1])
text = harness.read_text(encoding="utf-8")
if "BROKEN" in text:
    raise SystemExit(1)

workspace = harness.parent.parent
project = workspace / "_cargo_projects" / harness.stem
src = project / "src"
src.mkdir(parents=True, exist_ok=True)
package_name = re.sub(r"[^A-Za-z0-9_]+", "_", harness.stem)
(project / "Cargo.toml").write_text(
    f"[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    encoding="utf-8",
)
(src / "main.rs").write_text(text, encoding="utf-8")
"#,
    )
    .expect("write compile script");
    fs::write(
        &smoke_script,
        r#"from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
if "SERAPH_STEP_OK" not in text:
    raise SystemExit(3)
print(path.name)
"#,
    )
    .expect("write smoke script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--variants",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            &format!("python3 {} {{harness}}", compile_script.display()),
            "--fix-loop",
            "--fix-model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--fix-max-attempts",
            "2",
            "--smoke-command",
            &format!("python3 {} {{harness}}", smoke_script.display()),
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let smoke_index = fs::read_to_string(workspace.join("reports/smoke_001_index.json"))
        .expect("read smoke index");
    assert!(smoke_index.contains("harness_001_01_fixed_01.rs"));
    assert!(smoke_index.contains("\"report_count\": 1"));
    let coverage = fs::read_to_string(workspace.join("coverage.json")).expect("read coverage");
    assert!(coverage.contains("\"validated\""));
    assert!(coverage.contains("harness_001_01_fixed_01.rs"));
}

#[test]
fn run_afl_bootstrap_build_only_prefers_successful_fixed_harness() {
    let repo = repo_root();
    let workspace = temp_dir("run-afl-bootstrap-fixed");
    let model_script = workspace.join("fake_model.py");
    let compile_script = workspace.join("fake_compile.py");
    let smoke_script = workspace.join("fake_smoke.py");
    let cargo_afl_script = workspace.join("fake_cargo_afl.py");

    fs::write(
        &model_script,
        r#"import json, sys
payload = json.load(open(sys.argv[1], "r", encoding="utf-8"))
version = payload["version"]
target = payload.get("target_api_id", "fn::fixture_crate::Buffer::get_unchecked")
if version == "seraph.phase3.prompt.v1":
    print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); BROKEN }}')
elif version == "seraph.phase3.compile_fixer_request.v1":
    print(f'fn main() {{ println!("SERAPH_STEP_ENTER:1:{target}"); println!("SERAPH_STEP_OK:1:{target}"); }}')
else:
    raise SystemExit(version)
"#,
    )
    .expect("write model script");
    fs::write(
        &compile_script,
        r#"from pathlib import Path
import re
import sys

harness = Path(sys.argv[1])
text = harness.read_text(encoding="utf-8")
if "BROKEN" in text:
    raise SystemExit(1)

workspace = harness.parent.parent
project = workspace / "_cargo_projects" / harness.stem
src = project / "src"
src.mkdir(parents=True, exist_ok=True)
package_name = re.sub(r"[^A-Za-z0-9_]+", "_", harness.stem)
(project / "Cargo.toml").write_text(
    f"[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    encoding="utf-8",
)
(src / "main.rs").write_text(text, encoding="utf-8")
"#,
    )
    .expect("write compile script");
    fs::write(
        &smoke_script,
        r#"from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
if "SERAPH_STEP_OK" not in text:
    raise SystemExit(3)
print(path.name)
"#,
    )
    .expect("write smoke script");
    fs::write(
        &cargo_afl_script,
        r##"#!/usr/bin/env python3
import os
import sys
from pathlib import Path

args = sys.argv[1:]
if "--help" in args:
    print("fake cargo afl")
    raise SystemExit(0)
if not args or args[0] != "build":
    raise SystemExit("expected build")
manifest = Path(args[args.index("--manifest-path") + 1])
package_name = None
for line in manifest.read_text(encoding="utf-8").splitlines():
    if line.startswith("name = "):
        package_name = line.split("=", 1)[1].strip().strip('"')
        break
if package_name is None:
    raise SystemExit("missing package name")
target_dir = Path(os.environ["CARGO_TARGET_DIR"]) / "debug"
target_dir.mkdir(parents=True, exist_ok=True)
binary = target_dir / package_name
binary.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
binary.chmod(0o755)
print(binary)
"##,
    )
    .expect("write fake cargo afl script");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .env("SERAPH_EMBEDDER", "hashing")
        .env(
            "SERAPH_CARGO_AFL_COMMAND",
            format!("python3 {}", cargo_afl_script.display()),
        )
        .args([
            "run",
            "--knowledge",
            "rag/tests/fixtures/minimal_knowledge.json",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--round",
            "1",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--variants",
            "1",
            "--model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--compile-check",
            "--compile-command",
            &format!("python3 {} {{harness}}", compile_script.display()),
            "--fix-loop",
            "--fix-model-command",
            &format!("python3 {} {{input}}", model_script.display()),
            "--fix-max-attempts",
            "2",
            "--smoke-command",
            &format!("python3 {} {{harness}}", smoke_script.display()),
            "--afl-bootstrap",
            "--afl-build-only",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let fixed_binary = workspace.join("_afl_target/debug/harness_001_01_fixed_01");
    assert!(fixed_binary.exists());
    assert!(!workspace.join("_afl_target/debug/harness_001_01").exists());
    let project_main =
        fs::read_to_string(workspace.join("_cargo_projects/harness_001_01_fixed_01/src/main.rs"))
            .expect("read fixed project main");
    assert!(!project_main.contains("BROKEN"));
}
