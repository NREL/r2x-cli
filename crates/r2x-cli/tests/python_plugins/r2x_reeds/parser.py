"""Stub ReEDS parser used in integration tests."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


class ReEDSConfig:
    """Minimal config placeholder matching runtime expectations."""

    def __init__(self, weather_year: int | None = None, solve_year: int | None = None, **kwargs: Any) -> None:
        self.weather_year = weather_year
        self.solve_year = solve_year
        self.extra = kwargs


class ReEDSParser:
    """Parser that returns canned JSON for tests."""

    def __init__(self, config: ReEDSConfig | None = None, data_store: Any | None = None, **_: Any) -> None:
        self.config = config
        self.data_store = data_store

    def build_system(self) -> str:
        return '{"system": "reeds", "status": "ok"}'
