# SERAPH Repository Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a git-managed SERAPH monorepo skeleton with Rust workspace boundaries, runtime directories, placeholder files, and an OpenHarness submodule, without adding business implementation code.

**Architecture:** The repository will separate source ownership, runtime artifacts, integration notes, and third-party dependencies. Rust workspace manifests define package boundaries, while README and placeholder files document responsibilities before implementation begins.

**Tech Stack:** Git, Rust workspace manifests, Markdown documentation, shell entrypoint placeholders, OpenHarness git submodule

---

### Task 1: Initialize Repository Control Surface

**Files:**
- Create: `README.md`
- Create: `.gitignore`
- Create: `.gitattributes`
- Create: `.editorconfig`
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `clippy.toml`
- Create: `rustfmt.toml`
- Create: `LICENSE`

- [ ] **Step 1: Create root control files**

Add repository control files that define workspace boundaries, git ignore rules, text normalization, and repository status.

- [ ] **Step 2: Verify repository root layout**

Run: `ls -la`
Expected: root control files are present alongside the design documents.

### Task 2: Create Repository Documentation Skeleton

**Files:**
- Create: `docs/architecture/README.md`
- Create: `docs/decisions/README.md`
- Create: `docs/integration/openharness.md`
- Create: `docs/specs/README.md`
- Create: `docs/research/README.md`

- [ ] **Step 1: Create documentation entrypoints**

Add documentation placeholders that explain architecture, design decisions, OpenHarness integration boundaries, and research usage.

- [ ] **Step 2: Verify docs tree**

Run: `find docs -maxdepth 2 -type f | sort`
Expected: documentation README files and integration notes appear in the expected directories.

### Task 3: Create Workspace and Runtime Skeleton

**Files:**
- Create: `crates/seraph-types/Cargo.toml`
- Create: `crates/seraph-types/README.md`
- Create: `crates/s3-extract/Cargo.toml`
- Create: `crates/s3-extract/README.md`
- Create: `crates/s3-model/Cargo.toml`
- Create: `crates/s3-model/README.md`
- Create: `crates/s3-context/Cargo.toml`
- Create: `crates/s3-context/README.md`
- Create: `crates/s3-coverage/Cargo.toml`
- Create: `crates/s3-coverage/README.md`
- Create: `crates/seraph-cli/Cargo.toml`
- Create: `crates/seraph-cli/README.md`
- Create: `workspace/README.md`
- Create: `workspace/.gitignore`

- [ ] **Step 1: Create crate manifests and responsibility notes**

Add placeholder manifests and README files for each planned workspace member.

- [ ] **Step 2: Create runtime workspace placeholders**

Create README and ignore rules for runtime-only directories.

- [ ] **Step 3: Verify crate and workspace tree**

Run: `find crates workspace -maxdepth 2 | sort`
Expected: all planned workspace members and runtime directories exist.

### Task 4: Add Scripts, Skills, Configs, Templates, Examples, and Submodule

**Files:**
- Create: `scripts/README.md`
- Create: `scripts/run.sh`
- Create: `scripts/bootstrap-fuzz-target.sh`
- Create: `integrations/openharness/README.md`
- Create: `skills/**/README.md`
- Create: `configs/**/README.md`
- Create: `templates/**/README.md`
- Create: `examples/**/README.md`
- Create: `.github/workflows/README.md`
- Create: `.github/ISSUE_TEMPLATE/README.md`
- Create: `.gitmodules`
- Create: `third_party/openharness`

- [ ] **Step 1: Create ownership placeholders**

Add placeholder files for scripts, skills, configuration, templates, examples, integration notes, and GitHub metadata directories.

- [ ] **Step 2: Add OpenHarness as a submodule**

Run: `git submodule add https://github.com/HKUDS/OpenHarness.git third_party/openharness`
Expected: `.gitmodules` is created and `third_party/openharness` points to the upstream repository.

- [ ] **Step 3: Verify repository skeleton**

Run: `find . -maxdepth 3 | sort`
Expected: the monorepo skeleton, runtime placeholders, and OpenHarness submodule are all visible.
