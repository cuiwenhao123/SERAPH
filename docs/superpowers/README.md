# Superpowers Docs Guide

## Current Source Of Truth

- Current active schema contract:
  - `/home/cas/Desktop/SERAPH/docs/superpowers/specs/2026-04-15-knowledge-rs-v3-schema-spec.md`
- Current behavior should ultimately be checked against:
  - implementation in `crates/seraph-types/src/knowledge.rs`
  - implementation in `crates/s3-extract/src/lib.rs`
  - regression tests in `crates/seraph-types/tests/schema_roundtrip.rs`
  - regression tests in `crates/s3-extract/tests/minimal_extract.rs`

## Historical Docs

- Files under `docs/superpowers/plans/` are planning artifacts, not live schema contracts.
- `2026-04-13-knowledge-rs-v2-schema-spec.md` is a historical snapshot superseded by the v3 spec.
- Historical docs are still useful for audit trail and design evolution, but they should not override current code and tests.
