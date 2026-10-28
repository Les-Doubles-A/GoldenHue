"""
theme.py
========
Palettes de thème clair/sombre pour l'interface Tkinter (fini le codage en
dur de couleurs partout dans l'UI).
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Theme:
    name: str
    bg: str
    bg_alt: str
    panel: str
    text: str
    text_muted: str
    accent: str
    accent_text: str
    entry_bg: str
    entry_fg: str
    success: str
    error: str
    border: str
    font_family: str = "Segoe UI"


DARK = Theme(
    name="dark",
    bg="#1e2a38",
    bg_alt="#2c3e50",
    panel="#25333f",
    text="#ecf0f1",
    text_muted="#95a5a6",
    accent="#f1c40f",
    accent_text="#2c3e50",
    entry_bg="#ecf0f1",
    entry_fg="#2c3e50",
    success="#2ecc71",
    error="#e74c3c",
    border="#3d5166",
)

LIGHT = Theme(
    name="light",
    bg="#f4f6f7",
    bg_alt="#ffffff",
    panel="#ffffff",
    text="#2c3e50",
    text_muted="#7f8c8d",
    accent="#d68910",
    accent_text="#ffffff",
    entry_bg="#ffffff",
    entry_fg="#2c3e50",
    success="#27ae60",
    error="#c0392b",
    border="#dcdfe1",
)

THEMES = {"dark": DARK, "light": LIGHT}


def get_theme(name: str) -> Theme:
    return THEMES.get(name, DARK)
