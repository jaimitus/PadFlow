# 🔐 PadFlow v1.2.3 — Always Elevated: requireAdministrator Manifest

**What's new in this version / Novedades de la Versión 1.2.3:**

- **🔐 PadFlow now requests Administrator rights at launch (UAC):**
  - A `requireAdministrator` Windows manifest is embedded in the release binaries, so Windows shows the elevation prompt every time PadFlow starts and the whole session runs elevated.
  - No more in-app "RESTART AS ADMIN" banner — the app is already elevated from the first second.
  - HidHide cloaking, registry writes, driver installers and tray controls all run with full privileges automatically.
- **⚡ Everything from v1.2.2 kept:** correct HidHide IOCTL contract (device type `0x8001`), honest diagnostics, Shield Control Center, auto-cloak, cloak on startup, tray controls, live 3 s status refresh and one-click signed auto-updates.

> **Note:** because the app launches elevated, Windows will show a UAC prompt when you start PadFlow. This is intentional — it guarantees HidHide can always apply cloak changes.

---

## 📥 Downloads

| File | Description |
| :--- | :--- |
| `PadFlow-Portable.exe` | Standalone executable — no installation required |
| `PadFlow_1.2.3_x64-setup.exe` | Recommended Windows installer (NSIS) |
| `PadFlow_1.2.3_x64_en-US.msi` | MSI installer for enterprise / automated deployment |
| `latest.json` | Update manifest (used by the built-in updater) |

> **Note:** Requires the **ViGEmBus driver** (v1.22.0+) for virtual Xbox 360 controller emulation. Supports the **HidHide driver** for anti-double-input device cloaking — PadFlow provides 1-click in-app installers for both.

---

## 🛠️ System Requirements

- Windows 10 / 11 (x64)
- ViGEmBus driver 1.22.0+ (auto-install helper included)
- HidHide driver (recommended)

Developed with ❤️ by [jaimitus](https://github.com/jaimitus)
