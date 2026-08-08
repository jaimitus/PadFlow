import { DEFAULT_PROFILE, shapeStick, shapeTrigger, clamp01 } from "./curves";
import type {
  ConnectionType,
  EngineStats,
  EngineStatus,
  GamepadInfo,
  HidHideStatus,
  InputSnapshot,
  PadKind,
  StickProfileConfig,
} from "./types";
import { BUTTONS } from "./types";

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

type Unlisten = () => void;

export const isNative = (): boolean => {
  if (typeof window === "undefined") return false;
  return (
    "__TAURI__" in window ||
    "__TAURI_INTERNALS__" in window
  );
};

// ---------------------------------------------------------------------------
// Browser fallback engine — drives the exact same data contracts so the whole
// UI is testable (and demo-able) without ViGEmBus installed.
// ---------------------------------------------------------------------------

const emptySnapshot = (padId: string): InputSnapshot => ({
  padId,
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
});

const SIM_PAD: GamepadInfo = {
  id: "054c:0ce6:SIM-DEMO",
  name: "DualSense Wireless Controller",
  kind: "dualSense",
  connection: "usb",
  battery: 86,
  charging: true,
  led: [0, 140, 255],
  vendorId: 0x054c,
  productId: 0x0ce6,
  serial: "SIM-DEMO",
  path: "\\\\?\\HID#VID_054C&PID_0CE6#SIM",
  hasLightbar: true,
  hasGyro: true,
  hasTouchpad: true,
  reportRateHz: 1000,
};

function detectKind(id: string): { kind: PadKind; name: string } {
  const s = id.toLowerCase();
  if (s.includes("0ce6") || s.includes("dualsense") || s.includes("ps5"))
    return { kind: "dualSense", name: "DualSense Wireless Controller" };
  if (s.includes("0df2"))
    return { kind: "dualSenseEdge", name: "DualSense Edge Controller" };
  if (s.includes("09cc") || s.includes("05c4") || s.includes("dualshock"))
    return { kind: "dualShock4", name: "Wireless Controller (DualShock 4)" };
  if (s.includes("xbox") || s.includes("045e"))
    return { kind: "xInput", name: "Xbox Controller" };
  return { kind: "generic", name: id.slice(0, 42) || "HID Gamepad" };
}

class WebEngine {
  private profile: StickProfileConfig = DEFAULT_PROFILE;
  private inputCbs = new Set<(s: InputSnapshot) => void>();
  private statsCbs = new Set<(s: EngineStats) => void>();
  private hidhideCbs = new Set<(s: HidHideStatus) => void>();
  private raf = 0;
  private running = false;
  private polls = 0;
  private t0 = performance.now();
  private lastStats = 0;
  private battery = 86;
  private led: Record<string, [number, number, number]> = {};
  private rumbleUntil = 0;
  private peak = 0;
  private hidhideActive = true;
  private hiddenDevices: string[] = ["HID\\VID_054C&PID_0CE6&COL01\\7&3084128&0&0000"];

  devices(): GamepadInfo[] {
    const out: GamepadInfo[] = [];
    const pads =
      typeof navigator !== "undefined" && navigator.getGamepads
        ? navigator.getGamepads()
        : [];
    for (const p of pads) {
      if (!p) continue;
      const { kind, name } = detectKind(p.id);
      const id = `web:${p.index}:${p.id.slice(0, 24)}`;
      out.push({
        id,
        name,
        kind,
        connection: (p.id.toLowerCase().includes("bluetooth")
          ? "bluetooth"
          : "usb") as ConnectionType,
        battery: Math.round(this.battery),
        charging: false,
        led: this.led[id] ?? [0, 140, 255],
        vendorId: 0x054c,
        productId: kind === "dualShock4" ? 0x09cc : 0x0ce6,
        serial: String(p.index),
        path: `web:${p.index}`,
        hasLightbar: kind !== "xInput" && kind !== "generic",
        hasGyro: kind !== "xInput",
        hasTouchpad: kind !== "xInput" && kind !== "generic",
        reportRateHz: 1000,
      });
    }
    if (out.length === 0) {
      out.push({
        ...SIM_PAD,
        battery: Math.round(this.battery),
        led: this.led[SIM_PAD.id] ?? SIM_PAD.led,
      });
    }
    return out;
  }

  private profilesByPad: Record<string, StickProfileConfig> = {};

  setProfile(p: StickProfileConfig, padId?: string) {
    this.profile = p;
    if (padId) {
      this.profilesByPad[padId] = p;
    }
  }

  getProfile(padId?: string): StickProfileConfig {
    if (padId && this.profilesByPad[padId]) {
      return this.profilesByPad[padId];
    }
    return this.profile;
  }

  setLed(padId: string, rgb: [number, number, number]) {
    this.led[padId] = rgb;
  }

  rumble(ms = 450) {
    this.rumbleUntil = performance.now() + ms;
    const pads = navigator.getGamepads?.() ?? [];
    for (const p of pads) {
      const actuator = (
        p as Gamepad & {
          vibrationActuator?: {
            playEffect: (t: string, o: Record<string, number>) => Promise<unknown>;
          };
        }
      )?.vibrationActuator;
      actuator
        ?.playEffect("dual-rumble", {
          duration: ms,
          strongMagnitude: 0.8,
          weakMagnitude: 0.5,
        })
        .catch(() => undefined);
    }
  }

  stats(): EngineStats {
    const secs = Math.max((performance.now() - this.t0) / 1000, 0.001);
    const hz = this.running ? (this.profile.turboPolling ? 1000 : 500) : 0;
    return {
      running: this.running,
      virtualPadOnline: this.running,
      polls: this.polls,
      pollHz: hz,
      avgLatencyUs: this.running ? Math.round(320 + Math.sin(secs) * 45) : 0,
      peakLatencyUs: this.peak,
      droppedReports: 0,
      reconnects: 0,
      driver: isNative()
        ? "ViGEmBus / Xbox 360 Controller"
        : "Simulated target (browser preview)",
    };
  }

  hidhideStatus(): HidHideStatus {
    return {
      installed: true,
      active: this.hidhideActive,
      whitelisted: true,
      hiddenDevices: this.hiddenDevices,
      appPath: "C:\\Program Files\\PadFlow\\PadFlow.exe",
    };
  }

  setHidHideActive(active: boolean): HidHideStatus {
    this.hidhideActive = active;
    const st = this.hidhideStatus();
    this.hidhideCbs.forEach((cb) => cb(st));
    return st;
  }

  toggleDeviceHide(path: string, hide: boolean): HidHideStatus {
    const norm = path.replace(/[\\?#]/g, "_").toUpperCase();
    if (hide) {
      if (!this.hiddenDevices.includes(norm)) {
        this.hiddenDevices.push(norm);
      }
    } else {
      this.hiddenDevices = this.hiddenDevices.filter((p) => p !== norm && p !== path);
    }
    const st = this.hidhideStatus();
    this.hidhideCbs.forEach((cb) => cb(st));
    return st;
  }

  onInput(cb: (s: InputSnapshot) => void): Unlisten {
    this.inputCbs.add(cb);
    return () => this.inputCbs.delete(cb);
  }

  onStats(cb: (s: EngineStats) => void): Unlisten {
    this.statsCbs.add(cb);
    return () => this.statsCbs.delete(cb);
  }

  onHidHide(cb: (s: HidHideStatus) => void): Unlisten {
    this.hidhideCbs.add(cb);
    return () => this.hidhideCbs.delete(cb);
  }

  start() {
    if (this.running) return;
    this.running = true;
    this.t0 = performance.now();
    const loop = () => {
      if (!this.running) return;
      this.tick();
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
  }

  stop() {
    this.running = false;
    cancelAnimationFrame(this.raf);
    const s = this.stats();
    this.statsCbs.forEach((c) => c(s));
  }

  private tick() {
    const now = performance.now();
    const t = (now - this.t0) / 1000;
    this.polls += this.profile.turboPolling ? 16 : 8;

    const live = (navigator.getGamepads?.() ?? []).find(Boolean) as
      | Gamepad
      | undefined;

    const snap = emptySnapshot(live ? `web:${live.index}:${live.id.slice(0, 24)}` : SIM_PAD.id);

    if (live) {
      snap.rawLeft = [live.axes[0] ?? 0, -(live.axes[1] ?? 0)];
      snap.rawRight = [live.axes[2] ?? 0, -(live.axes[3] ?? 0)];
      snap.triggerLeft = clamp01(live.buttons[6]?.value ?? 0);
      snap.triggerRight = clamp01(live.buttons[7]?.value ?? 0);
      const b = live.buttons;
      let mask = 0;
      if (b[0]?.pressed) mask |= BUTTONS.CROSS;
      if (b[1]?.pressed) mask |= BUTTONS.CIRCLE;
      if (b[2]?.pressed) mask |= BUTTONS.SQUARE;
      if (b[3]?.pressed) mask |= BUTTONS.TRIANGLE;
      if (b[4]?.pressed) mask |= BUTTONS.L1;
      if (b[5]?.pressed) mask |= BUTTONS.R1;
      if (b[6]?.pressed) mask |= BUTTONS.L2;
      if (b[7]?.pressed) mask |= BUTTONS.R2;
      if (b[8]?.pressed) mask |= BUTTONS.SHARE;
      if (b[9]?.pressed) mask |= BUTTONS.OPTIONS;
      if (b[10]?.pressed) mask |= BUTTONS.L3;
      if (b[11]?.pressed) mask |= BUTTONS.R3;
      if (b[16]?.pressed) mask |= BUTTONS.PS;
      if (b[17]?.pressed) mask |= BUTTONS.TOUCHPAD;
      snap.buttons = mask;
      const up = b[12]?.pressed,
        down = b[13]?.pressed,
        left = b[14]?.pressed,
        right = b[15]?.pressed;
      snap.dpad = up && right ? 1 : right && down ? 3 : down && left ? 5 : left && up ? 7 : up ? 0 : right ? 2 : down ? 4 : left ? 6 : 8;
    } else {
      // Deterministic demo motion: a slow orbit + a flick pattern.
      const lx = Math.sin(t * 0.9) * 0.62 + Math.sin(t * 4.3) * 0.06;
      const ly = Math.cos(t * 0.72) * 0.55 + Math.cos(t * 5.1) * 0.05;
      const flick = Math.max(0, Math.sin(t * 0.55)) ** 3;
      const rx = Math.sin(t * 2.4) * (0.25 + flick * 0.7);
      const ry = Math.cos(t * 1.7) * (0.2 + flick * 0.55);
      snap.rawLeft = [clampAxis(lx), clampAxis(ly)];
      snap.rawRight = [clampAxis(rx), clampAxis(ry)];
      snap.triggerLeft = Math.max(0, Math.sin(t * 1.3)) ** 2;
      snap.triggerRight = Math.max(0, Math.sin(t * 1.9 + 1)) ** 2;
      let mask = 0;
      if (Math.sin(t * 3.1) > 0.85) mask |= BUTTONS.CROSS;
      if (Math.sin(t * 2.2 + 2) > 0.9) mask |= BUTTONS.R1;
      if (Math.sin(t * 1.4 + 4) > 0.93) mask |= BUTTONS.SQUARE;
      if (snap.triggerRight > 0.5) mask |= BUTTONS.R2;
      if (snap.triggerLeft > 0.5) mask |= BUTTONS.L2;
      snap.buttons = mask;
      snap.dpad = Math.sin(t * 0.6) > 0.92 ? 0 : 8;
      snap.touchPoints =
        Math.sin(t * 0.8) > 0.4
          ? [[0.5 + Math.sin(t * 1.6) * 0.28, 0.5 + Math.cos(t * 2.1) * 0.22]]
          : [];
    }

    snap.gyro = [Math.sin(t * 1.1) * 0.4, Math.cos(t * 0.9) * 0.3, Math.sin(t * 0.5) * 0.2];
    snap.accel = [Math.sin(t * 0.3) * 0.1, Math.cos(t * 0.4) * 0.1, 0.98];

    const [lx2, ly2] = shapeStick(snap.rawLeft[0], snap.rawLeft[1], this.profile.left);
    const [rx2, ry2] = shapeStick(snap.rawRight[0], snap.rawRight[1], this.profile.right);
    snap.left = [lx2, ly2];
    snap.right = [rx2, ry2];

    let lt = shapeTrigger(snap.triggerLeft, this.profile.triggerLeft);
    let rt = shapeTrigger(snap.triggerRight, this.profile.triggerRight);

    if (this.profile.flipTriggers) {
      const l1_pressed = (snap.buttons & BUTTONS.L1) !== 0;
      const r1_pressed = (snap.buttons & BUTTONS.R1) !== 0;

      const l2_bumper = lt > 0.3 ? BUTTONS.L1 : 0;
      const r2_bumper = rt > 0.3 ? BUTTONS.R1 : 0;

      lt = l1_pressed ? 1 : 0;
      rt = r1_pressed ? 1 : 0;

      snap.buttons = (snap.buttons & ~(BUTTONS.L1 | BUTTONS.R1)) | l2_bumper | r2_bumper;
    }

    snap.triggerLeft = lt;
    snap.triggerRight = rt;

    this.battery = Math.max(4, this.battery - 0.00035);
    snap.battery = Math.round(this.battery);
    snap.charging = now < this.rumbleUntil ? true : this.battery > 85;
    snap.latencyUs = Math.round(280 + Math.random() * 220);
    this.peak = Math.max(this.peak, snap.latencyUs);
    snap.pollHz = this.profile.turboPolling ? 1000 : 500;
    snap.timestampMs = Date.now();

    this.inputCbs.forEach((c) => c(snap));

    if (now - this.lastStats > 250) {
      this.lastStats = now;
      const s = this.stats();
      this.statsCbs.forEach((c) => c(s));
    }
  }
}

const clampAxis = (v: number) => (v < -1 ? -1 : v > 1 ? 1 : v);

const web = new WebEngine();

// ---------------------------------------------------------------------------
// Unified bridge
// ---------------------------------------------------------------------------

export const padflow = {
  mode: (): "native" | "web" => (isNative() ? "native" : "web"),

  async getConnectedGamepads(): Promise<GamepadInfo[]> {
    if (isNative()) return tauriInvoke<GamepadInfo[]>("get_connected_gamepads");
    return web.devices();
  },

  async setLedColor(padId: string, r: number, g: number, b: number) {
    if (isNative()) {
      await tauriInvoke("set_led_color", { padId, r, g, b });
      return;
    }
    web.setLed(padId, [r, g, b]);
  },

  async updateStickProfile(profileData: StickProfileConfig, padId?: string) {
    if (isNative()) {
      return tauriInvoke<StickProfileConfig>("update_stick_profile", { profileData, padId: padId ?? null });
    }
    web.setProfile(profileData, padId);
    return profileData;
  },

  async startEngine(): Promise<EngineStats> {
    if (isNative()) return tauriInvoke<EngineStats>("start_padflow_engine");
    web.start();
    return web.stats();
  },

  async stopEngine(): Promise<EngineStats> {
    if (isNative()) return tauriInvoke<EngineStats>("stop_padflow_engine");
    web.stop();
    return web.stats();
  },

  async testRumble(weak: number, strong: number) {
    if (isNative()) {
      await tauriInvoke("test_rumble", { weak, strong });
      return;
    }
    web.rumble(450);
  },

  async onInput(cb: (s: InputSnapshot) => void): Promise<Unlisten> {
    if (isNative()) {
      try {
        return await tauriListen<InputSnapshot>("padflow-input-update", (e) => {
          cb(e.payload);
        });
      } catch (err) {
        console.error("Failed to subscribe to padflow-input-update:", err);
      }
    }
    return web.onInput(cb);
  },

  async onStats(cb: (s: EngineStats) => void): Promise<Unlisten> {
    if (isNative()) {
      try {
        return await tauriListen<EngineStats>("padflow-engine-stats", (e) => {
          cb(e.payload);
        });
      } catch (err) {
        console.error("Failed to subscribe to padflow-engine-stats:", err);
      }
    }
    return web.onStats(cb);
  },

  async onHidHideUpdate(cb: (s: HidHideStatus) => void): Promise<Unlisten> {
    if (isNative()) {
      try {
        return await tauriListen<HidHideStatus>("padflow-hidhide-updated", (e) => {
          cb(e.payload);
        });
      } catch (err) {
        console.error("Failed to subscribe to padflow-hidhide-updated:", err);
      }
    }
    return web.onHidHide(cb);
  },

  async selectGamepad(padId: string): Promise<void> {
    if (isNative()) {
      await tauriInvoke("select_gamepad", { padId });
    }
  },

  async getLastSnapshot(padId?: string): Promise<InputSnapshot> {
    if (isNative()) return tauriInvoke<InputSnapshot>("get_last_snapshot", { padId });
    return {
      padId: padId ?? "",
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
  },

  async openUrl(url: string) {
    if (isNative()) {
      await tauriInvoke("open_url", { url });
    } else {
      window.open(url, "_blank");
    }
  },

  async installVigemDriver(): Promise<string> {
    if (isNative()) {
      return tauriInvoke<string>("install_vigem_driver");
    }
    throw new Error("ViGEmBus installation is only available in desktop native mode");
  },

  async getHidHideStatus(): Promise<HidHideStatus> {
    if (isNative()) return tauriInvoke<HidHideStatus>("get_hidhide_status");
    return web.hidhideStatus();
  },

  async setHidHideActive(active: boolean): Promise<HidHideStatus> {
    if (isNative()) return tauriInvoke<HidHideStatus>("set_hidhide_active", { active });
    return web.setHidHideActive(active);
  },

  async toggleDeviceHide(devicePath: string, hide: boolean): Promise<HidHideStatus> {
    if (isNative()) return tauriInvoke<HidHideStatus>("toggle_device_hide", { devicePath, hide });
    return web.toggleDeviceHide(devicePath, hide);
  },

  async autoCloakControllers(): Promise<HidHideStatus> {
    if (isNative()) return tauriInvoke<HidHideStatus>("auto_cloak_controllers");
    return web.hidhideStatus();
  },

  async installHidHideDriver(): Promise<string> {
    if (isNative()) {
      return tauriInvoke<string>("install_hidhide_driver");
    }
    throw new Error("HidHide installation is only available in desktop native mode");
  },

  async getEngineStatus(): Promise<EngineStatus> {
    if (isNative()) return tauriInvoke<EngineStatus>("get_engine_status");
    return {
      stats: web.stats(),
      profile: DEFAULT_PROFILE,
      deviceProfiles: {},
      devices: [],
      vigemInstalled: true,
      hidhideStatus: web.hidhideStatus(),
      version: "1.1.0",
    };
  },
};
