//! PadFlow input subsystem.
//!
//! `gamepad` owns everything that touches hardware: raw HID ingestion for
//! Sony DualShock 4 / DualSense pads, the response-curve mathematics, the
//! lightbar / rumble output reports and the ViGEmBus virtual Xbox target.

pub mod gamepad;

pub use gamepad::{
    ConnectionType, CurveKind, EngineStats, GamepadInfo, InputSnapshot, PadFlowEngine, PadKind,
    StickAxisProfile, StickProfileConfig,
};
