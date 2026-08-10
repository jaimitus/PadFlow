//! Native Mouse Driver Daemon
//! 
//! Main entry point for the standalone driver service.
//! Communicates with Electron app via Unix domain sockets (Linux/macOS) or named pipes (Windows).

use anyhow::Result;
use env_logger::Env;
use log::{error, info};
use mouse_driver_lib::MouseDriver;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let env = Env::default().filter_or("MOUSE_DRIVER_LOG", "info");
    env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .init();

    info!("🚀 Mouse Driver Daemon v1.4.0 starting...");
    info!("Native high-performance driver with zero JavaScript dependencies");

    // Create driver instance
    let driver = Arc::new(MouseDriver::new()?);
    let running = Arc::new(Mutex::new(true));

    // Handle shutdown signals
    let driver_clone = driver.clone();
    let running_clone = running.clone();
    
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("Shutdown signal received");
        let mut running_guard = running_clone.lock().await;
        *running_guard = false;
        driver_clone.stop();
    });

    // Start the driver
    driver.start()?;
    info!("✅ Driver initialized successfully");

    // Platform-specific initialization
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = driver.start_battery_monitoring() {
            error!("Failed to start battery monitoring: {}", e);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Err(e) = driver.start_battery_monitoring() {
            error!("Failed to start battery monitoring: {}", e);
        }
        
        // Set high thread priority on Windows
        driver.set_thread_priority(15)?; // THREAD_PRIORITY_HIGHEST
    }

    // Main event loop - would normally listen on IPC socket
    info!("🔄 Waiting for commands from Electron app...");
    info!("📡 IPC endpoint: /tmp/mouse_driver.sock (Unix) or \\\\.\\pipe\\mouse_driver (Windows)");
    
    while *running.lock().await {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // In production, this would:
        // 1. Listen for incoming connections on Unix socket / named pipe
        // 2. Parse JSON commands from Electron app
        // 3. Execute driver operations (load profile, activate, get stats, etc.)
        // 4. Send responses back with statistics and acknowledgments
        
        // Example command processing (placeholder):
        // - {"command": "load_profile", "data": "{...}"}
        // - {"command": "activate_profile", "profile_id": "..."}
        // - {"command": "get_stats"}
        // - {"command": "set_battery_saver", "enabled": true}
    }

    info!("👋 Driver daemon stopped gracefully");
    Ok(())
}
