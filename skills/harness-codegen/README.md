# harness-codegen

`harness-codegen` is the Phase 3 generation skill for the SERAPH v5.1 active path. It consumes a RAG context markdown file produced by `seraph_rag.cli retrieve` and emits Rust fuzz harness source files.

## RAG Context Input

Input file pattern:

```text
workspace/contexts/rag_target_*.md
```

The context is LLM-readable markdown with these sections:

- `Target API`: the unsafe API that every generated harness variant must call.
- `Related APIs`: graph-neighbor APIs useful for construction, mutation, query, and cleanup.
- `Semantically Similar API Docs`: ChromaDB vector matches from `api_docs`.
- `Rust Idioms`: ChromaDB vector matches from `rust_idioms`.
- `Generation Rules`: deterministic constraints that must be preserved.

Do not require scenario.json, api_mapping.json, api_plan.json, `ScenarioArtifact`, `ApiMappingArtifact`, or `ApiPlanArtifact` for the v5.1 active path.

## Output

Generate 2-3 harness variants per RAG context unless the context is too small to support meaningful variation.

Recommended filename shape:

```text
workspace/fuzz/fuzz_targets/harness_RRR_SS.rs
```

Where:

- `RRR` is the zero-padded round number.
- `SS` is a zero-padded variant index.

## Generation Rules

Every generated harness must:

1. Call the target unsafe API from the `Target API` section.
2. Use the real crate import name shown in the RAG context; never invent `target_lib`.
3. Use `Related APIs` and similar docs only as context. The LLM decides the concrete call sequence.
4. Prefer public constructors and public mutation APIs over fabricating internal state.
5. Treat `Rust Idioms` as safety guidance, especially for ownership, FFI, Drop, trait safety, and unsafe preconditions.
6. Handle recoverable `Result` and `Option` paths with `match`, `if let`, or early return; do not blindly `unwrap()`.
7. Keep unsafe blocks as narrow as possible and justify them through valid public preconditions.
8. Emit `SERAPH_STEP_ENTER:<step_no>:<api_id>` immediately before each targeted API call.
9. Emit `SERAPH_STEP_OK:<step_no>:<api_id>` immediately after the targeted API call returns successfully.
10. Preserve the exact stable `api_id` strings from the RAG context in all SERAPH markers.

## Diversity Strategy

Across the 2-3 harness variants, vary at least one of:

- Construction path from `Related APIs`.
- Input source: raw `&[u8]`, structured arbitrary data, boundary values, or split slices.
- Call depth: direct target call after minimal setup vs. longer setup/mutation chain.
- Error path: normal path vs. documented boundary/error path.

## Failure Handling

If a safe and meaningful call to the target API cannot be constructed from the provided context:

- Emit the smallest harness that still reaches the target API through public APIs.
- Leave SERAPH markers intact.
- Do not silently replace the target API with a different API.
- Prefer compile-fixable code over speculative private-field construction.

## Compile-Fixer Handoff

The compile fixer receives:

- The generated harness source.
- Rust compiler diagnostics.
- The same RAG context used for generation.

The fixer may adjust imports, type annotations, generic choices, lifetimes, and local helper code, but it must not remove the target API or SERAPH markers.
