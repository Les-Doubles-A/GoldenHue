//! Simulation de daltonisme (protanopie, deutéranopie, tritanopie) par
//! transformation matricielle 3x3 sur les composantes RGB.

use crate::color_utils::{clamp01, hex_to_rgb01, rgb01_to_hex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorblindType {
    Protanopia,
    Deuteranopia,
    Tritanopia,
}

impl ColorblindType {
    pub const ALL: [ColorblindType; 3] = [
        ColorblindType::Protanopia,
        ColorblindType::Deuteranopia,
        ColorblindType::Tritanopia,
    ];

    #[inline]
    pub const fn label(self) -> &'static str {
        match self {
            ColorblindType::Protanopia => "Protanopie (déficit rouge)",
            ColorblindType::Deuteranopia => "Deutéranopie (déficit vert)",
            ColorblindType::Tritanopia => "Tritanopie (déficit bleu)",
        }
    }

    #[inline]
    const fn matrix(self) -> [[f64; 3]; 3] {
        match self {
            ColorblindType::Protanopia => [
                [0.567, 0.433, 0.000],
                [0.558, 0.442, 0.000],
                [0.000, 0.242, 0.758],
            ],
            ColorblindType::Deuteranopia => [
                [0.625, 0.375, 0.000],
                [0.700, 0.300, 0.000],
                [0.000, 0.300, 0.700],
            ],
            ColorblindType::Tritanopia => [
                [0.950, 0.050, 0.000],
                [0.000, 0.433, 0.567],
                [0.000, 0.475, 0.525],
            ],
        }
    }
}

/// Retourne l'apparence approximative de `hex_code` pour le type de daltonisme donné.
#[inline]
pub fn simulate(hex_code: &str, cb_type: ColorblindType) -> String {
    let (r, g, b) = hex_to_rgb01(hex_code);
    let m = cb_type.matrix();
    let sr = clamp01(m[0][0] * r + m[0][1] * g + m[0][2] * b);
    let sg = clamp01(m[1][0] * r + m[1][1] * g + m[1][2] * b);
    let sb = clamp01(m[2][0] * r + m[2][1] * g + m[2][2] * b);
    rgb01_to_hex(sr, sg, sb)
}

pub fn simulate_all(hex_code: &str) -> [(ColorblindType, String); 3] {
    [
        (ColorblindType::Protanopia, simulate(hex_code, ColorblindType::Protanopia)),
        (ColorblindType::Deuteranopia, simulate(hex_code, ColorblindType::Deuteranopia)),
        (ColorblindType::Tritanopia, simulate(hex_code, ColorblindType::Tritanopia)),
    ]
}
