//! PadFlow — Tauri v2 IPC surface.
//!
//! Every command returns `Result<T, String>` so the TypeScript side can use a
//! single `try/catch` and surface a human readable toast. No command ever
//! blocks the input thread: they either read a `RwLock` snapshot or push a
//! request into a lock-free-ish queue drained by the poll loop.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::hidhide::{self, HidHideStatus};
use crate::input::gamepad::{
    apply_curve, shape_stick, CurveKind, EngineStats, GamepadInfo, InputSnapshot,
    StickAxisProfile, StickProfileConfig, TriggerProfile,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// Helper payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub stats: EngineStats,
    pub profile: StickProfileConfig,
    pub devices: Vec<GamepadInfo>,
    pub vigem_installed: bool,
    pub hidhide_status: HidHideStatus,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurvePreviewRequest {
    pub curve: CurveKind,
    pub power: f32,
    pub inner_deadzone: f32,
    pub outer_deadzone: f32,
    pub anti_deadzone: f32,
    pub samples: u32,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Scans the HID bus and returns every connected PS4 / PS5 / XInput pad.
#[tauri::command]
pub fn get_connected_gamepads(state: State<'_, AppState>) -> Result<Vec<GamepadInfo>, String> {
    state.engine.rescan()
}

/// Sends the lightbar output report (USB or Bluetooth + CRC32) to a pad.
#[tauri::command]
pub fn set_led_color(
    pad_id: String,
    r: u8,
    g: u8,
    b: u8,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if pad_id.trim().is_empty() {
        return Err("pad_id must not be empty".into());
    }
    state.engine.queue_led(&pad_id, [r, g, b]);
    let _ = app.emit(
        "padflow-led-changed",
        serde_json::json!({ "padId": pad_id, "rgb": [r, g, b] }),
    );
    Ok(())
}

/// Hot-swaps the stick shaping parameters. Applied on the very next HID report.
#[tauri::command]
pub fn update_stick_profile(
    profile_data: StickProfileConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<StickProfileConfig, String> {
    validate_axis("left", &profile_data.left)?;
    validate_axis("right", &profile_data.right)?;
    validate_trigger("triggerLeft", &profile_data.trigger_left)?;
    validate_trigger("triggerRight", &profile_data.trigger_right)?;
    if !(0.0..=1.0).contains(&profile_data.rumble_intensity) {
        return Err("rumble_intensity must be within 0.0..=1.0".into());
    }
    state.engine.set_profile(profile_data);
    let _ = app.emit("padflow-profile-updated", profile_data);
    Ok(profile_data)
}

fn validate_trigger(name: &str, t: &TriggerProfile) -> Result<(), String> {
    if t.inner_deadzone < 0.0 || t.inner_deadzone > 0.8 {
        return Err(format!("{name}.inner_deadzone must be within 0.0..=0.8"));
    }
    if t.outer_deadzone <= t.inner_deadzone || t.outer_deadzone > 1.0 {
        return Err(format!(
            "{name}.outer_deadzone must be greater than inner deadzone and <= 1.0"
        ));
    }
    Ok(())
}

fn validate_axis(name: &str, a: &StickAxisProfile) -> Result<(), String> {
    if a.inner_deadzone < 0.0 || a.inner_deadzone > 0.9 {
        return Err(format!("{name}.inner_deadzone must be within 0.0..=0.9"));
    }
    if a.outer_deadzone <= a.inner_deadzone || a.outer_deadzone > 1.0 {
        return Err(format!(
            "{name}.outer_deadzone must be greater than the inner deadzone and <= 1.0"
        ));
    }
    if a.anti_deadzone < 0.0 || a.anti_deadzone > 0.6 {
        return Err(format!("{name}.anti_deadzone must be within 0.0..=0.6"));
    }
    if a.curve_power < 0.5 || a.curve_power > 4.0 {
        return Err(format!("{name}.curve_power must be within 0.5..=4.0"));
    }
    if a.sensitivity < 0.25 || a.sensitivity > 3.0 {
        return Err(format!("{name}.sensitivity must be within 0.25..=3.0"));
    }
    Ok(())
}

/// Allocates the ViGEm virtual Xbox 360 target and starts the realtime loop.
/// Streams `padflow-input-update` to the webview at 60 Hz.
#[tauri::command]
pub fn start_padflow_engine(state: State<'_, AppState>, app: AppHandle) -> Result<EngineStats, String> {
    if state.engine.is_running() {
        return Ok(state.engine.stats());
    }
    let _ = state.engine.rescan();
    let sink = app.clone();
    state.engine.spawn(move |snapshot: InputSnapshot| {
        let _ = sink.emit("padflow-input-update", snapshot);
    })?;

    // Telemetry heartbeat: 4 Hz, cheap, keeps the header widgets honest.
    let engine = state.engine.clone();
    let beat = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            tick.tick().await;
            if !engine.is_running() {
                let _ = beat.emit("padflow-engine-stats", engine.stats());
                break;
            }
            let _ = beat.emit("padflow-engine-stats", engine.stats());
        }
    });

    let _ = app.emit("padflow-engine-started", state.engine.stats());
    Ok(state.engine.stats())
}

/// Stops the loop and unplugs the virtual pad (neutralised first).
#[tauri::command]
pub fn stop_padflow_engine(state: State<'_, AppState>, app: AppHandle) -> Result<EngineStats, String> {
    state.engine.stop();
    let stats = state.engine.stats();
    let _ = app.emit("padflow-engine-stopped", stats.clone());
    Ok(stats)
}

/// One-shot status pull used on window focus / after a resume from sleep.
#[tauri::command]
pub fn get_engine_status(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    let devices = state.engine.rescan().unwrap_or_default();
    Ok(EngineStatus {
        stats: state.engine.stats(),
        profile: state.engine.profile(),
        devices,
        vigem_installed: vigem_client::Client::connect().is_ok(),
        hidhide_status: hidhide::get_status(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Returns current HidHide device firewall status.
#[tauri::command]
pub fn get_hidhide_status() -> Result<HidHideStatus, String> {
    Ok(hidhide::get_status())
}

/// Enables or disables HidHide device hiding globally.
#[tauri::command]
pub fn set_hidhide_active(active: bool, app: AppHandle) -> Result<HidHideStatus, String> {
    hidhide::set_active(active)?;
    let status = hidhide::get_status();
    let _ = app.emit("padflow-hidhide-updated", status.clone());
    Ok(status)
}

/// Hides or unhides a physical controller from non-whitelisted applications.
#[tauri::command]
pub fn toggle_device_hide(
    device_path: String,
    hide: bool,
    app: AppHandle,
) -> Result<HidHideStatus, String> {
    if hide {
        hidhide::hide_device(&device_path)?;
    } else {
        hidhide::unhide_device(&device_path)?;
    }
    let status = hidhide::get_status();
    let _ = app.emit("padflow-hidhide-updated", status.clone());
    Ok(status)
}

/// Cloaks all currently connected PlayStation / HID controllers automatically.
#[tauri::command]
pub fn auto_cloak_controllers(state: State<'_, AppState>, app: AppHandle) -> Result<HidHideStatus, String> {
    let devices = state.engine.devices();
    let _ = hidhide::auto_whitelist_current_process();
    for dev in devices {
        let _ = hidhide::hide_device(&dev.path);
    }
    let _ = hidhide::set_active(true);
    let status = hidhide::get_status();
    let _ = app.emit("padflow-hidhide-updated", status.clone());
    Ok(status)
}

/// Launches the HidHide driver installer with UAC Administrator privileges.
#[tauri::command]
pub fn install_hidhide_driver(app: AppHandle) -> Result<String, String> {
    hidhide::install_hidhide_driver(&app)
}

/// Selects the active gamepad for UI stream and calibration canvas.
#[tauri::command]
pub fn select_gamepad(pad_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.engine.set_active_pad(&pad_id);
    Ok(())
}

/// Pulls the latest frame for a specific pad (or active pad).
#[tauri::command]
pub fn get_last_snapshot(pad_id: Option<String>, state: State<'_, AppState>) -> Result<InputSnapshot, String> {
    Ok(state.engine.snapshot_for(pad_id.as_deref()))
}

/// Fires a haptic pulse so the user can verify rumble intensity.
#[tauri::command]
pub fn test_rumble(weak: f32, strong: f32, state: State<'_, AppState>) -> Result<(), String> {
    if !(0.0..=1.0).contains(&weak) || !(0.0..=1.0).contains(&strong) {
        return Err("rumble values must be within 0.0..=1.0".into());
    }
    state.engine.queue_rumble(weak, strong);
    let engine = state.engine.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(450)).await;
        engine.queue_rumble(0.0, 0.0);
    });
    Ok(())
}

/// Server-side curve evaluation — guarantees the canvas draws *exactly* the
/// same maths the input thread executes (no drift between UI and engine).
#[tauri::command]
pub fn preview_curve(req: CurvePreviewRequest) -> Result<Vec<[f32; 2]>, String> {
    let n = req.samples.clamp(8, 512);
    let axis = StickAxisProfile {
        inner_deadzone: req.inner_deadzone,
        outer_deadzone: req.outer_deadzone,
        anti_deadzone: req.anti_deadzone,
        curve: req.curve,
        curve_power: req.power,
        sensitivity: 1.0,
        invert_y: false,
        radial: true,
    };
    let mut out = Vec::with_capacity(n as usize + 1);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let (x, _) = shape_stick(t, 0.0, &axis);
        out.push([t, x]);
    }
    // Touch `apply_curve` so the pure maths path is always linked in release.
    debug_assert!(apply_curve(0.5, req.curve, req.power) >= 0.0);
    Ok(out)
}

/// Toggles the main window from the tray / global shortcut.
#[tauri::command]
pub fn toggle_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            w.hide().map_err(|e| e.to_string())?;
        } else {
            w.show().map_err(|e| e.to_string())?;
            w.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Opens an external HTTP/HTTPS URL in the system default browser.
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("invalid URL scheme".into());
    }
    use tauri_plugin_shell::ShellExt;
    app.shell().open(url, None).map_err(|e| e.to_string())
}

/// Launches the ViGEmBus driver installer with UAC Administrator privileges.
#[tauri::command]
pub fn install_vigem_driver(app: AppHandle) -> Result<String, String> {
    let temp_exe = std::env::temp_dir().join("ViGEmBus_Setup.exe");

    // Check bundled resource path first
    let res_exe = app
        .path()
        .resource_dir()
        .map(|p| p.join("resources").join("ViGEmBus_Setup.exe"))
        .ok();

    let target_path = if let Some(ref r) = res_exe {
        if r.exists() {
            r.clone()
        } else {
            temp_exe.clone()
        }
    } else {
        temp_exe.clone()
    };

    if !target_path.exists() {
        // Download via powershell if not pre-bundled
        let dl_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; (New-Object System.Net.WebClient).DownloadFile('https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe', '{}')",
            temp_exe.to_string_lossy().replace("'", "''")
        );
        let dl_status = std::process::Command::new("powershell")
            .args(["-Command", &dl_cmd])
            .status()
            .map_err(|e| format!("failed to download driver installer: {e}"))?;
        if !dl_status.success() {
            return Err("Failed to download ViGEmBus driver installer from GitHub".into());
        }
    }

    let exe_to_run = if target_path.exists() { target_path } else { temp_exe };

    // Run installer with /norestart so the user sees the wizard and NO auto-reboot can happen
    let install_cmd = format!(
        "Start-Process -FilePath '{}' -ArgumentList '/norestart' -Verb RunAs -Wait",
        exe_to_run.to_string_lossy().replace("'", "''")
    );

    let status = std::process::Command::new("powershell")
        .args(["-Command", &install_cmd])
        .status()
        .map_err(|e| format!("failed to run installer: {e}"))?;

    // Attempt to start ViGEmBus service in case it's not automatically started by Windows
    let _ = std::process::Command::new("powershell")
        .args(["-Command", "Start-Service ViGEmBus -ErrorAction SilentlyContinue; net start ViGEmBus"])
        .status();

    if status.success() {
        // Verify if ViGEmBus client can connect
        if vigem_client::Client::connect().is_ok() {
            Ok("ViGEmBus driver installed and connected! Virtual pad online.".into())
        } else {
            Ok("ViGEmBus installation finished. If virtual pad remains offline, please restart Windows once to load driver.".into())
        }
    } else {
        Err("Driver installation was cancelled or denied Administrator permissions".into())
    }
}
