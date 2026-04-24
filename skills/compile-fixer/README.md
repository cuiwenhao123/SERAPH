# compile-fixer

`compile-fixer` repairs Rust harnesses generated from SERAPH v5.1 RAG context.

## Inputs

- Generated harness source file.
- `rustc` / `cargo fuzz build` diagnostics.
- The RAG context markdown used by `harness-codegen`.

## Rules

1. The fixer must not remove the target API from the harness.
2. The fixer must not remove or rename `SERAPH_STEP_ENTER:<step_no>:<api_id>` markers.
3. The fixer must not remove or rename `SERAPH_STEP_OK:<step_no>:<api_id>` markers.
4. Prefer precise compiler-driven edits: imports, crate paths, type annotations, borrow scopes, generic arguments, and result handling.
5. Preserve the semantic intent of the RAG context: construct public state, call the unsafe target meaningfully, and keep unsafe blocks narrow.
6. If compilation can only be achieved by deleting the target API call, report failure instead of producing a misleading harness.

## Retry Strategy

- Retry 1-2: minimal compiler-directed fixes.
- Retry 3-4: local restructuring of setup code, result handling, or borrow scopes.
- Retry 5: simplify surrounding code while preserving the target API and SERAPH markers.
