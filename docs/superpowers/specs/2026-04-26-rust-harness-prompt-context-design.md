# SERAPH Rust Harness Prompt + Context Redesign

## Background

The current executable Phase 3 path already works end to end:

1. `phase2 retrieve` renders a markdown RAG context
2. `phase3 harness-prompt` wraps that context into a prompt bundle
3. an LLM generates Rust harness variants
4. compile-check, fix-loop, smoke-run, and runtime diagnosis close the loop

However, the current first-round harness quality is still overly shaped by a long rules-heavy prompt. That creates three problems:

- too much Rust-specific guidance is repeated across system prompt, user prompt, and context
- the model is encouraged to behave like a rule follower rather than a fact-grounded harness designer
- setup information is present, but often not structured in the same way the model needs to reason about real Rust reachability, ownership, imports, traits, and variation

This design resets the Phase 3 first-round generation contract around a new split:

- RAG context provides the relevant facts and generation opportunities
- prompt text provides the decision contract and output contract
- the model remains free to design harness structure, but must stay inside the facts surfaced by SERAPH

## Goals

- Improve first-round Rust harness quality without turning the prompt into an ever-growing rules wall.
- Preserve model freedom to construct a harness from target and related APIs.
- Prevent hallucinated APIs, imports, trait impls, enum variants, and ownership transitions.
- Make diversity come from retrieved facts, not from ungrounded improvisation.
- Keep the design compatible with the existing Phase 2 markdown context and Phase 3 prompt bundle flow.

## Non-Goals

- Do not reintroduce scenario planning artifacts or ordered API plans.
- Do not require the model to copy a single fixed setup sequence.
- Do not move compile-fix or runtime-diagnosis logic into the initial generation prompt.
- Do not require a new non-markdown context format in this iteration.

## Design Principles

### 1. Fact-grounded freedom

The model should be allowed to design its own harness call sequence, but only from facts explicitly present in the context.

This means:

- the model may combine target and related APIs creatively
- the model may choose different setup and boundary strategies
- the model may use known reachable paths as anchors
- the model may not invent missing public APIs or implicit setup steps

### 2. Known reachable paths are hints, not scripts

The context should expose validated reachability hints, but they are not the only legal construction sequence.

This is especially important for Rust, where:

- a single target may be reachable through multiple public construction paths
- setup often depends on wrappers, `Deref`, trait implementors, or borrowed owners
- forcing one path reduces diversity and can hide real target-adjacent behavior

### 3. Compile-time facts must be authoritative

Rust module paths, trait method surfaces, enum exhaustiveness, and borrow-sensitive setup details are not soft semantic hints. They are hard compile-time facts.

The context must therefore elevate them into a dedicated section rather than leaving them scattered inside generic retrieved prose.

### 4. Diversity should be context-backed

Variant diversity should come from facts surfaced by retrieval:

- different public construction paths
- different argument production strategies
- different boundary conditions
- different pre-target state progression choices

This is preferable to asking for "3 variants" without telling the model where meaningful variation actually exists.

## New Prompt Contract

### System Prompt

```text
You are SERAPH's Rust fuzz harness generation expert.

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

Rust-specific rules:
- Handle recoverable `Result` and `Option` paths with early return, `match`, or `if let`; do not blindly unwrap recoverable failures.
- Keep unsafe blocks as narrow as possible and only use them when the context-supported preconditions justify them.
- Before creating an `&mut` borrow, compute indexes, lengths, and read-only bytes you still need.
- After creating an `&mut` borrow or mutable wrapper, do not read or immutably borrow the original owner again until that mutable borrow is no longer used.
- If implementing a trait shown in the context, copy the exact implementation-ready signatures from `Trait Method Signatures` and implement every required method.
- If matching an enum shown in the context, cover every listed variant unless it is marked non-exhaustive.

Forbidden:
- `libfuzzer_sys`, `#![no_main]`, `fuzz_target!`, `afl::fuzz!`
- `panic!`, `unreachable!`, `todo!`, `unimplemented!` as setup placeholders
- `MaybeUninit`, `mem::zeroed`, `transmute`, `Box::into_raw`, or similar tricks to fabricate missing state unless the context explicitly requires them

If perfect setup is unavailable, emit the smallest fact-grounded, compile-fixable harness that still attempts to reach the target through public APIs.
```

### User Prompt

```text
Generate {variants} Rust harness variants for the SERAPH target below.

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
```

## New RAG Context Schema

The Phase 2 retrieved context should remain markdown, but its sections should be restructured around first-round generation decisions rather than generic retrieval output.

Recommended shape:

```markdown
# SERAPH Rust Harness Context

## Crate Facts
- crate_name: ...
- crate_import_name: ...
- target_crate_kind: library

## Target API
- api_id: ...
- path: ...
- signature: ...
- target_kind: free_function | inherent_method | trait_method
- owner_type: ...
- owner_trait: ...
- receiver: ...
- return_shape: ...
- safety_summary: ...
- errors_summary: ...
- panics_summary: ...
- explicit_preconditions: ...

## Known Reachable Paths
- path_id: path_01
- goal: construct owner / produce arg / reach target
- summary: ...
- steps: api::... -> api::... -> api::...
- path_basis: producer_chain | deref_chain | trait_implementor | arg_producer

## Related APIs
- api_id: ...
- path: ...
- signature: ...
- roles: constructor | builder | mutator | accessor | validator | arg_producer | wrapper | companion_target
- usage_summary: ...
- relation_basis: graph_neighbor | semantic_similar | target_companion | setup_related

## Compile-Time Facts
### Exact Import Paths
- type::... => crate::... [kind=type]
- trait::... => crate::... [kind=trait]

### Required Traits
- crate::Trait: required_methods=...; provided_methods=...

### Trait Method Signatures
- crate::Trait::method: fn ... [required|provided]

### Enum Variants
- crate::Enum: A | B | C [non_exhaustive?]

### Borrow And Lifetime Constraints
- ...
- ...

## Variant Opportunities
### Setup Choices
- ...
- ...

### Input Shaping Choices
- ...
- ...

### State Progression Choices
- ...
- ...

### Boundary Choices
- ...
- ...

## Similar API Usage
- api::...: one-line usage hint
- api::...: one-line usage hint

## Rust Idioms
- ...
- ...
```

## Section Ownership: Extract vs RAG

The boundary should be:

- `extract` produces atomic static facts
- `rag` produces generation-ready views over those facts

### Extract-owned sections

#### `Crate Facts`

From Phase 1 facts:

- crate name
- crate import name
- crate kind

#### `Target API`

From Phase 1 facts, possibly with light formatting:

- stable target id
- canonical or preferred public path
- signature
- target kind
- owner type / owner trait
- receiver
- return shape
- doc-derived safety / errors / panics summaries
- explicit documented preconditions when extractable

These are target facts, not retrieval-time interpretations.

### RAG-owned sections

#### `Known Reachable Paths`

This is a retrieval-time derived view.

It should be assembled from:

- `api_returns_type`
- `api_accepts_type`
- `impl_connects_type_trait`
- `type_deref_target`
- producer backtracking over seed types relevant to the target

These paths are not mandatory execution scripts. They are validated fact-grounded anchors.

#### `Related APIs`

This is also retrieval-owned because relevance is target-specific.

It should merge and rank APIs from:

- graph-local neighbors
- setup-adjacent producers
- semantic similarity results
- target companion injection where useful

#### `Compile-Time Facts`

These facts originate from extract, but the selected subset should be surfaced by RAG.

RAG should decide which facts are truly relevant to the target and setup neighborhood, then render:

- exact import paths
- required traits
- implementation-ready trait method signatures
- enum variant coverage facts
- borrow/lifetime constraints inferred from receiver shape and setup relationships

This keeps the prompt focused and avoids dumping the full crate surface.

#### `Variant Opportunities`

This is retrieval-owned and should be derived from the current target neighborhood.

It should point out where the model can vary behavior without leaving the fact boundary:

- multiple reachable constructors or wrappers
- multiple argument production APIs
- visible boundary-sensitive APIs or documented preconditions
- public state transitions available before the target call

#### `Similar API Usage`

This comes from semantic retrieval and short usage summarization.

#### `Rust Idioms`

This comes from the existing idiom index and remains retrieval-owned.

## How Known Reachable Paths Should Be Computed

This section clarifies a likely implementation question.

`Known Reachable Paths` should not be invented by the model. They should be rendered by `phase2 retrieve` from graph-backed reachability facts.

The current code already computes most of the underlying information through `Required Setup APIs`:

- choose seed types from the target owner type, trait implementors, and accepted argument types
- locate producer APIs for those types
- expand produced types across `Deref` chains
- recurse backward through setup dependencies

What changes in this redesign is presentation, not the core source of truth:

- current output: flat `Required Setup APIs`
- new output: ordered and summarized `Known Reachable Paths`

The upgraded representation should preserve path structure when available, for example:

1. root producer API
2. intermediate wrapper or owner acquisition API
3. final receiver or argument production API
4. target API

But the model must not be told that this is the only legal sequence.

## Migration Guidance

### Minimal implementation slice

The smallest high-value implementation is:

1. update `retrieve.py` to render the new schema
2. rename the setup-facing section to `Known Reachable Paths`
3. keep `Related APIs`
4. add `Variant Opportunities`
5. replace the current system/user prompt text in `harness_prompt.py`

### What should stay the same

The following do not need to change in this iteration:

- prompt bundle JSON envelope fields
- harness writer behavior
- SERAPH marker validation
- compile-check / fix-loop / smoke-run / runtime-diagnose flow

## Acceptance Criteria

This redesign is successful if:

- first-round harnesses remain free to combine related APIs rather than following one forced setup script
- compile-time hallucinations decrease because hard Rust facts are isolated into a dedicated section
- generated variants differ in meaningful fact-backed ways
- contexts remain inspectable by humans and compatible with the existing Phase 3 prompt-bundle flow
- no new planning artifact is inserted between retrieval and generation

## Open Implementation Notes

- `Known Reachable Paths` should prefer public paths over private canonical paths wherever possible.
- `Trait Method Signatures` must remain implementation-ready, not declaration-shaped.
- `Borrow And Lifetime Constraints` should stay short and high-signal; this section is not a place to dump a long Rust tutorial.
- `Variant Opportunities` should remain factual. It should not speculate about paths unsupported by the current target neighborhood.
