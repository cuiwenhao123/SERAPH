# Extended Real Crate Phase 1/2 Evaluation Report

Evaluation base: `/tmp/seraph-real-crate-phase2-eval-v4`.

| Crate | Status | APIs | Unsafe Candidates | Target | Target Signal | Context Sections | Related/Similar/Idioms |
|-------|--------|------|-------------------|--------|---------------|------------------|------------------------|
| s3-audit-fixture | ok | 15 | 1 | `s3_audit_fixture::uses_unsafe_block` | unsafe_block,raw_ptr_arg | True | 14/4/5 |
| hashbrown | ok | 231 | 58 | `hashbrown::map::VacantEntryRef::insert_entry_with_key_unchecked` | unsafe_fn,unchecked_name,safety_doc | True | 24/4/5 |
| bytes | ok | 203 | 26 | `bytes::buf::uninit_slice::UninitSlice::from_raw_parts_mut` | unsafe_fn,safety_doc,raw_ptr_arg | True | 24/4/5 |
| crossbeam-queue | no_targets | 17 | 0 | - | - | - | - |
| globset | no_targets | 26 | 0 | - | - | - | - |
| url | no_targets | 58 | 0 | - | - | - | - |
| camino | ok | 84 | 14 | `camino::Utf8Components::as_path` | unsafe_block | True | 24/4/5 |
| tar | ok | 168 | 18 | `tar::header::GnuExtSparseHeader::new` | unsafe_block | True | 24/4/5 |
| csv-core | no_targets | 40 | 0 | - | - | - | - |
| http | ok | 195 | 6 | `http::header::value::HeaderValue::from_maybe_shared_unchecked` | unsafe_fn,unchecked_name,safety_doc | True | 24/4/5 |

## Per-Crate Details

### s3-audit-fixture
- Crate import: `s3_audit_fixture`
- APIs/types/traits/impls: `15` / `9` / `2` / `5`
- Unsafe candidates: `1`
- Graph nodes/edges: `27` / `40`
- Edge kinds: `{'module_contains_type': 9, 'module_contains_api': 15, 'type_owns_method': 8, 'impl_connects_type_trait': 2, 'api_returns_type': 5, 'api_accepts_trait': 1}`
- Selected target: `api::s3_audit_fixture::uses_unsafe_block` → `s3_audit_fixture::uses_unsafe_block`
- Signature: `fn uses_unsafe_block(*const u8) -> Option<u8>`
- Safety doc present: `False`
- Target flags: `is_unsafe=False`, `contains_unsafe_block=True`, args=`['*const u8']`, return=`Option<u8>`
- Context chars: `3668`
- Core sections complete: `True`
- Related/similar/idioms: `14` / `4` / `5`

### hashbrown
- Crate import: `hashbrown`
- APIs/types/traits/impls: `231` / `52` / `12` / `208`
- Unsafe candidates: `58`
- Graph nodes/edges: `299` / `718`
- Edge kinds: `{'module_contains_type': 52, 'module_contains_api': 231, 'impl_connects_type_trait': 26, 'type_owns_method': 231, 'api_returns_type': 178}`
- Selected target: `api::hashbrown::map::VacantEntryRef::insert_entry_with_key_unchecked` → `hashbrown::map::VacantEntryRef::insert_entry_with_key_unchecked`
- Signature: `fn insert_entry_with_key_unchecked(Self, K, V) -> OccupiedEntry<'map, K, V, S, A>`
- Safety doc present: `True`
- Target flags: `is_unsafe=True`, `contains_unsafe_block=False`, args=`['K', 'V']`, return=`OccupiedEntry<'map, K, V, S, A>`
- Context chars: `8915`
- Core sections complete: `True`
- Related/similar/idioms: `24` / `4` / `5`

### bytes
- Crate import: `bytes`
- APIs/types/traits/impls: `203` / `10` / `6` / `117`
- Unsafe candidates: `26`
- Graph nodes/edges: `221` / `362`
- Edge kinds: `{'module_contains_type': 10, 'module_contains_api': 203, 'type_owns_method': 68, 'impl_connects_type_trait': 11, 'api_returns_type': 68, 'api_accepts_type': 2}`
- Selected target: `api::bytes::buf::uninit_slice::UninitSlice::from_raw_parts_mut` → `bytes::buf::uninit_slice::UninitSlice::from_raw_parts_mut`
- Signature: `fn from_raw_parts_mut(*mut u8, usize) -> &'a mut UninitSlice`
- Safety doc present: `True`
- Target flags: `is_unsafe=True`, `contains_unsafe_block=False`, args=`['*mut u8', 'usize']`, return=`&'a mut UninitSlice`
- Context chars: `6162`
- Core sections complete: `True`
- Related/similar/idioms: `24` / `4` / `5`

### crossbeam-queue
- Crate import: `crossbeam_queue`
- APIs/types/traits/impls: `17` / `2` / `0` / `15`
- Unsafe candidates: `0`
- Graph nodes/edges: `20` / `38`
- Edge kinds: `{'module_contains_type': 2, 'module_contains_api': 17, 'type_owns_method': 17, 'api_returns_type': 2}`
- Context: `no_targets`

### globset
- Crate import: `globset`
- APIs/types/traits/impls: `26` / `8` / `2` / `29`
- Unsafe candidates: `0`
- Graph nodes/edges: `37` / `81`
- Edge kinds: `{'module_contains_type': 8, 'module_contains_api': 26, 'type_owns_method': 26, 'api_returns_type': 16, 'api_accepts_type': 5}`
- Context: `no_targets`

### url
- Crate import: `url`
- APIs/types/traits/impls: `58` / `10` / `4` / `61`
- Unsafe candidates: `0`
- Graph nodes/edges: `73` / `155`
- Edge kinds: `{'module_contains_type': 10, 'module_contains_api': 58, 'type_owns_method': 58, 'impl_connects_type_trait': 1, 'api_returns_type': 25, 'api_accepts_type': 2, 'api_accepts_trait': 1}`
- Context: `no_targets`

### camino
- Crate import: `camino`
- APIs/types/traits/impls: `84` / `14` / `2` / `216`
- Unsafe candidates: `14`
- Graph nodes/edges: `101` / `218`
- Edge kinds: `{'module_contains_type': 14, 'module_contains_api': 84, 'type_owns_method': 83, 'impl_connects_type_trait': 5, 'api_returns_type': 30, 'api_accepts_type': 2}`
- Selected target: `api::camino::Utf8Components::as_path` → `camino::Utf8Components::as_path`
- Signature: `fn as_path(&Self) -> &'a Utf8Path`
- Safety doc present: `False`
- Target flags: `is_unsafe=False`, `contains_unsafe_block=True`, args=`[]`, return=`&'a Utf8Path`
- Context chars: `5342`
- Core sections complete: `True`
- Related/similar/idioms: `24` / `4` / `5`

### tar
- Crate import: `tar`
- APIs/types/traits/impls: `168` / `16` / `6` / `26`
- Unsafe candidates: `18`
- Graph nodes/edges: `191` / `398`
- Edge kinds: `{'module_contains_type': 16, 'module_contains_api': 168, 'type_owns_method': 168, 'impl_connects_type_trait': 2, 'api_returns_type': 37, 'api_accepts_type': 7}`
- Selected target: `api::tar::header::GnuExtSparseHeader::new` → `tar::header::GnuExtSparseHeader::new`
- Signature: `fn new() -> GnuExtSparseHeader`
- Safety doc present: `False`
- Target flags: `is_unsafe=False`, `contains_unsafe_block=True`, args=`[]`, return=`GnuExtSparseHeader`
- Context chars: `5297`
- Core sections complete: `True`
- Related/similar/idioms: `24` / `4` / `5`

### csv-core
- Crate import: `csv_core`
- APIs/types/traits/impls: `40` / `11` / `0` / `43`
- Unsafe candidates: `0`
- Graph nodes/edges: `52` / `122`
- Edge kinds: `{'module_contains_type': 11, 'module_contains_api': 40, 'type_owns_method': 38, 'api_returns_type': 30, 'api_accepts_type': 3}`
- Context: `no_targets`

### http
- Crate import: `http`
- APIs/types/traits/impls: `195` / `44` / `14` / `296`
- Unsafe candidates: `6`
- Graph nodes/edges: `261` / `720`
- Edge kinds: `{'module_contains_type': 44, 'module_contains_api': 195, 'type_owns_method': 195, 'impl_connects_type_trait': 51, 'api_returns_type': 223, 'api_accepts_type': 12}`
- Selected target: `api::http::header::value::HeaderValue::from_maybe_shared_unchecked` → `http::header::value::HeaderValue::from_maybe_shared_unchecked`
- Signature: `fn from_maybe_shared_unchecked(T) -> HeaderValue`
- Safety doc present: `True`
- Target flags: `is_unsafe=True`, `contains_unsafe_block=False`, args=`['T']`, return=`HeaderValue`
- Context chars: `6004`
- Core sections complete: `True`
- Related/similar/idioms: `24` / `4` / `5`

## Observations

- End-to-end Phase 1/2 succeeds for 6 of 10 tested crates; 4 crates are clean `no_targets` under the current unsafe-focused scope.
- All generated contexts preserve the required sections after the related-API budgeting fix.
- Target prioritization improved for explicit safety/raw-pointer cases: `s3-audit-fixture` and `bytes` score higher and select appropriate targets.
- Some selected targets (`camino`, `tar`) are safe public APIs with internal unsafe blocks. This is expected: the active target scope is exactly unsafe APIs plus safe APIs that contain unsafe blocks.
- Safe-only crates (`crossbeam-queue`, `globset`, `url`, `csv-core`) now produce a clean no-target outcome instead of a traceback.

## Recommended Next Optimizations

1. Add a batch runner command that records `ok/no_targets/failed` as JSON instead of relying on shell logs.
2. Add source evidence snippets for `contains_unsafe_block` so RAG contexts can explain why a safe-signature API is considered unsafe-focused.
3. Continue tuning ranking with ordinary signals such as FFI, safety docs, raw pointer arguments, unchecked names, panic facts, and graph degree, without adding target classes.
