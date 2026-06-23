# StatsWidget

A lightweight, always-on-top system monitor widget that overlays real-time hardware stats (CPU, RAM, Disk, Network, GPU) on your Windows taskbar.

Sits between your taskbar apps and the system tray, stays click-through (doesn't block mouse), and auto-hides when a fullscreen game or video is active.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | [Tauri v2](https://v2.tauri.app/) |
| Backend | Rust (sysinfo + Windows PDH) |
| Frontend | TypeScript + HTML + CSS |
| Bundler | Vite |

## Prerequisites

- [Node.js](https://nodejs.org) 18+
- [Rust](https://rustup.rs) (stable)
- Windows 10 1803+ or Windows 11 (WebView2 is preinstalled)

## Getting Started

```bash
npm install          # install frontend dependencies
```

## NPM Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server only (frontend at `http://localhost:1420`) |
| `npm run tauri dev` | Start full Tauri app in dev mode (hot-reload, shows the widget window) |
| `npm run build` | Build frontend only (`tsc` + `vite build` → `dist/`) |
| `npm run tauri build` | Build full app (frontend + Rust → portable EXE) |
| `npm run build:release` | Build frontend + Tauri in one step |
| `npm run package` | Clean old build, build everything, copy EXE to `release/` |
| `npm run preview` | Preview the built frontend in a browser |

## Dev Mode

```bash
npm run tauri dev
```

This opens the widget window directly. The Vite dev server auto-reloads the frontend on file changes.

## Building (Portable EXE)

```bash
npm run package
```

Output: `release/StatsWidget.exe` — runs directly, no install required.

The build script:
1. Cleans `src-tauri/target/release/`
2. Builds frontend (`tsc && vite build`)
3. Builds Rust backend and bundles into EXE
4. Copies to `release/StatsWidget.exe`

## How It Works

- The widget finds the Windows taskbar's notification area (system tray) via Win32 APIs
- Positions itself to the left of the system tray with 8px padding
- Sets the window as click-through (mouse events pass through)
- Polls CPU/RAM/Network via `sysinfo`, Disk/GPU via Windows PDH every 1 second
- Auto-hides when a true fullscreen app (game, video) is foreground — checks for missing caption bar and resize border to distinguish from maximized windows
- Text and mini sparklines render via Canvas in the transparent Tauri webview

## License

MIT
