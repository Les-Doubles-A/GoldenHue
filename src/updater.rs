//! Système de vérification et de mise à jour automatique depuis le dépôt GitHub
//! `https://github.com/Les-Doubles-A/GoldenHue`.
//!
//! Fonctionnement :
//! 1. Au lancement (ou manuellement via la barre d'outils), un thread d'arrière-plan interroge :
//!    - L'API GitHub Releases (`https://api.github.com/repos/Les-Doubles-A/GoldenHue/releases/latest`)
//!    - En repli (si aucune Release GitHub n'est publiée), le `Cargo.toml` de la branche `main`
//!      (`https://raw.githubusercontent.com/Les-Doubles-A/GoldenHue/main/Cargo.toml`).
//! 2. Si la version distante est strictement supérieure à `CARGO_PKG_VERSION`, l'interface affiche
//!    une bannière proposant de télécharger et d'installer automatiquement la mise à jour
//!    (remplacement atomique de `GoldenHue.exe` et relance de l'application) ou d'ouvrir GitHub.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO_URL: &str = "https://github.com/Les-Doubles-A/GoldenHue";
const GITHUB_API_LATEST_RELEASE: &str =
    "https://api.github.com/repos/Les-Doubles-A/GoldenHue/releases/latest";
const GITHUB_RAW_CARGO_TOML: &str =
    "https://raw.githubusercontent.com/Les-Doubles-A/GoldenHue/main/Cargo.toml";
const GITHUB_RAW_EXE: &str =
    "https://github.com/Les-Doubles-A/GoldenHue/raw/main/GoldenHue.exe";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
    pub html_url: String,
    pub release_notes: String,
}

#[derive(Debug, Clone)]
pub enum UpdateCheckOutcome {
    UpToDate(String),
    UpdateAvailable(UpdateInfo),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum SelfUpdateOutcome {
    ReadyToRestart(PathBuf),
    Error(String),
}

/// Nettoie l'éventuel fichier `GoldenHue.exe.old` laissé par une mise à jour précédente.
pub fn cleanup_old_executable() {
    if let Ok(exe_path) = env::current_exe() {
        let old_path = exe_path.with_extension("exe.old");
        if old_path.exists() {
            let _ = fs::remove_file(old_path);
        }
    }
}

/// Analyse une version `"v2.1.0"` ou `"2.1.0"` en triplet `(major, minor, patch)`.
pub fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let clean = version
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()?;
    let mut parts = clean.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((major, minor, patch))
}

/// Retourne `true` si `remote` est strictement plus récente que `current`.
pub fn is_newer_version(current: &str, remote: &str) -> bool {
    match (parse_semver(current), parse_semver(remote)) {
        (Some(cur), Some(rem)) => rem > cur,
        _ => false,
    }
}

/// Extrait le champ `version = "X.Y.Z"` d'un contenu `Cargo.toml`.
pub fn extract_version_from_cargo_toml(content: &str) -> Option<String> {
    let mut in_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package && let Some(rest) = trimmed.strip_prefix("version") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let val = rest.trim().trim_matches('"').trim_matches('\'');
                if parse_semver(val).is_some() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn http_get_text(url: &str) -> Result<String, String> {
    let mut cmd = Command::new("curl.exe");
    cmd.args([
        "-sSL",
        "--max-time",
        "10",
        "-H",
        "User-Agent: GoldenHue-Rust-Updater",
        "-H",
        "Accept: application/vnd.github+json",
        url,
    ]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| format!("Impossible d'exécuter la requête réseau : {e}"))?;

    if !output.status.success() {
        return Err(format!("Code HTTP/réseau : {}", output.status));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Réponse UTF-8 invalide : {e}"))
}

fn check_github_for_update() -> UpdateCheckOutcome {
    // 1. Vérifier d'abord l'API GitHub Releases
    if let Ok(json_text) = http_get_text(GITHUB_API_LATEST_RELEASE)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_text)
        && let Some(tag) = val.get("tag_name").and_then(|v| v.as_str())
    {
        let clean_ver = tag.trim_start_matches(['v', 'V']).to_string();
        let html_url = val
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or(GITHUB_REPO_URL)
            .to_string();
        let notes = val
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        // Chercher un asset .exe dans la release, sinon repli sur GoldenHue.exe de main
        let mut download_url = GITHUB_RAW_EXE.to_string();
        if let Some(assets) = val.get("assets").and_then(|v| v.as_array()) {
            for asset in assets {
                if let Some(name) = asset.get("name").and_then(|v| v.as_str())
                    && name.to_ascii_lowercase().ends_with(".exe")
                    && let Some(dl) = asset.get("browser_download_url").and_then(|v| v.as_str())
                {
                    download_url = dl.to_string();
                    break;
                }
            }
        }

        if is_newer_version(CURRENT_VERSION, &clean_ver) {
            return UpdateCheckOutcome::UpdateAvailable(UpdateInfo {
                latest_version: clean_ver,
                download_url,
                html_url,
                release_notes: notes,
            });
        } else {
            return UpdateCheckOutcome::UpToDate(CURRENT_VERSION.to_string());
        }
    }

    // 2. Repli : vérifier la version dans le Cargo.toml de la branche main sur GitHub
    match http_get_text(GITHUB_RAW_CARGO_TOML) {
        Ok(cargo_toml) => {
            if let Some(remote_ver) = extract_version_from_cargo_toml(&cargo_toml) {
                if is_newer_version(CURRENT_VERSION, &remote_ver) {
                    UpdateCheckOutcome::UpdateAvailable(UpdateInfo {
                        latest_version: remote_ver,
                        download_url: GITHUB_RAW_EXE.to_string(),
                        html_url: GITHUB_REPO_URL.to_string(),
                        release_notes: "Nouvelle version publiée sur la branche principale GitHub."
                            .to_string(),
                    })
                } else {
                    UpdateCheckOutcome::UpToDate(CURRENT_VERSION.to_string())
                }
            } else {
                UpdateCheckOutcome::Error(
                    "Impossible de lire le numéro de version distant.".to_string(),
                )
            }
        }
        Err(err) => UpdateCheckOutcome::Error(format!(
            "Impossible de contacter GitHub pour vérifier les mises à jour ({err})."
        )),
    }
}

/// Lance une vérification de mise à jour asynchrone dans un thread séparé.
pub fn spawn_update_check(ctx: egui::Context) -> Receiver<UpdateCheckOutcome> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = check_github_for_update();
        let _ = tx.send(outcome);
        ctx.request_repaint();
    });
    rx
}

/// Télécharge le nouvel exécutable, remplace `GoldenHue.exe` de manière atomique et prépare le redémarrage.
pub fn spawn_self_update(download_url: String, ctx: egui::Context) -> Receiver<SelfUpdateOutcome> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = perform_self_update(&download_url);
        let _ = tx.send(outcome);
        ctx.request_repaint();
    });
    rx
}

fn perform_self_update(download_url: &str) -> SelfUpdateOutcome {
    let current_exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => return SelfUpdateOutcome::Error(format!("Chemin de l'exécutable introuvable : {e}")),
    };

    let new_exe = current_exe.with_extension("exe.new");
    let old_exe = current_exe.with_extension("exe.old");

    let new_exe_str = match new_exe.to_str() {
        Some(s) => s,
        None => return SelfUpdateOutcome::Error("Chemin temporaire invalide.".to_string()),
    };

    let mut cmd = Command::new("curl.exe");
    cmd.args([
        "-sSL",
        "--fail",
        "--max-time",
        "120",
        "-H",
        "User-Agent: GoldenHue-Rust-Updater",
        "-o",
        new_exe_str,
        download_url,
    ]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    match cmd.status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            let _ = fs::remove_file(&new_exe);
            return SelfUpdateOutcome::Error(format!(
                "Échec du téléchargement de la mise à jour (code {status})."
            ));
        }
        Err(e) => {
            return SelfUpdateOutcome::Error(format!("Erreur lors du téléchargement : {e}"));
        }
    }

    // Vérifier l'en-tête PE Windows ("MZ") pour s'assurer que le binaire téléchargé est valide
    match fs::read(&new_exe) {
        Ok(bytes) if bytes.len() > 1024 && &bytes[0..2] == b"MZ" => {}
        _ => {
            let _ = fs::remove_file(&new_exe);
            return SelfUpdateOutcome::Error(
                "Le fichier téléchargé n'est pas un exécutable Windows valide.".to_string(),
            );
        }
    }

    let _ = fs::remove_file(&old_exe);
    if let Err(e) = fs::rename(&current_exe, &old_exe) {
        let _ = fs::remove_file(&new_exe);
        return SelfUpdateOutcome::Error(format!(
            "Impossible de renommer l'exécutable en cours : {e}"
        ));
    }

    if let Err(e) = fs::rename(&new_exe, &current_exe) {
        // Restaurer l'ancien exécutable en cas d'erreur
        let _ = fs::rename(&old_exe, &current_exe);
        return SelfUpdateOutcome::Error(format!(
            "Impossible d'installer le nouvel exécutable : {e}"
        ));
    }

    SelfUpdateOutcome::ReadyToRestart(current_exe)
}

/// Relance la nouvelle version de l'application et ferme l'instance actuelle.
pub fn restart_application(exe_path: &PathBuf) {
    if Command::new(exe_path).spawn().is_ok() {
        std::process::exit(0);
    }
}

/// Ouvre une URL dans le navigateur par défaut.
pub fn open_url_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", url]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver() {
        assert_eq!(parse_semver("2.0.0"), Some((2, 0, 0)));
        assert_eq!(parse_semver("v2.1.3"), Some((2, 1, 3)));
        assert_eq!(parse_semver("V3.0.0-beta.1"), Some((3, 0, 0)));
        assert_eq!(parse_semver("invalid"), None);
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("2.0.0", "2.0.1"));
        assert!(is_newer_version("2.0.0", "v2.1.0"));
        assert!(is_newer_version("2.0.0", "3.0.0"));
        assert!(!is_newer_version("2.0.0", "2.0.0"));
        assert!(!is_newer_version("2.1.0", "2.0.9"));
    }

    #[test]
    fn test_extract_version_from_cargo_toml() {
        let sample = r#"
[package]
name = "goldenhue"
version = "2.3.1"
edition = "2024"
"#;
        assert_eq!(
            extract_version_from_cargo_toml(sample),
            Some("2.3.1".to_string())
        );
    }
}
