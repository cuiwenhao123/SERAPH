from pathlib import Path

from seraph_rag.cli import main
from seraph_rag.knowledge_loader import load_knowledge
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
