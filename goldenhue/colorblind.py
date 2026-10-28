"""
colorblind.py
=============
Simulation approximative de daltonisme (protanopie, deutéranopie, tritanopie)
par transformation matricielle en espace RGB linéaire. Approximation basée
sur les matrices couramment utilisées par les outils d'accessibilité web
(Brettel/Viénot simplifiées) : suffisant pour un aperçu indicatif, pas un
substitut à un test clinique.
"""

from __future__ import annotations

from enum import Enum
from typing import Tuple

from .color_utils import hex_to_rgb01, rgb01_to_hex


class ColorblindType(str, Enum):
    PROTANOPIA = "protanopia"
    DEUTERANOPIA = "deuteranopia"
    TRITANOPIA = "tritanopia"


_MATRICES = {
    ColorblindType.PROTANOPIA: (
        (0.567, 0.433, 0.000),
        (0.558, 0.442, 0.000),
        (0.000, 0.242, 0.758),
    ),
    ColorblindType.DEUTERANOPIA: (
        (0.625, 0.375, 0.000),
        (0.700, 0.300, 0.000),
        (0.000, 0.300, 0.700),
    ),
    ColorblindType.TRITANOPIA: (
        (0.950, 0.050, 0.000),
        (0.000, 0.433, 0.567),
        (0.000, 0.475, 0.525),
    ),
}

CB_LABELS = {
    ColorblindType.PROTANOPIA: "Protanopie (déficit rouge)",
    ColorblindType.DEUTERANOPIA: "Deutéranopie (déficit vert)",
    ColorblindType.TRITANOPIA: "Tritanopie (déficit bleu)",
}


def _apply_matrix(rgb: Tuple[float, float, float], matrix) -> Tuple[float, float, float]:
    r, g, b = rgb
    return tuple(
        max(0.0, min(1.0, row[0] * r + row[1] * g + row[2] * b))
        for row in matrix
    )  # type: ignore


def simulate(hex_code: str, cb_type: "ColorblindType | str") -> str:
    """Retourne l'apparence approximative de hex_code pour le type de daltonisme donné."""
    cb_type = ColorblindType(cb_type)
    rgb = hex_to_rgb01(hex_code)
    simulated = _apply_matrix(rgb, _MATRICES[cb_type])
    return rgb01_to_hex(*simulated)


def simulate_all(hex_code: str) -> "dict[ColorblindType, str]":
    return {cb: simulate(hex_code, cb) for cb in ColorblindType}
