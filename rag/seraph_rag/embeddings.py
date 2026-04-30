from __future__ import annotations

import json
import os
from dataclasses import dataclass
from http.client import IncompleteRead, RemoteDisconnected
from ssl import SSLEOFError
from typing import Any, Dict, List, Mapping, Optional, Protocol
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


class Embedder(Protocol):
    def encode(self, texts: List[str]) -> List[List[float]]:
        ...


@dataclass(frozen=True)
class OpenAICompatibleEmbedder:
    base_url: str
    model: str
    api_key: str = ""
    api_path: str = "/embeddings"
    timeout_seconds: float = 180.0
    max_retries: int = 1
    extra_headers: Optional[Mapping[str, str]] = None
    extra_body: Optional[Mapping[str, Any]] = None

    def encode(self, texts: List[str]) -> List[List[float]]:
        if not texts:
            return []
        request_body: Dict[str, Any] = {
            "model": self.model,
            "input": texts,
        }
        if self.extra_body:
            request_body.update(dict(self.extra_body))
        request = Request(
            "{}{}".format(self.base_url.rstrip("/"), _normalize_api_path(self.api_path)),
            data=json.dumps(request_body, ensure_ascii=False).encode("utf-8"),
            headers=_embedding_headers(self.api_key, self.extra_headers),
            method="POST",
        )
        payload = None
        for attempt in range(self.max_retries + 1):
            try:
                with urlopen(request, timeout=self.timeout_seconds) as response:
                    payload = json.loads(response.read().decode("utf-8"))
                break
            except Exception as exc:
                if not _is_retryable_embedding_error(exc) or attempt >= self.max_retries:
                    raise
        if payload is None:
            raise ValueError("embedding request did not produce a response payload")
        data = payload.get("data")
        if not isinstance(data, list):
            raise ValueError("embedding response missing data list")
        vectors: List[Optional[List[float]]] = [None] * len(texts)
        for expected_index, item in enumerate(sorted(data, key=_embedding_index)):
            if not isinstance(item, Mapping):
                raise ValueError("embedding response item must be an object")
            embedding = item.get("embedding")
            if not isinstance(embedding, list):
                raise ValueError("embedding response item missing embedding vector")
            if expected_index >= len(vectors):
                raise ValueError("embedding response returned too many vectors")
            vectors[expected_index] = [float(value) for value in embedding]
        if any(vector is None for vector in vectors):
            raise ValueError("embedding response missing vectors")
        return [vector for vector in vectors if vector is not None]


def embedding_backend_name() -> str:
    for env_name in ("SERAPH_EMBEDDING_BACKEND", "SERAPH_EMBEDDER"):
        value = os.environ.get(env_name)
        if value is not None:
            normalized = _normalize_backend_name(value)
            if normalized:
                return normalized
    return "openai_compatible"


def embedding_model_name() -> Optional[str]:
    value = os.environ.get("SERAPH_EMBEDDING_MODEL")
    if value is None:
        return None
    normalized = value.strip()
    return normalized or None


def embedder_backend_name() -> str:
    return embedding_backend_name()


def build_embedder() -> Embedder:
    backend = embedding_backend_name()
    if backend == "openai_compatible":
        base_url = _required_env("SERAPH_EMBEDDING_BASE_URL")
        model = _required_env("SERAPH_EMBEDDING_MODEL")
        return OpenAICompatibleEmbedder(
            base_url=base_url,
            api_key=_embedding_api_key(),
            model=model,
            api_path=os.environ.get("SERAPH_EMBEDDING_API_PATH", "/embeddings"),
            timeout_seconds=float(os.environ.get("SERAPH_EMBEDDING_TIMEOUT_SECONDS", "180")),
            max_retries=int(os.environ.get("SERAPH_EMBEDDING_MAX_RETRIES", "1")),
            extra_headers=_json_object_env("SERAPH_EMBEDDING_EXTRA_HEADERS"),
            extra_body=_json_object_env("SERAPH_EMBEDDING_EXTRA_BODY"),
        )
    raise ValueError(
        "unsupported SERAPH_EMBEDDING_BACKEND={!r}; currently supported: openai_compatible".format(
            backend
        )
    )


def _normalize_backend_name(value: str) -> str:
    normalized = value.strip().lower().replace("-", "_")
    if normalized in {"openai", "openai_compatible", "siliconflow"}:
        return "openai_compatible"
    return normalized


def _required_env(name: str) -> str:
    value = os.environ.get(name)
    if value:
        return value
    raise ValueError(f"missing required environment variable: {name}")


def _embedding_api_key() -> str:
    api_key = os.environ.get("SERAPH_EMBEDDING_API_KEY", "").strip()
    if _looks_like_duplicated_sk_token(api_key):
        raise ValueError(
            "SERAPH_EMBEDDING_API_KEY appears duplicated; "
            "it looks like the same 'sk-' token was pasted twice"
        )
    return api_key


def _json_object_env(name: str) -> Dict[str, Any]:
    raw = os.environ.get(name)
    if not raw:
        return {}
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be a JSON object")
    return value


def _normalize_api_path(api_path: str) -> str:
    if not api_path:
        return "/embeddings"
    return api_path if api_path.startswith("/") else f"/{api_path}"


def _looks_like_duplicated_sk_token(value: str) -> bool:
    if not value.startswith("sk-") or len(value) < 8:
        return False
    second_prefix = value.find("sk-", 3)
    if second_prefix <= 0:
        return False
    left = value[:second_prefix]
    right = value[second_prefix:]
    return left == right


def _embedding_headers(
    api_key: str,
    extra_headers: Optional[Mapping[str, str]],
) -> Dict[str, str]:
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
    }
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    if extra_headers:
        headers.update(dict(extra_headers))
    return headers


def _embedding_index(item: Any) -> int:
    if isinstance(item, Mapping):
        index = item.get("index")
        if isinstance(index, int):
            return index
    return 0


def _is_retryable_embedding_error(exc: Exception) -> bool:
    if isinstance(exc, (IncompleteRead, RemoteDisconnected, SSLEOFError)):
        return True
    if isinstance(exc, HTTPError):
        return exc.code == 429 or 500 <= exc.code <= 599
    if isinstance(exc, URLError):
        return True
    return False
