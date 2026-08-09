import { check, type Update } from "@tauri-apps/plugin-updater";
import { isNative } from "./engine";
import {
  APP_VERSION,
  REPO_OWNER,
  REPO_NAME,
  RELEASES_URL,
  compareSemver,
  type UpdateInfo,
} from "./version";

// ---------------------------------------------------------------------------
// Update detection.
//
// Native: Tauri updater plugin — queries the manifest published on GitHub
// (releases/latest/download/latest.json), verifies the signed bundle and
// installs it in place.
// Web: falls back to the GitHub Releases API so the browser preview also
// demonstrates the flow.
// ---------------------------------------------------------------------------

export type UpdateCheckState =
  | "idle"
  | "checking"
  | "available"
  | "up-to-date"
  | "error";

export interface UpdateCheckResult {
  state: UpdateCheckState;
  info?: UpdateInfo;
  error?: string;
}

export async function checkForUpdates(): Promise<UpdateCheckResult> {
  if (isNative()) {
    try {
      const update: Update | null = await check();
      if (!update) {
        return { state: "up-to-date" };
      }
      return {
        state: "available",
        info: {
          version: update.version,
          notes: update.body ?? "",
          url: `${RELEASES_URL}/tag/v${update.version}`,
          pubDate: update.date,
        },
      };
    } catch (e) {
      return { state: "error", error: String(e) };
    }
  }

  // ---- web fallback (browser preview) -------------------------------------
  try {
    const res = await fetch(
      `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest`,
    );
    if (!res.ok) return { state: "error", error: `GitHub API ${res.status}` };
    const data: {
      tag_name?: string;
      body?: string;
      html_url?: string;
      published_at?: string;
    } = await res.json();
    const version = (data.tag_name ?? "").replace(/^v/i, "");
    if (!version || compareSemver(version, APP_VERSION) <= 0) {
      return { state: "up-to-date" };
    }
    return {
      state: "available",
      info: {
        version,
        notes: data.body ?? "",
        url: data.html_url ?? `${RELEASES_URL}/tag/v${version}`,
        pubDate: data.published_at,
      },
    };
  } catch (e) {
    return { state: "error", error: String(e) };
  }
}

// ---------------------------------------------------------------------------
// Install flow (native only).
// ---------------------------------------------------------------------------

export type InstallPhase = "downloading" | "installing" | "done" | "failed";

export async function installUpdate(
  onProgress: (downloaded: number, total: number) => void,
): Promise<InstallPhase> {
  if (!isNative()) {
    throw new Error("In-app install is only available in desktop native mode");
  }
  const update = await check();
  if (!update) throw new Error("No update available");

  let downloaded = 0;
  let total = 0;
  let phase: InstallPhase = "installing";

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        total = event.data.contentLength ?? 0;
        phase = "downloading";
        break;
      case "Progress":
        downloaded += event.data.chunkLength ?? 0;
        onProgress(downloaded, total);
        break;
      case "Finished":
        phase = "installing";
        break;
    }
  });

  return phase;
}

/** Launches a fresh PadFlow instance and exits the current one. */
export async function relaunchApp(): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("relaunch_app");
}
