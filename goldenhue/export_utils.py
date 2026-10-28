"""
export_utils.py
================
ExportManager : exporte une palette de couleurs vers différents formats
(CSV, JSON, CSS, SCSS, TXT, Adobe .ase, image PNG).
"""

from __future__ import annotations

import csv
import io
import json
import re
import struct
from typing import Iterable, List, Sequence, Tuple

from .color_utils import (
    format_hsl,
    format_hsv,
    format_rgb,
    hex_to_rgb01,
    readable_text_color,
)

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:  # pragma: no cover
    Image = ImageDraw = ImageFont = None


def _slugify(name: str) -> str:
    slug = re.sub(r"[^a-zA-Z0-9]+", "-", name.strip().lower()).strip("-")
    return slug or "couleur"


class ExportManager:
    """Regroupe les formats d'export disponibles pour une liste de (role, hex)."""

    def __init__(self, colors: Sequence[Tuple[str, str]], palette_name: str = "GoldenHue"):
        self.colors = list(colors)
        self.palette_name = palette_name

    # -- formats texte --------------------------------------------------

    def to_txt(self) -> str:
        lines = [f"{role} : {hexcode.upper()}" for role, hexcode in self.colors]
        return "\n".join(lines) + "\n"

    def to_csv(self) -> str:
        buf = io.StringIO()
        writer = csv.writer(buf)
        writer.writerow(["role", "hex", "rgb", "hsl", "hsv"])
        for role, hexcode in self.colors:
            writer.writerow(
                [role, hexcode.upper(), format_rgb(hexcode), format_hsl(hexcode), format_hsv(hexcode)]
            )
        return buf.getvalue()

    def to_json(self) -> str:
        data = [
            {
                "role": role,
                "hex": hexcode.upper(),
                "rgb": format_rgb(hexcode),
                "hsl": format_hsl(hexcode),
                "hsv": format_hsv(hexcode),
            }
            for role, hexcode in self.colors
        ]
        return json.dumps({"palette": self.palette_name, "colors": data}, ensure_ascii=False, indent=2)

    def to_css(self) -> str:
        lines = [":root {"]
        for role, hexcode in self.colors:
            lines.append(f"  --{_slugify(role)}: {hexcode.upper()};")
        lines.append("}")
        return "\n".join(lines) + "\n"

    def to_scss(self) -> str:
        lines = [f"$goldenhue-{_slugify(role)}: {hexcode.upper()};" for role, hexcode in self.colors]
        return "\n".join(lines) + "\n"

    # -- Adobe Swatch Exchange (.ase) -----------------------------------

    def to_ase_bytes(self) -> bytes:
        """
        Construit un fichier .ase minimal (spécification binaire ASE) :
        en-tête 'ASEF', version 1.0, puis un bloc "couleur globale" par swatch.
        """
        blocks = io.BytesIO()
        block_count = 0
        for role, hexcode in self.colors:
            r, g, b = hex_to_rgb01(hexcode)
            name = role[:63]
            name_utf16 = name.encode("utf-16-be") + b"\x00\x00"
            name_len = len(name_utf16) // 2

            body = io.BytesIO()
            body.write(struct.pack(">H", name_len))
            body.write(name_utf16)
            body.write(b"RGB ")
            body.write(struct.pack(">fff", r, g, b))
            body.write(struct.pack(">H", 0))  # 0 = Global color type
            body_bytes = body.getvalue()

            blocks.write(struct.pack(">H", 0x0001))  # bloc "couleur"
            blocks.write(struct.pack(">I", len(body_bytes)))
            blocks.write(body_bytes)
            block_count += 1

        out = io.BytesIO()
        out.write(b"ASEF")
        out.write(struct.pack(">HH", 1, 0))  # version 1.0
        out.write(struct.pack(">I", block_count))
        out.write(blocks.getvalue())
        return out.getvalue()

    # -- image PNG --------------------------------------------------------

    def to_png_bytes(self, swatch_width: int = 160, height: int = 220) -> bytes:
        """Génère une image PNG représentant la palette (blocs + libellés)."""
        if Image is None:
            raise RuntimeError("Pillow n'est pas installé : export PNG indisponible.")

        n = max(1, len(self.colors))
        width = swatch_width * n
        img = Image.new("RGB", (width, height), "#FFFFFF")
        draw = ImageDraw.Draw(img)

        try:
            font = ImageFont.load_default()
        except Exception:  # pragma: no cover
            font = None

        for i, (role, hexcode) in enumerate(self.colors):
            x0, x1 = i * swatch_width, (i + 1) * swatch_width
            draw.rectangle([x0, 0, x1, height], fill=hexcode)
            text_color = readable_text_color(hexcode)
            draw.text((x0 + 10, height - 40), role, fill=text_color, font=font)
            draw.text((x0 + 10, height - 22), hexcode.upper(), fill=text_color, font=font)

        buf = io.BytesIO()
        img.save(buf, format="PNG")
        return buf.getvalue()

    # -- écriture fichier --------------------------------------------------

    def save(self, path: str, fmt: str) -> str:
        """Écrit la palette dans `path` selon `fmt` ('csv'|'json'|'css'|'scss'|'txt'|'ase'|'png')."""
        fmt = fmt.lower()
        if fmt == "csv":
            content = self.to_csv()
        elif fmt == "json":
            content = self.to_json()
        elif fmt == "css":
            content = self.to_css()
        elif fmt == "scss":
            content = self.to_scss()
        elif fmt == "txt":
            content = self.to_txt()
        elif fmt == "ase":
            with open(path, "wb") as f:
                f.write(self.to_ase_bytes())
            return path
        elif fmt == "png":
            with open(path, "wb") as f:
                f.write(self.to_png_bytes())
            return path
        else:
            raise ValueError(f"Format d'export inconnu : {fmt}")

        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        return path
