"""Stub file operations helpers."""

from __future__ import annotations

from pathlib import Path


def ensure_directory(path: str | Path) -> Path:
    """No-op helper that matches the production API."""
    return Path(path)
