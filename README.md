# PICO-8 Player

Offline PICO-8 cart player built with Tauri 2.
Rust serves carts straight from a folder, and the same page and web runtime run in a native webview.
Targets: Windows, macOS, Linux (installers and Flatpak) and Android.

## Runtime

Lexaloffle's web runtime (`pico8_0207.js`) is not part of this repo or the builds. On first
launch the app asks for it: save it from https://www.lexaloffle.com/play/ (or take it from your
own PICO-8 web export), select it once, and it is copied to `<app data dir>/runtime/runtime.js`.
"Change runtime…" replaces it. Written against 0.2.7.

## Carts

Carts are `.p8.png` files in `<app data dir>/carts`; use "Open carts folder" or "Add carts".

- Linux: `~/.local/share/dev.local.pico8player/carts`
- macOS: `~/Library/Application Support/dev.local.pico8player/carts`
- Windows: `%APPDATA%\dev.local.pico8player\carts`

Multi-cart games need every cart in the folder, named as on the BBS. A missing one shows up
as a 404 with its lid in the on-screen log.

Saves (cartdata, options) live in the webview's storage, not in the carts folder.
The player flushes them to storage every 5 s, when the window is hidden, and when you leave a game.
`update_carts.py` from the old setup still works if you point it at the new folder.

## Build

Needs Node 20+, Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

    npm install
    npm run tauri icon app-icon.png    # once
    npm run tauri dev
    npm run tauri build

Tauri can't cross-compile, so the workflows in `.github/workflows` do the multi-OS builds
(run them from the Actions tab or push a `v*` tag):

- `build.yml`: Windows, macOS (arm64 + x64) and Linux installers. Unsigned, so expect SmartScreen/Gatekeeper warnings.
- `flatpak.yml`: `.flatpak` bundle on the GNOME runtime. Install with `flatpak install --user pico8-player.flatpak`.
- `android.yml`: debug-signed APK (arm64 + armv7). Releases need your own keystore. Use "Add carts" on a phone.

`dragDropEnabled` is off in `tauri.conf.json` so HTML5 drag and drop still works in the page.
