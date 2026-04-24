from __future__ import annotations

from typing import Any, Dict, List, Optional, Set, Union

from seraph_rag.schema import ApiDocument

PRIMITIVE_RETURNS = {
    "bool",
    "char",
    "str",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "f32",
    "f64",
}

ScalarMetadata = Dict[str, Union[str, int, float, bool]]


def return_shape(return_type: Optional[str]) -> str:
    text = (return_type or "").strip()
    if text.startswith("Result"):
        return "Result"
    if text.startswith("Option"):
        return "Option"
    if text in {"Self", "self"}:
        return "Self"
    if text in {"", "()"}:
        return "Void"
    if text in PRIMITIVE_RETURNS:
        return "Primitive"
    return "Other"


def _risk_ids(risk_facts: Dict[str, Any], key: str) -> Set[str]:
    values = risk_facts.get(key, [])
    return {value for value in values if isinstance(value, str)}


def build_api_documents(knowledge: Dict[str, Any]) -> List[ApiDocument]:
    risk_facts = knowledge.get("risk_facts", {})
    unsafe_api_ids = _risk_ids(risk_facts, "unsafe_functions")
    ffi_api_ids = _risk_ids(risk_facts, "ffi_functions")
    panic_api_ids = _risk_ids(risk_facts, "panic_sites")
    documents = []
    for api in knowledge.get("apis", []):
        api_id = api["api_id"]
        sections = api.get("doc_sections", {}) or {}
        has_unsafe = (
            bool(api.get("is_unsafe"))
            or bool(api.get("contains_unsafe_block"))
            or api_id in unsafe_api_ids
        )
        parts = [
            "{}: {}".format(
                api.get("canonical_path", api_id),
                sections.get("summary") or api.get("docs", ""),
            ),
            "Signature: {}".format(api.get("signature") or api.get("signature_text", "")),
        ]
        if api.get("receiver"):
            parts.append("Receiver: {}".format(api["receiver"]))
        if api.get("where_clauses"):
            parts.append("Generic bounds: {}".format(", ".join(api["where_clauses"])))
        if sections.get("panics"):
            parts.append("Panics: {}".format(sections["panics"]))
        if sections.get("errors"):
            parts.append("Errors: {}".format(sections["errors"]))
        if sections.get("safety"):
            parts.append("Safety: {}".format(sections["safety"]))
        if has_unsafe:
            parts.append("WARNING: contains unsafe code block")
        metadata = {
            "api_id": api_id,
            "module_id": api.get("module_id") or api.get("public_anchor_module_id", ""),
            "owner_type_id": api.get("owner_type_id") or "",
            "path": api.get("canonical_path", api_id),
            "has_unsafe": has_unsafe,
            "has_ffi": api_id in ffi_api_ids,
            "has_panic_points": api_id in panic_api_ids,
            "receiver": api.get("receiver") or "",
            "return_shape": return_shape(api.get("return_type")),
            "risk_level": "high"
            if has_unsafe or api_id in ffi_api_ids
            else "medium"
            if api_id in panic_api_ids
            else "low",
        }
        documents.append(ApiDocument(api_doc_id(api_id), api_id, "\n".join(parts), metadata))
    return documents


def api_doc_id(api_id: str) -> str:
    return api_id if api_id.startswith("api::") else "api::{}".format(api_id)
