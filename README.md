# SERAPH

Semantic Rust Agent-driven Program Harness Synthesis.

This repository is the monorepo skeleton for the SERAPH project. It is currently in the `framework-only` stage: the repository layout, workspace boundaries, integration points, and runtime directories are in place, but business implementation code has not been started yet.

## Status

- Monorepo with Rust workspace boundaries
- Runtime workspace separated from source tree
- OpenHarness integrated as an external dependency via git submodule
- No production Rust or shell implementation code yet

## Repository Map

- `crates/`: Rust workspace members for core SERAPH components
- `scripts/`: orchestration entrypoints and developer helper scripts
- `skills/`: SERAPH-owned skill definitions and references
- `configs/`: long-lived configuration and environment layout
- `templates/`: schemas and reusable structural templates
- `workspace/`: runtime-only artifacts, logs, plans, fuzz workspaces
- `third_party/openharness/`: upstream OpenHarness submodule
- `integrations/openharness/`: SERAPH-specific OpenHarness integration notes
- `docs/`: architecture, specs, decisions, and research notes

## Key Design Docs

- `semantic_harness_agent_design_v5.md`
- `semantic_harness_agent_design_v4.md`
- `docs/integration/openharness.md`

## Current Boundary

This repository intentionally does not include implementation code yet. The current goal is to stabilize:

- repository structure
- ownership boundaries
- git layout
- workspace layout
- OpenHarness integration surface

## Next Recommended Implementation Order

1. `crates/seraph-types`
2. `crates/s3-extract`
3. `crates/s3-model`
4. `crates/s3-coverage`
5. `crates/s3-context`
6. `crates/seraph-cli`
