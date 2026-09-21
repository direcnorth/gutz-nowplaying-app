# Gutz Now Playing – Desktop-Wrapper

Native Desktop-App (Tauri 3 alpha, Rust + React) für die Web-UI von [gutz-nowplaying](https://github.com/direcnorth/gutz-nowplaying). Zeigt Startseite, Bühne, Anfragen, Wunsch-Overlay und Admin in eigenen Fenstern, mit Tray-Menü, konfigurierbarer Server-URL, Autostart und automatischem Neuladen nach Serverausfall.

## Voraussetzungen

| Plattform | Benötigt |
|---|---|
| Linux | `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev libxdo-dev pkg-config`, Rust (rustup), Node 20+, pnpm |
| Windows | Visual Studio Build Tools (C++), WebView2, Rust, Node, pnpm |
| macOS | Xcode Command Line Tools, Rust, Node, pnpm |

## Entwicklung

```bash
pnpm install
pnpm tauri dev
```

Beim ersten Start öffnet sich das Einstellungsfenster. Server-URL eintragen (Standard `http://gutz-bmax:8080`), „Verbindung testen“, „Speichern“. Danach erscheint das Hauptfenster; weitere Ansichten über das Tray-Icon.

## Build

```bash
pnpm tauri build
```

Bundles liegen in `src-tauri/target/release/bundle/`. Der GitHub-Workflow baut für macOS (arm64 + x86_64), Ubuntu und Windows.
