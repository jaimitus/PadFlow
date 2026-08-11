// Shared data contracts — mirrored 1:1 with src-tauri/src/input/gamepad.rs

export type PadKind =
  | "dualShock4"
  | "dualSense"
  | "dualSenseEdge"
  | "xInput"
  | "generic";

export type ConnectionType = "usb" | "bluetooth";

export type CurveKind = "linear" | "exponential" | "sCurve" | "aggressive";

export type GyroMode = "mouse" | "rightStick";

export interface GamepadInfo {
  id: string;
  name: string;
  kind: PadKind;
  connection: ConnectionType;
  /** 0..100, -1 when unavailable */
  battery: number;
  charging: boolean;
  led: [number, number, number];
  vendorId: number;
  productId: number;
  serial: string;
  path: string;
  hasLightbar: boolean;
  hasGyro: boolean;
  hasTouchpad: boolean;
  reportRateHz: number;
}

export interface StickAxisProfile {
  innerDeadzone: number;
  outerDeadzone: number;
  antiDeadzone: number;
  curve: CurveKind;
  curvePower: number;
  sensitivity: number;
  invertY: boolean;
  radial: boolean;
  /** Compensate the physical stick's elliptical range (auto-measured per pad). */
  circularityCorrection: boolean;
}

export interface TriggerProfile {
  innerDeadzone: number;
  outerDeadzone: number;
  hairTrigger: boolean;
}

export interface StickProfileConfig {
  left: StickAxisProfile;
  right: StickAxisProfile;
  triggerLeft: TriggerProfile;
  triggerRight: TriggerProfile;
  flipTriggers: boolean;
  touchpadMouse: boolean;
  touchpadSensitivity: number;
  batteryLedMode: boolean;
  rumbleIntensity: number;
  turboPolling: boolean;
  /** 16 entries: index = physical PS button bit, value = target bit. */
  buttonMap: number[];
  gyroEnabled: boolean;
  gyroMode: GyroMode;
  gyroSensitivity: number;
  gyroSmoothing: number;
  gyroInvert: boolean;
}

export interface InputSnapshot {
  padId: string;
  rawLeft: [number, number];
  rawRight: [number, number];
  left: [number, number];
  right: [number, number];
  triggerLeft: number;
  triggerRight: number;
  buttons: number;
  dpad: number;
  touchPoints: [number, number][];
  gyro: [number, number, number];
  accel: [number, number, number];
  battery: number;
  charging: boolean;
  latencyUs: number;
  pollHz: number;
  timestampMs: number;
}

export interface HidHideStatus {
  installed: boolean;
  active: boolean;
  whitelisted: boolean;
  hiddenDevices: string[];
  appPath: string;
  /** True when PadFlow holds Administrator privileges (required by HidHide writes). */
  elevated: boolean;
}

export interface EngineStats {
  running: boolean;
  virtualPadOnline: boolean;
  polls: number;
  pollHz: number;
  avgLatencyUs: number;
  peakLatencyUs: number;
  droppedReports: number;
  reconnects: number;
  driver: string;
}

export interface EngineStatus {
  stats: EngineStats;
  profile: StickProfileConfig;
  deviceProfiles: Record<string, StickProfileConfig>;
  devices: GamepadInfo[];
  vigemInstalled: boolean;
  hidhideStatus: HidHideStatus;
  version: string;
}

export const BUTTONS = {
  CROSS: 1 << 0,
  CIRCLE: 1 << 1,
  SQUARE: 1 << 2,
  TRIANGLE: 1 << 3,
  L1: 1 << 4,
  R1: 1 << 5,
  L2: 1 << 6,
  R2: 1 << 7,
  SHARE: 1 << 8,
  OPTIONS: 1 << 9,
  L3: 1 << 10,
  R3: 1 << 11,
  PS: 1 << 12,
  TOUCHPAD: 1 << 13,
  MUTE: 1 << 14,
} as const;

export const BUTTON_LAYOUT: { mask: number; label: string; xbox: string }[] = [
  { mask: BUTTONS.CROSS, label: "✕", xbox: "A" },
  { mask: BUTTONS.CIRCLE, label: "○", xbox: "B" },
  { mask: BUTTONS.SQUARE, label: "□", xbox: "X" },
  { mask: BUTTONS.TRIANGLE, label: "△", xbox: "Y" },
  { mask: BUTTONS.L1, label: "L1", xbox: "LB" },
  { mask: BUTTONS.R1, label: "R1", xbox: "RB" },
  { mask: BUTTONS.L2, label: "L2", xbox: "LT" },
  { mask: BUTTONS.R2, label: "R2", xbox: "RT" },
  { mask: BUTTONS.L3, label: "L3", xbox: "LS" },
  { mask: BUTTONS.R3, label: "R3", xbox: "RS" },
  { mask: BUTTONS.SHARE, label: "SHARE", xbox: "VIEW" },
  { mask: BUTTONS.OPTIONS, label: "OPT", xbox: "MENU" },
  { mask: BUTTONS.PS, label: "PS", xbox: "GUIDE" },
  { mask: BUTTONS.TOUCHPAD, label: "PAD", xbox: "—" },
];

export interface PadProfilePreset {
  id: string;
  name: string;
  tagline: string;
  accent: string;
  config: StickProfileConfig;
}
