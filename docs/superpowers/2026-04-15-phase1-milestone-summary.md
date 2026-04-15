# Phase 1 Milestone Summary

## Scope

This document summarizes the current state of SERAPH Phase 1 after the latest
schema tightening, extractor hardening, and real-crate audit work.

It is intentionally short and operational:

- what is already landed
- what is currently treated as source of truth
- what has been verified against real crates
- what is still out of scope or still worth improving later

## Current Branch State

- Branch:
  - `feat/seraph-types-schema-skeleton`
- Current pushed milestone commits:
  - `8004166` `feat: add schema and extraction foundations`
  - `ee95526` `feat: tighten phase1 extraction schema`
  - `105d6db` `fix: harden s3-extract rustdoc loading and doc parsing`
  - `daf2f0f` `docs: record phase1 schema and extractor audit`

## Current Source Of Truth

For Phase 1, prefer these in order:

1. Current schema code:
   - `crates/seraph-types/src/knowledge.rs`
2. Current extractor implementation:
   - `crates/s3-extract/src/lib.rs`
3. Regression tests:
   - `crates/seraph-types/tests/schema_roundtrip.rs`
   - `crates/s3-extract/tests/minimal_extract.rs`
4. Active schema spec:
   - `docs/superpowers/specs/2026-04-15-knowledge-rs-v3-schema-spec.md`

Historical planning docs and the older v2 schema spec are preserved for audit
history only. They should not override the current code and tests.

## Landed Phase 1 Capabilities

The current extractor and schema now cover these fact-level surfaces:

- Cargo / crate metadata
  - package name
  - lib target name
  - import name
  - version / edition / rust-version
  - repository / description
  - manifest path / lib.rs path
  - default features
- Public surface structure
  - modules
  - types
  - APIs
  - public symbols
  - trait registry
  - trait impl registry
- Type-level facts
  - generic params
  - where clauses
  - `#[non_exhaustive]`
  - fields / variants
  - hidden-field / hidden-variant flags
- API-level facts
  - signature text
  - receiver
  - arg / return type text
  - return shape
  - `is_unsafe` / `is_async` / `is_const`
  - `has_body`
  - `contains_unsafe_block`
- Trait-level facts
  - required / provided methods
  - supertraits
  - associated types
  - associated consts
  - reverse usage edges from public bounds
- Trait-impl facts
  - trait ref / for-type text
  - associated type bindings
  - associated const bindings
  - where clauses
  - raw cfg attrs
  - impl header unsafety
- Doc facts
  - summary
  - panics / errors / safety / examples sections
  - example snippets extracted from rustdoc docs
- Risk facts
  - extern ABI APIs
  - repr facts
  - explicit Drop impl presence
  - explicit panic-like sites
  - borrowed return facts

## Hardening Already Completed

The latest extractor hardening specifically fixed these real issues:

- Workspace-member extraction:
  - `s3-extract` now chooses the package whose manifest matches the requested
    manifest path instead of blindly taking the first package from workspace
    metadata.
- Rustdoc JSON output lookup:
  - extraction now checks both the local crate target directory and the
    workspace-root target directory.
- Doc section parsing:
  - both `# Example` and `# Examples` are treated as example sections.
- Root summary parsing:
  - decorative badge / link-definition / `<br>` blocks are skipped so summary
    lands on the first actual prose paragraph.

## Real-Crate Audit Status

The current implementation has been exercised against these real crates:

- `hashbrown`
- `semver`
- `http`
- `moonfire-ffmpeg`
- `sqlx-core`

Observed status:

- `hashbrown`
  - public API / trait / impl extraction is working
  - examples are heavily populated
  - public-bound trait graph behavior is verified
- `semver`
  - public inherent associated consts are extracted
  - root summary and singular example heading are handled correctly
- `http`
  - associated constants are extracted into `symbols`
  - singular API example headings are handled correctly
- `moonfire-ffmpeg`
  - dyn-trait rendering is correct
  - internal unsafe blocks are detected
  - manual unsafe impl headers are detected
  - `extern_abi_apis = 0` is expected because internal FFI usage does not imply
    public extern ABI surface
- `sqlx-core`
  - workspace-member extraction now succeeds
  - rustdoc JSON is correctly read from the workspace target output location

## What Phase 1 Still Does Not Do

Phase 1 is still facts-only. It does not yet do semantic inference such as:

- capability clustering
- API intent inference
- recommendation / best-practice synthesis
- risk scoring
- coverage judgment
- semantic grouping of trait impl surface

Those belong to later stages.

## Known Boundaries

These behaviors are intentional boundaries, not current bugs:

- `trait_registry` is a public-API-relevant trait graph, not a full
  “all traits seen anywhere” dump.
- `trait_impl_registry` is the explicit public trait impl surface and stays
  separate from `trait_registry`.
- `ExampleInfo.code_ref` points to the owning item span, not the exact code
  block line range.
- `extern_abi_apis` describes public ABI surface, not all internal FFI usage.
- rustdoc-based field/variant extraction is constrained by whatever rustdoc JSON
  actually preserves.

## Remaining Nice-To-Have Cleanup

These are not blocking issues, but they are reasonable future polish items:

- strip decorative trailing lines like `<br><br>` from extracted doc sections
- improve example-to-API linking precision further
- add more real-crate audit samples once Phase 1 stabilizes
- start planning Phase 2 semantic modeling on top of the current fact layer

## Recommended Next Step

The recommended next step is:

1. Freeze the Phase 1 fact contract at the current `knowledge.rs` + extractor
   + regression-test state.
2. Avoid broad schema churn unless a new real-crate audit proves a concrete
   mismatch.
3. Begin the next implementation slice on top of this stable fact layer rather
   than reopening Phase 1 abstractions too aggressively.
