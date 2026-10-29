//! Calcul ultra-optimisé du ratio de contraste WCAG 2.x entre deux couleurs,
//! avec table de linéarisation sRGB précalculée (`LazyLock` / table 256 entrées)
//! éliminant tout appel à `powf(2.4)` lors du rendu.

use crate::color_utils::hex_to_rgb256;
use std::sync::LazyLock;

/// Table précalculée des 256 valeurs sRGB -> luminance linéaire WCAG.
static SRGB_LINEAR_LUT: LazyLock<[f64; 256]> = LazyLock::new(|| {
    let mut table = [0.0f64; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let c = i as f64 / 255.0;
        *slot = if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        };
    }
    table
});

/// Luminance relative WCAG d'une couleur (`0.0` = noir, `1.0` = blanc) en O(1) par table lookup.
#[inline]
pub fn relative_luminance_rgb(r: u8, g: u8, b: u8) -> f64 {
    let lut = &*SRGB_LINEAR_LUT;
    0.2126 * lut[r as usize] + 0.7152 * lut[g as usize] + 0.0722 * lut[b as usize]
}

/// Luminance relative WCAG d'un code hexadécimal.
#[inline]
pub fn relative_luminance(hex_code: &str) -> f64 {
    let (r, g, b) = hex_to_rgb256(hex_code);
    relative_luminance_rgb(r, g, b)
}

/// Ratio de contraste WCAG entre deux couleurs, dans `[1.0, 21.0]`.
#[inline]
pub fn contrast_ratio(hex_a: &str, hex_b: &str) -> f64 {
    let l1 = relative_luminance(hex_a);
    let l2 = relative_luminance(hex_b);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContrastResult {
    pub ratio: f64,
    pub aa_normal: bool,
    pub aaa_normal: bool,
    pub aa_large: bool,
    pub aaa_large: bool,
}

impl ContrastResult {
    /// Le meilleur badge atteint pour du texte normal.
    #[inline]
    pub fn badge(&self) -> &'static str {
        if self.aaa_normal {
            "AAA"
        } else if self.aa_normal {
            "AA"
        } else if self.aa_large {
            "AA (grand texte)"
        } else {
            "Échec"
        }
    }
}

/// Évalue le contraste entre deux couleurs selon les seuils WCAG 2.x.
#[inline]
pub fn evaluate_contrast(hex_a: &str, hex_b: &str) -> ContrastResult {
    let ratio = contrast_ratio(hex_a, hex_b);
    ContrastResult {
        ratio,
        aa_normal: ratio >= 4.5,
        aaa_normal: ratio >= 7.0,
        aa_large: ratio >= 3.0,
        aaa_large: ratio >= 4.5,
    }
}

/// Retourne `"#000000"` ou `"#FFFFFF"`, celui offrant le meilleur contraste.
#[inline]
pub fn best_contrasting_bw(hex_code: &str) -> &'static str {
    let black_ratio = contrast_ratio(hex_code, "#000000");
    let white_ratio = contrast_ratio(hex_code, "#FFFFFF");
    if black_ratio >= white_ratio {
        "#000000"
    } else {
        "#FFFFFF"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_white_max_contrast() {
        let ratio = contrast_ratio("#000000", "#FFFFFF");
        assert!((ratio - 21.0).abs() <= 0.01);
    }

    #[test]
    fn test_same_color_min_contrast() {
        let ratio = contrast_ratio("#3498DB", "#3498DB");
        assert!((ratio - 1.0).abs() <= 0.01);
    }

    #[test]
    fn test_evaluate_contrast_badges() {
        let result = evaluate_contrast("#000000", "#FFFFFF");
        assert!(result.aaa_normal);
        assert_eq!(result.badge(), "AAA");
    }

    #[test]
    fn test_best_contrasting_bw() {
        assert_eq!(best_contrasting_bw("#000000"), "#FFFFFF");
        assert_eq!(best_contrasting_bw("#FFFFFF"), "#000000");
    }
}
