from __future__ import annotations

import hashlib
import math
import os
from dataclasses import dataclass
from typing import List, Protocol


class Embedder(Protocol):
    def encode(self, texts: List[str]) -> List[List[float]]:
        ...


@dataclass(frozen=True)
class HashingEmbedder:
    dimensions: int = 128

    def encode(self, texts: List[str]) -> List[List[float]]:
        return [self._encode_one(text) for text in texts]

    def _encode_one(self, text: str) -> List[float]:
        vector = [0.0] * self.dimensions
        for token in text.lower().replace("::", " ").split():
            digest = hashlib.sha256(token.encode("utf-8")).digest()
            index = int.from_bytes(digest[:4], "big") % self.dimensions
            sign = 1.0 if digest[4] % 2 == 0 else -1.0
            vector[index] += sign
        norm = math.sqrt(sum(value * value for value in vector)) or 1.0
        return [value / norm for value in vector]


def embedder_backend_name() -> str:
    return os.environ.get("SERAPH_EMBEDDER", "hashing").strip().lower() or "hashing"


def build_embedder() -> Embedder:
    backend = embedder_backend_name()
    if backend == "hashing":
        return HashingEmbedder()
    raise ValueError(
        "unsupported SERAPH_EMBEDDER={!r}; currently supported: hashing".format(backend)
    )
