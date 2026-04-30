from __future__ import annotations

import os
from pathlib import Path
from typing import Optional, Union

from seraph_rag.documents import build_api_documents
from seraph_rag.embeddings import Embedder, build_embedder, embedder_backend_name
from seraph_rag.idioms import bundled_idioms


def ensure_chromadb_sqlite() -> None:
    import sqlite3
    import sys

    if sqlite3.sqlite_version_info >= (3, 35, 0):
        return
    try:
        import pysqlite3
    except ImportError as exc:
        raise RuntimeError(
            "ChromaDB requires sqlite3 >= 3.35.0. Install pysqlite3-binary "
            "or use a newer Python sqlite build."
        ) from exc
    sys.modules["sqlite3"] = pysqlite3


def persistent_client(path: Union[str, Path]):
    ensure_chromadb_sqlite()
    import chromadb
    from chromadb.config import Settings

    settings = Settings(
        anonymized_telemetry=False,
        chroma_product_telemetry_impl="seraph_rag.chroma_telemetry.NoopProductTelemetryClient",
        chroma_telemetry_impl="seraph_rag.chroma_telemetry.NoopProductTelemetryClient",
    )
    return chromadb.PersistentClient(path=str(path), settings=settings)


def index_knowledge(
    knowledge: dict,
    vectordb: Union[str, Path],
    embedder: Optional[Embedder] = None,
    batch_size: Optional[int] = None,
) -> None:
    embedder = embedder or build_embedder()
    if batch_size is None:
        batch_size = int(os.environ.get("SERAPH_EMBEDDING_BATCH_SIZE", "8"))
    client = persistent_client(vectordb)
    api_collection = client.get_or_create_collection(
        name="api_docs",
        metadata={"hnsw:space": "cosine", "seraph:embedder": embedder_backend_name()},
    )
    api_docs = build_api_documents(knowledge)
    if api_docs:
        _upsert_documents_in_batches(api_collection, api_docs, embedder, batch_size=batch_size)
    idiom_collection = client.get_or_create_collection(
        name="rust_idioms",
        metadata={"hnsw:space": "cosine", "seraph:embedder": embedder_backend_name()},
    )
    idioms = bundled_idioms()
    _upsert_documents_in_batches(idiom_collection, idioms, embedder, batch_size=batch_size)


def _upsert_documents_in_batches(collection, documents, embedder: Embedder, *, batch_size: int) -> None:
    if batch_size <= 0:
        batch_size = len(documents) or 1
    for start in range(0, len(documents), batch_size):
        batch = documents[start : start + batch_size]
        collection.upsert(
            ids=[doc.doc_id for doc in batch],
            embeddings=embedder.encode([doc.text for doc in batch]),
            documents=[doc.text for doc in batch],
            metadatas=[doc.metadata for doc in batch],
        )
