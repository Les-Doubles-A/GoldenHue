# GoldenHue

Générateur de palettes de couleurs (Tkinter). Toutes les fonctionnalités
sont visibles en même temps dans une interface à sections empilées et
défilantes — rien n'est caché derrière un simple sélecteur.

## Installation

```bash
pip install -r requirements.txt
```

`tkinter` doit être disponible (`sudo apt install python3-tk` sous Debian/Ubuntu
si besoin — il est inclus par défaut avec la plupart des distributions Python
Windows/macOS).

## Lancer l'application

```bash
python3 GoldenHue_App.py
```

## Fonctionnalités

### 1. Harmonies de couleurs
Sept schémas générés à partir d'une fonction unique `generate_palette(base_hex, scheme)` :
Golden (nombre d'or, 137.5° — comportement d'origine), Complémentaire (180°),
Analogue (±30°), Triadique (120°/240°), Tétradique/carré (90°/180°/270°),
Split-complémentaire (150°/210°), Monochromatique. Elles sont **toutes affichées
simultanément** dans la section « Toutes les harmonies », en plus d'une palette
de travail interactive basée sur le schéma choisi dans la barre latérale.

### 2. Formats de couleur et export
- Bascule d'affichage HEX / RGB / HSL / HSV pour toute la palette.
- Export CSV, JSON, CSS (`:root { --var: ... }`), SCSS, TXT, Adobe `.ase`, image PNG.
- Copie de toute la palette de travail en un clic.
- `Ctrl+clic` sur une couleur copie directement sa valeur RGB.

### 3. Accessibilité / contraste
- Ratio de contraste WCAG de chaque couleur de la palette de travail vs
  blanc et noir, avec badge AA / AAA / Échec.
- Vérificateur libre entre deux couleurs quelconques de la palette.
- Simulation de daltonisme (protanopie, deutéranopie, tritanopie).

### 4. Ergonomie / UX
- Historique des couleurs de base récentes (mini-swatches cliquables).
- Favoris : palettes complètes sauvegardées localement (JSON).
- Verrouillage par couleur (🔒/🔓) : une couleur verrouillée ne change pas
  quand la base ou le schéma changent ; double-clic pour l'éditer manuellement
  (verrouille automatiquement).
- Génération aléatoire (« Surprends-moi »).
- Sliders de saturation/luminosité globale appliqués à la couleur de base.
- Undo/Redo sur les changements de couleur de base.

### 5. Interface
- Thème clair/sombre togglable (persisté, appliqué au redémarrage).
- Zone de contenu défilante, redimensionnement robuste : les rangées à
  beaucoup de couleurs (ex. tétradique = 14 swatches) ont leur propre
  défilement horizontal indépendant plutôt que d'être compressées ou coupées.
- Aperçu « UI mockup » (carte + bouton + texte) coloré avec la palette de travail.
- Barre de statut avec messages ✓ / ✗.
- Tooltips sur chaque swatch (rôle de la couleur).
- Section « Règles de design » : aide-mémoire dépliable regroupant les bonnes
  pratiques (harmonies, règle 60-30-10, contraste WCAG, daltonisme, couleurs
  sémantiques, psychologie des couleurs, dark mode, états interactifs...).

### 6. Fonctionnalités avancées
- Extraction de palette dominante depuis une image (k-means, `numpy` + `Pillow`).
- Génération à partir d'un nom de couleur français (« corail », « bleu marine »...).
- Mode dégradé : génère un dégradé CSS à partir de 2-3 couleurs de la palette
  de travail, avec aperçu visuel.
- Session sauvegardée automatiquement (dernière couleur, thème, schéma).

### 7. Qualité de code
- Logique séparée de l'UI : `goldenhue/color_utils.py`, `contrast_utils.py`,
  `colorblind.py`, `export_utils.py`, `image_extract.py`, `color_names.py`,
  `storage.py`, `theme.py` — aucune dépendance à Tkinter, testables isolément.
- Tests unitaires dans `tests/` (`python3 -m unittest discover -s tests`).
- Gestion des erreurs de presse-papier (`try/except tk.TclError`), messages
  affichés dans la barre de statut plutôt qu'un plantage silencieux.

## Structure

```
GoldenHue_App.py       point d'entrée
goldenhue/
  color_utils.py        conversions + harmonies (logique pure)
  contrast_utils.py      contraste WCAG
  colorblind.py           simulation de daltonisme
  color_names.py           noms de couleurs FR -> hex
  image_extract.py          extraction k-means depuis une image
  export_utils.py            CSV/JSON/CSS/SCSS/TXT/ASE/PNG
  storage.py                  historique / favoris / session (JSON local)
  theme.py                     thèmes clair/sombre
  ui.py                         interface Tkinter
tests/                  tests unitaires (logique pure, sans UI)
```
