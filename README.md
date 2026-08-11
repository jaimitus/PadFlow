# 🎮 PadFlow — Next-Gen Gamepad Input Calibrator & ViGEmBus Bridge

[![Release](https://img.shields.io/badge/Release-v1.2.4-cyan.svg?style=for-the-badge&logo=windows)](https://github.com/jaimitus/PadFlow/releases)
[![License](https://img.shields.io/badge/License-MIT-violet.svg?style=for-the-badge)](./LICENSE)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-Donate-orange.svg?style=for-the-badge&logo=buy-me-a-coffee)](https://buymeacoffee.com/jaimitus)

**PadFlow** is a native, ultra-lightweight gamepad calibration studio and virtual Xbox 360 controller emulator for **Windows 10 / 11**. Built from the ground up in **Rust** and **Tauri v2**, PadFlow bridges PlayStation 4 (DualShock 4) and PlayStation 5 (DualSense / Edge) controllers to XInput via **ViGEmBus** — up to **4 gamepads at once**, each in its own virtual Xbox 360 slot — with sub-millisecond processing latency, zero bloat, and **integrated HidHide anti-double-input protection**.

---

## 📸 Interface Preview

![PadFlow Studio Screenshot](./PadFlow_UI.png)

---

## 🚀 What's New in v1.2.4 / Novedades de la Versión 1.2.4

- **🤖 CI manifest check:** a GitHub Actions workflow runs on every push/PR touching the Rust side and verifies the release exe embeds `requireAdministrator` (elevated manifest) while the debug exe does **not** — elevation can never silently regress.
- **🚀 Automated release pipeline:** pushing a `v*.*.*` tag now makes GitHub Actions build, **sign** and publish the full release automatically — NSIS installer, MSI, portable exe, `.sig` signatures and `latest.json` for the built-in updater. No more manual builds or uploads.
  - Version **triple-guard**: the pipeline refuses to run unless the tag matches `tauri.conf.json`, `package.json` **and** `Cargo.toml` (fail-fast with the mismatched file named).
  - Manual trigger available too (Actions → Run workflow → tag input) for smoke-testing.
- **🔐 Everything from v1.2.3 kept:** `requireAdministrator` manifest (UAC at launch), correct HidHide IOCTL contract (device type `0x8001`), honest diagnostics, Shield Control Center, auto-cloak, cloak on startup, tray controls, live 3 s status refresh and one-click signed auto-updates.

---

## 📜 Changelog

Every release is documented in **[CHANGELOG.md](./CHANGELOG.md)** — full history from v1.0.0, kept **automatically up to date** by GitHub Actions (the release notes are prepended on every published release).

---

## ✨ Key Features

- **⚡ Sub-Millisecond Realtime Engine:** Multi-threaded Rust HID engine running up to **1,000 Hz (1 kHz turbo polling)** with sub-millisecond input translation.
- **🎮 Multi-Controller Support:** Connect and emulate up to **4 gamepads simultaneously** (any mix of DualShock 4 / DualSense / DualSense Edge), each mapped to its own virtual Xbox 360 controller slot.
- **🛡️ HidHide Anti-Double-Input Shield:** Direct integration with the Nefarius **HidHide** driver to cloak physical PlayStation DirectInput devices from games so only the emulated XInput pad is detected, eliminating double-tap and ghost input glitches.
- **🎮 100% Real PlayStation HID Support:** Direct USB and Bluetooth HID parsing for DualShock 4 (`0x054C:0x05C4`, `0x09CC`) and DualSense / DualSense Edge (`0x054C:0x0CE6`, `0x0DF2`).
- **📈 Dynamic Stick Response Curve Tuner:** Live 60 FPS interactive HTML5 canvas for visual curve shaping:
  - **Linear:** Predictable 1:1 raw translation.
  - **Exponential:** Micro-precision near deadzone center, rapid flick response at outer rim.
  - **S-Curve:** Smoothstep mathematical curve for balanced precision and speed.
  - **Aggressive:** Inverse-exponential instant ramp for arcade and fast-paced competitive titles.
- **🎯 Inner, Outer & Anti-Deadzone Calibration:** Independent radial or axis-aligned deadzone configuration per stick.
- **💡 Lightbar & Rumble Haptics:** Custom RGB lightbar color assignment and rumble intensity shaping.
- **💾 Custom User Profile Manager:** Save, load, export (copy JSON to clipboard), and delete personalized curves locally.
- **🛡️ Shield Control Center:** per-controller cloak toggles, global shield switch, CLOAK ALL / UNCLOAK ALL, hidden-device list, auto-cloak on connect & on startup, plus tray controls.
- **⬆️ Built-in Auto-Update:** Automatic GitHub release detection with one-click signed in-app updates and release-note preview.
- **🚀 Automated ViGEmBus & HidHide Integration:** Detects driver presence automatically with interactive 1-click installer support and official client launching.
- **🪶 Ultra-Lightweight Footprint:** Consumes **< 15 MB RAM**, zero background bloat, no account required, 100% telemetry-free.

---

## 📊 Feature Matrix: PadFlow vs Competition

| Feature / Metric | ⚡ PadFlow | 🎮 DS4Windows | 🕹️ reWASD | 💨 Steam Input | 🔮 DualSenseX |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **RAM Usage** | **< 15 MB** | ~60 - 120 MB | ~150 - 250 MB | ~200 - 400 MB | ~80 - 150 MB |
| **Backend Core** | **Rust + Tauri v2** | C# / .NET | C++ Closed | C++ Embedded | C# / Electron |
| **Anti-Double-Input (HidHide)** | **Integrated (1-click)** | Manual / Plugin | Proprietary | N/A | Limited |
| **Interactive Curve Canvas** | **Yes (Real-time 60 FPS)** | Basic Preset | Paid Feature | Basic Sliders | Basic Sliders |
| **1 kHz Turbo Polling** | **Native** | Configurable | Configurable | Driver Dependent | Limited |
| **Privacy / Telemetry** | **0% (100% Offline)** | 100% Offline | Online Checks | Steam Telemetry | Offline |
| **Price / License** | **Free (MIT)** | Free (Open) | Paid License | Bundled with Steam | Paid / Freemium |
| **Custom Saved Profiles** | **Yes + JSON Export** | Yes | Yes (Paid) | Cloud Sync | Yes |
| **Portable Executable** | **Yes (Standalone)** | Zip Extraction | No (Install Only) | No | Limited |

---

## 🖥️ Platform Support

PadFlow is **Windows-only (10 / 11, x64)** by design:

- The virtual Xbox 360 controller bridge runs on the **ViGEmBus** kernel driver and the anti-double-input shield on the **HidHide** filter driver — both are **exclusive to Windows** (XInput itself is a Windows API).
- PadFlow is also bound to `hidapi`'s Windows HID backend and the Windows `requireAdministrator` elevation manifest, so cross-platform builds would lose the app's core features (no virtual controller, no cloaking).
- The source tree keeps non-Windows fallbacks in the Rust modules (the app remains cleanly compilable on other platforms for development/auditing), but **releases are only built and published for Windows**.

---

## 📥 Download & Installation

Download the latest release from [Releases](https://github.com/jaimitus/PadFlow/releases):

1. **`PadFlow-Portable.exe`** — Single standalone executable. No installation required.
2. **`PadFlow_1.2.4_x64-setup.exe`** — Recommended Windows installer with start menu shortcuts and bundled driver setup.
3. **`PadFlow_1.2.4_x64_en-US.msi`** — Standard MSI installer package for enterprise / automated deployment.

> **Note:** PadFlow requires the **ViGEmBus driver** (`v1.22.0+`) for virtual Xbox 360 controller emulation, and supports the **HidHide driver** for anti-double-input device cloaking. If missing, PadFlow provides interactive 1-click in-app installer launchers for both official signed drivers.

---

## 🛠️ Building from Source

### Prerequisites
- [Node.js](https://nodejs.org/) (v18+)
- [Rust toolchain](https://www.rust-lang.org/) (MSVC `x86_64-pc-windows-msvc`)
- [Tauri v2 CLI](https://tauri.app/)

### Build Commands

```bash
# 1. Clone the repository
git clone https://github.com/jaimitus/PadFlow.git
cd PadFlow

# 2. Install dependencies
npm install

# 3. Run in development mode
npx tauri dev

# 4. Build production binaries (NSIS setup, MSI, updater artifacts & Portable)
npm run tauri build
```

> **Note (update signing):** production builds sign the update bundles, so they
> require the updater keys. Generate them once (`npx tauri signer generate -w ~/.padflow/padflow-updater.key`)
> and export the two environment variables before building:
>
> ```bash
> export TAURI_SIGNING_PRIVATE_KEY_PATH="$HOME/.padflow/padflow-updater.key"
> export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your-key-password"
> ```
>
> The updater fetches the update manifest from
> `https://github.com/jaimitus/PadFlow/releases/latest/download/latest.json`,
> which is published automatically as part of every release.

---

## 🚀 Cutting a Release (Automated Pipeline)

Releases are **fully automated by GitHub Actions** — no local build, signing or
upload needed. Three workflows guard the pipeline:

1. **`CI - Windows manifest check`** — runs on every push/PR touching `src-tauri/**`
   and verifies the release exe embeds `requireAdministrator` (elevated manifest)
   while the debug exe does **not**.
2. **`Release - publish signed binaries`** — triggered by pushing a version tag.
   It builds and **signs** the bundles with the repo secrets
   (`TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), publishes
   the GitHub release with **all assets** (NSIS installer, MSI, portable exe,
   `.sig` signatures and `latest.json`), and the in-app updater picks it up
   automatically.
3. **`Changelog - auto-update CHANGELOG.md`** — runs on every published release
   and prepends the release notes to `CHANGELOG.md` automatically.

### How to release a new version (3 steps)

```bash
# 1. Bump the version everywhere (package.json, package-lock.json,
#    src/lib/version.ts, src-tauri/Cargo.toml, src-tauri/tauri.conf.json)
#    and write RELEASE_NOTES.md (the release notes — the changelog entry is added
#    to CHANGELOG.md automatically when the release is published).

# 2. Commit and push the bump.

git add -A && git commit -m "v1.2.4: <summary of changes>" && git push origin main

# 3. Tag and push — the pipeline does the rest (build, sign, release, latest.json).

git tag v1.2.4 && git push origin v1.2.4
```

> **Guardrail:** the workflow refuses to run unless the tag version matches
> **all three** sources (`tauri.conf.json`, `package.json`, `Cargo.toml`) —
> fail-fast with a clear message naming the mismatched file. The release exe
> is also re-verified for the `requireAdministrator` manifest before publishing.
>
> **Manual trigger:** the workflow also accepts a manual run (Actions →
> *Release - publish signed binaries* → *Run workflow*) with the `tag` input,
> handy for smoke-testing the pipeline before tagging.

---

## 📜 License

This project is licensed under the **MIT License** — see the [LICENSE](./LICENSE) file for details.

Developed with ❤️ by [jaimitus](https://github.com/jaimitus). If you find PadFlow useful, consider supporting development:

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-Donate-orange.svg?style=for-the-badge&logo=buy-me-a-coffee)](https://buymeacoffee.com/jaimitus)
