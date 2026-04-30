use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

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

enum DefaultCorpusDirField<'a> {
    Missing,
    Null,
    Present(&'a Path),
}

fn write_minimal_workspace(workspace: &Path, default_corpus_dir: DefaultCorpusDirField<'_>) -> PathBuf {
    let reports_dir = workspace.join("reports");
    let project_dir = workspace.join("_cargo_projects").join("merged_fixture");
    let src_dir = project_dir.join("src");
    let merge_report = reports_dir.join("merge_fixture.json");

    fs::create_dir_all(&reports_dir).expect("create reports dir");
    fs::create_dir_all(&src_dir).expect("create project src");

    fs::write(
        project_dir.join("Cargo.toml"),
        "[package]\nname = \"merged_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("write main.rs");
    let mut payload = Map::from_iter([
        ("crate_name".to_owned(), json!("fixture")),
        (
            "manifest_path".to_owned(),
            json!(project_dir.join("Cargo.toml").display().to_string()),
        ),
        ("target_name".to_owned(), json!("merged_fixture")),
    ]);
    match default_corpus_dir {
        DefaultCorpusDirField::Missing => {}
        DefaultCorpusDirField::Null => {
            payload.insert("default_corpus_dir".to_owned(), Value::Null);
        }
        DefaultCorpusDirField::Present(path) => {
            payload.insert(
                "default_corpus_dir".to_owned(),
                json!(path.display().to_string()),
            );
        }
    }
    fs::write(
        &merge_report,
        serde_json::to_vec(&Value::Object(payload)).expect("serialize merge report"),
    )
    .expect("write merge report");

    merge_report
}

#[test]
fn bootstrap_fuzz_target_dry_run_prints_regular_asan_and_cmplog_commands() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-file");
    let merge_report = write_minimal_workspace(&workspace, DefaultCorpusDirField::Missing);

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("cargo afl build"));
    assert!(stdout.contains(
        workspace
            .join("_cargo_projects/merged_fixture/Cargo.toml")
            .to_str()
            .expect("manifest str")
    ));
    assert!(stdout.contains("AFL_USE_ASAN=1"));
    assert!(stdout.contains("AFL_LLVM_CMPLOG=1"));
    assert!(stdout.contains("-M asan_main"));
    assert!(stdout.contains("-S cmplog_aux"));
    assert!(stdout.contains(" -c "));
    assert!(stdout.contains(
        workspace
            .join("afl/merged_fixture/corpus")
            .to_str()
            .expect("corpus str")
    ));
    assert!(stdout.contains(
        workspace
            .join("afl/merged_fixture/findings")
            .to_str()
            .expect("findings str")
    ));
    assert!(stdout.contains("@@"));
}

#[test]
fn bootstrap_fuzz_target_dry_run_supports_stdin_mode() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-stdin");
    let merge_report = write_minimal_workspace(&workspace, DefaultCorpusDirField::Missing);

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--merge-report",
            merge_report.to_str().expect("merge report str"),
            "--input-mode",
            "stdin",
            "--dry-run",
        ])
        .output()
        .expect("run bootstrap script");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("afl-fuzz"));
    assert!(stdout.contains("-M asan_main"));
    assert!(stdout.contains("-S cmplog_aux"));
    assert!(!stdout.contains("@@"));
}

#[test]
fn bootstrap_fuzz_target_dry_run_prefers_merge_default_corpus_dir() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-default-corpus");
    let selector_safe_corpus = workspace.join("afl/merged_fixture/selector_safe_corpus");
    let merge_report = write_minimal_workspace(
        &workspace,
        DefaultCorpusDirField::Present(&selector_safe_corpus),
    );

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(
        selector_safe_corpus
            .to_str()
            .expect("selector-safe corpus str")
    ));
    assert!(!stdout.contains(
        workspace
            .join("afl/merged_fixture/corpus")
            .to_str()
            .expect("legacy corpus str")
    ));
}

#[test]
fn bootstrap_fuzz_target_dry_run_preserves_explicit_corpus_override() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-corpus-override");
    let selector_safe_corpus = workspace.join("afl/merged_fixture/selector_safe_corpus");
    let explicit_corpus = workspace.join("custom/corpus");
    let merge_report = write_minimal_workspace(
        &workspace,
        DefaultCorpusDirField::Present(&selector_safe_corpus),
    );

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--merge-report",
            merge_report.to_str().expect("merge report str"),
            "--corpus-dir",
            explicit_corpus.to_str().expect("explicit corpus str"),
            "--dry-run",
        ])
        .output()
        .expect("run bootstrap script");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(explicit_corpus.to_str().expect("explicit corpus str")));
    assert!(!stdout.contains(
        selector_safe_corpus
            .to_str()
            .expect("selector-safe corpus str")
    ));
}

#[test]
fn bootstrap_fuzz_target_dry_run_falls_back_when_merge_default_corpus_dir_missing() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-legacy-corpus");
    let merge_report = write_minimal_workspace(&workspace, DefaultCorpusDirField::Missing);
    let legacy_corpus = workspace.join("afl/merged_fixture/corpus");

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(legacy_corpus.to_str().expect("legacy corpus str")));
}

#[test]
fn bootstrap_fuzz_target_dry_run_falls_back_when_merge_default_corpus_dir_is_null() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-null-corpus");
    let merge_report = write_minimal_workspace(&workspace, DefaultCorpusDirField::Null);
    let legacy_corpus = workspace.join("afl/merged_fixture/corpus");

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(legacy_corpus.to_str().expect("legacy corpus str")));
    assert!(!stdout.contains(" -i None "));
}
