"""
color_names.py
==============
Petite base de correspondance nom de couleur (français) -> code hexadécimal,
avec recherche approximative si le nom exact n'est pas trouvé.
"""

from __future__ import annotations

import difflib
from typing import Optional

COLOR_NAMES = {
    "rouge": "#E74C3C",
    "corail": "#FF7F50",
    "orange": "#F39C12",
    "ambre": "#FFBF00",
    "jaune": "#F1C40F",
    "citron": "#F4D03F",
    "or": "#D4AF37",
    "doré": "#D4AF37",
    "olive": "#808000",
    "vert": "#2ECC71",
    "vert forêt": "#228B22",
    "menthe": "#3EB489",
    "émeraude": "#50C878",
    "turquoise": "#1ABC9C",
    "sarcelle": "#008080",
    "cyan": "#00FFFF",
    "bleu ciel": "#87CEEB",
    "bleu": "#3498DB",
    "bleu marine": "#001F54",
    "indigo": "#4B0082",
    "violet": "#9B59B6",
    "mauve": "#B39DDB",
    "lavande": "#B57EDC",
    "magenta": "#FF00FF",
    "rose": "#FF69B4",
    "rose poudré": "#F4C2C2",
    "fuchsia": "#FF00A6",
    "bordeaux": "#800020",
    "marron": "#8B4513",
    "brun": "#795548",
    "beige": "#F5F5DC",
    "sable": "#C2B280",
    "crème": "#FFFDD0",
    "ivoire": "#FFFFF0",
    "blanc": "#FFFFFF",
    "gris": "#95A5A6",
    "gris ardoise": "#708090",
    "anthracite": "#2C3E50",
    "noir": "#000000",
    "saumon": "#FA8072",
    "pêche": "#FFDAB9",
    "abricot": "#FBCEB1",
    "prune": "#8E4585",
    "aubergine": "#3B0910",
    "chocolat": "#7B3F00",
    "kaki": "#78866B",
    "moutarde": "#FFDB58",
    "rouille": "#B7410E",
    "terracotta": "#E2725B",
}


def lookup_color_name(name: str) -> Optional[str]:
    """Retourne le hex correspondant au nom donné (exact ou approché), sinon None."""
    key = name.strip().lower()
    if key in COLOR_NAMES:
        return COLOR_NAMES[key]
    matches = difflib.get_close_matches(key, COLOR_NAMES.keys(), n=1, cutoff=0.6)
    if matches:
        return COLOR_NAMES[matches[0]]
    return None
