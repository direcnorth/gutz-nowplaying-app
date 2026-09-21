//! Persistente Einstellungen des Wrappers (JSON im App-Config-Verzeichnis).

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use url::Url;

pub const DEFAULT_SERVER_URL: &str = "http://gutz-bmax:8080";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// Basis-URL des nowplaying-Servers, z. B. http://gutz-bmax:8080
    pub server_url: String,
    /// Bühne (/nowplaying) als Vollbild-Kiosk öffnen
    pub stage_fullscreen: bool,
    /// Wrapper beim Login automatisch starten
    pub autostart: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            stage_fullscreen: true,
            autostart: false,
        }
    }
}

impl Config {
    /// Baut aus der Server-URL und einem Pfad wie "/nowplaying" eine vollständige URL.
    pub fn url_for(&self, path: &str) -> Result<Url, String> {
        let mut base = self.server_url.trim().to_string();
        if !base.ends_with('/') {
            base.push('/');
        }
        let base = Url::parse(&base).map_err(|e| format!("Ungültige Server-URL: {e}"))?;
        if !matches!(base.scheme(), "http" | "https") {
            return Err("Server-URL muss mit http:// oder https:// beginnen".into());
        }
        base.join(path.trim_start_matches('/'))
            .map_err(|e| format!("Ungültiger Pfad {path}: {e}"))
    }

    pub fn validate(&self) -> Result<(), String> {
        self.url_for("/").map(|_| ())
    }
}

fn config_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|d| d.join("config.json"))
        .map_err(|e| format!("Config-Verzeichnis nicht ermittelbar: {e}"))
}

/// Liest die gespeicherte Konfiguration. `None`, wenn noch keine existiert.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Option<Config> {
    let path = config_path(app).ok()?;
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save<R: Runtime>(app: &AppHandle<R>, config: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("Config-Verzeichnis: {e}"))?;
    }
    let raw = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("Config schreiben ({}): {e}", path.display()))
}
