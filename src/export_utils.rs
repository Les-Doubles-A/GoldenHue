//! ExportManager : exporte une palette de couleurs vers CSV, JSON, CSS, SCSS, TXT,
//! Adobe Swatch Exchange (`.ase`) et image `.png` (avec moteur de rendu bitmap 5x7 embarqué).

use crate::color_utils::{
    SwatchSpec, format_hsl, format_hsv, format_rgb, hex_to_rgb01, hex_to_rgb256, readable_text_rgb,
};
use image::{ImageFormat, Rgb, RgbImage};
use serde::Serialize;
use std::fs;
use std::io::Cursor;
use std::path::Path;

pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = false;
    for ch in name.trim().chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "couleur".to_string()
    } else {
        out
    }
}

#[derive(Serialize)]
struct JsonPalette<'a> {
    palette: &'a str,
    colors: Vec<JsonColor>,
}

#[derive(Serialize)]
struct JsonColor {
    role: String,
    hex: String,
    rgb: String,
    hsl: String,
    hsv: String,
}

pub struct ExportManager {
    pub colors: Vec<SwatchSpec>,
    pub palette_name: String,
}

impl ExportManager {
    pub fn new(colors: Vec<SwatchSpec>, palette_name: impl Into<String>) -> Self {
        Self {
            colors,
            palette_name: palette_name.into(),
        }
    }

    pub fn to_txt(&self) -> String {
        let mut out = String::with_capacity(self.colors.len() * 32);
        for c in &self.colors {
            out.push_str(&c.role);
            out.push_str(" : ");
            out.push_str(&c.hex.to_uppercase());
            out.push('\n');
        }
        out
    }

    pub fn to_csv(&self) -> String {
        let mut out = String::with_capacity(self.colors.len() * 64 + 32);
        out.push_str("role,hex,rgb,hsl,hsv\n");
        for c in &self.colors {
            let hex = c.hex.to_uppercase();
            let rgb = format_rgb(&hex);
            let hsl = format_hsl(&hex);
            let hsv = format_hsv(&hex);
            // Guillemets autour des champs contenant des virgules
            out.push_str(&format!(
                "{},{},\"{}\",\"{}\",\"{}\"\n",
                csv_escape(&c.role),
                hex,
                rgb,
                hsl,
                hsv
            ));
        }
        out
    }

    pub fn to_json(&self) -> String {
        let colors = self
            .colors
            .iter()
            .map(|c| {
                let hex = c.hex.to_uppercase();
                JsonColor {
                    role: c.role.clone(),
                    rgb: format_rgb(&hex),
                    hsl: format_hsl(&hex),
                    hsv: format_hsv(&hex),
                    hex,
                }
            })
            .collect();
        let payload = JsonPalette {
            palette: &self.palette_name,
            colors,
        };
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    }

    pub fn to_css(&self) -> String {
        let mut out = String::from(":root {\n");
        for c in &self.colors {
            out.push_str(&format!("  --{}: {};\n", slugify(&c.role), c.hex.to_uppercase()));
        }
        out.push_str("}\n");
        out
    }

    pub fn to_scss(&self) -> String {
        let mut out = String::with_capacity(self.colors.len() * 40);
        for c in &self.colors {
            out.push_str(&format!(
                "$goldenhue-{}: {};\n",
                slugify(&c.role),
                c.hex.to_uppercase()
            ));
        }
        out
    }

    /// Construit un fichier binaire Adobe Swatch Exchange (`.ase`) version 1.0.
    pub fn to_ase_bytes(&self) -> Vec<u8> {
        let mut blocks = Vec::with_capacity(self.colors.len() * 64);
        let mut block_count = 0u32;

        for c in &self.colors {
            let (r, g, b) = hex_to_rgb01(&c.hex);
            let name_chars: Vec<u16> = c.role.chars().take(63).map(|ch| ch as u16).collect();
            let name_len = (name_chars.len() + 1) as u16; // +1 pour le terminateur nul UTF-16BE

            let mut body = Vec::with_capacity(2 + (name_len as usize) * 2 + 4 + 12 + 2);
            body.extend_from_slice(&name_len.to_be_bytes());
            for code_unit in name_chars {
                body.extend_from_slice(&code_unit.to_be_bytes());
            }
            body.extend_from_slice(&0u16.to_be_bytes()); // terminateur UTF-16BE
            body.extend_from_slice(b"RGB ");
            body.extend_from_slice(&(r as f32).to_be_bytes());
            body.extend_from_slice(&(g as f32).to_be_bytes());
            body.extend_from_slice(&(b as f32).to_be_bytes());
            body.extend_from_slice(&0u16.to_be_bytes()); // 0 = Global color type

            blocks.extend_from_slice(&0x0001u16.to_be_bytes()); // bloc couleur
            blocks.extend_from_slice(&(body.len() as u32).to_be_bytes());
            blocks.extend_from_slice(&body);
            block_count += 1;
        }

        let mut out = Vec::with_capacity(12 + blocks.len());
        out.extend_from_slice(b"ASEF");
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&block_count.to_be_bytes());
        out.extend_from_slice(&blocks);
        out
    }

    /// Génère une image PNG représentant la palette (blocs + libellés dessinés).
    pub fn to_png_bytes(&self, swatch_width: u32, height: u32) -> Result<Vec<u8>, String> {
        let n = self.colors.len().max(1) as u32;
        let width = swatch_width * n;
        let mut img = RgbImage::from_pixel(width, height, Rgb([255, 255, 255]));

        for (i, c) in self.colors.iter().enumerate() {
            let x0 = (i as u32) * swatch_width;
            let x1 = ((i + 1) as u32) * swatch_width;
            let (r, g, b) = hex_to_rgb256(&c.hex);
            let fill = Rgb([r, g, b]);

            for y in 0..height {
                for x in x0..x1 {
                    img.put_pixel(x, y, fill);
                }
            }

            let (tr, tg, tb) = readable_text_rgb(&c.hex);
            let text_col = Rgb([tr, tg, tb]);

            if height >= 45 {
                draw_bitmap_text(&mut img, x0 + 10, height - 42, &c.role, text_col, x1 - 4);
                draw_bitmap_text(
                    &mut img,
                    x0 + 10,
                    height - 22,
                    &c.hex.to_uppercase(),
                    text_col,
                    x1 - 4,
                );
            }
        }

        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| format!("Erreur d'encodage PNG : {e}"))?;
        Ok(buf)
    }

    pub fn save(&self, path: impl AsRef<Path>, fmt: &str) -> Result<(), String> {
        let path = path.as_ref();
        match fmt.trim().to_ascii_lowercase().as_str() {
            "csv" => fs::write(path, self.to_csv()).map_err(|e| e.to_string()),
            "json" => fs::write(path, self.to_json()).map_err(|e| e.to_string()),
            "css" => fs::write(path, self.to_css()).map_err(|e| e.to_string()),
            "scss" => fs::write(path, self.to_scss()).map_err(|e| e.to_string()),
            "txt" => fs::write(path, self.to_txt()).map_err(|e| e.to_string()),
            "ase" => fs::write(path, self.to_ase_bytes()).map_err(|e| e.to_string()),
            "png" => {
                let bytes = self.to_png_bytes(160, 220)?;
                fs::write(path, bytes).map_err(|e| e.to_string())
            }
            other => Err(format!("Format d'export inconnu : {other}")),
        }
    }
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Normalise un caractère accentué vers son équivalent ASCII pour le rendu bitmap 5x7.
fn ascii_fallback(ch: char) -> char {
    match ch {
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'à' | 'â' | 'ä' => 'a',
        'À' | 'Â' | 'Ä' => 'A',
        'î' | 'ï' => 'i',
        'Î' | 'Ï' => 'I',
        'ô' | 'ö' => 'o',
        'Ô' | 'Ö' => 'O',
        'ù' | 'û' | 'ü' => 'u',
        'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        '°' => 'o',
        '±' => '+',
        _ if ch.is_ascii() => ch,
        _ => '?',
    }
}

/// Police matricielle 5x7 compacte pour le rendu PNG autonome sans dépendance système.
fn glyph_5x7(ch: char) -> [u8; 7] {
    match ascii_fallback(ch).to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        '%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        _ => [0; 7],
    }
}

fn draw_bitmap_text(img: &mut RgbImage, x: u32, y: u32, text: &str, color: Rgb<u8>, max_x: u32) {
    let mut cursor_x = x;
    let scale = 2u32;
    for ch in text.chars() {
        if cursor_x + 5 * scale >= max_x {
            break;
        }
        let rows = glyph_5x7(ch);
        for (ry, &bits) in rows.iter().enumerate() {
            for rx in 0..5u32 {
                if (bits >> (4 - rx)) & 1 == 1 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = cursor_x + rx * scale + dx;
                            let py = y + (ry as u32) * scale + dy;
                            if px < img.width() && py < img.height() {
                                img.put_pixel(px, py, color);
                            }
                        }
                    }
                }
            }
        }
        cursor_x += 6 * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manager() -> ExportManager {
        ExportManager::new(
            vec![
                SwatchSpec::new("Base", "#F1C40F"),
                SwatchSpec::new("Complément", "#0F4FF1"),
            ],
            "TestPalette",
        )
    }

    #[test]
    fn test_csv_has_header_and_rows() {
        let csv_text = sample_manager().to_csv();
        assert!(csv_text.contains("role,hex,rgb,hsl,hsv"));
        assert!(csv_text.contains("#F1C40F"));
    }

    #[test]
    fn test_json_structure() {
        let json_str = sample_manager().to_json();
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(val["palette"], "TestPalette");
        assert_eq!(val["colors"].as_array().unwrap().len(), 2);
        assert_eq!(val["colors"][0]["hex"], "#F1C40F");
    }

    #[test]
    fn test_css_variables() {
        let css = sample_manager().to_css();
        assert!(css.contains(":root {"));
        assert!(css.contains("--base: #F1C40F;"));
    }

    #[test]
    fn test_scss_variables() {
        let scss = sample_manager().to_scss();
        assert!(scss.contains("$goldenhue-base: #F1C40F;"));
    }

    #[test]
    fn test_txt() {
        let txt = sample_manager().to_txt();
        assert!(txt.contains("Base : #F1C40F"));
    }

    #[test]
    fn test_ase_bytes_header() {
        let data = sample_manager().to_ase_bytes();
        assert_eq!(&data[..4], b"ASEF");
    }

    #[test]
    fn test_png_bytes_is_valid_png() {
        let data = sample_manager().to_png_bytes(160, 220).unwrap();
        assert_eq!(&data[..8], b"\x89PNG\r\n\x1a\n");
    }
}
