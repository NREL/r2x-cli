"""Stub Sienna parser used in integration tests."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


class SiennaConfig:
    """Minimal config placeholder."""

    def __init__(self, system_name: str | None = None, **kwargs: Any) -> None:
        self.system_name = system_name
        self.extra = kwargs


class SiennaParser:
    """Parser that returns canned JSON output."""

    def __init__(self, config: SiennaConfig | None = None, path: str | None = None, **_: Any) -> None:
        self.config = config
        self.path = path

    def build_system(self) -> str:
        return '{"system": "sienna", "status": "ok"}'
