from pathlib import Path

import pytest

from seraph_rag.cli import main
from seraph_rag.graph_builder import build_graph, write_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.vector_index import index_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_cli_retrieve_reports_no_unsafe_targets_cleanly(tmp_path):
    knowledge = load_knowledge(FIXTURE)
    for api in knowledge["apis"]:
        api["is_unsafe"] = False
        api["contains_unsafe_block"] = False
    knowledge["risk_facts"]["unsafe_functions"] = []
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    index_knowledge(knowledge, vectordb)
    write_graph(build_graph(knowledge), graph_path)

    with pytest.raises(SystemExit, match="no unsafe targets available"):
        main([
            "retrieve",
            "--knowledge",
            str(FIXTURE),
            "--graph",
            str(graph_path),
            "--vectordb",
            str(vectordb),
            "--output",
            str(tmp_path / "context.md"),
        ])
