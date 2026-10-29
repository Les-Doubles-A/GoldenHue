//! Base de correspondance nom de couleur (français) -> code hexadécimal,
//! avec recherche exacte et recherche floue rapide (similarité de sous-chaînes / Ratcliff-Obershelp).

pub const COLOR_NAMES: &[(&str, &str)] = &[
    ("rouge", "#E74C3C"),
    ("corail", "#FF7F50"),
    ("orange", "#F39C12"),
    ("ambre", "#FFBF00"),
    ("jaune", "#F1C40F"),
    ("citron", "#F4D03F"),
    ("or", "#D4AF37"),
    ("doré", "#D4AF37"),
    ("olive", "#808000"),
    ("vert", "#2ECC71"),
    ("vert forêt", "#228B22"),
    ("menthe", "#3EB489"),
    ("émeraude", "#50C878"),
    ("turquoise", "#1ABC9C"),
    ("sarcelle", "#008080"),
    ("cyan", "#00FFFF"),
    ("bleu ciel", "#87CEEB"),
    ("bleu", "#3498DB"),
    ("bleu marine", "#001F54"),
    ("indigo", "#4B0082"),
    ("violet", "#9B59B6"),
    ("mauve", "#B39DDB"),
    ("lavande", "#B57EDC"),
    ("magenta", "#FF00FF"),
    ("rose", "#FF69B4"),
    ("rose poudré", "#F4C2C2"),
    ("fuchsia", "#FF00A6"),
    ("bordeaux", "#800020"),
    ("marron", "#8B4513"),
    ("brun", "#795548"),
    ("beige", "#F5F5DC"),
    ("sable", "#C2B280"),
    ("crème", "#FFFDD0"),
    ("ivoire", "#FFFFF0"),
    ("blanc", "#FFFFFF"),
    ("gris", "#95A5A6"),
    ("gris ardoise", "#708090"),
    ("anthracite", "#2C3E50"),
    ("noir", "#000000"),
    ("saumon", "#FA8072"),
    ("pêche", "#FFDAB9"),
    ("abricot", "#FBCEB1"),
    ("prune", "#8E4585"),
    ("aubergine", "#3B0910"),
    ("chocolat", "#7B3F00"),
    ("kaki", "#78866B"),
    ("moutarde", "#FFDB58"),
    ("rouille", "#B7410E"),
    ("terracotta", "#E2725B"),
];

/// Calcule le ratio de similarité `2 * M / T` (style `difflib.SequenceMatcher`) entre deux tranches de `char`.
fn sequence_similarity(a: &[char], b: &[char]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let matches = count_matching_chars(a, b);
    (2.0 * matches as f64) / ((a.len() + b.len()) as f64)
}

fn count_matching_chars(a: &[char], b: &[char]) -> usize {
    let mut best_len = 0;
    let mut best_a = 0;
    let mut best_b = 0;

    for ia in 0..a.len() {
        for ib in 0..b.len() {
            let mut k = 0;
            while ia + k < a.len() && ib + k < b.len() && a[ia + k] == b[ib + k] {
                k += 1;
            }
            if k > best_len {
                best_len = k;
                best_a = ia;
                best_b = ib;
            }
        }
    }

    if best_len == 0 {
        return 0;
    }

    let left = if best_a > 0 && best_b > 0 {
        count_matching_chars(&a[..best_a], &b[..best_b])
    } else {
        0
    };
    let right = if best_a + best_len < a.len() && best_b + best_len < b.len() {
        count_matching_chars(&a[best_a + best_len..], &b[best_b + best_len..])
    } else {
        0
    };
    best_len + left + right
}

/// Retourne le code `#RRGGBB` correspondant au nom donné (exact ou approché avec seuil `0.6`).
pub fn lookup_color_name(name: &str) -> Option<&'static str> {
    let key = name.trim().to_lowercase();
    if key.is_empty() {
        return None;
    }

    for &(cname, hex) in COLOR_NAMES {
        if cname == key {
            return Some(hex);
        }
    }

    let key_chars: Vec<char> = key.chars().collect();
    let mut best_score = 0.6f64;
    let mut best_hex: Option<&'static str> = None;

    for &(cname, hex) in COLOR_NAMES {
        let cand_chars: Vec<char> = cname.chars().collect();
        let score = sequence_similarity(&key_chars, &cand_chars);
        if score >= best_score {
            best_score = score;
            best_hex = Some(hex);
        }
    }

    best_hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_and_fuzzy_lookup() {
        assert_eq!(lookup_color_name("corail"), Some("#FF7F50"));
        assert_eq!(lookup_color_name("  Bleu Marine "), Some("#001F54"));
        assert_eq!(lookup_color_name("coraille"), Some("#FF7F50"));
        assert_eq!(lookup_color_name("zzzzzzzz"), None);
    }
}
