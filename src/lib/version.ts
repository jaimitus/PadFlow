// ---------------------------------------------------------------------------
// PadFlow — single source of truth for the app version & repository metadata.
// Bump APP_VERSION when cutting a new release (keep in sync with
// package.json, src-tauri/Cargo.toml and src-tauri/tauri.conf.json).
// ---------------------------------------------------------------------------

export const APP_VERSION = "1.2.4";

export const REPO_OWNER = "jaimitus";
export const REPO_NAME = "PadFlow";
export const REPO_URL = `https://github.com/${REPO_OWNER}/${REPO_NAME}`;
export const RELEASES_URL = `${REPO_URL}/releases`;

/** Compares two semver strings (optionally v-prefixed). Returns <0, 0 or >0. */
export function compareSemver(a: string, b: string): number {
  const pa = a.replace(/^v/i, "").split(".").map((n) => parseInt(n, 10) || 0);
  const pb = b.replace(/^v/i, "").split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const na = pa[i] ?? 0;
    const nb = pb[i] ?? 0;
    if (na !== nb) return na - nb;
  }
  return 0;
}

export interface UpdateInfo {
  version: string;
  notes: string;
  url: string;
  pubDate?: string;
}
