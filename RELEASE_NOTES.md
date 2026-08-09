# 🎮 PadFlow v1.1.1 — Auto-Update & GitHub Release Detection 🚀

**⬆️ What's new in this version / Novedades de la Versión 1.1.1:**

- **Automatic update detection via GitHub:** new **"Check update"** button in the header plus a silent background check a few seconds after launch.
- **Notification popup** appears automatically whenever a **new release is published on GitHub**, with a release-notes preview.
- **One-click signed in-app updates:** **"Download & install"** with a live progress bar, signature-verified bundles and an automatic restart flow — or open the release page to grab the portable / installer manually.
- **Signed update pipeline:** `tauri-plugin-updater` fully configured (Ed25519 keys), `latest.json` manifest published on every release.
- Version bumps everywhere (`v1.1.1`).

---

## 📥 Downloads

| File | Description |
| :--- | :--- |
| `PadFlow-Portable.exe` | Standalone executable — no installation required |
| `PadFlow_1.1.1_x64-setup.exe` | Recommended Windows installer (NSIS) |
| `PadFlow_1.1.1_x64_en-US.msi` | MSI installer for enterprise / automated deployment |
| `latest.json` | Update manifest (used by the built-in updater) |

> **Note:** Requires the **ViGEmBus driver** (v1.22.0+) for virtual Xbox 360 controller emulation. Supports the **HidHide driver** for anti-double-input device cloaking. PadFlow provides 1-click in-app installers for both.

---

## 🛠️ System Requirements

- Windows 10 / 11 (x64)
- ViGEmBus driver 1.22.0+ (auto-install helper included)
- HidHide driver (recommended)

Developed with ❤️ by [jaimitus](https://github.com/jaimitus)
