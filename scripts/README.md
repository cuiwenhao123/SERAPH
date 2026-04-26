# Scripts

This directory is reserved for orchestration entrypoints and developer helper scripts.

Current policy:

- scripts define execution entrypoints
- scripts do not own core business logic
- reusable logic should eventually live in Rust crates

Available helper entrypoints:

- `scripts/third_party_openai_compatible.py`: Phase 3 model adapter template for OpenAI-compatible third-party gateways. It reads SERAPH JSON prompt/fix/runtime request files and calls a `/chat/completions` style endpoint using only Python stdlib.
- `scripts/phase3_real_crate.py`: Phase 3 real-crate compile/smoke helper. It creates `_cargo_projects/<harness_stem>/`, wires a path dependency to the target crate from `workspace/crate_config.json`, then runs `cargo build` or `cargo run`.
- `scripts/real_crate_bug_record.py`: Renders a Markdown bug-record section from an existing real-crate Phase 3 workspace by reading `contexts/`, `reports/`, and `coverage.json`. It can print to stdout, write to a file, or append directly into `docs/architecture/rust-feature-bug-record.md`.
- `scripts/bootstrap-fuzz-target.sh`: Builds an AFL++-instrumented binary for a compile-successful Phase 3 harness workspace and launches `afl-fuzz` against it.

`bootstrap-fuzz-target.sh` expects:

- a Phase 3 workspace that already contains `_cargo_projects/<harness_stem>/Cargo.toml`
- a generated harness under `fuzz/`
- `cargo afl` and `afl-fuzz` available on `PATH` (or overridden with `SERAPH_CARGO_AFL_COMMAND` / `SERAPH_AFL_FUZZ_COMMAND`)

Example:

```bash
bash scripts/bootstrap-fuzz-target.sh \
  --workspace-dir /tmp/seraph-phase3-real-aflpp-localresp/arrayvec \
  --harness fuzz/harness_001_01.rs
```

Useful options:

- `--dry-run`: print the resolved `cargo afl build` and `afl-fuzz` commands
- `--build-only`: build the instrumented binary without launching `afl-fuzz`
- `--input-mode stdin`: fuzz through stdin instead of `@@` file mode

`real_crate_bug_record.py` example:

```bash
python3 scripts/real_crate_bug_record.py \
  --workspace-dir /tmp/seraph-bytes-bugfix-verify-rerun-20260426/uninit_new \
  --crate bytes \
  --phase 'Phase 3 real rerun'
```

Append directly into the architecture note:

```bash
python3 scripts/real_crate_bug_record.py \
  --workspace-dir /tmp/seraph-bytes-bugfix-verify-rerun-20260426/uninit_new \
  --crate bytes \
  --phase 'Phase 3 real rerun' \
  --append-doc docs/architecture/rust-feature-bug-record.md
```
