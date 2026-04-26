import json

from seraph_rag.compile_fixer_bundle import write_compile_fixer_bundles


def test_write_compile_fixer_bundles_for_failed_reports(tmp_path):
    context_path = tmp_path / "rag_target_009.md"
    reports_dir = tmp_path / "reports"
    fix_dir = tmp_path / "fixes"
    harness_path = tmp_path / "fuzz" / "harness_009_01.rs"
    report_path = reports_dir / "compile_009_01.json"
    index_path = reports_dir / "compile_009_index.json"
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    harness_path.parent.mkdir()
    harness_path.write_text("fn fuzz() {}\n", encoding="utf-8")
    reports_dir.mkdir()
    report_path.write_text(
        json.dumps({
            "harness": str(harness_path),
            "status": "failed",
            "exit_code": 1,
            "stderr": "error[E0308]: mismatched types",
            "stdout": "",
            "command": "rustc harness.rs",
        }),
        encoding="utf-8",
    )
    index_path.write_text(
        json.dumps({
            "round": 9,
            "status": "failed",
            "reports": [{
                "harness": str(harness_path),
                "report": str(report_path),
                "status": "failed",
                "exit_code": 1,
            }],
        }),
        encoding="utf-8",
    )

    bundles = write_compile_fixer_bundles(index_path, context_path, fix_dir)

    assert [path.name for path in bundles] == ["fix_request_009_01.json"]
    data = json.loads(bundles[0].read_text(encoding="utf-8"))
    assert data["version"] == "seraph.phase3.compile_fixer_request.v1"
    assert data["round"] == 9
    assert data["variant"] == 1
    assert data["target_api_id"] == "api::fixture::danger"
    assert data["harness_source"] == "fn fuzz() {}\n"
    assert "mismatched types" in data["diagnostics"]["stderr"]
    assert "Only use crate APIs explicitly present in the rag_context or harness_source." in data["rules"]
    assert "Do not invent new crate APIs to satisfy compiler errors." in data["rules"]
    assert "Prefer Known Reachable Paths for opaque wrappers and borrowed handles." in data["rules"]
    assert "Prefer Required Setup APIs for opaque wrappers and borrowed handles." not in data["rules"]


def test_write_compile_fixer_bundles_skips_successful_reports(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    reports_dir = tmp_path / "reports"
    fix_dir = tmp_path / "fixes"
    report_path = reports_dir / "compile_001_01.json"
    index_path = reports_dir / "compile_001_index.json"
    context_path.write_text("## Target API\n- api_id: api::fixture::ok\n", encoding="utf-8")
    reports_dir.mkdir()
    report_path.write_text(json.dumps({"status": "ok"}), encoding="utf-8")
    index_path.write_text(
        json.dumps({"round": 1, "status": "ok", "reports": [{"report": str(report_path), "status": "ok"}]}),
        encoding="utf-8",
    )

    bundles = write_compile_fixer_bundles(index_path, context_path, fix_dir)

    assert bundles == []
    assert not fix_dir.exists()
