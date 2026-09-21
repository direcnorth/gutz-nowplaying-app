# Gutz Now Playing (Desktop-Wrapper)

Native Hülle um die Web-UI von **gutz-nowplaying** (Python-Server `receiver.py`, Port 8080, Repo github.com/direcnorth/gutz-nowplaying). Die komplette UI kommt vom Server, dieser Wrapper zeigt sie in Fenstern an und ergänzt Tray, Konfiguration und Verbindungsüberwachung.

Basis: **Tauri 3 (3.0.0-alpha)** + React 19 / TypeScript / Vite. Ziel: Windows (Bar-PC), Linux, macOS; Android/iOS vorbereitet.

## Struktur

- `src-tauri/src/lib.rs` – gesamte App-Logik: Ansichten (`View`), Fenster öffnen, Tray-Menü, Health-Monitor, Commands, Einstieg `run()`.
- `src-tauri/src/config.rs` – `Config` (Server-URL, Bühne-Vollbild, Autostart), gespeichert als `config.json` im App-Config-Verzeichnis (Linux `~/.config/de.gutz.nowplaying/`, Windows `%APPDATA%\de.gutz.nowplaying\`, macOS `~/Library/Application Support/de.gutz.nowplaying/`).
- `src/App.tsx` – einziges lokales Frontend: Einstellungsfenster (Label `settings`).
- `src-tauri/capabilities/default.json` – IPC nur für das `settings`-Fenster. Remote-Fenster haben keinen Tauri-Zugriff.
- `src-tauri/tauri.conf.json` – keine statischen Fenster (`app.windows: []`), alle Fenster entstehen zur Laufzeit.

## Ansichten und Fenster-Labels

| Label | Server-Pfad | Besonderheit |
|---|---|---|
| `main` | `/` | **Standardfenster fuer alle ausser BARPC** |
| `stage` | `/nowplaying` | Vollbild ohne Rahmen, wenn `stageFullscreen` (Desktop) |
| `requests` | `/requests` | Wird beim Start automatisch geoeffnet, wenn `startView` = `"requests"` |
| `overlay` | `/wish-overlay` | rahmenlos, immer im Vordergrund, nicht in Taskleiste (Desktop) |
| `admin` | `/admin` | Login serverseitig (Cookie) |
| `settings` | lokal (`index.html`) | nur über Tray „Einstellungen…“, nie automatisch |

`DEFAULT_SERVER_URL` (`config.rs`) ist `https://np.gutz.info` - der Wrapper startet ohne jede Einrichtung direkt in der normalen Uebersicht (`View::Main`), auch beim allerersten Start ohne `config.json`. Kein erzwungener Einstellungs-Dialog mehr (fruehere Version oeffnete bei fehlender Config nur die Einstellungen - das ist bewusst weg, siehe Git-History).

`Config.start_view` (`"main"` Standard, `"requests"` fuer BARPC) steuert, welche Ansicht beim Start automatisch aufgeht - einstellbar im Einstellungsfenster ("Beim Start automatisch oeffnen") oder direkt in `config.json`.

**BARPC-Sonderfall:** Zwei Einstellungen muessen dort einmalig ueber Tray → Einstellungen umgestellt werden (Anleitung fuer beide siehe `client/setup-requests-view.ps1`-Ersatz):
1. `startView` auf "Anfragen (nur BARPC)" - ersetzt den bisherigen Edge-App-Autostart fuer `/requests`.
2. Server-URL auf die interne LAN-Adresse (aktuell `http://192.168.0.10:8080`) statt der oeffentlichen `np.gutz.info`. Grund: Der Server vertraut BARPC ueber `SENDER_IPS` (Login-freier Zugriff auf `/requests`, Sender-Heartbeat) anhand der **rohen TCP-Peer-IP**, bewusst nicht `X-Forwarded-For` (Sicherheits-Design in `receiver.py`). Ueber die oeffentliche Adresse (durch NPM+Cloudflare) saehe der Server nur noch die IP des Proxys, `/requests` wuerde dann ploetzlich einen Login verlangen.

Der Health-Monitor pollt alle 10 s `GET /healthz`; bei Ausfall bekommen die Fenster den Titelzusatz „Server nicht erreichbar“, bei Rückkehr werden alle Remote-Fenster neu geladen. Das Event `server-status` geht an das Einstellungsfenster.

## Befehle

```bash
pnpm install
pnpm tauri dev                          # Wrapper mit Hot-Reload (Einstellungsfenster)
pnpm tauri build --debug --no-bundle    # Debug-Binary: src-tauri/target/debug/gutz-nowplaying-app
pnpm tauri build                        # Release-Bundles (.deb/.AppImage, .msi/.exe, .dmg)
cd src-tauri && cargo check && cargo clippy
pnpm build                              # tsc + vite (nur Einstellungs-Frontend)
```

Ohne Display (bmax), verifiziertes Rezept inkl. Screenshot:

```bash
Xvfb :99 -screen 0 1400x900x24 &
export DISPLAY=:99 WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1
dbus-run-session -- src-tauri/target/debug/gutz-nowplaying-app &   # DBus ist Pflicht (Tray, single-instance)
sleep 20 && xwd -root -silent | convert xwd:- png:/tmp/shot.png    # Pakete: x11-apps x11-utils imagemagick
```

Ohne Tray-Host (StatusNotifierWatcher) läuft die App ohne Tray-Menü weiter und loggt eine Warnung. Testkonfig: `~/.config/de.gutz.nowplaying/config.json` mit `{"serverUrl": "http://127.0.0.1:8080"}`.

## Tauri 3 Besonderheiten (wichtig)

- Runtime ist ein eigenes Crate: `tauri-runtime-wry` + `.runtime(tauri_runtime_wry::Wry::default())` in `lib.rs`. Ohne diesen Aufruf startet die App ohne Fenster. `tauri` hat **kein** `wry`-Feature.
- Versionen gepinnt: `tauri 3.0.0-alpha.1`, `tauri-runtime-wry 3.0.0-alpha.1`, `tauri-build 3.0.0-alpha.0`, Plugins `3.0.0-alpha.0`; npm `@tauri-apps/*` auf `3.0.0-alpha.*` (Tag `next`). Immer gemeinsam heben, danach alle Plattformen bauen.
- Desktop-only-Plugins (`single-instance`, `autostart`) und Tray sind mit `#[cfg(desktop)]` gekapselt, damit Mobile-Builds kompilieren.
- Identifier `de.gutz.nowplaying` ist nach Release unveränderlich und darf nicht auf `.app` enden.
- macOS/Xcode 27: in `gen/apple/project.yml` und `.pbxproj` `ENABLE_USER_SCRIPT_SANDBOXING = NO` setzen, Simulator-UI heißt DeviceHub. Siehe Referenzprojekt `gutz-app` auf dem Mac.

## Auto-Update

Tauri-Updater-Plugin (`tauri-plugin-updater`, Desktop-only). Check laeuft von
selbst im Hintergrund (30s nach Start, danach alle 6h, siehe `lib.rs`
`spawn_update_checker`/`check_for_update`) sowie manuell ueber "Nach Updates
suchen" im Tray-Menu. Bei gefundenem Update wird sofort heruntergeladen und
installiert; unter macOS/Linux zeigt die App zuerst eine native
Benachrichtigung ("Update auf Version X installiert - die App startet jetzt
neu"), wartet 3s und startet sich dann selbst neu (`app.restart()`) - ohne
den Hinweis wuerde die App fuer wen auch immer gerade davorsitzt einfach
kommentarlos verschwinden und neu aufgehen. Unter Windows beendet sich der
Prozess laut tauri-plugin-updater bereits waehrend `install()`, dieser Code
wird dort also nie erreicht (und BARPC soll ja bewusst "quiet"/unbeobachtet
bleiben, siehe unten).

- **Windows unbeaufsichtigt (BARPC-relevant):** `bundle.targets` enthaelt
  bewusst kein `"msi"` mehr - nur die NSIS-`.exe`, mit
  `bundle.windows.nsis.installMode: "currentUser"` (installiert nach
  `%LOCALAPPDATA%`, keine Admin-Rechte noetig) und
  `plugins.updater.windows.installMode: "quiet"` (kein Fortschrittsfenster,
  keine Rueckfrage). Laut Tauri-Doku braucht "quiet" zwingend eine
  Per-User-Installation, sonst kann der Installer keine Admin-Rechte anfragen
  und das Update haengt. MSI ist per-machine und haette bei jedem Update
  einen UAC-Dialog ausgeloest - auf einem unbeaufsichtigten Bar-PC waere das
  Update dann einfach steckengeblieben. **Noch nicht auf echter
  Windows-Hardware verifiziert** (kein Windows-Rechner beim Entwickeln
  verfuegbar) - vor dem produktiven Einsatz auf BARPC einmal einen
  Update-Zyklus real durchspielen.
- Endpoint + Public Key: `plugins.updater` in `tauri.conf.json`, zeigt auf
  `.../releases/latest/download/latest.json` (wird von `tauri-action` beim
  Release automatisch erzeugt, siehe `bundle.createUpdaterArtifacts`).
- Signiert wird ausschliesslich in CI: Private Key liegt lokal unter
  `~/.tauri/gutz-nowplaying-app.key` (nicht im Repo) und als GitHub-Actions-
  Secret `TAURI_SIGNING_PRIVATE_KEY` (+ `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`,
  aktuell leer) auf `direcnorth/gutz-nowplaying-app`. Ohne dieses Secret baut
  `release.yml` zwar noch Installer, aber keine gueltig signierten
  Updater-Artefakte - Updates wuerden dann von jedem Client abgelehnt.
- Key verloren = Updates fuer alle bisherigen Installationen kaputt (neuer
  Public Key passt nicht zu den bereits ausgelieferten Binaries). Kein
  Passwort auf dem Key, bewusst - vereinfacht den nicht-interaktiven CI-Lauf.

## App-Icon

Generiert aus `gutz-nowplaying/favicon-512.png` (512×512, transparent) via
`pnpm tauri icon <quelle>` - erzeugt den kompletten Satz unter
`src-tauri/icons/` (Windows/macOS/Linux/iOS/Android) neu. Bei einem neuen
Quellbild denselben Befehl erneut laufen lassen, nicht einzelne Icon-Dateien
von Hand ersetzen (Groessen/Formate muessen zueinander passen).

## Konventionen

- Plattformspezifisches Rust nur hinter `#[cfg(target_os = "...")]` / `#[cfg(desktop)]`.
- Keine IPC-Freigabe für Remote-Domains ohne dokumentierten Grund (`app.security.dangerousRemoteDomainIpcAccess`).
- Prettier (TS) und `cargo fmt` (Rust). Commits englisch, Imperativ.
- Bar-PC-Hilfsskripte (Sender, Overlay-Watcher) bleiben in `gutz-nowplaying/client/`; dieser Wrapper ersetzt sie nicht.
