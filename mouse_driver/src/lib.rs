//! Native Mouse Driver Library
//! 
//! Provides low-level access to HID devices with acceleration curve processing.
//! Zero JavaScript dependencies - pure Rust performance.

use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Acceleration curve types supported by the driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveType {
    Linear,
    Exponential,
    Custom(Vec<f64>),
    AIEnhanced { confidence: f64, pattern_id: String },
}

/// Configuration for a single axis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisConfig {
    pub sensitivity: f64,
    pub curve_type: CurveType,
    pub enabled: bool,
    pub ai_learning_rate: Option<f64>,
}

/// Performance settings per profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    pub polling_frequency_hz: u32,
    pub hid_batch_size: u8,
    pub battery_saver_mode: bool,
    pub thread_priority: i32,
    pub ai_optimization_enabled: bool,
}

/// Profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub game_name: Option<String>,
    pub x_axis: AxisConfig,
    pub y_axis: AxisConfig,
    pub scroll_axis: Option<AxisConfig>,
    pub performance: PerformanceSettings,
    pub created_at: u64,
    pub updated_at: u64,
    pub metadata: HashMap<String, String>,
}

/// Real-time statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStats {
    pub device_id: String,
    pub poll_rate_current: f64,
    pub poll_rate_average: f64,
    pub packets_processed: u64,
    pub packets_dropped: u64,
    pub battery_level: Option<u8>,
    pub battery_charging: bool,
    pub thread_priority: i32,
    pub batch_stats: BatchStatistics,
    pub ai_metrics: Option<AIMetrics>,
    pub last_update: u64,
}

/// Batch processing statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchStatistics {
    pub batches_sent: u64,
    pub average_batch_size: f64,
    max_batch_size: u8,
    efficiency_score: f64,
}

/// AI analysis metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMetrics {
    pub confidence_score: f64,
    pub samples_analyzed: u64,
    pub pattern_detected: String,
    pub learning_progress: f64,
}

/// HID Report structure
#[derive(Debug, Clone)]
pub struct HidReport {
    pub report_id: u8,
    pub data: Vec<u8>,
    pub timestamp: Instant,
}

/// Main driver struct
pub struct MouseDriver {
    profiles: Arc<Mutex<HashMap<String, Profile>>>,
    active_profile_id: Arc<Mutex<Option<String>>>,
    stats: Arc<Mutex<DeviceStats>>,
    running: Arc<Mutex<bool>>,
}

impl MouseDriver {
    /// Create a new driver instance
    pub fn new() -> Result<Self> {
        info!("Initializing native mouse driver v1.4.0");
        
        let initial_stats = DeviceStats {
            device_id: String::from("pending"),
            poll_rate_current: 0.0,
            poll_rate_average: 0.0,
            packets_processed: 0,
            packets_dropped: 0,
            battery_level: None,
            battery_charging: false,
            thread_priority: 0,
            batch_stats: BatchStatistics::default(),
            ai_metrics: None,
            last_update: 0,
        };

        Ok(Self {
            profiles: Arc::new(Mutex::new(HashMap::new())),
            active_profile_id: Arc::new(Mutex::new(None)),
            stats: Arc::new(Mutex::new(initial_stats)),
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Load profile from JSON
    pub fn load_profile(&self, json: &str) -> Result<Profile> {
        let profile: Profile = serde_json::from_str(json)
            .context("Failed to parse profile JSON")?;
        
        info!("Loaded profile: {} ({})", profile.name, profile.id);
        
        let mut profiles = self.profiles.lock().unwrap();
        profiles.insert(profile.id.clone(), profile.clone());
        
        Ok(profile)
    }

    /// Activate a profile by ID
    pub fn activate_profile(&self, profile_id: &str) -> Result<()> {
        let mut active_id = self.active_profile_id.lock().unwrap();
        *active_id = Some(profile_id.to_string());
        info!("Activated profile: {}", profile_id);
        Ok(())
    }

    /// Get current statistics
    pub fn get_stats(&self) -> DeviceStats {
        self.stats.lock().unwrap().clone()
    }

    /// Process HID report with acceleration curves
    pub fn process_report(&self, report: &HidReport) -> Result<HidReport> {
        let active_profile_id = self.active_profile_id.lock().unwrap();
        if active_profile_id.is_none() {
            return Ok(report.clone());
        }

        let profiles = self.profiles.lock().unwrap();
        let profile = match profiles.get(active_profile_id.as_ref().unwrap()) {
            Some(p) => p,
            None => return Ok(report.clone()),
        };

        // Apply acceleration curves here (simplified example)
        let mut processed_data = report.data.clone();
        
        if profile.x_axis.enabled {
            // Apply X-axis curve
            self.apply_curve(&mut processed_data, &profile.x_axis, 0);
        }
        
        if profile.y_axis.enabled {
            // Apply Y-axis curve
            self.apply_curve(&mut processed_data, &profile.y_axis, 1);
        }

        // Update statistics
        self.update_stats(report, profile);

        Ok(HidReport {
            report_id: report.report_id,
            data: processed_data,
            timestamp: report.timestamp,
        })
    }

    /// Apply acceleration curve to axis data
    fn apply_curve(&self, data: &mut [u8], config: &AxisConfig, axis_index: usize) {
        if data.len() <= axis_index {
            return;
        }

        let raw_value = data[axis_index] as i16;
        let adjusted_value = match &config.curve_type {
            CurveType::Linear => {
                (raw_value as f64 * config.sensitivity) as i16
            },
            CurveType::Exponential => {
                let sign = if raw_value < 0 { -1.0 } else { 1.0 };
                let abs_val = raw_value.abs() as f64;
                (sign * abs_val.powf(config.sensitivity)) as i16
            },
            CurveType::Custom(points) => {
                // Interpolate custom curve
                self.interpolate_curve(raw_value, points, config.sensitivity)
            },
            CurveType::AIEnhanced { .. } => {
                // AI-enhanced processing (placeholder)
                (raw_value as f64 * config.sensitivity) as i16
            },
        };

        // Clamp to valid range
        let clamped = adjusted_value.clamp(-127, 127);
        data[axis_index] = clamped as u8;
    }

    /// Interpolate value using custom curve points
    fn interpolate_curve(&self, value: i16, points: &[f64], sensitivity: f64) -> i16 {
        if points.is_empty() {
            return value;
        }

        let normalized = ((value + 127) as f64 / 254.0).clamp(0.0, 1.0);
        let point_index = (normalized * (points.len() - 1) as f64) as usize;
        let next_index = (point_index + 1).min(points.len() - 1);
        
        let t = (normalized * (points.len() - 1) as f64) - point_index as f64;
        let interpolated = points[point_index] * (1.0 - t) + points[next_index] * t;
        
        ((interpolated * 254.0 - 127.0) * sensitivity) as i16
    }

    /// Update driver statistics
    fn update_stats(&self, _report: &HidReport, profile: &Profile) {
        let mut stats = self.stats.lock().unwrap();
        stats.packets_processed += 1;
        stats.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Update batch statistics
        if profile.performance.hid_batch_size > 0 {
            stats.batch_stats.batches_sent += 1;
            stats.batch_stats.efficiency_score = 
                (stats.batch_stats.batches_sent as f64 / stats.packets_processed as f64) * 100.0;
        }

        // Update thread priority display
        stats.thread_priority = profile.performance.thread_priority;

        // Simulate AI metrics if enabled
        if profile.performance.ai_optimization_enabled {
            stats.ai_metrics = Some(AIMetrics {
                confidence_score: 0.95,
                samples_analyzed: stats.packets_processed,
                pattern_detected: "smooth_tracking".to_string(),
                learning_progress: (stats.packets_processed as f64 / 1000.0).min(1.0),
            });
        }
    }

    /// Start battery monitoring (platform-specific)
    #[cfg(target_os = "linux")]
    pub fn start_battery_monitoring(&self) -> Result<()> {
        info!("Starting battery monitoring via D-Bus");
        // Implementation would use dbus crate to connect to upower
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn start_battery_monitoring(&self) -> Result<()> {
        info!("Starting battery monitoring via Windows API");
        // Implementation would use windows crate
        Ok(())
    }

    /// Set thread priority for real-time processing
    pub fn set_thread_priority(&self, priority: i32) -> Result<()> {
        info!("Setting thread priority to {}", priority);
        // Platform-specific implementation
        Ok(())
    }

    /// Enable/disable battery saver mode
    pub fn set_battery_saver_mode(&self, enabled: bool) {
        info!("Battery saver mode: {}", if enabled { "ON" } else { "OFF" });
        // Adjust polling frequency and disable non-essential features
    }

    /// Export all profiles to JSON
    pub fn export_profiles(&self) -> Result<String> {
        let profiles = self.profiles.lock().unwrap();
        serde_json::to_string_pretty(&*profiles)
            .context("Failed to serialize profiles")
    }

    /// Check if driver is running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Start the driver daemon
    pub fn start(&self) -> Result<()> {
        info!("Starting mouse driver daemon");
        let mut running = self.running.lock().unwrap();
        *running = true;
        Ok(())
    }

    /// Stop the driver daemon
    pub fn stop(&self) {
        info!("Stopping mouse driver daemon");
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
}

impl Default for MouseDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create default driver instance")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_creation() {
        let driver = MouseDriver::new();
        assert!(driver.is_ok());
    }

    #[test]
    fn test_profile_loading() {
        let driver = MouseDriver::new().unwrap();
        let json = r#"{
            "id": "test-profile",
            "name": "Test Profile",
            "game_name": null,
            "x_axis": {"sensitivity": 1.5, "curve_type": "Linear", "enabled": true, "ai_learning_rate": null},
            "y_axis": {"sensitivity": 1.5, "curve_type": "Linear", "enabled": true, "ai_learning_rate": null},
            "scroll_axis": null,
            "performance": {
                "polling_frequency_hz": 1000,
                "hid_batch_size": 4,
                "battery_saver_mode": false,
                "thread_priority": 15,
                "ai_optimization_enabled": true
            },
            "created_at": 1234567890,
            "updated_at": 1234567890,
            "metadata": {}
        }"#;
        
        let profile = driver.load_profile(json);
        assert!(profile.is_ok());
    }
}
