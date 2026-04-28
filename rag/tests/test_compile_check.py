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


def test_run_compile_check_allows_generic_target_call_with_turbofish(tmp_path):
    harness = tmp_path / "harness_008_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """struct Client;

impl Client {
    fn set_as_callback<F>(&self, _callback: Option<F>) {}
}

fn main() {
    let client = Client;
    println!("SERAPH_STEP_ENTER:1:api::fixture::Client::set_as_callback");
    client.set_as_callback::<fn(i32)>(None);
    println!("SERAPH_STEP_OK:1:api::fixture::Client::set_as_callback");
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


def test_run_compile_check_allows_real_target_call_via_public_path_from_context(tmp_path):
    contexts = tmp_path / "contexts"
    fuzz = tmp_path / "fuzz"
    contexts.mkdir()
    fuzz.mkdir()
    (contexts / "rag_target_009.md").write_text(
        """## Target API
- api_id: api::fixture::utf8::decode
- path: fixture::decode_utf8
""",
        encoding="utf-8",
    )
    harness = fuzz / "harness_009_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """fn main() {
    let data = b\"abc\";
    println!(\"SERAPH_STEP_ENTER:1:api::fixture::utf8::decode\");
    let _value = fixture::decode_utf8(data);
    println!(\"SERAPH_STEP_OK:1:api::fixture::utf8::decode\");
}

mod fixture {
    pub fn decode_utf8(_bytes: &[u8]) -> (Option<char>, usize) {
        (Some('a'), 1)
    }
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


def test_run_compile_check_rejects_positive_offset_slice_without_empty_input_guard(tmp_path):
    harness = tmp_path / "harness_009_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """fn target(_value: &str) {}

fn main() {
    let data: Vec<u8> = Vec::new();
    let take = core::cmp::min(4, data.len().saturating_sub(1));
    let value = std::str::from_utf8(&data[1..1 + take]).unwrap_or("");
    println!("SERAPH_STEP_ENTER:1:api::fixture::target");
    target(value);
    println!("SERAPH_STEP_OK:1:api::fixture::target");
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
    assert "input-derived slice" in data["stderr"]


def test_run_compile_check_rejects_placeholder_pointee_in_named_fn_signature(tmp_path):
    harness = tmp_path / "harness_013_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """use std::os::raw::{c_int, c_void};

fn rw_cb(
    _usr_ptr: *mut c_void,
    _op: c_int,
    _transport_size: c_int,
    _tag: *mut _,
    _pdata: *mut c_void,
) {}

fn main() {
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
    assert "placeholder `_` in named function item signature" in data["stderr"]


def test_run_compile_check_rejects_saturating_plus_one_slice_without_guard(tmp_path):
    harness = tmp_path / "harness_010_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """fn target(_value: &[u8]) {}

fn main() {
    let data: Vec<u8> = Vec::new();
    let first_nul = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let value = &data[first_nul.saturating_add(1)..];
    println!("SERAPH_STEP_ENTER:1:api::fixture::target");
    target(value);
    println!("SERAPH_STEP_OK:1:api::fixture::target");
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
    assert "input-derived slice" in data["stderr"]


def test_run_compile_check_rejects_idx_slice_after_saturating_plus_one_without_guard(tmp_path):
    harness = tmp_path / "harness_011_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """fn target(_value: &str) {}

fn main() {
    let data: Vec<u8> = Vec::new();
    let mut idx = 0usize;
    idx = idx.saturating_add(1);
    let take = data.len().saturating_sub(idx);
    let ip_bytes = &data[idx..idx + take];
    let value = std::str::from_utf8(ip_bytes).unwrap_or("");
    println!("SERAPH_STEP_ENTER:1:api::fixture::target");
    target(value);
    println!("SERAPH_STEP_OK:1:api::fixture::target");
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
    assert "input-derived slice" in data["stderr"]


def test_run_compile_check_allows_get_based_slice_for_short_inputs(tmp_path):
    harness = tmp_path / "harness_012_01.rs"
    report = tmp_path / "compile_report.json"
    harness.write_text(
        """fn target(_value: &[u8]) {}

fn main() {
    let data: Vec<u8> = Vec::new();
    let take = core::cmp::min(4, data.len().saturating_sub(1));
    let value = data.get(1..1 + take).unwrap_or(&[]);
    println!("SERAPH_STEP_ENTER:1:api::fixture::target");
    target(value);
    println!("SERAPH_STEP_OK:1:api::fixture::target");
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
