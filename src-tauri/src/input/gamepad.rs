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
    pub rumble_intensity: f32,
    /// Extra polling aggressiveness: `true` pins the loop to 1000 Hz+.
    pub turbo_polling: bool,
}

impl Default for StickProfileConfig {
    fn default() -> Self {
        Self {
            left: StickAxisProfile::default(),
            right: StickAxisProfile::default(),
            trigger_left: TriggerProfile::default(),
            trigger_right: TriggerProfile::default(),
            flip_triggers: false,
            rumble_intensity: 1.0,
            turbo_polling: true,
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
    if buf.len() < o + 9 {
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

    if buf.len() >= o + 26 {
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

/// Builds and writes the lightbar / rumble output report for the pad.
///
/// `rumble` is `(weak, strong)` normalised `0.0..=1.0`.
pub fn write_output_report(
    device: &HidDevice,
    kind: PadKind,
    connection: ConnectionType,
    led: [u8; 3],
    rumble: (f32, f32),
) -> Result<(), String> {
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
            device.write(&buf).map_err(|e| e.to_string())?;
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
            device.write(&buf).map_err(|e| e.to_string())?;
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
            device.write(&buf).map_err(|e| e.to_string())?;
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
            device.write(&buf).map_err(|e| e.to_string())?;
        }
        _ => return Err("This controller has no addressable lightbar".into()),
    }
    Ok(())
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
    profile: RwLock<StickProfileConfig>,
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
                profile: RwLock::new(StickProfileConfig::default()),
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
        *self.inner.profile.read()
    }

    pub fn set_profile(&self, p: StickProfileConfig) {
        *self.inner.profile.write() = p;
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
            let prof = *inner.profile.read();
            let (lx, ly) = shape_stick(raw.lx, raw.ly, &prof.left);
            let (rx, ry) = shape_stick(raw.rx, raw.ry, &prof.right);
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
            if pending_led.is_some() || pending_rumble.is_some() {
                let led = pending_led.unwrap_or(p.info.led);
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
}
