from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

NodeKind = Literal["api", "type", "trait", "module", "cluster"]
EdgeKind = Literal[
    "module_contains_api",
    "type_owns_method",
    "api_returns_type",
    "api_accepts_type",
    "trait_has_method",
    "impl_connects_type_trait",
    "semantic_similar",
    "same_cluster",
    "llm_relation",
]


@dataclass(frozen=True)
class ApiDocument:
    doc_id: str
    api_id: str
    text: str
    metadata: dict[str, str | int | float | bool]


@dataclass(frozen=True)
class IdiomDocument:
    doc_id: str
    text: str
    metadata: dict[str, str | int | float | bool]


@dataclass(frozen=True)
class UnsafeTarget:
    api_id: str
    path: str
    score: float
    reason: str


@dataclass(frozen=True)
class RetrievalContext:
    target: UnsafeTarget
    markdown: str
    api_ids: list[str] = field(default_factory=list)
