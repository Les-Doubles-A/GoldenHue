# GoldenHue (Rust Edition)

Générateur de palettes de couleurs natif ultra-optimisé écrit en **Rust** avec **eframe / egui** (accélération matérielle OpenGL). Toutes les fonctionnalités sont visibles simultanément dans une interface fluide à 60 FPS avec sections empilées et défilantes.

## Points forts & Optimisations (vs version Python/Tkinter)

- **Démarrage instantané & binaire natif autonome** : exécutable unique `GoldenHue.exe` (~4 Mo au lieu de ~35 Mo sous PyInstaller), sans interpréteur ni machine virtuelle, compilé avec `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"` et `strip = true`.
- **Décodage HEX et luminance WCAG en `O(1)`** :
  - Validation et parsing `#RRGGBB` par table de correspondance statique (`HEX_LUT` 256 octets) sans expression régulière.
  - Luminance relative WCAG 2.x précalculée dans une table de linéarisation sRGB (`SRGB_LINEAR_LUT` 256 entrées), éliminant tout appel à `powf(2.4)` lors des évaluations de contraste.
- **Mémoïsation complète (`ComputedCache`)** : les 7 harmonies chromatiques, les bilans de contraste WCAG et les simulations de daltonisme ne sont recalculés que lorsque la couleur de base ou la palette de travail changent (0 calcul redondant pendant le rendu 60 FPS).
- **Rendu GPU des dégradés CSS** : interpolation directe par sommets via un `egui::Mesh` matériel en un seul appel de dessin (au lieu de 400 segments CPU).
- **K-Means sans allocation en boucle** : extraction des couleurs dominantes d'une image sur buffer contigu `[f32; 3]` avec arrêt anticipé à convergence et générateur déterministe `XorShift64`.
- **Changement de thème instantané** : bascule Clair / Sombre en temps réel sans redémarrer l'application.
- **Vérification et mise à jour automatique via GitHub** : vérifie en arrière-plan au démarrage (et via le bouton « 🔄 Vérifier màj ») si une version plus récente est disponible sur `Les-Doubles-A/GoldenHue`, affiche une bannière de mise à jour et permet de télécharger + remplacer `GoldenHue.exe` et relancer l'application en un clic.

## Prérequis & Compilation

```bash
# Lancer en mode développement
cargo run --release

# Exécuter la suite de tests unitaires
cargo test

# Produire GoldenHue.exe à la racine (sous Windows)
.\build.bat
```

## Fonctionnalités

### 1. Harmonies de couleurs
Sept schémas générés à partir de `generate_palette(base_hex, scheme)` :
- **Golden** (nombre d'or, `137.5°`), **Complémentaire** (`180°`), **Analogue** (`±30°`), **Triadique** (`120° / 240°`), **Tétradique / carré** (`90° / 180° / 270°`), **Split-complémentaire** (`150° / 210°`), **Monochromatique**.
- Affichées **simultanément** dans la section « Toutes les harmonies », en plus de la palette de travail interactive.

### 2. Formats de couleur et export
- Bascule d'affichage `HEX` / `RGB` / `HSL` / `HSV` en temps réel.
- Export **CSV**, **JSON**, **CSS** (`:root { --var: ... }`), **SCSS**, **TXT**, **Adobe `.ase`** (binaire big-endian UTF-16BE), et **image PNG** (avec moteur de rendu de police matricielle 5x7 embarqué).
- Copie d'une couleur au clic, ou directement en `RGB` via `Ctrl+clic`.

### 3. Accessibilité / contraste WCAG & Daltonisme
- Ratio de contraste WCAG de chaque couleur de la palette de travail face au blanc (`#FFFFFF`) et au noir (`#000000`) avec badge **AAA**, **AA**, **AA (grand texte)** ou **Échec**.
- Comparateur libre entre deux couleurs quelconques de la palette.
- Simulation de daltonisme (**Protanopie**, **Deutéranopie**, **Tritanopie**).

### 4. Ergonomie / UX
- Historique des 12 dernières couleurs de base (`~/.goldenhue/history.json`).
- Favoris persistés localement (`~/.goldenhue/favorites.json`).
- Verrouillage individuel par couleur (`🔒` / `🔓`) et édition directe par sélecteur de couleur sur chaque carte (verrouille automatiquement).
- Génération aléatoire (« 🎲 Surprends-moi »), ajustement global de saturation/luminosité, Annuler/Refaire (`Undo`/`Redo`), recherche par nom de couleur en français (exacte ou approximative).

## Structure du projet

```text
Cargo.toml             Configuration Cargo et profil Release optimisé
build.rs               Intégration de l'icône Windows dans l'exécutable (.rc)
build.bat              Script de test et de compilation vers GoldenHue.exe
src/
  main.rs              Point d'entrée natif (fenêtre eframe + icône embarquée)
  lib.rs               Export des modules de la bibliothèque
  color_utils.rs       Conversions RGB/HEX/HSL/HSV (LUT O(1)) et harmonies
  contrast_utils.rs    Contraste WCAG 2.x avec table sRGB précalculée
  colorblind.rs        Simulation matricielle de daltonisme
  color_names.rs       Dictionnaire de 50 noms français + recherche floue
  image_extract.rs     Extraction K-Means rapide depuis une image
  export_utils.rs      Exports CSV, JSON, CSS, SCSS, TXT, ASE et PNG
  storage.rs           Persistance JSON (historique, favoris, session)
  theme.rs             Thèmes sombre et clair dynamiques
  ui.rs                Interface eframe/egui à sections empilées et cache mémoïsé
  updater.rs           Vérification et auto-mise à jour depuis GitHub
```
