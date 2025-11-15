"""Minimal DataStore implementation for integration tests."""

from __future__ import annotations

from pathlib import Path
from typing import Any


class DataStore:
    """Stub that only tracks the provided path."""

    def __init__(self, path: Path | str, **_: Any) -> None:
        self.path = Path(path)

    @classmethod
    def from_plugin_config(cls, _config: Any, path: Path | str) -> "DataStore":
        return cls(path)
