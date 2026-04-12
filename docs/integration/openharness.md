# OpenHarness Integration Boundary

SERAPH uses OpenHarness as an external execution layer for single-stage skill invocations in Phase 3.

## Ownership Split

- `third_party/openharness/`: upstream OpenHarness source as a git submodule
- `integrations/openharness/`: SERAPH-specific integration notes, version policy, and invocation conventions
- `skills/`: SERAPH-owned skill materials, not upstream OpenHarness internals

## Current Policy

- OpenHarness is treated as an external dependency, not as the control plane of the repository
- SERAPH keeps orchestration ownership in `scripts/` and future Rust crates
- Direct edits inside the submodule should be avoided unless explicitly required

## Upstream

- Official site: `https://open-harness.dev`
- Upstream repository: `https://github.com/HKUDS/OpenHarness`
