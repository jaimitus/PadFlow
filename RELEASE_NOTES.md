# 🎮 PadFlow v1.2.0 — Shield Control Center 🛡️

**What's new in this version / Novedades de la Versión 1.2.0:**

- **🛡️ Shield Control Center:**
  - **Per-controller CLOAK / UNCLOAK** buttons on every gamepad card, with a live **CLOAKED / VISIBLE** badge per pad.
  - Global shield **ON/OFF switch**, one-click **CLOAK ALL** / **UNCLOAK ALL**, and a live **hidden-devices list** with counter.
  - **Auto-cloak on connect** — new PlayStation pads hide as they plug in.
  - **Cloak on startup** — already-connected pads hide automatically at launch.
  - **Tray integration** — cloak / uncloak all controllers straight from the system-tray icon.
  - "CLOAK ALL" now only hides **PlayStation** pads (never Xbox controllers).
- **⚡ Live shield status:** HidHide state auto-refreshes every 3 s — edits made in the official HidHideClient GUI are reflected instantly.
- **⬆️ Auto-update kept:** GitHub release detection with one-click signed in-app updates.

---

## 📥 Downloads

| File | Description |
| :--- | :--- |
| `PadFlow-Portable.exe` | Standalone executable — no installation required |
| `PadFlow_1.2.0_x64-setup.exe` | Recommended Windows installer (NSIS) |
| `PadFlow_1.2.0_x64_en-US.msi` | MSI installer for enterprise / automated deployment |
| `latest.json` | Update manifest (used by the built-in updater) |

> **Note:** Requires the **ViGEmBus driver** (v1.22.0+) for virtual Xbox 360 controller emulation. Supports the **HidHide driver** for anti-double-input device cloaking — PadFlow provides 1-click in-app installers for both.

---

## 🛠️ System Requirements

- Windows 10 / 11 (x64)
- ViGEmBus driver 1.22.0+ (auto-install helper included)
- HidHide driver (recommended)

Developed with ❤️ by [jaimitus](https://github.com/jaimitus)
