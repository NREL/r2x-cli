"""Stub Sienna upgrader for integration tests."""

from __future__ import annotations

from pathlib import Path
from typing import Any


class SiennaUpgrader:
    """Upgrader that simply records the provided path."""

    def __init__(self, path: Path | str, **_: Any) -> None:
        self.path = Path(path)

    def run(self) -> str:
        return f'{{"upgraded": "sienna", "path": "{self.path}"}}'
