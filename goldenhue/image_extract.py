"""
image_extract.py
=================
Extraction des couleurs dominantes d'une image par k-means sur les pixels.
Nécessite Pillow (chargement d'image) et numpy (k-means vectorisé).
"""

from __future__ import annotations

from typing import List

from .color_utils import rgb256_to_hex

try:
    import numpy as np
except ImportError:  # pragma: no cover
    np = None

try:
    from PIL import Image
except ImportError:  # pragma: no cover
    Image = None


class ImageExtractionError(RuntimeError):
    pass


def _require_deps() -> None:
    missing = []
    if Image is None:
        missing.append("Pillow")
    if np is None:
        missing.append("numpy")
    if missing:
        raise ImageExtractionError(
            "Dépendance manquante pour l'extraction d'image : " + ", ".join(missing)
        )


def _kmeans(pixels: "np.ndarray", k: int, iterations: int = 12, seed: int = 42) -> "np.ndarray":
    """K-means minimaliste et vectorisé (sans scikit-learn)."""
    rng = np.random.default_rng(seed)
    n = pixels.shape[0]
    k = min(k, n)
    init_idx = rng.choice(n, size=k, replace=False)
    centroids = pixels[init_idx].astype(np.float64)

    for _ in range(iterations):
        distances = ((pixels[:, None, :] - centroids[None, :, :]) ** 2).sum(axis=2)
        labels = distances.argmin(axis=1)
        new_centroids = centroids.copy()
        for i in range(k):
            members = pixels[labels == i]
            if len(members) > 0:
                new_centroids[i] = members.mean(axis=0)
        if np.allclose(new_centroids, centroids, atol=0.5):
            centroids = new_centroids
            break
        centroids = new_centroids

    # Trie les centroïdes par taille de cluster décroissante (couleurs les plus représentées d'abord)
    distances = ((pixels[:, None, :] - centroids[None, :, :]) ** 2).sum(axis=2)
    labels = distances.argmin(axis=1)
    counts = np.bincount(labels, minlength=k)
    order = np.argsort(-counts)
    return centroids[order]


def extract_dominant_colors(image_path: str, n_colors: int = 6, max_dim: int = 150) -> List[str]:
    """
    Extrait les `n_colors` couleurs dominantes d'une image par k-means.
    L'image est réduite à `max_dim` px sur son plus grand côté pour la performance.
    Retourne une liste de codes hexadécimaux, triés par prévalence décroissante.
    """
    _require_deps()

    img = Image.open(image_path).convert("RGB")
    img.thumbnail((max_dim, max_dim))
    pixels = np.asarray(img).reshape(-1, 3).astype(np.float64)

    if len(pixels) == 0:
        raise ImageExtractionError("Image vide ou illisible.")

    centroids = _kmeans(pixels, k=n_colors)
    hex_colors = [rgb256_to_hex(*c.round().astype(int)) for c in centroids]
    return hex_colors
