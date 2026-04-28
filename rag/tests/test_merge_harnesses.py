import json

from seraph_rag.merge_harnesses import write_merged_harnesses


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

    selected_cases = (workspace / "fuzz" / "fixture" / "merged" / "selected_cases.txt").read_text(
        encoding="utf-8"
    )
    main_rs = (workspace / "fuzz" / "fixture" / "merged" / "main.rs").read_text(encoding="utf-8")
    assert "harness_001_01.rs" in selected_cases
    assert "harness_001_02.rs" not in selected_cases
    assert "afl::fuzz!" in main_rs
    assert "dispatch(&data);" in main_rs
    assert report["selected_cases"] == [str(ok_case)]
    assert report["excluded_cases"] == {str(bug_case): "smoke_failed_or_missing"}
