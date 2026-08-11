# 📜 Changelog

All notable changes to **PadFlow** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> 🤖 **Automated:** every time a release is published on GitHub, the `Changelog - auto-update CHANGELOG.md` workflow prepends the release notes to this file. No manual editing needed for new releases.

---

## [1.2.5] - 2026-08-11

### Added

- **🌀 Gyro Motion Control:** aim with the controller — gyro→mouse or gyro→right stick, with sensitivity, smoothing, invert and one-tap recalibration (DualShock 4 / DualSense / DualSense Edge, per profile).
- **🔄 Real Circularity Correction:** the engine remaps each physical stick's elliptical range toward a perfect circle — beyond the existing measurement tester.
- **🔘 Button Remapping:** map any of the 14 physical PS buttons to any XInput output (per profile, one-click identity reset).
- **🎮 Per-Game Profiles:** assign the current profile to a focused game (.exe) and PadFlow auto-switches when that game takes focus, restoring the controller profile on exit.
- **🌍 Spanish / English (i18n):** full ES/EN localization with a header toggle and persisted preference.
- **📈 Input Oscilloscope:** live 5 s strip-chart of sticks and triggers.
- **⚙️ Settings Panel:** start minimized to tray, minimize-to-tray on close, launch at Windows startup (persisted via the backend store).
- **🩺 Diagnostic Report:** one-tap report with app version, OS, drivers (ViGEmBus / HidHide) and engine stats — copied to the clipboard.

### Changed

- Gyro motion processing, circularity shaping and button remapping now run inside the 1 kHz HID loop.
- All UI copy is now localized through the new i18n layer (English default, Español available).

---

## [1.2.4] - 2026-08-09

### Added

- **🤖 CI manifest check (new GitHub Actions workflow):**
  - Every push/PR touching the Rust side runs a CI job that builds both profiles and verifies the **release exe embeds `requireAdministrator`** (elevated manifest) while the **debug exe does not**.
  - Elevation can never silently regress again — the pipeline catches it before any release is cut.
- **🚀 Fully automated release pipeline:**
  - Pushing a `v*.*.*` tag makes GitHub Actions build, **sign** and publish the entire release — no local builds or manual uploads needed.
  - Publishes **all assets**: NSIS installer, MSI, portable exe, `.sig` signatures and the `latest.json` update manifest (so the built-in auto-updater picks the new version immediately).
  - **Version triple-guard:** the workflow refuses to run unless the tag matches `tauri.conf.json`, `package.json` **and** `Cargo.toml` — fail-fast naming the mismatched file.
  - Manual trigger available (Actions → *Release - publish signed binaries* → Run workflow → tag input) for smoke-testing.
- **🔐 Everything from v1.2.3 kept:** `requireAdministrator` manifest (UAC at launch), correct HidHide IOCTL contract (device type `0x8001`) so cloaking works without admin, honest diagnostics, Shield Control Center, auto-cloak, cloak on startup, tray controls, live 3 s status refresh and one-click signed auto-updates.

## [1.2.3] - 2026-08-09

### Changed

- **🔐 Always elevated — `requireAdministrator` manifest:**
  - Release binaries embed a Windows manifest requesting Administrator privileges, so the UAC prompt appears at launch and the entire session runs elevated.
  - The in-app **"RESTART AS ADMIN"** banner is gone — HidHide cloaking, registry writes and driver helpers always have full privileges from the first second.

## [1.2.2] - 2026-08-09

### Fixed

- **🛡️ CLOAK ALL truly fixed — correct HidHide IOCTL contract:**
  - The root cause of *"No PlayStation controllers to cloak"* was finally found: PadFlow used `FILE_DEVICE_UNKNOWN` as the IOCTL device type, but the HidHide driver expects its **custom device type 32769**. Every IOCTL was rejected with `ERROR_INVALID_PARAMETER (87)` — cloaking never reached the driver.
  - v1.2.2 mirrors the official `HidHideIoctlContract.h` from the Nefarius driver (device type `0x8001`, `METHOD_BUFFERED`, `FILE_READ_DATA`), so blacklist/whitelist/active writes now reach the driver and **cloaking works — even without Administrator rights** (the driver persists to the registry in kernel mode).
- **🧪 IOCTL contract regression test:** a unit test now guards the corrected HidHide IOCTL contract (device type `0x8001`) so the error-87 bug can never silently come back.
- **🗯️ Cleaner active-state errors:** HidHide active-state IOCTL errors are combined into a single clear message and stale admin hints removed.

## [1.2.1] - 2026-08-09

### Fixed

- **🔐 Cloak Fix — Real error reporting + Administrator elevation:**
  - HidHide rejects blacklist writes from non-elevated processes, which made **CLOAK ALL** silently report *"No PlayStation controllers to cloak"* even with a pad connected. v1.2.1 surfaces the **real cause** instead of hiding it.
  - New **elevation banner** with a **"RESTART AS ADMINISTRATOR"** button — one click relaunches PadFlow with UAC, and cloaking works instantly.
  - HidHide read/write errors are now propagated end-to-end (registry + IOCTL), so toasts tell you exactly what failed and why.
  - CLOAK ALL now reports *what* was found: detected controller names, PS pads, and per-device errors.
- **⚙️ Elevate-and-exit single-instance race fixed:** relaunching as Administrator no longer races with the single-instance lock — no ghost instances left behind after the UAC relaunch.
- **⚡ UNCLOAK ALL short-circuit:** uncloaking now short-circuits when nothing is hidden, skipping pointless HidHide writes.

## [1.2.0] - 2026-08-09

### Added

- **🛡️ Shield Control Center:**
  - Per-controller **CLOAK / UNCLOAK** buttons right on every gamepad card, with a live **CLOAKED / VISIBLE** badge per pad.
  - Global shield **ON/OFF switch**, one-click **CLOAK ALL** / **UNCLOAK ALL**, and a live list of hidden devices with counter.
  - **Auto-cloak on connect** (new pads hide as they plug in) and **Cloak on startup** (already-connected pads hide at launch) — both persisted and toggleable.
  - **Tray integration:** cloak / uncloak all controllers straight from the system-tray icon, no need to open the window.
  - "CLOAK ALL" now only hides **PlayStation** pads — never touches Xbox controllers.
- **⚡ Live shield status:** the app now auto-refreshes HidHide state every 3 s, so edits made in the official HidHideClient GUI are reflected instantly.

### Changed

- **⚙️ Hardened update checker:** the background update check now fires exactly once shortly after launch (single-shot + re-entry guard), and when an update can't be installed in-app it falls back to a web-mode CTA opening the release page.

## [1.1.1] - 2026-08-09

### Added

- **⬆️ Automatic Update Detection via GitHub:**
  - New **"Check update"** button in the header plus a silent background check a few seconds after launch.
  - A notification popup appears automatically whenever a **new release is published on GitHub**, showing the release notes.
  - One-click **"Download & install"** with live progress bar, signature-verified signed updates and an automatic restart flow — or open the GitHub release page to grab the portable/installer manually.
- **🔑 Signed Update Pipeline:**
  - Fully configured `tauri-plugin-updater` with Ed25519-signed bundles (`latest.json` manifest published on every GitHub release).
  - "Check update" button always reports the exact state: checking / update available / up to date / error.

## [1.1.0] - 2026-08-09

### Added

- **🎮 Multi-Controller Support — up to 4 gamepads at once:**
  - Connect and emulate up to **4 simultaneous gamepads**, each mapped to its own virtual Xbox 360 controller slot.
  - The realtime engine, gamepad cards and live telemetry manage every pad independently.
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

## [1.0.0] - 2026-08-08

### Added

- 🎉 **Initial public release** of PadFlow — native, ultra-lightweight gamepad calibration studio and virtual Xbox 360 controller emulator (ViGEmBus bridge) for Windows 10 / 11.
  - **🎮 Virtual Xbox 360 controller bridge** via ViGEmBus with a sub-millisecond realtime HID engine polling at up to 1 kHz.
  - **🎮 Direct HID parsing** for DualShock 4, DualSense and DualSense Edge (USB + Bluetooth).
  - **📈 Interactive stick response curve tuner** (Linear / Exponential / S-Curve / Aggressive) with a live 60 FPS canvas.
  - **🎯 Inner / outer / anti-deadzone calibration** per stick.
  - **💾 Custom profile manager** with save / load / export.
  - **📊 Live telemetry** — battery level, connection type, latency and polling rate.
