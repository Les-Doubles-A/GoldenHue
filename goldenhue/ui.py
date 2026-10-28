"""
ui.py
=====
Interface graphique Tkinter de GoldenHue.

Principe d'organisation : au lieu de cacher les fonctionnalités derrière des
onglets ou un simple sélecteur, TOUTES les sections (harmonies, contraste,
daltonisme, aperçu UI, dégradé, extraction d'image, favoris, export) sont
empilées dans une zone défilante et visibles en même temps. Un panneau de
gauche regroupe les contrôles globaux (couleur de base, historique,
undo/redo, thème, verrouillage...).
"""

from __future__ import annotations

import os
import tkinter as tk
from tkinter import colorchooser, filedialog, messagebox, simpledialog, ttk

from . import color_utils as cu
from . import colorblind as cbmod
from . import color_names as cnames
from . import contrast_utils as ctr
from . import export_utils as exu
from . import image_extract as imgx
from . import storage as store
from .resources import set_app_icon
from .theme import Theme, get_theme

APP_TITLE = "GoldenHue - Générateur de Palette"


class Tooltip:
    """Petite infobulle affichée au survol d'un widget."""

    def __init__(self, widget: tk.Widget, text: str, theme: Theme):
        self.widget = widget
        self.text = text
        self.theme = theme
        self.tip: tk.Toplevel | None = None
        widget.bind("<Enter>", self._show, add="+")
        widget.bind("<Leave>", self._hide, add="+")

    def _show(self, _event=None) -> None:
        if self.tip is not None or not self.text:
            return
        x = self.widget.winfo_rootx() + 12
        y = self.widget.winfo_rooty() + self.widget.winfo_height() + 6
        self.tip = tk.Toplevel(self.widget)
        self.tip.wm_overrideredirect(True)
        self.tip.wm_geometry(f"+{x}+{y}")
        lbl = tk.Label(
            self.tip, text=self.text, bg="#111111", fg="#ffffff",
            font=("Segoe UI", 9), padx=6, pady=3, relief="solid", bd=1,
        )
        lbl.pack()

    def _hide(self, _event=None) -> None:
        if self.tip is not None:
            self.tip.destroy()
            self.tip = None


class HScrollRow(tk.Frame):
    """
    Rangée de swatches à défilement horizontal indépendant.

    Certains schémas (tétradique = 14 couleurs, triadique/analogue/split = 11)
    peuvent dépasser la largeur disponible : plutôt que de les compresser
    jusqu'à rendre le texte illisible ou de les couper hors champ, cette
    rangée s'étire pour remplir l'espace quand le contenu tient, et devient
    défilable horizontalement sinon — toutes les couleurs restent toujours
    atteignables, quelle que soit la taille de la fenêtre.
    """

    def __init__(self, parent: tk.Frame, theme: Theme, height: int = 108):
        super().__init__(parent, bg=theme.panel)
        self.theme = theme
        self.canvas = tk.Canvas(self, bg=theme.panel, height=height, highlightthickness=0)
        self.hscroll = tk.Scrollbar(self, orient="horizontal", command=self.canvas.xview)
        self.canvas.configure(xscrollcommand=self.hscroll.set)
        self.canvas.pack(side=tk.TOP, fill=tk.X, expand=True)
        self.hscroll.pack(side=tk.TOP, fill=tk.X)

        self.inner = tk.Frame(self.canvas, bg=theme.panel)
        self.inner_window = self.canvas.create_window((0, 0), window=self.inner, anchor="nw")
        self.inner.bind("<Configure>", self._on_inner_configure)

    def _on_inner_configure(self, _event=None) -> None:
        # Le cadre interne garde sa taille naturelle (somme des largeurs des
        # swatches) : jamais compressé. S'il dépasse la largeur visible, la
        # scrollbar horizontale permet d'atteindre les couleurs restantes.
        self.canvas.configure(scrollregion=self.canvas.bbox("all"))

    def clear(self) -> None:
        for widget in self.inner.winfo_children():
            widget.destroy()


def _role_description(role: str) -> str:
    role_low = role.lower()
    if "fond neutre" in role_low:
        return "Couleur de fond neutre suggérée"
    if "texte neutre" in role_low:
        return "Couleur de texte neutre suggérée"
    if "-60%" in role:
        return "Variante assombrie (ombre)"
    if "+50%" in role:
        return "Variante éclaircie (teinte)"
    if role_low == "base":
        return "Couleur principale de la palette"
    return "Couleur harmonique dérivée de la base"


class PaletteApp:
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.title(APP_TITLE)
        self.root.geometry("1240x820")
        self.root.minsize(960, 600)
        set_app_icon(self.root)  # icône dans la barre de titre, la barre des tâches et Alt+Tab

        # --- État --------------------------------------------------------
        self.session = store.SessionStore()
        self.history_store = store.HistoryStore()
        self.favorites_store = store.FavoritesStore()

        session_data = self.session.load()
        self.theme_name = session_data.get("theme", "dark")
        self.theme = get_theme(self.theme_name)

        self.hex_var = tk.StringVar(value=session_data.get("base_hex", "#F1C40F"))
        self.name_var = tk.StringVar(value="")
        self.format_var = tk.StringVar(value=cu.ColorFormat.HEX.value)
        self.working_scheme_var = tk.StringVar(
            value=session_data.get("last_scheme", cu.SchemeType.GOLDEN.value)
        )
        self.sat_slider_var = tk.DoubleVar(value=0.0)
        self.light_slider_var = tk.DoubleVar(value=0.0)

        self.base_hex = cu.normalize_hex(self.hex_var.get())
        self.working_palette: list[cu.SwatchSpec] = []
        self.locked_indices: set[int] = set()
        self.undo_stack: list[str] = []
        self.redo_stack: list[str] = []
        self._label_registry: list[tuple[tk.Label, str]] = []  # (label, hex) pour bascule de format

        self._build_layout()
        self.regenerate_working_palette(push_history=False)
        self.refresh_all_sections()

    # ------------------------------------------------------------------
    # Construction du layout général
    # ------------------------------------------------------------------

    def _build_layout(self) -> None:
        t = self.theme
        self.root.configure(bg=t.bg)

        # Barre d'outils supérieure (toujours visible)
        self.toolbar = tk.Frame(self.root, bg=t.bg_alt)
        self.toolbar.pack(side=tk.TOP, fill=tk.X)
        self._build_toolbar(self.toolbar)

        # Corps : panneau latéral fixe + zone défilante
        body = tk.Frame(self.root, bg=t.bg)
        body.pack(side=tk.TOP, fill=tk.BOTH, expand=True)

        self.sidebar = tk.Frame(body, bg=t.bg_alt, width=300)
        self.sidebar.pack(side=tk.LEFT, fill=tk.Y)
        self.sidebar.pack_propagate(False)
        self._build_sidebar(self.sidebar)

        self.scroll_container = tk.Frame(body, bg=t.bg)
        self.scroll_container.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        self._build_scroll_area(self.scroll_container)

        # Barre de statut (toujours visible)
        self.status_lbl = tk.Label(
            self.root, text="Prêt.", font=("Segoe UI", 10, "italic"),
            fg=t.text_muted, bg=t.bg, anchor="w", padx=12, pady=6,
        )
        self.status_lbl.pack(side=tk.BOTTOM, fill=tk.X)

    def _build_scroll_area(self, parent: tk.Frame) -> None:
        t = self.theme
        self.canvas = tk.Canvas(parent, bg=t.bg, highlightthickness=0)
        vscroll = tk.Scrollbar(parent, orient="vertical", command=self.canvas.yview)
        self.canvas.configure(yscrollcommand=vscroll.set)
        self.canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        vscroll.pack(side=tk.RIGHT, fill=tk.Y)

        self.scroll_frame = tk.Frame(self.canvas, bg=t.bg)
        self.canvas_window = self.canvas.create_window((0, 0), window=self.scroll_frame, anchor="nw")

        def on_frame_configure(_event=None):
            self.canvas.configure(scrollregion=self.canvas.bbox("all"))

        def on_canvas_configure(event):
            self.canvas.itemconfig(self.canvas_window, width=event.width)

        self.scroll_frame.bind("<Configure>", on_frame_configure)
        self.canvas.bind("<Configure>", on_canvas_configure)

        def on_mousewheel(event):
            delta = -1 if event.num == 5 or event.delta < 0 else 1
            self.canvas.yview_scroll(-delta, "units")

        self.canvas.bind_all("<MouseWheel>", on_mousewheel)
        self.canvas.bind_all("<Button-4>", on_mousewheel)
        self.canvas.bind_all("<Button-5>", on_mousewheel)

        # --- Sections empilées, toutes visibles en même temps ---
        self.section_working = self._section(self.scroll_frame, "🎨 Palette de travail")
        self.section_schemes = self._section(self.scroll_frame, "🧭 Toutes les harmonies")
        self.section_contrast = self._section(self.scroll_frame, "🔎 Contraste WCAG")
        self.section_colorblind = self._section(self.scroll_frame, "👁 Simulation de daltonisme")
        self.section_mockup = self._section(self.scroll_frame, "🖼 Aperçu UI")
        self.section_gradient = self._section(self.scroll_frame, "🌈 Dégradé CSS")
        self.section_image = self._section(self.scroll_frame, "📷 Extraction depuis une image")
        self.section_favorites = self._section(self.scroll_frame, "⭐ Favoris")
        self.section_export = self._section(self.scroll_frame, "⬇ Export")
        self.section_rules = self._section(self.scroll_frame, "📚 Règles de design (aide-mémoire)")

        self._build_working_section()
        self._build_schemes_section()
        self._build_contrast_section()
        self._build_colorblind_section()
        self._build_mockup_section()
        self._build_gradient_section()
        self._build_image_section()
        self._build_favorites_section()
        self._build_export_section()
        self._build_rules_section()

    def _section(self, parent: tk.Frame, title: str) -> tk.Frame:
        t = self.theme
        outer = tk.Frame(parent, bg=t.panel, highlightbackground=t.border, highlightthickness=1)
        outer.pack(side=tk.TOP, fill=tk.X, expand=True, padx=14, pady=8)
        header = tk.Label(
            outer, text=title, font=("Segoe UI", 12, "bold"), bg=t.panel, fg=t.accent,
            anchor="w", padx=10, pady=8,
        )
        header.pack(side=tk.TOP, fill=tk.X)
        body = tk.Frame(outer, bg=t.panel, padx=10)
        body.pack(side=tk.TOP, fill=tk.BOTH, expand=True, pady=(0, 10))
        return body

    # ------------------------------------------------------------------
    # Barre d'outils
    # ------------------------------------------------------------------

    def _build_toolbar(self, parent: tk.Frame) -> None:
        t = self.theme
        tk.Label(
            parent, text="GoldenHue", font=("Segoe UI", 16, "bold"), fg=t.accent, bg=t.bg_alt,
            padx=14, pady=10,
        ).pack(side=tk.LEFT)

        right = tk.Frame(parent, bg=t.bg_alt)
        right.pack(side=tk.RIGHT, padx=10)

        self.undo_btn = self._toolbar_button(right, "↺ Annuler", self.undo)
        self.redo_btn = self._toolbar_button(right, "↻ Refaire", self.redo)
        self._toolbar_button(right, "🎲 Surprends-moi", self.on_random_color)
        self.theme_btn = self._toolbar_button(right, "☾ Thème", self.toggle_theme)

    def _toolbar_button(self, parent: tk.Frame, text: str, command) -> tk.Button:
        t = self.theme
        btn = tk.Button(
            parent, text=text, command=command, font=("Segoe UI", 10, "bold"),
            bg=t.panel, fg=t.text, relief="flat", padx=10, pady=6, cursor="hand2",
            activebackground=t.accent, activeforeground=t.accent_text,
        )
        btn.pack(side=tk.LEFT, padx=4, pady=8)
        return btn

    # ------------------------------------------------------------------
    # Panneau latéral : sélection de couleur, historique, format, sliders
    # ------------------------------------------------------------------

    def _build_sidebar(self, parent: tk.Frame) -> None:
        t = self.theme
        pad = dict(padx=16, pady=(14, 4))

        tk.Label(parent, text="Couleur de base", font=("Segoe UI", 11, "bold"),
                 fg=t.text, bg=t.bg_alt).pack(anchor="w", **pad)

        tk.Button(
            parent, text="Choisir une couleur…", command=self.on_pick_color,
            font=("Segoe UI", 10, "bold"), bg=t.accent, fg=t.accent_text,
            relief="flat", pady=8, cursor="hand2",
        ).pack(fill=tk.X, padx=16, pady=4)

        hex_row = tk.Frame(parent, bg=t.bg_alt)
        hex_row.pack(fill=tk.X, padx=16, pady=4)
        tk.Label(hex_row, text="HEX", font=("Consolas", 10, "bold"), fg=t.text_muted, bg=t.bg_alt).pack(side=tk.LEFT)
        self.hex_entry = tk.Entry(
            hex_row, textvariable=self.hex_var, font=("Consolas", 12, "bold"),
            bg=t.entry_bg, fg=t.entry_fg, relief="flat", justify="center",
        )
        self.hex_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=5, padx=(8, 0))
        self.hex_entry.bind("<Return>", self.on_apply_hex_entry)
        self.hex_entry.bind("<FocusOut>", self.on_apply_hex_entry)

        name_row = tk.Frame(parent, bg=t.bg_alt)
        name_row.pack(fill=tk.X, padx=16, pady=4)
        tk.Label(name_row, text="Nom", font=("Consolas", 10, "bold"), fg=t.text_muted, bg=t.bg_alt).pack(side=tk.LEFT)
        name_entry = tk.Entry(
            name_row, textvariable=self.name_var, font=("Segoe UI", 11),
            bg=t.entry_bg, fg=t.entry_fg, relief="flat",
        )
        name_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=4, padx=(8, 0))
        name_entry.bind("<Return>", self.on_apply_name_entry)
        Tooltip(name_entry, "Ex. « corail », « bleu marine »… puis Entrée", t)

        # Format d'affichage
        tk.Label(parent, text="Format affiché", font=("Segoe UI", 11, "bold"),
                 fg=t.text, bg=t.bg_alt).pack(anchor="w", **pad)
        fmt_row = tk.Frame(parent, bg=t.bg_alt)
        fmt_row.pack(fill=tk.X, padx=16, pady=4)
        fmt_combo = ttk.Combobox(
            fmt_row, textvariable=self.format_var, state="readonly",
            values=[f.value for f in cu.ColorFormat],
        )
        fmt_combo.pack(fill=tk.X)
        fmt_combo.bind("<<ComboboxSelected>>", lambda e: self.on_format_changed())
        Tooltip(fmt_row, "HEX / RGB / HSL / HSV — Ctrl+clic sur une couleur copie toujours en RGB", t)

        # Historique
        tk.Label(parent, text="Historique récent", font=("Segoe UI", 11, "bold"),
                 fg=t.text, bg=t.bg_alt).pack(anchor="w", **pad)
        self.history_row = tk.Frame(parent, bg=t.bg_alt)
        self.history_row.pack(fill=tk.X, padx=16, pady=4)

        # Sliders globaux saturation / luminosité
        tk.Label(parent, text="Ajustement global", font=("Segoe UI", 11, "bold"),
                 fg=t.text, bg=t.bg_alt).pack(anchor="w", **pad)
        tk.Label(parent, text="Saturation", font=("Segoe UI", 9), fg=t.text_muted, bg=t.bg_alt).pack(anchor="w", padx=16)
        tk.Scale(
            parent, from_=-50, to=50, orient=tk.HORIZONTAL, variable=self.sat_slider_var,
            bg=t.bg_alt, fg=t.text, troughcolor=t.panel, highlightthickness=0, showvalue=True,
        ).pack(fill=tk.X, padx=16)
        tk.Label(parent, text="Luminosité", font=("Segoe UI", 9), fg=t.text_muted, bg=t.bg_alt).pack(anchor="w", padx=16)
        tk.Scale(
            parent, from_=-50, to=50, orient=tk.HORIZONTAL, variable=self.light_slider_var,
            bg=t.bg_alt, fg=t.text, troughcolor=t.panel, highlightthickness=0, showvalue=True,
        ).pack(fill=tk.X, padx=16)
        tk.Button(
            parent, text="Appliquer l'ajustement", command=self.on_apply_sliders,
            font=("Segoe UI", 9, "bold"), bg=t.panel, fg=t.text, relief="flat", pady=6, cursor="hand2",
        ).pack(fill=tk.X, padx=16, pady=(4, 4))

        # Schéma de la palette de travail
        tk.Label(parent, text="Schéma (palette de travail)", font=("Segoe UI", 11, "bold"),
                 fg=t.text, bg=t.bg_alt).pack(anchor="w", **pad)
        scheme_combo = ttk.Combobox(
            parent, textvariable=self.working_scheme_var, state="readonly",
            values=[s.value for s in cu.SchemeType],
        )
        scheme_combo.pack(fill=tk.X, padx=16, pady=4)
        scheme_combo.bind("<<ComboboxSelected>>", lambda e: self.on_scheme_changed())

    # ------------------------------------------------------------------
    # Section : palette de travail (interactive, verrouillable)
    # ------------------------------------------------------------------

    def _build_working_section(self) -> None:
        t = self.theme
        parent = self.section_working
        info = tk.Label(
            parent,
            text="Double-clic sur une couleur : la modifier (verrouille automatiquement). "
                 "🔒 verrouille/déverrouille. Les couleurs verrouillées ne changent pas quand "
                 "la base ou le schéma changent.",
            font=("Segoe UI", 9, "italic"), fg=t.text_muted, bg=t.panel, wraplength=900, justify="left",
        )
        info.pack(anchor="w", pady=(0, 8))

        actions = tk.Frame(parent, bg=t.panel)
        actions.pack(anchor="w", pady=(0, 8))
        tk.Button(
            actions, text="🔄 Régénérer", command=self.on_regenerate_click,
            bg=t.panel, fg=t.text, relief="flat", padx=8, pady=4, cursor="hand2",
        ).pack(side=tk.LEFT, padx=(0, 6))
        tk.Button(
            actions, text="📋 Copier toute la palette", command=self.on_copy_working_palette,
            bg=t.panel, fg=t.text, relief="flat", padx=8, pady=4, cursor="hand2",
        ).pack(side=tk.LEFT, padx=(0, 6))
        tk.Button(
            actions, text="⭐ Enregistrer en favori", command=self.on_save_favorite,
            bg=t.panel, fg=t.text, relief="flat", padx=8, pady=4, cursor="hand2",
        ).pack(side=tk.LEFT)

        self.working_row = HScrollRow(parent, t)
        self.working_row.pack(fill=tk.X, expand=True)

    # ------------------------------------------------------------------
    # Section : toutes les harmonies (référence, visibles simultanément)
    # ------------------------------------------------------------------

    def _build_schemes_section(self) -> None:
        t = self.theme
        parent = self.section_schemes
        tk.Label(
            parent,
            text="Toutes les harmonies possibles pour la couleur de base actuelle "
                 "(clic = copier, Ctrl+clic = copier en RGB) :",
            font=("Segoe UI", 9, "italic"), fg=t.text_muted, bg=t.panel, wraplength=900, justify="left",
        ).pack(anchor="w", pady=(0, 8))
        self.scheme_rows: dict[cu.SchemeType, tuple[tk.Label, tk.Frame]] = {}
        for scheme in cu.SchemeType:
            title = tk.Label(
                parent, text=cu.SCHEME_LABELS[scheme], font=("Segoe UI", 10, "bold"),
                fg=t.text, bg=t.panel, anchor="w",
            )
            title.pack(anchor="w", pady=(6, 2))
            row = HScrollRow(parent, t)
            row.pack(fill=tk.X, expand=True, pady=(0, 4))
            self.scheme_rows[scheme] = (title, row)

    # ------------------------------------------------------------------
    # Section : contraste WCAG
    # ------------------------------------------------------------------

    def _build_contrast_section(self) -> None:
        t = self.theme
        parent = self.section_contrast
        self.contrast_list = tk.Frame(parent, bg=t.panel)
        self.contrast_list.pack(fill=tk.X, expand=True, pady=(0, 10))

        checker = tk.Frame(parent, bg=t.panel)
        checker.pack(fill=tk.X, pady=(6, 0))
        tk.Label(checker, text="Vérificateur libre :", font=("Segoe UI", 10, "bold"),
                 fg=t.text, bg=t.panel).pack(side=tk.LEFT, padx=(0, 8))

        self.contrast_role_a = tk.StringVar()
        self.contrast_role_b = tk.StringVar()
        self.contrast_combo_a = ttk.Combobox(checker, textvariable=self.contrast_role_a, state="readonly", width=18)
        self.contrast_combo_a.pack(side=tk.LEFT, padx=4)
        tk.Label(checker, text="vs", bg=t.panel, fg=t.text_muted).pack(side=tk.LEFT, padx=4)
        self.contrast_combo_b = ttk.Combobox(checker, textvariable=self.contrast_role_b, state="readonly", width=18)
        self.contrast_combo_b.pack(side=tk.LEFT, padx=4)
        tk.Button(
            checker, text="Comparer", command=self.on_check_contrast_pair,
            bg=t.accent, fg=t.accent_text, relief="flat", padx=10, pady=4, cursor="hand2",
        ).pack(side=tk.LEFT, padx=8)
        self.contrast_pair_result = tk.Label(
            checker, text="", font=("Segoe UI", 10, "bold"), bg=t.panel, fg=t.text,
        )
        self.contrast_pair_result.pack(side=tk.LEFT, padx=8)

    # ------------------------------------------------------------------
    # Section : simulation de daltonisme
    # ------------------------------------------------------------------

    def _build_colorblind_section(self) -> None:
        t = self.theme
        parent = self.section_colorblind
        self.colorblind_rows: dict[cbmod.ColorblindType, tuple[tk.Label, tk.Frame]] = {}
        for cb_type in cbmod.ColorblindType:
            title = tk.Label(
                parent, text=cbmod.CB_LABELS[cb_type], font=("Segoe UI", 10, "bold"),
                fg=t.text, bg=t.panel, anchor="w",
            )
            title.pack(anchor="w", pady=(6, 2))
            row = HScrollRow(parent, t)
            row.pack(fill=tk.X, expand=True, pady=(0, 4))
            self.colorblind_rows[cb_type] = (title, row)

    # ------------------------------------------------------------------
    # Section : aperçu UI (mockup)
    # ------------------------------------------------------------------

    def _build_mockup_section(self) -> None:
        t = self.theme
        parent = self.section_mockup
        self.mockup_frame = tk.Frame(parent, bg=t.bg, height=180, highlightbackground=t.border, highlightthickness=1)
        self.mockup_frame.pack(fill=tk.X, expand=True)
        self.mockup_frame.pack_propagate(False)

    # ------------------------------------------------------------------
    # Section : dégradé CSS
    # ------------------------------------------------------------------

    def _build_gradient_section(self) -> None:
        t = self.theme
        parent = self.section_gradient
        controls = tk.Frame(parent, bg=t.panel)
        controls.pack(fill=tk.X, pady=(0, 8))

        self.gradient_role_vars = [tk.StringVar(), tk.StringVar(), tk.StringVar()]
        self.gradient_combos: list[ttk.Combobox] = []
        for i, var in enumerate(self.gradient_role_vars):
            label = "Couleur 1" if i == 0 else ("Couleur 2" if i == 1 else "Couleur 3 (optionnel)")
            tk.Label(controls, text=label, bg=t.panel, fg=t.text_muted, font=("Segoe UI", 9)).pack(side=tk.LEFT, padx=(0, 4))
            combo = ttk.Combobox(controls, textvariable=var, state="readonly", width=16)
            combo.pack(side=tk.LEFT, padx=(0, 10))
            self.gradient_combos.append(combo)

        tk.Button(
            controls, text="Générer", command=self.on_generate_gradient,
            bg=t.accent, fg=t.accent_text, relief="flat", padx=10, pady=4, cursor="hand2",
        ).pack(side=tk.LEFT, padx=6)

        self.gradient_canvas = tk.Canvas(parent, height=60, highlightthickness=1, highlightbackground=t.border)
        self.gradient_canvas.pack(fill=tk.X, expand=True, pady=(4, 6))

        self.gradient_css_entry = tk.Entry(
            parent, font=("Consolas", 10), bg=t.entry_bg, fg=t.entry_fg, relief="flat",
        )
        self.gradient_css_entry.pack(fill=tk.X, ipady=5)
        tk.Button(
            parent, text="Copier le CSS", command=self.on_copy_gradient_css,
            bg=t.panel, fg=t.text, relief="flat", padx=8, pady=4, cursor="hand2",
        ).pack(anchor="e", pady=(6, 0))

    # ------------------------------------------------------------------
    # Section : extraction depuis une image
    # ------------------------------------------------------------------

    def _build_image_section(self) -> None:
        t = self.theme
        parent = self.section_image
        controls = tk.Frame(parent, bg=t.panel)
        controls.pack(fill=tk.X, pady=(0, 8))
        tk.Button(
            controls, text="Charger une image…", command=self.on_load_image,
            bg=t.accent, fg=t.accent_text, relief="flat", padx=10, pady=6, cursor="hand2",
        ).pack(side=tk.LEFT)
        self.image_path_lbl = tk.Label(controls, text="Aucune image chargée", bg=t.panel, fg=t.text_muted)
        self.image_path_lbl.pack(side=tk.LEFT, padx=10)

        self.image_result_row = HScrollRow(parent, t)
        self.image_result_row.pack(fill=tk.X, expand=True, pady=(4, 8))

        self.use_extracted_btn = tk.Button(
            parent, text="Utiliser comme palette de travail", command=self.on_use_extracted_palette,
            bg=t.panel, fg=t.text, relief="flat", padx=8, pady=4, cursor="hand2", state=tk.DISABLED,
        )
        self.use_extracted_btn.pack(anchor="w")
        self._extracted_colors: list[str] = []

    # ------------------------------------------------------------------
    # Section : favoris
    # ------------------------------------------------------------------

    def _build_favorites_section(self) -> None:
        t = self.theme
        parent = self.section_favorites
        tk.Label(
            parent, text="Double-clic sur un favori pour le charger.",
            font=("Segoe UI", 9, "italic"), fg=t.text_muted, bg=t.panel,
        ).pack(anchor="w", pady=(0, 6))
        list_frame = tk.Frame(parent, bg=t.panel)
        list_frame.pack(fill=tk.X, expand=True)
        self.favorites_listbox = tk.Listbox(
            list_frame, height=6, bg=t.entry_bg, fg=t.entry_fg, relief="flat",
            selectbackground=t.accent, selectforeground=t.accent_text,
        )
        self.favorites_listbox.pack(side=tk.LEFT, fill=tk.X, expand=True)
        self.favorites_listbox.bind("<Double-Button-1>", self.on_load_favorite)
        fav_scroll = tk.Scrollbar(list_frame, command=self.favorites_listbox.yview)
        fav_scroll.pack(side=tk.LEFT, fill=tk.Y)
        self.favorites_listbox.configure(yscrollcommand=fav_scroll.set)

        tk.Button(
            parent, text="🗑 Supprimer la sélection", command=self.on_delete_favorite,
            bg=t.panel, fg=t.error, relief="flat", padx=8, pady=4, cursor="hand2",
        ).pack(anchor="e", pady=(6, 0))

    # ------------------------------------------------------------------
    # Section : export
    # ------------------------------------------------------------------

    def _build_export_section(self) -> None:
        t = self.theme
        parent = self.section_export
        tk.Label(
            parent, text="Exporte la palette de travail actuelle :",
            font=("Segoe UI", 9, "italic"), fg=t.text_muted, bg=t.panel,
        ).pack(anchor="w", pady=(0, 8))
        row = tk.Frame(parent, bg=t.panel)
        row.pack(fill=tk.X)
        for fmt, label in [
            ("csv", "CSV"), ("json", "JSON"), ("css", "CSS"), ("scss", "SCSS"),
            ("txt", "TXT"), ("ase", "Adobe .ase"), ("png", "Image PNG"),
        ]:
            tk.Button(
                row, text=label, command=lambda f=fmt: self.on_export(f),
                bg=t.accent, fg=t.accent_text, relief="flat", padx=10, pady=6, cursor="hand2",
            ).pack(side=tk.LEFT, padx=4, pady=4)

    # ------------------------------------------------------------------
    # Règles de design (contenu de référence, statique)
    # ------------------------------------------------------------------

    # (titre, liste de règles) — contenu purement informatif, pas de logique.
    DESIGN_RULES: list[tuple[str, list[str]]] = [
        (
            "🧭 Harmonies chromatiques",
            [
                "Monochromatique : une seule teinte, variations de luminosité/saturation. "
                "Le plus sûr, toujours cohérent.",
                "Analogue (±30°) : teintes voisines sur le cercle. Doux, harmonieux — "
                "bon pour du contenu apaisant (santé, nature, lifestyle).",
                "Complémentaire (180°) : fort contraste. Idéal pour un accent/CTA qui doit "
                "ressortir, à utiliser avec parcimonie.",
                "Split-complémentaire (150°/210°) : contraste marqué mais moins agressif "
                "que le complémentaire pur.",
                "Triadique (120°/240°) : équilibré et vivant, demande de la rigueur pour "
                "ne pas devenir criard.",
                "Tétradique / carré (90°/180°/270°) : riche mais difficile à équilibrer — "
                "réserver aux marques créatives, une couleur doit dominer.",
                "Golden (137,5°, nombre d'or) : distribution naturelle et non répétitive, "
                "utile pour générer beaucoup de teintes sans motif visible.",
            ],
        ),
        (
            "📐 Répartition des surfaces",
            [
                "Règle 60-30-10 : 60% couleur dominante (fond/neutre), 30% secondaire "
                "(sections, cartes), 10% accent (boutons, liens, alertes).",
                "Variante interfaces denses : 90-6-3 — encore plus de neutre, l'accent "
                "reste rare donc reste efficace.",
                "Une couleur d'accent unique et cohérente sur toute l'app plutôt qu'un "
                "accent différent par écran.",
                "Limiter la palette à 2-4 couleurs principales + neutres (gris/blanc/noir). "
                "Trop de couleurs = perte de hiérarchie visuelle.",
                "Les couleurs neutres (gris) structurent 80% d'une interface ; les couleurs "
                "vives servent à guider l'œil, pas à décorer.",
            ],
        ),
        (
            "🔎 Contraste & accessibilité (WCAG)",
            [
                "Texte normal : ratio de contraste ≥ 4.5:1 (niveau AA).",
                "Grand texte (≥18pt ou ≥14pt gras) : ≥ 3:1 suffit.",
                "Niveau AAA (plus strict) : 7:1 pour le texte normal, 4.5:1 pour le grand texte.",
                "Éléments d'interface non textuels (icônes, bordures de champs) : ≥ 3:1 "
                "recommandé (WCAG 1.4.11).",
                "Ne jamais coder une information uniquement par la couleur (ex. erreur en "
                "rouge) : ajouter une icône, un libellé ou une texture.",
                "Tester le contraste sur les vraies couleurs de fond utilisées, pas "
                "seulement blanc/noir théoriques.",
            ],
        ),
        (
            "👁 Daltonisme",
            [
                "≈ 8% des hommes et 0,5% des femmes sont daltoniens (majoritairement "
                "deutéranopie/protanopie, rouge-vert).",
                "Éviter le duo rouge/vert seul pour distinguer des états (succès/erreur) : "
                "doubler avec forme, icône ou position.",
                "Le bleu/orange reste globalement bien perçu par la plupart des types de "
                "daltonisme — bon choix de secours pour un contraste fiable.",
                "Tester systématiquement en simulation protanopie, deutéranopie et "
                "tritanopie avant de valider une palette de statuts.",
            ],
        ),
        (
            "🚦 Couleurs sémantiques (conventions attendues)",
            [
                "Rouge = erreur / danger / suppression.",
                "Vert = succès / validé / disponible.",
                "Orange ou jaune = avertissement / attention.",
                "Bleu = information / lien / action neutre.",
                "Gris = désactivé / inactif.",
                "S'écarter de ces conventions perturbe l'utilisateur, même avec un joli design.",
                "Attention aux connotations culturelles (ex. le rouge = chance en Chine, "
                "danger en Occident) pour un public international.",
            ],
        ),
        (
            "🎭 Psychologie et température des couleurs",
            [
                "Couleurs chaudes (rouge, orange, jaune) : énergie, urgence, attirent "
                "l'attention en premier.",
                "Couleurs froides (bleu, vert, violet) : calme, confiance, sérieux — "
                "dominent souvent le fond des interfaces pro.",
                "Bleu : confiance, corporate, tech, santé — couleur la plus 'sûre' et "
                "la plus utilisée dans le B2B/SaaS.",
                "Vert : croissance, nature, argent (occident), validation.",
                "Noir/gris foncé : premium, élégance, sobriété.",
                "Ces associations sont des tendances, pas des lois — toujours les valider "
                "avec la cible et le secteur visés.",
            ],
        ),
        (
            "🌗 Mode sombre (dark mode)",
            [
                "Ne pas juste inverser les couleurs claires : redéfinir la palette pour "
                "le fond sombre.",
                "Éviter le noir pur (#000000) et le blanc pur (#FFFFFF) : préférer des gris "
                "très foncés/clairs pour réduire la fatigue oculaire et l'effet de halo.",
                "Désaturer légèrement les couleurs vives sur fond sombre : une couleur "
                "saturée 'vibre' davantage sur un fond noir.",
                "Vérifier à nouveau tous les contrastes WCAG en mode sombre : ils ne sont "
                "pas symétriques par rapport au mode clair.",
            ],
        ),
        (
            "🧩 États interactifs & dégradés de teinte",
            [
                "Prévoir des variantes pour chaque couleur d'action : normal, hover, "
                "active/pressed, disabled, focus.",
                "Garder les variantes dans la même teinte (ajuster seulement luminosité/"
                "saturation) pour rester cohérent visuellement.",
                "Le disabled doit toujours être clairement moins saturé/contrasté que "
                "l'état normal, sans devenir illisible.",
                "Un état 'focus' visible (contour, halo) est requis pour l'accessibilité "
                "clavier, indépendamment de la couleur de fond.",
            ],
        ),
        (
            "🧱 Cohérence & système de design",
            [
                "Définir la palette comme des 'tokens' nommés par rôle (primary, "
                "success, danger...) plutôt que par valeur brute réutilisée partout.",
                "Toute nouvelle couleur ajoutée à un écran doit se justifier — sinon "
                "réutiliser une couleur existante de la palette.",
                "Tester la palette sur de vrais écrans/maquettes (pas seulement des "
                "swatches isolés) : une couleur change de perception selon son voisinage.",
                "Vérifier la palette à la fois en petite taille (icônes, mobile) et en "
                "grande taille (bannières, fonds).",
            ],
        ),
        (
            "✅ Check-list rapide avant de valider une palette",
            [
                "Le contraste texte/fond passe le niveau AA minimum partout.",
                "La palette reste lisible en simulation daltonisme.",
                "Une seule couleur dominante est clairement identifiable (règle 60-30-10).",
                "Les couleurs sémantiques (erreur/succès/avertissement) suivent les "
                "conventions attendues.",
                "La palette fonctionne en mode clair ET en mode sombre.",
                "Pas plus de 2 à 4 couleurs principales, hors neutres.",
            ],
        ),
    ]

    def _build_rules_section(self) -> None:
        t = self.theme
        parent = self.section_rules

        tk.Label(
            parent,
            text=(
                "Aide-mémoire des règles de design colorimétrique les plus courantes — "
                "purement informatif, cliquez un titre pour déplier/replier."
            ),
            font=("Segoe UI", 9, "italic"), fg=t.text_muted, bg=t.panel,
            anchor="w", justify="left", wraplength=760,
        ).pack(anchor="w", fill=tk.X, pady=(0, 10))

        for title, rules in self.DESIGN_RULES:
            block = tk.Frame(parent, bg=t.panel)
            block.pack(anchor="w", fill=tk.X, pady=(0, 4))

            body = tk.Frame(block, bg=t.bg_alt)
            expanded = {"open": True}

            def toggle(b=body, e=expanded, btn_holder={}):
                if e["open"]:
                    b.pack_forget()
                    e["open"] = False
                else:
                    b.pack(anchor="w", fill=tk.X, padx=(14, 4), pady=(2, 8))
                    e["open"] = True

            header_btn = tk.Button(
                block, text=f"▾  {title}", command=toggle,
                font=("Segoe UI", 10, "bold"), fg=t.accent, bg=t.panel,
                anchor="w", relief="flat", cursor="hand2", padx=4, pady=4,
            )
            header_btn.pack(anchor="w", fill=tk.X)

            for rule in rules:
                tk.Label(
                    body, text=f"•  {rule}", font=("Segoe UI", 9), fg=t.text,
                    bg=t.bg_alt, anchor="w", justify="left", wraplength=720,
                ).pack(anchor="w", fill=tk.X, pady=2)

            body.pack(anchor="w", fill=tk.X, padx=(14, 4), pady=(2, 8))

    # ==================================================================
    # Construction / mise à jour des swatches
    # ==================================================================

    def _build_swatch(
        self, parent: tk.Frame, role: str, hex_code: str,
        editable: bool = False, index: int | None = None,
    ) -> tk.Frame:
        t = self.theme
        box = tk.Frame(parent, bg=hex_code, cursor="hand2", width=128, height=90)
        box.pack(side=tk.LEFT, fill=tk.Y, padx=2)
        box.pack_propagate(False)

        text_color = cu.readable_text_color(hex_code)

        top_row = tk.Frame(box, bg=hex_code)
        top_row.pack(side=tk.TOP, fill=tk.X)

        if editable:
            is_locked = index in self.locked_indices
            lock_lbl = tk.Label(
                top_row, text="🔒" if is_locked else "🔓", bg=hex_code, fg=text_color,
                font=("Segoe UI", 10), cursor="hand2",
            )
            lock_lbl.pack(side=tk.RIGHT, padx=4, pady=2)
            lock_lbl.bind("<Button-1>", lambda e, i=index: self.on_toggle_lock(i))

        lbl_name = tk.Label(
            box, text=role, bg=hex_code, fg=text_color, font=("Segoe UI", 9, "bold"),
            wraplength=100, cursor="hand2",
        )
        lbl_name.pack(expand=True)

        lbl_hex = tk.Label(
            box, text=cu.format_color(hex_code, self.format_var.get()), bg=hex_code, fg=text_color,
            font=("Segoe UI", 9), cursor="hand2",
        )
        lbl_hex.pack(expand=True)
        self._label_registry.append((lbl_hex, hex_code))

        elements = [box, lbl_name, lbl_hex]
        for element in elements:
            element.bind("<Button-1>", lambda e, code=hex_code: self.copy_to_clipboard(code, cu.ColorFormat(self.format_var.get())))
            element.bind("<Control-Button-1>", lambda e, code=hex_code: self.copy_to_clipboard(code, cu.ColorFormat.RGB))
            if editable and index is not None:
                element.bind("<Double-Button-1>", lambda e, i=index: self.on_edit_swatch(i))

        Tooltip(box, _role_description(role), t)
        return box

    def _clear_row(self, row) -> None:
        if isinstance(row, HScrollRow):
            row.clear()
        else:
            for widget in row.winfo_children():
                widget.destroy()

    @staticmethod
    def _row_target(row) -> tk.Frame:
        """Retourne le conteneur dans lequel packer des swatches (gère HScrollRow)."""
        return row.inner if isinstance(row, HScrollRow) else row

    # ==================================================================
    # Rafraîchissement des sections
    # ==================================================================

    def refresh_all_sections(self) -> None:
        self._label_registry.clear()
        self.refresh_working_section()
        self.refresh_schemes_section()
        self.refresh_contrast_section()
        self.refresh_colorblind_section()
        self.refresh_mockup_section()
        self.refresh_gradient_controls()
        self.refresh_favorites_section()
        self.refresh_history_row()
        self._update_undo_redo_buttons()

    def refresh_working_section(self) -> None:
        self._clear_row(self.working_row)
        for i, swatch in enumerate(self.working_palette):
            self._build_swatch(self.working_row.inner, swatch.role, swatch.hex, editable=True, index=i)

    def refresh_schemes_section(self) -> None:
        all_schemes = cu.generate_all_schemes(self.base_hex)
        for scheme, (title, row) in self.scheme_rows.items():
            self._clear_row(row)
            for swatch in all_schemes[scheme]:
                self._build_swatch(row.inner, swatch.role, swatch.hex, editable=False)

    def refresh_contrast_section(self) -> None:
        t = self.theme
        for widget in self.contrast_list.winfo_children():
            widget.destroy()

        header = tk.Frame(self.contrast_list, bg=t.panel)
        header.pack(fill=tk.X)
        for text, width in [("Couleur", 24), ("vs Blanc", 18), ("vs Noir", 18)]:
            tk.Label(header, text=text, font=("Segoe UI", 9, "bold"), fg=t.text_muted,
                     bg=t.panel, width=width, anchor="w").pack(side=tk.LEFT)

        for swatch in self.working_palette:
            row = tk.Frame(self.contrast_list, bg=t.panel)
            row.pack(fill=tk.X, pady=2)
            chip = tk.Label(row, text=f"  {swatch.role}", bg=swatch.hex,
                             fg=cu.readable_text_color(swatch.hex), width=24, anchor="w", font=("Segoe UI", 9))
            chip.pack(side=tk.LEFT, ipady=4)

            for bg_ref in ("#FFFFFF", "#000000"):
                result = ctr.evaluate_contrast(swatch.hex, bg_ref)
                badge_color = t.success if result.aa_normal else (t.accent if result.aa_large else t.error)
                lbl = tk.Label(
                    row, text=f"{result.ratio:.2f}:1  {result.badge}", bg=t.panel, fg=badge_color,
                    width=18, anchor="w", font=("Segoe UI", 9, "bold"),
                )
                lbl.pack(side=tk.LEFT)

        roles = [s.role for s in self.working_palette]
        self.contrast_combo_a.configure(values=roles)
        self.contrast_combo_b.configure(values=roles)
        if roles:
            if self.contrast_role_a.get() not in roles:
                self.contrast_role_a.set(roles[0])
            if self.contrast_role_b.get() not in roles:
                self.contrast_role_b.set(roles[-1])

    def refresh_colorblind_section(self) -> None:
        for cb_type, (title, row) in self.colorblind_rows.items():
            self._clear_row(row)
            for swatch in self.working_palette:
                simulated_hex = cbmod.simulate(swatch.hex, cb_type)
                self._build_swatch(row, swatch.role, simulated_hex, editable=False)

    def refresh_mockup_section(self) -> None:
        t = self.theme
        for widget in self.mockup_frame.winfo_children():
            widget.destroy()

        by_role = {s.role: s.hex for s in self.working_palette}
        accent = by_role.get("Base", self.working_palette[0].hex if self.working_palette else "#F1C40F")
        bg = next((h for r, h in by_role.items() if "Fond Neutre" in r), "#F4F3F1")
        fg = next((h for r, h in by_role.items() if "Texte Neutre" in r), "#2B2921")
        card_bg = next((h for r, h in by_role.items() if "+50%" in r), "#FFFFFF")

        self.mockup_frame.configure(bg=bg)
        wrap = tk.Frame(self.mockup_frame, bg=bg, padx=20, pady=16)
        wrap.pack(fill=tk.BOTH, expand=True)

        tk.Label(wrap, text="Aperçu d'interface", font=("Segoe UI", 12, "bold"), bg=bg, fg=fg).pack(anchor="w")
        card = tk.Frame(wrap, bg=card_bg, highlightbackground=t.border, highlightthickness=1, padx=14, pady=14)
        card.pack(anchor="w", pady=10, fill=tk.X)
        tk.Label(card, text="Titre de carte", font=("Segoe UI", 11, "bold"), bg=card_bg, fg=cu.readable_text_color(card_bg)).pack(anchor="w")
        tk.Label(
            card, text="Texte d'exemple utilisant les couleurs de la palette de travail.",
            font=("Segoe UI", 9), bg=card_bg, fg=cu.readable_text_color(card_bg), wraplength=500, justify="left",
        ).pack(anchor="w", pady=(4, 10))
        tk.Button(
            card, text="Bouton d'action", bg=accent, fg=cu.readable_text_color(accent),
            relief="flat", padx=14, pady=6,
        ).pack(anchor="w")

    def refresh_gradient_controls(self) -> None:
        roles = [s.role for s in self.working_palette]
        for combo in self.gradient_combos:
            combo.configure(values=[""] + roles)
        if roles:
            if not self.gradient_role_vars[0].get():
                self.gradient_role_vars[0].set(roles[0])
            if not self.gradient_role_vars[1].get():
                self.gradient_role_vars[1].set(roles[min(1, len(roles) - 1)])

    def refresh_favorites_section(self) -> None:
        self.favorites_listbox.delete(0, tk.END)
        for fav in self.favorites_store.load():
            self.favorites_listbox.insert(tk.END, f"{fav['name']}  ({fav['base_hex']})")

    def refresh_history_row(self) -> None:
        self._clear_row(self.history_row)
        t = self.theme
        for hex_code in self.history_store.load():
            chip = tk.Label(
                self.history_row, bg=hex_code, width=2, height=1, cursor="hand2",
                highlightbackground=t.border, highlightthickness=1,
            )
            chip.pack(side=tk.LEFT, padx=2, pady=2, ipady=6, ipadx=6)
            chip.bind("<Button-1>", lambda e, h=hex_code: self.set_base_color(h))
            Tooltip(chip, hex_code, t)

    def _update_undo_redo_buttons(self) -> None:
        self.undo_btn.configure(state=tk.NORMAL if self.undo_stack else tk.DISABLED)
        self.redo_btn.configure(state=tk.NORMAL if self.redo_stack else tk.DISABLED)

    # ==================================================================
    # Logique métier / callbacks
    # ==================================================================

    def set_status(self, text: str, kind: str = "info") -> None:
        t = self.theme
        color = {"info": t.text_muted, "success": t.success, "error": t.error}.get(kind, t.text_muted)
        self.status_lbl.configure(text=text, fg=color)

    def copy_to_clipboard(self, hex_code: str, fmt: cu.ColorFormat = cu.ColorFormat.HEX) -> None:
        text = cu.format_color(hex_code, fmt)
        try:
            self.root.clipboard_clear()
            self.root.clipboard_append(text)
            self.root.update()
            self.set_status(f"✓ {text} copié dans le presse-papier.", "success")
        except tk.TclError as exc:
            self.set_status(f"✗ Impossible de copier dans le presse-papier : {exc}", "error")

    def set_base_color(self, hex_code: str, push_history: bool = True, record_undo: bool = True) -> None:
        try:
            hex_code = cu.normalize_hex(hex_code)
        except ValueError as exc:
            self.set_status(f"✗ {exc}", "error")
            return

        if record_undo and hex_code != self.base_hex:
            self.undo_stack.append(self.base_hex)
            self.redo_stack.clear()

        self.base_hex = hex_code
        self.hex_var.set(hex_code)
        if push_history:
            self.history_store.add(hex_code)
        self.session.update(base_hex=hex_code)
        self.regenerate_working_palette(push_history=False)
        self.refresh_all_sections()

    def regenerate_working_palette(self, force: bool = False, push_history: bool = True) -> None:
        try:
            scheme = cu.SchemeType(self.working_scheme_var.get())
        except ValueError:
            scheme = cu.SchemeType.GOLDEN
            self.working_scheme_var.set(scheme.value)

        new_swatches = cu.generate_palette(self.base_hex, scheme)

        if force or len(new_swatches) != len(self.working_palette):
            if len(new_swatches) != len(self.working_palette):
                self.locked_indices.clear()
            self.working_palette = new_swatches
            if force:
                # Le bouton "Régénérer" garde les couleurs verrouillées, recalcule le reste.
                self.working_palette = [
                    self.working_palette[i] if i not in self.locked_indices else self.working_palette[i]
                    for i in range(len(new_swatches))
                ]
        else:
            self.working_palette = [
                self.working_palette[i] if i in self.locked_indices else new_swatches[i]
                for i in range(len(new_swatches))
            ]

        self.session.update(last_scheme=scheme.value)

    # -- couleur de base --------------------------------------------------

    def on_pick_color(self) -> None:
        color = colorchooser.askcolor(title="Choisissez une couleur de base", color=self.base_hex)
        if color[1]:
            self.set_base_color(color[1])

    def on_apply_hex_entry(self, _event=None) -> None:
        self.set_base_color(self.hex_var.get())

    def on_apply_name_entry(self, _event=None) -> None:
        name = self.name_var.get().strip()
        if not name:
            return
        hex_code = cnames.lookup_color_name(name)
        if hex_code is None:
            self.set_status(f"✗ Couleur inconnue : « {name} ».", "error")
            return
        self.set_base_color(hex_code)
        self.set_status(f"✓ « {name} » → {hex_code}", "success")

    def on_random_color(self) -> None:
        self.set_base_color(cu.random_hex())

    def on_apply_sliders(self) -> None:
        sat_delta = self.sat_slider_var.get()
        light_delta = self.light_slider_var.get()
        if sat_delta == 0 and light_delta == 0:
            self.set_status("Ajustez d'abord les curseurs de saturation/luminosité.", "info")
            return
        new_hex = cu.apply_global_adjustment(self.base_hex, sat_delta, light_delta)
        self.set_base_color(new_hex)
        self.sat_slider_var.set(0.0)
        self.light_slider_var.set(0.0)

    def on_regenerate_click(self) -> None:
        self.regenerate_working_palette(force=True)
        self.refresh_all_sections()

    def on_scheme_changed(self) -> None:
        self.locked_indices.clear()
        self.regenerate_working_palette(force=True)
        self.refresh_all_sections()

    def on_format_changed(self) -> None:
        fmt = self.format_var.get()
        for label, hex_code in self._label_registry:
            try:
                label.configure(text=cu.format_color(hex_code, fmt))
            except tk.TclError:
                pass  # le widget a pu être détruit lors d'un rafraîchissement entre-temps

    # -- undo / redo --------------------------------------------------

    def undo(self) -> None:
        if not self.undo_stack:
            return
        previous = self.undo_stack.pop()
        self.redo_stack.append(self.base_hex)
        self.set_base_color(previous, push_history=False, record_undo=False)

    def redo(self) -> None:
        if not self.redo_stack:
            return
        nxt = self.redo_stack.pop()
        self.undo_stack.append(self.base_hex)
        self.set_base_color(nxt, push_history=False, record_undo=False)

    # -- thème --------------------------------------------------

    def toggle_theme(self) -> None:
        self.theme_name = "light" if self.theme_name == "dark" else "dark"
        self.theme = get_theme(self.theme_name)
        self.session.update(theme=self.theme_name)
        self.set_status("Redémarrez l'application pour appliquer entièrement le nouveau thème.", "info")
        messagebox.showinfo(
            "Thème",
            "Le thème a été enregistré. Fermez puis rouvrez GoldenHue pour l'appliquer complètement.",
        )

    # -- verrouillage / édition manuelle --------------------------------

    def on_toggle_lock(self, index: int) -> None:
        if index in self.locked_indices:
            self.locked_indices.discard(index)
        else:
            self.locked_indices.add(index)
        self.refresh_working_section()

    def on_edit_swatch(self, index: int) -> None:
        if not (0 <= index < len(self.working_palette)):
            return
        current = self.working_palette[index]
        color = colorchooser.askcolor(title=f"Modifier « {current.role} »", color=current.hex)
        if color[1]:
            self.working_palette[index] = cu.SwatchSpec(current.role, cu.normalize_hex(color[1]))
            self.locked_indices.add(index)
            self.refresh_all_sections()

    # -- palette de travail : actions globales --------------------------

    def on_copy_working_palette(self) -> None:
        fmt = self.format_var.get()
        text = "\n".join(f"{s.role}: {cu.format_color(s.hex, fmt)}" for s in self.working_palette)
        try:
            self.root.clipboard_clear()
            self.root.clipboard_append(text)
            self.root.update()
            self.set_status("✓ Palette complète copiée dans le presse-papier.", "success")
        except tk.TclError as exc:
            self.set_status(f"✗ Échec de la copie : {exc}", "error")

    def on_save_favorite(self) -> None:
        name = simpledialog.askstring("Enregistrer en favori", "Nom de la palette :", parent=self.root)
        if not name:
            return
        colors = [{"role": s.role, "hex": s.hex} for s in self.working_palette]
        self.favorites_store.save_palette(name, self.base_hex, self.working_scheme_var.get(), colors)
        self.refresh_favorites_section()
        self.set_status(f"✓ Palette « {name} » enregistrée dans les favoris.", "success")

    def on_load_favorite(self, _event=None) -> None:
        selection = self.favorites_listbox.curselection()
        if not selection:
            return
        fav = self.favorites_store.load()[selection[0]]
        self.working_scheme_var.set(fav.get("scheme", cu.SchemeType.GOLDEN.value))
        self.working_palette = [cu.SwatchSpec(c["role"], c["hex"]) for c in fav["colors"]]
        self.locked_indices = set(range(len(self.working_palette)))
        self.set_base_color(fav["base_hex"], push_history=True)
        self.set_status(f"✓ Favori « {fav['name']} » chargé.", "success")

    def on_delete_favorite(self) -> None:
        selection = self.favorites_listbox.curselection()
        if not selection:
            return
        self.favorites_store.delete(selection[0])
        self.refresh_favorites_section()

    # -- contraste --------------------------------------------------

    def on_check_contrast_pair(self) -> None:
        by_role = {s.role: s.hex for s in self.working_palette}
        hex_a = by_role.get(self.contrast_role_a.get())
        hex_b = by_role.get(self.contrast_role_b.get())
        if not hex_a or not hex_b:
            return
        result = ctr.evaluate_contrast(hex_a, hex_b)
        t = self.theme
        color = t.success if result.aa_normal else (t.accent if result.aa_large else t.error)
        self.contrast_pair_result.configure(text=f"{result.ratio:.2f}:1 — {result.badge}", fg=color)

    # -- dégradé --------------------------------------------------

    def on_generate_gradient(self) -> None:
        by_role = {s.role: s.hex for s in self.working_palette}
        chosen = [by_role[v.get()] for v in self.gradient_role_vars if v.get() and v.get() in by_role]
        if len(chosen) < 2:
            self.set_status("Choisissez au moins 2 couleurs pour le dégradé.", "error")
            return

        self._draw_gradient(chosen)
        css = f"linear-gradient(90deg, {', '.join(c.upper() for c in chosen)})"
        self.gradient_css_entry.delete(0, tk.END)
        self.gradient_css_entry.insert(0, f"background: {css};")

    def _draw_gradient(self, colors: list[str]) -> None:
        self.gradient_canvas.delete("all")
        width = max(self.gradient_canvas.winfo_width(), 400)
        height = 60
        segments = len(colors) - 1
        rgb_colors = [cu.hex_to_rgb256(c) for c in colors]

        for x in range(width):
            pos = x / max(1, width - 1) * segments
            seg = min(int(pos), segments - 1)
            local_t = pos - seg
            r0, g0, b0 = rgb_colors[seg]
            r1, g1, b1 = rgb_colors[seg + 1]
            r = round(r0 + (r1 - r0) * local_t)
            g = round(g0 + (g1 - g0) * local_t)
            b = round(b0 + (b1 - b0) * local_t)
            self.gradient_canvas.create_line(x, 0, x, height, fill=cu.rgb256_to_hex(r, g, b))

    def on_copy_gradient_css(self) -> None:
        text = self.gradient_css_entry.get()
        if not text:
            return
        try:
            self.root.clipboard_clear()
            self.root.clipboard_append(text)
            self.root.update()
            self.set_status("✓ CSS du dégradé copié.", "success")
        except tk.TclError as exc:
            self.set_status(f"✗ Échec de la copie : {exc}", "error")

    # -- extraction d'image --------------------------------------------------

    def on_load_image(self) -> None:
        path = filedialog.askopenfilename(
            title="Choisir une image",
            filetypes=[("Images", "*.png *.jpg *.jpeg *.bmp *.gif *.webp"), ("Tous les fichiers", "*.*")],
        )
        if not path:
            return
        try:
            colors = imgx.extract_dominant_colors(path, n_colors=6)
        except imgx.ImageExtractionError as exc:
            self.set_status(f"✗ {exc}", "error")
            return
        except Exception as exc:  # défense en profondeur : fichier corrompu, etc.
            self.set_status(f"✗ Impossible de lire l'image : {exc}", "error")
            return

        self._extracted_colors = colors
        self.image_path_lbl.configure(text=os.path.basename(path))
        self._clear_row(self.image_result_row)
        for i, hex_code in enumerate(colors):
            self._build_swatch(self.image_result_row.inner, f"Dominante {i + 1}", hex_code, editable=False)
        self.use_extracted_btn.configure(state=tk.NORMAL)
        self.set_status(f"✓ {len(colors)} couleurs dominantes extraites.", "success")

    def on_use_extracted_palette(self) -> None:
        if not self._extracted_colors:
            return
        self.working_palette = [
            cu.SwatchSpec(f"Dominante {i + 1}", h) for i, h in enumerate(self._extracted_colors)
        ]
        self.locked_indices = set(range(len(self.working_palette)))
        self.working_scheme_var.set(cu.SchemeType.GOLDEN.value)  # valeur neutre par défaut
        self.set_base_color(self._extracted_colors[0], push_history=True)
        self.set_status("✓ Palette de travail remplacée par les couleurs extraites de l'image.", "success")

    # -- export --------------------------------------------------

    def on_export(self, fmt: str) -> None:
        if not self.working_palette:
            return
        extensions = {
            "csv": ".csv", "json": ".json", "css": ".css", "scss": ".scss",
            "txt": ".txt", "ase": ".ase", "png": ".png",
        }
        ext = extensions[fmt]
        path = filedialog.asksaveasfilename(
            title="Exporter la palette", defaultextension=ext,
            initialfile=f"goldenhue_palette{ext}",
            filetypes=[(fmt.upper(), f"*{ext}"), ("Tous les fichiers", "*.*")],
        )
        if not path:
            return
        manager = exu.ExportManager([(s.role, s.hex) for s in self.working_palette], palette_name="GoldenHue")
        try:
            manager.save(path, fmt)
            self.set_status(f"✓ Palette exportée : {path}", "success")
        except Exception as exc:
            self.set_status(f"✗ Échec de l'export {fmt.upper()} : {exc}", "error")
