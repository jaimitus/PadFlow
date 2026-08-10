//! PadFlow — Game detection engine for automatic profile switching.
//!
//! Features:
//! * Real-time process monitoring for game launches
//! * Automatic profile application based on game executable
//! * AI optimization triggers per-game
//! * Battery saver suggestions per-game

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::{CloseHandle, HANDLE, ERROR_SUCCESS},
    Win32::System::Threading::{
        EnumProcesses, GetModuleFileNameExW, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    },
};

/// Game profile configuration with AI and performance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameProfile {
    pub game_id: String,
    pub executable_name: String,
    pub game_title: String,
    pub recommended_profile: StickProfileConfig,
    pub ai_curve_optimization: bool,
    pub battery_saver_recommended: bool,
    pub battery_threshold: u8,
    pub icon_path: Option<String>,
    pub last_played: Option<u64>,
    pub play_time_seconds: u64,
}

/// Detection result for a running game
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedGame {
    pub game_id: String,
    pub executable_name: String,
    pub game_title: String,
    pub process_id: u32,
    pub detected_at: u64,
    pub profile_applied: bool,
}

/// Internal state for the game detector
struct GameDetectorInner {
    /// Map of executable names to game profiles
    games_db: HashMap<String, GameProfile>,
    /// Currently detected running games
    detected_games: HashMap<u32, DetectedGame>,
    /// Profile to apply when no game is detected (default)
    default_profile: StickProfileConfig,
    /// Auto-switch enabled/disabled
    auto_switch_enabled: bool,
    /// Last scan timestamp
    last_scan: Instant,
    /// Scan interval in milliseconds
    scan_interval_ms: u64,
}

/// Main game detector handle (cheap to clone, thread-safe)
#[derive(Clone)]
pub struct GameDetector {
    inner: Arc<RwLock<GameDetectorInner>>,
}

impl Default for GameDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl GameDetector {
    pub fn new() -> Self {
        let mut games_db = HashMap::new();
        
        // Load built-in game database
        Self::load_builtin_games(&mut games_db);
        
        Self {
            inner: Arc::new(RwLock::new(GameDetectorInner {
                games_db,
                detected_games: HashMap::new(),
                default_profile: StickProfileConfig::default(),
                auto_switch_enabled: true,
                last_scan: Instant::now(),
                scan_interval_ms: 1000, // Scan every second
            })),
        }
    }
    
    /// Load built-in game database with popular titles
    fn load_builtin_games(games_db: &mut HashMap<String, GameProfile>) {
        // Apex Legends
        games_db.insert(
            "apex_legends.exe".to_string(),
            GameProfile {
                game_id: "apex_legends".to_string(),
                executable_name: "apex_legends.exe".to_string(),
                game_title: "Apex Legends".to_string(),
                recommended_profile: StickProfileConfig {
                    adaptive_polling: true,
                    target_poll_hz: 1000,
                    batch_reports: false,
                    battery_saver: false,
                    ai_curve_optimization: true,
                    ..StickProfileConfig::default()
                },
                ai_curve_optimization: true,
                battery_saver_recommended: false,
                battery_threshold: 30,
                icon_path: None,
                last_played: None,
                play_time_seconds: 0,
            },
        );
        
        // Call of Duty
        games_db.insert(
            "cod_warzone.exe".to_string(),
            GameProfile {
                game_id: "cod_warzone".to_string(),
                executable_name: "cod_warzone.exe".to_string(),
                game_title: "Call of Duty: Warzone".to_string(),
                recommended_profile: StickProfileConfig {
                    adaptive_polling: true,
                    target_poll_hz: 1000,
                    batch_reports: false,
                    battery_saver: false,
                    ai_curve_optimization: true,
                    ..StickProfileConfig::default()
                },
                ai_curve_optimization: true,
                battery_saver_recommended: false,
                battery_threshold: 25,
                icon_path: None,
                last_played: None,
                play_time_seconds: 0,
            },
        );
        
        // Elden Ring
        games_db.insert(
            "eldenring.exe".to_string(),
            GameProfile {
                game_id: "elden_ring".to_string(),
                executable_name: "eldenring.exe".to_string(),
                game_title: "Elden Ring".to_string(),
                recommended_profile: StickProfileConfig {
                    adaptive_polling: true,
                    target_poll_hz: 500,
                    batch_reports: true,
                    battery_saver: false,
                    ai_curve_optimization: true,
                    ..StickProfileConfig::default()
                },
                ai_curve_optimization: true,
                battery_saver_recommended: false,
                battery_threshold: 30,
                icon_path: None,
                last_played: None,
                play_time_seconds: 0,
            },
        );
        
        // Fortnite
        games_db.insert(
            "fortnite.exe".to_string(),
            GameProfile {
                game_id: "fortnite".to_string(),
                executable_name: "fortnite.exe".to_string(),
                game_title: "Fortnite".to_string(),
                recommended_profile: StickProfileConfig {
                    adaptive_polling: true,
                    target_poll_hz: 1000,
                    batch_reports: false,
                    battery_saver: false,
                    ai_curve_optimization: true,
                    ..StickProfileConfig::default()
                },
                ai_curve_optimization: true,
                battery_saver_recommended: false,
                battery_threshold: 30,
                icon_path: None,
                last_played: None,
                play_time_seconds: 0,
            },
        );
        
        // Rocket League
        games_db.insert(
            "rocketleague.exe".to_string(),
            GameProfile {
                game_id: "rocket_league".to_string(),
                executable_name: "rocketleague.exe".to_string(),
                game_title: "Rocket League".to_string(),
                recommended_profile: StickProfileConfig {
                    adaptive_polling: true,
                    target_poll_hz: 1000,
                    batch_reports: false,
                    battery_saver: false,
                    ai_curve_optimization: true,
                    ..StickProfileConfig::default()
                },
                ai_curve_optimization: true,
                battery_saver_recommended: false,
                battery_threshold: 30,
                icon_path: None,
                last_played: None,
                play_time_seconds: 0,
            },
        );
        
        // Cyberpunk 2077
        games_db.insert(
            "cyberpunk2077.exe".to_string(),
            GameProfile {
                game_id: "cyberpunk_2077".to_string(),
                executable_name: "cyberpunk2077.exe".to_string(),
                game_title: "Cyberpunk 2077".to_string(),
                recommended_profile: StickProfileConfig {
                    adaptive_polling: true,
                    target_poll_hz: 500,
                    batch_reports: true,
                    battery_saver: false,
                    ai_curve_optimization: true,
                    ..StickProfileConfig::default()
                },
                ai_curve_optimization: true,
                battery_saver_recommended: false,
                battery_threshold: 30,
                icon_path: None,
                last_played: None,
                play_time_seconds: 0,
            },
        );
        
        // Generic AAA games - battery saver recommended at 30%
        games_db.insert(
            "*.exe".to_string(),
            GameProfile {
                game_id: "generic_game".to_string(),
                executable_name: "*.exe".to_string(),
                game_title: "Generic Game".to_string(),
                recommended_profile: StickProfileConfig {
                    adaptive_polling: true,
                    target_poll_hz: 500,
                    batch_reports: true,
                    battery_saver: false,
                    ai_curve_optimization: false,
                    ..StickProfileConfig::default()
                },
                ai_curve_optimization: false,
                battery_saver_recommended: true,
                battery_threshold: 30,
                icon_path: None,
                last_played: None,
                play_time_seconds: 0,
            },
        );
    }
    
    /// Scan for running games and return detected list
    pub fn scan_for_games(&self) -> Vec<DetectedGame> {
        let mut inner = self.inner.write();
        
        // Rate limit scans
        if inner.last_scan.elapsed().as_millis() < inner.scan_interval_ms as u128 {
            return inner.detected_games.values().cloned().collect();
        }
        
        inner.last_scan = Instant::now();
        
        #[cfg(target_os = "windows")]
        {
            let mut process_ids: [u32; 1024] = [0; 1024];
            let mut bytes_returned: u32 = 0;
            
            unsafe {
                if EnumProcesses(
                    process_ids.as_mut_ptr(),
                    (process_ids.len() * std::mem::size_of::<u32>()) as u32,
                    &mut bytes_returned,
                )
                .is_ok()
                {
                    let num_processes = bytes_returned as usize / std::mem::size_of::<u32>();
                    
                    for &pid in &process_ids[..num_processes] {
                        if pid == 0 {
                            continue;
                        }
                        
                        if let Ok(game_info) = Self::get_process_info(pid) {
                            let exe_name = game_info.1.to_lowercase();
                            
                            // Check if this executable matches a known game
                            if let Some(profile) = inner.games_db.get(&exe_name) {
                                let detected = DetectedGame {
                                    game_id: profile.game_id.clone(),
                                    executable_name: exe_name.clone(),
                                    game_title: profile.game_title.clone(),
                                    process_id: pid,
                                    detected_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                    profile_applied: true,
                                };
                                
                                inner.detected_games.insert(pid, detected.clone());
                                
                                // Update play time
                                let mut profile = inner.games_db.get_mut(&exe_name).unwrap();
                                profile.last_played = Some(detected.detected_at);
                                profile.play_time_seconds += 1;
                            }
                        }
                    }
                }
            }
            
            // Clean up processes that are no longer running
            inner.detected_games.retain(|&pid, _| {
                unsafe {
                    let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid);
                    if handle.is_err() {
                        return false;
                    }
                    CloseHandle(handle.unwrap());
                    true
                }
            });
        }
        
        inner.detected_games.values().cloned().collect()
    }
    
    #[cfg(target_os = "windows")]
    fn get_process_info(pid: u32) -> Result<(String, String), String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                .map_err(|_| format!("Failed to open process {}", pid))?;
            
            let mut exe_path = [0u16; 260];
            let len = GetModuleFileNameExW(handle, None, &mut exe_path);
            CloseHandle(handle);
            
            if len == 0 {
                return Err("Failed to get module name".to_string());
            }
            
            let exe_path_str = String::from_utf16_lossy(&exe_path[..len as usize]);
            let exe_name = PathBuf::from(&exe_path_str)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.exe")
                .to_string();
            
            Ok((exe_path_str, exe_name))
        }
    }
    
    /// Get the recommended profile for a specific game
    pub fn get_profile_for_game(&self, executable_name: &str) -> Option<StickProfileConfig> {
        let inner = self.inner.read();
        let exe_lower = executable_name.to_lowercase();
        
        inner
            .games_db
            .get(&exe_lower)
            .map(|p| p.recommended_profile.clone())
    }
    
    /// Check if battery saver should be recommended for current game
    pub fn should_recommend_battery_saver(&self, executable_name: &str, battery_level: u8) -> bool {
        let inner = self.inner.read();
        let exe_lower = executable_name.to_lowercase();
        
        if let Some(profile) = inner.games_db.get(&exe_lower) {
            return profile.battery_saver_recommended && battery_level <= profile.battery_threshold;
        }
        
        // Default recommendation for unknown games
        battery_level <= 30
    }
    
    /// Enable or disable auto-switching
    pub fn set_auto_switch_enabled(&self, enabled: bool) {
        let mut inner = self.inner.write();
        inner.auto_switch_enabled = enabled;
    }
    
    /// Check if auto-switching is enabled
    pub fn is_auto_switch_enabled(&self) -> bool {
        self.inner.read().auto_switch_enabled
    }
    
    /// Add or update a game profile
    pub fn add_game_profile(&self, profile: GameProfile) {
        let mut inner = self.inner.write();
        inner.games_db.insert(profile.executable_name.clone(), profile);
    }
    
    /// Remove a game profile
    pub fn remove_game_profile(&self, executable_name: &str) {
        let mut inner = self.inner.write();
        inner.games_db.remove(&executable_name.to_lowercase());
    }
    
    /// Get all game profiles
    pub fn get_all_profiles(&self) -> Vec<GameProfile> {
        let inner = self.inner.read();
        inner.games_db.values().cloned().collect()
    }
    
    /// Get currently detected games
    pub fn get_detected_games(&self) -> Vec<DetectedGame> {
        let inner = self.inner.read();
        inner.detected_games.values().cloned().collect()
    }
    
    /// Clear detected games (e.g., when all games close)
    pub fn clear_detected_games(&self) {
        let mut inner = self.inner.write();
        inner.detected_games.clear();
    }
    
    /// Set default profile for non-game usage
    pub fn set_default_profile(&self, profile: StickProfileConfig) {
        let mut inner = self.inner.write();
        inner.default_profile = profile;
    }
    
    /// Get default profile
    pub fn get_default_profile(&self) -> StickProfileConfig {
        self.inner.read().default_profile.clone()
    }
}

// Re-export types needed by other modules
pub use crate::input::gamepad::StickProfileConfig;
