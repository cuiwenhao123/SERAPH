from __future__ import annotations

from typing import List

from seraph_rag.schema import IdiomDocument


def bundled_idioms() -> List[IdiomDocument]:
    rows = [
        (
            "idiom::unsafe::aliasing::001",
            "unsafe_semantics",
            "Raw pointers and references created inside unsafe code must still respect Rust aliasing and validity rules. Fuzz harnesses should construct valid inputs before crossing unsafe boundaries.",
        ),
        (
            "idiom::ownership::mut-ref::001",
            "ownership",
            "An &mut reference must be exclusive for its scope. Harnesses should avoid keeping shared references alive while calling mutating APIs.",
        ),
        (
            "idiom::error::result::001",
            "error_handling",
            "Recoverable Result errors should be handled with match or early return in fuzz harnesses. Do not unwrap fallible parsing or construction paths.",
        ),
        (
            "idiom::ffi::pointer::001",
            "ffi",
            "Pointer arguments crossing FFI or unsafe boundaries must be non-null and aligned unless documentation explicitly allows otherwise.",
        ),
        (
            "idiom::drop::cleanup::001",
            "drop",
            "Types with Drop may encode cleanup invariants. Harnesses should prefer public constructors and finalizers instead of fabricating internal states.",
        ),
        (
            "idiom::trait::unsafe::001",
            "trait_safety",
            "Unsafe trait implementations rely on documented invariants. Harnesses should use existing implementations rather than inventing unsound custom implementations.",
        ),
    ]
    return [
        IdiomDocument(doc_id, text, {"category": category, "source": "bundled"})
        for doc_id, category, text in rows
    ]
