# 🎮 PadFlow v1.2.6 — Engine Hardening

This release is all about **bulletproofing the engine**: the new regression suite and deep fuzzing pipeline caught and fixed real bugs, and every release is now gated on the full fuzz suite before publishing.

## 🐛 Bug fixes

- **DualSense HID parser off-by-one (poll-thread crash):** the motion-block length guard was one byte short of the deepest read — a truncated report could crash the 1 kHz polling thread. Now it degrades gracefully.
- **DualSense HID parser off-by-one (truncated reports):** the core guard read one index past its floor on 10–11 byte reports. Fixed with exact boundary tests.
- **🔄 Circularity correction was a silent no-op:** the per-axis reach tracker was seeded at `1.0` but inputs are `[-1, 1]` — it never learned the real reach. Now seeded correctly with learn hysteresis and **pass-through until measured**, so fine-aim inputs are never amplified.

## 🧪 Quality

- **59 regression tests** (up from ~4): button remapping, circularity, gyro shaping, HID parsing (DS4/DualSense USB+BT), output reports and CRC-32.
- **Deterministic fuzzing** with zero dependencies: 50k buffers on every test run, plus a **deep suite of 500k+ inputs** (parsers + CRC + full hot-loop pipeline) in a dedicated CI job.
- The whole processing pipeline (shape → remap → circularity → gyro) is now pure and unit-tested.

## 🔒 Release gate

- Every release **runs the full fuzz suite and blocks publishing** until it's green — a broken version can never ship again.

---

**Install:** [PadFlow_1.2.6_x64-setup.exe](https://github.com/jaimitus/PadFlow/releases/latest) · [MSI](https://github.com/jaimitus/PadFlow/releases/latest) · [Portable](https://github.com/jaimitus/PadFlow/releases/latest)
