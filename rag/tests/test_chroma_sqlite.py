def test_persistent_client_imports_chromadb_on_old_system_sqlite(tmp_path):
    from seraph_rag.vector_index import persistent_client

    client = persistent_client(tmp_path / "vectordb")
    assert client is not None
