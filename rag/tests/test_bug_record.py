import json
import subprocess
import sys
from pathlib import Path

from seraph_rag.bug_record import append_bug_record_entry, render_bug_record_entry

REPO_ROOT = Path(__file__).resolve().parents[2]


def test_render_bug_record_entry_from_success_workspace(tmp_path):
    workspace = tmp_path / "workspace"
    contexts_dir = workspace / "contexts"
    reports_dir = workspace / "reports"
    contexts_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    (contexts_dir / "rag_target_001.md").write_text(
        "# SERAPH RAG Harness Context\n\n## Target API\n- api_id: api::bytes::buf::buf_mut::BufMut::advance_mut\n",
        encoding="utf-8",
    )
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "ok",
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                        "report": str(reports_dir / "compile_001_01.json"),
                        "status": "ok",
                        "exit_code": 0,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "ok",
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                        "report": str(reports_dir / "smoke_001_01.json"),
                        "status": "ok",
                        "classification": "completed",
                        "exit_code": 0,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "runtime_error_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": []}),
        encoding="utf-8",
    )
    (workspace / "coverage.json").write_text(
        json.dumps(
            {
                "total_api_ids": ["api::bytes::buf::buf_mut::BufMut::advance_mut"],
                "covered_api_ids": ["api::bytes::buf::buf_mut::BufMut::advance_mut"],
                "related_total_api_ids": ["api::a", "api::b", "api::c"],
                "related_covered_api_ids": ["api::a", "api::b"],
            }
        ),
        encoding="utf-8",
    )

    markdown = render_bug_record_entry(
        workspace,
        crate_name="bytes",
        phase="Phase 3 real rerun",
        title="2026-04-26 `bytes`：advance_mut real verification",
    )

    assert "### 2026-04-26 `bytes`：advance_mut real verification" in markdown
    assert "- **crate / 阶段**：`bytes`，Phase 3 real rerun" in markdown
    assert "- **target API**：`api::bytes::buf::buf_mut::BufMut::advance_mut`" in markdown
    assert "- compile：`ok`" in markdown
    assert "- smoke：`ok`" in markdown
    assert "- runtime diagnose：`ok`" in markdown
    assert "- related API 覆盖：`2 / 3`" in markdown
    assert f"- context：`{contexts_dir / 'rag_target_001.md'}`" in markdown
    assert f"- compile index：`{reports_dir / 'compile_001_index.json'}`" in markdown
    assert "- **根因**：" in markdown
    assert "- 待补充" in markdown


def test_render_bug_record_entry_collects_compile_error_codes_and_appends(tmp_path):
    workspace = tmp_path / "workspace"
    contexts_dir = workspace / "contexts"
    reports_dir = workspace / "reports"
    contexts_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    (contexts_dir / "rag_target_003.md").write_text(
        "# SERAPH RAG Harness Context\n\n## Target API\n- api_id: api::bytes::buf::uninit_slice::UninitSlice::new\n",
        encoding="utf-8",
    )
    (reports_dir / "compile_003_01.json").write_text(
        json.dumps(
            {
                "harness": str(workspace / "fuzz" / "harness_003_01.rs"),
                "status": "failed",
                "exit_code": 1,
                "stdout": "",
                "stderr": "error[E0502]: cannot borrow\\nerror[E0133]: call to unsafe function",
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "compile_003_index.json").write_text(
        json.dumps(
            {
                "round": 3,
                "status": "failed",
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_003_01.rs"),
                        "report": str(reports_dir / "compile_003_01.json"),
                        "status": "failed",
                        "exit_code": 1,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    markdown = render_bug_record_entry(
        workspace,
        crate_name="bytes",
        phase="Phase 3 compile failure",
        symptom=["首轮 harness 编译失败"],
        root_cause=["Phase 3 prompt 对 borrow 约束不够具体"],
    )

    assert "- **target API**：`api::bytes::buf::uninit_slice::UninitSlice::new`" in markdown
    assert "- compile：`failed`" in markdown
    assert "- **编译错误码**：" in markdown
    assert "- `E0133`" in markdown
    assert "- `E0502`" in markdown
    assert "- 首轮 harness 编译失败" in markdown
    assert "- Phase 3 prompt 对 borrow 约束不够具体" in markdown

    doc_path = tmp_path / "bug-record.md"
    doc_path.write_text("# Existing\n", encoding="utf-8")
    append_bug_record_entry(doc_path, markdown)
    appended = doc_path.read_text(encoding="utf-8")
    assert appended.startswith("# Existing\n")
    assert "api::bytes::buf::uninit_slice::UninitSlice::new" in appended


def test_real_crate_bug_record_script_prints_markdown(tmp_path):
    workspace = tmp_path / "workspace"
    contexts_dir = workspace / "contexts"
    reports_dir = workspace / "reports"
    contexts_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    (contexts_dir / "rag_target_001.md").write_text(
        "# SERAPH RAG Harness Context\n\n## Target API\n- api_id: api::bytes::buf::uninit_slice::UninitSlice::uninit\n",
        encoding="utf-8",
    )
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": []}),
        encoding="utf-8",
    )
    (workspace / "coverage.json").write_text(
        json.dumps(
            {
                "total_api_ids": ["api::bytes::buf::uninit_slice::UninitSlice::uninit"],
                "covered_api_ids": ["api::bytes::buf::uninit_slice::UninitSlice::uninit"],
            }
        ),
        encoding="utf-8",
    )

    result = subprocess.run(
        [
            sys.executable,
            str(REPO_ROOT / "scripts" / "real_crate_bug_record.py"),
            "--workspace-dir",
            str(workspace),
            "--crate",
            "bytes",
            "--phase",
            "Phase 3 real rerun",
        ],
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert "- **crate / 阶段**：`bytes`，Phase 3 real rerun" in result.stdout
    assert "api::bytes::buf::uninit_slice::UninitSlice::uninit" in result.stdout


def test_render_bug_record_entry_uses_fix_loop_as_effective_compile_status(tmp_path):
    workspace = tmp_path / "workspace"
    contexts_dir = workspace / "contexts"
    reports_dir = workspace / "reports"
    contexts_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    (contexts_dir / "rag_target_001.md").write_text(
        "# SERAPH RAG Harness Context\n\n## Target API\n- api_id: api::moonfire_ffmpeg::avcodec::DecodeContext::decode_video\n",
        encoding="utf-8",
    )
    (reports_dir / "compile_001_01.json").write_text(
        json.dumps(
            {
                "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                "status": "failed",
                "exit_code": 101,
                "stderr": "error[E0046]: not all trait items implemented",
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "failed",
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                        "report": str(reports_dir / "compile_001_01.json"),
                        "status": "failed",
                        "exit_code": 101,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "fix_loop_001_index.json").write_text(
        json.dumps(
            {
                "status": "ok",
                "successful_requests": 1,
                "failed_requests": 0,
                "loops": [
                    {
                        "status": "ok",
                        "successful_attempt": 1,
                        "stop_reason": "compiled",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": []}),
        encoding="utf-8",
    )

    markdown = render_bug_record_entry(
        workspace,
        crate_name="moonfire-ffmpeg",
        phase="Phase 3 auto round 1",
    )

    assert "- compile：`ok`" in markdown
    assert "- compile 初始状态：`failed`" in markdown
    assert "- fix-loop：`ok`" in markdown
    assert "- `E0046`" in markdown


def test_render_bug_record_entry_includes_runtime_diagnosis_details(tmp_path):
    workspace = tmp_path / "workspace"
    contexts_dir = workspace / "contexts"
    reports_dir = workspace / "reports"
    contexts_dir.mkdir(parents=True)
    reports_dir.mkdir(parents=True)

    (contexts_dir / "rag_target_001.md").write_text(
        "# SERAPH RAG Harness Context\n\n## Target API\n- api_id: api::bytes::buf::uninit_slice::UninitSlice::new\n",
        encoding="utf-8",
    )
    (reports_dir / "compile_001_index.json").write_text(
        json.dumps({"round": 1, "status": "ok", "reports": []}),
        encoding="utf-8",
    )
    (reports_dir / "smoke_001_index.json").write_text(
        json.dumps({"round": 1, "status": "bug", "reports": []}),
        encoding="utf-8",
    )
    (reports_dir / "runtime_error_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "runtime_error",
                "reports": [
                    {
                        "status": "runtime_error",
                        "classification": "panic_or_crash",
                        "summary": "slice range end index out of bounds before target returned",
                        "report": str(reports_dir / "runtime_error_001_01.json"),
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    markdown = render_bug_record_entry(
        workspace,
        crate_name="bytes",
        phase="Phase 3 runtime rerun",
    )

    assert "- **运行时诊断**：" in markdown
    assert "- `panic_or_crash`：slice range end index out of bounds before target returned" in markdown
