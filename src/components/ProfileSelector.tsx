import { useState } from "react";
import {
  PRESETS,
  deleteUserProfile,
  loadUserProfiles,
  saveUserProfile,
} from "../lib/curves";
import type { PadProfilePreset, StickProfileConfig } from "../lib/types";
import { cn } from "../utils/cn";

interface Props {
  activeId: string | null;
  currentConfig: StickProfileConfig;
  onSelect: (preset: PadProfilePreset) => void;
  onReset: () => void;
  notify: (msg: string) => void;
}

export default function ProfileSelector({
  activeId,
  currentConfig,
  onSelect,
  onReset,
  notify,
}: Props) {
  const [userProfiles, setUserProfiles] = useState<PadProfilePreset[]>(() =>
    loadUserProfiles(),
  );
  const [isSaving, setIsSaving] = useState(false);
  const [profileName, setProfileName] = useState("");

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    if (!profileName.trim()) return;
    const updated = saveUserProfile(profileName, currentConfig);
    setUserProfiles(updated);
    const newPreset = updated[0];
    if (newPreset) {
      onSelect(newPreset);
    }
    setProfileName("");
    setIsSaving(false);
    notify(`Saved custom profile "${profileName.trim()}"`);
  };

  const handleDelete = (e: React.MouseEvent, id: string, name: string) => {
    e.stopPropagation();
    const updated = deleteUserProfile(id);
    setUserProfiles(updated);
    notify(`Deleted profile "${name}"`);
  };

  const handleExport = (e: React.MouseEvent, p: PadProfilePreset) => {
    e.stopPropagation();
    const json = JSON.stringify(p.config, null, 2);
    navigator.clipboard
      .writeText(json)
      .then(() => notify(`Exported "${p.name}" configuration to clipboard`))
      .catch(() => notify("Failed to copy to clipboard"));
  };

  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
          Profiles & Presets
        </h3>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsSaving(true)}
            className="flex items-center gap-1.5 rounded-md border border-cyan-400/30 bg-cyan-400/10 px-2.5 py-1 font-mono text-[10px] text-cyan-300 transition-colors hover:bg-cyan-400/20"
          >
            <svg viewBox="0 0 24 24" className="h-3 w-3 fill-none stroke-current stroke-2">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
              <polyline points="17 21 17 13 7 13 7 21" />
              <polyline points="7 3 7 8 15 8" />
            </svg>
            SAVE CURRENT
          </button>
          <button
            onClick={onReset}
            className="rounded-md border border-white/8 bg-white/5 px-2.5 py-1 font-mono text-[10px] text-slate-400 transition-colors hover:text-slate-100"
          >
            RESET
          </button>
        </div>
      </div>

      {isSaving && (
        <form onSubmit={handleSave} className="mb-3.5 rounded-xl border border-cyan-400/30 bg-slate-900/90 p-3 backdrop-blur">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-cyan-200">Save Custom Profile</span>
            <button
              type="button"
              onClick={() => setIsSaving(false)}
              className="text-slate-400 hover:text-white"
            >
              ✕
            </button>
          </div>
          <div className="mt-2 flex gap-2">
            <input
              type="text"
              placeholder="Profile name (e.g. Apex Precision, Halo Smooth)..."
              value={profileName}
              onChange={(e) => setProfileName(e.target.value)}
              autoFocus
              className="flex-1 rounded-lg border border-white/10 bg-slate-950 px-3 py-1.5 text-xs text-white placeholder-slate-500 focus:border-cyan-400 focus:outline-none"
            />
            <button
              type="submit"
              disabled={!profileName.trim()}
              className="rounded-lg bg-gradient-to-r from-cyan-400 to-violet-500 px-3.5 py-1.5 text-xs font-semibold text-slate-950 transition-all hover:brightness-110 disabled:opacity-50"
            >
              Save
            </button>
          </div>
        </form>
      )}

      {/* Built-in Presets */}
      <div className="grid gap-2 sm:grid-cols-3">
        {PRESETS.map((p) => {
          const active = activeId === p.id;
          return (
            <button
              key={p.id}
              onClick={() => onSelect(p)}
              className={cn(
                "group relative overflow-hidden rounded-xl border p-3 text-left transition-all",
                active
                  ? "border-white/25 bg-white/[0.07]"
                  : "border-white/8 bg-white/[0.02] hover:border-white/16 hover:bg-white/[0.05]",
              )}
            >
              <div
                className={cn(
                  "absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r opacity-70 transition-opacity",
                  p.accent,
                  active ? "opacity-100" : "opacity-30 group-hover:opacity-70",
                )}
              />
              <div className="flex items-center justify-between">
                <span className="text-[13px] font-semibold text-slate-100">{p.name}</span>
                {active && (
                  <span className="rounded-full bg-emerald-400/15 px-1.5 py-0.5 font-mono text-[9px] uppercase text-emerald-300">
                    live
                  </span>
                )}
              </div>
              <p className="mt-1 font-mono text-[10px] leading-relaxed text-slate-500">
                {p.tagline}
              </p>
              <div className="mt-2 flex gap-1 font-mono text-[9px] text-slate-500">
                <span className="rounded bg-white/5 px-1.5 py-0.5">
                  DZ {(p.config.right.innerDeadzone * 100).toFixed(0)}%
                </span>
                <span className="rounded bg-white/5 px-1.5 py-0.5">
                  ADZ {(p.config.right.antiDeadzone * 100).toFixed(0)}%
                </span>
                <span className="rounded bg-white/5 px-1.5 py-0.5">
                  {p.config.turboPolling ? "1 kHz" : "500 Hz"}
                </span>
              </div>
            </button>
          );
        })}
      </div>

      {/* User Custom Profiles */}
      {userProfiles.length > 0 && (
        <div className="mt-3.5 border-t border-white/8 pt-3">
          <div className="mb-2 flex items-center justify-between">
            <span className="font-mono text-[10px] font-semibold uppercase tracking-wider text-slate-400">
              User Saved Profiles ({userProfiles.length})
            </span>
          </div>
          <div className="grid gap-2 sm:grid-cols-2">
            {userProfiles.map((p) => {
              const active = activeId === p.id;
              return (
                <div
                  key={p.id}
                  onClick={() => onSelect(p)}
                  className={cn(
                    "group relative cursor-pointer overflow-hidden rounded-xl border p-2.5 text-left transition-all",
                    active
                      ? "border-emerald-400/40 bg-emerald-400/[0.08]"
                      : "border-white/8 bg-white/[0.02] hover:border-white/16 hover:bg-white/[0.04]",
                  )}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-semibold text-slate-100">{p.name}</span>
                      {active && (
                        <span className="rounded-full bg-emerald-400/20 px-1.5 py-0.2 font-mono text-[8.5px] uppercase text-emerald-300">
                          active
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-1.5 opacity-80 group-hover:opacity-100">
                      <button
                        title="Copy config JSON to clipboard"
                        onClick={(e) => handleExport(e, p)}
                        className="rounded p-1 text-slate-400 hover:bg-white/10 hover:text-cyan-300"
                      >
                        <svg viewBox="0 0 24 24" className="h-3.5 w-3.5 fill-none stroke-current stroke-2">
                          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                        </svg>
                      </button>
                      <button
                        title="Delete profile"
                        onClick={(e) => handleDelete(e, p.id, p.name)}
                        className="rounded p-1 text-slate-400 hover:bg-rose-500/20 hover:text-rose-300"
                      >
                        <svg viewBox="0 0 24 24" className="h-3.5 w-3.5 fill-none stroke-current stroke-2">
                          <polyline points="3 6 5 6 21 6" />
                          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                        </svg>
                      </button>
                    </div>
                  </div>
                  <p className="mt-1 font-mono text-[9.5px] text-slate-500">{p.tagline}</p>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
