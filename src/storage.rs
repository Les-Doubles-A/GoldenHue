//! Persistance locale JSON dans `~/.goldenhue/` (100% compatible avec les fichiers
//! existants `history.json`, `favorites.json` et `session.json`).

use crate::color_utils::SwatchSpec;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_DIR_NAME: &str = ".goldenhue";

fn app_dir() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join(APP_DIR_NAME);
    let _ = fs::create_dir_all(&dir);
    dir
}

pub struct HistoryStore {
    pub max_items: usize,
    pub path: PathBuf,
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self {
            max_items: 12,
            path: app_dir().join("history.json"),
        }
    }
}

impl HistoryStore {
    pub fn load(&self) -> Vec<String> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn add(&self, hex_code: &str) -> Vec<String> {
        let upper = hex_code.to_uppercase();
        let mut items: Vec<String> = self
            .load()
            .into_iter()
            .filter(|h| !h.eq_ignore_ascii_case(&upper))
            .collect();
        items.insert(0, upper);
        items.truncate(self.max_items);
        if let Ok(json) = serde_json::to_string_pretty(&items) {
            let _ = fs::write(&self.path, json);
        }
        items
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteEntry {
    pub name: String,
    pub base_hex: String,
    #[serde(default = "default_scheme")]
    pub scheme: String,
    pub colors: Vec<SwatchSpec>,
    #[serde(default)]
    pub saved_at: f64,
}

fn default_scheme() -> String {
    "golden".to_string()
}

pub struct FavoritesStore {
    pub path: PathBuf,
}

impl Default for FavoritesStore {
    fn default() -> Self {
        Self {
            path: app_dir().join("favorites.json"),
        }
    }
}

impl FavoritesStore {
    pub fn load(&self) -> Vec<FavoriteEntry> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_palette(
        &self,
        name: &str,
        base_hex: &str,
        scheme: &str,
        colors: Vec<SwatchSpec>,
    ) -> Vec<FavoriteEntry> {
        let mut favorites = self.load();
        let saved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        favorites.insert(
            0,
            FavoriteEntry {
                name: name.to_string(),
                base_hex: base_hex.to_uppercase(),
                scheme: scheme.to_string(),
                colors,
                saved_at,
            },
        );
        if let Ok(json) = serde_json::to_string_pretty(&favorites) {
            let _ = fs::write(&self.path, json);
        }
        favorites
    }

    pub fn delete(&self, index: usize) -> Vec<FavoriteEntry> {
        let mut favorites = self.load();
        if index < favorites.len() {
            favorites.remove(index);
            if let Ok(json) = serde_json::to_string_pretty(&favorites) {
                let _ = fs::write(&self.path, json);
            }
        }
        favorites
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    #[serde(default = "default_base_hex")]
    pub base_hex: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_scheme")]
    pub last_scheme: String,
}

fn default_base_hex() -> String {
    "#F1C40F".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            base_hex: default_base_hex(),
            theme: default_theme(),
            last_scheme: default_scheme(),
        }
    }
}

pub struct SessionStore {
    pub path: PathBuf,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self {
            path: app_dir().join("session.json"),
        }
    }
}

impl SessionStore {
    pub fn load(&self) -> SessionData {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, data: &SessionData) {
        if let Ok(json) = serde_json::to_string_pretty(data) {
            let _ = fs::write(&self.path, json);
        }
    }
}
