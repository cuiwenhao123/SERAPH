import json

from seraph_rag.compile_fixer_bundle import write_compile_fixer_bundles


def test_write_compile_fixer_bundles_for_failed_reports(tmp_path):
    context_path = tmp_path / "rag_target_009.md"
    reports_dir = tmp_path / "reports"
    fix_dir = tmp_path / "fixes"
    harness_path = tmp_path / "fuzz" / "harness_009_01.rs"
    report_path = reports_dir / "compile_009_01.json"
    index_path = reports_dir / "compile_009_index.json"
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    harness_path.parent.mkdir()
    harness_path.write_text("fn fuzz() {}\n", encoding="utf-8")
    reports_dir.mkdir()
    report_path.write_text(
        json.dumps({
            "harness": str(harness_path),
            "status": "failed",
            "exit_code": 1,
            "stderr": "error[E0308]: mismatched types",
            "stdout": "",
            "command": "rustc harness.rs",
        }),
        encoding="utf-8",
    )
    index_path.write_text(
        json.dumps({
            "round": 9,
            "status": "failed",
            "reports": [{
                "harness": str(harness_path),
                "report": str(report_path),
                "status": "failed",
                "exit_code": 1,
            }],
        }),
        encoding="utf-8",
    )

    bundles = write_compile_fixer_bundles(index_path, context_path, fix_dir)

    assert [path.name for path in bundles] == ["fix_request_009_01.json"]
    data = json.loads(bundles[0].read_text(encoding="utf-8"))
    assert data["version"] == "seraph.phase3.compile_fixer_request.v1"
    assert data["round"] == 9
    assert data["variant"] == 1
    assert data["target_api_id"] == "api::fixture::danger"
    assert data["harness_source"] == "fn fuzz() {}\n"
    assert "mismatched types" in data["diagnostics"]["stderr"]
    assert "Only use crate APIs explicitly present in the rag_context or harness_source." in data["rules"]
    assert "Do not invent new crate APIs to satisfy compiler errors." in data["rules"]
    assert (
        "Treat Known Reachable Paths as fact-grounded reachability/setup hints for opaque wrappers and "
        "borrowed handles."
        in data["rules"]
    )
    assert "Treat Type Trait Facts in the rag_context as authoritative for Copy/Clone and ownership assumptions." in data["rules"]
    assert (
        "If Output Initialization Facts provide a concrete initializer for a mutable output type, use that factual initializer instead of inventing `Default`, `mem::zeroed`, or `MaybeUninit`."
        in data["rules"]
    )
    assert (
        "If a callback or function-pointer type needs a raw-pointer pointee that is not surfaced in Exact Import Paths, preserve compiler inference with `*mut _`, `*const _`, or closure argument `_` annotations instead of guessing a private path or substituting `()`."
        in data["rules"]
    )
    assert (
        "Do not write `*mut _` or `*const _` directly in a named `fn` item's parameter types; use a closure, keep the placeholder at the call site, or make the helper generic over the pointee type."
        in data["rules"]
    )
    assert (
        "When fixing fuzz input parsing, do not keep direct positive-offset slices like `data[1..]` or `data[idx..idx + take]` unless a local bounds proof is explicit; prefer `get`, `split_first`, `split_at`, or an early return on short input."
        in data["rules"]
    )
    assert (
        "If you need lengths, indexes, or read-only slices from an owner or backing buffer, compute them before creating a mutable wrapper or `&mut` view from that owner."
        in data["rules"]
    )
    assert (
        "Once a mutable wrapper or `&mut` view is live, do not read, index, or immutably borrow the original owner again until that mutable wrapper is no longer used."
        in data["rules"]
    )
    assert (
        "After the target API succeeds, stop unless explicit cleanup is required."
        in data["rules"]
    )
    assert (
        "If the target API returns a borrowed view, wrapper, or handle tied to an owner or backing buffer, treat the target call itself as sufficient reachability and delete follow-up same-owner method calls unless explicit cleanup is required."
        in data["rules"]
    )
    assert (
        "If the target API docs or Boundary Choices describe a panic condition or exact input relation, satisfy that documented relation exactly before the target call."
        in data["rules"]
    )
    assert (
        "Do not approximate an exact precondition with `min`, truncation, or a shorter slice; if the documented relation cannot be met honestly, return early."
        in data["rules"]
    )
    assert (
        "If the rag_context surfaces an `unsafe fn`, wrap only the minimal required call in `unsafe` and shorten the live mutable borrow before reusing the owner."
        in data["rules"]
    )
    assert (
        "If a semantic guard reports that the real target call is missing, restore the exact target API named by target_api_id instead of a neighboring same-owner substitute."
        in data["rules"]
    )
    assert (
        "Do not keep a read-only slice or reference from the same backing owner alive across constructing a mutable target view; move the source to separate input/backing or end the immutable borrow first."
        in data["rules"]
    )
    assert (
        "If a surfaced setup API has a generic or opaque input and the rag_context does not show concrete bounds or accepted shapes, delete guessed tuple/composite owners and prefer an explicit concrete setup API already present."
        in data["rules"]
    )
    assert (
        "If the rag_context already provides a safe concrete producer for the target owner, replace unnecessary `unsafe` raw-pointer/raw-parts setup with that safe producer."
        in data["rules"]
    )
    assert (
        "Delete `unsafe` raw-pointer/raw-parts setup that was chosen only for diversity when a safe surfaced producer reaches the same owner and the required raw-parts invariants are not explicitly surfaced."
        in data["rules"]
    )
    assert (
        "If a safe surfaced producer already reaches the target owner, delete `unsafe` raw-pointer/raw-parts setup unless the rag_context explicitly provides the invariants needed to construct that raw-parts state honestly."
        in data["rules"]
    )
    assert (
        "If rustc reports that a trait-based or generic producer leaves an owner or collection type unconstrained, add an explicit concrete owner type annotation or replace it with another surfaced constructor or producer for the same owner whose concrete type can be written honestly."
        in data["rules"]
    )
    assert (
        "If rustc reports that a provided associated function on a trait cannot be called without a concrete implementor type, rewrite it using UFCS with the surfaced concrete owner type or replace it with an equivalent standard-library constructor that preserves the same honest shape."
        in data["rules"]
    )
    assert (
        "A left-hand-side type annotation alone is not enough to call a provided associated function through a trait path; if rustc still rejects the call, rewrite the constructor with UFCS or replace it with an equivalent concrete constructor."
        in data["rules"]
    )
    assert (
        "If rustc reports inconsistent bindings across `|` pattern alternatives on a returned `Cow` or other enum wrapper, delete that post-target destructuring or replace it with a whole-value method call; do not force different variant payload types into one shared binding."
        in data["rules"]
    )
    assert (
        "If rustc reports that a helper-returned wrapper or view does not implement the surfaced target trait, delete that wrapper bridge and call the trait method on the simpler concrete backing owner already in scope, unless the rag_context explicitly proves the wrapper implements the trait."
        in data["rules"]
    )
    assert (
        "Do not add `.clone()` to fix move errors unless the rag_context explicitly shows that the moved type implements `Clone`."
        in data["rules"]
    )
    assert "Prefer Required Setup APIs for opaque wrappers and borrowed handles." not in data["rules"]


def test_write_compile_fixer_bundles_skips_successful_reports(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    reports_dir = tmp_path / "reports"
    fix_dir = tmp_path / "fixes"
    report_path = reports_dir / "compile_001_01.json"
    index_path = reports_dir / "compile_001_index.json"
    context_path.write_text("## Target API\n- api_id: api::fixture::ok\n", encoding="utf-8")
    reports_dir.mkdir()
    report_path.write_text(json.dumps({"status": "ok"}), encoding="utf-8")
    index_path.write_text(
        json.dumps({"round": 1, "status": "ok", "reports": [{"report": str(report_path), "status": "ok"}]}),
        encoding="utf-8",
    )

    bundles = write_compile_fixer_bundles(index_path, context_path, fix_dir)

    assert bundles == []
    assert not fix_dir.exists()
