# api-planner

Deprecated in v5.1 active path.

SERAPH v5.1 no longer uses this skill in the main harness synthesis pipeline. The active path is:

1. `s3-extract` produces `knowledge.json`.
2. `seraph_rag.cli index` builds `workspace/vectordb/`.
3. `seraph_rag.cli graph` builds `workspace/graph.pkl`.
4. `seraph_rag.cli retrieve` builds `workspace/contexts/rag_target_*.md`.
5. `harness-codegen` consumes the RAG context directly and lets the LLM decide the call sequence.

This directory is retained only for historical comparison with the older scenario/mapping/planning design.
