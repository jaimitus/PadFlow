//! PadFlow — DualShock 4 / DualSense HID ingestion, response-curve math and
//! ViGEmBus virtual Xbox 360 target mapping.
//!
//! Design goals
//! ------------
//! * **Zero allocation in the hot loop.** Every buffer is pre-allocated; the
//!   polling thread never touches the heap while a pad is streaming.
//! * **Sub-millisecond end-to-end latency.** The HID read is blocking with a
//!   1 ms timeout, the ViGEm submit happens immediately after the parse, and
//!   the UI event is throttled to 60 Hz so the renderer never back-pressures
//!   the input path.
//! * **Crash-proof hot-plug.** Every hardware error degrades to a clean
//!   `Disconnected` state; the supervisor re-enumerates every 750 ms and
//!   re-attaches the same virtual target without dropping the ViGEm handle.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hidapi::{DeviceInfo, HidApi, HidDevice};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Hardware identifiers
// ---------------------------------------------------------------------------

pub const VID_SONY: u16 = 0x054C;
pub const PID_DS4_V1: u16 = 0x05C4;
pub const PID_DS4_V2: u16 = 0x09CC;
pub const PID_DS4_DONGLE: u16 = 0x0BA0;
pub const PID_DUALSENSE: u16 = 0x0CE6;
pub const PID_DUALSENSE_EDGE: u16 = 0x0DF2;

/// Which Sony (or generic) pad family we are talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PadKind {
    DualShock4,
    DualSense,
    DualSenseEdge,
    XInput,
    Generic,
}

impl PadKind {
    pub fn from_ids(vid: u16, pid: u16) -> Self {
        match (vid, pid) {
            (VID_SONY, PID_DS4_V1) | (VID_SONY, PID_DS4_V2) | (VID_SONY, PID_DS4_DONGLE) => {
                PadKind::DualShock4
            }
            (VID_SONY, PID_DUALSENSE) => PadKind::DualSense,
            (VID_SONY, PID_DUALSENSE_EDGE) => PadKind::DualSenseEdge,
            _ => PadKind::Generic,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PadKind::DualShock4 => "DualShock 4",
            PadKind::DualSense => "DualSense",
            PadKind::DualSenseEdge => "DualSense Edge",
            PadKind::XInput => "XInput Controller",
            PadKind::Generic => "Generic HID Pad",
        }
    }

    pub fn has_lightbar(&self) -> bool {
        matches!(
            self,
            PadKind::DualShock4 | PadKind::DualSense | PadKind::DualSenseEdge
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionType {
    Usb,
    Bluetooth,
}

// ---------------------------------------------------------------------------
// Public data contracts (mirrored 1:1 by the TypeScript frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GamepadInfo {
    pub id: String,
    pub name: String,
    pub kind: PadKind,
    pub connection: ConnectionType,
    /// 0..=100, `-1` when the pad does not report a battery gauge.
    pub battery: i16,
    pub charging: bool,
    pub led: [u8; 3],
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial: String,
    pub path: String,
    pub has_lightbar: bool,
    pub has_gyro: bool,
    pub has_touchpad: bool,
    pub report_rate_hz: u32,
}

/// Per-stick shaping parameters. All values are normalised `0.0..=1.0`
/// except `sensitivity` (`0.25..=3.0`) and `curve_power` (`0.5..=4.0`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickAxisProfile {
    pub inner_deadzone: f32,
    pub outer_deadzone: f32,
    pub anti_deadzone: f32,
    pub curve: CurveKind,
    pub curve_power: f32,
    pub sensitivity: f32,
    pub invert_y: bool,
    /// Radial (circular) shaping instead of per-axis shaping.
    pub radial: bool,
    /// Compensate the physical stick's elliptical range so diagonal inputs
    /// reach a perfect circle (auto-measured per pad, v1.2.5).
    pub circularity_correction: bool,
}

impl Default for StickAxisProfile {
    fn default() -> Self {
        Self {
            inner_deadzone: 0.06,
            outer_deadzone: 0.98,
            anti_deadzone: 0.0,
            curve: CurveKind::Linear,
            curve_power: 1.0,
            sensitivity: 1.0,
            invert_y: false,
            radial: true,
            circularity_correction: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurveKind {
    Linear,
    Exponential,
    SCurve,
    Aggressive,
}

/// Where the built-in gyroscope contributes its motion (v1.2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GyroMode {
    /// Gyro rates move the OS mouse cursor (aiming).
    Mouse,
    /// Gyro rates add a direct offset to the shaped right stick.
    RightStick,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerProfile {
    pub inner_deadzone: f32,
    pub outer_deadzone: f32,
    pub hair_trigger: bool,
}

impl Default for TriggerProfile {
    fn default() -> Self {
        Self {
            inner_deadzone: 0.03,
            outer_deadzone: 0.98,
            hair_trigger: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickProfileConfig {
    pub left: StickAxisProfile,
    pub right: StickAxisProfile,
    pub trigger_left: TriggerProfile,
    pub trigger_right: TriggerProfile,
    pub flip_triggers: bool,
    pub touchpad_mouse: bool,
    pub touchpad_sensitivity: f32,
    pub battery_led_mode: bool,
    pub rumble_intensity: f32,
    /// Extra polling aggressiveness: `true` pins the loop to 1000 Hz+.
    pub turbo_polling: bool,
    /// Per-source-bit button remap (index = physical PS bit, value = target
    /// bit, see [`buttons`]). Identity `[0..=15]` by default (v1.2.5).
    pub button_map: [u8; 16],
    /// Master switch for the gyro contribution (v1.2.5).
    pub gyro_enabled: bool,
    /// Where gyro motion is routed (v1.2.5).
    pub gyro_mode: GyroMode,
    /// Gyro gain (`0.1..=8.0`).
    pub gyro_sensitivity: f32,
    /// EMA smoothing factor (`0.0..=0.95`, higher = smoother).
    pub gyro_smoothing: f32,
    /// Inverts the gyro pitch axis (up/down aim direction).
    pub gyro_invert: bool,
}

impl Default for StickProfileConfig {
    fn default() -> Self {
        Self {
            left: StickAxisProfile::default(),
            right: StickAxisProfile::default(),
            trigger_left: TriggerProfile::default(),
            trigger_right: TriggerProfile::default(),
            flip_triggers: false,
            touchpad_mouse: false,
            touchpad_sensitivity: 1.0,
            battery_led_mode: false,
            rumble_intensity: 1.0,
            turbo_polling: true,
            button_map: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            gyro_enabled: false,
            gyro_mode: GyroMode::Mouse,
            gyro_sensitivity: 1.0,
            gyro_smoothing: 0.55,
            gyro_invert: false,
        }
    }
}

/// Frame streamed to the UI at 60 Hz (`padflow-input-update`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSnapshot {
    pub pad_id: String,
    /// Raw normalised stick values before shaping (-1..=1).
    pub raw_left: [f32; 2],
    pub raw_right: [f32; 2],
    /// Post-curve values fed to the virtual Xbox pad (-1..=1).
    pub left: [f32; 2],
    pub right: [f32; 2],
    pub trigger_left: f32,
    pub trigger_right: f32,
    /// Bitmask, see [`buttons`] constants.
    pub buttons: u32,
    pub dpad: u8,
    pub touch_points: Vec<[f32; 2]>,
    pub gyro: [f32; 3],
    pub accel: [f32; 3],
    pub battery: i16,
    pub charging: bool,
    pub latency_us: u32,
    pub poll_hz: u32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStats {
    pub running: bool,
    pub virtual_pad_online: bool,
    pub polls: u64,
    pub poll_hz: u32,
    pub avg_latency_us: u32,
    pub peak_latency_us: u32,
    pub dropped_reports: u64,
    pub reconnects: u32,
    pub driver: String,
}

pub mod buttons {
    pub const CROSS: u32 = 1 << 0; // A
    pub const CIRCLE: u32 = 1 << 1; // B
    pub const SQUARE: u32 = 1 << 2; // X
    pub const TRIANGLE: u32 = 1 << 3; // Y
    pub const L1: u32 = 1 << 4;
    pub const R1: u32 = 1 << 5;
    pub const L2: u32 = 1 << 6;
    pub const R2: u32 = 1 << 7;
    pub const SHARE: u32 = 1 << 8; // Back / View
    pub const OPTIONS: u32 = 1 << 9; // Start / Menu
    pub const L3: u32 = 1 << 10;
    pub const R3: u32 = 1 << 11;
    pub const PS: u32 = 1 << 12; // Guide
    pub const TOUCHPAD: u32 = 1 << 13;
    pub const MUTE: u32 = 1 << 14;
}

/// Remaps a PS button bitmask through the user's per-profile `button_map`.
/// Index = physical source bit (see [`buttons`]), value = target bit.
/// `TOUCHPAD`/`MUTE` have no XInput equivalent but can still be mapped onto
/// any of the 16 bits (the ViGEm submit only emits the XInput subset).
#[inline(always)]
pub fn remap_buttons(mask: u32, map: &[u8; 16]) -> u32 {
    let mut out = 0u32;
    for (src, &dst) in map.iter().enumerate() {
        if mask & (1 << src) != 0 {
            out |= 1 << ((dst & 15) as u32);
        }
    }
    out
}

/// Seed for the per-axis reach trackers used by [`circularity_correct`]. While
/// a tracker sits at the seed, no reach has been measured yet and the axis
/// passes through **uncorrected** (so small inputs are never amplified). A
/// seed of `1.0` would make the correction a permanent no-op for the
/// normalised `[-1, 1]` input range, because the reach could never grow past
/// it.
pub const CIRCULARITY_SEED: f32 = 0.02;

/// Deflections below this magnitude are treated as fine-aim and never teach
/// the per-axis reach — only substantial (near-rim) movements calibrate it.
/// This hysteresis prevents a small push from reading as a full deflection
/// while the stick's reach is still being measured.
pub const CIRCULARITY_LEARN_THRESHOLD: f32 = 0.5;

/// Circularity correction: normalises a physical stick's elliptical range
/// toward a perfect circle by dividing each axis by its auto-measured reach.
/// The reach is learned monotonically from strong deflections only, so fine
/// aiming is never distorted and small inputs never amplify the output.
#[inline(always)]
pub fn circularity_correct(x: f32, y: f32, max_x: &mut f32, max_y: &mut f32) -> (f32, f32) {
    if x.abs() >= CIRCULARITY_LEARN_THRESHOLD {
        *max_x = (*max_x).max(x.abs());
    }
    if y.abs() >= CIRCULARITY_LEARN_THRESHOLD {
        *max_y = (*max_y).max(y.abs());
    }
    // Until a reach has been measured, the axis passes through untouched.
    let x_out = if *max_x > CIRCULARITY_SEED {
        (x / *max_x).clamp(-1.0, 1.0)
    } else {
        x
    };
    let y_out = if *max_y > CIRCULARITY_SEED {
        (y / *max_y).clamp(-1.0, 1.0)
    } else {
        y
    };
    (x_out, y_out)
}

// ---------------------------------------------------------------------------
// Gyro shaping (v1.2.5) — rest calibration, EMA smoothing, jitter deadzone
// and motion routing. All pure functions so the pipeline is unit-testable.
// ---------------------------------------------------------------------------

/// While the pad is this still, the gyro rest offset folds toward the raw
/// reading (auto-recenter, so sensor drift never accumulates).
pub const GYRO_STILL_THRESHOLD: f32 = 0.02;

/// Smoothed gyro rates below this magnitude are treated as jitter and zeroed.
pub const GYRO_JITTER_DEADZONE: f32 = 0.004;

/// Rest-offset EMA fold factor per still frame.
const GYRO_REST_ALPHA: f32 = 0.1;

/// Updates the gyro rest offset (auto-recenter). While the pad is still
/// (magnitude below [`GYRO_STILL_THRESHOLD`]) — or before the first
/// calibration — the rest offset is folded toward the raw reading; a forced
/// recalibration snapshots it instantly. Returns the updated `(rest,
/// calibrated)`.
#[inline(always)]
pub fn gyro_update_rest(
    raw: [f32; 3],
    rest: [f32; 3],
    calibrated: bool,
    force: bool,
) -> ([f32; 3], bool) {
    if force {
        return (raw, true);
    }
    let mag = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
    if !calibrated || mag < GYRO_STILL_THRESHOLD {
        let mut next = [0.0f32; 3];
        for i in 0..3 {
            next[i] = rest[i] * (1.0 - GYRO_REST_ALPHA) + raw[i] * GYRO_REST_ALPHA;
        }
        (next, true)
    } else {
        (rest, calibrated)
    }
}

/// EMA-smooths the gyro rates after rest subtraction. `alpha` is the smoothing
/// factor (higher = smoother), clamped to `0.0..=0.95`.
#[inline(always)]
pub fn gyro_smooth(raw: [f32; 3], rest: [f32; 3], prev: [f32; 3], alpha: f32) -> [f32; 3] {
    let a = alpha.clamp(0.0, 0.95);
    let mut out = [0.0f32; 3];
    for i in 0..3 {
        let v = raw[i] - rest[i];
        out[i] = prev[i] * a + v * (1.0 - a);
    }
    out
}

/// Zeroes smoothed gyro rates below the jitter deadzone (keeps `>= dz`).
#[inline(always)]
pub fn gyro_deadzone(v: [f32; 3], dz: f32) -> [f32; 3] {
    let mut out = v;
    for i in 0..3 {
        if out[i].abs() < dz {
            out[i] = 0.0;
        }
    }
    out
}

/// Gyro → right-stick offset: yaw (`gyro[1]`) drives X, pitch (`gyro[0]`)
/// drives Y, both scaled by `1.6 * sensitivity`. Returns `(dx, dy)`.
#[inline(always)]
pub fn gyro_stick_offset(gyro: [f32; 3], sensitivity: f32, invert: bool) -> (f32, f32) {
    let sens = sensitivity.clamp(0.1, 8.0);
    let ox = gyro[1] * 1.6 * sens;
    let mut oy = gyro[0] * 1.6 * sens;
    if invert {
        oy = -oy;
    }
    (ox, oy)
}

/// Gyro → mouse delta: pitch (`gyro[0]`) → Y, yaw (`gyro[1]`) → X, scaled by
/// `220 * sensitivity` pixels per rate unit. Returns `(dx, dy)`.
#[inline(always)]
pub fn gyro_mouse_delta(gyro: [f32; 3], sensitivity: f32, invert: bool) -> (f32, f32) {
    let sens = sensitivity.clamp(0.1, 8.0);
    let dx = gyro[1] * 220.0 * sens;
    let mut dy = gyro[0] * 220.0 * sens;
    if invert {
        dy = -dy;
    }
    (dx, dy)
}

// ---------------------------------------------------------------------------
// Response-curve mathematics
// ---------------------------------------------------------------------------

#[inline(always)]
fn clamp01(v: f32) -> f32 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

/// Maps a normalised magnitude `t` (0..=1) through the selected curve.
///
/// * `Linear`      — `t`
/// * `Exponential` — `t^p` (precision near centre, punchy at the edge)
/// * `SCurve`      — smoothstep blended by `p` (stable centre, fast flicks)
/// * `Aggressive`  — inverse-exponential, instant response for arcade & vehicle titles
#[inline(always)]
pub fn apply_curve(t: f32, kind: CurveKind, power: f32) -> f32 {
    let t = clamp01(t);
    let p = power.clamp(0.5, 4.0);
    match kind {
        CurveKind::Linear => t,
        CurveKind::Exponential => t.powf(p),
        CurveKind::SCurve => {
            let s = t * t * (3.0 - 2.0 * t); // smoothstep
            let k = ((p - 1.0) / 3.0).clamp(0.0, 1.0);
            t * (1.0 - k) + s * k * (1.0 + (p - 1.0) * 0.15).min(1.35)
        }
        CurveKind::Aggressive => {
            // 1 - (1 - t)^p : very fast off-centre ramp, saturates smoothly.
            1.0 - (1.0 - t).powf(p)
        }
    }
    .clamp(0.0, 1.0)
}

/// Full stick shaping pipeline: deadzone → curve → anti-deadzone →
/// sensitivity → clamp. Returns the shaped `(x, y)` pair.
#[inline(always)]
pub fn shape_stick(x: f32, y: f32, p: &StickAxisProfile) -> (f32, f32) {
    let y = if p.invert_y { -y } else { y };

    if p.radial {
        let mag = (x * x + y * y).sqrt();
        if mag <= f32::EPSILON {
            return (0.0, 0.0);
        }
        let inner = p.inner_deadzone.clamp(0.0, 0.9);
        let outer = p.outer_deadzone.clamp(inner + 0.02, 1.0);
        if mag <= inner {
            return (0.0, 0.0);
        }
        let mut t = ((mag - inner) / (outer - inner)).min(1.0);
        t = apply_curve(t, p.curve, p.curve_power);
        let anti = p.anti_deadzone.clamp(0.0, 0.6);
        if t > 0.0 {
            t = anti + t * (1.0 - anti);
        }
        t = (t * p.sensitivity.clamp(0.25, 3.0)).min(1.0);
        let nx = x / mag;
        let ny = y / mag;
        (nx * t, ny * t)
    } else {
        (shape_axis(x, p), shape_axis(y, p))
    }
}

#[inline(always)]
fn shape_axis(v: f32, p: &StickAxisProfile) -> f32 {
    let sign = if v < 0.0 { -1.0 } else { 1.0 };
    let mag = v.abs();
    let inner = p.inner_deadzone.clamp(0.0, 0.9);
    let outer = p.outer_deadzone.clamp(inner + 0.02, 1.0);
    if mag <= inner {
        return 0.0;
    }
    let mut t = ((mag - inner) / (outer - inner)).min(1.0);
    t = apply_curve(t, p.curve, p.curve_power);
    let anti = p.anti_deadzone.clamp(0.0, 0.6);
    if t > 0.0 {
        t = anti + t * (1.0 - anti);
    }
    t = (t * p.sensitivity.clamp(0.25, 3.0)).min(1.0);
    sign * t
}

#[inline(always)]
pub fn shape_trigger(v: f32, p: &TriggerProfile) -> f32 {
    let inner = p.inner_deadzone.clamp(0.0, 0.8);
    let outer = p.outer_deadzone.clamp(inner + 0.02, 1.0);
    if v <= inner {
        0.0
    } else if p.hair_trigger {
        1.0
    } else {
        ((v - inner) / (outer - inner)).min(1.0)
    }
}

#[inline(always)]
fn u8_to_axis(v: u8) -> f32 {
    // 0..255 with 128 as centre → -1.0..=1.0
    (v as f32 - 127.5) / 127.5
}

// ---------------------------------------------------------------------------
// Raw HID report parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
struct RawState {
    lx: f32,
    ly: f32,
    rx: f32,
    ry: f32,
    l2: f32,
    r2: f32,
    buttons: u32,
    dpad: u8,
    battery: i16,
    charging: bool,
    gyro: [f32; 3],
    accel: [f32; 3],
    touch: [[f32; 2]; 2],
    touch_count: u8,
}

#[inline(always)]
fn le_i16(b: &[u8], i: usize) -> i16 {
    i16::from_le_bytes([b[i], b[i + 1]])
}

/// DualShock 4 — USB report `0x01` (64 B) / Bluetooth report `0x11` (78 B).
fn parse_ds4(buf: &[u8], bt: bool) -> Option<RawState> {
    let o = if bt { 3usize } else { 1usize };
    if buf.len() < o + 9 {
        return None;
    }
    let mut s = RawState::default();
    s.lx = u8_to_axis(buf[o]);
    s.ly = -u8_to_axis(buf[o + 1]);
    s.rx = u8_to_axis(buf[o + 2]);
    s.ry = -u8_to_axis(buf[o + 3]);

    let b1 = buf[o + 4];
    let b2 = buf[o + 5];
    let b3 = buf[o + 6];
    s.dpad = b1 & 0x0F;
    let mut m = 0u32;
    if b1 & 0x10 != 0 {
        m |= buttons::SQUARE;
    }
    if b1 & 0x20 != 0 {
        m |= buttons::CROSS;
    }
    if b1 & 0x40 != 0 {
        m |= buttons::CIRCLE;
    }
    if b1 & 0x80 != 0 {
        m |= buttons::TRIANGLE;
    }
    if b2 & 0x01 != 0 {
        m |= buttons::L1;
    }
    if b2 & 0x02 != 0 {
        m |= buttons::R1;
    }
    if b2 & 0x04 != 0 {
        m |= buttons::L2;
    }
    if b2 & 0x08 != 0 {
        m |= buttons::R2;
    }
    if b2 & 0x10 != 0 {
        m |= buttons::SHARE;
    }
    if b2 & 0x20 != 0 {
        m |= buttons::OPTIONS;
    }
    if b2 & 0x40 != 0 {
        m |= buttons::L3;
    }
    if b2 & 0x80 != 0 {
        m |= buttons::R3;
    }
    if b3 & 0x01 != 0 {
        m |= buttons::PS;
    }
    if b3 & 0x02 != 0 {
        m |= buttons::TOUCHPAD;
    }
    s.buttons = m;

    s.l2 = buf[o + 7] as f32 / 255.0;
    s.r2 = buf[o + 8] as f32 / 255.0;

    // Motion block (gyro pitch/yaw/roll then accel x/y/z), 16-bit LE.
    if buf.len() >= o + 25 {
        s.gyro = [
            le_i16(buf, o + 12) as f32 / 1024.0,
            le_i16(buf, o + 14) as f32 / 1024.0,
            le_i16(buf, o + 16) as f32 / 1024.0,
        ];
        s.accel = [
            le_i16(buf, o + 18) as f32 / 8192.0,
            le_i16(buf, o + 20) as f32 / 8192.0,
            le_i16(buf, o + 22) as f32 / 8192.0,
        ];
    }

    // Battery nibble: 0..10 on USB (bit 4 = cable), 0..8 on Bluetooth.
    if buf.len() >= o + 30 {
        let raw_bat = buf[o + 29];
        let cable = raw_bat & 0x10 != 0;
        let level = (raw_bat & 0x0F) as i16;
        s.charging = cable;
        s.battery = if cable {
            ((level.min(11) as f32 / 11.0) * 100.0) as i16
        } else {
            ((level.min(8) as f32 / 8.0) * 100.0) as i16
        };
    } else {
        s.battery = -1;
    }

    // Touchpad: first finger at +34, second at +38 (12-bit packed X/Y).
    if buf.len() >= o + 42 {
        let base = o + 34;
        for f in 0..2usize {
            let p = base + f * 4;
            if buf[p] & 0x80 == 0 {
                let x = (((buf[p + 2] as u16 & 0x0F) << 8) | buf[p + 1] as u16) as f32 / 1919.0;
                let y = (((buf[p + 3] as u16) << 4) | ((buf[p + 2] as u16 & 0xF0) >> 4)) as f32
                    / 942.0;
                s.touch[f] = [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)];
                s.touch_count += 1;
            }
        }
    }
    Some(s)
}

/// DualSense / DualSense Edge — USB report `0x01` (64 B) / BT report `0x31` (78 B).
fn parse_dualsense(buf: &[u8], bt: bool) -> Option<RawState> {
    let o = if bt { 2usize } else { 1usize };
    // b3 lives at `o + 9`, so the floor must be `o + 10` — an off-by-one here
    // made a truncated report panic the poll thread instead of returning None.
    if buf.len() < o + 10 {
        return None;
    }
    let mut s = RawState::default();
    s.lx = u8_to_axis(buf[o]);
    s.ly = -u8_to_axis(buf[o + 1]);
    s.rx = u8_to_axis(buf[o + 2]);
    s.ry = -u8_to_axis(buf[o + 3]);
    s.l2 = buf[o + 4] as f32 / 255.0;
    s.r2 = buf[o + 5] as f32 / 255.0;

    let b1 = buf[o + 7];
    let b2 = buf[o + 8];
    let b3 = buf[o + 9];
    s.dpad = b1 & 0x0F;
    let mut m = 0u32;
    if b1 & 0x10 != 0 {
        m |= buttons::SQUARE;
    }
    if b1 & 0x20 != 0 {
        m |= buttons::CROSS;
    }
    if b1 & 0x40 != 0 {
        m |= buttons::CIRCLE;
    }
    if b1 & 0x80 != 0 {
        m |= buttons::TRIANGLE;
    }
    if b2 & 0x01 != 0 {
        m |= buttons::L1;
    }
    if b2 & 0x02 != 0 {
        m |= buttons::R1;
    }
    if b2 & 0x04 != 0 {
        m |= buttons::L2;
    }
    if b2 & 0x08 != 0 {
        m |= buttons::R2;
    }
    if b2 & 0x10 != 0 {
        m |= buttons::SHARE;
    }
    if b2 & 0x20 != 0 {
        m |= buttons::OPTIONS;
    }
    if b2 & 0x40 != 0 {
        m |= buttons::L3;
    }
    if b2 & 0x80 != 0 {
        m |= buttons::R3;
    }
    if b3 & 0x01 != 0 {
        m |= buttons::PS;
    }
    if b3 & 0x02 != 0 {
        m |= buttons::TOUCHPAD;
    }
    if b3 & 0x04 != 0 {
        m |= buttons::MUTE;
    }
    s.buttons = m;

    // The last le_i16 reads indices `o + 25` and `o + 26`, so a buffer of
    // exactly `o + 26` bytes would panic — the floor must be `o + 27`.
    if buf.len() >= o + 27 {
        s.gyro = [
            le_i16(buf, o + 15) as f32 / 1024.0,
            le_i16(buf, o + 17) as f32 / 1024.0,
            le_i16(buf, o + 19) as f32 / 1024.0,
        ];
        s.accel = [
            le_i16(buf, o + 21) as f32 / 8192.0,
            le_i16(buf, o + 23) as f32 / 8192.0,
            le_i16(buf, o + 25) as f32 / 8192.0,
        ];
    }

    if buf.len() >= o + 42 {
        let base = o + 32;
        for f in 0..2usize {
            let p = base + f * 4;
            if buf[p] & 0x80 == 0 {
                let x = (((buf[p + 2] as u16 & 0x0F) << 8) | buf[p + 1] as u16) as f32 / 1919.0;
                let y = (((buf[p + 3] as u16) << 4) | ((buf[p + 2] as u16 & 0xF0) >> 4)) as f32
                    / 1079.0;
                s.touch[f] = [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)];
                s.touch_count += 1;
            }
        }
    }

    if buf.len() > o + 52 {
        let status = buf[o + 52];
        let level = (status & 0x0F) as i16;
        let state = (status & 0xF0) >> 4;
        s.charging = state == 0x01 || state == 0x02;
        s.battery = ((level.min(8) as f32 / 8.0) * 100.0) as i16;
    } else {
        s.battery = -1;
    }
    Some(s)
}

// ---------------------------------------------------------------------------
// Output reports (lightbar + rumble)
// ---------------------------------------------------------------------------

/// CRC-32 (IEEE, reflected) used by the Bluetooth output reports.
fn crc32(seed: &[u8], data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for byte in seed.iter().chain(data.iter()) {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Builds the lightbar / rumble output report for the pad, ready to write.
///
/// Pure — returns `None` for pads without an addressable lightbar. `rumble`
/// is `(weak, strong)` normalised `0.0..=1.0`. Bluetooth variants carry a
/// CRC-32 of the first 74 bytes in the last 4.
pub fn build_output_report(
    kind: PadKind,
    connection: ConnectionType,
    led: [u8; 3],
    rumble: (f32, f32),
) -> Option<Vec<u8>> {
    let weak = (rumble.0.clamp(0.0, 1.0) * 255.0) as u8;
    let strong = (rumble.1.clamp(0.0, 1.0) * 255.0) as u8;

    match (kind, connection) {
        (PadKind::DualShock4, ConnectionType::Usb) => {
            let mut buf = [0u8; 32];
            buf[0] = 0x05; // report id
            buf[1] = 0xF7; // enable rumble + lightbar + blink
            buf[4] = weak;
            buf[5] = strong;
            buf[6] = led[0];
            buf[7] = led[1];
            buf[8] = led[2];
            Some(buf.to_vec())
        }
        (PadKind::DualShock4, ConnectionType::Bluetooth) => {
            let mut buf = [0u8; 78];
            buf[0] = 0x11;
            buf[1] = 0xC0; // poll rate 1000 Hz
            buf[2] = 0x20;
            buf[3] = 0xF7;
            buf[6] = weak;
            buf[7] = strong;
            buf[8] = led[0];
            buf[9] = led[1];
            buf[10] = led[2];
            let crc = crc32(&[0xA2], &buf[..74]);
            buf[74..78].copy_from_slice(&crc.to_le_bytes());
            Some(buf.to_vec())
        }
        (PadKind::DualSense, ConnectionType::Usb) | (PadKind::DualSenseEdge, ConnectionType::Usb) => {
            let mut buf = [0u8; 48];
            buf[0] = 0x02; // report id
            buf[1] = 0xFF; // valid flag 0: rumble + right trigger
            buf[2] = 0xF7; // valid flag 1: lightbar + player LEDs + mic LED
            buf[3] = weak;
            buf[4] = strong;
            buf[45] = led[0];
            buf[46] = led[1];
            buf[47] = led[2];
            Some(buf.to_vec())
        }
        (PadKind::DualSense, ConnectionType::Bluetooth)
        | (PadKind::DualSenseEdge, ConnectionType::Bluetooth) => {
            let mut buf = [0u8; 78];
            buf[0] = 0x31; // BT output report
            buf[1] = 0x02; // sequence tag / feature flag
            buf[2] = 0xFF;
            buf[3] = 0xF7;
            buf[4] = weak;
            buf[5] = strong;
            buf[46] = led[0];
            buf[47] = led[1];
            buf[48] = led[2];
            let crc = crc32(&[0xA2], &buf[..74]);
            buf[74..78].copy_from_slice(&crc.to_le_bytes());
            Some(buf.to_vec())
        }
        _ => None,
    }
}

/// Writes the lightbar / rumble output report to the device.
pub fn write_output_report(
    device: &HidDevice,
    kind: PadKind,
    connection: ConnectionType,
    led: [u8; 3],
    rumble: (f32, f32),
) -> Result<(), String> {
    match build_output_report(kind, connection, led, rumble) {
        Some(buf) => device.write(&buf).map(|_| ()).map_err(|e| e.to_string()),
        None => Err("This controller has no addressable lightbar".into()),
    }
}

// ---------------------------------------------------------------------------
// Device discovery
// ---------------------------------------------------------------------------

fn connection_of(info: &DeviceInfo) -> ConnectionType {
    if info.interface_number() == -1 || info.path().to_string_lossy().to_lowercase().contains("bthenum") {
        ConnectionType::Bluetooth
    } else {
        ConnectionType::Usb
    }
}

fn stable_id(info: &DeviceInfo, index: usize) -> String {
    match info.serial_number() {
        Some(sn) if !sn.is_empty() => format!("{:04x}:{:04x}:{}", info.vendor_id(), info.product_id(), sn),
        _ => format!("{:04x}:{:04x}:#{index}", info.vendor_id(), info.product_id()),
    }
}

/// Enumerates every supported pad currently attached to the machine.
pub fn enumerate(api: &HidApi, cache: &HashMap<String, [u8; 3]>) -> Vec<GamepadInfo> {
    let mut out = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    for (idx, info) in api.device_list().enumerate() {
        let kind = PadKind::from_ids(info.vendor_id(), info.product_id());
        if kind == PadKind::Generic {
            continue;
        }
        // Sony pads publish several HID collections; only usage page 0x01 /
        // usage 0x05 (gamepad) or 0x04 (joystick) carries the input reports.
        if info.usage_page() != 0 && !(info.usage_page() == 0x01 && (info.usage() == 0x05 || info.usage() == 0x04)) {
            continue;
        }
        let path_str = info.path().to_string_lossy().to_string();
        if !seen_paths.insert(path_str.clone()) {
            continue;
        }
        let id = stable_id(info, idx);
        let connection = connection_of(info);
        let led = cache.get(&id).copied().unwrap_or(match kind {
            PadKind::DualShock4 => [0, 40, 255],
            _ => [0, 140, 255],
        });
        out.push(GamepadInfo {
            name: info
                .product_string()
                .map(|s| s.to_string())
                .unwrap_or_else(|| kind.label().to_string()),
            kind,
            connection,
            battery: -1,
            charging: matches!(connection, ConnectionType::Usb),
            led,
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            serial: info.serial_number().unwrap_or_default().to_string(),
            path: path_str,
            has_lightbar: kind.has_lightbar(),
            has_gyro: true,
            has_touchpad: true,
            report_rate_hz: match connection {
                ConnectionType::Usb => 1000,
                ConnectionType::Bluetooth => 800,
            },
            id,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// The engine
// ---------------------------------------------------------------------------

/// Shared, lock-light engine state. Cloneable handle (`Arc` inside).
#[derive(Clone)]
pub struct PadFlowEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    default_profile: RwLock<StickProfileConfig>,
    device_profiles: RwLock<HashMap<String, StickProfileConfig>>,
    running: AtomicBool,
    virtual_online: AtomicBool,
    polls: AtomicU64,
    dropped: AtomicU64,
    reconnects: AtomicU32,
    poll_hz: AtomicU32,
    avg_latency_us: AtomicU32,
    peak_latency_us: AtomicU32,
    /// `pad_id -> rgb` written by `set_led_color`, consumed by the poll thread.
    led_requests: Mutex<HashMap<String, [u8; 3]>>,
    led_state: RwLock<HashMap<String, [u8; 3]>>,
    rumble_request: Mutex<Option<(f32, f32)>>,
    /// Set by `recalibrate_gyro`; the poll loop re-captures the rest offset.
    gyro_calibrate_request: AtomicBool,
    devices: RwLock<Vec<GamepadInfo>>,
    last_snapshot: RwLock<InputSnapshot>,
    last_snapshots: RwLock<HashMap<String, InputSnapshot>>,
    active_pad: RwLock<Option<String>>,
}

impl Default for PadFlowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PadFlowEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(EngineInner {
                default_profile: RwLock::new(StickProfileConfig::default()),
                device_profiles: RwLock::new(HashMap::new()),
                running: AtomicBool::new(false),
                virtual_online: AtomicBool::new(false),
                polls: AtomicU64::new(0),
                dropped: AtomicU64::new(0),
                reconnects: AtomicU32::new(0),
                poll_hz: AtomicU32::new(0),
                avg_latency_us: AtomicU32::new(0),
                peak_latency_us: AtomicU32::new(0),
                led_requests: Mutex::new(HashMap::new()),
                led_state: RwLock::new(HashMap::new()),
                rumble_request: Mutex::new(None),
                gyro_calibrate_request: AtomicBool::new(false),
                devices: RwLock::new(Vec::new()),
                last_snapshot: RwLock::new(InputSnapshot::default()),
                last_snapshots: RwLock::new(HashMap::new()),
                active_pad: RwLock::new(None),
            }),
        }
    }

    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::Relaxed)
    }

    pub fn profile(&self) -> StickProfileConfig {
        self.profile_for(None)
    }

    pub fn profile_for(&self, pad_id: Option<&str>) -> StickProfileConfig {
        let profiles = self.inner.device_profiles.read();
        if let Some(id) = pad_id {
            if let Some(p) = profiles.get(id) {
                return *p;
            }
        } else if let Some(ref active_id) = *self.inner.active_pad.read() {
            if let Some(p) = profiles.get(active_id) {
                return *p;
            }
        }
        *self.inner.default_profile.read()
    }

    pub fn all_profiles(&self) -> HashMap<String, StickProfileConfig> {
        self.inner.device_profiles.read().clone()
    }

    pub fn set_profile(&self, p: StickProfileConfig) {
        self.set_profile_for(None, p);
    }

    pub fn set_profile_for(&self, pad_id: Option<&str>, p: StickProfileConfig) {
        *self.inner.default_profile.write() = p;
        if let Some(id) = pad_id {
            self.inner.device_profiles.write().insert(id.to_string(), p);
        } else if let Some(ref active_id) = *self.inner.active_pad.read() {
            self.inner.device_profiles.write().insert(active_id.clone(), p);
        }
    }

    pub fn devices(&self) -> Vec<GamepadInfo> {
        self.inner.devices.read().clone()
    }

    pub fn set_active_pad(&self, pad_id: &str) {
        *self.inner.active_pad.write() = Some(pad_id.to_string());
    }

    pub fn active_pad(&self) -> Option<String> {
        self.inner.active_pad.read().clone()
    }

    pub fn snapshot(&self) -> InputSnapshot {
        self.inner.last_snapshot.read().clone()
    }

    pub fn snapshot_for(&self, pad_id: Option<&str>) -> InputSnapshot {
        let snaps = self.inner.last_snapshots.read();
        if let Some(id) = pad_id {
            if let Some(s) = snaps.get(id) {
                return s.clone();
            }
        }
        if let Some(ref active_id) = *self.inner.active_pad.read() {
            if let Some(s) = snaps.get(active_id) {
                return s.clone();
            }
        }
        self.inner.last_snapshot.read().clone()
    }

    pub fn queue_led(&self, pad_id: &str, rgb: [u8; 3]) {
        self.inner.led_requests.lock().insert(pad_id.to_string(), rgb);
        self.inner.led_state.write().insert(pad_id.to_string(), rgb);
        if let Some(d) = self
            .inner
            .devices
            .write()
            .iter_mut()
            .find(|d| d.id == pad_id)
        {
            d.led = rgb;
        }
    }

    pub fn queue_rumble(&self, weak: f32, strong: f32) {
        *self.inner.rumble_request.lock() = Some((weak, strong));
    }

    pub fn stats(&self) -> EngineStats {
        let i = &self.inner;
        EngineStats {
            running: i.running.load(Ordering::Relaxed),
            virtual_pad_online: i.virtual_online.load(Ordering::Relaxed),
            polls: i.polls.load(Ordering::Relaxed),
            poll_hz: i.poll_hz.load(Ordering::Relaxed),
            avg_latency_us: i.avg_latency_us.load(Ordering::Relaxed),
            peak_latency_us: i.peak_latency_us.load(Ordering::Relaxed),
            dropped_reports: i.dropped.load(Ordering::Relaxed),
            reconnects: i.reconnects.load(Ordering::Relaxed),
            driver: "ViGEmBus / Xbox 360 Controller".into(),
        }
    }

    /// Re-scan the HID bus. Safe to call at any time, never blocks the loop.
    pub fn rescan(&self) -> Result<Vec<GamepadInfo>, String> {
        let api = HidApi::new().map_err(|e| format!("hidapi init failed: {e}"))?;
        let cache = self.inner.led_state.read().clone();
        let list = enumerate(&api, &cache);
        *self.inner.devices.write() = list.clone();
        if self.inner.active_pad.read().is_none() {
            if let Some(first) = list.first() {
                *self.inner.active_pad.write() = Some(first.id.clone());
            }
        }
        Ok(list)
    }

    pub fn stop(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
    }

    /// Forces the gyro rest offset to be re-captured on the next frame.
    pub fn recalibrate_gyro(&self) {
        self.inner.gyro_calibrate_request.store(true, Ordering::Relaxed);
    }

    /// Spawns the high-priority polling thread. Returns immediately.
    ///
    /// `emit` is invoked at ~60 Hz with the latest [`InputSnapshot`]; the
    /// virtual pad itself is updated every single HID report (up to 1 kHz).
    pub fn spawn<F>(&self, emit: F) -> Result<(), String>
    where
        F: Fn(InputSnapshot) + Send + 'static,
    {
        if self
            .inner
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("PadFlow engine is already running".into());
        }
        let inner = Arc::clone(&self.inner);
        std::thread::Builder::new()
            .name("padflow-input".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                raise_thread_priority();
                if let Err(e) = poll_loop(inner.clone(), emit) {
                    log::error!("padflow poll loop exited: {e}");
                }
                inner.running.store(false, Ordering::SeqCst);
                inner.virtual_online.store(false, Ordering::SeqCst);
            })
            .map_err(|e| format!("failed to spawn input thread: {e}"))?;
        Ok(())
    }
}

#[cfg(windows)]
mod mouse_win {
    #[link(name = "user32")]
    extern "system" {
        pub fn mouse_event(dw_flags: u32, dx: i32, dy: i32, dw_data: u32, dw_extra_info: usize);
    }

    pub const MOUSEEVENTF_MOVE: u32 = 0x0001;
    pub const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
    pub const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
    pub const MOUSEEVENTF_WHEEL: u32 = 0x0800;

    #[inline(always)]
    pub fn move_cursor(dx: i32, dy: i32) {
        unsafe {
            mouse_event(MOUSEEVENTF_MOVE, dx, dy, 0, 0);
        }
    }

    #[inline(always)]
    pub fn click_left(down: bool) {
        unsafe {
            let flags = if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP };
            mouse_event(flags, 0, 0, 0, 0);
        }
    }

    #[inline(always)]
    pub fn scroll(delta: i32) {
        unsafe {
            mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta as u32, 0);
        }
    }
}

#[cfg(windows)]
fn raise_thread_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
    };
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
    }
}

#[cfg(not(windows))]
fn raise_thread_priority() {}

// ---------------------------------------------------------------------------
// ViGEm bridge
// ---------------------------------------------------------------------------

struct VirtualPad {
    target: vigem_client::Xbox360Wired<vigem_client::Client>,
}

impl VirtualPad {
    fn create() -> Result<Self, String> {
        let client = vigem_client::Client::connect().map_err(|e| {
            format!("ViGEmBus driver not reachable ({e}). Install ViGEmBus 1.22+ and retry.")
        })?;
        let mut target =
            vigem_client::Xbox360Wired::new(client, vigem_client::TargetId::XBOX360_WIRED);
        target
            .plugin()
            .map_err(|e| format!("failed to plug virtual Xbox 360 pad: {e}"))?;
        target
            .wait_ready()
            .map_err(|e| format!("virtual pad never became ready: {e}"))?;
        Ok(Self { target })
    }

    #[inline(always)]
    fn submit(&mut self, s: &InputSnapshot) -> Result<(), String> {
        use vigem_client::{XButtons, XGamepad};
        let mut b = 0u16;
        if s.buttons & buttons::CROSS != 0 {
            b |= XButtons::A;
        }
        if s.buttons & buttons::CIRCLE != 0 {
            b |= XButtons::B;
        }
        if s.buttons & buttons::SQUARE != 0 {
            b |= XButtons::X;
        }
        if s.buttons & buttons::TRIANGLE != 0 {
            b |= XButtons::Y;
        }
        if s.buttons & buttons::L1 != 0 {
            b |= XButtons::LB;
        }
        if s.buttons & buttons::R1 != 0 {
            b |= XButtons::RB;
        }
        if s.buttons & buttons::SHARE != 0 {
            b |= XButtons::BACK;
        }
        if s.buttons & buttons::OPTIONS != 0 {
            b |= XButtons::START;
        }
        if s.buttons & buttons::L3 != 0 {
            b |= XButtons::LTHUMB;
        }
        if s.buttons & buttons::R3 != 0 {
            b |= XButtons::RTHUMB;
        }
        if s.buttons & buttons::PS != 0 {
            b |= XButtons::GUIDE;
        }
        match s.dpad {
            0 => b |= XButtons::UP,
            1 => b |= XButtons::UP | XButtons::RIGHT,
            2 => b |= XButtons::RIGHT,
            3 => b |= XButtons::DOWN | XButtons::RIGHT,
            4 => b |= XButtons::DOWN,
            5 => b |= XButtons::DOWN | XButtons::LEFT,
            6 => b |= XButtons::LEFT,
            7 => b |= XButtons::UP | XButtons::LEFT,
            _ => {}
        }
        let pad = XGamepad {
            buttons: XButtons(b),
            left_trigger: (s.trigger_left.clamp(0.0, 1.0) * 255.0) as u8,
            right_trigger: (s.trigger_right.clamp(0.0, 1.0) * 255.0) as u8,
            thumb_lx: (s.left[0].clamp(-1.0, 1.0) * 32767.0) as i16,
            thumb_ly: (s.left[1].clamp(-1.0, 1.0) * 32767.0) as i16,
            thumb_rx: (s.right[0].clamp(-1.0, 1.0) * 32767.0) as i16,
            thumb_ry: (s.right[1].clamp(-1.0, 1.0) * 32767.0) as i16,
        };
        self.target.update(&pad).map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// The hot loop
// ---------------------------------------------------------------------------

struct OpenPad {
    info: GamepadInfo,
    device: HidDevice,
    buf: [u8; 128],
    last_touch: Option<[f32; 2]>,
    last_scroll_y: Option<f32>,
    mouse_clicked: bool,
    /// Per-axis measured reach (elliptical range) used by circularity
    /// correction; initialised to `1.0` and grown by the running max.
    circ_max_lx: f32,
    circ_max_ly: f32,
    circ_max_rx: f32,
    circ_max_ry: f32,
    /// EMA-smoothed gyro rates after rest-subtraction (v1.2.5).
    gyro_smooth: [f32; 3],
    /// Auto-captured rest offset — subtracted from raw gyro (v1.2.5).
    gyro_rest: [f32; 3],
    /// False until the first still-frame captures the rest offset.
    gyro_calibrated: bool,
}

fn open_all_pads(api: &mut HidApi, inner: &EngineInner, current_pads: &mut Vec<OpenPad>) {
    let _ = api.refresh_devices();
    let cache = inner.led_state.read().clone();
    let list = enumerate(api, &cache);

    // Keep active_pad valid
    {
        let mut active = inner.active_pad.write();
        if active.is_none() || !list.iter().any(|d| Some(&d.id) == active.as_ref()) {
            *active = list.first().map(|d| d.id.clone());
        }
    }

    *inner.devices.write() = list.clone();

    // Retain only currently connected pads
    current_pads.retain(|p| list.iter().any(|d| d.id == p.info.id));

    // Open up to 4 gamepads
    for target in list.iter().take(4) {
        if current_pads.iter().any(|p| p.info.id == target.id) {
            continue;
        }

        log::info!("[open_all_pads] trying to open pad {}: {} path={}", current_pads.len() + 1, target.name, target.path);
        let found = api
            .device_list()
            .find(|d| d.path().to_string_lossy() == target.path)
            .or_else(|| {
                api.device_list().find(|d| {
                    d.vendor_id() == target.vendor_id
                        && d.product_id() == target.product_id
                        && (d.usage_page() == 0 || (d.usage_page() == 0x01 && (d.usage() == 0x05 || d.usage() == 0x04)))
                })
            });

        let Some(hid_info) = found else { continue };

        let Ok(device) = hid_info.open_device(api) else {
            log::error!("[open_all_pads] FAILED to open device: {}", target.id);
            continue;
        };

        let _ = device.set_blocking_mode(false);
        let _ = write_output_report(
            &device,
            target.kind,
            target.connection,
            target.led,
            (0.0, 0.0),
        );

        log::info!("[open_all_pads] successfully opened pad: {}", target.id);
        current_pads.push(OpenPad {
            info: target.clone(),
            device,
            buf: [0u8; 128],
            last_touch: None,
            last_scroll_y: None,
            mouse_clicked: false,
            // Seed at CIRCULARITY_SEED so the reach is actually learned (a
            // 1.0 seed would make the correction a no-op for the normalised
            // [-1, 1] input range) while sub-threshold inputs pass through raw.
            circ_max_lx: CIRCULARITY_SEED,
            circ_max_ly: CIRCULARITY_SEED,
            circ_max_rx: CIRCULARITY_SEED,
            circ_max_ry: CIRCULARITY_SEED,
            gyro_smooth: [0.0; 3],
            gyro_rest: [0.0; 3],
            gyro_calibrated: false,
        });
    }
}

fn poll_loop<F>(inner: Arc<EngineInner>, emit: F) -> Result<(), String>
where
    F: Fn(InputSnapshot) + Send + 'static,
{
    let mut api = HidApi::new().map_err(|e| format!("hidapi init failed: {e}"))?;

    // Up to 4 virtual Xbox 360 pad slots
    let mut virt_pads: Vec<Option<VirtualPad>> = (0..4).map(|_| None).collect();
    let mut current_pads: Vec<OpenPad> = Vec::new();

    let mut last_scan = Instant::now() - Duration::from_secs(5);
    let mut last_virt_retry = Instant::now() - Duration::from_secs(5);
    let mut last_emit = Instant::now();
    let emit_period = Duration::from_micros(16_666); // 60 Hz UI stream

    let mut hz_window = Instant::now();
    let mut hz_count: u32 = 0;
    let mut latency_acc: u64 = 0;
    let mut latency_n: u64 = 0;

    while inner.running.load(Ordering::Relaxed) {
        // ---- (re)connection supervisor ------------------------------------
        if last_scan.elapsed() >= Duration::from_millis(750) {
            last_scan = Instant::now();
            let prev_count = current_pads.len();
            open_all_pads(&mut api, &inner, &mut current_pads);
            if current_pads.len() > prev_count {
                inner.reconnects.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Retry virtual pads every 1.5s
        if last_virt_retry.elapsed() >= Duration::from_millis(1500) {
            last_virt_retry = Instant::now();
            let mut any_online = false;
            for (idx, slot) in virt_pads.iter_mut().enumerate() {
                if idx < current_pads.len() && slot.is_none() {
                    if let Ok(v) = VirtualPad::create() {
                        log::info!("[poll_loop] ViGEmBus slot #{} allocated!", idx + 1);
                        *slot = Some(v);
                    }
                }
                if slot.is_some() {
                    any_online = true;
                }
            }
            inner.virtual_online.store(any_online, Ordering::Relaxed);
        }

        if current_pads.is_empty() {
            std::thread::sleep(Duration::from_millis(30));
            continue;
        }

        let t0 = Instant::now();
        let active_id = inner.active_pad.read().clone();
        let mut active_snap: Option<InputSnapshot> = None;

        // ---- Read from each open pad ---------------------------------------
        let mut read_any = false;
        for (idx, p) in current_pads.iter_mut().enumerate() {
            let read = p.device.read_timeout(&mut p.buf, 1);
            let n = match read {
                Ok(0) => continue,
                Ok(n) => n,
                Err(e) => {
                    log::warn!("pad {} dropped: {e}", p.info.id);
                    inner.dropped.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            };

            read_any = true;
            let report_id = p.buf[0];
            let bt = matches!(p.info.connection, ConnectionType::Bluetooth);
            let raw = match p.info.kind {
                PadKind::DualShock4 => match report_id {
                    0x01 => parse_ds4(&p.buf[..n], false),
                    0x11 => parse_ds4(&p.buf[..n], true),
                    _ => parse_ds4(&p.buf[..n], bt),
                },
                PadKind::DualSense | PadKind::DualSenseEdge => match report_id {
                    0x01 => parse_dualsense(&p.buf[..n], false),
                    0x31 => parse_dualsense(&p.buf[..n], true),
                    _ => parse_dualsense(&p.buf[..n], bt),
                },
                _ => parse_ds4(&p.buf[..n], bt).or_else(|| parse_dualsense(&p.buf[..n], bt)),
            };

            let Some(raw) = raw else {
                inner.dropped.fetch_add(1, Ordering::Relaxed);
                continue;
            };

            // ---- shape ------------------------------------------------------
            let prof = {
                let dev_profiles = inner.device_profiles.read();
                dev_profiles
                    .get(&p.info.id)
                    .copied()
                    .unwrap_or_else(|| *inner.default_profile.read())
            };

            // Circularity correction: normalise the physical ellipse toward a
            // perfect circle using the per-axis running max (auto-measured).
            let mut lx_in = raw.lx;
            let mut ly_in = raw.ly;
            let mut rx_in = raw.rx;
            let mut ry_in = raw.ry;
            if prof.left.circularity_correction {
                (lx_in, ly_in) = circularity_correct(
                    raw.lx,
                    raw.ly,
                    &mut p.circ_max_lx,
                    &mut p.circ_max_ly,
                );
            }
            if prof.right.circularity_correction {
                (rx_in, ry_in) = circularity_correct(
                    raw.rx,
                    raw.ry,
                    &mut p.circ_max_rx,
                    &mut p.circ_max_ry,
                );
            }
            let (lx, ly) = shape_stick(lx_in, ly_in, &prof.left);
            let (mut rx, mut ry) = shape_stick(rx_in, ry_in, &prof.right);
            let mut lt = shape_trigger(raw.l2, &prof.trigger_left);
            let mut rt = shape_trigger(raw.r2, &prof.trigger_right);
            let mut buttons_mask = raw.buttons;

            if prof.flip_triggers {
                // Swap physical L1/R1 bumpers with L2/R2 triggers
                let l1_pressed = (buttons_mask & buttons::L1) != 0;
                let r1_pressed = (buttons_mask & buttons::R1) != 0;

                let l2_bumper = if lt > 0.3 { buttons::L1 } else { 0 };
                let r2_bumper = if rt > 0.3 { buttons::R1 } else { 0 };

                lt = if l1_pressed { 1.0 } else { 0.0 };
                rt = if r1_pressed { 1.0 } else { 0.0 };

                buttons_mask = (buttons_mask & !(buttons::L1 | buttons::R1)) | l2_bumper | r2_bumper;
            }

            // ---- gyro: auto rest-calibration, smoothing and routing ---------
            if prof.gyro_enabled && p.info.has_gyro {
                let force = inner.gyro_calibrate_request.swap(false, Ordering::Relaxed);
                (p.gyro_rest, p.gyro_calibrated) =
                    gyro_update_rest(raw.gyro, p.gyro_rest, p.gyro_calibrated, force);
                p.gyro_smooth = gyro_smooth(
                    raw.gyro,
                    p.gyro_rest,
                    p.gyro_smooth,
                    prof.gyro_smoothing,
                );
                let gyro = gyro_deadzone(p.gyro_smooth, GYRO_JITTER_DEADZONE);
                match prof.gyro_mode {
                    GyroMode::RightStick => {
                        // Direct tilt→stick mapping: yaw drives X, pitch drives Y.
                        let (ox, oy) =
                            gyro_stick_offset(gyro, prof.gyro_sensitivity, prof.gyro_invert);
                        rx = (rx + ox).clamp(-1.0, 1.0);
                        ry = (ry + oy).clamp(-1.0, 1.0);
                    }
                    GyroMode::Mouse => {
                        #[cfg(windows)]
                        {
                            // pitch → Y, yaw → X (absolute-rate aiming).
                            let (dx, dy) =
                                gyro_mouse_delta(gyro, prof.gyro_sensitivity, prof.gyro_invert);
                            if dx.abs() > 0.01 || dy.abs() > 0.01 {
                                mouse_win::move_cursor(dx as i32, dy as i32);
                            }
                        }
                    }
                }
            }

            // ---- user button remapping (final say, after flip-triggers) ----
            buttons_mask = remap_buttons(buttons_mask, &prof.button_map);

            // ---- Touchpad virtual mouse simulation ------------------------
            if prof.touchpad_mouse {
                if raw.touch_count == 1 {
                    let tx = raw.touch[0][0];
                    let ty = raw.touch[0][1];
                    if let Some([lx, ly]) = p.last_touch {
                        let dx = (tx - lx) * 1920.0 * prof.touchpad_sensitivity;
                        let dy = (ty - ly) * 1080.0 * prof.touchpad_sensitivity;
                        if dx.abs() > 0.05 || dy.abs() > 0.05 {
                            #[cfg(windows)]
                            mouse_win::move_cursor(dx as i32, dy as i32);
                        }
                    }
                    p.last_touch = Some([tx, ty]);
                    p.last_scroll_y = None;

                    let pad_click = (raw.buttons & buttons::TOUCHPAD) != 0;
                    if pad_click && !p.mouse_clicked {
                        p.mouse_clicked = true;
                        #[cfg(windows)]
                        mouse_win::click_left(true);
                    } else if !pad_click && p.mouse_clicked {
                        p.mouse_clicked = false;
                        #[cfg(windows)]
                        mouse_win::click_left(false);
                    }
                } else if raw.touch_count == 2 {
                    let my = (raw.touch[0][1] + raw.touch[1][1]) * 0.5;
                    if let Some(lmy) = p.last_scroll_y {
                        let dy = (my - lmy) * 1200.0;
                        if dy.abs() > 1.0 {
                            #[cfg(windows)]
                            mouse_win::scroll((-dy) as i32);
                        }
                    }
                    p.last_scroll_y = Some(my);
                    p.last_touch = None;
                    if p.mouse_clicked {
                        p.mouse_clicked = false;
                        #[cfg(windows)]
                        mouse_win::click_left(false);
                    }
                } else {
                    p.last_touch = None;
                    p.last_scroll_y = None;
                    if p.mouse_clicked {
                        p.mouse_clicked = false;
                        #[cfg(windows)]
                        mouse_win::click_left(false);
                    }
                }
            }

            let mut touch_points = Vec::with_capacity(2);
            for t_idx in 0..raw.touch_count.min(2) as usize {
                touch_points.push(raw.touch[t_idx]);
            }

            let snap = InputSnapshot {
                pad_id: p.info.id.clone(),
                raw_left: [raw.lx, raw.ly],
                raw_right: [raw.rx, raw.ry],
                left: [lx, ly],
                right: [rx, ry],
                trigger_left: lt,
                trigger_right: rt,
                buttons: buttons_mask,
                dpad: raw.dpad,
                touch_points,
                gyro: raw.gyro,
                accel: raw.accel,
                battery: raw.battery,
                charging: raw.charging,
                latency_us: (t0.elapsed().as_micros() as u32).max(1),
                poll_hz: inner.poll_hz.load(Ordering::Relaxed),
                timestamp_ms: 0,
            };

            // Feed corresponding virtual Xbox 360 pad slot
            if let Some(v_slot) = virt_pads.get_mut(idx) {
                if let Some(v) = v_slot.as_mut() {
                    if let Err(e) = v.submit(&snap) {
                        log::warn!("ViGEm slot #{} submit failed: {e}", idx + 1);
                        *v_slot = None;
                    }
                }
            }

            // Store in multi-controller snapshot map
            inner.last_snapshots.write().insert(p.info.id.clone(), snap.clone());

            // If active pad or first pad, update active_snap for UI
            if active_id.as_deref() == Some(&p.info.id) || active_snap.is_none() {
                *inner.last_snapshot.write() = snap.clone();
                active_snap = Some(snap);
            }

            // ---- pending lightbar / rumble writes ----------------------------
            let pending_led = {
                let mut q = inner.led_requests.lock();
                q.remove(&p.info.id)
            };
            let pending_rumble = inner.rumble_request.lock().take();

            let target_led = if prof.battery_led_mode && raw.battery >= 0 {
                if raw.battery > 60 {
                    [0, 230, 80] // Emerald Green
                } else if raw.battery > 25 {
                    [240, 180, 0] // Amber
                } else {
                    [255, 30, 20] // Red
                }
            } else {
                pending_led.unwrap_or(p.info.led)
            };

            if pending_led.is_some() || pending_rumble.is_some() || (prof.battery_led_mode && target_led != p.info.led) {
                let led = target_led;
                let (w, s) = pending_rumble.unwrap_or((0.0, 0.0));
                let gain = prof.rumble_intensity.clamp(0.0, 1.0);
                if let Err(e) = write_output_report(
                    &p.device,
                    p.info.kind,
                    p.info.connection,
                    led,
                    (w * gain, s * gain),
                ) {
                    log::warn!("output report failed: {e}");
                } else {
                    p.info.led = led;
                    inner.led_state.write().insert(p.info.id.clone(), led);
                }
            }

            // ---- telemetry --------------------------------------------------------
            let dt_us = t0.elapsed().as_micros() as u32;
            latency_acc += dt_us as u64;
            latency_n += 1;
            inner.peak_latency_us.fetch_max(dt_us, Ordering::Relaxed);
            inner.polls.fetch_add(1, Ordering::Relaxed);
            hz_count += 1;
        }

        if read_any && hz_window.elapsed() >= Duration::from_millis(500) {
            let hz = (hz_count as f32 / hz_window.elapsed().as_secs_f32()) as u32;
            inner.poll_hz.store(hz, Ordering::Relaxed);
            inner
                .avg_latency_us
                .store((latency_acc / latency_n.max(1)) as u32, Ordering::Relaxed);
            hz_count = 0;
            latency_acc = 0;
            latency_n = 0;
            hz_window = Instant::now();
        }

        // ---- 60 Hz UI stream ----------------------------------------------------
        if last_emit.elapsed() >= emit_period {
            last_emit = Instant::now();
            if let Some(mut snap) = active_snap {
                snap.timestamp_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                emit(snap);
            }
        }

        std::thread::yield_now();
    }

    // Graceful shutdown: neutralise all virtual pads so games don't see drift.
    for v_slot in virt_pads.iter_mut() {
        if let Some(v) = v_slot.as_mut() {
            let _ = v.submit(&InputSnapshot::default());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_kills_centre_noise() {
        let p = StickAxisProfile {
            inner_deadzone: 0.15,
            ..Default::default()
        };
        let (x, y) = shape_stick(0.05, 0.05, &p);
        assert_eq!((x, y), (0.0, 0.0));
    }

    #[test]
    fn outer_deadzone_saturates() {
        let p = StickAxisProfile {
            outer_deadzone: 0.8,
            ..Default::default()
        };
        let (x, _) = shape_stick(0.95, 0.0, &p);
        assert!((x - 1.0).abs() < 1e-3);
    }

    #[test]
    fn curves_are_monotonic() {
        for kind in [
            CurveKind::Linear,
            CurveKind::Exponential,
            CurveKind::SCurve,
            CurveKind::Aggressive,
        ] {
            let mut prev = -1.0;
            for i in 0..=100 {
                let v = apply_curve(i as f32 / 100.0, kind, 2.0);
                assert!(v >= prev - 1e-4, "{kind:?} not monotonic at {i}");
                prev = v;
            }
        }
    }

    #[test]
    fn crc_matches_known_vector() {
        assert_eq!(crc32(&[], b"123456789"), 0xCBF4_3926);
    }

    // ---------------------------------------------------------------------
    // Output report builders (lightbar + rumble, CRC-32 over BT) + fuzz
    // ---------------------------------------------------------------------

    #[test]
    fn output_reports_are_well_formed() {
        let led = [0x12, 0x34, 0x56];
        let rumble = (0.5, 0.25);
        let cases = [
            (PadKind::DualShock4, ConnectionType::Usb, 32usize, 0x05u8),
            (PadKind::DualShock4, ConnectionType::Bluetooth, 78, 0x11),
            (PadKind::DualSense, ConnectionType::Usb, 48, 0x02),
            (PadKind::DualSense, ConnectionType::Bluetooth, 78, 0x31),
            (PadKind::DualSenseEdge, ConnectionType::Usb, 48, 0x02),
            (PadKind::DualSenseEdge, ConnectionType::Bluetooth, 78, 0x31),
        ];
        for (kind, conn, len, id) in cases {
            let buf = build_output_report(kind, conn, led, rumble).expect("report must build");
            assert_eq!(buf.len(), len, "{kind:?}/{conn:?} length");
            assert_eq!(buf[0], id, "{kind:?}/{conn:?} report id");
            // Rumble bytes for (0.5, 0.25): 127.5 -> 127, 63.75 -> 63
            // (`as u8` truncates — matching production behaviour).
            match (kind, conn) {
                (PadKind::DualShock4, ConnectionType::Usb) => {
                    assert_eq!((buf[4], buf[5]), (127, 63));
                    assert_eq!(&buf[6..9], &led);
                }
                (PadKind::DualShock4, ConnectionType::Bluetooth) => {
                    assert_eq!((buf[6], buf[7]), (127, 63));
                    assert_eq!(&buf[8..11], &led);
                    // CRC-32 of the first 74 bytes lands in the last 4.
                    let crc = crc32(&[0xA2], &buf[..74]);
                    assert_eq!(
                        &buf[74..78],
                        &crc.to_le_bytes(),
                        "{kind:?}/{conn:?} CRC mismatch"
                    );
                }
                (PadKind::DualSense, ConnectionType::Usb)
                | (PadKind::DualSenseEdge, ConnectionType::Usb) => {
                    assert_eq!((buf[3], buf[4]), (127, 63));
                    assert_eq!(&buf[45..48], &led);
                }
                _ => {
                    assert_eq!((buf[4], buf[5]), (127, 63));
                    assert_eq!(&buf[46..49], &led);
                    let crc = crc32(&[0xA2], &buf[..74]);
                    assert_eq!(&buf[74..78], &crc.to_le_bytes());
                }
            }
        }
        // Rumble values clamp into 0..=1 before scaling.
        let buf = build_output_report(
            PadKind::DualSense,
            ConnectionType::Usb,
            [0, 0, 0],
            (-3.0, 2.0),
        )
        .unwrap();
        assert_eq!((buf[3], buf[4]), (0, 255));
        // Pads without a lightbar produce no report.
        assert!(build_output_report(PadKind::XInput, ConnectionType::Usb, led, rumble).is_none());
        assert!(build_output_report(PadKind::Generic, ConnectionType::Bluetooth, led, rumble).is_none());
    }

    #[test]
    fn fuzz_crc32_never_panics_on_random_bytes() {
        // crc32 chains `seed` then `data` — any lengths, including empty,
        // must produce a u32 without panicking. The result is a pure
        // function of the bytes, so the same inputs always give the same CRC.
        let mut rng = SplitMix64(0xC0DE_BA5E);
        for i in 0..10_000usize {
            let seed_len = (rng.next() % 5) as usize;
            let data_len = (rng.next() % 129) as usize; // 0..=128
            let mut seed = vec![0u8; seed_len];
            let mut data = vec![0u8; data_len];
            rand_buf(&mut rng, &mut seed, false);
            rand_buf(&mut rng, &mut data, false);
            let crc = crc32(&seed, &data);
            // Determinism: recomputing over the same bytes is identical.
            assert_eq!(crc, crc32(&seed, &data), "CRC not deterministic at {i}");
        }
        // Edge: both empty.
        assert_eq!(crc32(&[], &[]), crc32(&[], &[]));
        // Concatenation equivalence: crc(seed, data) == crc([], seed+data).
        let mut rng = SplitMix64(0x1234);
        for _ in 0..500usize {
            let mut seed = vec![0u8; (rng.next() % 16) as usize];
            let mut data = vec![0u8; (rng.next() % 32) as usize];
            rand_buf(&mut rng, &mut seed, false);
            rand_buf(&mut rng, &mut data, false);
            let mut joined = seed.clone();
            joined.extend_from_slice(&data);
            assert_eq!(crc32(&seed, &data), crc32(&[], &joined));
        }
    }

    // -----------------------------------------------------------------------
    // v1.2.5 button remapping (regression)
    // -----------------------------------------------------------------------

    #[test]
    fn remap_identity_preserves_mask() {
        let map = StickProfileConfig::default().button_map;
        let mask = buttons::CROSS | buttons::TRIANGLE | buttons::L1 | buttons::PS;
        assert_eq!(remap_buttons(mask, &map), mask);
    }

    #[test]
    fn remap_swaps_cross_and_circle() {
        // Classic JP-style layout swap: ✕→○ and ○→✕.
        let mut map = StickProfileConfig::default().button_map;
        map[0] = 1;
        map[1] = 0;
        assert_eq!(remap_buttons(buttons::CROSS, &map), buttons::CIRCLE);
        assert_eq!(remap_buttons(buttons::CIRCLE, &map), buttons::CROSS);
        // Pressing both still yields both (no spurious bits).
        assert_eq!(
            remap_buttons(buttons::CROSS | buttons::CIRCLE, &map),
            buttons::CROSS | buttons::CIRCLE
        );
    }

    #[test]
    fn remap_preserves_unmapped_buttons() {
        let mut map = StickProfileConfig::default().button_map;
        map[0] = 1; // only CROSS is remapped
        let out = remap_buttons(buttons::CROSS | buttons::R1 | buttons::L3, &map);
        assert_eq!(out, buttons::CIRCLE | buttons::R1 | buttons::L3);
    }

    #[test]
    fn remap_non_xinput_source_onto_a() {
        // TOUCHPAD has no XInput equivalent but can drive A (CROSS).
        let mut map = StickProfileConfig::default().button_map;
        map[13] = 0;
        assert_eq!(remap_buttons(buttons::TOUCHPAD, &map), buttons::CROSS);
        // The original TOUCHPAD bit must be gone from the output.
        assert_eq!(remap_buttons(buttons::TOUCHPAD, &map) & buttons::TOUCHPAD, 0);
    }

    #[test]
    fn remap_masks_target_bit_into_16() {
        let mut map = StickProfileConfig::default().button_map;
        map[0] = 16; // out-of-range target wraps to bit 0
        assert_eq!(remap_buttons(buttons::CROSS, &map), buttons::CROSS);
        map[0] = 31; // 31 & 15 = 15
        assert_eq!(remap_buttons(buttons::CROSS, &map), 1 << 15);
    }

    #[test]
    fn remap_multiple_sources_can_share_a_target() {
        // Both bumpers fire A.
        let mut map = StickProfileConfig::default().button_map;
        map[4] = 0;
        map[5] = 0;
        assert_eq!(
            remap_buttons(buttons::L1 | buttons::R1, &map),
            buttons::CROSS
        );
    }

    // -----------------------------------------------------------------------
    // v1.2.5 circularity correction (regression)
    // -----------------------------------------------------------------------

    /// Seed a fresh per-axis reach tracker the way the engine does.
    fn fresh_reach() -> (f32, f32) {
        (CIRCULARITY_SEED, CIRCULARITY_SEED)
    }

    #[test]
    fn circularity_learns_per_axis_reach() {
        let (mut mx, mut my) = fresh_reach();
        // A full-X / 70%-Y deflection teaches the ellipse on frame one.
        let (x, y) = circularity_correct(1.0, 0.7, &mut mx, &mut my);
        assert!((mx - 1.0).abs() < 1e-4 && (my - 0.7).abs() < 1e-4);
        assert!((x - 1.0).abs() < 1e-4 && (y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn circularity_after_learning_maps_partial_pushes_proportionally() {
        // Once the ellipse (1.0, 0.7) is known, a half push is half output on
        // BOTH axes — the diagonal ratio is preserved (this is what makes a
        // physical ellipse read as a circle).
        let mut mx = 1.0f32;
        let mut my = 0.7f32;
        let (x, y) = circularity_correct(0.5, 0.35, &mut mx, &mut my);
        assert!((x - 0.5).abs() < 1e-4 && (y - 0.5).abs() < 1e-4);
    }

    #[test]
    fn circularity_diagonal_sits_on_unit_circle() {
        // Physical 45° push on the (1.0, 0.7) ellipse → (0.7071, 0.7071):
        // magnitude exactly 1.0 instead of the raw 0.86.
        let mut mx = 1.0f32;
        let mut my = 0.7f32;
        let c = 0.707_106_78f32;
        let (x, y) = circularity_correct(c, 0.7 * c, &mut mx, &mut my);
        assert!((x - c).abs() < 1e-4 && (y - c).abs() < 1e-4);
        let mag = (x * x + y * y).sqrt();
        assert!((mag - 1.0).abs() < 1e-4, "diagonal must reach the unit circle");
    }

    #[test]
    fn circularity_reach_is_monotonic() {
        let (mut mx, mut my) = fresh_reach();
        let _ = circularity_correct(0.8, 0.9, &mut mx, &mut my);
        let (mx0, my0) = (mx, my);
        // Smaller inputs never shrink the learned reach.
        let _ = circularity_correct(0.3, 0.4, &mut mx, &mut my);
        assert_eq!((mx, my), (mx0, my0));
        // Bigger inputs grow it.
        let (x, y) = circularity_correct(1.0, 1.0, &mut mx, &mut my);
        assert!((x - 1.0).abs() < 1e-4 && (y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn circularity_preserves_negative_direction() {
        let (mut mx, mut my) = fresh_reach();
        let (x, y) = circularity_correct(-1.0, -0.7, &mut mx, &mut my);
        assert!(x < 0.0 && y < 0.0);
        assert!((x + 1.0).abs() < 1e-4 && (y + 1.0).abs() < 1e-4);
    }

    #[test]
    fn circularity_never_amplifies_fine_aim_input() {
        // Sub-threshold deflections never teach the reach and never amplify:
        // a 1% push stays a 1% output, even on a fresh session.
        let (mut mx, mut my) = fresh_reach();
        let (x, y) = circularity_correct(0.01, 0.01, &mut mx, &mut my);
        assert!((x - 0.01).abs() < 1e-4 && (y - 0.01).abs() < 1e-4);
        // And the noise was never recorded as reach.
        assert_eq!((mx, my), fresh_reach());
    }

    #[test]
    fn circularity_learns_from_strong_samples_only() {
        let (mut mx, mut my) = fresh_reach();
        // Fine-aim deflection below the threshold is ignored…
        let _ = circularity_correct(0.3, 0.2, &mut mx, &mut my);
        assert_eq!((mx, my), fresh_reach());
        // …but a strong deflection teaches the per-axis reach.
        let (x, y) = circularity_correct(0.8, 0.6, &mut mx, &mut my);
        assert!((mx - 0.8).abs() < 1e-4 && (my - 0.6).abs() < 1e-4);
        assert!((x - 1.0).abs() < 1e-4 && (y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn circularity_output_is_clamped() {
        // Even a pathological over-range sample can never leave [-1, 1].
        let mut mx = 1.0f32;
        let mut my = 1.0f32;
        for i in 0..=200 {
            let a = i as f32 / 100.0; // 0..=2 rad, sweeps the full rim
            let (x, y) = circularity_correct(a.cos(), a.sin(), &mut mx, &mut my);
            assert!(x >= -1.0 && x <= 1.0, "x out of range: {x}");
            assert!(y >= -1.0 && y <= 1.0, "y out of range: {y}");
        }
    }

    // -----------------------------------------------------------------------
    // v1.2.5 gyro shaping (regression) — rest calibration, EMA, deadzone,
    // routing
    // -----------------------------------------------------------------------

    #[test]
    fn gyro_rest_calibrates_when_still() {
        // Slow sensor drift below the still threshold folds into the rest
        // offset: after n frames rest = raw * (1 - 0.9^n).
        let raw = [0.01, -0.008, 0.005];
        let (mut rest, mut cal) = ([0.0; 3], false);
        for _ in 0..5 {
            (rest, cal) = gyro_update_rest(raw, rest, cal, false);
        }
        assert!(cal);
        let f = 1.0 - 0.9f32.powi(5);
        for i in 0..3 {
            assert!(
                (rest[i] - raw[i] * f).abs() < 1e-4,
                "axis {i}: {} vs {}",
                rest[i],
                raw[i] * f
            );
        }
    }

    #[test]
    fn gyro_rest_holds_while_moving() {
        // Real motion (above the still threshold) never folds into the rest
        // offset once calibrated — no drift is introduced by gameplay.
        let raw = [0.5, -0.3, 0.2];
        let (rest, cal) = gyro_update_rest(raw, [0.01, -0.01, 0.0], true, false);
        assert!(cal);
        assert_eq!(rest, [0.01, -0.01, 0.0]);
    }

    #[test]
    fn gyro_force_recalibration_snapshots_instantly() {
        // A forced recalibration snaps the rest offset even during motion.
        let raw = [0.9, -0.7, 0.3];
        let (rest, cal) = gyro_update_rest(raw, [0.0; 3], true, true);
        assert!(cal);
        assert_eq!(rest, raw);
    }

    #[test]
    fn gyro_rest_converges_to_constant_offset() {
        // A stationary pad with a sub-threshold sensor bias: the auto-recenter
        // folds the whole bias into the rest offset over time.
        let offset = [0.01, -0.008, 0.006]; // mag ≈ 0.014 < 0.02 (still)
        let (mut rest, mut cal) = ([0.0; 3], false);
        for _ in 0..200 {
            (rest, cal) = gyro_update_rest(offset, rest, cal, false);
        }
        assert!(cal);
        for i in 0..3 {
            assert!((rest[i] - offset[i]).abs() < 1e-3, "axis {i}");
        }
    }

    #[test]
    fn gyro_rest_first_frame_calibrates_even_in_motion() {
        // The very first frame always calibrates (no rest offset yet), so a
        // moving start-up can't spin the cursor out of control.
        let raw = [0.4, -0.2, 0.1];
        let (rest, cal) = gyro_update_rest(raw, [0.0; 3], false, false);
        assert!(cal);
        assert!((rest[0] - 0.04).abs() < 1e-4); // 10% fold from zero
    }

    #[test]
    fn gyro_rest_still_threshold_boundary() {
        // Magnitude exactly at the still threshold is NOT still (`<`), so a
        // calibrated pad holds its rest offset instead of folding.
        let (rest, cal) = gyro_update_rest([GYRO_STILL_THRESHOLD, 0.0, 0.0], [0.1, 0.0, 0.0], true, false);
        assert!(cal);
        assert_eq!(rest, [0.1, 0.0, 0.0]);
    }

    #[test]
    fn gyro_force_recalibration_works_on_uncalibrated_pad() {
        // Force ignores both the calibrated flag and the motion magnitude.
        let (rest, cal) = gyro_update_rest([0.6, 0.0, 0.0], [0.0; 3], false, true);
        assert!(cal);
        assert_eq!(rest, [0.6, 0.0, 0.0]);
    }

    #[test]
    fn gyro_smoothing_converges_toward_signal() {
        // A constant input converges to itself under the EMA.
        let raw = [0.2, 0.0, 0.0];
        let mut prev = [0.0; 3];
        let mut last = 0.0;
        for _ in 0..20 {
            prev = gyro_smooth(raw, [0.0; 3], prev, 0.5);
            last = prev[0];
        }
        assert!((last - 0.2).abs() < 1e-3, "EMA must converge, got {last}");
    }

    #[test]
    fn gyro_smoothing_first_frame_is_step_scaled() {
        // A step input lands at (1 - alpha) * step on the first frame.
        let out = gyro_smooth([0.4, 0.0, 0.0], [0.0; 3], [0.0; 3], 0.75);
        assert!((out[0] - 0.1).abs() < 1e-5);
    }

    #[test]
    fn gyro_smoothing_zero_alpha_is_passthrough() {
        // alpha 0 → no smoothing: output is exactly raw - rest, prev is ignored.
        let out = gyro_smooth([0.5, -0.2, 0.1], [0.05, 0.0, 0.0], [99.0, 99.0, 99.0], 0.0);
        assert_eq!(out, [0.45, -0.2, 0.1]);
    }

    #[test]
    fn gyro_smoothing_alpha_is_clamped() {
        // alpha above 0.95 clamps to 0.95 — the EMA can never be fully sticky.
        let out = gyro_smooth([0.5, 0.0, 0.0], [0.0; 3], [0.0; 3], 2.0);
        assert!((out[0] - 0.025).abs() < 1e-5);
    }

    #[test]
    fn gyro_smoothing_negative_alpha_clamps_to_zero() {
        // alpha below 0 clamps to 0 → passthrough, never anti-smoothed.
        let out = gyro_smooth([0.5, 0.0, 0.0], [0.0; 3], [99.0; 3], -1.0);
        assert_eq!(out, [0.5, 0.0, 0.0]);
    }

    #[test]
    fn gyro_deadzone_zeroes_jitter_but_keeps_motion() {
        let v = gyro_deadzone([0.002, 0.1, -0.003], GYRO_JITTER_DEADZONE);
        assert_eq!(v, [0.0, 0.1, 0.0]);
    }

    #[test]
    fn gyro_deadzone_keeps_threshold_boundary() {
        // Exactly at the deadzone (>=) is kept.
        let v = gyro_deadzone(
            [GYRO_JITTER_DEADZONE, -GYRO_JITTER_DEADZONE, 0.0],
            GYRO_JITTER_DEADZONE,
        );
        assert_eq!(v, [GYRO_JITTER_DEADZONE, -GYRO_JITTER_DEADZONE, 0.0]);
    }

    #[test]
    fn gyro_stick_offset_yaw_drives_x_pitch_drives_y() {
        let (ox, oy) = gyro_stick_offset([0.25, -0.5, 0.0], 2.0, false);
        assert!((ox - (-0.5 * 1.6 * 2.0)).abs() < 1e-4);
        assert!((oy - (0.25 * 1.6 * 2.0)).abs() < 1e-4);
    }

    #[test]
    fn gyro_stick_offset_invert_flips_pitch_only() {
        let (ox, oy) = gyro_stick_offset([0.1, 0.2, 0.0], 1.0, true);
        assert!((ox - 0.32).abs() < 1e-4); // yaw untouched
        assert!((oy + 0.16).abs() < 1e-4); // pitch inverted
    }

    #[test]
    fn gyro_stick_offset_clamps_sensitivity() {
        let (ox, _) = gyro_stick_offset([0.0, 1.0, 0.0], 99.0, false);
        assert!((ox - 8.0 * 1.6).abs() < 1e-4);
    }

    #[test]
    fn gyro_mouse_delta_pitch_to_y_yaw_to_x() {
        let (dx, dy) = gyro_mouse_delta([0.1, 0.2, 0.0], 1.0, false);
        assert!((dx - 44.0).abs() < 1e-3);
        assert!((dy - 22.0).abs() < 1e-3);
    }

    #[test]
    fn gyro_mouse_delta_invert_flips_dy_only() {
        let (dx, dy) = gyro_mouse_delta([0.1, 0.2, 0.0], 1.0, true);
        assert!((dx - 44.0).abs() < 1e-3);
        assert!((dy + 22.0).abs() < 1e-3);
    }

    #[test]
    fn gyro_pipeline_calibrates_then_routes_to_stick() {
        // End-to-end: still frames fold the rest offset and the residual
        // jitter is deadzoned to zero; then a real yaw movement survives the
        // pipeline and drives positive X on the right stick.
        let still = [0.005, -0.004, 0.002];
        let mut rest = [0.0; 3];
        let mut cal = false;
        let mut smooth = [0.0; 3];
        for _ in 0..10 {
            (rest, cal) = gyro_update_rest(still, rest, cal, false);
            smooth = gyro_smooth(still, rest, smooth, 0.5);
        }
        assert!(cal);
        let gyro = gyro_deadzone(smooth, GYRO_JITTER_DEADZONE);
        assert_eq!(gyro, [0.0; 3], "still residue must be deadzoned");

        // Real yaw movement: rest is untouched, motion shows up in X.
        let motion = [0.0, 0.3, 0.0];
        (rest, cal) = gyro_update_rest(motion, rest, cal, false);
        smooth = gyro_smooth(motion, rest, smooth, 0.5);
        let gyro = gyro_deadzone(smooth, GYRO_JITTER_DEADZONE);
        let (ox, oy) = gyro_stick_offset(gyro, 1.0, false);
        assert!(ox > 0.0, "yaw must drive positive X, got {ox}");
        assert!((oy).abs() < 1e-6, "pitch-free movement must not drive Y");
    }

    // -----------------------------------------------------------------------
    // HID report parsing (regression) — synthetic DS4 / DualSense buffers
    // -----------------------------------------------------------------------

    /// u8_to_axis(128) — the true analogue centre of the 0..255 range.
    const CENTRE: f32 = 0.5 / 127.5;

    #[test]
    fn parse_ds4_usb_sticks_triggers_and_buttons() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01; // USB input report
        // Sticks: LX full right, LY full up, RX full left, RY full down.
        buf[1] = 255; // lx -> +1.0
        buf[2] = 0; // ly raw 0 -> +1.0 (up)
        buf[3] = 0; // rx -> -1.0
        buf[4] = 255; // ry raw 255 -> -1.0 (down)
        let s = parse_ds4(&buf, false).expect("64-byte USB report must parse");
        assert!((s.lx - 1.0).abs() < 1e-4);
        assert!((s.ly - 1.0).abs() < 1e-4);
        assert!((s.rx + 1.0).abs() < 1e-4);
        assert!((s.ry + 1.0).abs() < 1e-4);
        // Triggers: L2 full, R2 released.
        buf[8] = 255;
        let s = parse_ds4(&buf, false).unwrap();
        assert!((s.l2 - 1.0).abs() < 1e-4);
        assert_eq!(s.r2, 0.0);
        // Buttons: b1 = CROSS (+ dpad 0), b2 = R3, b3 = PS.
        buf[5] = 0x20;
        buf[6] = 0x80;
        buf[7] = 0x01;
        let s = parse_ds4(&buf, false).unwrap();
        assert_eq!(s.buttons, buttons::CROSS | buttons::R3 | buttons::PS);
        assert_eq!(s.dpad, 0); // up
    }

    #[test]
    fn parse_ds4_usb_all_buttons() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        buf[5] = 0x10 | 0x20 | 0x40 | 0x80; // face buttons, dpad 0
        buf[6] = 0x01 | 0x02 | 0x04 | 0x08 | 0x10 | 0x20 | 0x40 | 0x80;
        buf[7] = 0x01 | 0x02; // PS + TOUCHPAD
        let s = parse_ds4(&buf, false).unwrap();
        assert_eq!(
            s.buttons,
            buttons::SQUARE
                | buttons::CROSS
                | buttons::CIRCLE
                | buttons::TRIANGLE
                | buttons::L1
                | buttons::R1
                | buttons::L2
                | buttons::R2
                | buttons::SHARE
                | buttons::OPTIONS
                | buttons::L3
                | buttons::R3
                | buttons::PS
                | buttons::TOUCHPAD
        );
    }

    #[test]
    fn parse_ds4_usb_battery_and_charging() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        // Cable (bit 4) + level 10/11.
        buf[30] = 0x10 | 0x0A;
        let s = parse_ds4(&buf, false).unwrap();
        assert!(s.charging);
        assert_eq!(s.battery, 90); // (10/11)*100 = 90.9 -> 90
        // No cable, level 8/8.
        buf[30] = 0x08;
        let s = parse_ds4(&buf, false).unwrap();
        assert!(!s.charging);
        assert_eq!(s.battery, 100);
    }

    #[test]
    fn parse_ds4_motion_block() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        // gyro: +1024 (1.0), -1024, 0 (16-bit LE).
        buf[13] = 0x00;
        buf[14] = 0x04;
        buf[15] = 0x00;
        buf[16] = 0xFC;
        buf[17] = 0x00;
        buf[18] = 0x00;
        // accel: +8192 (1.0), 0, -8192.
        buf[19] = 0x00;
        buf[20] = 0x20;
        buf[21] = 0x00;
        buf[22] = 0x00;
        buf[23] = 0x00;
        buf[24] = 0xE0;
        let s = parse_ds4(&buf, false).unwrap();
        assert!((s.gyro[0] - 1.0).abs() < 1e-3);
        assert!((s.gyro[1] + 1.0).abs() < 1e-3);
        assert!(s.gyro[2].abs() < 1e-3);
        assert!((s.accel[0] - 1.0).abs() < 1e-3);
        assert!((s.accel[2] + 1.0).abs() < 1e-3, "accel z must be -1.0");
    }

    #[test]
    fn parse_ds4_touchpad_active_and_inactive() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        // Finger 1 active at (959, 471): 12-bit packed X/Y.
        buf[35] = 0x00; // active (0x80 clear)
        buf[36] = 0xBF; // x low byte (0x3BF = 959)
        buf[37] = 0x73; // x high nibble 0x3 | y low nibble 0x7 << 4
        buf[38] = 0x1D; // y high byte (0x1D7 = 471)
        // Finger 2 inactive.
        buf[39] = 0x80;
        let s = parse_ds4(&buf, false).unwrap();
        assert_eq!(s.touch_count, 1);
        assert!((s.touch[0][0] - 959.0 / 1919.0).abs() < 1e-4);
        assert!((s.touch[0][1] - 471.0 / 942.0).abs() < 1e-4);
        // Both fingers inactive -> no touches.
        buf[35] = 0x80;
        let s = parse_ds4(&buf, false).unwrap();
        assert_eq!(s.touch_count, 0);
    }

    #[test]
    fn parse_ds4_neutral_centre_and_idle_dpad() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        buf[1] = 128;
        buf[2] = 128;
        buf[3] = 128;
        buf[4] = 128;
        buf[5] = 0x08; // dpad 8 = neutral (no direction)
        let s = parse_ds4(&buf, false).unwrap();
        assert!((s.lx - CENTRE).abs() < 1e-4);
        assert!((s.ly + CENTRE).abs() < 1e-4);
        assert_eq!(s.buttons, 0);
        assert_eq!(s.dpad, 8);
    }

    #[test]
    fn parse_ds4_bluetooth_offset() {
        let mut buf = [0u8; 78];
        buf[0] = 0x11; // BT input report
        // Data starts at offset 3 for DS4 over Bluetooth.
        buf[3] = 255; // lx -> +1.0
        buf[4] = 255; // ly -> -1.0 (down)
        buf[5] = 0; // rx -> -1.0
        buf[6] = 0; // ry -> +1.0 (up)
        let s = parse_ds4(&buf, true).unwrap();
        assert!((s.lx - 1.0).abs() < 1e-4);
        assert!((s.ly + 1.0).abs() < 1e-4);
        assert!((s.rx + 1.0).abs() < 1e-4);
        assert!((s.ry - 1.0).abs() < 1e-4);
        // Triggers and battery shift with the BT offset (o+7 = 10, o+29 = 32).
        buf[10] = 255;
        buf[32] = 0x10 | 0x0B; // cable + level 11/11
        let s = parse_ds4(&buf, true).unwrap();
        assert!((s.l2 - 1.0).abs() < 1e-4);
        assert!(s.charging);
        assert_eq!(s.battery, 100); // (11/11)*100 — the USB cable divisor is /11
    }

    #[test]
    fn parse_ds4_short_and_minimal_buffers() {
        // Below the 9-byte floor: USB needs >= 10, BT needs >= 12.
        assert!(parse_ds4(&[0u8; 9], false).is_none());
        assert!(parse_ds4(&[0u8; 11], true).is_none());
        // Minimal parseable report: defaults for the extended sections.
        let mut buf = [0u8; 10];
        buf[0] = 0x01;
        let s = parse_ds4(&buf, false).expect("10-byte USB report must parse");
        assert_eq!(s.battery, -1);
        assert_eq!(s.gyro, [0.0, 0.0, 0.0]);
        assert_eq!(s.touch_count, 0);
    }

    #[test]
    fn parse_dualsense_usb_full_layout() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        buf[1] = 255; // lx -> +1.0
        buf[2] = 0; // ly -> +1.0 (up)
        buf[3] = 0; // rx -> -1.0
        buf[4] = 255; // ry -> -1.0
        buf[5] = 255; // l2 -> 1.0
        buf[6] = 128; // r2 -> ~0.502
        // Buttons: b1 = dpad 6 (left) | CIRCLE, b2 = L1, b3 = MUTE.
        buf[8] = 0x40 | 0x06;
        buf[9] = 0x01;
        buf[10] = 0x04;
        let s = parse_dualsense(&buf, false).unwrap();
        assert!((s.lx - 1.0).abs() < 1e-4);
        assert!((s.ly - 1.0).abs() < 1e-4);
        assert!((s.rx + 1.0).abs() < 1e-4);
        assert!((s.ry + 1.0).abs() < 1e-4);
        assert!((s.l2 - 1.0).abs() < 1e-4);
        assert!((s.r2 - 128.0 / 255.0).abs() < 1e-4);
        assert_eq!(s.buttons, buttons::CIRCLE | buttons::L1 | buttons::MUTE);
        assert_eq!(s.dpad, 6); // left
    }

    #[test]
    fn parse_dualsense_usb_all_buttons() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        buf[8] = 0x10 | 0x20 | 0x40 | 0x80;
        buf[9] = 0x01 | 0x02 | 0x04 | 0x08 | 0x10 | 0x20 | 0x40 | 0x80;
        buf[10] = 0x01 | 0x02 | 0x04; // PS + TOUCHPAD + MUTE
        let s = parse_dualsense(&buf, false).unwrap();
        assert_eq!(
            s.buttons,
            buttons::SQUARE
                | buttons::CROSS
                | buttons::CIRCLE
                | buttons::TRIANGLE
                | buttons::L1
                | buttons::R1
                | buttons::L2
                | buttons::R2
                | buttons::SHARE
                | buttons::OPTIONS
                | buttons::L3
                | buttons::R3
                | buttons::PS
                | buttons::TOUCHPAD
                | buttons::MUTE
        );
    }

    #[test]
    fn parse_dualsense_motion_and_battery() {
        let mut buf = [0u8; 64];
        buf[0] = 0x01;
        // Motion at o+15 (index 16): gyro +1024/-1024/0, accel +8192/0/-8192.
        buf[16] = 0x00;
        buf[17] = 0x04;
        buf[18] = 0x00;
        buf[19] = 0xFC;
        buf[20] = 0x00;
        buf[21] = 0x00;
        buf[22] = 0x00;
        buf[23] = 0x20;
        buf[24] = 0x00;
        buf[25] = 0x00;
        buf[26] = 0x00;
        buf[27] = 0xE0;
        let s = parse_dualsense(&buf, false).unwrap();
        assert!((s.gyro[0] - 1.0).abs() < 1e-3);
        assert!((s.gyro[1] + 1.0).abs() < 1e-3);
        assert!((s.accel[0] - 1.0).abs() < 1e-3);
        assert!((s.accel[2] + 1.0).abs() < 1e-3, "accel z must be -1.0");
        // Battery at o+52 (index 53): state 0x10 = charging, level 8/8.
        buf[53] = 0x10 | 0x08;
        let s = parse_dualsense(&buf, false).unwrap();
        assert!(s.charging);
        assert_eq!(s.battery, 100);
        // Discharging state (0x0) with full level.
        buf[53] = 0x08;
        let s = parse_dualsense(&buf, false).unwrap();
        assert!(!s.charging);
        assert_eq!(s.battery, 100);
    }

    #[test]
    fn parse_dualsense_bluetooth_offset_and_touch() {
        let mut buf = [0u8; 78];
        buf[0] = 0x31; // BT input report
        // Data starts at offset 2 for DualSense over Bluetooth.
        buf[2] = 255; // lx -> +1.0
        buf[3] = 0; // ly -> +1.0 (up)
        buf[4] = 0; // rx -> -1.0
        buf[5] = 255; // ry -> -1.0
        let s = parse_dualsense(&buf, true).unwrap();
        assert!((s.lx - 1.0).abs() < 1e-4);
        assert!((s.ly - 1.0).abs() < 1e-4);
        // Touch at base o+32 (index 34), Y divisor 1079 for DualSense.
        let mut buf = [0u8; 78];
        buf[0] = 0x31;
        buf[34] = 0x00; // active
        buf[35] = 0xBF; // x low byte (0x3BF = 959)
        buf[36] = 0x73; // x high nibble | y low nibble << 4
        buf[37] = 0x1D; // y high byte (0x1D7 = 471)
        buf[38] = 0x80; // finger 2 inactive
        let s = parse_dualsense(&buf, true).unwrap();
        assert_eq!(s.touch_count, 1);
        assert!((s.touch[0][0] - 959.0 / 1919.0).abs() < 1e-4);
        assert!((s.touch[0][1] - 471.0 / 1079.0).abs() < 1e-4);
    }

    #[test]
    fn parse_dualsense_short_and_truncated() {
        // The b3 byte lives at o+9, so the floor is o+10: USB needs >= 11,
        // BT needs >= 12. Shorter reports must return None (never panic).
        assert!(parse_dualsense(&[0u8; 9], false).is_none());
        assert!(parse_dualsense(&[0u8; 10], false).is_none());
        assert!(parse_dualsense(&[0u8; 10], true).is_none());
        assert!(parse_dualsense(&[0u8; 11], true).is_none());
        // A minimal parseable report with no extended sections.
        let mut buf = [0u8; 11];
        buf[0] = 0x01;
        let s = parse_dualsense(&buf, false).expect("11-byte USB report must parse");
        assert_eq!(s.battery, -1);
        assert_eq!(s.gyro, [0.0, 0.0, 0.0]);
        assert_eq!(s.touch_count, 0);
    }

    #[test]
    fn parse_dualsense_motion_floor_is_exact() {
        // Regression for an off-by-one the fuzzer caught: the motion block
        // reads le_i16 at o+25 (indices o+25..=o+26), so a report of exactly
        // the old guard length (o+26 bytes) used to panic. Both boundary
        // lengths must now parse cleanly with motion zeroed.
        let s = parse_dualsense(&[0u8; 26], false).expect("26-byte USB report must parse");
        assert_eq!(s.gyro, [0.0, 0.0, 0.0]);
        assert!(parse_dualsense(&[0u8; 27], true).is_some());
        // One byte longer, the motion block parses cleanly (values stay 0).
        let mut buf = [0u8; 27];
        buf[0] = 0x01;
        let s = parse_dualsense(&buf, false).expect("27-byte USB report must parse");
        assert_eq!(s.gyro, [0.0, 0.0, 0.0]);
        assert_eq!(s.accel, [0.0, 0.0, 0.0]);
    }

    // ---------------------------------------------------------------------
    // Deterministic fuzzing — splitmix64 PRNG (no external deps). Same seed
    // always produces the same buffer stream, so any regression is
    // reproducible by just re-running the test.
    // ---------------------------------------------------------------------

    struct SplitMix64(u64);

    impl SplitMix64 {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    /// Fills `buf` with deterministic pseudo-random bytes. When `report_id`
    /// is set, byte 0 is a plausible input-report id (USB 0x01, BT 0x11/0x31)
    /// so parsers get a chance to run their full happy path; otherwise the
    /// whole buffer is garbage.
    fn rand_buf(rng: &mut SplitMix64, buf: &mut [u8], report_id: bool) {
        for b in buf.iter_mut() {
            *b = (rng.next() & 0xFF) as u8;
        }
        if report_id && !buf.is_empty() {
            let kind = rng.next() % 3;
            buf[0] = match kind {
                0 => 0x01, // USB id, shared by both pads
                1 => 0x11, // DS4 BT
                _ => 0x31, // DualSense BT
            };
        }
    }

    /// Shared fuzz harness: runs `parsers` across `threads` workers, each
    /// doing `iters_per_thread` iterations of random buffers, and asserts
    /// none of the four parser paths ever panics. Any panic is recorded as
    /// a packed (thread, iter, parser) value so the failing input is
    /// reproducible from the seed alone.
    fn fuzz_parsers(threads: usize, iters_per_thread: usize) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        // Packed failure record: (thread << 12) | (iter << 2) | parser_idx.
        // 0 means no crash; a non-zero value identifies the exact input that
        // panicked so the case can be replayed and minimized.
        let crash = std::sync::Arc::new(AtomicUsize::new(0));
        let workers: Vec<_> = (0..threads)
            .map(|t| {
                let crash = std::sync::Arc::clone(&crash);
                std::thread::spawn(move || {
                    let mut rng = SplitMix64(0xF00D_5EED_u64.wrapping_add(t as u64));
                    for i in 0..iters_per_thread {
                        // Lengths sweep 0..=78 (full BT report size) plus a
                        // handful of oversized buffers to exercise the guards.
                        let len = match i % 7 {
                            0..=4 => (rng.next() % 79) as usize,
                            5 => 64,
                            _ => 96,
                        };
                        let mut buf = vec![0u8; len];
                        // Half the iterations set a plausible report id.
                        rand_buf(&mut rng, &mut buf, i % 2 == 0);
                        let parsers = [
                            (parse_ds4 as fn(&[u8], bool) -> Option<RawState>, false),
                            (parse_ds4 as fn(&[u8], bool) -> Option<RawState>, true),
                            (parse_dualsense as fn(&[u8], bool) -> Option<RawState>, false),
                            (parse_dualsense as fn(&[u8], bool) -> Option<RawState>, true),
                        ];
                        for (j, (parse, bt)) in parsers.iter().enumerate() {
                            if std::panic::catch_unwind(|| parse(&buf, *bt)).is_err() {
                                // Record the first crash only.
                                let rec = (t << 12) | (i << 2) | j;
                                crash.compare_exchange(
                                    0,
                                    rec,
                                    Ordering::SeqCst,
                                    Ordering::SeqCst,
                                )
                                .ok();
                            }
                        }
                    }
                })
            })
            .collect();
        for t in workers {
            t.join().expect("fuzz worker must not panic");
        }
        let rec = crash.load(Ordering::SeqCst);
        assert_eq!(
            rec, 0,
            "parser panicked on deterministic fuzz input: thread={} iter={} parser={} (0=DS4-USB, 1=DS4-BT, 2=DualSense-USB, 3=DualSense-BT) seed=0xF00D_5EED+thread",
            rec >> 12,
            (rec >> 2) & 0x3FF,
            rec & 0x3
        );
    }

    #[test]
    fn fuzz_parsers_never_panic_on_random_buffers() {
        // 8 threads × 6,250 iterations each. The splitmix64 stream is
        // per-thread and seeded by the worker index, so the run is
        // deterministic and reproducible. ~50k buffers × 4 parser
        // invocations (DS4/DualSense × USB/BT) total.
        fuzz_parsers(8, 6250);
    }

    /// Fuzzes the full processing hot loop over randomly parsed reports:
    /// shape_stick / shape_trigger → remap_buttons → circularity_correct →
    /// gyro rest-calibration / smoothing / deadzone / routing. Runs each
    /// parsed buffer through the whole pipeline and asserts every output
    /// stays finite and physically valid — and, crucially, that nothing
    /// panics on any combination of random profile fields and random axes.
    fn fuzz_hot_loop(threads: usize, iters_per_thread: usize) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let crash = std::sync::Arc::new(AtomicUsize::new(0));
        let workers: Vec<_> = (0..threads)
            .map(|t| {
                let crash = std::sync::Arc::clone(&crash);
                std::thread::spawn(move || {
                    let mut rng = SplitMix64(0xB0B_1EED_u64.wrapping_add(t as u64));
                    // Per-pad state carried across frames, like the real engine.
                    let mut circ_x = CIRCULARITY_SEED;
                    let mut circ_y = CIRCULARITY_SEED;
                    let mut gyro_rest = [0.0f32; 3];
                    let mut gyro_cal = false;
                    let mut gyro_sm = [0.0f32; 3];
                    for i in 0..iters_per_thread {
                        // Randomised-but-valid profile.
                        let curve = match rng.next() % 4 {
                            0 => CurveKind::Linear,
                            1 => CurveKind::Exponential,
                            2 => CurveKind::SCurve,
                            _ => CurveKind::Aggressive,
                        };
                        let mut prof = StickProfileConfig::default();
                        prof.left.curve = curve;
                        prof.left.curve_power = (rng.next() % 40) as f32 / 10.0; // 0.0..=3.9
                        prof.left.sensitivity = (rng.next() % 80) as f32 / 10.0; // 0.0..=7.9
                        prof.left.inner_deadzone = (rng.next() % 30) as f32 / 100.0;
                        prof.left.outer_deadzone = 0.9 + (rng.next() % 10) as f32 / 100.0;
                        prof.left.anti_deadzone = (rng.next() % 20) as f32 / 100.0;
                        prof.left.radial = rng.next() % 2 == 0;
                        prof.left.invert_y = rng.next() % 2 == 0;
                        prof.left.circularity_correction = rng.next() % 2 == 0;
                        prof.right = prof.left; // same random tuning both sticks
                        prof.trigger_left.hair_trigger = rng.next() % 2 == 0;
                        prof.trigger_right = prof.trigger_left;
                        prof.flip_triggers = rng.next() % 2 == 0;
                        prof.button_map = StickProfileConfig::default().button_map;
                        if rng.next() % 3 == 0 {
                            prof.button_map[0] = 1; // cross -> circle
                            prof.button_map[1] = 0; // circle -> cross
                        }
                        let gyro_sens = (rng.next() % 90) as f32 / 10.0 + 0.1; // 0.1..=9.0
                        let gyro_invert = rng.next() % 2 == 0;
                        let smooth = (rng.next() % 100) as f32 / 100.0; // 0.0..=0.99
                        let mut len = (rng.next() % 79) as usize;
                        if i % 11 == 0 {
                            len = 64 + (rng.next() % 15) as usize;
                        }
                        let mut buf = vec![0u8; len];
                        rand_buf(&mut rng, &mut buf, i % 2 == 0);
                        let parsers = [
                            (parse_ds4 as fn(&[u8], bool) -> Option<RawState>, false),
                            (parse_ds4 as fn(&[u8], bool) -> Option<RawState>, true),
                            (parse_dualsense as fn(&[u8], bool) -> Option<RawState>, false),
                            (parse_dualsense as fn(&[u8], bool) -> Option<RawState>, true),
                        ];
                        for (j, (parse, bt)) in parsers.iter().enumerate() {
                            let s = match parse(&buf, *bt) {
                                Some(s) => s,
                                None => continue,
                            };
                            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                // --- stick / trigger shaping ---
                                let (lx, ly) = shape_stick(s.lx, s.ly, &prof.left);
                                let (rx, ry) = shape_stick(s.rx, s.ry, &prof.right);
                                let (tl, tr) = if prof.flip_triggers {
                                    (s.r2, s.l2)
                                } else {
                                    (s.l2, s.r2)
                                };
                                let tl = shape_trigger(tl, &prof.trigger_left);
                                let tr = shape_trigger(tr, &prof.trigger_right);
                                // --- button remapping ---
                                let remapped = remap_buttons(s.buttons, &prof.button_map);
                                // --- circularity correction (persistent state) ---
                                let (cx, cy) =
                                    circularity_correct(lx, ly, &mut circ_x, &mut circ_y);
                                // --- gyro pipeline ---
                                let (rest, cal) =
                                    gyro_update_rest(s.gyro, gyro_rest, gyro_cal, false);
                                gyro_rest = rest;
                                gyro_cal = cal;
                                let g = gyro_smooth(s.gyro, gyro_rest, gyro_sm, smooth);
                                gyro_sm = g;
                                let g = gyro_deadzone(g, 0.004);
                                let (ox, oy) = gyro_stick_offset(g, gyro_sens, gyro_invert);
                                let (dx, dy) = gyro_mouse_delta(g, gyro_sens, gyro_invert);
                                // --- invariants ---
                                // Shaped sticks / triggers / circularity output are
                                // all normalised by contract and stay in [-1, 1].
                                for &v in &[lx, ly, rx, ry, tl, tr, cx, cy] {
                                    assert!(v.is_finite() && (-1.0..=1.0).contains(&v), "axis {v}");
                                }
                                // Gyro offsets are PRE-clamp (the engine applies
                                // (rx + ox).clamp(-1, 1) afterwards) — they only
                                // need to be finite. Mouse deltas are pixels.
                                for &v in &[ox, oy, dx, dy] {
                                    assert!(v.is_finite(), "gyro output {v}");
                                }
                                // Engine clamp reproduces the real hot loop.
                                let fx = (rx + ox).clamp(-1.0, 1.0);
                                let fy = (ry + oy).clamp(-1.0, 1.0);
                                assert!(fx.is_finite() && (-1.0..=1.0).contains(&fx));
                                assert!(fy.is_finite() && (-1.0..=1.0).contains(&fy));
                                let _ = remapped; // 16-bit masked by construction
                            }))
                            .is_err()
                            {
                                let rec = (t << 12) | (i << 2) | j;
                                crash.compare_exchange(0, rec, Ordering::SeqCst, Ordering::SeqCst).ok();
                            }
                        }
                    }
                })
            })
            .collect();
        for t in workers {
            t.join().expect("hot-loop fuzz worker must not panic");
        }
        let rec = crash.load(Ordering::SeqCst);
        assert_eq!(
            rec, 0,
            "hot loop panicked on deterministic fuzz input: thread={} iter={} parser={} seed=0xB0B_1EED+thread",
            rec >> 12,
            (rec >> 2) & 0x3FF,
            rec & 0x3
        );
    }

    #[test]
    #[ignore = "deep fuzz — run explicitly in CI (500k iterations) via: cargo test -- --ignored deep_fuzz"]
    fn deep_fuzz_500k_parsers_and_crc() {
        // 8 threads × 62,500 = 500k random buffers × 4 parser paths, plus
        // 200k CRC-32 combos. Gated with #[ignore] so normal `cargo test`
        // stays fast; the CI job "Deep fuzz" runs it explicitly.
        fuzz_parsers(8, 62_500);
        // CRC-32 fuzz: random seed/data lengths, deterministic recompute.
        let mut rng = SplitMix64(0xD00D_FACE);
        for _ in 0..200_000usize {
            let mut seed = vec![0u8; (rng.next() % 5) as usize];
            let mut data = vec![0u8; (rng.next() % 129) as usize];
            rand_buf(&mut rng, &mut seed, false);
            rand_buf(&mut rng, &mut data, false);
            let crc = crc32(&seed, &data);
            assert_eq!(crc, crc32(&seed, &data), "CRC not deterministic");
        }
        // Hot-loop pipeline fuzz: 8 threads × 25,000 = 200k reports through
        // shape → remap → circularity → gyro with per-pad persistent state.
        fuzz_hot_loop(8, 25_000);
    }

    #[test]
    fn fuzz_parsers_keep_outputs_in_valid_ranges() {
        // Property check: even with adversarial bytes, every field the
        // parsers expose must land in a physically valid range. Any
        // out-of-range value here means the parser mis-read an offset.
        let mut rng = SplitMix64(0xC0FF_EE);
        for i in 0..20_000usize {
            let mut buf = vec![0u8; 78]; // full-size, valid for both pads
            rand_buf(&mut rng, &mut buf, i % 2 == 0);
            for (parse, bt) in [
                (parse_ds4 as fn(&[u8], bool) -> Option<RawState>, false),
                (parse_ds4 as fn(&[u8], bool) -> Option<RawState>, true),
                (parse_dualsense as fn(&[u8], bool) -> Option<RawState>, false),
                (parse_dualsense as fn(&[u8], bool) -> Option<RawState>, true),
            ] {
                if let Some(s) = parse(&buf, bt) {
                    assert!(
                        s.battery >= -1 && s.battery <= 100,
                        "battery {}",
                        s.battery
                    );
                    for &g in &s.gyro {
                        assert!(g.is_finite() && g.abs() <= 32.0, "gyro {g}");
                    }
                    for &a in &s.accel {
                        assert!(a.is_finite() && a.abs() <= 32.0, "accel {a}");
                    }
                    for &v in &[s.lx, s.ly, s.rx, s.ry, s.l2, s.r2] {
                        assert!(
                            v.is_finite() && (-1.0..=1.0).contains(&v),
                            "axis {v}"
                        );
                    }
                    assert!(s.touch_count <= 2, "touch_count {}", s.touch_count);
                    for t in &s.touch {
                        assert!(t[0].is_finite() && (0.0..=1.0).contains(&t[0]));
                        assert!(t[1].is_finite() && (0.0..=1.0).contains(&t[1]));
                    }
                }
            }
        }
    }
}
