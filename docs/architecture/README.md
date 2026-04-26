# Architecture Notes

- `phase2-rag-redesign-v5.md` documents the Phase 2 reset from static modeling artifacts to a Python RAG knowledge layer.
- `phase2-rag-quickstart.md` records the Python/ChromaDB setup and smoke-test commands for the RAG path.
- `current-phase1-phase2-flow.md` describes the current executable Phase 1/2 flow and validation commands.
- `real-crate-phase2-evaluation.md` records observed Phase 1/2 behavior across several real crates.
- `extended-real-crate-phase2-evaluation.md` records a broader 10-crate Phase 1/2 evaluation after target/context quality optimizations.
- `rust-feature-bug-record.md` records real-crate failures, Rust-specific root causes, and the corresponding SERAPH design fixes for paper writing and regression tracking.

This directory will hold high-level architecture documents for SERAPH, including pipeline diagrams, crate boundaries, data flow notes, and runtime control-surface decisions.
- `phase3-harness-generation.md` documents the first executable Phase 3 step: RAG context to harness generation prompt bundle.
