from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Union


def load_knowledge(path: Union[str, Path]) -> Dict[str, Any]:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    for key in ("crate_meta", "modules", "types", "apis", "risk_facts"):
        if key not in data:
            raise ValueError("knowledge.json missing required key: {}".format(key))
    return data
