//! Gutz Now Playing – Desktop-Wrapper um die Web-UI von gutz-nowplaying.
//!
//! Die UI kommt komplett vom Server (Remote-URL-Fenster). Dieser Wrapper liefert:
//! Fenster pro Ansicht, Tray-Menü, Server-URL-Konfiguration, Health-Überwachung
//! mit Auto-Reload und ein kleines lokales Einstellungsfenster.

mod config;

use config::Config;
use serde::{Deserialize, Serialize};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

const HEALTH_INTERVAL: Duration = Duration::from_secs(10);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(3);
const SETTINGS_LABEL: &str = "settings";

/// Die Ansichten der Web-UI, die als eigene Fenster geöffnet werden können.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Main,
    Stage,
    Requests,
    Overlay,
    Admin,
}

impl View {
    pub const ALL: [View; 5] = [
        View::Main,
        View::Stage,
        View::Requests,
        View::Overlay,
        View::Admin,
    ];

    fn label(self) -> &'static str {
        match self {
            View::Main => "main",
            View::Stage => "stage",
            View::Requests => "requests",
            View::Overlay => "overlay",
            View::Admin => "admin",
        }
    }

    fn path(self) -> &'static str {
        match self {
            View::Main => "/",
            View::Stage => "/nowplaying",
            View::Requests => "/requests",
            View::Overlay => "/wish-overlay",
            View::Admin => "/admin",
        }
    }

    fn title(self) -> &'static str {
        match self {
            View::Main => "Gutz Now Playing",
            View::Stage => "Gutz – Bühne",
            View::Requests => "Gutz – Anfragen",
            View::Overlay => "Gutz – Wunsch-Overlay",
            View::Admin => "Gutz – Admin",
        }
    }

    fn from_label(label: &str) -> Option<View> {
        View::ALL.into_iter().find(|v| v.label() == label)
    }
}

pub struct AppState {
    config: Mutex<Config>,
    /// `None` = noch nicht geprüft, sonst letzter Health-Status.
    server_ok: Mutex<Option<bool>>,
}

impl AppState {
    fn config(&self) -> Config {
        self.config.lock().expect("config mutex").clone()
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerStatus {
    ok: bool,
    server_url: String,
}

// ---------------------------------------------------------------------------
// Fenster
// ---------------------------------------------------------------------------

fn open_view<R: Runtime>(app: &AppHandle<R>, view: View) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(view.label()) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let cfg = app.state::<AppState>().config();
    let url = cfg.url_for(view.path())?;

    let builder = WebviewWindowBuilder::new(app, view.label(), WebviewUrl::External(url))
        .title(view.title());

    let builder = match view {
        View::Stage => builder
            .inner_size(1280.0, 720.0)
            .fullscreen(cfg.stage_fullscreen)
            .decorations(!cfg.stage_fullscreen),
        View::Overlay => builder
            .inner_size(480.0, 270.0)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true),
        View::Main | View::Requests | View::Admin => builder.inner_size(1100.0, 800.0),
    };

    builder.build().map_err(|e| format!("Fenster {}: {e}", view.label()))?;
    Ok(())
}

fn open_settings<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
        .title("Gutz Now Playing – Einstellungen")
        .inner_size(560.0, 620.0)
        .resizable(true)
        .build()
        .map_err(|e| format!("Einstellungsfenster: {e}"))?;
    Ok(())
}

/// Lädt alle offenen Remote-Fenster neu (nach Reconnect oder URL-Wechsel).
fn reload_remote_windows<R: Runtime>(app: &AppHandle<R>) {
    let cfg = app.state::<AppState>().config();
    for view in View::ALL {
        if let Some(window) = app.get_webview_window(view.label()) {
            match cfg.url_for(view.path()) {
                // Navigation auf die (ggf. neue) URL statt reinem reload, damit ein
                // URL-Wechsel in den Einstellungen sofort greift.
                Ok(url) => {
                    let _ = window.navigate(url);
                }
                Err(_) => {
                    let _ = window.eval("location.reload()");
                }
            }
        }
    }
}

fn set_offline_titles<R: Runtime>(app: &AppHandle<R>, offline: bool) {
    for view in View::ALL {
        if let Some(window) = app.get_webview_window(view.label()) {
            let title = if offline {
                format!("{} – Server nicht erreichbar", view.title())
            } else {
                view.title().to_string()
            };
            let _ = window.set_title(&title);
        }
    }
}

// ---------------------------------------------------------------------------
// Health-Überwachung
// ---------------------------------------------------------------------------

fn probe_server(cfg: &Config) -> bool {
    let Ok(url) = cfg.url_for("/healthz") else {
        return false;
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(HEALTH_TIMEOUT))
        .build()
        .into();
    match agent.get(url.as_str()).call() {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

fn spawn_health_monitor<R: Runtime>(app: AppHandle<R>) {
    std::thread::Builder::new()
        .name("health-monitor".into())
        .spawn(move || loop {
            let started = Instant::now();
            let state = app.state::<AppState>();
            let cfg = state.config();
            let ok = probe_server(&cfg);
            let previous = state
                .server_ok
                .lock()
                .expect("server_ok mutex")
                .replace(ok);

            if previous != Some(ok) {
                set_offline_titles(&app, !ok);
                let _ = app.emit(
                    "server-status",
                    ServerStatus {
                        ok,
                        server_url: cfg.server_url.clone(),
                    },
                );
                if ok && previous == Some(false) {
                    reload_remote_windows(&app);
                }
            }

            let elapsed = started.elapsed();
            if elapsed < HEALTH_INTERVAL {
                std::thread::sleep(HEALTH_INTERVAL - elapsed);
            }
        })
        .expect("health monitor thread");
}

// ---------------------------------------------------------------------------
// Tray
// ---------------------------------------------------------------------------

#[cfg(desktop)]
fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let mut items: Vec<MenuItem<R>> = Vec::new();
    for view in View::ALL {
        items.push(MenuItem::with_id(
            app,
            format!("view:{}", view.label()),
            view.title(),
            true,
            None::<&str>,
        )?);
    }
    let settings = MenuItem::with_id(app, "settings", "Einstellungen…", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Alle Fenster neu laden", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::new(app)?;
    for item in &items {
        menu.append(item)?;
    }
    menu.append(&sep1)?;
    menu.append(&settings)?;
    menu.append(&reload)?;
    menu.append(&sep2)?;
    menu.append(&quit)?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Gutz Now Playing")
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            let result = match id {
                "settings" => open_settings(app),
                "reload" => {
                    reload_remote_windows(app);
                    Ok(())
                }
                "quit" => {
                    app.exit(0);
                    Ok(())
                }
                other => match other.strip_prefix("view:").and_then(View::from_label) {
                    Some(view) => open_view(app, view),
                    None => Ok(()),
                },
            };
            if let Err(err) = result {
                eprintln!("Tray-Aktion {id} fehlgeschlagen: {err}");
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands (nur für das lokale Einstellungsfenster)
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Config {
    state.config()
}

#[tauri::command]
fn set_config<R: Runtime>(app: AppHandle<R>, config: Config) -> Result<(), String> {
    config.validate()?;
    config::save(&app, &config)?;
    *app.state::<AppState>().config.lock().expect("config mutex") = config.clone();
    // Status zurücksetzen, damit der Monitor den neuen Server frisch bewertet.
    *app.state::<AppState>().server_ok.lock().expect("server_ok mutex") = None;
    apply_autostart(&app, config.autostart)?;
    reload_remote_windows(&app);
    Ok(())
}

#[tauri::command]
fn check_server(config: Config) -> Result<bool, String> {
    config.validate()?;
    Ok(probe_server(&config))
}

#[tauri::command]
fn server_status(state: tauri::State<'_, AppState>) -> Option<bool> {
    *state.server_ok.lock().expect("server_ok mutex")
}

#[tauri::command]
fn open_view_cmd<R: Runtime>(app: AppHandle<R>, view: View) -> Result<(), String> {
    open_view(&app, view)
}

#[tauri::command]
fn default_server_url() -> String {
    config::DEFAULT_SERVER_URL.to_string()
}

#[cfg(desktop)]
fn apply_autostart<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| format!("Autostart: {e}"))
}

#[cfg(not(desktop))]
fn apply_autostart<R: Runtime>(_app: &AppHandle<R>, _enabled: bool) -> Result<(), String> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Einstieg
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .runtime(tauri_runtime_wry::Wry::default())
        .plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Zweiter Start: Hauptfenster in den Vordergrund holen.
            let _ = open_view(app, View::Main);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));

    let app = builder
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            check_server,
            server_status,
            open_view_cmd,
            default_server_url
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let stored = config::load(&handle);
            let configured = stored.is_some();
            app.manage(AppState {
                config: Mutex::new(stored.unwrap_or_default()),
                server_ok: Mutex::new(None),
            });

            // Tray ist optional: ohne Tray-Host (z. B. headless oder Desktop ohne
            // StatusNotifierWatcher) läuft die App trotzdem, nur ohne Tray-Menü.
            #[cfg(desktop)]
            if let Err(err) = build_tray(&handle) {
                eprintln!("Tray-Icon nicht verfügbar, weiter ohne Tray: {err}");
            }

            spawn_health_monitor(handle.clone());

            if configured {
                open_view(&handle, View::Main)?;
            } else {
                open_settings(&handle)?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app, event| {
        // Ohne Fenster im Tray weiterlaufen; explizites Beenden (app.exit) hat einen Code.
        #[cfg(desktop)]
        if let tauri::RunEvent::ExitRequested { api, code: None, .. } = &event {
            api.prevent_exit();
        }
        #[cfg(not(desktop))]
        let _ = event;
    });
}
