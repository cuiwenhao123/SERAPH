import json

from seraph_rag.real_crate_runner import ensure_cargo_project, compile_harness


def test_ensure_cargo_project_writes_manifest_and_main(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    harness = fuzz_dir / "harness_001_01.rs"
    fuzz_dir.mkdir(parents=True)
    harness.write_text("fn main() {}\n", encoding="utf-8")
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

    manifest = (project_dir / "Cargo.toml").read_text(encoding="utf-8")
    main_rs = (project_dir / "src/main.rs").read_text(encoding="utf-8")
    assert 'name = "harness_001_01"' in manifest
    assert 'moonfire_ffmpeg = { package = "moonfire-ffmpeg", path = "/tmp/target-crate" }' in manifest
    assert main_rs == "fn main() {}\n"


def test_compile_harness_runs_cargo_build(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    harness = fuzz_dir / "harness_001_02.rs"
    fuzz_dir.mkdir(parents=True)
    harness.write_text("fn main() {}\n", encoding="utf-8")
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
    assert captured["kwargs"]["capture_output"] is True
    assert captured["kwargs"]["text"] is True
