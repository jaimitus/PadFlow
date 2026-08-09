// ---------------------------------------------------------------------------
// HidHide device-path helpers.
//
// These mirror the Rust normalization in src-tauri/src/hidhide.rs
// (normalize_device_instance_path / extract_all_device_instance_ids) so the
// frontend can answer "is this controller cloaked right now?" by matching a
// controller's device path against the driver's blacklist — with the exact
// same string rules the backend uses when it writes entries.
// ---------------------------------------------------------------------------

/** Strips prefixes / GUID tails and normalizes # to \ (uppercase). */
export function normalizeDeviceInstancePath(path: string): string {
  let clean = path.trim();
  if (clean.startsWith("\\\\?\\")) clean = clean.slice(4);
  else if (clean.startsWith("\\\\.\\")) clean = clean.slice(4);

  const guidIdx = clean.lastIndexOf("#{");
  if (guidIdx >= 0) {
    clean = clean.slice(0, guidIdx);
  } else {
    const hashIdx = clean.lastIndexOf("#");
    if (hashIdx >= 0) {
      const tail = clean.slice(hashIdx + 1);
      if (tail.includes("-") || tail.length >= 32) {
        clean = clean.slice(0, hashIdx);
      }
    }
  }

  return clean.replace(/#/g, "\\").toUpperCase();
}

/**
 * Generates every collection / container ID HidHide needs to fully hide a
 * controller (COL01..COL06 + base HID + USB parent) — same as the Rust side.
 */
export function extractAllDeviceInstanceIds(path: string): string[] {
  const ids: string[] = [];
  const norm = normalizeDeviceInstancePath(path);
  if (!norm) return ids;
  ids.push(norm);

  const colIdx = norm.indexOf("&COL");
  if (colIdx >= 0) {
    const slashOffset = norm.slice(colIdx).indexOf("\\");
    if (slashOffset >= 0) {
      const prefix = norm.slice(0, colIdx);
      const suffix = norm.slice(colIdx + slashOffset);
      for (let c = 1; c <= 6; c++) {
        const colId = `${prefix}&COL${String(c).padStart(2, "0")}${suffix}`;
        if (!ids.includes(colId)) ids.push(colId);
      }
      const baseHid = `${prefix}${suffix}`;
      if (!ids.includes(baseHid)) ids.push(baseHid);
    }
  }

  if (norm.startsWith("HID\\VID_")) {
    const usbId = norm.replace(/^HID\\/, "USB\\");
    if (!ids.includes(usbId)) ids.push(usbId);
  }

  return ids;
}

/** True when any of the controller's device IDs appears in the HidHide blacklist. */
export function isPadCloaked(path: string, hiddenDevices: string[]): boolean {
  if (hiddenDevices.length === 0) return false;
  const ids = extractAllDeviceInstanceIds(path);
  if (ids.length === 0) return false;
  const hidden = hiddenDevices.map((h) => h.toUpperCase());
  return ids.some((id) => hidden.includes(id));
}

const AUTO_CLOAK_KEY = "padflow-auto-cloak";

/** Auto-cloak-on-connect preference (persisted). Defaults to OFF. */
export function getAutoCloakPreference(): boolean {
  try {
    return localStorage.getItem(AUTO_CLOAK_KEY) === "1";
  } catch {
    return false;
  }
}

export function setAutoCloakPreference(enabled: boolean): void {
  try {
    localStorage.setItem(AUTO_CLOAK_KEY, enabled ? "1" : "0");
  } catch {
    /* storage unavailable — ignore */
  }
}

const CLOAK_ON_START_KEY = "padflow-cloak-on-start";

/** Cloak already-connected pads when PadFlow launches. Defaults to OFF. */
export function getCloakOnStartPreference(): boolean {
  try {
    return localStorage.getItem(CLOAK_ON_START_KEY) === "1";
  } catch {
    return false;
  }
}

export function setCloakOnStartPreference(enabled: boolean): void {
  try {
    localStorage.setItem(CLOAK_ON_START_KEY, enabled ? "1" : "0");
  } catch {
    /* storage unavailable — ignore */
  }
}
