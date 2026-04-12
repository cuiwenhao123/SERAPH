# s3-extract

This crate is reserved for Phase 1 raw extraction from rustdoc JSON, AST sources, Cargo metadata, and example indexes.

It should only produce raw extraction artifacts corresponding to `knowledge.json`.

Current implementation status:

- provides a minimal bootstrap generator for valid `knowledge.json`
- normalizes `crate_import_name` from crate names such as `serde-json-wrapper`
- writes a placeholder extraction artifact without pretending rustdoc / AST extraction is complete

This is an artifact-contract bootstrap, not the final extractor pipeline.
