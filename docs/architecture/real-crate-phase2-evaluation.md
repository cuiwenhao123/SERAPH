# Real Crate Phase 1/2 Evaluation Report

Evaluation base: `/tmp/seraph-real-crate-phase2-eval-v3`.

| Crate | APIs | Types | Traits | Impl Surfaces | Unsafe Candidates | Graph Nodes/Edges | Context | Result |
|-------|------|-------|--------|---------------|-------------------|-------------------|---------|--------|
| s3-audit-fixture | 15 | 9 | 2 | 5 | 1 | 27/40 | 3438 chars | ok |
| hashbrown | 231 | 52 | 12 | 208 | 58 | 299/718 | 5987 chars | ok |
| bytes | 203 | 10 | 6 | 117 | 26 | 221/362 | 5697 chars | ok |
| crossbeam-queue | 17 | 2 | 0 | 15 | 0 | 20/38 | - | no unsafe target |

## s3-audit-fixture

- Crate import name: `s3_audit_fixture`
- Unsafe candidates: `1`
- Unsafe sample:
  - `api::s3_audit_fixture::uses_unsafe_block` → `s3_audit_fixture::uses_unsafe_block`
- Graph: `27` nodes, `40` edges
- Edge kinds: `{'module_contains_type': 9, 'module_contains_api': 15, 'type_owns_method': 8, 'impl_connects_type_trait': 2, 'api_returns_type': 5, 'api_accepts_trait': 1}`
- Context: `/tmp/seraph-real-crate-phase2-eval-v3/s3-audit-fixture/contexts/rag_target_001.md`
- Target: - api_id: api::s3_audit_fixture::uses_unsafe_block
- Signature: - signature: fn uses_unsafe_block(*const u8) -> Option<u8>
- Safety: - safety: `
- Section completeness: `{'## Target API': True, '## Related APIs': True, '## Semantically Similar API Docs': True, '## Rust Idioms': True, '## Generation Rules': True}`
- Related/similar/idiom counts: `14` / `4` / `5`

## hashbrown

- Crate import name: `hashbrown`
- Unsafe candidates: `58`
- Unsafe sample:
  - `api::hashbrown::map::HashMap::extract_if` → `hashbrown::map::HashMap::extract_if`
  - `api::hashbrown::map::HashMap::get_disjoint_key_value_unchecked_mut` → `hashbrown::map::HashMap::get_disjoint_key_value_unchecked_mut`
  - `api::hashbrown::map::HashMap::get_disjoint_unchecked_mut` → `hashbrown::map::HashMap::get_disjoint_unchecked_mut`
  - `api::hashbrown::map::HashMap::get_many_key_value_unchecked_mut` → `hashbrown::map::HashMap::get_many_key_value_unchecked_mut`
  - `api::hashbrown::map::HashMap::get_many_unchecked_mut` → `hashbrown::map::HashMap::get_many_unchecked_mut`
  - `api::hashbrown::map::HashMap::insert` → `hashbrown::map::HashMap::insert`
  - `api::hashbrown::map::HashMap::insert_unique_unchecked` → `hashbrown::map::HashMap::insert_unique_unchecked`
  - `api::hashbrown::map::HashMap::iter` → `hashbrown::map::HashMap::iter`
- Graph: `299` nodes, `718` edges
- Edge kinds: `{'module_contains_type': 52, 'module_contains_api': 231, 'impl_connects_type_trait': 26, 'type_owns_method': 231, 'api_returns_type': 178}`
- Context: `/tmp/seraph-real-crate-phase2-eval-v3/hashbrown/contexts/rag_target_001.md`
- Target: - api_id: api::hashbrown::map::HashMap::extract_if
- Signature: - signature: fn extract_if(&mut Self, F) -> ExtractIf<'_, K, V, F, A>
- Safety: - safety: `
- Section completeness: `{'## Target API': True, '## Related APIs': True, '## Semantically Similar API Docs': True, '## Rust Idioms': True, '## Generation Rules': True}`
- Related/similar/idiom counts: `24` / `4` / `5`

## bytes

- Crate import name: `bytes`
- Unsafe candidates: `26`
- Unsafe sample:
  - `api::bytes::buf::buf_mut::BufMut::advance_mut` → `bytes::buf::buf_mut::BufMut::advance_mut`
  - `api::bytes::buf::buf_mut::BufMut::put` → `bytes::buf::buf_mut::BufMut::put`
  - `api::bytes::buf::buf_mut::BufMut::put_bytes` → `bytes::buf::buf_mut::BufMut::put_bytes`
  - `api::bytes::buf::buf_mut::BufMut::put_slice` → `bytes::buf::buf_mut::BufMut::put_slice`
  - `api::bytes::buf::uninit_slice::UninitSlice::as_uninit_slice_mut` → `bytes::buf::uninit_slice::UninitSlice::as_uninit_slice_mut`
  - `api::bytes::buf::uninit_slice::UninitSlice::copy_from_slice` → `bytes::buf::uninit_slice::UninitSlice::copy_from_slice`
  - `api::bytes::buf::uninit_slice::UninitSlice::from_raw_parts_mut` → `bytes::buf::uninit_slice::UninitSlice::from_raw_parts_mut`
  - `api::bytes::buf::uninit_slice::UninitSlice::new` → `bytes::buf::uninit_slice::UninitSlice::new`
- Graph: `221` nodes, `362` edges
- Edge kinds: `{'module_contains_type': 10, 'module_contains_api': 203, 'type_owns_method': 68, 'impl_connects_type_trait': 11, 'api_returns_type': 68, 'api_accepts_type': 2}`
- Context: `/tmp/seraph-real-crate-phase2-eval-v3/bytes/contexts/rag_target_001.md`
- Target: - api_id: api::bytes::buf::uninit_slice::UninitSlice::from_raw_parts_mut
- Signature: - signature: fn from_raw_parts_mut(*mut u8, usize) -> &'a mut UninitSlice
- Safety: - safety: The caller must ensure that `ptr` references a valid memory region owned`
- Section completeness: `{'## Target API': True, '## Related APIs': True, '## Semantically Similar API Docs': True, '## Rust Idioms': True, '## Generation Rules': True}`
- Related/similar/idiom counts: `24` / `4` / `5`

## crossbeam-queue

- Crate import name: `crossbeam_queue`
- Unsafe candidates: `0`
- Graph: `20` nodes, `38` edges
- Edge kinds: `{'module_contains_type': 2, 'module_contains_api': 17, 'type_owns_method': 17, 'api_returns_type': 2}`
- Context: not generated because no unsafe target was available.
- CLI outcome: clean `no unsafe targets available` message.

## Overall Assessment

- Flow correctness: Phase 1 extraction, vector indexing, graph construction, target ranking, and context retrieval completed for crates with unsafe candidates.
- Input/output correctness: expected artifacts were produced: `knowledge.json`, `vectordb/chroma.sqlite3`, `graph.pkl`, and `contexts/rag_target_001.md`.
- Target alignment: generated contexts are unsafe-target-centered. `bytes` selects a pointer/safety-doc API; `hashbrown` selects an internal-unsafe `HashMap` API; `s3-audit-fixture` selects the fixture unsafe-block API.
- Completeness: after section-budget fixes, generated contexts preserve all core sections: target, related APIs, similar API docs, Rust idioms, and generation rules.
- Safe-only behavior: `crossbeam-queue` has no extracted unsafe public API candidate and now exits with a clean `no unsafe targets available` message.

## Content Quality Notes

- `bytes` is the best-quality result: safety docs are present, related APIs focus on `UninitSlice`, and similar docs/idioms are relevant to raw pointer and initialization invariants.
- `hashbrown` is correct but still broad: owner-type graph expansion around `HashMap` can produce many related APIs. The related API ranking/limit now keeps context complete, but future work should rank constructors and safety-doc/unchecked APIs more precisely.
- `s3-audit-fixture` is useful for regression because it exercises `contains_unsafe_block`, but its free-function raw pointer target has limited graph-neighbor context.
- Earlier hashing-based validation established the pipeline, but the active path now uses OpenAI-compatible embedding backends for real-crate evaluation.

## Remaining Phase 1/2 Improvements

1. Add a batch evaluation command that records `ok` vs `no_targets` without treating safe-only crates as failed runs.
2. Improve unsafe target priority so APIs with explicit `unchecked`, raw pointer args, or `# Safety` docs outrank broad internal-unsafe methods when appropriate.
3. Add graph-neighbor labels/reasons in context so LLM can distinguish constructors, mutators, query methods, and safety-relevant APIs.
4. Add a real embedding backend only after graph/ranking behavior is stable.
