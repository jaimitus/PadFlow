//! PadFlow — HidHide driver client and anti-double-input protection.
//!
//! HidHide is a kernel-mode filter driver by Nefarius that acts as a device firewall.
//! It hides physical PlayStation (DS4 / DualSense) controllers from games and third-party
//! applications while allowing whitelisted feeder applications (PadFlow) to read them and
//! expose a single virtual XInput controller via ViGEmBus.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HidHideStatus {
    pub installed: bool,
    pub active: bool,
    pub whitelisted: bool,
    pub hidden_devices: Vec<String>,
    pub app_path: String,
}

// ---------------------------------------------------------------------------
// IOCTL definitions
// ---------------------------------------------------------------------------

const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

const FILE_DEVICE_UNKNOWN: u32 = 0x00000022;
const METHOD_BUFFERED: u32 = 0;
const FILE_READ_DATA: u32 = 1;
const FILE_WRITE_DATA: u32 = 2;

pub const IOCTL_GET_WHITELIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2048, METHOD_BUFFERED, FILE_READ_DATA);
pub const IOCTL_SET_WHITELIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2049, METHOD_BUFFERED, FILE_WRITE_DATA);
pub const IOCTL_GET_BLACKLIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2050, METHOD_BUFFERED, FILE_READ_DATA);
pub const IOCTL_SET_BLACKLIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2051, METHOD_BUFFERED, FILE_WRITE_DATA);
pub const IOCTL_GET_ACTIVE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2052, METHOD_BUFFERED, FILE_READ_DATA);
pub const IOCTL_SET_ACTIVE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2053, METHOD_BUFFERED, FILE_WRITE_DATA);
pub const IOCTL_GET_WLINVERSE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2054, METHOD_BUFFERED, FILE_READ_DATA);
pub const IOCTL_SET_WLINVERSE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2055, METHOD_BUFFERED, FILE_WRITE_DATA);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Normalizes a hidapi device path into a standard Windows Device Instance Path.
/// Example: `\\?\hid#vid_054c&pid_0ce6&col01#7&3084128&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}`
/// becomes: `HID\VID_054C&PID_0CE6&COL01\7&3084128&0&0000`
pub fn normalize_device_instance_path(path: &str) -> String {
    let mut clean = path.trim();
    if let Some(stripped) = clean.strip_prefix(r"\\?\") {
        clean = stripped;
    } else if let Some(stripped) = clean.strip_prefix(r"\\.\") {
        clean = stripped;
    }

    if let Some(idx) = clean.rfind("#{") {
        clean = &clean[..idx];
    } else if let Some(idx) = clean.rfind('#') {
        let tail = &clean[idx + 1..];
        if tail.contains('-') || tail.len() >= 32 {
            clean = &clean[..idx];
        }
    }

    clean.replace('#', "\\").to_uppercase()
}

/// Converts a list of Rust Strings into a MULTI_SZ UTF-16 byte buffer (double null-terminated).
pub fn string_list_to_multi_sz(strings: &[String]) -> Vec<u16> {
    if strings.is_empty() {
        return vec![0, 0];
    }
    let mut buffer = Vec::new();
    for s in strings {
        if s.is_empty() {
            continue;
        }
        buffer.extend(s.encode_utf16());
        buffer.push(0);
    }
    buffer.push(0);
    buffer
}

/// Parses a MULTI_SZ UTF-16 slice into a list of Rust Strings.
pub fn multi_sz_to_string_list(buffer: &[u16]) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = Vec::new();

    for &ch in buffer {
        if ch == 0 {
            if current.is_empty() {
                // Double null indicates end of MULTI_SZ
                break;
            }
            result.push(String::from_utf16_lossy(&current));
            current.clear();
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        result.push(String::from_utf16_lossy(&current));
    }

    result
}

// ---------------------------------------------------------------------------
// Windows Native Driver Communication
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HANDLE = *mut std::ffi::c_void;
    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x00000080;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(
            lpFileName: *const u16,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *mut std::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: *mut std::ffi::c_void,
        ) -> HANDLE;

        fn CloseHandle(hObject: HANDLE) -> i32;

        fn DeviceIoControl(
            hDevice: HANDLE,
            dwIoControlCode: u32,
            lpInBuffer: *const std::ffi::c_void,
            nInBufferSize: u32,
            lpOutBuffer: *mut std::ffi::c_void,
            nOutBufferSize: u32,
            lpBytesReturned: *mut u32,
            lpOverlapped: *mut std::ffi::c_void,
        ) -> i32;
    }

    pub struct HidHideHandle(HANDLE);

    impl HidHideHandle {
        pub fn open() -> Result<Self, String> {
            let path_wide: Vec<u16> = OsStr::new(r"\\.\HidHide")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            unsafe {
                let handle = CreateFileW(
                    path_wide.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null_mut(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    std::ptr::null_mut(),
                );

                if handle == INVALID_HANDLE_VALUE || handle.is_null() {
                    return Err("HidHide driver not installed or control device not accessible".into());
                }

                Ok(Self(handle))
            }
        }

        pub fn get_active(&self) -> Result<bool, String> {
            let mut active_byte = 0u8;
            let mut returned = 0u32;
            let ok = unsafe {
                DeviceIoControl(
                    self.0,
                    IOCTL_GET_ACTIVE,
                    std::ptr::null(),
                    0,
                    &mut active_byte as *mut _ as *mut _,
                    1,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if ok != 0 {
                Ok(active_byte != 0)
            } else {
                Err("Failed to get HidHide active status".into())
            }
        }

        pub fn set_active(&self, active: bool) -> Result<(), String> {
            let active_byte = if active { 1u8 } else { 0u8 };
            let mut returned = 0u32;
            let ok = unsafe {
                DeviceIoControl(
                    self.0,
                    IOCTL_SET_ACTIVE,
                    &active_byte as *const _ as *const _,
                    1,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if ok != 0 {
                Ok(())
            } else {
                Err("Failed to set HidHide active status".into())
            }
        }

        pub fn get_blacklist(&self) -> Result<Vec<String>, String> {
            self.get_multi_sz(IOCTL_GET_BLACKLIST)
        }

        pub fn set_blacklist(&self, list: &[String]) -> Result<(), String> {
            self.set_multi_sz(IOCTL_SET_BLACKLIST, list)
        }

        pub fn get_whitelist(&self) -> Result<Vec<String>, String> {
            self.get_multi_sz(IOCTL_GET_WHITELIST)
        }

        pub fn set_whitelist(&self, list: &[String]) -> Result<(), String> {
            self.set_multi_sz(IOCTL_SET_WHITELIST, list)
        }

        fn get_multi_sz(&self, ioctl: u32) -> Result<Vec<String>, String> {
            let mut needed = 0u32;
            // First probe for required size
            unsafe {
                let _ = DeviceIoControl(
                    self.0,
                    ioctl,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    0,
                    &mut needed,
                    std::ptr::null_mut(),
                );
            }

            if needed == 0 {
                return Ok(Vec::new());
            }

            let u16_count = (needed as usize + 1) / 2;
            let mut buffer = vec![0u16; u16_count + 4];
            let mut returned = 0u32;

            let ok = unsafe {
                DeviceIoControl(
                    self.0,
                    ioctl,
                    std::ptr::null(),
                    0,
                    buffer.as_mut_ptr() as *mut _,
                    (buffer.len() * 2) as u32,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };

            if ok == 0 {
                return Err(format!("DeviceIoControl failed for IOCTL {ioctl}"));
            }

            Ok(multi_sz_to_string_list(&buffer))
        }

        fn set_multi_sz(&self, ioctl: u32, list: &[String]) -> Result<(), String> {
            let multi_sz = string_list_to_multi_sz(list);
            let mut returned = 0u32;
            let ok = unsafe {
                DeviceIoControl(
                    self.0,
                    ioctl,
                    multi_sz.as_ptr() as *const _,
                    (multi_sz.len() * 2) as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if ok != 0 {
                Ok(())
            } else {
                Err(format!("DeviceIoControl failed to update list for IOCTL {ioctl}"))
            }
        }
    }

    impl Drop for HidHideHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// High Level Public API
// ---------------------------------------------------------------------------

pub fn is_installed() -> bool {
    #[cfg(windows)]
    {
        win::HidHideHandle::open().is_ok()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn get_status() -> HidHideStatus {
    let current_exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    #[cfg(windows)]
    {
        let Ok(handle) = win::HidHideHandle::open() else {
            return HidHideStatus {
                installed: false,
                active: false,
                whitelisted: false,
                hidden_devices: Vec::new(),
                app_path: current_exe,
            };
        };

        let active = handle.get_active().unwrap_or(false);
        let hidden = handle.get_blacklist().unwrap_or_default();
        let whitelist = handle.get_whitelist().unwrap_or_default();
        let exe_lower = current_exe.to_lowercase();
        let whitelisted = whitelist.iter().any(|w| {
            let w_lower = w.to_lowercase();
            w_lower == exe_lower || exe_lower.ends_with(&w_lower) || w_lower.ends_with(&exe_lower)
        });

        HidHideStatus {
            installed: true,
            active,
            whitelisted,
            hidden_devices: hidden,
            app_path: current_exe,
        }
    }

    #[cfg(not(windows))]
    {
        HidHideStatus {
            installed: false,
            active: false,
            whitelisted: false,
            hidden_devices: Vec::new(),
            app_path: current_exe,
        }
    }
}

pub fn auto_whitelist_current_process() -> Result<(), String> {
    #[cfg(windows)]
    {
        let handle = win::HidHideHandle::open()?;
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {e}"))?
            .to_string_lossy()
            .to_string();

        let mut whitelist = handle.get_whitelist().unwrap_or_default();
        let exe_lower = current_exe.to_lowercase();
        if !whitelist.iter().any(|w| w.to_lowercase() == exe_lower) {
            whitelist.push(current_exe);
            handle.set_whitelist(&whitelist)?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

pub fn hide_device(raw_path: &str) -> Result<(), String> {
    let normalized = normalize_device_instance_path(raw_path);
    if normalized.is_empty() {
        return Err("Invalid device path".into());
    }

    #[cfg(windows)]
    {
        let _ = auto_whitelist_current_process();
        let handle = win::HidHideHandle::open()?;
        let mut list = handle.get_blacklist().unwrap_or_default();
        if !list.iter().any(|p| p.eq_ignore_ascii_case(&normalized)) {
            list.push(normalized);
            handle.set_blacklist(&list)?;
        }
        let _ = handle.set_active(true);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = raw_path;
        Ok(())
    }
}

pub fn unhide_device(raw_path: &str) -> Result<(), String> {
    let normalized = normalize_device_instance_path(raw_path);

    #[cfg(windows)]
    {
        let handle = win::HidHideHandle::open()?;
        let mut list = handle.get_blacklist().unwrap_or_default();
        let initial_len = list.len();
        list.retain(|p| !p.eq_ignore_ascii_case(&normalized) && !p.eq_ignore_ascii_case(raw_path));
        if list.len() != initial_len {
            handle.set_blacklist(&list)?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = raw_path;
        Ok(())
    }
}

pub fn set_active(active: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let handle = win::HidHideHandle::open()?;
        handle.set_active(active)
    }
    #[cfg(not(windows))]
    {
        let _ = active;
        Ok(())
    }
}

pub fn install_hidhide_driver(app: &tauri::AppHandle) -> Result<String, String> {
    use std::path::PathBuf;
    use tauri::Manager;

    let candidate_names = [
        "HidHide_Setup.exe",
        "HidHide_1.5.230_x64.exe",
        "HidHideMSI.msi",
        "HidHide.exe",
    ];

    let mut found_installer: Option<PathBuf> = None;

    // 1. Search in Tauri app resource directory
    if let Ok(res_dir) = app.path().resource_dir() {
        for name in &candidate_names {
            let p1 = res_dir.join("resources").join(name);
            let p2 = res_dir.join(name);
            if p1.exists() {
                found_installer = Some(p1);
                break;
            }
            if p2.exists() {
                found_installer = Some(p2);
                break;
            }
        }
    }

    // 2. Search alongside current executable or working directory
    if found_installer.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                for name in &candidate_names {
                    let p1 = parent.join("resources").join(name);
                    let p2 = parent.join(name);
                    let p3 = parent.join("src-tauri").join("resources").join(name);
                    if p1.exists() {
                        found_installer = Some(p1);
                        break;
                    }
                    if p2.exists() {
                        found_installer = Some(p2);
                        break;
                    }
                    if p3.exists() {
                        found_installer = Some(p3);
                        break;
                    }
                }
            }
        }
    }

    // 3. Search in current working directory / src-tauri/resources
    if found_installer.is_none() {
        for name in &candidate_names {
            let p1 = PathBuf::from("src-tauri").join("resources").join(name);
            let p2 = PathBuf::from("resources").join(name);
            let p3 = PathBuf::from(name);
            if p1.exists() {
                found_installer = Some(p1);
                break;
            }
            if p2.exists() {
                found_installer = Some(p2);
                break;
            }
            if p3.exists() {
                found_installer = Some(p3);
                break;
            }
        }
    }

    // 4. If not found locally, download official installer via PowerShell
    let target_path = if let Some(local_path) = found_installer {
        local_path
    } else {
        let temp_exe = std::env::temp_dir().join("HidHide_Setup.exe");
        let dl_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; $ProgressPreference = 'SilentlyContinue'; (New-Object System.Net.WebClient).DownloadFile('https://github.com/nefarius/HidHide/releases/download/v1.5.230.0/HidHide_1.5.230_x64.exe', '{}')",
            temp_exe.to_string_lossy().replace('\'', "''")
        );
        let dl_status = std::process::Command::new("powershell")
            .args(["-Command", &dl_cmd])
            .status()
            .map_err(|e| format!("failed to download HidHide installer: {e}"))?;

        if !dl_status.success() || !temp_exe.exists() {
            // Fallback download URL
            let fallback_url = "https://github.com/nefarius/HidHide/releases/download/v1.5.212.0/HidHide_1.5.212_x64.exe";
            let dl_fallback = format!(
                "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; $ProgressPreference = 'SilentlyContinue'; (New-Object System.Net.WebClient).DownloadFile('{}', '{}')",
                fallback_url,
                temp_exe.to_string_lossy().replace('\'', "''")
            );
            let fallback_status = std::process::Command::new("powershell")
                .args(["-Command", &dl_fallback])
                .status()
                .map_err(|e| format!("failed to download HidHide installer: {e}"))?;

            if !fallback_status.success() || !temp_exe.exists() {
                return Err("Failed to download HidHide installer from GitHub releases. Please check your internet connection or place HidHide_Setup.exe in src-tauri/resources/".into());
            }
        }
        temp_exe
    };

    let is_msi = target_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase() == "msi")
        .unwrap_or(false);

    let install_cmd = if is_msi {
        format!(
            "Start-Process -FilePath 'msiexec.exe' -ArgumentList '/i', '{}', '/norestart' -Verb RunAs -Wait",
            target_path.to_string_lossy().replace('\'', "''")
        )
    } else {
        format!(
            "Start-Process -FilePath '{}' -ArgumentList '/norestart' -Verb RunAs -Wait",
            target_path.to_string_lossy().replace('\'', "''")
        )
    };

    let status = std::process::Command::new("powershell")
        .args(["-Command", &install_cmd])
        .status()
        .map_err(|e| format!("failed to run HidHide installer: {e}"))?;

    let _ = std::process::Command::new("powershell")
        .args(["-Command", "Start-Service HidHide -ErrorAction SilentlyContinue; net start HidHide"])
        .status();

    if status.success() {
        if is_installed() {
            let _ = auto_whitelist_current_process();
            Ok("HidHide driver installed successfully! Anti-double-input protection active.".into())
        } else {
            Ok("HidHide installation completed. If shielding remains inactive, please restart Windows once to load filter driver.".into())
        }
    } else {
        Err("HidHide installation was cancelled or denied Administrator permissions".into())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_device_instance_path() {
        let raw = r"\\?\hid#vid_054c&pid_0ce6&col01#7&3084128&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        let normalized = normalize_device_instance_path(raw);
        assert_eq!(normalized, r"HID\VID_054C&PID_0CE6&COL01\7&3084128&0&0000");
    }

    #[test]
    fn test_multi_sz_roundtrip() {
        let original = vec![
            r"C:\Program Files\PadFlow\padflow.exe".to_string(),
            r"HID\VID_054C&PID_0CE6\12345".to_string(),
        ];
        let encoded = string_list_to_multi_sz(&original);
        let decoded = multi_sz_to_string_list(&encoded);
        assert_eq!(original, decoded);
    }
}
