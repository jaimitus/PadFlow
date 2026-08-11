# 🎮 PadFlow v1.2.5 — The Big Feature Drop

8 new features, all wired into the 1 kHz engine:

## 🌀 Gyro Motion Control
Aim with the controller — **gyro → mouse** or **gyro → right stick**, with sensitivity, smoothing, invert and one-tap recalibration. Works on DualShock 4, DualSense and DualSense Edge, and is stored **per profile**.

## 🔄 Real Circularity Correction
The engine now **corrects** each physical stick's elliptical range toward a perfect circle — beyond the existing measurement tester.

## 🔘 Button Remapping
Map any of the **14 physical PS buttons** to any XInput output (✕→A, ○→B, L1→LB...), per profile, with a one-click identity reset.

## 🎮 Per-Game Profiles
Assign the current profile to a focused game (`.exe`) — PadFlow **auto-switches** to it whenever that game takes focus and restores your controller profile when you leave.

## 🌍 Spanish / English (i18n)
The whole UI is now localized in **ES/EN** with a header toggle and a persisted preference.

## 📈 Input Oscilloscope
Live 5-second strip-chart of sticks and triggers.

## ⚙️ Settings Panel
Start minimized to tray, minimize-to-tray on close, launch at Windows startup — persisted in the backend store.

## 🩺 Diagnostic Report
One-tap report with app version, OS, drivers (ViGEmBus / HidHide) and engine stats — copied to the clipboard for support.

---

**Includes everything from v1.2.4**: automated release pipeline, CI manifest check, `requireAdministrator` elevation, HidHide Shield Control Center, auto-cloak, tray controls and signed one-click updates.
