import json

from seraph_rag.cli import main


def test_cli_runtime_diagnose_writes_runtime_error_index(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    reports_dir = tmp_path / "reports"
    smoke_index_path = reports_dir / "smoke_001_index.json"
    smoke_report_path = reports_dir / "smoke_001_01.json"
    harness_path = tmp_path / "fuzz" / "harness_001_01.rs"

    harness_path.parent.mkdir(parents=True)
    reports_dir.mkdir(parents=True)
    harness_path.write_text("fn main() {}\n", encoding="utf-8")
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    smoke_report_path.write_text(
        json.dumps(
            {
                "harness": str(harness_path),
                "status": "bug",
                "classification": "panic_detected",
                "exit_code": 101,
                "stdout": "",
                "stderr": "panicked at bad input",
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
                        "report": str(smoke_report_path),
                        "status": "bug",
                        "classification": "panic_detected",
                        "exit_code": 101,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    main(
        [
            "runtime-diagnose",
            "--context",
            str(context_path),
            "--smoke-index",
            str(smoke_index_path),
            "--output-dir",
            str(reports_dir),
            "--round",
            "1",
            "--command-template",
            (
                "python3 -c \"import json; print(json.dumps({"
                "'classification':'panic_or_crash',"
                "'summary':'panic after target',"
                "'is_bug':False}))\""
            ),
        ]
    )

    report = json.loads((reports_dir / "runtime_error_001_index.json").read_text(encoding="utf-8"))
    assert report["status"] == "runtime_error"
    assert report["report_count"] == 1
