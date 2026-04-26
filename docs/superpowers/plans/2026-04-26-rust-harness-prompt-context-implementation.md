# Rust Harness Prompt + Context Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the new fact-grounded Rust harness prompt contract and the redesigned Phase 2 markdown context schema without changing the downstream Phase 3 orchestration flow.

**Architecture:** Keep the existing `phase2 retrieve -> phase3 harness-prompt -> harness-write` pipeline and bundle JSON shape intact. Replace the prompt text in `rag/seraph_rag/harness_prompt.py`, restructure the markdown emitted by `rag/seraph_rag/retrieve.py`, and keep the first implementation of `Known Reachable Paths` line-oriented so existing downstream consumers can be upgraded safely.

**Tech Stack:** Python (`rag/seraph_rag`), pytest, Rust (`crates/s3-coverage`), markdown context rendering

---

## File Structure

- Modify: `rag/seraph_rag/harness_prompt.py`
  - Replace the current rules-heavy prompt text with the new fact-grounded system/user prompt contract.
- Modify: `rag/seraph_rag/retrieve.py`
  - Render the new markdown schema: `Crate Facts`, `Known Reachable Paths`, `Compile-Time Facts`, `Variant Opportunities`, `Similar API Usage`.
  - Keep `Known Reachable Paths` line-oriented in the first implementation so downstream parsers can still extract setup-related API candidates.
- Modify: `rag/tests/test_harness_prompt.py`
  - Lock the new prompt contract with direct assertions on prompt text.
- Modify: `rag/tests/test_retrieve.py`
  - Lock the new section layout and compile-time-facts block.
- Modify: `rag/tests/test_target_and_related_ranking.py`
  - Update setup-chain expectations to the new `Known Reachable Paths` section name.
- Modify: `rag/tests/test_retrieval_quality.py`
  - Update section slicing to the new similar-usage layout and ensure budget behavior still holds.
- Modify: `rag/tests/test_context_section_budget.py`
  - Update the “must survive budget compression” section list.
- Modify: `rag/tests/test_cli.py`
  - Keep retrieve CLI expectations aligned with the new markdown schema.
- Modify: `crates/s3-coverage/src/lib.rs`
  - Accept `Known Reachable Paths` as a setup-bearing section when extracting related API candidates from markdown context.
- Modify: `crates/s3-coverage/tests/phase3_coverage.rs`
  - Prove that coverage still counts setup-path APIs from the renamed section.
- Modify: `docs/architecture/phase3-harness-generation.md`
  - Document the new prompt/context contract for the active Phase 3 path.

### Task 1: Replace the Prompt Contract in `harness_prompt.py`

**Files:**
- Modify: `rag/tests/test_harness_prompt.py`
- Modify: `rag/seraph_rag/harness_prompt.py`

- [ ] **Step 1: Write the failing prompt-contract test**

```python
def test_build_prompt_bundle_uses_fact_grounded_prompt_contract():
    context = """# SERAPH Rust Harness Context

## Crate Facts
- crate_name: fixture
- crate_import_name: fixture
- target_crate_kind: library

## Target API
- api_id: api::fixture::Buffer::get_unchecked
- path: fixture::Buffer::get_unchecked
- signature: unsafe fn get_unchecked(&self, index: usize) -> u8

## Known Reachable Paths
- api::fixture::Buffer::new: fixture::Buffer::new — fn new() -> Buffer [goal=construct_owner basis=producer_chain depth=0]

## Related APIs
- api::fixture::Buffer::len: fixture::Buffer::len — fn len(&self) -> usize [roles=accessor relation=graph_neighbor]

## Compile-Time Facts
### Exact Import Paths
- type::fixture::Buffer => fixture::Buffer [kind=type]
"""

    bundle = build_prompt_bundle(context, variants=2)

    assert "SERAPH's Rust fuzz harness generation expert" in bundle["system_prompt"]
    assert "`Known Reachable Paths` are validated, fact-grounded examples" in bundle["system_prompt"]
    assert "`Related APIs` are the main building blocks" in bundle["system_prompt"]
    assert "You may design your own setup and call sequence using the facts in the context." in bundle["user_prompt"]
    assert "Prefer `Related APIs` as the main construction pool." in bundle["user_prompt"]
    assert "Use `Known Reachable Paths` as validated anchors when helpful" in bundle["user_prompt"]
```

- [ ] **Step 2: Run the prompt tests and verify they fail on the old prompt wording**

Run: `PYTHONPATH=rag pytest rag/tests/test_harness_prompt.py -q`
Expected: FAIL because `system_prompt` still starts with `You are a Rust fuzz harness expert.` and `user_prompt` still says `Generate {variants} harness variants from the SERAPH RAG context below.`

- [ ] **Step 3: Replace `_build_system_prompt()` and the `user_prompt` template with the redesigned contract**

```python
def _build_system_prompt(style: str) -> str:
    if style != DEFAULT_HARNESS_STYLE:
        raise ValueError("unsupported harness style: {}".format(style))
    return """You are SERAPH's Rust fuzz harness generation expert.

Your job is to generate AFL++-friendly Rust harness variants from a structured SERAPH context.

Primary goals, in order:
1. Real target reachability: every variant must truly call the Target API.
2. Factual correctness: use only crate APIs, types, traits, enum variants, module paths, and setup facts explicitly present in the context.
3. Rust compile realism: treat Compile-Time Facts as authoritative and keep the code compile-fixable.
4. Diversity: when the context supports it, vary setup, input shaping, boundary selection, or state progression across variants.

Output contract:
- Output only Rust code blocks, one harness variant per code block.
- Generate a normal Rust binary with `fn main()`.
- Read fuzz bytes from stdin or an optional file path argument using only the Rust standard library.
- Call the Target API in every variant.
- Preserve exact `SERAPH_STEP_ENTER:<step_no>:<api_id>` and `SERAPH_STEP_OK:<step_no>:<api_id>` markers around each successful target call.
- Use the crate import name specified in the context.
- Do not use `target_lib` as a crate name.

How to use the context:
- `Known Reachable Paths` are validated, fact-grounded examples of how the target can be reached. They are strong hints, not the only allowed sequence.
- `Related APIs` are the main building blocks for designing the harness.
- `Compile-Time Facts` are hard constraints, not suggestions.
- `Variant Opportunities` indicate where diversity is likely to be meaningful.
- `Rust Idioms` are safety and ownership guidance.

Do not hallucinate:
- Do not invent constructors, helper methods, modules, trait impls, enum variants, imports, ownership transitions, or preconditions not supported by the context.
- Do not use crate APIs that are not explicitly named in the context.
- If a setup step is not factually supported, do not guess; prefer a smaller conservative harness or early return.
""".strip()


def build_prompt_bundle(
    rag_context: str,
    variants: int = 3,
    style: str = DEFAULT_HARNESS_STYLE,
) -> Dict[str, Any]:
    normalized_style = _normalize_style(style)
    target_api_id = extract_target_api_id(rag_context)
    user_prompt = """Generate {variants} Rust harness variants for the SERAPH target below.

Requirements:
- Every variant must call the Target API.
- You may design your own setup and call sequence using the facts in the context.
- Prefer `Related APIs` as the main construction pool.
- Use `Known Reachable Paths` as validated anchors when helpful, but do not copy them mechanically.
- Treat `Compile-Time Facts` as authoritative.
- Make variants meaningfully different when the context supports it. Prefer diversity in setup path, input shaping, boundary selection, state progression, or recoverable error exploration.
- If a more ambitious path is not factually supported, choose a smaller conservative path instead of guessing.
- Keep all logic inside a normal Rust binary `fn main()`.
- Return only Rust code blocks, one code block per variant, with no prose outside the code blocks.

Target API id: {target_api_id}
Harness style: {style}
Requested variants: {variants}

{rag_context}
""".format(
        variants=variants,
        target_api_id=target_api_id,
        style=normalized_style,
        rag_context=rag_context.rstrip(),
    )
```

- [ ] **Step 4: Run the prompt tests again and verify they pass**

Run: `PYTHONPATH=rag pytest rag/tests/test_harness_prompt.py -q`
Expected: PASS

- [ ] **Step 5: Commit the prompt-contract slice**

```bash
git add rag/tests/test_harness_prompt.py rag/seraph_rag/harness_prompt.py
git commit -m "feat: adopt fact-grounded rust harness prompt contract"
```

### Task 2: Reshape Retrieved Markdown Around the New Section Layout

**Files:**
- Modify: `rag/tests/test_retrieve.py`
- Modify: `rag/tests/test_target_and_related_ranking.py`
- Modify: `rag/tests/test_cli.py`
- Modify: `rag/seraph_rag/retrieve.py`

- [ ] **Step 1: Add failing tests for the new context sections**

```python
def test_render_context_markdown_uses_redesigned_sections():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(
        knowledge,
        graph,
        target,
        idioms=["Handle Result with early return."],
    )

    assert "# SERAPH Rust Harness Context" in markdown
    assert "## Crate Facts" in markdown
    assert "## Target API" in markdown
    assert "## Known Reachable Paths" in markdown
    assert "## Related APIs" in markdown
    assert "## Compile-Time Facts" in markdown
    assert "## Similar API Usage" in markdown
    assert "## Rust Idioms" in markdown
    assert "## Generation Rules" not in markdown
```

```python
def test_render_context_markdown_surfaces_known_reachable_paths_by_default():
    knowledge = setup_chain_knowledge()
    graph = build_graph(knowledge)
    target = next(
        target
        for target in rank_unsafe_targets(graph)
        if target.api_id == "api::chain_fixture::DecodeContext::decode_video"
    )

    markdown = render_context_markdown(knowledge, graph, target)

    reachable = markdown.split("## Known Reachable Paths", 1)[1].split("## Related APIs", 1)[0]
    assert "InputCodecParameters::new_decoder" in reachable
    assert "InputStream::codecpar" in reachable
    assert "InputFormatContext::read_frame" in reachable
```

```python
def test_cli_graph_targets_and_retrieve(tmp_path, capsys):
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    context_path = tmp_path / "context.md"
    index_knowledge(load_knowledge(FIXTURE), vectordb)

    main(["graph", "--knowledge", str(FIXTURE), "--graph", str(graph_path)])
    main([
        "retrieve",
        "--knowledge",
        str(FIXTURE),
        "--graph",
        str(graph_path),
        "--vectordb",
        str(vectordb),
        "--output",
        str(context_path),
    ])

    context = context_path.read_text(encoding="utf-8")
    assert "## Known Reachable Paths" in context
    assert "## Similar API Usage" in context
    assert "## Generation Rules" not in context
```

- [ ] **Step 2: Run the retrieve-focused Python tests and capture the section-name failures**

Run: `PYTHONPATH=rag pytest rag/tests/test_retrieve.py rag/tests/test_target_and_related_ranking.py rag/tests/test_cli.py -q`
Expected: FAIL because the renderer still emits `# SERAPH RAG Harness Context`, `## Required Setup APIs`, `## Semantically Similar API Docs`, and `## Generation Rules`.

- [ ] **Step 3: Restructure `render_context_markdown()` around the new top-level sections**

```python
lines = [
    "# SERAPH Rust Harness Context",
    "",
    "## Crate Facts",
    "- crate_name: {}".format(knowledge["crate_meta"].get("crate_name", crate_import_name)),
    "- crate_import_name: {}".format(crate_import_name),
    "- target_crate_kind: library",
    "",
    "## Target API",
    "- api_id: {}".format(target.api_id),
    "- path: {}".format(_api_display_path(target_api)),
    "- signature: {}".format(_api_signature_display(target_api)),
    "- target_kind: {}".format(target_api.get("api_kind", "")),
    "- owner_type: {}".format(_type_display_name(type_index.get(target_api.get("owner_type_id")), "")),
    "- owner_trait: {}".format(_trait_display_name(trait_index.get(target_api.get("owner_trait_id")), graph, "")),
    "- receiver: {}".format(target_api.get("receiver", "")),
    "- return_shape: {}".format(target_api.get("return_type", "")),
    "- safety_summary: {}".format((target_api.get("doc_sections") or {}).get("safety", "")),
    "- errors_summary: {}".format((target_api.get("doc_sections") or {}).get("errors", "")),
    "- panics_summary: {}".format((target_api.get("doc_sections") or {}).get("panics", "")),
    "",
    "## Known Reachable Paths",
]
for entry in setup_entries[:max_setup_apis]:
    api = entry["api"]
    lines.append(
        "- {}: {} — {} [goal=reach_target basis=producer_chain produces={} depth={}]".format(
            api["api_id"],
            _api_display_path(api),
            _api_signature_display(api),
            ", ".join(entry["produced_types"]),
            entry["upstream_depth"],
        )
    )
```

```python
if compile_hints["imports"] or compile_hints["traits"] or compile_hints["trait_methods"] or compile_hints["enums"]:
    lines.extend(["", "## Compile-Time Facts"])
if compile_hints["imports"]:
    lines.extend(["### Exact Import Paths"])
    for item in compile_hints["imports"]:
        lines.append("- {} => {} [kind={}]".format(item["id"], item["path"], item["kind"]))
if compile_hints["traits"]:
    lines.extend(["", "### Required Traits"])
    for item in compile_hints["traits"]:
        lines.append(
            "- {}: required_methods={}; provided_methods={}".format(
                item["path"],
                ", ".join(item["required_methods"]) or "(none)",
                ", ".join(item["provided_methods"]) or "(none)",
            )
        )
```

```python
lines.extend(["", "## Similar API Usage"])
for doc in _filter_similar_docs(
    similar_api_docs,
    target.api_id,
    target_path,
    excluded_api_ids=excluded_api_ids,
    excluded_paths=excluded_paths,
)[:max_similar_docs]:
    lines.append("- {}".format(_one_line(doc)))
```

- [ ] **Step 4: Re-run the retrieve-focused tests**

Run: `PYTHONPATH=rag pytest rag/tests/test_retrieve.py rag/tests/test_target_and_related_ranking.py rag/tests/test_cli.py -q`
Expected: PASS

- [ ] **Step 5: Commit the section-layout slice**

```bash
git add rag/tests/test_retrieve.py rag/tests/test_target_and_related_ranking.py rag/tests/test_cli.py rag/seraph_rag/retrieve.py
git commit -m "feat: render redesigned rust harness context sections"
```

### Task 3: Add Variant Opportunities and Update Budget Preservation

**Files:**
- Modify: `rag/tests/test_context_section_budget.py`
- Modify: `rag/tests/test_retrieval_quality.py`
- Modify: `rag/tests/test_retrieve.py`
- Modify: `rag/seraph_rag/retrieve.py`

- [ ] **Step 1: Add failing tests for `Variant Opportunities` and the new budget-critical sections**

```python
def test_render_context_markdown_surfaces_variant_opportunities():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    assert "## Variant Opportunities" in markdown
    assert "### Setup Choices" in markdown
    assert "### Input Shaping Choices" in markdown
```

```python
def test_context_budget_preserves_all_core_sections(tmp_path):
    knowledge = load_knowledge(FIXTURE)
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    index_knowledge(knowledge, vectordb)
    write_graph(build_graph(knowledge), graph_path)

    markdown = render_context_from_stores(
        knowledge,
        vectordb,
        graph_path,
        max_context_chars=1800,
    )

    assert len(markdown) <= 1800
    for section in [
        "## Target API",
        "## Known Reachable Paths",
        "## Related APIs",
        "## Compile-Time Facts",
        "## Variant Opportunities",
    ]:
        assert section in markdown
```

```python
def test_render_context_from_stores_dedupes_target_and_honors_budget(tmp_path):
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    knowledge = load_knowledge(FIXTURE)
    index_knowledge(knowledge, vectordb)
    write_graph(build_graph(knowledge), graph_path)

    markdown = render_context_from_stores(
        knowledge,
        vectordb,
        graph_path,
        max_context_chars=900,
    )

    similar_section = markdown.split("## Similar API Usage", 1)[1].split("## Rust Idioms", 1)[0]
    assert "fixture_crate::Buffer::get_unchecked" not in similar_section
    assert len(markdown) <= 900
```

- [ ] **Step 2: Run the budget-and-variation tests and verify they fail**

Run: `PYTHONPATH=rag pytest rag/tests/test_retrieve.py rag/tests/test_context_section_budget.py rag/tests/test_retrieval_quality.py -q`
Expected: FAIL because `Variant Opportunities` does not exist and `_fit_context_budget()` still prioritizes the old section names.

- [ ] **Step 3: Add factual `Variant Opportunities` helpers and update compression order**

```python
def _collect_variant_opportunities(
    target_api: Dict[str, Any],
    setup_entries: List[Dict[str, Any]],
    related_apis: List[Dict[str, Any]],
) -> Dict[str, List[str]]:
    setup_choices = [
        "{} via {}".format(_api_display_path(entry["api"]), ", ".join(entry["produced_types"]))
        for entry in setup_entries[:3]
    ]
    input_choices: List[str] = []
    for arg_type in target_api.get("arg_types", []):
        if "[u8]" in arg_type or "Vec<u8>" in arg_type or "str" in arg_type:
            input_choices.extend([
                "feed raw stdin bytes directly into byte-oriented arguments",
                "use a bounded prefix of the fuzz input for shorter boundary-oriented calls",
            ])
        if "usize" in arg_type or "u32" in arg_type or "u64" in arg_type:
            input_choices.extend([
                "derive small numeric boundary values from the fuzz input",
                "clamp numeric values to public-length or capacity bounds before the target call",
            ])
    state_choices = [
        "call the target immediately after minimal setup",
    ]
    if any(api.get("receiver") in {"&mut Self", "&mut self"} for api in related_apis):
        state_choices.append("apply one public mutator before calling the target")
    boundary_choices = [
        "exercise empty or one-byte inputs when the target accepts externally supplied bytes",
        "prefer documented recoverable boundaries over invented invalid states",
    ]
    return {
        "setup": list(dict.fromkeys(setup_choices))[:3],
        "input": list(dict.fromkeys(input_choices))[:3],
        "state": list(dict.fromkeys(state_choices))[:3],
        "boundary": list(dict.fromkeys(boundary_choices))[:3],
    }
```

```python
compression_order = [
    "## Similar API Usage",
    "## Rust Idioms",
    "## Variant Opportunities",
    "## Related APIs",
    "## Compile-Time Facts",
    "## Known Reachable Paths",
    "## Target API",
    "## Crate Facts",
]
```

```python
variant_opportunities = _collect_variant_opportunities(target_api, setup_entries[:max_setup_apis], related_apis)
lines.extend(["", "## Variant Opportunities", "### Setup Choices"])
for item in variant_opportunities["setup"]:
    lines.append("- {}".format(item))
lines.extend(["", "### Input Shaping Choices"])
for item in variant_opportunities["input"]:
    lines.append("- {}".format(item))
lines.extend(["", "### State Progression Choices"])
for item in variant_opportunities["state"]:
    lines.append("- {}".format(item))
lines.extend(["", "### Boundary Choices"])
for item in variant_opportunities["boundary"]:
    lines.append("- {}".format(item))
```

- [ ] **Step 4: Re-run the budget-and-variation tests**

Run: `PYTHONPATH=rag pytest rag/tests/test_retrieve.py rag/tests/test_context_section_budget.py rag/tests/test_retrieval_quality.py -q`
Expected: PASS

- [ ] **Step 5: Commit the variation slice**

```bash
git add rag/tests/test_context_section_budget.py rag/tests/test_retrieval_quality.py rag/tests/test_retrieve.py rag/seraph_rag/retrieve.py
git commit -m "feat: add variant opportunities to rust harness context"
```

### Task 4: Keep Coverage Extraction Compatible With the Renamed Setup Section

**Files:**
- Modify: `crates/s3-coverage/tests/phase3_coverage.rs`
- Modify: `crates/s3-coverage/src/lib.rs`

- [ ] **Step 1: Add a failing coverage test for `Known Reachable Paths`**

```rust
#[test]
fn write_coverage_counts_known_reachable_paths_as_related_candidates() {
    let root = temp_dir("coverage-known-reachable-paths");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_006.md");
    let compile_index_path = root.join("reports/compile_006_index.json");
    let fuzz_dir = root.join("fuzz");
    let reports_dir = root.join("reports");

    fs::create_dir_all(&fuzz_dir).expect("create fuzz dir");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let harness_path = fuzz_dir.join("harness_006_01.rs");
    fs::write(
        &context_path,
        "\
# SERAPH Rust Harness Context

## Target API
- api_id: api::fixture::Buffer::get_unchecked

## Known Reachable Paths
- api::fixture::Buffer::new: fixture::Buffer::new — fn new() -> Buffer [goal=construct_owner basis=producer_chain depth=0]
- api::fixture::Builder::with_capacity: fixture::Builder::with_capacity — fn with_capacity(usize) -> Builder [goal=construct_owner basis=producer_chain depth=1]

## Related APIs
- api::fixture::Buffer::len: fixture::Buffer::len — fn len(&Self) -> usize [roles=accessor relation=graph_neighbor]
",
    )
    .expect("write context");

    fs::write(
        &harness_path,
        "\
use fixture::Buffer;
use fixture::Builder;

fn main() {
    let mut buffer = Buffer::new();
    let builder = Builder::with_capacity(16);
    let _ = buffer.len();
    println!(\"SERAPH_STEP_ENTER:1:api::fixture::Buffer::get_unchecked\");
    let _ = buffer.get_unchecked(0);
    println!(\"SERAPH_STEP_OK:1:api::fixture::Buffer::get_unchecked\");
    let _ = builder;
}
",
    )
    .expect("write harness");

    fs::write(
        &compile_index_path,
        format!(
            "{{\"round\":6,\"status\":\"ok\",\"reports\":[{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"ok\",\"exit_code\":0}}]}}\n",
            harness_path.display(),
            reports_dir.join("compile_006_01.json").display(),
        ),
    )
    .expect("write compile index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        None,
    )
    .expect("update coverage");

    assert!(update.state.related_total_api_ids.contains(&ApiId::from("api::fixture::Buffer::new")));
    assert!(update.state.related_total_api_ids.contains(&ApiId::from("api::fixture::Builder::with_capacity")));
}
```

- [ ] **Step 2: Run the Rust coverage test and verify it fails**

Run: `cargo test -p s3-coverage write_coverage_counts_known_reachable_paths_as_related_candidates -- --exact`
Expected: FAIL because `extract_context_api_set()` currently only collects related candidates from `Required Setup APIs` and `Related APIs`.

- [ ] **Step 3: Teach `extract_context_api_set()` to accept both the new and legacy setup-bearing section names**

```rust
fn extract_context_api_set(context: &str) -> Result<ContextApiSet, String> {
    let mut target_api_id = None;
    let mut related_candidates = Vec::new();
    let mut section = "";

    for line in context.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("## ") {
            section = value.trim();
            continue;
        }
        if section == "Target API" {
            if let Some(value) = trimmed.strip_prefix("- api_id:") {
                let value = value.trim();
                if !value.is_empty() {
                    target_api_id = Some(ApiId::from(value));
                }
            }
            continue;
        }
        if section != "Known Reachable Paths"
            && section != "Required Setup APIs"
            && section != "Related APIs"
        {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("- ") else {
            continue;
        };
        let Some((api_id, detail)) = rest.split_once(": ") else {
            continue;
        };
        if !api_id.starts_with("api::") {
            continue;
        }
        if let Some(candidate) = build_static_api_candidate(ApiId::from(api_id), detail) {
            related_candidates.push(candidate);
        }
    }

    let Some(target_api_id) = target_api_id else {
        return Err("context missing target api_id".to_string());
    };
    related_candidates.retain(|candidate| candidate.api_id != target_api_id);
    Ok(ContextApiSet {
        target_api_id,
        related_candidates,
    })
}
```

- [ ] **Step 4: Re-run the Rust coverage test**

Run: `cargo test -p s3-coverage write_coverage_counts_known_reachable_paths_as_related_candidates -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the compatibility slice**

```bash
git add crates/s3-coverage/tests/phase3_coverage.rs crates/s3-coverage/src/lib.rs
git commit -m "fix: accept known reachable paths in phase3 coverage parsing"
```

### Task 5: Refresh User-Facing Docs and Run the Verification Sweep

**Files:**
- Modify: `docs/architecture/phase3-harness-generation.md`
- Test: `rag/tests/test_harness_prompt.py`
- Test: `rag/tests/test_retrieve.py`
- Test: `rag/tests/test_target_and_related_ranking.py`
- Test: `rag/tests/test_retrieval_quality.py`
- Test: `rag/tests/test_context_section_budget.py`
- Test: `rag/tests/test_cli.py`
- Test: `crates/s3-coverage/tests/phase3_coverage.rs`

- [ ] **Step 1: Update the Phase 3 architecture doc to describe the new prompt/context contract**

```md
The active Phase 3 prompt contract is now fact-grounded rather than rule-heavy.

The retrieved markdown context is organized around:

- `Crate Facts`
- `Target API`
- `Known Reachable Paths`
- `Related APIs`
- `Compile-Time Facts`
- `Variant Opportunities`
- `Similar API Usage`
- `Rust Idioms`

`Known Reachable Paths` are validated reachability hints, not mandatory scripts. The model may construct a different harness sequence as long as every API, import path, trait fact, and setup assumption is explicitly supported by the context.
```

- [ ] **Step 2: Run the full focused Python verification suite**

Run: `PYTHONPATH=rag pytest rag/tests/test_harness_prompt.py rag/tests/test_retrieve.py rag/tests/test_target_and_related_ranking.py rag/tests/test_retrieval_quality.py rag/tests/test_context_section_budget.py rag/tests/test_cli.py -q`
Expected: PASS

- [ ] **Step 3: Run the focused Rust coverage verification**

Run: `cargo test -p s3-coverage phase3_coverage -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit the doc-and-verification slice**

```bash
git add docs/architecture/phase3-harness-generation.md
git commit -m "docs: document redesigned rust harness prompt context"
```

## Self-Review

### Spec coverage

- New `system prompt`: covered by Task 1.
- New `user prompt`: covered by Task 1.
- New RAG context schema: covered by Tasks 2 and 3.
- `Known Reachable Paths` are hints, not scripts: enforced by Task 1 prompt text and Task 5 docs.
- `extract` vs `rag` section boundary: reflected by Tasks 2 and 3 because only `retrieve.py` is changed to select and render generation-ready views.
- Downstream compatibility for coverage: covered by Task 4.

### Placeholder scan

- No `TODO`, `TBD`, or “implement later” markers remain.
- Every task includes exact file paths, concrete code snippets, commands, expected failures, and commit messages.

### Type consistency

- The plan consistently uses `Known Reachable Paths`, `Compile-Time Facts`, `Variant Opportunities`, and `Similar API Usage`.
- The prompt contract consistently refers to the same section names as the context-rendering tasks.
- The coverage compatibility task explicitly supports both `Known Reachable Paths` and legacy `Required Setup APIs` to avoid rollout breakage.
