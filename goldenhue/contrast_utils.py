"""
contrast_utils.py
==================
Calcul du ratio de contraste WCAG 2.x entre deux couleurs, et classification
en badges AA / AAA (texte normal et texte large).
"""

from __future__ import annotations

from dataclasses import dataclass

from .color_utils import hex_to_rgb256


def _relative_luminance_channel(c_255: int) -> float:
    c = c_255 / 255.0
    if c <= 0.03928:
        return c / 12.92
    return ((c + 0.055) / 1.055) ** 2.4


def relative_luminance(hex_code: str) -> float:
    """Luminance relative WCAG d'une couleur (0 = noir, 1 = blanc)."""
    r, g, b = hex_to_rgb256(hex_code)
    r_lin, g_lin, b_lin = (_relative_luminance_channel(c) for c in (r, g, b))
    return 0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin


def contrast_ratio(hex_a: str, hex_b: str) -> float:
    """Ratio de contraste WCAG entre deux couleurs, dans [1, 21]."""
    l1 = relative_luminance(hex_a)
    l2 = relative_luminance(hex_b)
    lighter, darker = max(l1, l2), min(l1, l2)
    return (lighter + 0.05) / (darker + 0.05)


@dataclass(frozen=True)
class ContrastResult:
    ratio: float
    aa_normal: bool
    aaa_normal: bool
    aa_large: bool
    aaa_large: bool

    @property
    def badge(self) -> str:
        """Le meilleur badge atteint, pour du texte normal."""
        if self.aaa_normal:
            return "AAA"
        if self.aa_normal:
            return "AA"
        if self.aa_large:
            return "AA (grand texte)"
        return "Échec"


def evaluate_contrast(hex_a: str, hex_b: str) -> ContrastResult:
    """Évalue le contraste entre deux couleurs selon les seuils WCAG 2.x."""
    ratio = contrast_ratio(hex_a, hex_b)
    return ContrastResult(
        ratio=ratio,
        aa_normal=ratio >= 4.5,
        aaa_normal=ratio >= 7.0,
        aa_large=ratio >= 3.0,
        aaa_large=ratio >= 4.5,
    )


def best_contrasting_bw(hex_code: str) -> str:
    """Retourne '#000000' ou '#FFFFFF', celui offrant le meilleur contraste."""
    black_ratio = contrast_ratio(hex_code, "#000000")
    white_ratio = contrast_ratio(hex_code, "#FFFFFF")
    return "#000000" if black_ratio >= white_ratio else "#FFFFFF"
