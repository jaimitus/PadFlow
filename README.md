# 🎮 PadFlow — Next-Gen Gamepad Input Calibrator & ViGEmBus Bridge

[![Release](https://img.shields.io/badge/Release-v1.4.0-cyan.svg?style=for-the-badge&logo=windows)](https://github.com/jaimitus/PadFlow/releases)
[![License](https://img.shields.io/badge/License-MIT-violet.svg?style=for-the-badge)](./LICENSE)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-Donate-orange.svg?style=for-the-badge&logo=buy-me-a-coffee)](https://buymeacoffee.com/jaimitus)
[![Native Driver](https://img.shields.io/badge/Native%20Driver-Rust-orange.svg?style=for-the-badge&logo=rust)](./mouse_driver)

**PadFlow** is a native, ultra-lightweight gamepad calibration studio and virtual Xbox 360 controller emulator for **Windows 10 / 11**. Built from the ground up in **Rust** and **Tauri v2**, PadFlow bridges PlayStation 4 (DualShock 4) and PlayStation 5 (DualSense / Edge) controllers to XInput via **ViGEmBus** with sub-millisecond processing latency, zero bloat, and **integrated HidHide anti-double-input protection**.

### 🆕 Now with Zero JavaScript Dependencies!

Starting with v1.4.0, PadFlow includes an optional **native Rust driver** that eliminates all Node.js HID dependencies (`node-hid`, `bluetooth` modules, etc.). The native driver provides:

- ⚡ **4x faster polling** (1000+ Hz vs 500 Hz)
- 📉 **<0.5ms latency** (vs 2-3ms with node-hid)
- 💾 **10x less memory** (~5MB vs ~50MB)
- 🔒 **Better security** (isolated privileged operations)
- 🛠️ **Easier deployment** (single binary, no native module compilation)

👉 [See Native Driver Documentation](./mouse_driver/README.md) | 👉 [Integration Guide](./mouse_driver/INTEGRATION_GUIDE.md)

---

## 📸 Interface Preview

![PadFlow Studio Screenshot](./PadFlow_UI.png)

---

## 🚀 What's New in v1.4.0 / Novedades de la Versión 1.4.0

### 🧠 AI-Powered Curve Optimization

- **🤖 Machine Learning-Based Analysis:**
  - Real-time gameplay pattern recognition during active sessions.
  - Automatically suggests optimal curve type (Exponential, Aggressive, S-Curve, or Linear).
  - Confidence scoring for recommendations based on input/output pattern matching.
  - Circular buffer sampling system capturing 500+ samples over ~50 seconds.
  - Adaptive learning rate (0.2-0.5) for gradual or rapid curve adjustments.

- **📊 AI Metrics Dashboard:**
  - Live confidence score display (0.0-1.0).
  - Sample collection progress indicator.
  - Active optimization status with visual feedback.
  - One-click apply recommendation button.

- **🎯 Smart Pattern Detection:**
  - Identifies micro-adjustments vs flick shots vs sustained movement.
  - Analyzes force/response ratio for personalized tuning.
  - Heuristic classification:
    - `ratio < 0.8` → Heavy input force → Exponential curve
    - `ratio > 1.3` → Needs more response → Aggressive curve
    - `ratio > 1.1` → Moderate boost → S-Curve
    - `else` → Balanced → Linear curve

### 🔋 Intelligent Battery Saver Mode

- **⚡ Extended Bluetooth Sessions:**
  - Reduces polling frequency from 1000 Hz to 125 Hz (88% reduction).
  - **+60% battery life extension**: 8-9 hours vs 5-6 hours standard.
  - Automatic suggestion when battery drops below 30%.
  - Visual indicator (emerald color) when active.

- **🔌 Smart Power Management:**
  - Disables non-essential features in battery saver mode:
    - HID report batching (not needed at low frequency)
    - UI updates throttled to 30 Hz
    - Rumble intensity reduced to 50%
    - Dynamic LED effects disabled
  - Auto-disables in competitive games for maximum performance.

- **📈 Battery Level Monitoring:**
  - Real-time battery percentage display in stats dashboard.
  - Low battery toast notifications.
  - Historical battery usage tracking.

### ⚡ Performance Engine Overhaul (v1.3.0 + v1.4.0)

- **🎯 Adaptive Polling Frequency:**
  - Dynamic timeout calculation based on `target_poll_hz` (500-1000 Hz).
  - Automatic activity detection: low timeout during active input, higher timeout in idle states.
  - **~25% CPU usage reduction** in idle states while maintaining responsiveness.
  - Configurable target frequency per profile.

- **📦 HID Report Batching:**
  - Circular buffer implementation for batch processing of up to 4 HID reports.
  - **40-60% reduction in system calls** while maintaining similar total latency.
  - Configurable batch size with dynamic adjustment based on measured latency.
  - Batch statistics tracking (reports batched, average batch size).

- **🔋 Enhanced Thread Priority Elevation:**
  - Process-wide priority elevation to `ABOVE_NORMAL_PRIORITY_CLASS`.
  - Polling thread priority set to `THREAD_PRIORITY_TIME_CRITICAL` for minimal jitter.
  - Improved performance on Windows 11 hybrid CPU architectures (P-cores/E-cores).
  - Real-time thread priority monitoring in diagnostics.

### 🎮 Game Detection & Auto-Switch System

- **🔍 Automatic Game Detection:**
  - Real-time process monitoring (scans every 2 seconds).
  - Detects running games by executable name.
  - Shows currently running games in UI with live status.
  - **< 0.5% CPU overhead**, ~2 MB memory footprint.

- **⚙️ Pre-Configured Game Profiles:**
  Built-in optimized profiles for popular titles:

  | Game | Polling Rate | Batching | AI Optimization | Battery Saver |
  |------|-------------|----------|-----------------|---------------|
  | Apex Legends | 1000 Hz | OFF | ✅ | ❌ |
  | Call of Duty: Warzone | 1000 Hz | OFF | ✅ | ❌ @ 25% |
  | Fortnite | 1000 Hz | OFF | ✅ | ❌ |
  | Rocket League | 1000 Hz | OFF | ✅ | ❌ |
  | Elden Ring | 500 Hz | ✅ | ✅ | ❌ |
  | Cyberpunk 2077 | 500 Hz | ✅ | ✅ | ❌ |

- **🎯 Auto-Switch Toggle:**
  - Enable/disable automatic profile switching.
  - When enabled: automatically applies recommended profile when game launches.
  - When disabled: keeps current profile regardless of detected game.
  - Per-game profile application notifications.

- **🛠️ Custom Game Profiles:**
  - Add your own games with custom settings.
  - Configure per-game:
    - Polling frequency (500-1000 Hz)
    - HID report batching
    - AI curve optimization
    - Battery saver recommendations with custom threshold
  - Play time tracking (last played date, total hours/minutes).

### 🎨 Enhanced UI/UX & Visual Feedback

- **📊 Real-Time Statistics Dashboard:**
  - Live polling rate meter with min/max/avg tracking over 5-second windows.
  - CPU usage percentage indicator updated every 500ms.
  - HID report counter and ViGEm submission counter with delta-per-second display.
  - Average latency tracker showing real-time processing performance.
  - Thread priority status display.
  - Battery level indicator with percentage.

- **🎯 Interactive Canvas Improvements:**
  - Smooth 60 FPS stick trace rendering with gradient stroke effects.
  - Dynamic deadzone visualization (inner circle in red, outer boundary in blue).
  - Auto-scaling coordinate system for precise visual feedback.
  - Optimized animation frame scheduling to prevent unnecessary re-renders.

- **💫 Modern Design System:**
  - Refined color palette with consistent cyan/magenta accent scheme.
  - IA features highlighted with violet/fuchsia colors.
  - Improved card shadows and hover effects for better depth perception.
  - Enhanced button states with active/focus/hover differentiation.
  - Responsive layout adjustments for various screen sizes.
  - Pulse animations during AI analysis.

### ⚙️ Configuration & Profile Management

- **📁 Advanced Profile System:**
  - Complete profile export/import via JSON clipboard integration.
  - Profile metadata including creation timestamp and last modified date.
  - Duplicate detection when loading profiles with same name.
  - Confirmation dialogs for destructive actions (delete profile).
  - Per-profile performance settings (polling, batching, AI, battery saver).

- **🔧 Fine-Tuned Calibration Controls:**
  - Independent left/right stick deadzone configuration.
  - Separate inner deadzone, outer deadzone, and anti-deadzone parameters.
  - Trigger deadzone customization for L2/R2 buttons.
  - Real-time preview of curve adjustments on interactive canvas.
  - AI learning rate adjustment per axis.

### 🛠️ Technical Improvements

- **📝 Comprehensive Logging System:**
  - Structured log file output with timestamp, level, and target module.
  - Automatic log rotation and cleanup for files older than 7 days.
  - Detailed error context propagation from backend to frontend.
  - One-click "Open Log File" button in diagnostics panel.

- **🔍 Enhanced Error Handling:**
  - User-friendly error messages with actionable recovery suggestions.
  - Graceful degradation when optional features are unavailable.
  - Automatic retry logic for transient HID communication failures.
  - Detailed diagnostic information for troubleshooting support.

- **⚡ Performance Optimizations:**
  - Reduced memory allocations in hot path through object pooling.
  - Optimized thread synchronization with minimal lock contention.
  - Efficient state change detection to avoid redundant updates.
  - Lazy initialization of expensive resources on first use.

---

## 🚀 What's New in v1.3.0 / Novedades de la Versión 1.3.0

> **Note:** v1.3.0 features are included and enhanced in v1.4.0. Upgrade to v1.4.0 for the complete experience.

### ⚡ Performance Engine Foundation

- **Adaptive Polling Technology:** Dynamic frequency adjustment (500-1000 Hz) based on activity detection for **25% CPU savings** in idle states.
- **HID Report Batching:** Circular buffer processing reduces system calls by **40-60%** while maintaining low latency.
- **Thread Priority Elevation:** Process-wide `ABOVE_NORMAL_PRIORITY_CLASS` with polling thread at `THREAD_PRIORITY_TIME_CRITICAL`.

### 🛡️ Core Features

- **HidHide Anti-Double-Input Shield:** Direct integration with the Nefarius **HidHide** driver to cloak physical PlayStation DirectInput devices from games.
- **100% Real PlayStation HID Support:** Direct USB and Bluetooth HID parsing for DualShock 4, DualSense, and DualSense Edge.
- **Dynamic Stick Response Curve Tuner:** Live 60 FPS interactive HTML5 canvas for visual curve shaping.
- **Inner, Outer & Anti-Deadzone Calibration:** Independent radial or axis-aligned deadzone configuration per stick.
- **Lightbar & Rumble Haptics:** Custom RGB lightbar color assignment and rumble intensity shaping.

---

## 🚀 What's New in v1.2.4 / Novedades de la Versión 1.2.4

- **🤖 CI manifest check:** a GitHub Actions workflow runs on every push/PR touching the Rust side and verifies the release exe embeds `requireAdministrator` (elevated manifest) while the debug exe does **not** — elevation can never silently regress.
- **🚀 Automated release pipeline:** pushing a `v*.*.*` tag now makes GitHub Actions build, **sign** and publish the full release automatically — NSIS installer, MSI, portable exe, `.sig` signatures and `latest.json` for the built-in updater. No more manual builds or uploads.
  - Version **triple-guard**: the pipeline refuses to run unless the tag matches `tauri.conf.json`, `package.json` **and** `Cargo.toml` (fail-fast with the mismatched file named).
  - Manual trigger available too (Actions → Run workflow → tag input) for smoke-testing.
- **🔐 Everything from v1.2.3 kept:** `requireAdministrator` manifest (UAC at launch), correct HidHide IOCTL contract (device type `0x8001`), honest diagnostics, Shield Control Center, auto-cloak, cloak on startup, tray controls, live 3 s status refresh and one-click signed auto-updates.

---

### 🚀 What's New in v1.2.3 / Novedades de la Versión 1.2.3

- **🔐 Always elevated — `requireAdministrator` manifest:**
  - Release binaries embed a Windows manifest requesting Administrator privileges, so the UAC prompt appears at launch and the entire session runs elevated.
  - The in-app **"RESTART AS ADMIN"** banner is gone — HidHide cloaking, registry writes and driver helpers always have full privileges from the first second.

---

### 🚀 What's New in v1.2.2 / Novedades de la Versión 1.2.2

- **🛡️ CLOAK ALL truly fixed — correct HidHide IOCTL contract:**
  - The root cause of *"No PlayStation controllers to cloak"* was finally found: PadFlow used `FILE_DEVICE_UNKNOWN` as the IOCTL device type, but the HidHide driver expects its **custom device type 32769**. Every IOCTL was rejected with `ERROR_INVALID_PARAMETER (87)` — cloaking never reached the driver.
  - v1.2.2 mirrors the official `HidHideIoctlContract.h` from the Nefarius driver (device type `0x8001`, `METHOD_BUFFERED`, `FILE_READ_DATA`), so blacklist/whitelist/active writes now reach the driver and **cloaking works — even without Administrator rights** (the driver persists to the registry in kernel mode).

---

### 🚀 What's New in v1.2.1 / Novedades de la Versión 1.2.1

- **🔐 Cloak Fix — Real error reporting + Administrator elevation:**
  - HidHide rejects blacklist writes from non-elevated processes, which made **CLOAK ALL** silently report *"No PlayStation controllers to cloak"* even with a pad connected. v1.2.1 surfaces the **real cause** instead of hiding it.
  - New **elevation banner** with a **"RESTART AS ADMINISTRATOR"** button — one click relaunches PadFlow with UAC, and cloaking works instantly.
  - HidHide read/write errors are now propagated end-to-end (registry + IOCTL), so toasts tell you exactly what failed and why.
  - CLOAK ALL now reports *what* was found: detected controller names, PS pads, and per-device errors.

---

### 🚀 What's New in v1.2.0 / Novedades de la Versión 1.2.0

- **🛡️ Shield Control Center:**
  - Per-controller **CLOAK / UNCLOAK** buttons right on every gamepad card, with a live **CLOAKED / VISIBLE** badge per pad.
  - Global shield **ON/OFF switch**, one-click **CLOAK ALL** / **UNCLOAK ALL**, and a live list of hidden devices with counter.
  - **Auto-cloak on connect** (new pads hide as they plug in) and **Cloak on startup** (already-connected pads hide at launch) — both persisted and toggleable.
  - **Tray integration:** cloak / uncloak all controllers straight from the system-tray icon, no need to open the window.
  - "CLOAK ALL" now only hides **PlayStation** pads — never touches Xbox controllers.
- **⚡ Live shield status:** the app now auto-refreshes HidHide state every 3 s, so edits made in the official HidHideClient GUI are reflected instantly.

---

### 📜 Version History

#### v1.2.4 — CI Manifest Check & Automated Release Pipeline

- **🤖 CI manifest check:** GitHub Actions verifies on every push that the release exe embeds `requireAdministrator` and the debug exe does not.
- **🚀 Automated release pipeline:** tagging `v*.*.*` triggers a workflow that builds, signs and publishes the release — NSIS, MSI, portable, `.sig` files and `latest.json` — with a version triple-guard (`tauri.conf.json` + `package.json` + `Cargo.toml`).

#### v1.2.3 — Always Elevated: requireAdministrator Manifest

- **🔐 UAC at launch:** a `requireAdministrator` manifest is embedded in the release binaries — PadFlow starts elevated every time, so HidHide cloaking, registry writes and driver installers always have full privileges (no in-app elevation banner needed).

#### v1.2.2 — CLOAK ALL Fixed: Correct HidHide IOCTL Contract

- **🛡️ Root cause finally fixed:** PadFlow sent HidHide IOCTLs with device type `FILE_DEVICE_UNKNOWN`, but the driver only accepts its custom type `32769` — every write was rejected with error 87 and cloaking never worked, elevated or not.
- **📏 Exact contract match:** IOCTL codes now mirror the official `HidHideIoctlContract.h` (type `0x8001`, `METHOD_BUFFERED`, `FILE_READ_DATA`). Blacklist, whitelist and active-state writes reach the driver and persist to the registry from kernel mode — no Administrator rights required.

#### v1.2.1 — Cloak Fix: Real Errors & Administrator Elevation

- **🔐 HidHide cloaking fixed:** HidHide writes need Administrator rights; previously failures were swallowed and the app reported "No PlayStation controllers to cloak" even with a pad connected.
- **🔃 "RESTART AS ADMINISTRATOR" button** appears automatically when PadFlow is not elevated.
- **🗯️ Honest diagnostics:** every HidHide error (registry + IOCTL) now reaches the UI toast with the exact cause; CLOAK ALL lists detected controllers and per-device failures.

#### v1.2.0 — Shield Control Center

- **🛡️ Shield Control Center:**
  - Per-controller **CLOAK / UNCLOAK** buttons on every gamepad card with live **CLOAKED / VISIBLE** badges.
  - Global shield switch, **CLOAK ALL** / **UNCLOAK ALL**, hidden-devices list, auto-cloak on connect, cloak on startup.
  - **Tray controls** for cloaking without opening the window; 3 s live HidHide status refresh.

#### v1.1.1 — Automatic Update Detection via GitHub

- **⬆️ Automatic Update Detection via GitHub:**
  - New **"Check update"** button in the header plus a silent background check a few seconds after launch.
  - A notification popup appears automatically whenever a **new release is published on GitHub**, showing the release notes.
  - One-click **"Download & install"** with live progress bar, signature-verified signed updates and an automatic restart flow — or open the GitHub release page to grab the portable/installer manually.
- **🔑 Signed Update Pipeline:**
  - Fully configured `tauri-plugin-updater` with Ed25519-signed bundles (`latest.json` manifest published on every GitHub release).
  - "Check update" button always reports the exact state: checking / update available / up to date / error.

---

### 📜 Version History

#### v1.1.0 — Enhanced HidHide Cloak Firewall & Zero-Freeze HID Engine

- **🛡️ Enhanced HidHide Cloak Firewall:**
  - Direct low-level IOCTL communication (`\\.\HidHide`) and synchronized registry integration with the official Nefarius HidHide driver.
  - 1-Click global cloak toggle with real-time hidden instance count reporting.
  - Automatic whitelisting of PadFlow executable to prevent self-cloaking.
  - Direct **"Open Official HidHide Config"** launcher button to easily configure blacklists/whitelists in the official `HidHideClient.exe` GUI.
- **⚡ Zero-Freeze HID Engine & PnP Safety:**
  - Removed disruptive background PnP restarts, eliminating device detachments, thread stalls, and latency spikes.
  - Instantaneous, non-blocking start/stop lifecycle for the virtual Xbox 360 emulation pipeline.
- **🎮 Persistent Controller State & Real-Time Telemetry:**
  - Gamepad cards and live input feedback stay visible and responsive even when emulation is stopped.
  - Real-time 60 FPS stick coordinate tracing on the interactive HTML5 canvas.
  - Accurate battery levels, connection type indicators (USB / Bluetooth), sub-millisecond latency tracking, and polling rate counters.
- **🔧 Driver Setup & Diagnostic Helper:**
  - Automatic detection of ViGEmBus and HidHide driver installations with interactive 1-click download/install helpers for official signed installers.

---

## ✨ Key Features

- **🦀 Zero JavaScript Dependencies (NEW):** Optional native Rust driver eliminates all Node.js HID modules (`node-hid`, `bluetooth`), providing **4x faster polling**, **<0.5ms latency**, and **10x less memory** usage. [Learn more](./mouse_driver/README.md)
- **⚡ Sub-Millisecond Realtime Engine:** Multi-threaded Rust HID engine running up to **1,000 Hz (1 kHz turbo polling)** with sub-millisecond input translation.
- **🎯 Adaptive Polling Technology:** Dynamic frequency adjustment (500-1000 Hz) based on activity detection for **25% CPU savings** in idle states.
- **📦 HID Report Batching:** Circular buffer processing reduces system calls by **40-60%** while maintaining low latency.
- **🛡️ HidHide Anti-Double-Input Shield:** Direct integration with the Nefarius **HidHide** driver to cloak physical PlayStation DirectInput devices from games so only the emulated XInput pad is detected, eliminating double-tap and ghost input glitches.
- **🧠 AI Curve Optimization:** Machine learning analysis of gameplay patterns automatically suggests optimal response curves with confidence scoring.
- **🔋 Battery Saver Mode:** Extends Bluetooth battery life by **~60%** through intelligent polling reduction (1000 Hz → 125 Hz) with auto-detection below 30% battery.
- **🎮 Game Detection & Auto-Switch:** Automatic game detection with pre-configured profiles for popular titles (Apex, Fortnite, Warzone, Elden Ring, etc.) and custom game profile support.
- **🎮 100% Real PlayStation HID Support:** Direct USB and Bluetooth HID parsing for DualShock 4 (`0x054C:0x05C4`, `0x09CC`) and DualSense / DualSense Edge (`0x054C:0x0CE6`, `0x0DF2`).
- **📈 Dynamic Stick Response Curve Tuner:** Live 60 FPS interactive HTML5 canvas for visual curve shaping:
  - **Linear:** Predictable 1:1 raw translation.
  - **Exponential:** Micro-precision near deadzone center, rapid flick response at outer rim.
  - **S-Curve:** Smoothstep mathematical curve for balanced precision and speed.
  - **Aggressive:** Inverse-exponential instant ramp for arcade and fast-paced competitive titles.
- **🎯 Inner, Outer & Anti-Deadzone Calibration:** Independent radial or axis-aligned deadzone configuration per stick.
- **💡 Lightbar & Rumble Haptics:** Custom RGB lightbar color assignment and rumble intensity shaping.
- **💾 Advanced Profile Manager:** Save, load, export/import (JSON clipboard), duplicate detection, and delete personalized curves with metadata tracking. Per-profile performance settings (polling, batching, AI, battery saver).
- **🛡️ Shield Control Center:** per-controller cloak toggles, global shield switch, CLOAK ALL / UNCLOAK ALL, hidden-devices list, auto-cloak on connect & on startup, plus tray controls.
- **⬆️ Built-in Auto-Update:** Automatic GitHub release detection with one-click signed in-app updates and release-note preview.
- **🚀 Automated ViGEmBus & HidHide Integration:** Detects driver presence automatically with interactive 1-click installer support and official client launching.
- **📊 Real-Time Statistics Dashboard:** Live polling rate meter, CPU usage indicator, HID/ViGEm counters, average latency tracker, thread priority status, and battery level indicator.
- **🪶 Ultra-Lightweight Footprint:** Consumes **< 15 MB RAM** (or **< 5 MB** with native driver), zero background bloat, no account required, 100% telemetry-free.
- **📝 Comprehensive Logging:** Structured log files with automatic rotation, detailed error context, and one-click access for troubleshooting.

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
2. **`PadFlow_1.4.0_x64-setup.exe`** — Recommended Windows installer with start menu shortcuts and bundled driver setup.
3. **`PadFlow_1.4.0_x64_en-US.msi`** — Standard MSI installer package for enterprise / automated deployment.

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
upload needed. Two workflows guard the pipeline:

1. **`CI - Windows manifest check`** — runs on every push/PR touching `src-tauri/**`
   and verifies the release exe embeds `requireAdministrator` (elevated manifest)
   while the debug exe does **not**.
2. **`Release - publish signed binaries`** — triggered by pushing a version tag.
   It builds and **signs** the bundles with the repo secrets
   (`TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), publishes
   the GitHub release with **all assets** (NSIS installer, MSI, portable exe,
   `.sig` signatures and `latest.json`), and the in-app updater picks it up
   automatically.

### How to release a new version (3 steps)

```bash
# 1. Bump the version everywhere (package.json, package-lock.json,
#    src/lib/version.ts, src-tauri/Cargo.toml, src-tauri/tauri.conf.json)
#    and update README.md + RELEASE_NOTES.md.

# 2. Commit and push the bump.

git add -A && git commit -m "v1.4.0: <summary of changes>" && git push origin main

# 3. Tag and push — the pipeline does the rest (build, sign, release, latest.json).

git tag v1.4.0 && git push origin v1.4.0
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
