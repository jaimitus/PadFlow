import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import CircularityTester from "./components/CircularityTester";
import DeadzoneTuner from "./components/DeadzoneTuner";
import GamepadCard from "./components/GamepadCard";
import LiveTelemetry from "./components/LiveTelemetry";
import ProfileSelector from "./components/ProfileSelector";
import SourceExplorer from "./components/SourceExplorer";
import StickCurveCanvas from "./components/StickCurveCanvas";
import TriggerTuner from "./components/TriggerTuner";
import { DEFAULT_PROFILE, cloneProfile } from "./lib/curves";
import { padflow } from "./lib/engine";
import type {
  EngineStats,
  GamepadInfo,
  HidHideStatus,
  InputSnapshot,
  PadProfilePreset,
  StickAxisProfile,
  StickProfileConfig,
  TriggerProfile,
} from "./lib/types";
import { cn } from "./utils/cn";

const EMPTY_SNAPSHOT: InputSnapshot = {
  padId: "",
  rawLeft: [0, 0],
  rawRight: [0, 0],
  left: [0, 0],
  right: [0, 0],
  triggerLeft: 0,
  triggerRight: 0,
  buttons: 0,
  dpad: 8,
  touchPoints: [],
  gyro: [0, 0, 0],
  accel: [0, 0, 1],
  battery: -1,
  charging: false,
  latencyUs: 0,
  pollHz: 0,
  timestampMs: 0,
};

const ACCENT_L = "34,211,238";
const ACCENT_R = "168,85,247";

function normalizeDevicePath(path: string): string {
  return path.replace(/[\\?#]/g, "_").toUpperCase();
}

export default function App() {
  const [devices, setDevices] = useState<GamepadInfo[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [deviceConfigs, setDeviceConfigs] = useState<
    Record<string, { profile: StickProfileConfig; presetId: string | null; name: string }>
  >({});
  const [profile, setProfile] = useState<StickProfileConfig>(cloneProfile(DEFAULT_PROFILE));
  const [presetId, setPresetId] = useState<string | null>(null);
  const [stats, setStats] = useState<EngineStats | null>(null);
  const [hidhideStatus, setHidhideStatus] = useState<HidHideStatus | null>(null);
  const [tab, setTab] = useState<"studio" | "source">("studio");
  const [toast, setToast] = useState<string | null>(null);
  const [battery, setBattery] = useState({ level: -1, charging: false });
  const [running, setRunning] = useState(false);
  const [vigemInstalled, setVigemInstalled] = useState(true);

  const snapRef = useRef<InputSnapshot>(EMPTY_SNAPSHOT);
  const native = padflow.mode() === "native";

  const notify = useCallback((msg: string) => {
    setToast(msg);
    window.setTimeout(() => setToast(null), 2600);
  }, []);

  // Switch active profile when selected controller changes
  const selectController = useCallback(
    (padId: string) => {
      setSelectedId(padId);
      padflow.selectGamepad(padId).catch(() => undefined);
      setDeviceConfigs((prev) => {
        const existing = prev[padId];
        if (existing) {
          setProfile(cloneProfile(existing.profile));
          setPresetId(existing.presetId);
        } else {
          const fresh = cloneProfile(DEFAULT_PROFILE);
          setProfile(fresh);
          setPresetId(null);
          return {
            ...prev,
            [padId]: { profile: fresh, presetId: null, name: "Default" },
          };
        }
        return prev;
      });
    },
    [],
  );

  // ---- boot: enumerate + subscribe + start engine --------------------------
  useEffect(() => {
    let unInput: (() => void) | undefined;
    let unStats: (() => void) | undefined;
    let unHidHide: (() => void) | undefined;
    let pollTimer: ReturnType<typeof setInterval> | undefined;
    let alive = true;

    (async () => {
      try {
        const list = await padflow.getConnectedGamepads();
        if (!alive) return;
        setDevices(list);
        setSelectedId((cur) => cur ?? list[0]?.id ?? null);
      } catch (e) {
        notify(`Enumeration failed: ${String(e)}`);
      }

      // Event-driven snapshot subscription (primary path)
      unInput = await padflow.onInput((s) => {
        snapRef.current = s;
      });
      unStats = await padflow.onStats((s) => {
        setStats(s);
        setRunning(s.running);
      });
      unHidHide = await padflow.onHidHideUpdate((st) => {
        setHidhideStatus(st);
      });

      try {
        const s = await padflow.startEngine();
        if (!alive) return;
        setStats(s);
        setRunning(true);
        notify(
          native
            ? "ViGEmBus target allocated · realtime loop online"
            : "Browser preview engine online · plug a pad to drive it live",
        );
      } catch {
        // Engine may already be running from auto-start; that's OK.
        setRunning(true);
      }

      padflow
        .getHidHideStatus()
        .then((st) => {
          if (alive) setHidhideStatus(st);
        })
        .catch(() => undefined);

      if (native) {
        padflow
          .getEngineStatus()
          .then((st) => {
            if (alive) {
              setVigemInstalled(st.vigemInstalled);
              if (st.hidhideStatus) setHidhideStatus(st.hidhideStatus);
            }
          })
          .catch(() => undefined);
      }

      // Polling fallback: directly pull last snapshot via IPC at 60Hz.
      if (native) {
        pollTimer = setInterval(async () => {
          try {
            const s = await padflow.getLastSnapshot(selectedId ?? undefined);
            if (s && (s.padId || s.timestampMs > 0)) {
              snapRef.current = s;
            }
          } catch {
            // silently retry next tick
          }
        }, 16);
      }
    })();

    return () => {
      alive = false;
      unInput?.();
      unStats?.();
      unHidHide?.();
      if (pollTimer) clearInterval(pollTimer);
    };
  }, [native, notify, selectedId]);

  // ---- slow UI refreshes (battery / device list) ---------------------------
  useEffect(() => {
    const id = window.setInterval(() => {
      const s = snapRef.current;
      setBattery((b) =>
        b.level === s.battery && b.charging === s.charging
          ? b
          : { level: s.battery, charging: s.charging },
      );
    }, 1000);
    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    const id = window.setInterval(async () => {
      try {
        const list = await padflow.getConnectedGamepads();
        setDevices((prev) =>
          JSON.stringify(prev.map((p) => p.id)) === JSON.stringify(list.map((p) => p.id))
            ? prev.map((p) => ({ ...p, ...list.find((l) => l.id === p.id) }))
            : list,
        );
        setSelectedId((cur) => (cur && list.some((l) => l.id === cur) ? cur : list[0]?.id ?? null));
      } catch {
        /* pad vanished mid-scan — retried next tick */
      }
    }, 3000);
    return () => window.clearInterval(id);
  }, []);

  // ---- push profile to the engine (debounced) ------------------------------
  useEffect(() => {
    const id = window.setTimeout(() => {
      if (selectedId) {
        setDeviceConfigs((prev) => {
          const cur = prev[selectedId];
          return {
            ...prev,
            [selectedId]: {
              profile: cloneProfile(profile),
              presetId: cur?.presetId ?? presetId,
              name: cur?.name ?? "Custom Profile",
            },
          };
        });
      }
      padflow
        .updateStickProfile(profile, selectedId ?? undefined)
        .catch((e) => notify(`Profile rejected: ${String(e)}`));
    }, 40);
    return () => window.clearTimeout(id);
  }, [profile, selectedId, presetId, notify]);

  // ---- handlers ------------------------------------------------------------
  const patchAxis = useCallback(
    (side: "left" | "right", patch: Partial<StickAxisProfile>) => {
      setPresetId(null);
      setProfile((p) => {
        const next = { ...p, [side]: { ...p[side], ...patch } };
        if (selectedId) {
          setDeviceConfigs((prev) => ({
            ...prev,
            [selectedId]: { profile: next, presetId: null, name: "Custom Calibration" },
          }));
        }
        return next;
      });
    },
    [selectedId],
  );

  const patchTrigger = useCallback(
    (side: "left" | "right", patch: Partial<TriggerProfile>) => {
      setPresetId(null);
      setProfile((p) => {
        const next = {
          ...p,
          [side === "left" ? "triggerLeft" : "triggerRight"]: {
            ...p[side === "left" ? "triggerLeft" : "triggerRight"],
            ...patch,
          },
        };
        if (selectedId) {
          setDeviceConfigs((prev) => ({
            ...prev,
            [selectedId]: { profile: next, presetId: null, name: "Custom Calibration" },
          }));
        }
        return next;
      });
    },
    [selectedId],
  );

  const patchFlipTriggers = useCallback(
    (flip: boolean) => {
      setPresetId(null);
      setProfile((p) => {
        const next = { ...p, flipTriggers: flip };
        if (selectedId) {
          setDeviceConfigs((prev) => ({
            ...prev,
            [selectedId]: { profile: next, presetId: null, name: "Custom Calibration" },
          }));
        }
        return next;
      });
      notify(flip ? "Bumper & Trigger swap enabled (L1/R1 ↔ L2/R2)" : "Standard Bumper & Trigger mapping restored");
    },
    [selectedId, notify],
  );

  const applyPreset = useCallback(
    (preset: PadProfilePreset) => {
      const cloned = cloneProfile(preset.config);
      setProfile(cloned);
      setPresetId(preset.id);
      if (selectedId) {
        setDeviceConfigs((prev) => ({
          ...prev,
          [selectedId]: { profile: cloned, presetId: preset.id, name: preset.name },
        }));
      }
      notify(`${preset.name} profile applied to controller`);
    },
    [selectedId, notify],
  );

  const setLed = useCallback(
    async (padId: string, rgb: [number, number, number]) => {
      setDevices((d) => d.map((p) => (p.id === padId ? { ...p, led: rgb } : p)));
      try {
        await padflow.setLedColor(padId, rgb[0], rgb[1], rgb[2]);
      } catch (e) {
        notify(`LED report failed: ${String(e)}`);
      }
    },
    [notify],
  );

  const toggleEngine = useCallback(async () => {
    try {
      if (running) {
        const s = await padflow.stopEngine();
        setStats(s);
        setRunning(false);
        snapRef.current = EMPTY_SNAPSHOT;
        notify("Engine stopped · virtual pad neutralised");
      } else {
        const s = await padflow.startEngine();
        setStats(s);
        setRunning(true);
        notify("Engine running · feeding virtual Xbox 360 pad");
      }
    } catch (e) {
      notify(String(e));
    }
  }, [running, notify]);

  const isDeviceCloaked = useCallback(
    (devicePath: string) => {
      if (!hidhideStatus?.installed || !hidhideStatus.active || !hidhideStatus.hiddenDevices.length) {
        return false;
      }
      const target = normalizeDevicePath(devicePath);
      return hidhideStatus.hiddenDevices.some((d) => {
        const norm = normalizeDevicePath(d);
        return norm === target || norm.includes(target) || target.includes(norm) || d.toLowerCase() === devicePath.toLowerCase();
      });
    },
    [hidhideStatus],
  );

  const toggleDeviceCloak = useCallback(
    async (devicePath: string) => {
      const cloaked = isDeviceCloaked(devicePath);
      try {
        const st = await padflow.toggleDeviceHide(devicePath, !cloaked);
        setHidhideStatus(st);
        notify(
          !cloaked
            ? "🛡️ Device cloaked! Physical gamepad hidden from games (Anti-Double Input active)."
            : "Device uncloaked. Physical gamepad now visible to other apps.",
        );
      } catch (e) {
        notify(`HidHide error: ${String(e)}`);
      }
    },
    [isDeviceCloaked, notify],
  );

  const autoCloakAll = useCallback(async () => {
    try {
      const st = await padflow.autoCloakControllers();
      setHidhideStatus(st);
      notify("🛡️ All detected controllers cloaked! Anti-double-input protection active.");
    } catch (e) {
      notify(`Auto-cloak failed: ${String(e)}`);
    }
  }, [notify]);

  const toggleHidHideActive = useCallback(async () => {
    try {
      const currentActive = hidhideStatus?.active ?? false;
      const nextActive = !currentActive;
      const st = await padflow.setHidHideActive(nextActive);
      setHidhideStatus(st);
      notify(
        nextActive
          ? "🛡️ HidHide protection enabled"
          : "HidHide protection paused · physical gamepads uncloaked",
      );
    } catch (e) {
      notify(`HidHide toggle failed: ${String(e)}`);
    }
  }, [hidhideStatus, notify]);

  const getLeft = useCallback(
    () => ({ raw: snapRef.current.rawLeft, shaped: snapRef.current.left }),
    [],
  );
  const getRight = useCallback(
    () => ({ raw: snapRef.current.rawRight, shaped: snapRef.current.right }),
    [],
  );
  const getSnapshot = useCallback(() => snapRef.current, []);

  const selected = useMemo(
    () => devices.find((d) => d.id === selectedId) ?? devices[0] ?? null,
    [devices, selectedId],
  );

  const latencyMs = stats ? stats.avgLatencyUs / 1000 : 0;
  const isCloakingActive = hidhideStatus?.installed && hidhideStatus.active && hidhideStatus.hiddenDevices.length > 0;

  return (
    <div className="relative min-h-screen bg-[#05070d] text-slate-200">
      <div className="pointer-events-none absolute inset-0 pf-grid" />
      <div className="pointer-events-none absolute -top-40 left-1/2 h-[420px] w-[820px] -translate-x-1/2 rounded-full bg-cyan-500/10 blur-[120px]" />

      <div className="relative mx-auto max-w-[1500px] px-5 pb-16 pt-5">
        {/* ---------- header ---------- */}
        <header className="mb-5 flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.03] px-5 py-3.5 backdrop-blur">
          <div className="flex items-center gap-3">
            <img
              src="/logo.png"
              alt="PadFlow Logo"
              className="h-10 w-10 rounded-xl object-cover shadow-[0_0_28px_-4px] shadow-cyan-400/50 border border-white/10"
            />
            <div>
              <h1 className="text-lg font-bold leading-tight tracking-tight text-white">
                PadFlow<span className="ml-1.5 font-mono text-[10px] font-normal text-cyan-300">v1.1.0</span>
              </h1>
              <p className="font-mono text-[10px] text-slate-500">
                DualShock 4 / DualSense → XInput bridge · ViGEmBus · HidHide Shield · &lt;15 MB RAM
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Chip
              label="POLL"
              value={`${stats?.pollHz ?? 0} Hz`}
              tone={running ? "good" : "idle"}
            />
            <Chip
              label="LATENCY"
              value={`${latencyMs.toFixed(2)} ms`}
              tone={latencyMs > 0 && latencyMs < 1 ? "good" : "idle"}
            />
            <Chip
              label="VIRTUAL PAD"
              value={stats?.virtualPadOnline ? "X360 ONLINE" : "OFFLINE"}
              tone={stats?.virtualPadOnline ? "good" : "bad"}
            />
            <Chip
              label="HIDHIDE"
              value={
                isCloakingActive
                  ? "CLOAKED 🛡️"
                  : hidhideStatus?.installed
                    ? "SHIELD READY"
                    : "UNPROTECTED"
              }
              tone={isCloakingActive ? "good" : hidhideStatus?.installed ? "idle" : "warn"}
            />
            <Chip label="MODE" value={native ? "NATIVE HID" : "WEB PREVIEW"} tone={native ? "good" : "warn"} />

            <div className="ml-1 flex rounded-lg border border-white/8 bg-white/5 p-0.5">
              {(["studio", "source"] as const).map((t) => (
                <button
                  key={t}
                  onClick={() => setTab(t)}
                  className={cn(
                    "rounded-md px-3 py-1 font-mono text-[10px] uppercase tracking-wider transition-colors",
                    tab === t ? "bg-white/10 text-white" : "text-slate-500 hover:text-slate-300",
                  )}
                >
                  {t === "studio" ? "Studio" : "Rust core"}
                </button>
              ))}
            </div>

            <button
              onClick={toggleEngine}
              className={cn(
                "flex items-center gap-2 rounded-lg px-4 py-2 text-xs font-semibold transition-all",
                running
                  ? "bg-rose-500/15 text-rose-300 hover:bg-rose-500/25"
                  : "bg-gradient-to-r from-cyan-400 to-violet-500 text-slate-950 hover:brightness-110",
              )}
            >
              <span
                className={cn(
                  "h-1.5 w-1.5 rounded-full",
                  running ? "bg-rose-400 pf-live-dot" : "bg-slate-900",
                )}
              />
              {running ? "STOP ENGINE" : "START ENGINE"}
            </button>
          </div>
        </header>

        {/* ViGEmBus driver warning banner */}
        {native && !vigemInstalled && (
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-amber-400/30 bg-amber-500/10 px-4 py-3 text-amber-200 shadow-lg backdrop-blur">
            <div className="flex items-center gap-2.5">
              <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-amber-400/20 text-amber-300 font-bold">
                ⚠️
              </span>
              <div>
                <p className="text-xs font-semibold text-amber-100">
                  ViGEmBus Driver Required for Virtual Xbox 360 Pad
                </p>
                <p className="font-mono text-[10px] text-amber-300/80">
                  Input curves and live monitoring work, but games require ViGEmBus to receive mapped inputs.
                </p>
              </div>
            </div>
            <button
              onClick={async () => {
                notify("Launching ViGEmBus installer wizard (accept Administrator prompt)...");
                try {
                  const res = await padflow.installVigemDriver();
                  notify(res);
                  const st = await padflow.getEngineStatus();
                  setVigemInstalled(st.vigemInstalled);
                  setStats(st.stats);
                } catch (e) {
                  notify(`Install error: ${String(e)}`);
                }
              }}
              className="flex items-center gap-2 rounded-xl bg-amber-400 px-4 py-2 text-xs font-bold text-slate-950 transition-all hover:bg-amber-300 hover:shadow-md hover:shadow-amber-400/20"
            >
              🛠️ INSTALL VIGEMBUS DRIVER
            </button>
          </div>
        )}

        {/* HidHide driver installation recommendation banner */}
        {native && hidhideStatus && !hidhideStatus.installed && (
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-emerald-400/30 bg-emerald-500/10 px-4 py-3 text-emerald-200 shadow-lg backdrop-blur">
            <div className="flex items-center gap-2.5">
              <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-emerald-400/20 text-emerald-300 font-bold">
                🛡️
              </span>
              <div>
                <p className="text-xs font-semibold text-emerald-100">
                  HidHide Driver Recommended — Prevent Double-Input Conflicts
                </p>
                <p className="font-mono text-[10px] text-emerald-300/80">
                  Hides physical DirectInput controllers from games so only the virtual Xbox pad is detected.
                </p>
              </div>
            </div>
            <button
              onClick={async () => {
                notify("Launching HidHide installer wizard (accept Administrator prompt)...");
                try {
                  const res = await padflow.installHidHideDriver();
                  notify(res);
                  const st = await padflow.getHidHideStatus();
                  setHidhideStatus(st);
                } catch (e) {
                  notify(`Install error: ${String(e)}`);
                }
              }}
              className="flex items-center gap-2 rounded-xl bg-emerald-400 px-4 py-2 text-xs font-bold text-slate-950 transition-all hover:bg-emerald-300 hover:shadow-md hover:shadow-emerald-400/20"
            >
              🛡️ INSTALL HIDHIDE DRIVER
            </button>
          </div>
        )}

        {tab === "source" ? (
          <SourceExplorer />
        ) : (
          <div className="grid gap-4 xl:grid-cols-[360px_minmax(0,1fr)]">
            {/* ---------- left column ---------- */}
            <div className="space-y-4">
              <section>
                <SectionTitle
                  title="Controllers"
                  right={`${devices.length} detected`}
                />
                <div className="space-y-2.5">
                  {devices.map((pad) => (
                    <GamepadCard
                      key={pad.id}
                      pad={pad}
                      selected={pad.id === selected?.id}
                      liveBattery={pad.id === snapRef.current.padId ? battery.level : pad.battery}
                      charging={pad.id === snapRef.current.padId ? battery.charging : pad.charging}
                      isCloaked={isDeviceCloaked(pad.path)}
                      hidhideInstalled={hidhideStatus?.installed ?? false}
                      activeProfileName={
                        deviceConfigs[pad.id]?.name ??
                        (pad.id === selectedId
                          ? presetId
                            ? "Preset Active"
                            : "Default"
                          : undefined)
                      }
                      onSelect={() => selectController(pad.id)}
                      onLed={(rgb) => setLed(pad.id, rgb)}
                      onRumble={() => {
                        padflow.testRumble(0.6, 0.9).catch(() => undefined);
                        notify("Haptic pulse sent (450 ms)");
                      }}
                      onToggleCloak={() => toggleDeviceCloak(pad.path)}
                    />
                  ))}
                  {devices.length === 0 && (
                    <div className="rounded-2xl border border-dashed border-white/10 bg-white/[0.02] p-6 text-center">
                      <p className="text-sm text-slate-400">No controller detected</p>
                      <p className="mt-1 font-mono text-[10px] text-slate-600">
                        Connect a DualShock 4 / DualSense over USB or Bluetooth
                      </p>
                    </div>
                  )}
                </div>
              </section>

              {/* HidHide Shield Panel */}
              <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
                <SectionTitle
                  title="Anti-Double Input Shield"
                  right={hidhideStatus?.installed ? (hidhideStatus.active ? "ACTIVE" : "PAUSED") : "NOT INSTALLED"}
                />
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-[11px] text-slate-200">Cloak Device Firewall (HidHide)</p>
                      <p className="font-mono text-[9.5px] text-slate-500">
                        Hides physical Sony pad so games only read virtual XInput
                      </p>
                    </div>
                    <button
                      onClick={
                        hidhideStatus?.installed
                          ? toggleHidHideActive
                          : () => notify("HidHide driver is not installed. Click Install Driver above.")
                      }
                      className={cn(
                        "relative h-5 w-9 rounded-full transition-colors",
                        hidhideStatus?.installed && hidhideStatus?.active ? "bg-emerald-400" : "bg-white/12",
                      )}
                    >
                      <span
                        className={cn(
                          "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                          hidhideStatus?.installed && hidhideStatus?.active ? "left-[18px]" : "left-0.5",
                        )}
                      />
                    </button>
                  </div>

                  {hidhideStatus?.installed && (
                    <div className="flex items-center justify-between pt-1 text-xs">
                      <span className="font-mono text-[10px] text-slate-400">
                        Hidden instances: <span className="text-emerald-300 font-bold">{hidhideStatus.hiddenDevices.length}</span>
                      </span>
                      <button
                        onClick={autoCloakAll}
                        className="rounded-md border border-emerald-400/30 bg-emerald-400/10 px-2.5 py-1 font-mono text-[10px] text-emerald-300 hover:bg-emerald-400/20 transition-colors"
                      >
                        🛡️ CLOAK ALL CONTROLLERS
                      </button>
                    </div>
                  )}
                </div>
              </section>

              <ProfileSelector
                activeId={presetId}
                currentConfig={profile}
                onSelect={applyPreset}
                onReset={() => {
                  setProfile(cloneProfile(DEFAULT_PROFILE));
                  setPresetId(null);
                  notify("Profile reset to engine defaults");
                }}
                notify={notify}
              />

              <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
                <SectionTitle title="Engine & Extras" right={stats?.driver ?? "—"} />
                <div className="grid grid-cols-2 gap-2">
                  <Stat label="Reports" value={(stats?.polls ?? 0).toLocaleString()} />
                  <Stat label="Peak lat." value={`${((stats?.peakLatencyUs ?? 0) / 1000).toFixed(2)} ms`} />
                  <Stat label="Dropped" value={String(stats?.droppedReports ?? 0)} />
                  <Stat label="Reconnects" value={String(stats?.reconnects ?? 0)} />
                </div>

                <div className="mt-3 space-y-2.5 border-t border-white/8 pt-3">
                  <div>
                    <div className="mb-1 flex items-baseline justify-between">
                      <span className="text-[11px] text-slate-300">Rumble intensity</span>
                      <span className="font-mono text-[11px] text-slate-100">
                        {(profile.rumbleIntensity * 100).toFixed(0)}%
                      </span>
                    </div>
                    <input
                      type="range"
                      min={0}
                      max={1}
                      step={0.01}
                      value={profile.rumbleIntensity}
                      onChange={(e) =>
                        setProfile((p) => ({ ...p, rumbleIntensity: parseFloat(e.target.value) }))
                      }
                      className="pf-range h-1.5 w-full cursor-pointer appearance-none rounded-full"
                      style={{
                        background: `linear-gradient(90deg, rgb(250,204,21) 0%, rgb(250,204,21) ${
                          profile.rumbleIntensity * 100
                        }%, rgba(255,255,255,0.09) ${profile.rumbleIntensity * 100}%, rgba(255,255,255,0.09) 100%)`,
                        ["--pf-accent" as string]: "rgb(250,204,21)",
                      }}
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-[11px] text-slate-300">Turbo polling (1 kHz)</p>
                      <p className="font-mono text-[9.5px] text-slate-600">
                        pins HID loop to sub-millisecond thread priority
                      </p>
                    </div>
                    <button
                      onClick={() => setProfile((p) => ({ ...p, turboPolling: !p.turboPolling }))}
                      className={cn(
                        "relative h-5 w-9 rounded-full transition-colors",
                        profile.turboPolling ? "bg-cyan-400" : "bg-white/12",
                      )}
                    >
                      <span
                        className={cn(
                          "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                          profile.turboPolling ? "left-[18px]" : "left-0.5",
                        )}
                      />
                    </button>
                  </div>

                  <div className="flex items-center justify-between border-t border-white/6 pt-2">
                    <div>
                      <p className="text-[11px] text-slate-300">Touchpad as Virtual Mouse</p>
                      <p className="font-mono text-[9.5px] text-slate-600">
                        1-finger move/click · 2-finger scroll
                      </p>
                    </div>
                    <button
                      onClick={() => setProfile((p) => ({ ...p, touchpadMouse: !p.touchpadMouse }))}
                      className={cn(
                        "relative h-5 w-9 rounded-full transition-colors",
                        profile.touchpadMouse ? "bg-cyan-400" : "bg-white/12",
                      )}
                    >
                      <span
                        className={cn(
                          "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                          profile.touchpadMouse ? "left-[18px]" : "left-0.5",
                        )}
                      />
                    </button>
                  </div>

                  {profile.touchpadMouse && (
                    <div className="pl-1">
                      <div className="mb-1 flex items-baseline justify-between font-mono text-[9.5px]">
                        <span className="text-slate-400">Touchpad Sensitivity</span>
                        <span className="text-slate-200">{profile.touchpadSensitivity.toFixed(2)}×</span>
                      </div>
                      <input
                        type="range"
                        min={0.25}
                        max={3.0}
                        step={0.05}
                        value={profile.touchpadSensitivity}
                        onChange={(e) =>
                          setProfile((p) => ({
                            ...p,
                            touchpadSensitivity: parseFloat(e.target.value),
                          }))
                        }
                        className="pf-range h-1.5 w-full cursor-pointer appearance-none rounded-full"
                        style={{
                          background: `linear-gradient(90deg, rgb(34,211,238) 0%, rgb(34,211,238) ${
                            ((profile.touchpadSensitivity - 0.25) / 2.75) * 100
                          }%, rgba(255,255,255,0.09) ${
                            ((profile.touchpadSensitivity - 0.25) / 2.75) * 100
                          }%, rgba(255,255,255,0.09) 100%)`,
                          ["--pf-accent" as string]: "rgb(34,211,238)",
                        }}
                      />
                    </div>
                  )}

                  <div className="flex items-center justify-between border-t border-white/6 pt-2">
                    <div>
                      <p className="text-[11px] text-slate-300">Smart Battery Lightbar</p>
                      <p className="font-mono text-[9.5px] text-slate-600">
                        dynamic LED color (Green &gt; 60%, Amber, Red)
                      </p>
                    </div>
                    <button
                      onClick={() => setProfile((p) => ({ ...p, batteryLedMode: !p.batteryLedMode }))}
                      className={cn(
                        "relative h-5 w-9 rounded-full transition-colors",
                        profile.batteryLedMode ? "bg-emerald-400" : "bg-white/12",
                      )}
                    >
                      <span
                        className={cn(
                          "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                          profile.batteryLedMode ? "left-[18px]" : "left-0.5",
                        )}
                      />
                    </button>
                  </div>
                </div>
              </section>
            </div>

            {/* ---------- right column ---------- */}
            <div className="space-y-4">
              <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
                <SectionTitle
                  title="Stick response matrix"
                  right="input magnitude → virtual output"
                />
                <div className="grid gap-5 lg:grid-cols-2">
                  <StickCurveCanvas
                    label="Left stick · movement"
                    accent={ACCENT_L}
                    profile={profile.left}
                    onChange={(patch) => patchAxis("left", patch)}
                    getSample={getLeft}
                  />
                  <StickCurveCanvas
                    label="Right stick · aim"
                    accent={ACCENT_R}
                    profile={profile.right}
                    onChange={(patch) => patchAxis("right", patch)}
                    getSample={getRight}
                  />
                </div>
              </section>

              <div className="grid gap-4 lg:grid-cols-2">
                <DeadzoneTuner
                  title="Left stick tuner"
                  accent={ACCENT_L}
                  profile={profile.left}
                  onChange={(patch) => patchAxis("left", patch)}
                />
                <DeadzoneTuner
                  title="Right stick tuner"
                  accent={ACCENT_R}
                  profile={profile.right}
                  onChange={(patch) => patchAxis("right", patch)}
                />
              </div>

              <TriggerTuner
                triggerLeft={profile.triggerLeft}
                triggerRight={profile.triggerRight}
                flipTriggers={profile.flipTriggers}
                onLeftChange={(patch) => patchTrigger("left", patch)}
                onRightChange={(patch) => patchTrigger("right", patch)}
                onFlipChange={patchFlipTriggers}
                getSnapshot={getSnapshot}
              />

              <CircularityTester
                profileLeft={profile.left}
                profileRight={profile.right}
                onApplyDeadzone={(side, recDZ) => {
                  patchAxis(side, { innerDeadzone: recDZ });
                  notify(`Auto-calibrated ${side} stick deadzone to ${(recDZ * 100).toFixed(1)}%`);
                }}
                getSnapshot={getSnapshot}
              />

              <LiveTelemetry getSnapshot={getSnapshot} flipTriggers={profile.flipTriggers} />
            </div>
          </div>
        )}

        <footer className="mt-6 flex flex-wrap items-center justify-between gap-3 border-t border-white/8 pt-4 font-mono text-[10px] text-slate-600">
          <span>
            PadFlow v1.1.0 · open source ·{" "}
            <button
              type="button"
              onClick={() => padflow.openUrl("https://github.com/jaimitus/PadFlow")}
              className="text-slate-400 hover:text-cyan-300 underline transition-colors cursor-pointer"
            >
              github.com/jaimitus/PadFlow
            </button>
            {" "}· Windows 10 / 11 · ViGEmBus 1.22+ · HidHide Support
          </span>
          <span>
            active pad:{" "}
            <span className="text-slate-400">{selected?.name ?? "none"}</span>
          </span>
        </footer>
      </div>

      {toast && (
        <div className="fixed bottom-5 left-1/2 z-50 -translate-x-1/2 rounded-xl border border-cyan-400/30 bg-slate-950/95 px-4 py-2.5 font-mono text-[11px] text-cyan-100 shadow-2xl backdrop-blur">
          {toast}
        </div>
      )}
    </div>
  );
}

function SectionTitle({ title, right }: { title: string; right?: string }) {
  return (
    <div className="mb-2.5 flex items-baseline justify-between">
      <h2 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
        {title}
      </h2>
      {right && <span className="font-mono text-[10px] text-slate-600">{right}</span>}
    </div>
  );
}

function Chip({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "good" | "bad" | "warn" | "idle";
}) {
  const tones = {
    good: "border-emerald-400/25 bg-emerald-400/10 text-emerald-300",
    bad: "border-rose-400/25 bg-rose-400/10 text-rose-300",
    warn: "border-amber-400/25 bg-amber-400/10 text-amber-300",
    idle: "border-white/8 bg-white/5 text-slate-400",
  } as const;
  return (
    <div className={cn("rounded-lg border px-2.5 py-1", tones[tone])}>
      <span className="font-mono text-[9px] uppercase tracking-wider opacity-70">{label}</span>
      <span className="ml-1.5 font-mono text-[11px] font-semibold tabular-nums">{value}</span>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-white/8 bg-white/[0.03] px-2.5 py-1.5">
      <p className="font-mono text-[9px] uppercase tracking-wider text-slate-500">{label}</p>
      <p className="font-mono text-[13px] tabular-nums text-slate-100">{value}</p>
    </div>
  );
}
