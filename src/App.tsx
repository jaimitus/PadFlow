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
import {
  extractAllDeviceInstanceIds,
  getAutoCloakPreference,
  getCloakOnStartPreference,
  isPadCloaked,
  setAutoCloakPreference,
  setCloakOnStartPreference,
} from "./lib/hidhide";
import {
  installUpdate,
  relaunchApp,
  checkForUpdates,
  type UpdateCheckState,
} from "./lib/updater";
import { APP_VERSION, type UpdateInfo } from "./lib/version";
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

export default function App() {
  const [devices, setDevices] = useState<GamepadInfo[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [deviceConfigs, setDeviceConfigs] = useState<
    Record<string, { profile: StickProfileConfig; presetId: string | null; name: string }>
  >({});
  const [profile, setProfile] = useState<StickProfileConfig>(cloneProfile(DEFAULT_PROFILE));
  const [presetId, setPresetId] = useState<string | null>(null);
  const [stats, setStats] = useState<EngineStats | null>({
    running: true,
    virtualPadOnline: true,
    polls: 0,
    pollHz: 1000,
    avgLatencyUs: 280,
    peakLatencyUs: 450,
    droppedReports: 0,
    reconnects: 0,
    driver: "ViGEmBus / Xbox 360 Controller",
  });
  const [hidhideStatus, setHidhideStatus] = useState<HidHideStatus | null>(null);
  const [tab, setTab] = useState<"studio" | "source">("studio");
  const [toast, setToast] = useState<string | null>(null);
  const [battery, setBattery] = useState({ level: -1, charging: false });
  const [running, setRunning] = useState(true);
  const [vigemInstalled, setVigemInstalled] = useState(true);
  const [shieldBusy, setShieldBusy] = useState(false);
  const [autoCloak, setAutoCloak] = useState<boolean>(getAutoCloakPreference);
  const [cloakOnStart, setCloakOnStart] = useState<boolean>(getCloakOnStartPreference);

  // ---- update checker state -------------------------------------------------
  const [updateState, setUpdateState] = useState<UpdateCheckState>("idle");
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateModalOpen, setUpdateModalOpen] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installed, setInstalled] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<{
    downloaded: number;
    total: number;
  } | null>(null);
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(() =>
    localStorage.getItem("padflow-dismissed-update"),
  );

  const snapRef = useRef<InputSnapshot>(EMPTY_SNAPSHOT);
  const native = padflow.mode() === "native";

  const notify = useCallback((msg: string) => {
    setToast(msg);
    window.setTimeout(() => setToast(null), 2600);
  }, []);

  // ---- update checking ------------------------------------------------------
  const checkingRef = useRef(false);
  const installingRef = useRef(false);
  installingRef.current = installing;

  const runUpdateCheck = useCallback(
    async (respectDismissed = false) => {
      if (checkingRef.current || installingRef.current) return;
      checkingRef.current = true;
      setUpdateState("checking");
      try {
        const res = await checkForUpdates();
        if (res.state === "available" && res.info) {
          setUpdateInfo(res.info);
          setUpdateState("available");
          setInstalled(false);
          if (!respectDismissed || res.info.version !== dismissedVersion) {
            setUpdateModalOpen(true);
          }
        } else if (res.state === "up-to-date") {
          setUpdateState("up-to-date");
          if (!respectDismissed) notify("You're running the latest version 🎉");
        } else {
          setUpdateState("error");
          if (!respectDismissed) {
            notify(`Update check failed: ${res.error ?? "unknown error"}`);
          }
        }
      } finally {
        checkingRef.current = false;
      }
    },
    [dismissedVersion, notify],
  );

  // Silent background check a few seconds after launch — pops the modal if a
  // new release has been published on GitHub. Runs exactly once per session.
  const autoCheckRan = useRef(false);
  useEffect(() => {
    if (autoCheckRan.current) return;
    autoCheckRan.current = true;
    const t = window.setTimeout(() => runUpdateCheck(true), 4000);
    return () => window.clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [runUpdateCheck]);

  const handleInstall = useCallback(async () => {
    if (!updateInfo || installing) return;
    setInstalling(true);
    setUpdateProgress(null);
    try {
      await installUpdate((downloaded, total) =>
        setUpdateProgress({ downloaded, total }),
      );
      setInstalled(true);
      notify(`v${updateInfo.version} installed — restart to finish`);
    } catch (e) {
      notify(`Install failed: ${String(e)}`);
    } finally {
      setInstalling(false);
    }
  }, [installing, notify, updateInfo]);

  const handleRestart = useCallback(async () => {
    try {
      await relaunchApp();
    } catch (e) {
      notify(`Could not restart: ${String(e)}`);
    }
  }, [notify]);

  const handleDismissUpdate = useCallback(() => {
    setUpdateModalOpen(false);
    if (updateInfo) {
      localStorage.setItem("padflow-dismissed-update", updateInfo.version);
      setDismissedVersion(updateInfo.version);
      notify("Update reminder hidden — you can check again anytime");
    }
  }, [notify, updateInfo]);

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

  const selectedIdRef = useRef<string | null>(null);
  selectedIdRef.current = selectedId;

  // ---- HidHide shield handlers ---------------------------------------------
  const toggleShieldActive = useCallback(async () => {
    if (!hidhideStatus || shieldBusy) return;
    setShieldBusy(true);
    try {
      const st = await padflow.setHidHideActive(!hidhideStatus.active);
      setHidhideStatus(st);
      notify(
        st.active
          ? "Shield ACTIVE — physical pads hidden from games 🛡️"
          : "Shield OFF — physical pads visible everywhere",
      );
    } catch (e) {
      notify(String(e));
    } finally {
      setShieldBusy(false);
    }
  }, [hidhideStatus, notify, shieldBusy]);

  const cloakAllControllers = useCallback(async () => {
    if (shieldBusy) return;
    setShieldBusy(true);
    try {
      const st = await padflow.autoCloakControllers();
      setHidhideStatus(st);
      const n = st.hiddenDevices.length;
      notify(
        n > 0
          ? `Cloaked ${n} device entr${n === 1 ? "y" : "ies"} — only the virtual pad reaches games 🛡️`
          : "No PlayStation controllers to cloak",
      );
    } catch (e) {
      notify(`Cloak failed: ${String(e)}`);
    } finally {
      setShieldBusy(false);
    }
  }, [notify, shieldBusy]);

  const uncloakAllControllers = useCallback(async () => {
    if (shieldBusy) return;
    setShieldBusy(true);
    try {
      const st = await padflow.uncloakAllControllers();
      setHidhideStatus(st);
      notify("All controllers uncloaked — physical pads visible to every app");
    } catch (e) {
      notify(`Uncloak failed: ${String(e)}`);
    } finally {
      setShieldBusy(false);
    }
  }, [notify, shieldBusy]);

  const togglePadCloak = useCallback(
    async (pad: GamepadInfo) => {
      if (shieldBusy) return;
      const cloaked = isPadCloaked(pad.path, hidhideStatus?.hiddenDevices ?? []);
      setShieldBusy(true);
      try {
        const st = await padflow.toggleDeviceHide(pad.path, !cloaked);
        setHidhideStatus(st);
        notify(
          cloaked
            ? `${pad.name} uncloaked — visible again`
            : `${pad.name} cloaked — hidden from games 🛡️`,
        );
      } catch (e) {
        notify(`Shield toggle failed: ${String(e)}`);
      } finally {
        setShieldBusy(false);
      }
    },
    [hidhideStatus, notify, shieldBusy],
  );

  const toggleAutoCloak = useCallback(() => {
    const next = !autoCloak;
    setAutoCloak(next);
    setAutoCloakPreference(next);
    notify(next ? "Auto-cloak ON — new controllers hide automatically 🛡️" : "Auto-cloak OFF");
  }, [autoCloak, notify]);

  const toggleCloakOnStart = useCallback(() => {
    const next = !cloakOnStart;
    setCloakOnStart(next);
    setCloakOnStartPreference(next);
    notify(next ? "Cloak on startup ON — pads already connected get hidden at launch 🛡️" : "Cloak on startup OFF");
  }, [cloakOnStart, notify]);

  // Cloak already-connected PlayStation pads once at launch when enabled.
  const cloakStartDone = useRef(false);
  useEffect(() => {
    if (!native || cloakStartDone.current) return;
    if (!cloakOnStart || !hidhideStatus?.installed) return;
    cloakStartDone.current = true;
    padflow
      .autoCloakControllers()
      .then((st) => {
        setHidhideStatus(st);
        const n = st.hiddenDevices.length;
        if (n > 0) {
          notify(`🛡️ Cloaked ${n} device entr${n === 1 ? "y" : "ies"} on startup`);
        }
      })
      .catch(() => undefined);
  }, [cloakOnStart, hidhideStatus, native, notify]);

  // Auto-cloak controllers that are plugged in AFTER launch (never touches the
  // pads that were already connected when PadFlow started).
  const autoCloakHandled = useRef<Set<string>>(new Set());
  const bootedRef = useRef(false);
  useEffect(() => {
    if (!autoCloak || !native || !hidhideStatus?.installed) return;
    if (!bootedRef.current && devices.length > 0) {
      bootedRef.current = true;
      devices.forEach((p) => autoCloakHandled.current.add(p.id));
      return;
    }
    for (const pad of devices) {
      if (pad.kind === "xInput" || pad.kind === "generic") continue;
      if (autoCloakHandled.current.has(pad.id)) continue;
      autoCloakHandled.current.add(pad.id);
      if (!isPadCloaked(pad.path, hidhideStatus.hiddenDevices)) {
        padflow
          .toggleDeviceHide(pad.path, true)
          .then((st) => {
            setHidhideStatus(st);
            notify(`🛡️ Auto-cloaked ${pad.name}`);
          })
          .catch(() => undefined);
      }
    }
  }, [autoCloak, devices, hidhideStatus, native, notify]);

  const hiddenSet = useMemo(() => {
    const s = new Set<string>();
    for (const h of hidhideStatus?.hiddenDevices ?? []) s.add(h.toUpperCase());
    return s;
  }, [hidhideStatus]);

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
              setStats(st.stats);
              setRunning(st.stats.running);
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
            const s = await padflow.getLastSnapshot(selectedIdRef.current ?? undefined);
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
  }, [native, notify]);

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

        if (native) {
          const st = await padflow.getEngineStatus();
          setStats(st.stats);
          setRunning(st.stats.running);
          setVigemInstalled(st.vigemInstalled);
          if (st.hidhideStatus) setHidhideStatus(st.hidhideStatus);
        }
      } catch {
        /* pad vanished mid-scan — retried next tick */
      }
    }, 3000);
    return () => window.clearInterval(id);
  }, [native]);

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
                PadFlow<span className="ml-1.5 font-mono text-[10px] font-normal text-cyan-300">v{APP_VERSION}</span>
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
              value={hidhideStatus?.installed ? "DRIVER READY 🛡️" : "NOT INSTALLED"}
              tone={hidhideStatus?.installed ? "good" : "warn"}
            />
            <Chip label="MODE" value={native ? "NATIVE HID" : "WEB PREVIEW"} tone={native ? "good" : "warn"} />

            <button
              onClick={() => runUpdateCheck(false)}
              disabled={updateState === "checking" || installing}
              title="Check GitHub for a new PadFlow release"
              className={cn(
                "flex items-center gap-1.5 rounded-lg border px-3 py-2 font-mono text-[10px] font-semibold uppercase tracking-wider transition-all cursor-pointer",
                updateState === "available"
                  ? "border-cyan-400/40 bg-cyan-400/10 text-cyan-200 hover:bg-cyan-400/20 hover:shadow-md hover:shadow-cyan-400/10"
                  : "border-white/10 bg-white/5 text-slate-400 hover:border-white/25 hover:text-white",
                (updateState === "checking" || installing) && "opacity-60 cursor-wait",
              )}
            >
              <span
                className={cn(
                  "text-[11px] leading-none",
                  updateState === "checking" && "animate-spin",
                )}
              >
                {updateState === "checking" ? "◌" : "⬆"}
              </span>
              {updateState === "checking"
                ? "Checking…"
                : updateState === "available"
                  ? `Update v${updateInfo?.version} ready`
                  : "Check update"}
              {updateState === "available" && (
                <span className="h-1.5 w-1.5 rounded-full bg-cyan-300 pf-live-dot" />
              )}
            </button>

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
                      activeProfileName={
                        deviceConfigs[pad.id]?.name ??
                        (pad.id === selectedId
                          ? presetId
                            ? "Preset Active"
                            : "Default"
                          : undefined)
                      }
                      cloaked={extractAllDeviceInstanceIds(pad.path).some((id) =>
                        hiddenSet.has(id),
                      )}
                      shieldAvailable={!!hidhideStatus?.installed}
                      onSelect={() => selectController(pad.id)}
                      onLed={(rgb) => setLed(pad.id, rgb)}
                      onRumble={() => {
                        padflow.testRumble(0.6, 0.9).catch(() => undefined);
                        notify("Haptic pulse sent (450 ms)");
                      }}
                      onToggleCloak={() => togglePadCloak(pad)}
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

              {/* HidHide Shield Control Center */}
              <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
                <SectionTitle
                  title="Anti-Double Input Shield"
                  right={
                    hidhideStatus?.installed
                      ? hidhideStatus.active
                        ? "ACTIVE 🛡️"
                        : "PAUSED"
                      : "NOT INSTALLED"
                  }
                />
                <div className="space-y-3">
                  <p className="font-mono text-[9.5px] leading-relaxed text-slate-400">
                    Hides physical PlayStation controllers from games so only the virtual Xbox 360 pad is detected — no more double-input.
                  </p>

                  {!hidhideStatus?.installed ? (
                    <div className="flex flex-col items-start gap-2 border-t border-white/6 pt-2.5">
                      <div className="flex items-center gap-1.5 font-mono text-[10px] text-slate-400">
                        <span className="h-1.5 w-1.5 rounded-full bg-rose-400" />
                        <span>Driver not detected</span>
                      </div>
                      <button
                        onClick={async () => {
                          notify("Launching HidHide driver installer...");
                          try {
                            const msg = await padflow.installHidHideDriver();
                            notify(msg);
                            const st = await padflow.getHidHideStatus();
                            setHidhideStatus(st);
                          } catch (e) {
                            notify(`Install error: ${String(e)}`);
                          }
                        }}
                        className="rounded-lg bg-emerald-400 px-3 py-1.5 font-mono text-[10px] font-bold text-slate-950 shadow-md shadow-emerald-400/20 hover:brightness-110 transition-all cursor-pointer"
                      >
                        🛡️ INSTALL HIDHIDE DRIVER
                      </button>
                    </div>
                  ) : (
                    <>
                      {/* global shield switch */}
                      <div className="flex items-center justify-between rounded-xl border border-white/6 bg-white/[0.02] px-3 py-2.5">
                        <div>
                          <p className="text-[11px] text-slate-200">Global shield</p>
                          <p className="font-mono text-[9.5px] text-slate-500">
                            {hidhideStatus.active
                              ? "firewall intercepting HID reports"
                              : "firewall paused — controllers visible"}
                          </p>
                        </div>
                        <button
                          onClick={toggleShieldActive}
                          disabled={shieldBusy}
                          aria-label="Toggle global HidHide shield"
                          className={cn(
                            "relative h-5 w-9 rounded-full transition-colors",
                            hidhideStatus.active ? "bg-emerald-400" : "bg-white/12",
                            shieldBusy && "opacity-60",
                          )}
                        >
                          <span
                            className={cn(
                              "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                              hidhideStatus.active ? "left-[18px]" : "left-0.5",
                            )}
                          />
                        </button>
                      </div>

                      {/* cloak all / uncloak all */}
                      <div className="grid grid-cols-2 gap-2">
                        <button
                          onClick={cloakAllControllers}
                          disabled={shieldBusy}
                          className="flex items-center justify-center gap-1.5 rounded-lg bg-gradient-to-r from-cyan-400 to-violet-500 px-3 py-2 font-mono text-[10px] font-bold text-slate-950 shadow-md shadow-cyan-400/20 transition-all hover:brightness-110 cursor-pointer disabled:opacity-50"
                        >
                          🛡️ CLOAK ALL
                        </button>
                        <button
                          onClick={uncloakAllControllers}
                          disabled={shieldBusy}
                          className="flex items-center justify-center gap-1.5 rounded-lg border border-white/10 bg-white/5 px-3 py-2 font-mono text-[10px] font-bold text-slate-300 transition-all hover:border-rose-400/40 hover:text-rose-300 cursor-pointer disabled:opacity-50"
                        >
                          ◉ UNCLOAK ALL
                        </button>
                      </div>

                      <div className="flex items-center gap-1.5 font-mono text-[10px] text-slate-400">
                        <span
                          className={cn(
                            "h-1.5 w-1.5 rounded-full",
                            hidhideStatus.whitelisted ? "bg-emerald-400" : "bg-amber-400",
                          )}
                        />
                        <span>
                          Whitelist:{" "}
                          <strong className={hidhideStatus.whitelisted ? "text-emerald-300" : "text-amber-300"}>
                            {hidhideStatus.whitelisted ? "PadFlow Authorized" : "NOT WHITELISTED"}
                          </strong>
                        </span>
                      </div>

                      {/* hidden devices list */}
                      <details className="group">
                        <summary className="flex cursor-pointer select-none items-center justify-between font-mono text-[10px] text-slate-400 transition-colors hover:text-slate-200">
                          <span>Hidden devices ({hidhideStatus.hiddenDevices.length})</span>
                          <span className="text-slate-600 group-open:hidden">▸</span>
                          <span className="hidden text-slate-600 group-open:inline">▾</span>
                        </summary>
                        <div className="mt-2 max-h-28 space-y-0.5 overflow-y-auto rounded-lg border border-white/8 bg-black/30 p-2 font-mono text-[9px] leading-relaxed text-slate-500">
                          {hidhideStatus.hiddenDevices.length === 0 ? (
                            <p className="italic text-slate-600">
                              Nothing hidden — games see your physical controllers directly.
                            </p>
                          ) : (
                            hidhideStatus.hiddenDevices.map((d) => (
                              <div key={d} className="truncate" title={d}>
                                {d}
                              </div>
                            ))
                          )}
                        </div>
                      </details>

                      {/* auto-cloak on connect */}
                      <div className="flex items-center justify-between border-t border-white/6 pt-2.5">
                        <div>
                          <p className="text-[11px] text-slate-300">Auto-cloak on connect</p>
                          <p className="font-mono text-[9.5px] text-slate-600">
                            hide new PlayStation pads as they plug in
                          </p>
                        </div>
                        <button
                          onClick={toggleAutoCloak}
                          aria-label="Toggle auto-cloak on connect"
                          className={cn(
                            "relative h-5 w-9 rounded-full transition-colors",
                            autoCloak ? "bg-cyan-400" : "bg-white/12",
                          )}
                        >
                          <span
                            className={cn(
                              "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                              autoCloak ? "left-[18px]" : "left-0.5",
                            )}
                          />
                        </button>
                      </div>

                      {/* cloak on startup */}
                      <div className="flex items-center justify-between border-t border-white/6 pt-2.5">
                        <div>
                          <p className="text-[11px] text-slate-300">Cloak on startup</p>
                          <p className="font-mono text-[9.5px] text-slate-600">
                            hide already-connected pads when PadFlow launches
                          </p>
                        </div>
                        <button
                          onClick={toggleCloakOnStart}
                          aria-label="Toggle cloak on startup"
                          className={cn(
                            "relative h-5 w-9 rounded-full transition-colors",
                            cloakOnStart ? "bg-violet-400" : "bg-white/12",
                          )}
                        >
                          <span
                            className={cn(
                              "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                              cloakOnStart ? "left-[18px]" : "left-0.5",
                            )}
                          />
                        </button>
                      </div>

                      <button
                        onClick={async () => {
                          try {
                            const msg = await padflow.launchHidHideGui();
                            notify(msg);
                          } catch (e) {
                            notify(String(e));
                          }
                        }}
                        className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 font-mono text-[10px] font-bold text-slate-300 transition-all hover:border-cyan-400/40 hover:text-cyan-200 cursor-pointer"
                      >
                        ⚙️ OPEN HIDHIDE CLIENT (OFFICIAL GUI)
                      </button>
                    </>
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
            PadFlow v{APP_VERSION} · open source ·{" "}
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

      {/* ---------- update available modal ---------- */}
      {updateModalOpen && updateInfo && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <div
            className="absolute inset-0 bg-black/70 backdrop-blur-sm"
            onClick={() => !installing && setUpdateModalOpen(false)}
          />
          <div className="relative w-full max-w-md overflow-hidden rounded-2xl border border-cyan-400/25 bg-[#0a0e18] shadow-[0_0_70px_-12px] shadow-cyan-400/25">
            <div className="pointer-events-none absolute -top-24 left-1/2 h-48 w-72 -translate-x-1/2 rounded-full bg-cyan-400/15 blur-[70px]" />
            <div className="relative p-5">
              <div className="flex items-start gap-3">
                <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-cyan-400 to-violet-500 text-lg text-slate-950 shadow-lg shadow-cyan-400/25">
                  ⬆
                </span>
                <div className="min-w-0">
                  <h3 className="text-sm font-bold uppercase tracking-[0.18em] text-white">
                    Update available
                  </h3>
                  <p className="mt-0.5 font-mono text-[11px] text-cyan-300">
                    PadFlow v{APP_VERSION} → v{updateInfo.version}
                  </p>
                </div>
              </div>

              {updateInfo.notes ? (
                <div className="mt-4 max-h-44 overflow-y-auto rounded-xl border border-white/8 bg-white/[0.02] p-3 text-[11px] leading-relaxed whitespace-pre-wrap text-slate-300">
                  {updateInfo.notes}
                </div>
              ) : (
                <p className="mt-4 text-[11px] text-slate-500">
                  A new PadFlow release is published on GitHub.
                </p>
              )}

              {installing && (
                <div className="mt-4">
                  <div className="mb-1 flex items-center justify-between font-mono text-[9.5px] text-slate-400">
                    <span>{installed ? "Applying update…" : "Downloading update…"}</span>
                    <span>
                      {updateProgress && updateProgress.total > 0
                        ? `${Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100))}% · ${(updateProgress.downloaded / 1048576).toFixed(1)}/${(updateProgress.total / 1048576).toFixed(1)} MB`
                        : "…"}
                    </span>
                  </div>
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/8">
                    <div
                      className="h-full rounded-full bg-gradient-to-r from-cyan-400 to-violet-500 transition-all duration-150"
                      style={{
                        width: `${
                          updateProgress && updateProgress.total > 0
                            ? Math.min(100, (updateProgress.downloaded / updateProgress.total) * 100)
                            : installing
                              ? 6
                              : 0
                        }%`,
                      }}
                    />
                  </div>
                </div>
              )}

              <div className="mt-5 flex flex-wrap items-center justify-end gap-2">
                {!installing && (
                  <>
                    {installed ? (
                      <button
                        onClick={handleRestart}
                        className="flex items-center gap-2 rounded-xl bg-gradient-to-r from-emerald-400 to-cyan-500 px-4 py-2 text-xs font-bold text-slate-950 shadow-md shadow-emerald-400/20 transition-all hover:brightness-110"
                      >
                        🔄 Restart PadFlow now
                      </button>
                    ) : (
                      <>
                        {native && (
                          <button
                            onClick={handleInstall}
                            className="flex items-center gap-2 rounded-xl bg-gradient-to-r from-cyan-400 to-violet-500 px-4 py-2 text-xs font-bold text-slate-950 shadow-md shadow-cyan-400/20 transition-all hover:brightness-110"
                          >
                            ⬇ Download &amp; install
                          </button>
                        )}
                        <button
                          onClick={() => padflow.openUrl(updateInfo.url).catch(() => undefined)}
                          className="rounded-xl border border-white/10 bg-white/5 px-3.5 py-2 text-xs font-semibold text-slate-300 transition-colors hover:border-white/25 hover:text-white"
                        >
                          View release
                        </button>
                      </>
                    )}
                    <button
                      onClick={handleDismissUpdate}
                      className="rounded-xl px-3.5 py-2 text-xs font-semibold text-slate-500 transition-colors hover:text-slate-300"
                    >
                      {installed ? "Later" : "Not now"}
                    </button>
                  </>
                )}
                {installing && !installed && (
                  <span className="font-mono text-[9.5px] text-slate-500">
                    Keep PadFlow open while the update installs…
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

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
