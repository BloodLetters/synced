<div align="center">

<img src="assets/icon.png" width="100" height="100" alt="synceD Icon">

# synceD

High-performance, segmented download manager built with Rust.

[Features](#features) • [Architecture](#architecture) • [Installation](#installation) • [Build](#build-from-source) • [Browser Extensions](#browser-extensions)

</div>

## Features

- **Segmented Engine**: Multi-connection chunk downloading with direct offset byte writing.
- **Pause & Resume**: Safe pause and resume without data corruption.
- **Desktop GUI**: Hardware-accelerated interface with real-time throughput metrics and segment visualizer.
- **CLI Client**: Lightweight terminal interface with progress reporting.
- **Browser Interception**: Automatically catches downloads from Firefox and Google Chrome.
- **Default Downloads Path**: Auto-resolves the system Downloads folder via Windows Known Folders.

## Architecture

| Component | Path | Description |
| :--- | :--- | :--- |
| **synced-core** | `crates/synced-core` | Core engine: HTTP probing, chunking, storage pre-allocation |
| **synced-gui** | `crates/synced-gui` | Desktop application frontend (egui / eframe) |
| **synced-cli** | `crates/synced-cli` | Command-line client (clap / indicatif) |
| **Firefox Add-on** | `extension/firefox` | Browser extension for Mozilla Firefox |
| **Chrome Extension** | `extension/chrome` | Browser extension for Google Chrome & Chromium |

## Installation

### Windows Installer (.msi)
Download the latest `synceD-Setup.msi` from GitHub Releases or compile it locally. The installer registers Desktop and Start Menu shortcuts, and integrates natively with Windows Installed Apps for complete uninstallation.

### Standalone Executables
Standalone `synced-gui.exe` and `synced-cli.exe` binaries require no runtime dependencies and can be executed directly from `target/release/`.

## Build from Source

### Prerequisites
- Rust (via `rustup`)
- WiX Toolset v4 (for `.msi` creation: `dotnet tool install -g wix --version 4.0.5`)

### Compilation

```bash
# Build desktop GUI
cargo build --release -p synced-gui

# Build CLI
cargo build --release -p synced-cli

# Build entire workspace
cargo build --release
```

### Windows Installer (.msi)

```bash
wix build -arch x64 installer/synced.wxs -o target/release/synceD-Setup.msi
```

## Browser Extensions

Browser extensions intercept web downloads and send them directly to synceD over a local IPC endpoint (`127.0.0.1:17890`).

- **Firefox**: Load via `about:debugging` -> [`extension/firefox/`](extension/firefox/manifest.json)
- **Google Chrome**: Load via `chrome://extensions/` -> [`extension/chrome/`](extension/chrome/manifest.json)

## License
Licensed under the MIT License.
