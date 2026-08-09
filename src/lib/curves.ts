import type {
  CurveKind,
  PadProfilePreset,
  StickAxisProfile,
  StickProfileConfig,
  TriggerProfile,
} from "./types";

const clamp = (v: number, lo: number, hi: number) =>
  v < lo ? lo : v > hi ? hi : v;

export const clamp01 = (v: number) => clamp(v, 0, 1);

/** Bit-for-bit mirror of `apply_curve` in src-tauri/src/input/gamepad.rs */
export function applyCurve(t: number, kind: CurveKind, power: number): number {
  const x = clamp01(t);
  const p = clamp(power, 0.5, 4);
  let out: number;
  switch (kind) {
    case "linear":
      out = x;
      break;
    case "exponential":
      out = Math.pow(x, p);
      break;
    case "sCurve": {
      const s = x * x * (3 - 2 * x);
      const k = clamp((p - 1) / 3, 0, 1);
      out = x * (1 - k) + s * k * Math.min(1 + (p - 1) * 0.15, 1.35);
      break;
    }
    case "aggressive":
      out = 1 - Math.pow(1 - x, p);
      break;
    default:
      out = x;
  }
  return clamp01(out);
}

/** Mirror of `shape_stick` (radial branch) — magnitude only. */
export function shapeMagnitude(mag: number, p: StickAxisProfile): number {
  const inner = clamp(p.innerDeadzone, 0, 0.9);
  const outer = clamp(p.outerDeadzone, inner + 0.02, 1);
  if (mag <= inner) return 0;
  let t = Math.min((mag - inner) / (outer - inner), 1);
  t = applyCurve(t, p.curve, p.curvePower);
  const anti = clamp(p.antiDeadzone, 0, 0.6);
  if (t > 0) t = anti + t * (1 - anti);
  return Math.min(t * clamp(p.sensitivity, 0.25, 3), 1);
}

export function shapeStick(
  x: number,
  y: number,
  p: StickAxisProfile,
): [number, number] {
  const yy = p.invertY ? -y : y;
  if (p.radial) {
    const mag = Math.hypot(x, yy);
    if (mag <= 1e-6) return [0, 0];
    const t = shapeMagnitude(Math.min(mag, 1), p);
    return [(x / mag) * t, (yy / mag) * t];
  }
  const axis = (v: number) => {
    const s = v < 0 ? -1 : 1;
    return s * shapeMagnitude(Math.min(Math.abs(v), 1), p);
  };
  return [axis(x), axis(yy)];
}

/** Mirror of `shape_trigger` in src-tauri/src/input/gamepad.rs */
export function shapeTrigger(v: number, p: TriggerProfile): number {
  const inner = clamp(p.innerDeadzone, 0, 0.8);
  const outer = clamp(p.outerDeadzone, inner + 0.02, 1);
  if (v <= inner) return 0;
  if (p.hairTrigger) return 1;
  return Math.min((v - inner) / (outer - inner), 1);
}

export const CURVE_LABELS: Record<CurveKind, string> = {
  linear: "Linear",
  exponential: "Exponential",
  sCurve: "S-Curve",
  aggressive: "Aggressive",
};

export const CURVE_HINTS: Record<CurveKind, string> = {
  linear: "1:1 raw translation. Predictable, zero shaping.",
  exponential: "Fine micro-aim near centre, full power at the edge.",
  sCurve: "Stable centre, fast flicks — the all-rounder.",
  aggressive: "Instant off-centre ramp for arcade & vehicle titles.",
};

const axis = (o: Partial<StickAxisProfile> = {}): StickAxisProfile => ({
  innerDeadzone: 0.06,
  outerDeadzone: 0.98,
  antiDeadzone: 0,
  curve: "linear",
  curvePower: 1,
  sensitivity: 1,
  invertY: false,
  radial: true,
  ...o,
});

const trigger = (o: Partial<TriggerProfile> = {}): TriggerProfile => ({
  innerDeadzone: 0.03,
  outerDeadzone: 0.98,
  hairTrigger: false,
  ...o,
});

export const DEFAULT_PROFILE: StickProfileConfig = {
  left: axis(),
  right: axis(),
  triggerLeft: trigger(),
  triggerRight: trigger(),
  flipTriggers: false,
  touchpadMouse: false,
  touchpadSensitivity: 1.0,
  batteryLedMode: false,
  rumbleIntensity: 1,
  turboPolling: true,
};

export const PRESETS: PadProfilePreset[] = [
  {
    id: "fps",
    name: "FPS Competitive",
    tagline: "Tight centre · exponential aim · hair triggers",
    accent: "from-cyan-400 to-sky-500",
    config: {
      left: axis({
        innerDeadzone: 0.04,
        outerDeadzone: 0.95,
        curve: "linear",
        curvePower: 1,
      }),
      right: axis({
        innerDeadzone: 0.03,
        outerDeadzone: 0.94,
        antiDeadzone: 0.12,
        curve: "exponential",
        curvePower: 1.9,
        sensitivity: 1.1,
      }),
      triggerLeft: trigger({
        innerDeadzone: 0.02,
        outerDeadzone: 0.9,
        hairTrigger: true,
      }),
      triggerRight: trigger({
        innerDeadzone: 0.02,
        outerDeadzone: 0.9,
        hairTrigger: true,
      }),
      flipTriggers: false,
      touchpadMouse: false,
      touchpadSensitivity: 1.0,
      batteryLedMode: true,
      rumbleIntensity: 0.35,
      turboPolling: true,
    },
  },
  {
    id: "action",
    name: "Action / Casual",
    tagline: "Smooth S-Curve · comfortable travel · full rumble",
    accent: "from-violet-400 to-fuchsia-500",
    config: {
      left: axis({
        innerDeadzone: 0.08,
        outerDeadzone: 0.97,
        curve: "sCurve",
        curvePower: 2,
      }),
      right: axis({
        innerDeadzone: 0.08,
        outerDeadzone: 0.97,
        antiDeadzone: 0.05,
        curve: "sCurve",
        curvePower: 2.2,
      }),
      triggerLeft: trigger({
        innerDeadzone: 0.05,
        outerDeadzone: 0.98,
        hairTrigger: false,
      }),
      triggerRight: trigger({
        innerDeadzone: 0.05,
        outerDeadzone: 0.98,
        hairTrigger: false,
      }),
      flipTriggers: false,
      touchpadMouse: true,
      touchpadSensitivity: 1.0,
      batteryLedMode: true,
      rumbleIntensity: 1,
      turboPolling: true,
    },
  },
  {
    id: "arcade",
    name: "Arcade / Fast Response",
    tagline: "Near-zero deadzone · aggressive ramp · 1 kHz polling",
    accent: "from-amber-400 to-orange-500",
    config: {
      left: axis({
        innerDeadzone: 0.01,
        outerDeadzone: 0.9,
        curve: "aggressive",
        curvePower: 1.6,
        sensitivity: 1.15,
      }),
      right: axis({
        innerDeadzone: 0.01,
        outerDeadzone: 0.9,
        curve: "aggressive",
        curvePower: 1.4,
      }),
      triggerLeft: trigger({
        innerDeadzone: 0.01,
        outerDeadzone: 0.85,
        hairTrigger: true,
      }),
      triggerRight: trigger({
        innerDeadzone: 0.01,
        outerDeadzone: 0.85,
        hairTrigger: true,
      }),
      flipTriggers: false,
      touchpadMouse: false,
      touchpadSensitivity: 1.2,
      batteryLedMode: false,
      rumbleIntensity: 0.15,
      turboPolling: true,
    },
  },
];

export const cloneProfile = (p: StickProfileConfig): StickProfileConfig =>
  JSON.parse(JSON.stringify(p)) as StickProfileConfig;

const STORAGE_KEY = "padflow_custom_profiles_v1";

export function loadUserProfiles(): PadProfilePreset[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as PadProfilePreset[];
  } catch {
    return [];
  }
}

export function saveUserProfile(name: string, config: StickProfileConfig): PadProfilePreset[] {
  const current = loadUserProfiles();
  const newPreset: PadProfilePreset = {
    id: `custom_${Date.now()}`,
    name: name.trim() || "Custom Profile",
    tagline: `User Profile · ${config.right.curve} aim · DZ ${(config.right.innerDeadzone * 100).toFixed(0)}%`,
    accent: "from-emerald-400 to-teal-500",
    config: cloneProfile(config),
  };
  const updated = [newPreset, ...current];
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  } catch {
    /* storage quota exceeded or disabled */
  }
  return updated;
}

export function deleteUserProfile(id: string): PadProfilePreset[] {
  const current = loadUserProfiles();
  const updated = current.filter((p) => p.id !== id);
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  } catch {
    /* storage quota error */
  }
  return updated;
}
