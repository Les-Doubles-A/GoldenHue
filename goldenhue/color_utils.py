"""
color_utils.py
==============
Logique pure de conversion de couleurs et de génération de palettes/harmonies.
Aucune dépendance à l'UI : ce module peut être testé et réutilisé indépendamment.
"""

from __future__ import annotations

import colorsys
import re
from dataclasses import dataclass
from enum import Enum
from typing import List, Tuple

HEX_RE = re.compile(r"^#[0-9A-Fa-f]{6}$")


def clamp(value: float, lo: float = 0.0, hi: float = 1.0) -> float:
    """Restreint une valeur à l'intervalle [lo, hi]."""
    return max(lo, min(hi, value))


def is_valid_hex(hex_code: str) -> bool:
    """Vérifie qu'une chaîne est un code hexadécimal #RRGGBB valide."""
    return bool(HEX_RE.fullmatch(hex_code.strip()))


def normalize_hex(hex_code: str) -> str:
    """Ajoute le '#' si absent et met en majuscules. Lève ValueError si invalide."""
    value = hex_code.strip()
    if not value.startswith("#"):
        value = "#" + value
    if not is_valid_hex(value):
        raise ValueError(f"Code hexadécimal invalide : {hex_code!r}")
    return value.upper()


# ---------------------------------------------------------------------------
# Conversions RGB / HEX
# ---------------------------------------------------------------------------

def hex_to_rgb01(hex_code: str) -> Tuple[float, float, float]:
    """Convertit un code hexadécimal en RGB flottant [0, 1]."""
    h = hex_code.lstrip("#")
    return tuple(int(h[i:i + 2], 16) / 255.0 for i in (0, 2, 4))  # type: ignore


def hex_to_rgb256(hex_code: str) -> Tuple[int, int, int]:
    """Convertit un code hexadécimal en RGB entier [0, 255]."""
    h = hex_code.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))  # type: ignore


def rgb01_to_hex(r: float, g: float, b: float) -> str:
    """Convertit du RGB flottant [0, 1] en code hexadécimal."""
    return "#{:02X}{:02X}{:02X}".format(
        round(clamp(r) * 255), round(clamp(g) * 255), round(clamp(b) * 255)
    )


def rgb256_to_hex(r: int, g: int, b: int) -> str:
    """Convertit du RGB entier [0, 255] en code hexadécimal."""
    return "#{:02X}{:02X}{:02X}".format(
        int(clamp(r, 0, 255)), int(clamp(g, 0, 255)), int(clamp(b, 0, 255))
    )


# Alias conservé pour compatibilité avec l'ancien script
def rgb_to_hex(r: float, g: float, b: float) -> str:
    return rgb01_to_hex(r, g, b)


def hex_to_rgb(hex_code: str) -> Tuple[float, float, float]:
    return hex_to_rgb01(hex_code)


# ---------------------------------------------------------------------------
# Conversions HSL / HSV (unités "humaines" : H en degrés, S/L/V en %)
# ---------------------------------------------------------------------------

def rgb01_to_hsl(r: float, g: float, b: float) -> Tuple[float, float, float]:
    """RGB [0,1] -> HSL (H en degrés [0,360), S et L en % [0,100])."""
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    return (h * 360.0) % 360.0, s * 100.0, l * 100.0


def hsl_to_rgb01(h: float, s: float, l: float) -> Tuple[float, float, float]:
    """HSL (H en degrés, S/L en %) -> RGB [0,1]."""
    return colorsys.hls_to_rgb((h % 360.0) / 360.0, clamp(l / 100.0), clamp(s / 100.0))


def rgb01_to_hsv(r: float, g: float, b: float) -> Tuple[float, float, float]:
    """RGB [0,1] -> HSV (H en degrés, S/V en %)."""
    h, s, v = colorsys.rgb_to_hsv(r, g, b)
    return (h * 360.0) % 360.0, s * 100.0, v * 100.0


def hsv_to_rgb01(h: float, s: float, v: float) -> Tuple[float, float, float]:
    """HSV (H en degrés, S/V en %) -> RGB [0,1]."""
    return colorsys.hsv_to_rgb((h % 360.0) / 360.0, clamp(s / 100.0), clamp(v / 100.0))


def hex_to_hsl(hex_code: str) -> Tuple[float, float, float]:
    return rgb01_to_hsl(*hex_to_rgb01(hex_code))


def hex_to_hsv(hex_code: str) -> Tuple[float, float, float]:
    return rgb01_to_hsv(*hex_to_rgb01(hex_code))


def hsl_to_hex(h: float, s: float, l: float) -> str:
    return rgb01_to_hex(*hsl_to_rgb01(h, s, l))


def hsv_to_hex(h: float, s: float, v: float) -> str:
    return rgb01_to_hex(*hsv_to_rgb01(h, s, v))


def format_rgb(hex_code: str) -> str:
    r, g, b = hex_to_rgb256(hex_code)
    return f"rgb({r}, {g}, {b})"


def format_hsl(hex_code: str) -> str:
    h, s, l = hex_to_hsl(hex_code)
    return f"hsl({round(h)}, {round(s)}%, {round(l)}%)"


def format_hsv(hex_code: str) -> str:
    h, s, v = hex_to_hsv(hex_code)
    return f"hsv({round(h)}, {round(s)}%, {round(v)}%)"


class ColorFormat(str, Enum):
    HEX = "HEX"
    RGB = "RGB"
    HSL = "HSL"
    HSV = "HSV"


def format_color(hex_code: str, fmt: "ColorFormat | str") -> str:
    """Formate une couleur selon le format demandé (HEX / RGB / HSL / HSV)."""
    fmt = ColorFormat(fmt)
    if fmt is ColorFormat.HEX:
        return hex_code.upper()
    if fmt is ColorFormat.RGB:
        return format_rgb(hex_code)
    if fmt is ColorFormat.HSL:
        return format_hsl(hex_code)
    if fmt is ColorFormat.HSV:
        return format_hsv(hex_code)
    raise ValueError(f"Format inconnu : {fmt}")


def readable_text_color(hex_code: str) -> str:
    """Retourne 'white' ou 'black' selon la luminosité perçue de hex_code."""
    r, g, b = hex_to_rgb256(hex_code)
    # Luminance perçue (formule classique, pas WCAG) : suffisant pour le choix de texte.
    perceived = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0
    return "black" if perceived > 0.6 else "white"


# ---------------------------------------------------------------------------
# Génération de palettes / harmonies
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class SwatchSpec:
    """Une couleur nommée dans une palette générée."""
    role: str
    hex: str


class SchemeType(str, Enum):
    GOLDEN = "golden"
    COMPLEMENTARY = "complementary"
    ANALOGOUS = "analogous"
    TRIADIC = "triadic"
    TETRADIC = "tetradic"
    SPLIT_COMPLEMENTARY = "split_complementary"
    MONOCHROMATIC = "monochromatic"


SCHEME_LABELS = {
    SchemeType.GOLDEN: "Golden (nombre d'or, 137.5°)",
    SchemeType.COMPLEMENTARY: "Complémentaire (180°)",
    SchemeType.ANALOGOUS: "Analogue (±30°)",
    SchemeType.TRIADIC: "Triadique (120° / 240°)",
    SchemeType.TETRADIC: "Tétradique / carré (90° / 180° / 270°)",
    SchemeType.SPLIT_COMPLEMENTARY: "Split-complémentaire (150° / 210°)",
    SchemeType.MONOCHROMATIC: "Monochromatique",
}

NEUTRAL_ROLE_BG = "Fond Neutre"
NEUTRAL_ROLE_TEXT = "Texte Neutre"


def _tri_tone(h: float, l: float, s: float, label: str) -> List[SwatchSpec]:
    """Pour une teinte donnée : ton de base + ombre (-60%) + lumière (+50%)."""
    l_dark = clamp(l * 0.40)
    l_light = clamp(l + (1 - l) * 0.50)
    return [
        SwatchSpec(label, rgb01_to_hex(*colorsys.hls_to_rgb(h, l, s))),
        SwatchSpec(f"{label} -60%", rgb01_to_hex(*colorsys.hls_to_rgb(h, l_dark, s))),
        SwatchSpec(f"{label} +50%", rgb01_to_hex(*colorsys.hls_to_rgb(h, l_light, s))),
    ]


def _neutrals(h: float, s: float) -> List[SwatchSpec]:
    return [
        SwatchSpec(NEUTRAL_ROLE_BG, rgb01_to_hex(*colorsys.hls_to_rgb(h, 0.95, clamp(s * 0.15)))),
        SwatchSpec(NEUTRAL_ROLE_TEXT, rgb01_to_hex(*colorsys.hls_to_rgb(h, 0.15, clamp(s * 0.15)))),
    ]


def _hue_scheme(base_hex: str, hue_offsets_labels: List[Tuple[float, str]]) -> List[SwatchSpec]:
    """Construit une palette à partir d'une liste de (décalage de teinte en °, libellé)."""
    r, g, b = hex_to_rgb01(base_hex)
    h, l, s = colorsys.rgb_to_hls(r, g, b)

    swatches: List[SwatchSpec] = []
    for offset_deg, label in hue_offsets_labels:
        hue = (h + offset_deg / 360.0) % 1.0
        swatches.extend(_tri_tone(hue, l, s, label))
    swatches.extend(_neutrals(h, s))
    return swatches


def generate_golden(base_hex: str) -> List[SwatchSpec]:
    """Schéma d'origine de GoldenHue : décalage de l'angle d'or (137.5°)."""
    return _hue_scheme(base_hex, [(0.0, "Base"), (137.5, "Sec. (137.5°)")])


def generate_complementary(base_hex: str) -> List[SwatchSpec]:
    return _hue_scheme(base_hex, [(0.0, "Base"), (180.0, "Complément")])


def generate_analogous(base_hex: str) -> List[SwatchSpec]:
    return _hue_scheme(
        base_hex, [(-30.0, "Analogue -30°"), (0.0, "Base"), (30.0, "Analogue +30°")]
    )


def generate_triadic(base_hex: str) -> List[SwatchSpec]:
    return _hue_scheme(
        base_hex, [(0.0, "Base"), (120.0, "Triade 120°"), (240.0, "Triade 240°")]
    )


def generate_tetradic(base_hex: str) -> List[SwatchSpec]:
    return _hue_scheme(
        base_hex,
        [(0.0, "Base"), (90.0, "Carré 90°"), (180.0, "Carré 180°"), (270.0, "Carré 270°")],
    )


def generate_split_complementary(base_hex: str) -> List[SwatchSpec]:
    return _hue_scheme(
        base_hex, [(0.0, "Base"), (150.0, "Split 150°"), (210.0, "Split 210°")]
    )


def generate_monochromatic(base_hex: str) -> List[SwatchSpec]:
    """Variations de luminosité/saturation sur la même teinte."""
    r, g, b = hex_to_rgb01(base_hex)
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    steps = [0.15, 0.30, 0.45, l, clamp(l + (1 - l) * 0.35), clamp(l + (1 - l) * 0.65), 0.90]
    swatches = []
    for i, step_l in enumerate(steps):
        label = "Base" if abs(step_l - l) < 1e-6 else f"Ton {i + 1}"
        swatches.append(SwatchSpec(label, rgb01_to_hex(*colorsys.hls_to_rgb(h, clamp(step_l), s))))
    swatches.extend(_neutrals(h, s))
    return swatches


_SCHEME_GENERATORS = {
    SchemeType.GOLDEN: generate_golden,
    SchemeType.COMPLEMENTARY: generate_complementary,
    SchemeType.ANALOGOUS: generate_analogous,
    SchemeType.TRIADIC: generate_triadic,
    SchemeType.TETRADIC: generate_tetradic,
    SchemeType.SPLIT_COMPLEMENTARY: generate_split_complementary,
    SchemeType.MONOCHROMATIC: generate_monochromatic,
}


def generate_palette(base_hex: str, scheme: "SchemeType | str" = SchemeType.GOLDEN) -> List[SwatchSpec]:
    """Point d'entrée unique : dispatch vers le générateur du schéma demandé."""
    scheme = SchemeType(scheme)
    return _SCHEME_GENERATORS[scheme](base_hex)


def generate_all_schemes(base_hex: str) -> "dict[SchemeType, List[SwatchSpec]]":
    """Génère TOUS les schémas d'un coup (utilisé pour un affichage simultané)."""
    return {scheme: gen(base_hex) for scheme, gen in _SCHEME_GENERATORS.items()}


def apply_global_adjustment(base_hex: str, sat_delta: float, light_delta: float) -> str:
    """Applique un delta global de saturation/luminosité (en points de %) à une couleur."""
    h, s, l = hex_to_hsl(base_hex)
    return hsl_to_hex(h, clamp(s + sat_delta, 0, 100), clamp(l + light_delta, 0, 100))


def random_hex() -> str:
    """Génère une couleur aléatoire agréable (saturation/luminosité bornées)."""
    import random
    h = random.uniform(0, 360)
    s = random.uniform(45, 85)
    l = random.uniform(35, 65)
    return hsl_to_hex(h, s, l)
