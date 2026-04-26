import json
from pathlib import Path

from seraph_rag.cli import main
from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import rank_unsafe_targets
from seraph_rag.vector_index import index_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_cli_graph_targets_and_retrieve(tmp_path, capsys):
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    context_path = tmp_path / "context.md"
    index_knowledge(load_knowledge(FIXTURE), vectordb)

    main(["graph", "--knowledge", str(FIXTURE), "--graph", str(graph_path)])
    assert graph_path.exists()

    main(["targets", "--graph", str(graph_path)])
    captured = capsys.readouterr()
    assert "fn::fixture_crate::Buffer::get_unchecked" in captured.out

    main([
        "retrieve",
        "--knowledge",
        str(FIXTURE),
        "--graph",
        str(graph_path),
        "--vectordb",
        str(vectordb),
        "--output",
        str(context_path),
    ])
    context = context_path.read_text(encoding="utf-8")
    assert "SERAPH_STEP_OK" in context
    assert "## Semantically Similar API Docs" in context


def test_cli_harness_prompt_writes_prompt_bundle(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    prompt_path = tmp_path / "harness_prompt_001.json"
    context_path.write_text(
        "## Target API\n- api_id: api::fixture::Buffer::get_unchecked\n",
        encoding="utf-8",
    )

    main([
        "harness-prompt",
        "--context",
        str(context_path),
        "--output",
        str(prompt_path),
        "--variants",
        "2",
        "--style",
        "aflpp",
    ])

    prompt = prompt_path.read_text(encoding="utf-8")
    assert "seraph.phase3.prompt.v1" in prompt
    assert "api::fixture::Buffer::get_unchecked" in prompt
    assert '"variants": 2' in prompt
    assert '"style": "aflpp"' in prompt


def test_cli_retrieve_supports_round_queue_and_target_override(tmp_path):
    knowledge = {
        "crate_meta": {"crate_import_name": "queue_fixture"},
        "modules": [{"module_id": "mod::queue_fixture", "canonical_path": "queue_fixture"}],
        "types": [
            {
                "type_id": "type::queue_fixture::Bag",
                "name": "Bag",
                "canonical_path": "queue_fixture::Bag",
                "public_anchor_module_id": "mod::queue_fixture",
            }
        ],
        "apis": [
            {
                "api_id": "api::queue_fixture::Bag::get_unchecked",
                "name": "get_unchecked",
                "canonical_path": "queue_fixture::Bag::get_unchecked",
                "public_anchor_module_id": "mod::queue_fixture",
                "owner_type_id": "type::queue_fixture::Bag",
                "api_kind": "inherent_method",
                "signature_text": "unsafe fn get_unchecked(&Self, usize) -> u8",
                "receiver": "&Self",
                "return_type": "u8",
                "arg_types": ["usize"],
                "is_unsafe": True,
                "doc_sections": {"safety": "index must be in bounds"},
            },
            {
                "api_id": "api::queue_fixture::Bag::copy_from_raw",
                "name": "copy_from_raw",
                "canonical_path": "queue_fixture::Bag::copy_from_raw",
                "public_anchor_module_id": "mod::queue_fixture",
                "owner_type_id": "type::queue_fixture::Bag",
                "api_kind": "inherent_method",
                "signature_text": "fn copy_from_raw(&mut Self, *const u8, usize)",
                "receiver": "&mut Self",
                "return_type": "()",
                "arg_types": ["*const u8", "usize"],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    knowledge_path = tmp_path / "knowledge.json"
    graph_path = tmp_path / "graph.pkl"
    round_context_path = tmp_path / "round_context.md"
    override_context_path = tmp_path / "override_context.md"
    knowledge_path.write_text(json.dumps(knowledge), encoding="utf-8")

    main(["graph", "--knowledge", str(knowledge_path), "--graph", str(graph_path)])

    ranked = rank_unsafe_targets(build_graph(knowledge))
    assert len(ranked) == 2

    main([
        "retrieve",
        "--knowledge",
        str(knowledge_path),
        "--graph",
        str(graph_path),
        "--output",
        str(round_context_path),
        "--round",
        "2",
    ])
    assert ranked[1].api_id in round_context_path.read_text(encoding="utf-8")

    main([
        "retrieve",
        "--knowledge",
        str(knowledge_path),
        "--graph",
        str(graph_path),
        "--output",
        str(override_context_path),
        "--round",
        "2",
        "--target-api-id",
        ranked[0].api_id,
    ])
    assert ranked[0].api_id in override_context_path.read_text(encoding="utf-8")


def test_cli_harness_write_splits_response_into_harnesses(tmp_path):
    prompt_path = tmp_path / "harness_prompt_003.json"
    response_path = tmp_path / "response.md"
    output_dir = tmp_path / "fuzz"
    prompt_path.write_text(
        '{"target_api_id": "api::fixture::danger"}',
        encoding="utf-8",
    )
    response_path.write_text(
        '```rust\nfn fuzz() { println!("SERAPH_STEP_ENTER:1:api::fixture::danger"); println!("SERAPH_STEP_OK:1:api::fixture::danger"); }\n```',
        encoding="utf-8",
    )

    main([
        "harness-write",
        "--prompt",
        str(prompt_path),
        "--response",
        str(response_path),
        "--output-dir",
        str(output_dir),
        "--round",
        "3",
    ])

    assert (output_dir / "harness_003_01.rs").exists()


def test_cli_compile_check_writes_report(tmp_path):
    harness_path = tmp_path / "harness_001_01.rs"
    report_path = tmp_path / "compile_report.json"
    harness_path.write_text("fn main() {}\n", encoding="utf-8")

    main([
        "compile-check",
        "--harness",
        str(harness_path),
        "--report",
        str(report_path),
        "--command-template",
        "python3 -c 'import sys; sys.exit(0)'",
    ])

    assert '"status": "ok"' in report_path.read_text(encoding="utf-8")


def test_cli_compile_check_accepts_harness_glob(tmp_path):
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    fuzz_dir.mkdir()
    (fuzz_dir / "harness_004_01.rs").write_text("fn main() {}\n", encoding="utf-8")
    (fuzz_dir / "harness_004_02.rs").write_text("fn main() {}\n", encoding="utf-8")

    main([
        "compile-check",
        "--harness-glob",
        str(fuzz_dir / "harness_004_*.rs"),
        "--report-dir",
        str(report_dir),
        "--round",
        "4",
        "--command-template",
        "python3 -c 'import sys; sys.exit(0)'",
    ])

    assert (report_dir / "compile_004_index.json").exists()
    assert (report_dir / "compile_004_01.json").exists()
    assert (report_dir / "compile_004_02.json").exists()


def test_cli_smoke_run_accepts_compile_and_fix_indexes(tmp_path):
    reports_dir = tmp_path / "reports"
    compile_index = reports_dir / "compile_004_index.json"
    fix_loop_index = reports_dir / "fix_loop_004_index.json"
    compile_ok = tmp_path / "fuzz" / "harness_004_01.rs"
    fixed_ok = tmp_path / "fuzz" / "harness_004_02_fixed_02.rs"
    compile_ok.parent.mkdir(parents=True)
    compile_ok.write_text("fn main() {}\n", encoding="utf-8")
    fixed_ok.write_text("fn main() {}\n", encoding="utf-8")
    reports_dir.mkdir()
    compile_index.write_text(
        json.dumps(
            {
                "round": 4,
                "status": "failed",
                "reports": [
                    {"harness": str(compile_ok), "status": "ok", "exit_code": 0},
                    {"harness": str(tmp_path / "fuzz" / "harness_004_02.rs"), "status": "failed", "exit_code": 1},
                ],
            }
        ),
        encoding="utf-8",
    )
    fix_loop_index.write_text(
        json.dumps(
            {
                "status": "failed",
                "loops": [
                    {
                        "status": "ok",
                        "successful_attempt": 2,
                        "attempts": [
                            {"attempt": 2, "harness": str(fixed_ok), "status": "ok", "exit_code": 0}
                        ],
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    main([
        "smoke-run",
        "--compile-index",
        str(compile_index),
        "--fix-loop-index",
        str(fix_loop_index),
        "--report-dir",
        str(reports_dir),
        "--round",
        "4",
        "--command-template",
        "python3 -c 'import sys; sys.exit(0)'",
    ])

    assert (reports_dir / "smoke_004_index.json").exists()
    assert (reports_dir / "smoke_004_01.json").exists()
    assert (reports_dir / "smoke_004_02_fixed_02.json").exists()


def test_cli_fixer_bundle_writes_failed_request(tmp_path):
    context_path = tmp_path / "rag_target_002.md"
    reports_dir = tmp_path / "reports"
    fix_dir = tmp_path / "fixes"
    harness_path = tmp_path / "fuzz" / "harness_002_01.rs"
    report_path = reports_dir / "compile_002_01.json"
    index_path = reports_dir / "compile_002_index.json"
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    harness_path.parent.mkdir()
    harness_path.write_text("fn fuzz() {}\n", encoding="utf-8")
    reports_dir.mkdir()
    report_path.write_text(
        '{"harness":"%s","status":"failed","exit_code":1,"stderr":"bad","stdout":"","command":"rustc"}' % harness_path,
        encoding="utf-8",
    )
    index_path.write_text(
        '{"round":2,"status":"failed","reports":[{"harness":"%s","report":"%s","status":"failed","exit_code":1}]}' % (harness_path, report_path),
        encoding="utf-8",
    )

    main([
        "fixer-bundle",
        "--compile-index",
        str(index_path),
        "--context",
        str(context_path),
        "--output-dir",
        str(fix_dir),
    ])

    assert (fix_dir / "fix_request_002_01.json").exists()


def test_cli_fixer_write_outputs_fixed_harness(tmp_path):
    request_path = tmp_path / "fix_request_005_01.json"
    response_path = tmp_path / "fix_response.md"
    output_dir = tmp_path / "fuzz"
    request_path.write_text(
        '{"round":5,"variant":1,"target_api_id":"api::fixture::danger"}',
        encoding="utf-8",
    )
    response_path.write_text(
        '```rust\nfn fuzz() { println!("SERAPH_STEP_ENTER:1:api::fixture::danger"); println!("SERAPH_STEP_OK:1:api::fixture::danger"); }\n```',
        encoding="utf-8",
    )

    main([
        "fixer-write",
        "--request",
        str(request_path),
        "--response",
        str(response_path),
        "--output-dir",
        str(output_dir),
        "--attempt",
        "2",
    ])

    assert (output_dir / "harness_005_01_fixed_02.rs").exists()


def test_cli_fix_once_writes_fixed_compile_report(tmp_path):
    request_path = tmp_path / "fix_request_006_01.json"
    response_path = tmp_path / "fix_response_006_01.md"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    request_path.write_text(
        '{"round":6,"variant":1,"target_api_id":"api::fixture::danger"}',
        encoding="utf-8",
    )
    response_path.write_text(
        '```rust\nfn main() { println!("SERAPH_STEP_ENTER:1:api::fixture::danger"); println!("SERAPH_STEP_OK:1:api::fixture::danger"); }\n```',
        encoding="utf-8",
    )

    main([
        "fix-once",
        "--request",
        str(request_path),
        "--response",
        str(response_path),
        "--output-dir",
        str(fuzz_dir),
        "--report-dir",
        str(report_dir),
        "--attempt",
        "1",
        "--command-template",
        "python3 -c 'import sys; sys.exit(0)'",
    ])

    assert (fuzz_dir / "harness_006_01_fixed_01.rs").exists()
    assert (report_dir / "compile_006_01_fixed_01.json").exists()


def test_cli_fix_loop_stops_after_first_success(tmp_path):
    request_path = tmp_path / "fix_request_007_01.json"
    responses_dir = tmp_path / "fixes"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    request_path.write_text(
        '{"round":7,"variant":1,"target_api_id":"api::fixture::danger"}',
        encoding="utf-8",
    )
    responses_dir.mkdir()
    (responses_dir / "fix_response_007_01_01.md").write_text(
        '```rust\nfn main() { println!("SERAPH_STEP_ENTER:1:api::fixture::danger"); println!("SERAPH_STEP_OK:1:api::fixture::danger"); }\n```',
        encoding="utf-8",
    )
    (responses_dir / "fix_response_007_01_02.md").write_text(
        '```rust\nfn main() { println!("SERAPH_STEP_ENTER:1:api::fixture::danger"); println!("SERAPH_STEP_OK:1:api::fixture::danger"); }\n```',
        encoding="utf-8",
    )

    main([
        "fix-loop",
        "--request",
        str(request_path),
        "--responses-dir",
        str(responses_dir),
        "--output-dir",
        str(fuzz_dir),
        "--report-dir",
        str(report_dir),
        "--max-attempts",
        "3",
        "--command-template",
        "python3 -c \"import pathlib, sys; sys.exit(0 if 'fixed_02' in pathlib.Path(sys.argv[1]).name else 1)\" {harness}",
    ])

    loop_report = json.loads((report_dir / "fix_loop_007_01.json").read_text(encoding="utf-8"))
    assert loop_report["status"] == "ok"
    assert loop_report["successful_attempt"] == 2
    assert len(loop_report["attempts"]) == 2


def test_cli_fix_loop_batch_writes_index(tmp_path):
    requests_dir = tmp_path / "fixes"
    responses_dir = tmp_path / "fixes"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    index_path = report_dir / "fix_loop_010_index.json"
    requests_dir.mkdir()
    (requests_dir / "fix_request_010_01.json").write_text(
        '{"round":10,"variant":1,"target_api_id":"api::fixture::danger"}',
        encoding="utf-8",
    )
    (responses_dir / "fix_response_010_01_01.md").write_text(
        '```rust\nfn main() { println!("SERAPH_STEP_ENTER:1:api::fixture::danger"); println!("SERAPH_STEP_OK:1:api::fixture::danger"); }\n```',
        encoding="utf-8",
    )

    main([
        "fix-loop-batch",
        "--request-glob",
        str(requests_dir / "fix_request_010_*.json"),
        "--responses-dir",
        str(responses_dir),
        "--output-dir",
        str(fuzz_dir),
        "--report-dir",
        str(report_dir),
        "--index-output",
        str(index_path),
        "--max-attempts",
        "2",
        "--command-template",
        "python3 -c 'import sys; sys.exit(0)'",
    ])

    batch_report = json.loads(index_path.read_text(encoding="utf-8"))
    assert batch_report["status"] == "ok"
    assert batch_report["request_count"] == 1
    assert batch_report["successful_requests"] == 1


def test_cli_fix_acceptance_writes_index(tmp_path):
    reports_dir = tmp_path / "reports"
    reports_dir.mkdir()
    fix_loop_report = reports_dir / "fix_loop_010_01.json"
    fix_loop_index = reports_dir / "fix_loop_010_index.json"
    smoke_index = reports_dir / "smoke_010_index.json"
    output_path = reports_dir / "fix_acceptance_010_index.json"
    fixed_ok = tmp_path / "fuzz" / "harness_010_01_fixed_01.rs"
    fixed_ok.parent.mkdir(parents=True)
    fixed_ok.write_text("fn main() {}\n", encoding="utf-8")

    fix_loop_report.write_text(
        json.dumps(
            {
                "round": 10,
                "variant": 1,
                "status": "ok",
                "successful_attempt": 1,
                "attempts": [
                    {
                        "attempt": 1,
                        "harness": str(fixed_ok),
                        "report": str(reports_dir / "compile_010_01_fixed_01.json"),
                        "status": "ok",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    fix_loop_index.write_text(
        json.dumps(
            {
                "status": "ok",
                "loops": [
                    {
                        "status": "ok",
                        "successful_attempt": 1,
                        "report": str(fix_loop_report),
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    smoke_index.write_text(
        json.dumps(
            {
                "round": 10,
                "status": "ok",
                "reports": [
                    {
                        "harness": str(fixed_ok),
                        "report": str(reports_dir / "smoke_010_01_fixed_01.json"),
                        "status": "ok",
                        "classification": "completed",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    main([
        "fix-acceptance",
        "--fix-loop-index",
        str(fix_loop_index),
        "--smoke-index",
        str(smoke_index),
        "--output",
        str(output_path),
    ])

    report = json.loads(output_path.read_text(encoding="utf-8"))
    assert report["status"] == "accepted"
    assert report["entry_count"] == 1


def test_cli_fix_acceptance_write_updates_index(tmp_path):
    reports_dir = tmp_path / "reports"
    reports_dir.mkdir()
    index_path = reports_dir / "fix_acceptance_011_index.json"
    harness = tmp_path / "fuzz" / "harness_011_01_fixed_01.rs"
    harness.parent.mkdir(parents=True)
    harness.write_text("fn main() {}\n", encoding="utf-8")
    index_path.write_text(
        json.dumps(
            {
                "version": "seraph.phase3.fix_acceptance_index.v1",
                "round": 11,
                "status": "needs_review",
                "entry_count": 1,
                "entries": [
                    {
                        "harness": str(harness),
                        "compile_report": str(reports_dir / "compile_011_01_fixed_01.json"),
                        "smoke_report": str(reports_dir / "smoke_011_01_fixed_01.json"),
                        "status": "needs_review",
                        "reason": "needs_review",
                        "source": "auto",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    main([
        "fix-acceptance-write",
        "--index",
        str(index_path),
        "--harness",
        str(harness),
        "--status",
        "accepted",
        "--reason",
        "manual_triage_ok",
    ])

    report = json.loads(index_path.read_text(encoding="utf-8"))
    assert report["status"] == "accepted"
    assert report["entries"][0]["status"] == "accepted"
    assert report["entries"][0]["reason"] == "manual_triage_ok"


def test_cli_fix_acceptance_write_batch_updates_index(tmp_path):
    reports_dir = tmp_path / "reports"
    reports_dir.mkdir()
    index_path = reports_dir / "fix_acceptance_012_index.json"
    decisions_path = reports_dir / "fix_acceptance_012_decisions.json"
    harness_a = tmp_path / "fuzz" / "harness_012_01_fixed_01.rs"
    harness_b = tmp_path / "fuzz" / "harness_012_02_fixed_01.rs"
    harness_a.parent.mkdir(parents=True)
    harness_a.write_text("fn main() {}\n", encoding="utf-8")
    harness_b.write_text("fn main() {}\n", encoding="utf-8")
    index_path.write_text(
        json.dumps(
            {
                "version": "seraph.phase3.fix_acceptance_index.v1",
                "round": 12,
                "status": "needs_review",
                "entry_count": 2,
                "entries": [
                    {
                        "harness": str(harness_a),
                        "compile_report": str(reports_dir / "compile_012_01_fixed_01.json"),
                        "smoke_report": str(reports_dir / "smoke_012_01_fixed_01.json"),
                        "status": "needs_review",
                        "reason": "smoke_missing",
                        "source": "auto",
                    },
                    {
                        "harness": str(harness_b),
                        "compile_report": str(reports_dir / "compile_012_02_fixed_01.json"),
                        "smoke_report": str(reports_dir / "smoke_012_02_fixed_01.json"),
                        "status": "needs_review",
                        "reason": "needs_review",
                        "source": "auto",
                    },
                ],
            }
        ),
        encoding="utf-8",
    )
    decisions_path.write_text(
        json.dumps(
            {
                "decisions": [
                    {
                        "harness": str(harness_a),
                        "status": "accepted",
                        "reason": "manual_triage_ok",
                    },
                    {
                        "harness": str(harness_b),
                        "status": "bug",
                        "reason": "manual_bug_confirmed",
                    },
                ]
            }
        ),
        encoding="utf-8",
    )

    main([
        "fix-acceptance-write-batch",
        "--index",
        str(index_path),
        "--decisions",
        str(decisions_path),
    ])

    report = json.loads(index_path.read_text(encoding="utf-8"))
    assert report["status"] == "bug"
    assert report["entries"][0]["status"] == "accepted"
    assert report["entries"][1]["status"] == "bug"


def test_cli_model_response_writes_output(tmp_path):
    input_path = tmp_path / "harness_prompt_012.json"
    output_path = tmp_path / "llm_response_012.md"
    input_path.write_text(
        '{"target_api_id":"api::fixture::danger"}',
        encoding="utf-8",
    )

    main([
        "model-response",
        "--input",
        str(input_path),
        "--output",
        str(output_path),
        "--command-template",
        "python3 -c \"print('fn main() { println!(\\\"SERAPH_STEP_ENTER:1:api::fixture::danger\\\"); println!(\\\"SERAPH_STEP_OK:1:api::fixture::danger\\\"); }')\"",
    ])

    assert output_path.exists()
    assert "SERAPH_STEP_OK:1:api::fixture::danger" in output_path.read_text(encoding="utf-8")
