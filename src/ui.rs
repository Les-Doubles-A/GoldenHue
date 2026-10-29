//! Interface graphique matérielle (`eframe`/`egui`) de GoldenHue.
//!
//! Optimisations architecturales :
//! - Mémoïsation complète (`ComputedCache`) des 7 harmonies, des contrastes WCAG et des simulations
//!   de daltonisme : aucun recalcul tant que la couleur de base ou la palette de travail ne change pas.
//! - Dégradé CSS rendu directement sur le GPU via un `egui::Mesh` interpolé en 1 seul appel de dessin
//!   (au lieu de 400 lignes CPU comme sous Tkinter).
//! - Bascule de thème clair/sombre instantanée sans redémarrage.

use crate::color_names::lookup_color_name;
use crate::color_utils::{
    self as cu, ColorFormat, NEUTRAL_ROLE_BG, NEUTRAL_ROLE_TEXT, SchemeType, SwatchSpec,
};
use crate::colorblind::{self as cbmod, ColorblindType};
use crate::contrast_utils::{self as ctr, ContrastResult};
use crate::export_utils::ExportManager;
use crate::image_extract::extract_dominant_colors;
use crate::storage::{FavoriteEntry, FavoritesStore, HistoryStore, SessionData, SessionStore};
use crate::theme::{Theme, get_theme};
use crate::updater::{
    self, CURRENT_VERSION, SelfUpdateOutcome, UpdateCheckOutcome, UpdateInfo,
};
use egui::{
    Align, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Frame, Layout, Margin, Mesh,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

pub const APP_TITLE: &str = "GoldenHue - Générateur de Palette";

const DESIGN_RULES: &[(&str, &[&str])] = &[
    (
        "🧭 Harmonies chromatiques",
        &[
            "Monochromatique : une seule teinte, variations de luminosité/saturation. Le plus sûr, toujours cohérent.",
            "Analogue (±30°) : teintes voisines sur le cercle. Doux, harmonieux — bon pour du contenu apaisant (santé, nature, lifestyle).",
            "Complémentaire (180°) : fort contraste. Idéal pour un accent/CTA qui doit ressortir, à utiliser avec parcimonie.",
            "Split-complémentaire (150°/210°) : contraste marqué mais moins agressif que le complémentaire pur.",
            "Triadique (120°/240°) : équilibré et vivant, demande de la rigueur pour ne pas devenir criard.",
            "Tétradique / carré (90°/180°/270°) : riche mais difficile à équilibrer — réserver aux marques créatives, une couleur doit dominer.",
            "Golden (137,5°, nombre d'or) : distribution naturelle et non répétitive, utile pour générer beaucoup de teintes sans motif visible.",
        ],
    ),
    (
        "📐 Répartition des surfaces",
        &[
            "Règle 60-30-10 : 60% couleur dominante (fond/neutre), 30% secondaire (sections, cartes), 10% accent (boutons, liens, alertes).",
            "Variante interfaces denses : 90-6-3 — encore plus de neutre, l'accent reste rare donc reste efficace.",
            "Une couleur d'accent unique et cohérente sur toute l'app plutôt qu'un accent différent par écran.",
            "Limiter la palette à 2-4 couleurs principales + neutres (gris/blanc/noir). Trop de couleurs = perte de hiérarchie visuelle.",
            "Les couleurs neutres (gris) structurent 80% d'une interface ; les couleurs vives servent à guider l'œil, pas à décorer.",
        ],
    ),
    (
        "🔎 Contraste & accessibilité (WCAG)",
        &[
            "Texte normal : ratio de contraste ≥ 4.5:1 (niveau AA).",
            "Grand texte (≥18pt ou ≥14pt gras) : ≥ 3:1 suffit.",
            "Niveau AAA (plus strict) : 7:1 pour le texte normal, 4.5:1 pour le grand texte.",
            "Éléments d'interface non textuels (icônes, bordures de champs) : ≥ 3:1 recommandé (WCAG 1.4.11).",
            "Ne jamais coder une information uniquement par la couleur (ex. erreur en rouge) : ajouter une icône, un libellé ou une texture.",
            "Tester le contraste sur les vraies couleurs de fond utilisées, pas seulement blanc/noir théoriques.",
        ],
    ),
    (
        "👁 Daltonisme",
        &[
            "≈ 8% des hommes et 0,5% des femmes sont daltoniens (majoritairement deutéranopie/protanopie, rouge-vert).",
            "Éviter le duo rouge/vert seul pour distinguer des états (succès/erreur) : doubler avec forme, icône ou position.",
            "Le bleu/orange reste globalement bien perçu par la plupart des types de daltonisme — bon choix de secours pour un contraste fiable.",
            "Tester systématiquement en simulation protanopie, deutéranopie et tritanopie avant de valider une palette de statuts.",
        ],
    ),
    (
        "🚦 Couleurs sémantiques (conventions attendues)",
        &[
            "Rouge = erreur / danger / suppression.",
            "Vert = succès / validé / disponible.",
            "Orange ou jaune = avertissement / attention.",
            "Bleu = information / lien / action neutre.",
            "Gris = désactivé / inactif.",
            "S'écarter de ces conventions perturbe l'utilisateur, même avec un joli design.",
            "Attention aux connotations culturelles (ex. le rouge = chance en Chine, danger en Occident) pour un public international.",
        ],
    ),
    (
        "🎭 Psychologie et température des couleurs",
        &[
            "Couleurs chaudes (rouge, orange, jaune) : énergie, urgence, attirent l'attention en premier.",
            "Couleurs froides (bleu, vert, violet) : calme, confiance, sérieux — dominent souvent le fond des interfaces pro.",
            "Bleu : confiance, corporate, tech, santé — couleur la plus 'sûre' et la plus utilisée dans le B2B/SaaS.",
            "Vert : croissance, nature, argent (occident), validation.",
            "Noir/gris foncé : premium, élégance, sobriété.",
            "Ces associations sont des tendances, pas des lois — toujours les valider avec la cible et le secteur visés.",
        ],
    ),
    (
        "🌗 Mode sombre (dark mode)",
        &[
            "Ne pas juste inverser les couleurs claires : redéfinir la palette pour le fond sombre.",
            "Éviter le noir pur (#000000) et le blanc pur (#FFFFFF) : préférer des gris très foncés/clairs pour réduire la fatigue oculaire et l'effet de halo.",
            "Désaturer légèrement les couleurs vives sur fond sombre : une couleur saturée 'vibre' davantage sur un fond noir.",
            "Vérifier à nouveau tous les contrastes WCAG en mode sombre : ils ne sont pas symétriques par rapport au mode clair.",
        ],
    ),
    (
        "🧩 États interactifs & dégradés de teinte",
        &[
            "Prévoir des variantes pour chaque couleur d'action : normal, hover, active/pressed, disabled, focus.",
            "Garder les variantes dans la même teinte (ajuster seulement luminosité/saturation) pour rester cohérent visuellement.",
            "Le disabled doit toujours être clairement moins saturé/contrasté que l'état normal, sans devenir illisible.",
            "Un état 'focus' visible (contour, halo) est requis pour l'accessibilité clavier, indépendamment de la couleur de fond.",
        ],
    ),
    (
        "🧱 Cohérence & système de design",
        &[
            "Définir la palette comme des 'tokens' nommés par rôle (primary, success, danger...) plutôt que par valeur brute réutilisée partout.",
            "Toute nouvelle couleur ajoutée à un écran doit se justifier — sinon réutiliser une couleur existante de la palette.",
            "Tester la palette sur de vrais écrans/maquettes (pas seulement des swatches isolés) : une couleur change de perception selon son voisinage.",
            "Vérifier la palette à la fois en petite taille (icônes, mobile) et en grande taille (bannières, fonds).",
        ],
    ),
    (
        "✅ Check-list rapide avant de valider une palette",
        &[
            "Le contraste texte/fond passe le niveau AA minimum partout.",
            "La palette reste lisible en simulation daltonisme.",
            "Une seule couleur dominante est clairement identifiable (règle 60-30-10).",
            "Les couleurs sémantiques (erreur/succès/avertissement) suivent les conventions attendues.",
            "La palette fonctionne en mode clair ET en mode sombre.",
            "Pas plus de 2 à 4 couleurs principales, hors neutres.",
        ],
    ),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusKind {
    Info,
    Success,
    Error,
}

/// Cache mémoïsé évitant tout recalcul redondant pendant la boucle de rendu 60 FPS.
struct ComputedCache {
    all_schemes: Vec<(SchemeType, Vec<SwatchSpec>)>,
    contrasts_bw: Vec<(ContrastResult, ContrastResult)>,
    colorblind_rows: Vec<(ColorblindType, Vec<SwatchSpec>)>,
}

impl ComputedCache {
    fn new(base_hex: &str, working_palette: &[SwatchSpec]) -> Self {
        let mut cache = Self {
            all_schemes: Vec::new(),
            contrasts_bw: Vec::new(),
            colorblind_rows: Vec::new(),
        };
        cache.recompute_schemes(base_hex);
        cache.recompute_working_dependents(working_palette);
        cache
    }

    fn recompute_schemes(&mut self, base_hex: &str) {
        self.all_schemes = cu::generate_all_schemes(base_hex);
    }

    fn recompute_working_dependents(&mut self, working_palette: &[SwatchSpec]) {
        self.contrasts_bw = working_palette
            .iter()
            .map(|s| {
                (
                    ctr::evaluate_contrast(&s.hex, "#FFFFFF"),
                    ctr::evaluate_contrast(&s.hex, "#000000"),
                )
            })
            .collect();

        self.colorblind_rows = ColorblindType::ALL
            .iter()
            .map(|&cb| {
                let row = working_palette
                    .iter()
                    .map(|s| SwatchSpec::new(s.role.clone(), cbmod::simulate(&s.hex, cb)))
                    .collect();
                (cb, row)
            })
            .collect();
    }
}

#[inline]
fn hex_to_color32(hex: &str) -> Color32 {
    let (r, g, b) = cu::hex_to_rgb256(hex);
    Color32::from_rgb(r, g, b)
}

#[inline]
fn readable_color32(hex: &str) -> Color32 {
    let (r, g, b) = cu::readable_text_rgb(hex);
    Color32::from_rgb(r, g, b)
}

fn section_frame<R>(ui: &mut Ui, t: Theme, title: &str, add_contents: impl FnOnce(&mut Ui) -> R) {
    Frame::new()
        .fill(t.panel)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(16.0).strong().color(t.accent));
            ui.add_space(6.0);
            add_contents(ui)
        });
    ui.add_space(10.0);
}

pub struct PaletteApp {
    session_store: SessionStore,
    history_store: HistoryStore,
    favorites_store: FavoritesStore,
    session_data: SessionData,

    theme: Theme,
    theme_dirty: bool,

    base_hex: String,
    hex_input: String,
    name_input: String,
    color_format: ColorFormat,
    working_scheme: SchemeType,
    sat_slider: f64,
    light_slider: f64,

    working_palette: Vec<SwatchSpec>,
    locked_indices: HashSet<usize>,
    undo_stack: Vec<String>,
    redo_stack: Vec<String>,

    history: Vec<String>,
    favorites: Vec<FavoriteEntry>,
    favorite_name_input: String,
    selected_favorite: Option<usize>,

    contrast_idx_a: usize,
    contrast_idx_b: usize,
    contrast_pair_result: Option<(f64, &'static str, Color32)>,

    gradient_idx_1: usize,
    gradient_idx_2: usize,
    gradient_idx_3: Option<usize>,
    gradient_colors: Vec<String>,
    gradient_css: String,

    extracted_image_name: String,
    extracted_colors: Vec<String>,

    status_text: String,
    status_kind: StatusKind,

    clipboard: Option<arboard::Clipboard>,
    cache: ComputedCache,

    update_check_rx: Option<Receiver<UpdateCheckOutcome>>,
    update_check_manual: bool,
    available_update: Option<UpdateInfo>,
    self_update_rx: Option<Receiver<SelfUpdateOutcome>>,
    is_updating: bool,
}

impl PaletteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let session_store = SessionStore::default();
        let history_store = HistoryStore::default();
        let favorites_store = FavoritesStore::default();

        let mut session_data = session_store.load();
        let base_hex = cu::normalize_hex(&session_data.base_hex).unwrap_or_else(|_| "#F1C40F".to_string());
        session_data.base_hex = base_hex.clone();

        let theme = get_theme(&session_data.theme);
        theme.apply_to_ctx(&cc.egui_ctx);

        let working_scheme = SchemeType::from_id(&session_data.last_scheme);
        let working_palette = cu::generate_palette(&base_hex, working_scheme);
        let last_idx = working_palette.len().saturating_sub(1);
        let second_idx = 1.min(last_idx);

        let gradient_colors = if working_palette.len() >= 2 {
            vec![working_palette[0].hex.clone(), working_palette[second_idx].hex.clone()]
        } else {
            vec![base_hex.clone(), base_hex.clone()]
        };
        let gradient_css = format!(
            "background: linear-gradient(90deg, {});",
            gradient_colors.join(", ")
        );

        let cache = ComputedCache::new(&base_hex, &working_palette);
        let history = history_store.load();
        let favorites = favorites_store.load();

        updater::cleanup_old_executable();
        let update_check_rx = Some(updater::spawn_update_check(cc.egui_ctx.clone()));

        Self {
            session_store,
            history_store,
            favorites_store,
            session_data,
            theme,
            theme_dirty: false,
             hex_input: base_hex.clone(),
            base_hex,
            name_input: String::new(),
            color_format: ColorFormat::HEX,
            working_scheme,
            sat_slider: 0.0,
            light_slider: 0.0,
            working_palette,
            locked_indices: HashSet::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history,
            favorites,
            favorite_name_input: String::new(),
            selected_favorite: None,
            contrast_idx_a: 0,
            contrast_idx_b: last_idx,
            contrast_pair_result: None,
            gradient_idx_1: 0,
            gradient_idx_2: second_idx,
            gradient_idx_3: None,
            gradient_colors,
            gradient_css,
            extracted_image_name: "Aucune image chargée".to_string(),
            extracted_colors: Vec::new(),
            status_text: "Prêt — Cliquez sur une couleur pour la copier (Ctrl+clic = RGB).".to_string(),
            status_kind: StatusKind::Info,
            clipboard: arboard::Clipboard::new().ok(),
            cache,
            update_check_rx,
            update_check_manual: false,
            available_update: None,
            self_update_rx: None,
            is_updating: false,
        }
    }

    fn set_status(&mut self, text: impl Into<String>, kind: StatusKind) {
        self.status_text = text.into();
        self.status_kind = kind;
    }

    fn copy_text_to_clipboard(&mut self, ui: &Ui, text: &str, msg: String) {
        ui.ctx().copy_text(text.to_string());
        if let Some(cb) = &mut self.clipboard {
            let _ = cb.set_text(text.to_string());
        }
        self.set_status(msg, StatusKind::Success);
    }

    fn copy_color(&mut self, ui: &Ui, hex_code: &str, fmt: ColorFormat) {
        let formatted = cu::format_color(hex_code, fmt);
        self.copy_text_to_clipboard(
            ui,
            &formatted,
            format!("✓ {formatted} copié dans le presse-papier."),
        );
    }

    fn set_base_color(&mut self, hex_code: &str, push_history: bool, record_undo: bool) {
        let normalized = match cu::normalize_hex(hex_code) {
            Ok(h) => h,
            Err(e) => {
                self.set_status(format!("✗ {e}"), StatusKind::Error);
                return;
            }
        };

        if record_undo && normalized != self.base_hex {
            self.undo_stack.push(self.base_hex.clone());
            self.redo_stack.clear();
        }

        self.base_hex = normalized.clone();
        self.hex_input = normalized.clone();

        if push_history {
            self.history = self.history_store.add(&normalized);
        }

        self.session_data.base_hex = normalized.clone();
        self.session_store.save(&self.session_data);

        self.regenerate_working_palette(false);
        self.cache.recompute_schemes(&self.base_hex);
    }

    fn regenerate_working_palette(&mut self, force: bool) {
        let new_swatches = cu::generate_palette(&self.base_hex, self.working_scheme);

        if force || new_swatches.len() != self.working_palette.len() {
            if new_swatches.len() != self.working_palette.len() {
                self.locked_indices.clear();
            }
            self.working_palette = new_swatches
                .into_iter()
                .enumerate()
                .map(|(i, s)| {
                    if self.locked_indices.contains(&i) && i < self.working_palette.len() {
                        self.working_palette[i].clone()
                    } else {
                        s
                    }
                })
                .collect();
        } else {
            for (i, s) in new_swatches.into_iter().enumerate() {
                if !self.locked_indices.contains(&i) {
                    self.working_palette[i] = s;
                }
            }
        }

        let len = self.working_palette.len().max(1);
        if self.contrast_idx_a >= len {
            self.contrast_idx_a = 0;
        }
        if self.contrast_idx_b >= len {
            self.contrast_idx_b = len - 1;
        }
        if self.gradient_idx_1 >= len {
            self.gradient_idx_1 = 0;
        }
        if self.gradient_idx_2 >= len {
            self.gradient_idx_2 = (len - 1).min(1);
        }
        if let Some(idx3) = self.gradient_idx_3
            && idx3 >= len
        {
            self.gradient_idx_3 = None;
        }

        self.session_data.last_scheme = self.working_scheme.id().to_string();
        self.session_store.save(&self.session_data);
        self.cache.recompute_working_dependents(&self.working_palette);
        self.update_gradient_from_selection();
    }

    fn update_gradient_from_selection(&mut self) {
        if self.working_palette.len() < 2 {
            return;
        }
        let mut chosen = Vec::with_capacity(3);
        if let Some(s1) = self.working_palette.get(self.gradient_idx_1) {
            chosen.push(s1.hex.to_uppercase());
        }
        if let Some(s2) = self.working_palette.get(self.gradient_idx_2) {
            chosen.push(s2.hex.to_uppercase());
        }
        if let Some(idx3) = self.gradient_idx_3
            && let Some(s3) = self.working_palette.get(idx3)
        {
            chosen.push(s3.hex.to_uppercase());
        }
        if chosen.len() >= 2 {
            self.gradient_css = format!("background: linear-gradient(90deg, {});", chosen.join(", "));
            self.gradient_colors = chosen;
        }
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            let current = self.base_hex.clone();
            self.redo_stack.push(current);
            self.set_base_color(&prev, false, false);
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = self.base_hex.clone();
            self.undo_stack.push(current);
            self.set_base_color(&next, false, false);
        }
    }

    fn toggle_theme(&mut self) {
        let next_name = if self.theme.name == "dark" { "light" } else { "dark" };
        self.theme = get_theme(next_name);
        self.theme_dirty = true;
        self.session_data.theme = next_name.to_string();
        self.session_store.save(&self.session_data);
        self.set_status(
            format!(
                "✓ Thème {} appliqué instantanément.",
                if next_name == "dark" { "sombre" } else { "clair" }
            ),
            StatusKind::Success,
        );
    }

    // ------------------------------------------------------------------
    // Rendu des panneaux
    // ------------------------------------------------------------------

    fn render_toolbar(&mut self, ui: &mut Ui) {
        let t = self.theme;
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new("🎨 GoldenHue")
                    .font(FontId::new(20.0, FontFamily::Proportional))
                    .strong()
                    .color(t.accent),
            );
            ui.label(
                RichText::new(format!("Rust Edition v{CURRENT_VERSION}"))
                    .font(FontId::new(11.0, FontFamily::Proportional))
                    .italics()
                    .color(t.text_muted),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(6.0);
                let theme_label = if t.name == "dark" { "☀ Thème clair" } else { "☾ Thème sombre" };
                if ui.button(RichText::new(theme_label).strong()).clicked() {
                    self.toggle_theme();
                }

                let checking = self.update_check_rx.is_some() || self.is_updating;
                let upd_label = if self.is_updating {
                    "⏳ Mise à jour…"
                } else if self.update_check_rx.is_some() {
                    "⏳ Vérification…"
                } else {
                    "🔄 Vérifier màj"
                };
                if ui
                    .add_enabled(!checking, egui::Button::new(RichText::new(upd_label).strong()))
                    .on_hover_text("Vérifier si une nouvelle version de GoldenHue est disponible sur GitHub")
                    .clicked()
                {
                    self.update_check_manual = true;
                    self.update_check_rx = Some(updater::spawn_update_check(ui.ctx().clone()));
                    self.set_status("Vérification des mises à jour sur GitHub…", StatusKind::Info);
                }

                if ui.button(RichText::new("🎲 Surprends-moi").strong()).clicked() {
                    let rnd = cu::random_hex();
                    self.set_base_color(&rnd, true, true);
                }

                if ui
                    .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("↻ Refaire"))
                    .clicked()
                {
                    self.redo();
                }

                if ui
                    .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("↺ Annuler"))
                    .clicked()
                {
                    self.undo();
                }
            });
        });
    }

    fn render_sidebar(&mut self, ui: &mut Ui) {
        let t = self.theme;
        ScrollArea::vertical()
            .id_salt("sidebar_scroll")
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Couleur de base").size(14.0).strong().color(t.text));
                ui.add_space(4.0);

                // Sélecteur de couleur interactif + aperçu direct
                ui.horizontal(|ui| {
                    let (r, g, b) = cu::hex_to_rgb256(&self.base_hex);
                    let mut srgb = [r, g, b];
                    let resp = ui.color_edit_button_srgb(&mut srgb);
                    if resp.changed() {
                        let new_hex = cu::rgb256_to_hex(srgb[0], srgb[1], srgb[2]);
                        self.set_base_color(&new_hex, true, true);
                    }
                    ui.label(
                        RichText::new("← Choisir une couleur")
                            .size(12.5)
                            .color(t.text),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("HEX").monospace().strong().color(t.text_muted));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.hex_input)
                            .font(FontId::new(13.5, FontFamily::Monospace))
                            .desired_width(110.0),
                    );
                    let apply_clicked = ui.button("Appliquer").clicked();
                    if apply_clicked
                        || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        let input = self.hex_input.clone();
                        self.set_base_color(&input, true, true);
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Nom").monospace().strong().color(t.text_muted));
                    let resp = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.name_input)
                                .hint_text("ex. corail, bleu marine…")
                                .desired_width(135.0),
                        )
                        .on_hover_text("Saisissez un nom de couleur français puis appuyez sur Entrée");
                    let ok_clicked = ui.button("OK").clicked();
                    if ok_clicked
                        || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        let name = self.name_input.trim().to_string();
                        if !name.is_empty() {
                            if let Some(hex) = lookup_color_name(&name) {
                                self.set_base_color(hex, true, true);
                                self.set_status(
                                    format!("✓ « {name} » → {hex}"),
                                    StatusKind::Success,
                                );
                            } else {
                                self.set_status(
                                    format!("✗ Couleur inconnue : « {name} »."),
                                    StatusKind::Error,
                                );
                            }
                        }
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Format d'affichage
                ui.label(RichText::new("Format affiché").size(14.0).strong().color(t.text));
                ui.horizontal(|ui| {
                    for fmt in ColorFormat::ALL {
                        ui.selectable_value(&mut self.color_format, fmt, fmt.as_str());
                    }
                });
                ui.label(
                    RichText::new("Astuce : Ctrl+clic copie toujours en RGB")
                        .size(11.0)
                        .italics()
                        .color(t.text_muted),
                );

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Historique récent
                ui.label(RichText::new("Historique récent").size(14.0).strong().color(t.text));
                if self.history.is_empty() {
                    ui.label(RichText::new("Aucun historique").size(11.5).italics().color(t.text_muted));
                } else {
                    let history_clone = self.history.clone();
                    ui.horizontal_wrapped(|ui| {
                        for hex in history_clone {
                            let col = hex_to_color32(&hex);
                            let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
                            ui.painter().rect_filled(rect, CornerRadius::same(4), col);
                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::same(4),
                                Stroke::new(1.0, t.border),
                                StrokeKind::Inside,
                            );
                            if resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(&hex).clicked() {
                                self.set_base_color(&hex, true, true);
                            }
                        }
                    });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Ajustement global
                ui.label(RichText::new("Ajustement global").size(14.0).strong().color(t.text));
                ui.label(RichText::new("Saturation (%)").size(12.0).color(t.text_muted));
                ui.add(egui::Slider::new(&mut self.sat_slider, -50.0..=50.0).step_by(1.0));
                ui.label(RichText::new("Luminosité (%)").size(12.0).color(t.text_muted));
                ui.add(egui::Slider::new(&mut self.light_slider, -50.0..=50.0).step_by(1.0));

                if ui.button("Appliquer l'ajustement").clicked() {
                    if self.sat_slider == 0.0 && self.light_slider == 0.0 {
                        self.set_status(
                            "Ajustez d'abord les curseurs de saturation/luminosité.",
                            StatusKind::Info,
                        );
                    } else {
                        let new_hex = cu::apply_global_adjustment(
                            &self.base_hex,
                            self.sat_slider,
                            self.light_slider,
                        );
                        self.sat_slider = 0.0;
                        self.light_slider = 0.0;
                        self.set_base_color(&new_hex, true, true);
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Schéma de la palette de travail
                ui.label(
                    RichText::new("Schéma (palette de travail)")
                        .size(14.0)
                        .strong()
                        .color(t.text),
                );
                let prev_scheme = self.working_scheme;
                egui::ComboBox::from_id_salt("working_scheme_combo")
                    .selected_text(self.working_scheme.label())
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        for scheme in SchemeType::ALL {
                            ui.selectable_value(&mut self.working_scheme, scheme, scheme.label());
                        }
                    });
                if self.working_scheme != prev_scheme {
                    self.locked_indices.clear();
                    self.regenerate_working_palette(true);
                }
            });
    }

    /// Dessine une carte de couleur (swatch) avec support clic (copier), Ctrl+clic (copier RGB),
    /// verrouillage optionnel et sélecteur de couleur direct pour l'édition manuelle.
    fn draw_swatch_card(
        &mut self,
        ui: &mut Ui,
        role: &str,
        hex_code: &str,
        editable_idx: Option<usize>,
    ) {
        let fill = hex_to_color32(hex_code);
        let text_col = readable_color32(hex_code);
        let formatted = cu::format_color(hex_code, self.color_format);

        let card_size = vec2(134.0, 96.0);
        let (rect, resp) = ui.allocate_exact_size(card_size, Sense::click());

        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), fill);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.0, self.theme.border),
            StrokeKind::Inside,
        );

        // Texte centré : rôle + valeur formatée
        let center_x = rect.center().x;
        let role_y = if editable_idx.is_some() {
            rect.min.y + 38.0
        } else {
            rect.min.y + 32.0
        };
        ui.painter().text(
            pos2(center_x, role_y),
            egui::Align2::CENTER_CENTER,
            role,
            FontId::new(12.0, FontFamily::Proportional),
            text_col,
        );
        ui.painter().text(
            pos2(center_x, role_y + 22.0),
            egui::Align2::CENTER_CENTER,
            &formatted,
            FontId::new(11.5, FontFamily::Monospace),
            text_col,
        );

        let mut lock_clicked = false;
        let mut edited_hex: Option<String> = None;

        if let Some(idx) = editable_idx {
            let is_locked = self.locked_indices.contains(&idx);
            let top_bar = egui::Rect::from_min_size(rect.min + vec2(6.0, 4.0), vec2(card_size.x - 12.0, 22.0));
            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(top_bar)
                    .layout(Layout::left_to_right(Align::Center)),
            );

            // Sélecteur de couleur direct pour modifier ce swatch (équivalent amélioré du double-clic)
            let (r, g, b) = cu::hex_to_rgb256(hex_code);
            let mut srgb = [r, g, b];
            let picker_resp = child_ui
                .color_edit_button_srgb(&mut srgb)
                .on_hover_text("Modifier cette couleur (verrouille automatiquement)");
            if picker_resp.changed() {
                edited_hex = Some(cu::rgb256_to_hex(srgb[0], srgb[1], srgb[2]));
            }

            child_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let lock_icon = if is_locked { "🔒" } else { "🔓" };
                let btn = egui::Button::new(RichText::new(lock_icon).size(13.0).color(text_col))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE);
                if ui
                    .add(btn)
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text(if is_locked {
                        "Déverrouiller cette couleur"
                    } else {
                        "Verrouiller cette couleur"
                    })
                    .clicked()
                {
                    lock_clicked = true;
                }
            });

            if lock_clicked {
                if is_locked {
                    self.locked_indices.remove(&idx);
                } else {
                    self.locked_indices.insert(idx);
                }
            }

            if let Some(new_h) = edited_hex {
                if idx < self.working_palette.len() {
                    self.working_palette[idx].hex = new_h;
                    self.locked_indices.insert(idx);
                    self.cache.recompute_working_dependents(&self.working_palette);
                    self.update_gradient_from_selection();
                }
                return;
            }
        }

        if !lock_clicked && resp.clicked() {
            let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            let fmt = if ctrl {
                ColorFormat::RGB
            } else {
                self.color_format
            };
            self.copy_color(ui, hex_code, fmt);
        }

        resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(format!(
            "{}\n{} — Clic : copier {} | Ctrl+clic : copier RGB",
            cu::role_description(role),
            hex_code,
            self.color_format.as_str()
        ));
    }

    fn render_main_sections(&mut self, ui: &mut Ui) {
        let t = self.theme;

        // 1. Palette de travail
        section_frame(ui, t, "🎨 Palette de travail", |ui| {
            ui.label(
                RichText::new(
                    "Utilisez le bouton couleur en haut à gauche d'une carte pour l'éditer manuellement (verrouille automatiquement), \
                     🔒 pour verrouiller/déverrouiller, ou cliquez sur une carte pour copier son code.",
                )
                .size(12.0)
                .italics()
                .color(t.text_muted),
            );
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                if ui.button("🔄 Régénérer").clicked() {
                    self.regenerate_working_palette(true);
                }
                if ui.button("📋 Copier toute la palette").clicked() {
                    let fmt = self.color_format;
                    let text = self
                        .working_palette
                        .iter()
                        .map(|s| format!("{}: {}", s.role, cu::format_color(&s.hex, fmt)))
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.copy_text_to_clipboard(
                        ui,
                        &text,
                        "✓ Palette complète copiée dans le presse-papier.".to_string(),
                    );
                }

                ui.separator();
                ui.label(RichText::new("Nom du favori :").size(12.0).color(t.text_muted));
                ui.add(
                    egui::TextEdit::singleline(&mut self.favorite_name_input)
                        .hint_text("Ma palette…")
                        .desired_width(140.0),
                );
                if ui.button("⭐ Enregistrer en favori").clicked() {
                    let name = if self.favorite_name_input.trim().is_empty() {
                        format!("Palette {}", self.base_hex)
                    } else {
                        self.favorite_name_input.trim().to_string()
                    };
                    self.favorites = self.favorites_store.save_palette(
                        &name,
                        &self.base_hex,
                        self.working_scheme.id(),
                        self.working_palette.clone(),
                    );
                    self.favorite_name_input.clear();
                    self.set_status(
                        format!("✓ Palette « {name} » enregistrée dans les favoris."),
                        StatusKind::Success,
                    );
                }
            });

            ui.add_space(6.0);
            let working_snapshot = self.working_palette.clone();
            ScrollArea::horizontal()
                .id_salt("working_palette_hscroll")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (i, swatch) in working_snapshot.iter().enumerate() {
                            self.draw_swatch_card(ui, &swatch.role, &swatch.hex, Some(i));
                        }
                    });
                });
        });

        // 2. Toutes les harmonies
        section_frame(ui, t, "🧭 Toutes les harmonies", |ui| {
            ui.label(
                RichText::new(
                    "Toutes les harmonies générées simultanément pour la couleur de base actuelle (clic = copier, Ctrl+clic = copier en RGB) :",
                )
                .size(12.0)
                .italics()
                .color(t.text_muted),
            );
            ui.add_space(4.0);

            let schemes_snapshot = self.cache.all_schemes.clone();
            for (scheme, swatches) in schemes_snapshot {
                ui.add_space(4.0);
                ui.label(RichText::new(scheme.label()).size(13.5).strong().color(t.text));
                ScrollArea::horizontal()
                    .id_salt(("scheme_hscroll", scheme.id()))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for s in &swatches {
                                self.draw_swatch_card(ui, &s.role, &s.hex, None);
                            }
                        });
                    });
            }
        });

        // 3. Contraste WCAG
        section_frame(ui, t, "🔎 Contraste WCAG", |ui| {
            egui::Grid::new("wcag_contrast_grid")
                .striped(true)
                .min_col_width(160.0)
                .spacing(vec2(16.0, 6.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("Couleur").strong().color(t.text_muted));
                    ui.label(RichText::new("vs Blanc (#FFFFFF)").strong().color(t.text_muted));
                    ui.label(RichText::new("vs Noir (#000000)").strong().color(t.text_muted));
                    ui.end_row();

                    for (swatch, (vs_white, vs_black)) in
                        self.working_palette.iter().zip(self.cache.contrasts_bw.iter())
                    {
                        let chip_bg = hex_to_color32(&swatch.hex);
                        let chip_fg = readable_color32(&swatch.hex);
                        Frame::new()
                            .fill(chip_bg)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{} ({})", swatch.role, swatch.hex))
                                        .color(chip_fg)
                                        .size(12.0)
                                        .strong(),
                                );
                            });

                        for res in [vs_white, vs_black] {
                            let col = if res.aa_normal {
                                t.success
                            } else if res.aa_large {
                                t.accent
                            } else {
                                t.error
                            };
                            ui.label(
                                RichText::new(format!("{:.2}:1  {}", res.ratio, res.badge()))
                                    .strong()
                                    .color(col),
                            );
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Vérificateur libre :").strong().color(t.text));
                let len = self.working_palette.len();
                if len > 0 {
                    let label_a = &self.working_palette[self.contrast_idx_a.min(len - 1)].role;
                    egui::ComboBox::from_id_salt("contrast_combo_a")
                        .selected_text(label_a)
                        .show_ui(ui, |ui| {
                            for (i, s) in self.working_palette.iter().enumerate() {
                                ui.selectable_value(&mut self.contrast_idx_a, i, &s.role);
                            }
                        });

                    ui.label(RichText::new("vs").color(t.text_muted));

                    let label_b = &self.working_palette[self.contrast_idx_b.min(len - 1)].role;
                    egui::ComboBox::from_id_salt("contrast_combo_b")
                        .selected_text(label_b)
                        .show_ui(ui, |ui| {
                            for (i, s) in self.working_palette.iter().enumerate() {
                                ui.selectable_value(&mut self.contrast_idx_b, i, &s.role);
                            }
                        });

                    if ui.button("Comparer").clicked() {
                        let hex_a = &self.working_palette[self.contrast_idx_a.min(len - 1)].hex;
                        let hex_b = &self.working_palette[self.contrast_idx_b.min(len - 1)].hex;
                        let res = ctr::evaluate_contrast(hex_a, hex_b);
                        let col = if res.aa_normal {
                            t.success
                        } else if res.aa_large {
                            t.accent
                        } else {
                            t.error
                        };
                        self.contrast_pair_result = Some((res.ratio, res.badge(), col));
                    }

                    if let Some((ratio, badge, col)) = self.contrast_pair_result {
                        ui.label(
                            RichText::new(format!("{ratio:.2}:1 — {badge}"))
                                .strong()
                                .color(col),
                        );
                    }
                }
            });
        });

        // 4. Simulation de daltonisme
        section_frame(ui, t, "👁 Simulation de daltonisme", |ui| {
            let cb_snapshot = self.cache.colorblind_rows.clone();
            for (cb_type, row) in cb_snapshot {
                ui.add_space(4.0);
                ui.label(RichText::new(cb_type.label()).size(13.5).strong().color(t.text));
                ScrollArea::horizontal()
                    .id_salt(("cb_hscroll", cb_type.label()))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for s in &row {
                                self.draw_swatch_card(ui, &s.role, &s.hex, None);
                            }
                        });
                    });
            }
        });

        // 5. Aperçu UI (mockup)
        section_frame(ui, t, "🖼 Aperçu UI", |ui| {
            let accent = self
                .working_palette
                .iter()
                .find(|s| s.role == "Base")
                .or_else(|| self.working_palette.first())
                .map(|s| s.hex.as_str())
                .unwrap_or("#F1C40F");
            let bg = self
                .working_palette
                .iter()
                .find(|s| s.role.contains(NEUTRAL_ROLE_BG))
                .map(|s| s.hex.as_str())
                .unwrap_or("#F4F3F1");
            let fg = self
                .working_palette
                .iter()
                .find(|s| s.role.contains(NEUTRAL_ROLE_TEXT))
                .map(|s| s.hex.as_str())
                .unwrap_or("#2B2921");
            let card_bg = self
                .working_palette
                .iter()
                .find(|s| s.role.contains("+50%"))
                .map(|s| s.hex.as_str())
                .unwrap_or("#FFFFFF");

            Frame::new()
                .fill(hex_to_color32(bg))
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("Aperçu d'interface")
                            .size(16.0)
                            .strong()
                            .color(hex_to_color32(fg)),
                    );
                    ui.add_space(8.0);

                    Frame::new()
                        .fill(hex_to_color32(card_bg))
                        .stroke(Stroke::new(1.0, t.border))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::same(14))
                        .show(ui, |ui| {
                            let card_fg = readable_color32(card_bg);
                            ui.label(
                                RichText::new("Titre de carte")
                                    .size(14.5)
                                    .strong()
                                    .color(card_fg),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "Texte d'exemple utilisant en temps réel les couleurs de la palette de travail.",
                                )
                                .size(12.5)
                                .color(card_fg),
                            );
                            ui.add_space(10.0);

                            let btn_bg = hex_to_color32(accent);
                            let btn_fg = readable_color32(accent);
                            let btn = egui::Button::new(
                                RichText::new("Bouton d'action").strong().color(btn_fg),
                            )
                            .fill(btn_bg)
                            .corner_radius(CornerRadius::same(6));
                            ui.add(btn);
                        });
                });
        });

        // 6. Dégradé CSS (rendu GPU par Mesh)
        section_frame(ui, t, "🌈 Dégradé CSS", |ui| {
            let len = self.working_palette.len();
            if len >= 2 {
                let mut changed = false;
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Couleur 1 :").color(t.text_muted));
                    let r1 = &self.working_palette[self.gradient_idx_1.min(len - 1)].role;
                    egui::ComboBox::from_id_salt("grad_c1")
                        .selected_text(r1)
                        .show_ui(ui, |ui| {
                            for (i, s) in self.working_palette.iter().enumerate() {
                                if ui.selectable_value(&mut self.gradient_idx_1, i, &s.role).changed() {
                                    changed = true;
                                }
                            }
                        });

                    ui.label(RichText::new("Couleur 2 :").color(t.text_muted));
                    let r2 = &self.working_palette[self.gradient_idx_2.min(len - 1)].role;
                    egui::ComboBox::from_id_salt("grad_c2")
                        .selected_text(r2)
                        .show_ui(ui, |ui| {
                            for (i, s) in self.working_palette.iter().enumerate() {
                                if ui.selectable_value(&mut self.gradient_idx_2, i, &s.role).changed() {
                                    changed = true;
                                }
                            }
                        });

                    ui.label(RichText::new("Couleur 3 (optionnel) :").color(t.text_muted));
                    let r3 = self
                        .gradient_idx_3
                        .and_then(|idx| self.working_palette.get(idx))
                        .map(|s| s.role.as_str())
                        .unwrap_or("(Aucune)");
                    egui::ComboBox::from_id_salt("grad_c3")
                        .selected_text(r3)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(&mut self.gradient_idx_3, None, "(Aucune)")
                                .changed()
                            {
                                changed = true;
                            }
                            for (i, s) in self.working_palette.iter().enumerate() {
                                if ui
                                    .selectable_value(&mut self.gradient_idx_3, Some(i), &s.role)
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });

                    if ui.button("Générer").clicked() || changed {
                        self.update_gradient_from_selection();
                    }
                });

                ui.add_space(6.0);
                // Rendu matériel du dégradé par interpolation de sommets GPU (1 seul Mesh !)
                let width = ui.available_width().max(200.0);
                let height = 56.0;
                let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                if self.gradient_colors.len() >= 2 {
                    let mut mesh = Mesh::default();
                    let n_stops = self.gradient_colors.len();
                    for (i, hex) in self.gradient_colors.iter().enumerate() {
                        let frac = (i as f32) / ((n_stops - 1) as f32);
                        let x = rect.min.x + frac * rect.width();
                        let col = hex_to_color32(hex);
                        mesh.colored_vertex(pos2(x, rect.min.y), col);
                        mesh.colored_vertex(pos2(x, rect.max.y), col);
                        if i > 0 {
                            let base = ((i - 1) * 2) as u32;
                            mesh.add_triangle(base, base + 1, base + 2);
                            mesh.add_triangle(base + 1, base + 3, base + 2);
                        }
                    }
                    ui.painter().add(egui::Shape::mesh(mesh));
                }
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(4),
                    Stroke::new(1.0, t.border),
                    StrokeKind::Inside,
                );

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.gradient_css)
                            .font(FontId::new(12.5, FontFamily::Monospace))
                            .desired_width(ui.available_width() - 120.0),
                    );
                    if ui.button("📋 Copier le CSS").clicked() {
                        let css = self.gradient_css.clone();
                        self.copy_text_to_clipboard(
                            ui,
                            &css,
                            "✓ CSS du dégradé copié.".to_string(),
                        );
                    }
                });
            }
        });

        // 7. Extraction depuis une image
        section_frame(ui, t, "📷 Extraction depuis une image", |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Charger une image…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .set_title("Choisir une image")
                        .add_filter(
                            "Images",
                            &["png", "jpg", "jpeg", "bmp", "ico", "tiff", "tif", "tga"],
                        )
                        .pick_file()
                {
                    match extract_dominant_colors(&path, 6, 150) {
                        Ok(colors) => {
                            self.extracted_image_name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Image")
                                .to_string();
                            self.extracted_colors = colors;
                            self.set_status(
                                format!(
                                    "✓ {} couleurs dominantes extraites.",
                                    self.extracted_colors.len()
                                ),
                                StatusKind::Success,
                            );
                        }
                        Err(err) => {
                            self.set_status(format!("✗ {err}"), StatusKind::Error);
                        }
                    }
                }
                ui.label(RichText::new(&self.extracted_image_name).color(t.text_muted));
            });

            if !self.extracted_colors.is_empty() {
                ui.add_space(6.0);
                let extracted_snapshot = self.extracted_colors.clone();
                ScrollArea::horizontal()
                    .id_salt("extracted_image_hscroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (i, hex) in extracted_snapshot.iter().enumerate() {
                                self.draw_swatch_card(
                                    ui,
                                    &format!("Dominante {}", i + 1),
                                    hex,
                                    None,
                                );
                            }
                        });
                    });

                ui.add_space(6.0);
                if ui.button("✨ Utiliser comme palette de travail").clicked() {
                    self.working_palette = self
                        .extracted_colors
                        .iter()
                        .enumerate()
                        .map(|(i, h)| SwatchSpec::new(format!("Dominante {}", i + 1), h.clone()))
                        .collect();
                    self.locked_indices = (0..self.working_palette.len()).collect();
                    let first = self.extracted_colors[0].clone();
                    self.set_base_color(&first, true, true);
                    self.set_status(
                        "✓ Palette de travail remplacée par les couleurs extraites de l'image.",
                        StatusKind::Success,
                    );
                }
            }
        });

        // 8. Favoris
        section_frame(ui, t, "⭐ Favoris", |ui| {
            ui.label(
                RichText::new("Cliquez sur « Charger » ou double-cliquez sur un favori pour l'appliquer.")
                    .size(12.0)
                    .italics()
                    .color(t.text_muted),
            );
            ui.add_space(4.0);

            if self.favorites.is_empty() {
                ui.label(
                    RichText::new("Aucun favori enregistré pour le moment.")
                        .italics()
                        .color(t.text_muted),
                );
            } else {
                let mut load_idx: Option<usize> = None;
                let mut delete_idx: Option<usize> = None;

                for (i, fav) in self.favorites.iter().enumerate() {
                    let is_selected = self.selected_favorite == Some(i);
                    ui.horizontal(|ui| {
                        let resp = ui.selectable_label(
                            is_selected,
                            format!("{}  ({}) — [{}]", fav.name, fav.base_hex, fav.scheme),
                        );
                        if resp.clicked() {
                            self.selected_favorite = Some(i);
                        }
                        if resp.double_clicked() {
                            load_idx = Some(i);
                        }

                        // Mini aperçu des couleurs du favori
                        for c in fav.colors.iter().take(8) {
                            let (r_rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                            ui.painter()
                                .rect_filled(r_rect, CornerRadius::same(3), hex_to_color32(&c.hex));
                        }

                        if ui.small_button("Charger").clicked() {
                            load_idx = Some(i);
                        }
                        if ui
                            .small_button(RichText::new("🗑").color(t.error))
                            .on_hover_text("Supprimer ce favori")
                            .clicked()
                        {
                            delete_idx = Some(i);
                        }
                    });
                }

                if let Some(idx) = load_idx
                    && let Some(fav) = self.favorites.get(idx).cloned()
                {
                    self.working_scheme = SchemeType::from_id(&fav.scheme);
                    self.working_palette = fav.colors.clone();
                    self.locked_indices = (0..self.working_palette.len()).collect();
                    self.set_base_color(&fav.base_hex, true, true);
                    self.set_status(
                        format!("✓ Favori « {} » chargé.", fav.name),
                        StatusKind::Success,
                    );
                }

                if let Some(idx) = delete_idx {
                    self.favorites = self.favorites_store.delete(idx);
                    if self.selected_favorite == Some(idx) {
                        self.selected_favorite = None;
                    }
                    self.set_status("✓ Favori supprimé.", StatusKind::Info);
                }
            }
        });

        // 9. Export
        section_frame(ui, t, "⬇ Export", |ui| {
            ui.label(
                RichText::new("Exporte la palette de travail actuelle :")
                    .size(12.0)
                    .italics()
                    .color(t.text_muted),
            );
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                let formats = [
                    ("csv", "CSV"),
                    ("json", "JSON"),
                    ("css", "CSS"),
                    ("scss", "SCSS"),
                    ("txt", "TXT"),
                    ("ase", "Adobe .ase"),
                    ("png", "Image PNG"),
                ];
                for (fmt, label) in formats {
                    let btn = egui::Button::new(
                        RichText::new(label).strong().color(t.accent_text),
                    )
                    .fill(t.accent);
                    if ui.add(btn).clicked() {
                        self.export_working_palette(fmt);
                    }
                }
            });
        });

        // 10. Règles de design (aide-mémoire)
        section_frame(ui, t, "📚 Règles de design (aide-mémoire)", |ui| {
            ui.label(
                RichText::new(
                    "Aide-mémoire des règles de design colorimétrique les plus courantes — cliquez sur un titre pour déplier/replier.",
                )
                .size(12.0)
                .italics()
                .color(t.text_muted),
            );
            ui.add_space(6.0);

            for &(title, rules) in DESIGN_RULES {
                egui::CollapsingHeader::new(RichText::new(title).size(13.5).strong().color(t.accent))
                    .default_open(true)
                    .show(ui, |ui| {
                        for &rule in rules {
                            ui.label(RichText::new(format!("•  {rule}")).size(12.5).color(t.text));
                        }
                    });
                ui.add_space(2.0);
            }
        });
    }

    fn export_working_palette(&mut self, fmt: &str) {
        if self.working_palette.is_empty() {
            return;
        }
        let default_name = format!("goldenhue_palette.{fmt}");
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Exporter la palette")
            .set_file_name(&default_name)
            .add_filter(&fmt.to_uppercase(), &[fmt])
            .save_file()
        {
            let manager = ExportManager::new(self.working_palette.clone(), "GoldenHue");
            match manager.save(&path, fmt) {
                Ok(()) => {
                    self.set_status(
                        format!("✓ Palette exportée : {}", path.display()),
                        StatusKind::Success,
                    );
                }
                Err(err) => {
                    self.set_status(
                        format!("✗ Échec de l'export {} : {err}", fmt.to_uppercase()),
                        StatusKind::Error,
                    );
                }
            }
        }
    }

    fn poll_update_channels(&mut self) {
        if let Some(rx) = &self.update_check_rx
            && let Ok(outcome) = rx.try_recv()
        {
            self.update_check_rx = None;
            let manual = self.update_check_manual;
            self.update_check_manual = false;
            match outcome {
                UpdateCheckOutcome::UpToDate(ver) => {
                    if manual {
                        self.set_status(
                            format!("✓ GoldenHue est à jour (v{ver})."),
                            StatusKind::Success,
                        );
                    }
                }
                UpdateCheckOutcome::UpdateAvailable(info) => {
                    self.set_status(
                        format!(
                            "🚀 Nouvelle version v{} disponible ! Cliquez sur « Mettre à jour maintenant » en haut.",
                            info.latest_version
                        ),
                        StatusKind::Success,
                    );
                    self.available_update = Some(info);
                }
                UpdateCheckOutcome::Error(err) => {
                    if manual {
                        self.set_status(format!("✗ {err}"), StatusKind::Error);
                    }
                }
            }
        }

        if let Some(rx) = &self.self_update_rx
            && let Ok(outcome) = rx.try_recv()
        {
            self.self_update_rx = None;
            self.is_updating = false;
            match outcome {
                SelfUpdateOutcome::ReadyToRestart(exe_path) => {
                    self.set_status(
                        "✓ Mise à jour installée ! Redémarrage de GoldenHue…",
                        StatusKind::Success,
                    );
                    updater::restart_application(&exe_path);
                }
                SelfUpdateOutcome::Error(err) => {
                    self.set_status(format!("✗ {err}"), StatusKind::Error);
                }
            }
        }
    }
}

impl eframe::App for PaletteApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_update_channels();

        if self.theme_dirty {
            self.theme.apply_to_ctx(ui.ctx());
            self.theme_dirty = false;
        }

        let t = self.theme;

        // Barre d'outils supérieure
        egui::Panel::top("goldenhue_toolbar")
            .frame(
                Frame::new()
                    .fill(t.bg_alt)
                    .inner_margin(Margin::symmetric(10, 8))
                    .stroke(Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                self.render_toolbar(ui);
            });

        // Bannière de mise à jour (affichée si une nouvelle version est détectée sur GitHub)
        if let Some(info) = self.available_update.clone() {
            egui::Panel::top("goldenhue_update_banner")
                .frame(
                    Frame::new()
                        .fill(t.panel)
                        .inner_margin(Margin::symmetric(14, 8))
                        .stroke(Stroke::new(1.5, t.accent)),
                )
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "🚀 Nouvelle version disponible : v{} (actuelle : v{CURRENT_VERSION})",
                                info.latest_version
                            ))
                            .strong()
                            .color(t.accent),
                        );

                        if !info.release_notes.is_empty() {
                            let first_line = info.release_notes.lines().next().unwrap_or("");
                            if !first_line.is_empty() {
                                ui.label(
                                    RichText::new(format!("— {first_line}"))
                                        .italics()
                                        .color(t.text_muted),
                                );
                            }
                        }

                        let btn_update = egui::Button::new(
                            RichText::new(if self.is_updating {
                                "⏳ Téléchargement en cours…"
                            } else {
                                "⬇ Mettre à jour maintenant"
                            })
                            .strong()
                            .color(t.accent_text),
                        )
                        .fill(t.accent);

                        if ui.add_enabled(!self.is_updating, btn_update).clicked() {
                            self.is_updating = true;
                            self.set_status(
                                format!(
                                    "Téléchargement de GoldenHue v{} depuis GitHub…",
                                    info.latest_version
                                ),
                                StatusKind::Info,
                            );
                            self.self_update_rx = Some(updater::spawn_self_update(
                                info.download_url.clone(),
                                ui.ctx().clone(),
                            ));
                        }

                        if ui.button("🌐 Voir sur GitHub").clicked() {
                            updater::open_url_in_browser(&info.html_url);
                        }

                        if ui.button("✖ Ignorer").clicked() {
                            self.available_update = None;
                        }
                    });
                });
        }

        // Barre de statut inférieure
        egui::Panel::bottom("goldenhue_statusbar")
            .frame(
                Frame::new()
                    .fill(t.bg)
                    .inner_margin(Margin::symmetric(12, 6))
                    .stroke(Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                let status_color = match self.status_kind {
                    StatusKind::Info => t.text_muted,
                    StatusKind::Success => t.success,
                    StatusKind::Error => t.error,
                };
                ui.label(RichText::new(&self.status_text).italics().color(status_color));
            });

        // Panneau latéral gauche
        egui::Panel::left("goldenhue_sidebar")
            .default_size(290.0)
            .min_size(250.0)
            .max_size(380.0)
            .frame(
                Frame::new()
                    .fill(t.bg_alt)
                    .inner_margin(Margin::symmetric(14, 10))
                    .stroke(Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                self.render_sidebar(ui);
            });

        // Zone principale défilante contenant toutes les sections empilées
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(t.bg)
                    .inner_margin(Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("main_vertical_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_main_sections(ui);
                    });
            });
    }
}
