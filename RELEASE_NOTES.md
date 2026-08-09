# 🤖 PadFlow v1.2.4 — CI Manifest Check & Automated Release Pipeline

**What's new in this version / Novedades de la Versión 1.2.4:**

- **🤖 CI manifest check (new GitHub Actions workflow):**
  - Every push/PR touching the Rust side now runs a CI job that builds both profiles and verifies the **release exe embeds `requireAdministrator`** (elevated manifest) while the **debug exe does not**.
  - Elevation can never silently regress again — the pipeline catches it before any release is cut.
- **🚀 Fully automated release pipeline:**
  - Pushing a `v*.*.*` tag makes GitHub Actions build, **sign** and publish the entire release — no local builds or manual uploads needed.
  - Publishes **all assets**: NSIS installer, MSI, portable exe, `.sig` signatures and the `latest.json` update manifest (so the built-in auto-updater picks the new version immediately).
  - **Version triple-guard:** the workflow refuses to run unless the tag matches `tauri.conf.json`, `package.json` **and** `Cargo.toml` — fail-fast naming the mismatched file.
  - Manual trigger available (Actions → *Release - publish signed binaries* → Run workflow → tag input) for smoke-testing.
- **🔐 Everything from v1.2.3 kept:** `requireAdministrator` manifest (UAC at launch), correct HidHide IOCTL contract (device type `0x8001`) so cloaking works without admin, honest diagnostics, Shield Control Center, auto-cloak, cloak on startup, tray controls, live 3 s status refresh and one-click signed auto-updates.

> **Note:** because the app launches elevated, Windows will show a UAC prompt when you start PadFlow. This is intentional — it guarantees HidHide can always apply cloak changes.

---

## 📥 Downloads

| File | Description |
| :--- | :--- |
| `PadFlow-Portable.exe` | Standalone executable — no installation required |
| `PadFlow_1.2.4_x64-setup.exe` | Recommended Windows installer (NSIS) |
| `PadFlow_1.2.4_x64_en-US.msi` | MSI installer for enterprise / automated deployment |
| `latest.json` | Update manifest (used by the built-in updater) |

> **Note:** Requires the **ViGEmBus driver** (v1.22.0+) for virtual Xbox 360 controller emulation. Supports the **HidHide driver** for anti-double-input device cloaking — PadFlow provides 1-click in-app installers for both.

---

## 🛠️ System Requirements

- Windows 10 / 11 (x64)
- ViGEmBus driver 1.22.0+ (auto-install helper included)
- HidHide driver (recommended)

Developed with ❤️ by [jaimitus](https://github.com/jaimitus)
