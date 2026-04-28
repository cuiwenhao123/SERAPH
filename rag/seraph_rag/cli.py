from __future__ import annotations

import argparse
import glob
import json
from pathlib import Path
from typing import List, Optional

from seraph_rag.compile_check import DEFAULT_COMMAND_TEMPLATE, run_compile_check, run_compile_checks
from seraph_rag.compile_fixer_bundle import write_compile_fixer_bundles
from seraph_rag.compile_fixer_response import write_fixed_harness
from seraph_rag.fix_loop import run_fix_loop
from seraph_rag.fix_once import run_fix_once
from seraph_rag.fix_acceptance import write_fix_acceptance_index
from seraph_rag.fix_acceptance_write import (
    write_fix_acceptance_decision,
    write_fix_acceptance_decisions,
)
from seraph_rag.graph_builder import build_graph, read_graph, write_graph
from seraph_rag.harness_codegen import write_harnesses_from_response
from seraph_rag.harness_prompt import DEFAULT_HARNESS_STYLE, write_prompt_bundle
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.merge_harnesses import write_merged_harnesses
from seraph_rag.model_provider import write_model_response
from seraph_rag.runtime_diagnose import run_runtime_diagnosis
from seraph_rag.smoke_run import (
    DEFAULT_SMOKE_COMMAND_TEMPLATE,
    collect_successful_harnesses,
    run_smoke_check,
    run_smoke_checks,
)
from seraph_rag.retrieve import (
    rank_unsafe_targets,
    select_unsafe_target,
    render_context_from_stores,
    render_context_markdown,
)
from seraph_rag.vector_index import index_knowledge


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="seraph-rag")
    subparsers = parser.add_subparsers(dest="command", required=True)

    index_parser = subparsers.add_parser("index")
    index_parser.add_argument("--knowledge", required=True)
    index_parser.add_argument("--vectordb", required=True)

    graph_parser = subparsers.add_parser("graph")
    graph_parser.add_argument("--knowledge", required=True)
    graph_parser.add_argument("--graph", required=True)

    targets_parser = subparsers.add_parser("targets")
    targets_parser.add_argument("--graph", required=True)

    retrieve_parser = subparsers.add_parser("retrieve")
    retrieve_parser.add_argument("--knowledge", required=True)
    retrieve_parser.add_argument("--graph", required=True)
    retrieve_parser.add_argument("--vectordb")
    retrieve_parser.add_argument("--output", required=True)
    retrieve_parser.add_argument("--round", type=int, default=1)
    retrieve_parser.add_argument("--target-api-id")

    prompt_parser = subparsers.add_parser("harness-prompt")
    prompt_parser.add_argument("--context", required=True)
    prompt_parser.add_argument("--output", required=True)
    prompt_parser.add_argument("--variants", type=int, default=3)
    prompt_parser.add_argument("--style", default=DEFAULT_HARNESS_STYLE)

    write_parser = subparsers.add_parser("harness-write")
    write_parser.add_argument("--prompt", required=True)
    write_parser.add_argument("--response", required=True)
    write_parser.add_argument("--output-dir", required=True)
    write_parser.add_argument("--round", type=int, required=True)

    compile_parser = subparsers.add_parser("compile-check")
    compile_parser.add_argument("--harness")
    compile_parser.add_argument("--report")
    compile_parser.add_argument("--harness-glob")
    compile_parser.add_argument("--report-dir")
    compile_parser.add_argument("--round", type=int)
    compile_parser.add_argument("--command-template", default=DEFAULT_COMMAND_TEMPLATE)

    smoke_parser = subparsers.add_parser("smoke-run")
    smoke_parser.add_argument("--harness")
    smoke_parser.add_argument("--report")
    smoke_parser.add_argument("--harness-glob")
    smoke_parser.add_argument("--compile-index")
    smoke_parser.add_argument("--fix-loop-index")
    smoke_parser.add_argument("--report-dir")
    smoke_parser.add_argument("--round", type=int)
    smoke_parser.add_argument("--command-template", default=DEFAULT_SMOKE_COMMAND_TEMPLATE)

    merge_parser = subparsers.add_parser("merge-harnesses")
    merge_parser.add_argument("--workspace-dir", required=True)
    merge_parser.add_argument("--round", type=int, required=True)

    runtime_diagnose_parser = subparsers.add_parser("runtime-diagnose")
    runtime_diagnose_parser.add_argument("--context", required=True)
    runtime_diagnose_parser.add_argument("--smoke-index", required=True)
    runtime_diagnose_parser.add_argument("--output-dir", required=True)
    runtime_diagnose_parser.add_argument("--round", type=int, required=True)
    runtime_diagnose_parser.add_argument("--command-template", required=True)

    fixer_parser = subparsers.add_parser("fixer-bundle")
    fixer_parser.add_argument("--compile-index", required=True)
    fixer_parser.add_argument("--context", required=True)
    fixer_parser.add_argument("--output-dir", required=True)

    fixer_write_parser = subparsers.add_parser("fixer-write")
    fixer_write_parser.add_argument("--request", required=True)
    fixer_write_parser.add_argument("--response", required=True)
    fixer_write_parser.add_argument("--output-dir", required=True)
    fixer_write_parser.add_argument("--attempt", type=int, required=True)

    fix_once_parser = subparsers.add_parser("fix-once")
    fix_once_parser.add_argument("--request", required=True)
    fix_once_parser.add_argument("--response", required=True)
    fix_once_parser.add_argument("--output-dir", required=True)
    fix_once_parser.add_argument("--report-dir", required=True)
    fix_once_parser.add_argument("--attempt", type=int, required=True)
    fix_once_parser.add_argument("--command-template", default=DEFAULT_COMMAND_TEMPLATE)

    fix_loop_parser = subparsers.add_parser("fix-loop")
    fix_loop_parser.add_argument("--request", required=True)
    fix_loop_parser.add_argument("--responses-dir", required=True)
    fix_loop_parser.add_argument("--output-dir", required=True)
    fix_loop_parser.add_argument("--report-dir", required=True)
    fix_loop_parser.add_argument("--max-attempts", type=int, required=True)
    fix_loop_parser.add_argument("--command-template", default=DEFAULT_COMMAND_TEMPLATE)
    fix_loop_parser.add_argument("--response-command-template")

    fix_loop_batch_parser = subparsers.add_parser("fix-loop-batch")
    fix_loop_batch_parser.add_argument("--request-glob", required=True)
    fix_loop_batch_parser.add_argument("--responses-dir", required=True)
    fix_loop_batch_parser.add_argument("--output-dir", required=True)
    fix_loop_batch_parser.add_argument("--report-dir", required=True)
    fix_loop_batch_parser.add_argument("--index-output", required=True)
    fix_loop_batch_parser.add_argument("--max-attempts", type=int, required=True)
    fix_loop_batch_parser.add_argument("--command-template", default=DEFAULT_COMMAND_TEMPLATE)
    fix_loop_batch_parser.add_argument("--response-command-template")

    fix_acceptance_parser = subparsers.add_parser("fix-acceptance")
    fix_acceptance_parser.add_argument("--fix-loop-index", required=True)
    fix_acceptance_parser.add_argument("--smoke-index")
    fix_acceptance_parser.add_argument("--output", required=True)

    fix_acceptance_write_parser = subparsers.add_parser("fix-acceptance-write")
    fix_acceptance_write_parser.add_argument("--index", required=True)
    fix_acceptance_write_parser.add_argument("--harness", required=True)
    fix_acceptance_write_parser.add_argument("--status", required=True)
    fix_acceptance_write_parser.add_argument("--reason", required=True)
    fix_acceptance_write_parser.add_argument("--source", default="manual")

    fix_acceptance_write_batch_parser = subparsers.add_parser("fix-acceptance-write-batch")
    fix_acceptance_write_batch_parser.add_argument("--index", required=True)
    fix_acceptance_write_batch_parser.add_argument("--decisions", required=True)
    fix_acceptance_write_batch_parser.add_argument("--source", default="manual")

    model_response_parser = subparsers.add_parser("model-response")
    model_response_parser.add_argument("--input", required=True)
    model_response_parser.add_argument("--output", required=True)
    model_response_parser.add_argument("--command-template", required=True)
    model_response_parser.add_argument("--attempt", type=int)

    return parser


def main(argv: Optional[List[str]] = None) -> None:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "index":
        index_knowledge(load_knowledge(args.knowledge), args.vectordb)
        return
    if args.command == "graph":
        write_graph(build_graph(load_knowledge(args.knowledge)), args.graph)
        return
    if args.command == "targets":
        for target in rank_unsafe_targets(read_graph(args.graph)):
            print("{}\t{:.2f}\t{}".format(target.api_id, target.score, target.path))
        return
    if args.command == "retrieve":
        knowledge = load_knowledge(args.knowledge)
        if args.vectordb:
            try:
                markdown = render_context_from_stores(
                    knowledge,
                    args.vectordb,
                    args.graph,
                    round_no=args.round,
                    target_api_id=args.target_api_id,
                )
            except ValueError as exc:
                raise SystemExit(str(exc)) from exc
        else:
            graph = read_graph(args.graph)
            try:
                target = select_unsafe_target(
                    graph,
                    round_no=args.round,
                    target_api_id=args.target_api_id,
                )
            except ValueError as exc:
                raise SystemExit(str(exc)) from exc
            markdown = render_context_markdown(knowledge, graph, target)
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(markdown, encoding="utf-8")
        return
    if args.command == "harness-prompt":
        write_prompt_bundle(args.context, args.output, variants=args.variants, style=args.style)
        return
    if args.command == "harness-write":
        write_harnesses_from_response(args.prompt, args.response, args.output_dir, args.round)
        return
    if args.command == "compile-check":
        if args.harness_glob:
            if not args.report_dir or args.round is None:
                raise SystemExit("--harness-glob requires --report-dir and --round")
            harnesses = sorted(glob.glob(args.harness_glob))
            if not harnesses:
                raise SystemExit("no harnesses matched: {}".format(args.harness_glob))
            run_compile_checks(
                harnesses,
                args.report_dir,
                args.round,
                command_template=args.command_template,
            )
            return
        if not args.harness or not args.report:
            raise SystemExit("compile-check requires --harness/--report or --harness-glob/--report-dir/--round")
        run_compile_check(args.harness, args.report, command_template=args.command_template)
        return
    if args.command == "smoke-run":
        if args.harness_glob:
            if not args.report_dir or args.round is None:
                raise SystemExit("--harness-glob requires --report-dir and --round")
            harnesses = sorted(glob.glob(args.harness_glob))
            run_smoke_checks(
                harnesses,
                args.report_dir,
                args.round,
                command_template=args.command_template,
            )
            return
        if args.compile_index or args.fix_loop_index:
            if not args.report_dir or args.round is None:
                raise SystemExit("--compile-index/--fix-loop-index require --report-dir and --round")
            harnesses = collect_successful_harnesses(args.compile_index, args.fix_loop_index)
            run_smoke_checks(
                harnesses,
                args.report_dir,
                args.round,
                command_template=args.command_template,
            )
            return
        if not args.harness or not args.report:
            raise SystemExit(
                "smoke-run requires --harness/--report, --harness-glob/--report-dir/--round, or --compile-index/--report-dir/--round"
            )
        run_smoke_check(args.harness, args.report, command_template=args.command_template)
        return
    if args.command == "merge-harnesses":
        crate_config = json.loads(
            (Path(args.workspace_dir) / "crate_config.json").read_text(encoding="utf-8")
        )
        write_merged_harnesses(
            workspace_dir=args.workspace_dir,
            round_no=args.round,
            crate_name=str(crate_config["package_name"]),
            crate_import_name=str(crate_config["crate_import_name"]),
        )
        return
    if args.command == "runtime-diagnose":
        run_runtime_diagnosis(
            args.context,
            args.smoke_index,
            args.output_dir,
            args.round,
            args.command_template,
        )
        return
    if args.command == "fixer-bundle":
        write_compile_fixer_bundles(args.compile_index, args.context, args.output_dir)
        return
    if args.command == "fixer-write":
        write_fixed_harness(args.request, args.response, args.output_dir, args.attempt)
        return
    if args.command == "fix-once":
        run_fix_once(
            args.request,
            args.response,
            args.output_dir,
            args.report_dir,
            args.attempt,
            command_template=args.command_template,
        )
        return
    if args.command == "fix-loop":
        run_fix_loop(
            args.request,
            args.responses_dir,
            args.output_dir,
            args.report_dir,
            max_attempts=args.max_attempts,
            command_template=args.command_template,
            response_command_template=args.response_command_template,
        )
        return
    if args.command == "fix-loop-batch":
        from seraph_rag.fix_loop import run_fix_loops

        run_fix_loops(
            args.request_glob,
            args.responses_dir,
            args.output_dir,
            args.report_dir,
            args.index_output,
            max_attempts=args.max_attempts,
            command_template=args.command_template,
            response_command_template=args.response_command_template,
        )
        return
    if args.command == "fix-acceptance":
        write_fix_acceptance_index(
            args.fix_loop_index,
            args.smoke_index,
            args.output,
        )
        return
    if args.command == "fix-acceptance-write":
        write_fix_acceptance_decision(
            args.index,
            args.harness,
            args.status,
            args.reason,
            source=args.source,
        )
        return
    if args.command == "fix-acceptance-write-batch":
        write_fix_acceptance_decisions(
            args.index,
            args.decisions,
            source=args.source,
        )
        return
    if args.command == "model-response":
        write_model_response(
            args.input,
            args.output,
            args.command_template,
            attempt=args.attempt,
        )
        return
    parser.error("unknown command: {}".format(args.command))


if __name__ == "__main__":
    main()
