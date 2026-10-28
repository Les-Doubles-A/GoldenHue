"""
storage.py
==========
Persistance locale (JSON, dans le dossier utilisateur) pour :
- l'historique des couleurs de base récentes
- les palettes favorites
- l'état de session (dernière couleur, thème)
"""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Dict, List, Optional

APP_DIR_NAME = ".goldenhue"


def _app_dir() -> Path:
    d = Path.home() / APP_DIR_NAME
    d.mkdir(parents=True, exist_ok=True)
    return d


def _read_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError):
        return default


def _write_json(path: Path, data: Any) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


class HistoryStore:
    """Historique des dernières couleurs de base utilisées (les plus récentes en tête)."""

    def __init__(self, max_items: int = 12, path: Optional[Path] = None):
        self.max_items = max_items
        self.path = path or (_app_dir() / "history.json")

    def load(self) -> List[str]:
        return _read_json(self.path, [])

    def add(self, hex_code: str) -> List[str]:
        items = [h for h in self.load() if h.upper() != hex_code.upper()]
        items.insert(0, hex_code.upper())
        items = items[: self.max_items]
        _write_json(self.path, items)
        return items


class FavoritesStore:
    """Palettes complètes sauvegardées par l'utilisateur."""

    def __init__(self, path: Optional[Path] = None):
        self.path = path or (_app_dir() / "favorites.json")

    def load(self) -> List[Dict[str, Any]]:
        return _read_json(self.path, [])

    def save_palette(self, name: str, base_hex: str, scheme: str, colors: List[Dict[str, str]]) -> Dict[str, Any]:
        favorites = self.load()
        entry = {
            "name": name,
            "base_hex": base_hex.upper(),
            "scheme": scheme,
            "colors": colors,
            "saved_at": time.time(),
        }
        favorites.insert(0, entry)
        _write_json(self.path, favorites)
        return entry

    def delete(self, index: int) -> None:
        favorites = self.load()
        if 0 <= index < len(favorites):
            favorites.pop(index)
            _write_json(self.path, favorites)


class SessionStore:
    """Dernier état de la fenêtre : couleur de base, thème, schéma sélectionné."""

    def __init__(self, path: Optional[Path] = None):
        self.path = path or (_app_dir() / "session.json")

    def load(self) -> Dict[str, Any]:
        return _read_json(
            self.path,
            {"base_hex": "#F1C40F", "theme": "dark", "last_scheme": "golden"},
        )

    def update(self, **kwargs: Any) -> Dict[str, Any]:
        data = self.load()
        data.update(kwargs)
        _write_json(self.path, data)
        return data
