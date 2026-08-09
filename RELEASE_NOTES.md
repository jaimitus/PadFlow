# 🔐 PadFlow v1.2.1 — Cloak Fix: Real Errors & Administrator Elevation

**What's new in this version / Novedades de la Versión 1.2.1:**

- **🔐 CLOAK ALL finally works — and tells the truth when it can't:**
  - HidHide rejects blacklist changes from non-elevated processes, which made the app silently report *"No PlayStation controllers to cloak"* even with a DualSense connected (the write was failing behind the scenes).
  - v1.2.1 detects elevation and shows a **🔐 banner with a "RESTART AS ADMINISTRATOR" button** — one click relaunches PadFlow with UAC and cloaking works immediately.
- **🗯️ Honest diagnostics everywhere:**
  - Every HidHide read/write error (registry + IOCTL) is now propagated end-to-end to the UI toast with the exact cause.
  - CLOAK ALL reports what it found: connected controller names, PlayStation pads detected, and per-device errors — no more silent failures.
- **⚡ Everything from v1.2.0 kept:** Shield Control Center, per-controller CLOAK/UNCLOAK, global shield switch, auto-cloak on connect, cloak on startup, tray controls, live 3 s status refresh, and one-click signed auto-updates.

---

## 📥 Downloads

| File | Description |
| :--- | :--- |
| `PadFlow-Portable.exe` | Standalone executable — no installation required |
| `PadFlow_1.2.1_x64-setup.exe` | Recommended Windows installer (NSIS) |
| `PadFlow_1.2.1_x64_en-US.msi` | MSI installer for enterprise / automated deployment |
| `latest.json` | Update manifest (used by the built-in updater) |

> **Note:** Requires the **ViGEmBus driver** (v1.22.0+) for virtual Xbox 360 controller emulation. Supports the **HidHide driver** for anti-double-input device cloaking — PadFlow provides 1-click in-app installers for both. HidHide configuration changes require running PadFlow as **Administrator** (the app offers a one-click elevated relaunch).

---

## 🛠️ System Requirements

- Windows 10 / 11 (x64)
- ViGEmBus driver 1.22.0+ (auto-install helper included)
- HidHide driver (recommended)

Developed with ❤️ by [jaimitus](https://github.com/jaimitus)
