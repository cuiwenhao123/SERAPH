import json

from seraph_rag.compile_check import run_compile_check


def test_run_compile_check_records_success(tmp_path):
    harness = tmp_path / "harness_001_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text("fn main() {}\n", encoding="utf-8")

    result = run_compile_check(
        harness,
        report,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "ok"
    assert data["harness"] == str(harness)
    assert data["exit_code"] == 0
    assert data["stdout"] == ""
    assert data["stderr"] == ""


def test_run_compile_check_records_failure_diagnostics(tmp_path):
    harness = tmp_path / "harness_002_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text("fn main() {}\n", encoding="utf-8")

    result = run_compile_check(
        harness,
        report,
        command_template="python3 -c 'import sys; print(\"bad type\", file=sys.stderr); sys.exit(2)'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "failed"
    assert data["exit_code"] == 2
    assert "bad type" in data["stderr"]


def test_run_compile_checks_writes_index_for_all_harnesses(tmp_path):
    from seraph_rag.compile_check import run_compile_checks

    output_dir = tmp_path / "reports"
    harness_a = tmp_path / "harness_003_01.rs"
    harness_b = tmp_path / "harness_003_02.rs"
    harness_a.write_text("fn main() {}\n", encoding="utf-8")
    harness_b.write_text("fn main() {}\n", encoding="utf-8")

    result = run_compile_checks(
        [harness_a, harness_b],
        output_dir,
        round_no=3,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    index_path = output_dir / "compile_003_index.json"
    index = json.loads(index_path.read_text(encoding="utf-8"))
    assert result["status"] == "ok"
    assert index["round"] == 3
    assert len(index["reports"]) == 2
    assert (output_dir / "compile_003_01.json").exists()
    assert (output_dir / "compile_003_02.json").exists()


def test_run_compile_check_rejects_diverging_placeholder_helper(tmp_path):
    harness = tmp_path / "harness_004_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """fn fake<T>() -> T {
    std::process::exit(0)
}

fn main() {
    let _value: &str = fake();
    println!("SERAPH_STEP_ENTER:1:api::fixture::danger");
    println!("SERAPH_STEP_OK:1:api::fixture::danger");
}
""",
        encoding="utf-8",
    )

    result = run_compile_check(
        harness,
        report,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "failed"
    assert data["exit_code"] == 2
    assert "diverging placeholder helper" in data["stderr"]


def test_run_compile_check_rejects_marker_without_target_call_between_markers(tmp_path):
    harness = tmp_path / "harness_005_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """fn main() {
    println!("SERAPH_STEP_ENTER:1:api::fixture::danger");
    println!("SERAPH_STEP_OK:1:api::fixture::danger");
}
""",
        encoding="utf-8",
    )

    result = run_compile_check(
        harness,
        report,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "failed"
    assert data["exit_code"] == 2
    assert "target call between SERAPH_STEP_ENTER and SERAPH_STEP_OK" in data["stderr"]


def test_run_compile_check_rejects_unsafe_initialization_fabrication(tmp_path):
    harness = tmp_path / "harness_006_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """use std::mem::MaybeUninit;

fn main() {
    let _value = unsafe { MaybeUninit::<u64>::zeroed().assume_init() };
    println!("SERAPH_STEP_ENTER:1:api::fixture::danger");
    danger();
    println!("SERAPH_STEP_OK:1:api::fixture::danger");
}

fn danger() {}
""",
        encoding="utf-8",
    )

    result = run_compile_check(
        harness,
        report,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "failed"
    assert data["exit_code"] == 2
    assert "unsafe initialization trick" in data["stderr"]


def test_run_compile_check_allows_signature_required_maybe_uninit_usage(tmp_path):
    harness = tmp_path / "harness_007_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """use std::mem::MaybeUninit;

fn danger(_buf: &mut [MaybeUninit<u8>]) {}

fn main() {
    let mut backing = vec![MaybeUninit::<u8>::uninit(); 4];
    println!("SERAPH_STEP_ENTER:1:api::fixture::danger");
    danger(backing.as_mut_slice());
    println!("SERAPH_STEP_OK:1:api::fixture::danger");
}
""",
        encoding="utf-8",
    )

    result = run_compile_check(
        harness,
        report,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "ok"
    assert data["exit_code"] == 0
