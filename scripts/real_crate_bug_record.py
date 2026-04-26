#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "rag"))

from seraph_rag.bug_record import append_bug_record_entry, render_bug_record_entry  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Render a reusable Markdown bug-record section from a real-crate "
            "SERAPH workspace."
        )
    )
    parser.add_argument("--workspace-dir", required=True)
    parser.add_argument("--crate", dest="crate_name", required=True)
    parser.add_argument("--phase", default="Phase 3")
    parser.add_argument("--round", type=int)
    parser.add_argument("--title")
    parser.add_argument("--symptom", action="append", default=[])
    parser.add_argument("--rust-feature", action="append", default=[])
    parser.add_argument("--root-cause", action="append", default=[])
    parser.add_argument("--tool-fix", action="append", default=[])
    parser.add_argument("--capability-gain", action="append", default=[])
    parser.add_argument("--output")
    parser.add_argument("--append-doc")
    args = parser.parse_args()

    markdown = render_bug_record_entry(
        args.workspace_dir,
        crate_name=args.crate_name,
        phase=args.phase,
        round_no=args.round,
        title=args.title,
        symptom=args.symptom,
        rust_feature=args.rust_feature,
        root_cause=args.root_cause,
        tool_fix=args.tool_fix,
        capability_gain=args.capability_gain,
    )

    if args.output:
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(markdown, encoding="utf-8")
    else:
        sys.stdout.write(markdown)

    if args.append_doc:
        append_bug_record_entry(args.append_doc, markdown)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
