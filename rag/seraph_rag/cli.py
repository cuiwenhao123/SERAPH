from __future__ import annotations

import argparse
from pathlib import Path
from typing import List, Optional

from seraph_rag.graph_builder import build_graph, read_graph, write_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import (
    rank_unsafe_targets,
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
                markdown = render_context_from_stores(knowledge, args.vectordb, args.graph)
            except ValueError as exc:
                raise SystemExit(str(exc)) from exc
        else:
            graph = read_graph(args.graph)
            targets = rank_unsafe_targets(graph)
            if not targets:
                raise SystemExit("no unsafe targets available")
            markdown = render_context_markdown(knowledge, graph, targets[0])
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(markdown, encoding="utf-8")
        return
    parser.error("unknown command: {}".format(args.command))


if __name__ == "__main__":
    main()
