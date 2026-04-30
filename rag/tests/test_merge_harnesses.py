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
    default_corpus_dir = workspace / "afl" / "merged_fixture" / "corpus"
    default_corpus_meta_dir = workspace / "afl" / "merged_fixture" / "corpus_meta"
    selector_0000 = default_corpus_dir / "selector_0000.bin"

    assert selector_0000.read_bytes() == b"\x00\x00"
    assert report["version"] == "seraph.phase3.merge_harnesses.v2"
    assert report["selected_cases"] == [str(ok_case)]
    assert report["selected_case_count"] == 1
    assert report["safe_seed_count"] == 1
    assert report["default_corpus_dir"] == str(default_corpus_dir)
    assert report["default_corpus_meta_dir"] == str(default_corpus_meta_dir)
    assert report["default_seed_files"] == [str(selector_0000)]
    assert report["excluded_cases"] == {str(bug_case): "smoke_failed_or_missing"}

    report_path = reports_dir / "merge_fixture.json"
    serialized_report = json.loads(report_path.read_text(encoding="utf-8"))
    assert serialized_report["version"] == "seraph.phase3.merge_harnesses.v2"
    assert serialized_report["selected_case_count"] == 1
    assert serialized_report["safe_seed_count"] == 1
    assert serialized_report["default_corpus_dir"] == str(default_corpus_dir)
    assert serialized_report["default_corpus_meta_dir"] == str(default_corpus_meta_dir)
    assert serialized_report["default_seed_files"] == [str(selector_0000)]
    assert serialized_report["selected_cases"] == [str(ok_case)]
    assert serialized_report["excluded_cases"] == {str(bug_case): "smoke_failed_or_missing"}

    summary = json.loads((default_corpus_meta_dir / "summary.json").read_text(encoding="utf-8"))
    assert summary["version"] == "seraph.phase3.selector_safe_corpus.v1"
    assert summary["target_name"] == "merged_fixture"
    assert summary["manifest_path"] == report["manifest_path"]
    assert summary["merge_report"] == str(report_path)
    assert summary["corpus_dir"] == str(default_corpus_dir)
    assert summary["meta_dir"] == str(default_corpus_meta_dir)
    assert summary["selected_case_count"] == 1
    assert summary["safe_seed_count"] == 1
    assert summary["crash_seed_count"] == 0
    assert summary["seed_files"] == [str(selector_0000)]


def test_write_merged_harnesses_writes_selector_safe_seed_for_each_selected_case(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    reports_dir = workspace / "reports"
    fuzz_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    case_0 = fuzz_dir / "harness_001_01.rs"
    case_1 = fuzz_dir / "harness_001_02.rs"
    case_0.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
    case_1.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
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

    reports = [
        {"harness": str(case_0), "status": "ok", "exit_code": 0},
        {"harness": str(case_1), "status": "ok", "exit_code": 0},
    ]
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": reports}),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": reports}),
        encoding="utf-8",
    )

    report = write_merged_harnesses(
        workspace_dir=workspace,
        round_no=1,
        crate_name="fixture",
        crate_import_name="fixture",
    )

    default_corpus_dir = workspace / "afl" / "merged_fixture" / "corpus"
    selector_0000 = default_corpus_dir / "selector_0000.bin"
    selector_0001 = default_corpus_dir / "selector_0001.bin"

    assert selector_0000.read_bytes() == b"\x00\x00"
    assert selector_0001.read_bytes() == b"\x00\x01"
    assert report["selected_case_count"] == 2
    assert report["safe_seed_count"] == 2
    assert report["default_seed_files"] == [str(selector_0000), str(selector_0001)]

    serialized_report = json.loads((reports_dir / "merge_fixture.json").read_text(encoding="utf-8"))
    assert serialized_report["selected_case_count"] == 2
    assert serialized_report["safe_seed_count"] == 2
    assert serialized_report["default_seed_files"] == [str(selector_0000), str(selector_0001)]


def test_write_merged_harnesses_removes_stale_selector_seeds_on_rerun(tmp_path):
    workspace = tmp_path / "workspace"
    fuzz_dir = workspace / "fuzz"
    reports_dir = workspace / "reports"
    fuzz_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    case_0 = fuzz_dir / "harness_001_01.rs"
    case_1 = fuzz_dir / "harness_001_02.rs"
    case_0.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
    case_1.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
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

    both_reports = [
        {"harness": str(case_0), "status": "ok", "exit_code": 0},
        {"harness": str(case_1), "status": "ok", "exit_code": 0},
    ]
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": both_reports}),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": both_reports}),
        encoding="utf-8",
    )

    write_merged_harnesses(
        workspace_dir=workspace,
        round_no=1,
        crate_name="fixture",
        crate_import_name="fixture",
    )

    one_selected_compile = [
        {"harness": str(case_0), "status": "ok", "exit_code": 0},
        {"harness": str(case_1), "status": "ok", "exit_code": 0},
    ]
    one_selected_smoke = [
        {"harness": str(case_0), "status": "ok", "exit_code": 0},
        {"harness": str(case_1), "status": "bug", "exit_code": 1},
    ]
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": one_selected_compile}),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps({"round": 1, "status": "bug", "reports": one_selected_smoke}),
        encoding="utf-8",
    )

    report = write_merged_harnesses(
        workspace_dir=workspace,
        round_no=1,
        crate_name="fixture",
        crate_import_name="fixture",
    )

    default_corpus_dir = workspace / "afl" / "merged_fixture" / "corpus"
    seed_files = sorted(path.name for path in default_corpus_dir.glob("selector_*.bin"))
    assert seed_files == ["selector_0000.bin"]
    assert (default_corpus_dir / "selector_0000.bin").read_bytes() == b"\x00\x00"

    serialized_report = json.loads((reports_dir / "merge_fixture.json").read_text(encoding="utf-8"))
    assert serialized_report["default_seed_files"] == [str(default_corpus_dir / "selector_0000.bin")]
    assert serialized_report["selected_case_count"] == 1
    assert serialized_report["safe_seed_count"] == 1

    summary = json.loads(
        (workspace / "afl" / "merged_fixture" / "corpus_meta" / "summary.json").read_text(
            encoding="utf-8"
        )
    )
    assert report["default_seed_files"] == [str(default_corpus_dir / "selector_0000.bin")]
    assert summary["seed_files"] == [str(default_corpus_dir / "selector_0000.bin")]
    assert summary["selected_case_count"] == 1
    assert summary["safe_seed_count"] == 1


def test_write_merged_harnesses_rejects_more_than_65536_selected_cases(tmp_path):
    workspace = tmp_path / "workspace"
    reports_dir = workspace / "reports"
    reports_dir.mkdir(parents=True)

    selected_cases = []
    for case_no in range(65537):
        case_path = workspace / "fuzz" / f"harness_{case_no:05d}.rs"
        case_path.parent.mkdir(parents=True, exist_ok=True)
        case_path.write_text("pub fn run_case(input: &[u8]) { let _ = input; }\n", encoding="utf-8")
        selected_cases.append({"harness": str(case_path), "status": "ok", "exit_code": 0})
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
        json.dumps({"round": 1, "status": "ok", "reports": selected_cases}),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": selected_cases}),
        encoding="utf-8",
    )

    try:
        write_merged_harnesses(
            workspace_dir=workspace,
            round_no=1,
            crate_name="fixture",
            crate_import_name="fixture",
        )
    except ValueError as exc:
        assert "selector-safe seed limit" in str(exc)
    else:
        raise AssertionError("expected selector-safe seed limit ValueError")
