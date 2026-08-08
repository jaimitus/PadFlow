# 🎮 PadFlow — Next-Gen Gamepad Input Calibrator & ViGEmBus Bridge

[![Release](https://img.shields.io/badge/Release-v1.1.0-cyan.svg?style=for-the-badge&logo=windows)](https://github.com/jaimitus/PadFlow/releases)
[![License](https://img.shields.io/badge/License-MIT-violet.svg?style=for-the-badge)](./LICENSE)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-Donate-orange.svg?style=for-the-badge&logo=buy-me-a-coffee)](https://buymeacoffee.com/jaimitus)

**PadFlow** is a native, ultra-lightweight gamepad calibration studio and virtual Xbox 360 controller emulator for **Windows 10 / 11**. Built from the ground up in **Rust** and **Tauri v2**, PadFlow bridges PlayStation 4 (DualShock 4) and PlayStation 5 (DualSense / Edge) controllers to XInput via **ViGEmBus** with sub-millisecond processing latency, zero bloat, and **HidHide anti-double-input protection**.

---

## 📸 Interface Preview

![PadFlow Studio Screenshot](./PadFlow_UI.png)

---

## ✨ Key Features

- **⚡ Sub-Millisecond Realtime Engine:** Multi-threaded HID engine running up to **1,000 Hz (1 kHz turbo polling)** with sub-millisecond input translation.
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
- **🚀 Automated ViGEmBus & HidHide Integration:** Detects driver presence automatically with interactive 1-click installer support.
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

## 📥 Download & Installation

Download the latest release from [Releases](https://github.com/jaimitus/PadFlow/releases):

1. **`PadFlow-Portable.exe`** — Single standalone executable. No installation required.
2. **`PadFlow-Setup-1.1.0.exe`** — Recommended Windows installer with start menu shortcuts and bundled driver setup.
3. **`PadFlow-Installer-1.1.0.msi`** — Standard MSI installer package for enterprise / automated deployment.

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

# 4. Build production binaries (NSIS setup, MSI & Portable)
npx tauri build
```

---

## 📜 License

This project is licensed under the **MIT License** — see the [LICENSE](./LICENSE) file for details.

Developed with ❤️ by [jaimitus](https://github.com/jaimitus). If you find PadFlow useful, consider supporting development:

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-Donate-orange.svg?style=for-the-badge&logo=buy-me-a-coffee)](https://buymeacoffee.com/jaimitus)
