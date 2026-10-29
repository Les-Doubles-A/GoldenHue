//! Logique pure et ultra-optimisée de conversion de couleurs et de génération d'harmonies.
//!
//! Optimisations clés :
//! - Validation et décodage HEX via table de correspondance statique (`HEX_LUT`) en O(1) sans regex.
//! - Formatage HEX direct sur buffer pile avant conversion `String`.
//! - Calculs HLS/HSV vectorisables et inlinés (`#[inline]`).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Table de correspondance O(1) pour convertir un octet ASCII hexadécimal en valeur 0..15 (ou 0xFF si invalide).
const HEX_LUT: [u8; 256] = {
    let mut lut = [0xFFu8; 256];
    let mut i = b'0';
    while i <= b'9' {
        lut[i as usize] = i - b'0';
        i += 1;
    }
    let mut i = b'A';
    while i <= b'F' {
        lut[i as usize] = i - b'A' + 10;
        i += 1;
    }
    let mut i = b'a';
    while i <= b'f' {
        lut[i as usize] = i - b'a' + 10;
        i += 1;
    }
    lut
};

const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";

pub const NEUTRAL_ROLE_BG: &str = "Fond Neutre";
pub const NEUTRAL_ROLE_TEXT: &str = "Texte Neutre";

#[inline]
pub fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    value.clamp(lo, hi)
}

#[inline]
pub fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// Décode 6 caractères hexadécimaux (sans '#') en `(r, g, b)` u8.
#[inline]
fn parse_hex6_bytes(bytes: &[u8]) -> Option<(u8, u8, u8)> {
    if bytes.len() != 6 {
        return None;
    }
    let r_hi = HEX_LUT[bytes[0] as usize];
    let r_lo = HEX_LUT[bytes[1] as usize];
    let g_hi = HEX_LUT[bytes[2] as usize];
    let g_lo = HEX_LUT[bytes[3] as usize];
    let b_hi = HEX_LUT[bytes[4] as usize];
    let b_lo = HEX_LUT[bytes[5] as usize];
    if (r_hi | r_lo | g_hi | g_lo | b_hi | b_lo) & 0xF0 != 0 {
        return None;
    }
    Some(((r_hi << 4) | r_lo, (g_hi << 4) | g_lo, (b_hi << 4) | b_lo))
}

/// Vérifie qu'une chaîne est un code hexadécimal `#RRGGBB` valide.
#[inline]
pub fn is_valid_hex(hex_code: &str) -> bool {
    let b = hex_code.trim().as_bytes();
    b.len() == 7 && b[0] == b'#' && parse_hex6_bytes(&b[1..]).is_some()
}

/// Ajoute le `#` si absent et met en majuscules. Retourne `Err` si invalide.
pub fn normalize_hex(hex_code: &str) -> Result<String, String> {
    let trimmed = hex_code.trim();
    let raw = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if let Some((r, g, b)) = parse_hex6_bytes(raw.as_bytes()) {
        Ok(rgb256_to_hex(r, g, b))
    } else {
        Err(format!("Code hexadécimal invalide : {hex_code:?}"))
    }
}

/// Convertit un code hexadécimal en RGB entier `[0, 255]`.
#[inline]
pub fn hex_to_rgb256(hex_code: &str) -> (u8, u8, u8) {
    let trimmed = hex_code.trim();
    let raw = trimmed.strip_prefix('#').unwrap_or(trimmed);
    parse_hex6_bytes(raw.as_bytes()).unwrap_or((0, 0, 0))
}

/// Convertit un code hexadécimal en RGB flottant `[0.0, 1.0]`.
#[inline]
pub fn hex_to_rgb01(hex_code: &str) -> (f64, f64, f64) {
    let (r, g, b) = hex_to_rgb256(hex_code);
    const INV_255: f64 = 1.0 / 255.0;
    (r as f64 * INV_255, g as f64 * INV_255, b as f64 * INV_255)
}

/// Convertit du RGB entier `[0, 255]` en code `#RRGGBB` majuscule sans formatage lent.
#[inline]
pub fn rgb256_to_hex(r: u8, g: u8, b: u8) -> String {
    let buf = [
        b'#',
        HEX_CHARS[(r >> 4) as usize],
        HEX_CHARS[(r & 0x0F) as usize],
        HEX_CHARS[(g >> 4) as usize],
        HEX_CHARS[(g & 0x0F) as usize],
        HEX_CHARS[(b >> 4) as usize],
        HEX_CHARS[(b & 0x0F) as usize],
    ];
    // SAFETY: `buf` ne contient que des caractères ASCII valides.
    unsafe { String::from_utf8_unchecked(buf.to_vec()) }
}

/// Convertit du RGB flottant `[0.0, 1.0]` en code `#RRGGBB`.
#[inline]
pub fn rgb01_to_hex(r: f64, g: f64, b: f64) -> String {
    let r8 = (clamp01(r) * 255.0).round() as u8;
    let g8 = (clamp01(g) * 255.0).round() as u8;
    let b8 = (clamp01(b) * 255.0).round() as u8;
    rgb256_to_hex(r8, g8, b8)
}

/// Équivalent exact de `colorsys.rgb_to_hls(r, g, b)` en `[0.0, 1.0]`.
#[inline]
pub fn rgb_to_hls01(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let maxc = r.max(g).max(b);
    let minc = r.min(g).min(b);
    let sumc = maxc + minc;
    let rangec = maxc - minc;
    let l = sumc * 0.5;
    if minc == maxc {
        return (0.0, l, 0.0);
    }
    let s = if l <= 0.5 {
        rangec / sumc
    } else {
        rangec / (2.0 - maxc - minc)
    };
    let rc = (maxc - r) / rangec;
    let gc = (maxc - g) / rangec;
    let bc = (maxc - b) / rangec;
    let mut h = if r == maxc {
        bc - gc
    } else if g == maxc {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    h = (h / 6.0).rem_euclid(1.0);
    (h, l, s)
}

#[inline]
fn hls_value(m1: f64, m2: f64, hue: f64) -> f64 {
    let hue = hue.rem_euclid(1.0);
    if hue < 1.0 / 6.0 {
        m1 + (m2 - m1) * hue * 6.0
    } else if hue < 0.5 {
        m2
    } else if hue < 2.0 / 3.0 {
        m1 + (m2 - m1) * (2.0 / 3.0 - hue) * 6.0
    } else {
        m1
    }
}

/// Équivalent exact de `colorsys.hls_to_rgb(h, l, s)` en `[0.0, 1.0]`.
#[inline]
pub fn hls01_to_rgb(h: f64, l: f64, s: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        return (l, l, l);
    }
    let m2 = if l <= 0.5 {
        l * (1.0 + s)
    } else {
        l + s - (l * s)
    };
    let m1 = 2.0 * l - m2;
    (
        hls_value(m1, m2, h + 1.0 / 3.0),
        hls_value(m1, m2, h),
        hls_value(m1, m2, h - 1.0 / 3.0),
    )
}

/// RGB `[0, 1]` -> HSL (`H` en degrés `[0, 360)`, `S` et `L` en `%` `[0, 100]`).
#[inline]
pub fn rgb01_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (h, l, s) = rgb_to_hls01(r, g, b);
    ((h * 360.0).rem_euclid(360.0), s * 100.0, l * 100.0)
}

/// HSL (`H` en degrés, `S`/`L` en `%`) -> RGB `[0, 1]`.
#[inline]
pub fn hsl_to_rgb01(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    hls01_to_rgb(
        h.rem_euclid(360.0) / 360.0,
        clamp01(l / 100.0),
        clamp01(s / 100.0),
    )
}

/// RGB `[0, 1]` -> HSV (`H` en degrés `[0, 360)`, `S` et `V` en `%` `[0, 100]`).
#[inline]
pub fn rgb01_to_hsv(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let maxc = r.max(g).max(b);
    let minc = r.min(g).min(b);
    let rangec = maxc - minc;
    let v = maxc;
    if minc == maxc {
        return (0.0, 0.0, v * 100.0);
    }
    let s = rangec / maxc;
    let rc = (maxc - r) / rangec;
    let gc = (maxc - g) / rangec;
    let bc = (maxc - b) / rangec;
    let mut h = if r == maxc {
        bc - gc
    } else if g == maxc {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    h = (h / 6.0).rem_euclid(1.0);
    ((h * 360.0).rem_euclid(360.0), s * 100.0, v * 100.0)
}

/// HSV (`H` en degrés, `S`/`V` en `%`) -> RGB `[0, 1]`.
#[inline]
pub fn hsv_to_rgb01(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
    let h01 = h.rem_euclid(360.0) / 360.0;
    let s01 = clamp01(s / 100.0);
    let v01 = clamp01(v / 100.0);
    if s01 == 0.0 {
        return (v01, v01, v01);
    }
    let i = (h01 * 6.0).floor();
    let f = (h01 * 6.0) - i;
    let p = v01 * (1.0 - s01);
    let q = v01 * (1.0 - s01 * f);
    let t = v01 * (1.0 - s01 * (1.0 - f));
    match (i as i32).rem_euclid(6) {
        0 => (v01, t, p),
        1 => (q, v01, p),
        2 => (p, v01, t),
        3 => (p, q, v01),
        4 => (t, p, v01),
        _ => (v01, p, q),
    }
}

#[inline]
pub fn hex_to_hsl(hex_code: &str) -> (f64, f64, f64) {
    let (r, g, b) = hex_to_rgb01(hex_code);
    rgb01_to_hsl(r, g, b)
}

#[inline]
pub fn hex_to_hsv(hex_code: &str) -> (f64, f64, f64) {
    let (r, g, b) = hex_to_rgb01(hex_code);
    rgb01_to_hsv(r, g, b)
}

#[inline]
pub fn hsl_to_hex(h: f64, s: f64, l: f64) -> String {
    let (r, g, b) = hsl_to_rgb01(h, s, l);
    rgb01_to_hex(r, g, b)
}

#[inline]
pub fn hsv_to_hex(h: f64, s: f64, v: f64) -> String {
    let (r, g, b) = hsv_to_rgb01(h, s, v);
    rgb01_to_hex(r, g, b)
}

#[inline]
pub fn format_rgb(hex_code: &str) -> String {
    let (r, g, b) = hex_to_rgb256(hex_code);
    format!("rgb({r}, {g}, {b})")
}

#[inline]
pub fn format_hsl(hex_code: &str) -> String {
    let (h, s, l) = hex_to_hsl(hex_code);
    format!("hsl({}, {}%, {}%)", h.round() as i32, s.round() as i32, l.round() as i32)
}

#[inline]
pub fn format_hsv(hex_code: &str) -> String {
    let (h, s, v) = hex_to_hsv(hex_code);
    format!("hsv({}, {}%, {}%)", h.round() as i32, s.round() as i32, v.round() as i32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorFormat {
    HEX,
    RGB,
    HSL,
    HSV,
}

impl ColorFormat {
    pub const ALL: [ColorFormat; 4] = [
        ColorFormat::HEX,
        ColorFormat::RGB,
        ColorFormat::HSL,
        ColorFormat::HSV,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ColorFormat::HEX => "HEX",
            ColorFormat::RGB => "RGB",
            ColorFormat::HSL => "HSL",
            ColorFormat::HSV => "HSV",
        }
    }
}

impl fmt::Display for ColorFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[inline]
pub fn format_color(hex_code: &str, fmt: ColorFormat) -> String {
    match fmt {
        ColorFormat::HEX => {
            let (r, g, b) = hex_to_rgb256(hex_code);
            rgb256_to_hex(r, g, b)
        }
        ColorFormat::RGB => format_rgb(hex_code),
        ColorFormat::HSL => format_hsl(hex_code),
        ColorFormat::HSV => format_hsv(hex_code),
    }
}

/// Retourne `"black"` ou `"white"` selon la luminosité perçue de `hex_code`.
#[inline]
pub fn readable_text_color(hex_code: &str) -> &'static str {
    let (r, g, b) = hex_to_rgb256(hex_code);
    // Évite la division flottante : 0.6 * 255 * 1000 = 153_000
    let perceived_scaled = 299u32 * (r as u32) + 587u32 * (g as u32) + 114u32 * (b as u32);
    if perceived_scaled > 153_000 {
        "black"
    } else {
        "white"
    }
}

/// Retourne directement `(r, g, b)` du texte lisible (`(0,0,0)` ou `(255,255,255)`).
#[inline]
pub fn readable_text_rgb(hex_code: &str) -> (u8, u8, u8) {
    if readable_text_color(hex_code) == "black" {
        (0, 0, 0)
    } else {
        (255, 255, 255)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwatchSpec {
    pub role: String,
    pub hex: String,
}

impl SwatchSpec {
    #[inline]
    pub fn new(role: impl Into<String>, hex: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            hex: hex.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemeType {
    Golden,
    Complementary,
    Analogous,
    Triadic,
    Tetradic,
    SplitComplementary,
    Monochromatic,
}

impl SchemeType {
    pub const ALL: [SchemeType; 7] = [
        SchemeType::Golden,
        SchemeType::Complementary,
        SchemeType::Analogous,
        SchemeType::Triadic,
        SchemeType::Tetradic,
        SchemeType::SplitComplementary,
        SchemeType::Monochromatic,
    ];

    pub fn id(self) -> &'static str {
        match self {
            SchemeType::Golden => "golden",
            SchemeType::Complementary => "complementary",
            SchemeType::Analogous => "analogous",
            SchemeType::Triadic => "triadic",
            SchemeType::Tetradic => "tetradic",
            SchemeType::SplitComplementary => "split_complementary",
            SchemeType::Monochromatic => "monochromatic",
        }
    }

    pub fn from_id(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "complementary" => SchemeType::Complementary,
            "analogous" => SchemeType::Analogous,
            "triadic" => SchemeType::Triadic,
            "tetradic" => SchemeType::Tetradic,
            "split_complementary" => SchemeType::SplitComplementary,
            "monochromatic" => SchemeType::Monochromatic,
            _ => SchemeType::Golden,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SchemeType::Golden => "Golden (nombre d'or, 137.5°)",
            SchemeType::Complementary => "Complémentaire (180°)",
            SchemeType::Analogous => "Analogue (±30°)",
            SchemeType::Triadic => "Triadique (120° / 240°)",
            SchemeType::Tetradic => "Tétradique / carré (90° / 180° / 270°)",
            SchemeType::SplitComplementary => "Split-complémentaire (150° / 210°)",
            SchemeType::Monochromatic => "Monochromatique",
        }
    }
}

#[inline]
fn push_tri_tone(out: &mut Vec<SwatchSpec>, h: f64, l: f64, s: f64, label: &str) {
    let l_dark = clamp01(l * 0.40);
    let l_light = clamp01(l + (1.0 - l) * 0.50);

    let (r0, g0, b0) = hls01_to_rgb(h, l, s);
    let (r1, g1, b1) = hls01_to_rgb(h, l_dark, s);
    let (r2, g2, b2) = hls01_to_rgb(h, l_light, s);

    out.push(SwatchSpec::new(label, rgb01_to_hex(r0, g0, b0)));
    out.push(SwatchSpec::new(format!("{label} -60%"), rgb01_to_hex(r1, g1, b1)));
    out.push(SwatchSpec::new(format!("{label} +50%"), rgb01_to_hex(r2, g2, b2)));
}

#[inline]
fn push_neutrals(out: &mut Vec<SwatchSpec>, h: f64, s: f64) {
    let s_neutral = clamp01(s * 0.15);
    let (rb, gb, bb) = hls01_to_rgb(h, 0.95, s_neutral);
    let (rt, gt, bt) = hls01_to_rgb(h, 0.15, s_neutral);
    out.push(SwatchSpec::new(NEUTRAL_ROLE_BG, rgb01_to_hex(rb, gb, bb)));
    out.push(SwatchSpec::new(NEUTRAL_ROLE_TEXT, rgb01_to_hex(rt, gt, bt)));
}

fn hue_scheme(base_hex: &str, offsets_labels: &[(f64, &str)]) -> Vec<SwatchSpec> {
    let (r, g, b) = hex_to_rgb01(base_hex);
    let (h, l, s) = rgb_to_hls01(r, g, b);
    let mut swatches = Vec::with_capacity(offsets_labels.len() * 3 + 2);
    for &(offset_deg, label) in offsets_labels {
        let hue = (h + offset_deg / 360.0).rem_euclid(1.0);
        push_tri_tone(&mut swatches, hue, l, s, label);
    }
    push_neutrals(&mut swatches, h, s);
    swatches
}

pub fn generate_golden(base_hex: &str) -> Vec<SwatchSpec> {
    hue_scheme(base_hex, &[(0.0, "Base"), (137.5, "Sec. (137.5°)")])
}

pub fn generate_complementary(base_hex: &str) -> Vec<SwatchSpec> {
    hue_scheme(base_hex, &[(0.0, "Base"), (180.0, "Complément")])
}

pub fn generate_analogous(base_hex: &str) -> Vec<SwatchSpec> {
    hue_scheme(
        base_hex,
        &[(-30.0, "Analogue -30°"), (0.0, "Base"), (30.0, "Analogue +30°")],
    )
}

pub fn generate_triadic(base_hex: &str) -> Vec<SwatchSpec> {
    hue_scheme(
        base_hex,
        &[(0.0, "Base"), (120.0, "Triade 120°"), (240.0, "Triade 240°")],
    )
}

pub fn generate_tetradic(base_hex: &str) -> Vec<SwatchSpec> {
    hue_scheme(
        base_hex,
        &[
            (0.0, "Base"),
            (90.0, "Carré 90°"),
            (180.0, "Carré 180°"),
            (270.0, "Carré 270°"),
        ],
    )
}

pub fn generate_split_complementary(base_hex: &str) -> Vec<SwatchSpec> {
    hue_scheme(
        base_hex,
        &[(0.0, "Base"), (150.0, "Split 150°"), (210.0, "Split 210°")],
    )
}

pub fn generate_monochromatic(base_hex: &str) -> Vec<SwatchSpec> {
    let (r, g, b) = hex_to_rgb01(base_hex);
    let (h, l, s) = rgb_to_hls01(r, g, b);
    let steps = [
        0.15,
        0.30,
        0.45,
        l,
        clamp01(l + (1.0 - l) * 0.35),
        clamp01(l + (1.0 - l) * 0.65),
        0.90,
    ];
    let mut swatches = Vec::with_capacity(9);
    for (i, &step_l) in steps.iter().enumerate() {
        let label = if (step_l - l).abs() < 1e-6 {
            "Base".to_string()
        } else {
            format!("Ton {}", i + 1)
        };
        let (sr, sg, sb) = hls01_to_rgb(h, clamp01(step_l), s);
        swatches.push(SwatchSpec::new(label, rgb01_to_hex(sr, sg, sb)));
    }
    push_neutrals(&mut swatches, h, s);
    swatches
}

pub fn generate_palette(base_hex: &str, scheme: SchemeType) -> Vec<SwatchSpec> {
    match scheme {
        SchemeType::Golden => generate_golden(base_hex),
        SchemeType::Complementary => generate_complementary(base_hex),
        SchemeType::Analogous => generate_analogous(base_hex),
        SchemeType::Triadic => generate_triadic(base_hex),
        SchemeType::Tetradic => generate_tetradic(base_hex),
        SchemeType::SplitComplementary => generate_split_complementary(base_hex),
        SchemeType::Monochromatic => generate_monochromatic(base_hex),
    }
}

pub fn generate_all_schemes(base_hex: &str) -> Vec<(SchemeType, Vec<SwatchSpec>)> {
    SchemeType::ALL
        .iter()
        .map(|&scheme| (scheme, generate_palette(base_hex, scheme)))
        .collect()
}

pub fn apply_global_adjustment(base_hex: &str, sat_delta: f64, light_delta: f64) -> String {
    let (h, s, l) = hex_to_hsl(base_hex);
    hsl_to_hex(h, clamp(s + sat_delta, 0.0, 100.0), clamp(l + light_delta, 0.0, 100.0))
}

pub fn random_hex() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let h: f64 = rng.random_range(0.0..360.0);
    let s: f64 = rng.random_range(45.0..85.0);
    let l: f64 = rng.random_range(35.0..65.0);
    hsl_to_hex(h, s, l)
}

pub fn role_description(role: &str) -> &'static str {
    let role_low = role.to_lowercase();
    if role_low.contains("fond neutre") {
        "Couleur de fond neutre suggérée"
    } else if role_low.contains("texte neutre") {
        "Couleur de texte neutre suggérée"
    } else if role.contains("-60%") {
        "Variante assombrie (ombre)"
    } else if role.contains("+50%") {
        "Variante éclaircie (teinte)"
    } else if role_low == "base" {
        "Couleur principale de la palette"
    } else {
        "Couleur harmonique dérivée de la base"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_rgb_roundtrip() {
        for hexcode in ["#000000", "#FFFFFF", "#F1C40F", "#3498DB", "#1A2B3C"] {
            let (r, g, b) = hex_to_rgb256(hexcode);
            assert_eq!(rgb256_to_hex(r, g, b), hexcode);
        }
    }

    #[test]
    fn test_hsl_roundtrip_approx() {
        let hexcode = "#3498DB";
        let (h, s, l) = hex_to_hsl(hexcode);
        let back = hsl_to_hex(h, s, l);
        assert_eq!(back, hexcode);
    }

    #[test]
    fn test_normalize_hex() {
        assert_eq!(normalize_hex("f1c40f").unwrap(), "#F1C40F");
        assert_eq!(normalize_hex("#f1c40f").unwrap(), "#F1C40F");
        assert!(normalize_hex("not-a-color").is_err());
    }

    #[test]
    fn test_format_color() {
        assert!(format_color("#3498DB", ColorFormat::RGB).starts_with("rgb("));
        assert!(format_color("#3498DB", ColorFormat::HSL).starts_with("hsl("));
        assert!(format_color("#3498DB", ColorFormat::HSV).starts_with("hsv("));
        assert_eq!(format_color("#3498db", ColorFormat::HEX), "#3498DB");
    }

    #[test]
    fn test_complementary_hue_is_opposite() {
        let swatches = generate_complementary("#FF0000");
        let base = swatches.iter().find(|s| s.role == "Base").unwrap();
        let comp = swatches.iter().find(|s| s.role == "Complément").unwrap();
        let (h_base, _, _) = hex_to_hsl(&base.hex);
        let (h_comp, _, _) = hex_to_hsl(&comp.hex);
        assert!(((h_comp - h_base).abs() % 360.0 - 180.0).abs() <= 1.0);
    }

    #[test]
    fn test_all_schemes_have_neutrals() {
        for scheme in SchemeType::ALL {
            let swatches = generate_palette("#2ECC71", scheme);
            assert!(swatches.iter().any(|s| s.role == NEUTRAL_ROLE_BG));
            assert!(swatches.iter().any(|s| s.role == NEUTRAL_ROLE_TEXT));
        }
    }

    #[test]
    fn test_tetradic_has_fourteen_swatches() {
        let swatches = generate_tetradic("#2ECC71");
        assert_eq!(swatches.len(), 14);
    }

    #[test]
    fn test_random_hex_is_valid() {
        for _ in 0..20 {
            assert!(is_valid_hex(&random_hex()));
        }
    }
}
