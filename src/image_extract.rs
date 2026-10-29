//! Extraction ultra-rapide des couleurs dominantes d'une image par K-Means.
//!
//! Optimisations :
//! - Redimensionnement rapide (`FilterType::Triangle`) à `max_dim = 150`.
//! - Pixels représentés en tableau contigu `[f32; 3]` sur la pile/tas sans matrice intermédiaire `N x K`
//!   (calcul de distance en place avec arrêt anticipé dès convergence `atol = 0.5`).
//! - Initialisation déterministe (Xorshift64 `seed = 42`).

use crate::color_utils::rgb256_to_hex;
use std::path::Path;

struct XorShift64(u64);

impl XorShift64 {
    #[inline]
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    #[inline]
    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }
}

#[inline]
fn dist_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    dr * dr + dg * dg + db * db
}

/// K-Means rapide sans allocation dans la boucle d'itération.
fn kmeans(pixels: &[[f32; 3]], k: usize, iterations: usize, seed: u64) -> Vec<[f32; 3]> {
    let n = pixels.len();
    let k = k.min(n).max(1);
    let mut rng = XorShift64::new(seed);

    // Sélection sans remise par réservoir/permutation partielle
    let mut indices: Vec<usize> = (0..n).collect();
    for i in 0..k {
        let j = i + rng.next_usize(n - i);
        indices.swap(i, j);
    }

    let mut centroids: Vec<[f32; 3]> = indices[..k].iter().map(|&idx| pixels[idx]).collect();
    let mut sums = vec![[0.0f64; 3]; k];
    let mut counts = vec![0usize; k];

    for _ in 0..iterations {
        for s in &mut sums {
            *s = [0.0; 3];
        }
        counts.fill(0);

        for &px in pixels {
            let mut best_c = 0;
            let mut best_d = dist_sq(px, centroids[0]);
            for (ci, &centroid) in centroids.iter().enumerate().skip(1) {
                let d = dist_sq(px, centroid);
                if d < best_d {
                    best_d = d;
                    best_c = ci;
                }
            }
            sums[best_c][0] += px[0] as f64;
            sums[best_c][1] += px[1] as f64;
            sums[best_c][2] += px[2] as f64;
            counts[best_c] += 1;
        }

        let mut converged = true;
        for i in 0..k {
            if counts[i] > 0 {
                let inv = 1.0 / (counts[i] as f64);
                let new_c = [
                    (sums[i][0] * inv) as f32,
                    (sums[i][1] * inv) as f32,
                    (sums[i][2] * inv) as f32,
                ];
                if (new_c[0] - centroids[i][0]).abs() > 0.5
                    || (new_c[1] - centroids[i][1]).abs() > 0.5
                    || (new_c[2] - centroids[i][2]).abs() > 0.5
                {
                    converged = false;
                }
                centroids[i] = new_c;
            }
        }

        if converged {
            break;
        }
    }

    // Comptage final et tri par prévalence décroissante
    counts.fill(0);
    for &px in pixels {
        let mut best_c = 0;
        let mut best_d = dist_sq(px, centroids[0]);
        for (ci, &centroid) in centroids.iter().enumerate().skip(1) {
            let d = dist_sq(px, centroid);
            if d < best_d {
                best_d = d;
                best_c = ci;
            }
        }
        counts[best_c] += 1;
    }

    let mut order: Vec<usize> = (0..k).collect();
    order.sort_by(|&a, &b| counts[b].cmp(&counts[a]));
    order.into_iter().map(|i| centroids[i]).collect()
}

/// Extrait les `n_colors` couleurs dominantes d'une image par K-Means.
pub fn extract_dominant_colors(
    image_path: impl AsRef<Path>,
    n_colors: usize,
    max_dim: u32,
) -> Result<Vec<String>, String> {
    let img = image::open(image_path.as_ref())
        .map_err(|e| format!("Impossible de lire l'image : {e}"))?;

    let thumb = img.thumbnail(max_dim, max_dim).to_rgb8();
    let raw = thumb.as_raw();
    if raw.is_empty() {
        return Err("Image vide ou illisible.".to_string());
    }

    let mut pixels = Vec::with_capacity(raw.len() / 3);
    for chunk in raw.chunks_exact(3) {
        pixels.push([chunk[0] as f32, chunk[1] as f32, chunk[2] as f32]);
    }

    let centroids = kmeans(&pixels, n_colors, 12, 42);
    Ok(centroids
        .into_iter()
        .map(|c| {
            rgb256_to_hex(
                c[0].round().clamp(0.0, 255.0) as u8,
                c[1].round().clamp(0.0, 255.0) as u8,
                c[2].round().clamp(0.0, 255.0) as u8,
            )
        })
        .collect())
}
