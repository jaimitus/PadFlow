//! PadFlow — Tauri v2 application shell.
//!
//! Responsibilities:
//! * own the [`PadFlowEngine`] singleton in Tauri managed state,
//! * build the tray icon (battery aware) + context menu,
//! * keep the process alive in the tray when the window is closed,
//! * auto-start the input engine as soon as the webview is ready.

pub mod commands;
pub mod hidhide;
pub mod input;

use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

use crate::input::gamepad::PadFlowEngine;

/// Everything the IPC layer needs. Cheap to clone (engine is `Arc` inside).
pub struct AppState {
    pub engine: PadFlowEngine,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let engine = PadFlowEngine::new();
    let engine_for_tray = engine.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .manage(AppState {
            engine: engine.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_connected_gamepads,
            commands::set_led_color,
            commands::update_stick_profile,
            commands::start_padflow_engine,
            commands::stop_padflow_engine,
            commands::get_engine_status,
            commands::get_last_snapshot,
            commands::select_gamepad,
            commands::test_rumble,
            commands::preview_curve,
            commands::toggle_window,
            commands::open_url,
            commands::install_vigem_driver,
            commands::get_hidhide_status,
            commands::set_hidhide_active,
            commands::toggle_device_hide,
            commands::auto_cloak_controllers,
            commands::uncloak_all_controllers,
            commands::launch_hidhide_gui,
            commands::install_hidhide_driver,
            commands::relaunch_app,
        ])
        .setup(move |app| {
            // ---- tray -----------------------------------------------------
            let show_i = MenuItem::with_id(app, "show", "Open PadFlow", true, None::<&str>)?;
            let engine_i = MenuItem::with_id(app, "engine", "Start / Stop engine", true, None::<&str>)?;
            let rescan_i = MenuItem::with_id(app, "rescan", "Rescan controllers", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit PadFlow", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &engine_i, &rescan_i, &sep, &quit_i])?;

            let tray_engine = engine_for_tray.clone();
            TrayIconBuilder::with_id("padflow-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PadFlow — idle")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "engine" => {
                        let state = app.state::<AppState>();
                        if state.engine.is_running() {
                            state.engine.stop();
                            let _ = hidhide::uncloak_all_controllers();
                            let stats = state.engine.stats();
                            let _ = app.emit("padflow-engine-stopped", stats.clone());
                            let _ = app.emit("padflow-engine-stats", stats);
                        } else {
                            let _ = commands::run_engine_with_telemetry(&state.engine, app);
                        }
                    }
                    "rescan" => {
                        let state = app.state::<AppState>();
                        if let Ok(list) = state.engine.rescan() {
                            let _ = app.emit("padflow-devices-changed", list);
                        }
                    }
                    "quit" => {
                        tray_engine.stop();
                        let _ = hidhide::uncloak_all_controllers();
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // ---- battery-aware tray tooltip (1 Hz, negligible cost) -------
            let handle = app.handle().clone();
            let tip_engine = engine.clone();
            tauri::async_runtime::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
                loop {
                    tick.tick().await;
                    let snap = tip_engine.snapshot();
                    let running = tip_engine.is_running();
                    let devices = tip_engine.devices();
                    let name = devices
                        .first()
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "no controller".into());
                    let battery = if snap.battery >= 0 {
                        format!(" · {}%{}", snap.battery, if snap.charging { " ⚡" } else { "" })
                    } else {
                        String::new()
                    };
                    let tip = if running {
                        format!("PadFlow — {name}{battery} · {} Hz", snap.poll_hz.max(1))
                    } else {
                        format!("PadFlow — idle ({name})")
                    };
                    if let Some(tray) = handle.tray_by_id("padflow-tray") {
                        let _ = tray.set_tooltip(Some(tip));
                    }
                }
            });

            // ---- auto-start the realtime engine & HidHide whitelist ------
            let boot = app.handle().clone();
            let boot_engine = engine.clone();
            tauri::async_runtime::spawn(async move {
                let _ = hidhide::auto_whitelist_current_process();
                let _ = commands::run_engine_with_telemetry(&boot_engine, &boot);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window parks PadFlow in the tray instead of exiting.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running PadFlow application");
}
