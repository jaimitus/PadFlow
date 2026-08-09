# 🛡️ PadFlow v1.2.2 — CLOAK ALL Fixed: Correct HidHide IOCTL Contract

**What's new in this version / Novedades de la Versión 1.2.2:**

- **🛡️ CLOAK ALL finally works — the real root cause is fixed:**
  - PadFlow was sending HidHide IOCTLs with device type `FILE_DEVICE_UNKNOWN`, but the HidHide driver only recognizes its **custom device type 32769** (`0x8001`). Every control call was rejected with `ERROR_INVALID_PARAMETER (87)` — the blacklist write never reached the driver, so cloaking never worked (elevated or not).
  - v1.2.2 mirrors the official `Shared/HidHideIoctlContract.h` from the Nefarius driver exactly: device type `0x8001`, `METHOD_BUFFERED`, `FILE_READ_DATA` on all codes.
  - Blacklist / whitelist / active-state writes now reach the driver, which persists them to the registry from **kernel mode** — cloaking works **without Administrator rights**.
- **🗯️ Honest diagnostics kept:** real error codes and per-device reports from v1.2.1 remain.
- **⚡ Everything from v1.2.0 / v1.2.1 kept:** Shield Control Center, per-controller CLOAK/UNCLOAK, global shield switch, auto-cloak on connect, cloak on startup, tray controls, live 3 s status refresh, one-click signed auto-updates.

---

## 📥 Downloads

| File | Description |
| :--- | :--- |
| `PadFlow-Portable.exe` | Standalone executable — no installation required |
| `PadFlow_1.2.2_x64-setup.exe` | Recommended Windows installer (NSIS) |
| `PadFlow_1.2.2_x64_en-US.msi` | MSI installer for enterprise / automated deployment |
| `latest.json` | Update manifest (used by the built-in updater) |

> **Note:** Requires the **ViGEmBus driver** (v1.22.0+) for virtual Xbox 360 controller emulation. Supports the **HidHide driver** for anti-double-input device cloaking — PadFlow provides 1-click in-app installers for both.

---

## 🛠️ System Requirements

- Windows 10 / 11 (x64)
- ViGEmBus driver 1.22.0+ (auto-install helper included)
- HidHide driver (recommended)

Developed with ❤️ by [jaimitus](https://github.com/jaimitus)
