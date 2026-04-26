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

fn write_minimal_workspace(workspace: &Path) -> PathBuf {
    let fuzz_dir = workspace.join("fuzz");
    let harness_path = fuzz_dir.join("harness_001_01.rs");
    let project_dir = workspace.join("_cargo_projects").join("harness_001_01");
    let src_dir = project_dir.join("src");

    fs::create_dir_all(&fuzz_dir).expect("create fuzz dir");
    fs::create_dir_all(&src_dir).expect("create project src");

    fs::write(&harness_path, "fn main() {}\n").expect("write harness");
    fs::write(
        project_dir.join("Cargo.toml"),
        "[package]\nname = \"harness_001_01\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("write main.rs");

    harness_path
}

#[test]
fn bootstrap_fuzz_target_dry_run_prints_real_afl_file_mode_commands() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-file");
    let harness_path = write_minimal_workspace(&workspace);

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--harness",
            "fuzz/harness_001_01.rs",
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
            .join("_cargo_projects/harness_001_01/Cargo.toml")
            .to_str()
            .expect("manifest str")
    ));
    assert!(stdout.contains(
        workspace
            .join("_afl_target/debug/harness_001_01")
            .to_str()
            .expect("binary str")
    ));
    assert!(stdout.contains(
        workspace
            .join("afl/harness_001_01/corpus")
            .to_str()
            .expect("corpus str")
    ));
    assert!(stdout.contains(
        workspace
            .join("afl/harness_001_01/findings")
            .to_str()
            .expect("findings str")
    ));
    assert!(stdout.contains("@@"));
    assert!(stdout.contains(harness_path.to_str().expect("harness str")));
}

#[test]
fn bootstrap_fuzz_target_dry_run_supports_stdin_mode() {
    let repo = repo_root();
    let workspace = temp_dir("bootstrap-afl-stdin");
    write_minimal_workspace(&workspace);

    let output = Command::new("bash")
        .current_dir(&repo)
        .args([
            "scripts/bootstrap-fuzz-target.sh",
            "--workspace-dir",
            workspace.to_str().expect("workspace str"),
            "--harness",
            "fuzz/harness_001_01.rs",
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
    assert!(!stdout.contains("@@"));
}
