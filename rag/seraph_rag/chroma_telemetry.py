from __future__ import annotations

from chromadb.telemetry.product import ProductTelemetryClient
from chromadb.telemetry.product.events import ProductTelemetryEvent
from overrides import override


class NoopProductTelemetryClient(ProductTelemetryClient):
    @override
    def capture(self, event: ProductTelemetryEvent) -> None:
        return None
