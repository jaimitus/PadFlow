//! PadFlow — HidHide driver client and anti-double-input protection.
//!
//! HidHide is a kernel-mode filter driver by Nefarius that acts as a device firewall.
//! It hides physical PlayStation (DS4 / DualSense) controllers from games and third-party
//! applications while allowing whitelisted feeder applications (PadFlow) to read them and
//! expose a single virtual XInput controller via ViGEmBus.

use std::path::PathBuf;
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
const FILE_ANY_ACCESS: u32 = 0;

pub const IOCTL_GET_WHITELIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2048, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_SET_WHITELIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2049, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_GET_BLACKLIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2050, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_SET_BLACKLIST: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2051, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_GET_ACTIVE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2052, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_SET_ACTIVE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2053, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_GET_WLINVERSE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2054, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_SET_WLINVERSE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 2055, METHOD_BUFFERED, FILE_ANY_ACCESS);

// ---------------------------------------------------------------------------
// Path & Multi-SZ Helpers
// ---------------------------------------------------------------------------

/// Normalizes a hidapi device path into standard Windows Device Instance Paths.
/// Handles both raw HID interface paths and generates base hardware container IDs.
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

/// Extracts multiple potential match variations for a controller (e.g. HID and base USB/Bluetooth).
pub fn extract_all_device_instance_ids(path: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let norm = normalize_device_instance_path(path);
    if !norm.is_empty() {
        ids.push(norm.clone());
    }

    // If it is a composite HID (e.g. HID\VID_054C&PID_0CE6&COL01\...), also add base HID without &COL
    if let Some(col_idx) = norm.find("&COL") {
        if let Some(next_slash) = norm[col_idx..].find('\\') {
            let base_hid = format!("{}{}", &norm[..col_idx], &norm[col_idx + next_slash..]);
            if !ids.contains(&base_hid) {
                ids.push(base_hid);
            }
        }
    }

    // Also support USB prefix if HID is present
    if norm.starts_with(r"HID\VID_") {
        let usb_id = norm.replacen(r"HID\", r"USB\", 1);
        if !ids.contains(&usb_id) {
            ids.push(usb_id);
        }
    }

    ids
}

pub fn string_list_to_multi_sz(strings: &[String]) -> Vec<u16> {
    if strings.is_empty() {
        return vec![0, 0];
    }
    let mut buffer = Vec::new();
    for s in strings {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            continue;
        }
        buffer.extend(trimmed.encode_utf16());
        buffer.push(0);
    }
    buffer.push(0);
    buffer
}

pub fn multi_sz_to_string_list(buffer: &[u16]) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = Vec::new();

    for &ch in buffer {
        if ch == 0 {
            if current.is_empty() {
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
// Windows Native Driver Communication (CLI + Registry + IOCTL)
// ---------------------------------------------------------------------------

#[cfg(windows)]
pub mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HANDLE = *mut std::ffi::c_void;
    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const FILE_SHARE_DELETE: u32 = 0x00000004;
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
        fn GetLastError() -> u32;
        fn QueryDosDeviceW(lpDeviceName: *const u16, lpTargetPath: *mut u16, ucchMax: u32) -> u32;

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

    pub fn dos_path_to_device_path(dos_path: &str) -> String {
        let clean = dos_path.trim();
        if clean.starts_with(r"\Device\") {
            return clean.to_string();
        }

        if clean.len() >= 2 && clean.as_bytes()[1] == b':' {
            let drive = &clean[..2];
            let rest = &clean[2..];

            let drive_wide: Vec<u16> = OsStr::new(drive)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut target_buf = vec![0u16; 512];
            let len = unsafe {
                QueryDosDeviceW(
                    drive_wide.as_ptr(),
                    target_buf.as_mut_ptr(),
                    target_buf.len() as u32,
                )
            };

            if len > 0 {
                let device_prefix = String::from_utf16_lossy(&target_buf[..len as usize])
                    .trim_matches('\0')
                    .to_string();
                return format!("{}{}", device_prefix, rest);
            }
        }

        clean.to_string()
    }

    pub fn find_hidhide_cli() -> Option<PathBuf> {
        let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
        let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into());

        let candidates = [
            format!(r"{program_files}\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe"),
            format!(r"{program_files}\Nefarius Software Solutions\HidHide\HidHideCLI.exe"),
            format!(r"{program_files}\Nefarius Software Solutions e.U\HidHideCLI\HidHideCLI.exe"),
            format!(r"{program_files}\Nefarius Software Solutions e.U\HidHide\x64\HidHideCLI.exe"),
            format!(r"{program_files}\Nefarius Software Solutions e.U\HidHide\HidHideCLI.exe"),
            format!(r"{program_files}\Nefarius\HidHide\x64\HidHideCLI.exe"),
            format!(r"{program_files}\Nefarius\HidHide\HidHideCLI.exe"),
            format!(r"{program_files}\HidHide\x64\HidHideCLI.exe"),
            format!(r"{program_files}\HidHide\HidHideCLI.exe"),
            format!(r"{program_files_x86}\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe"),
            format!(r"{program_files_x86}\Nefarius Software Solutions\HidHide\HidHideCLI.exe"),
            r"C:\Program Files\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe".into(),
            r"C:\Program Files\Nefarius Software Solutions\HidHide\HidHideCLI.exe".into(),
            r"C:\Program Files\Nefarius Software Solutions e.U\HidHideCLI\HidHideCLI.exe".into(),
        ];

        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }

        if let Ok(output) = std::process::Command::new("where").arg("HidHideCLI.exe").output() {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = out_str.lines().next() {
                    let p = PathBuf::from(first_line.trim());
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }

        None
    }

    pub fn cli_get_cloak_state() -> Option<bool> {
        let cli = find_hidhide_cli()?;
        let output = std::process::Command::new(cli).arg("--cloak-state").output().ok()?;
        let txt = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if txt.contains("cloak-on") {
            Some(true)
        } else if txt.contains("cloak-off") {
            Some(false)
        } else {
            None
        }
    }

    pub fn cli_get_blacklist() -> Option<Vec<String>> {
        let cli = find_hidhide_cli()?;
        let output = std::process::Command::new(cli).arg("--dev-list").output().ok()?;
        let txt = String::from_utf8_lossy(&output.stdout);
        let mut list = Vec::new();
        for line in txt.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("HID\\") || trimmed.starts_with("USB\\") || trimmed.starts_with("BTHENUM\\") {
                list.push(trimmed.to_string());
            }
        }
        Some(list)
    }

    pub fn cli_get_gaming_devices() -> Option<Vec<String>> {
        let cli = find_hidhide_cli()?;
        let output = std::process::Command::new(cli).arg("--dev-gaming").output().ok()?;
        let txt = String::from_utf8_lossy(&output.stdout);
        let mut list = Vec::new();
        for line in txt.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("HID\\") || trimmed.starts_with("USB\\") || trimmed.starts_with("BTHENUM\\") {
                list.push(trimmed.to_string());
            }
        }
        Some(list)
    }

    pub fn cli_get_whitelist() -> Option<Vec<String>> {
        let cli = find_hidhide_cli()?;
        let output = std::process::Command::new(cli).arg("--app-list").output().ok()?;
        let txt = String::from_utf8_lossy(&output.stdout);
        let mut list = Vec::new();
        for line in txt.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && (trimmed.contains('\\') || trimmed.contains('/')) {
                list.push(trimmed.to_string());
            }
        }
        Some(list)
    }

    pub struct HidHideHandle(HANDLE);

    impl HidHideHandle {
        pub fn open() -> Result<Self, String> {
            let path_wide: Vec<u16> = OsStr::new(r"\\.\HidHide")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            unsafe {
                let mut handle = CreateFileW(
                    path_wide.as_ptr(),
                    GENERIC_READ,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    std::ptr::null_mut(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    std::ptr::null_mut(),
                );

                if handle == INVALID_HANDLE_VALUE || handle.is_null() {
                    handle = CreateFileW(
                        path_wide.as_ptr(),
                        GENERIC_READ | GENERIC_WRITE,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        std::ptr::null_mut(),
                        OPEN_EXISTING,
                        FILE_ATTRIBUTE_NORMAL,
                        std::ptr::null_mut(),
                    );
                }

                if handle == INVALID_HANDLE_VALUE || handle.is_null() {
                    let err = GetLastError();
                    return Err(format!(
                        "HidHide driver not installed or control device not accessible (Win32 error {err})"
                    ));
                }

                Ok(Self(handle))
            }
        }

        pub fn get_active(&self) -> Result<bool, String> {
            if let Some(cli_state) = cli_get_cloak_state() {
                return Ok(cli_state);
            }

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
                return Ok(active_byte != 0);
            }

            Ok(false)
        }

        pub fn set_active(&self, active: bool) -> Result<(), String> {
            if let Some(cli) = find_hidhide_cli() {
                let arg = if active { "--cloak-on" } else { "--cloak-off" };
                let _ = std::process::Command::new(cli).arg(arg).status();
            }

            let active_byte = if active { 1u8 } else { 0u8 };
            let mut returned = 0u32;
            let _ = unsafe {
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

            Ok(())
        }

        pub fn get_blacklist(&self) -> Result<Vec<String>, String> {
            if let Some(cli_list) = cli_get_blacklist() {
                if !cli_list.is_empty() {
                    return Ok(cli_list);
                }
            }
            self.get_multi_sz(IOCTL_GET_BLACKLIST)
        }

        pub fn set_blacklist(&self, list: &[String]) -> Result<(), String> {
            if let Some(cli) = find_hidhide_cli() {
                for item in list {
                    let _ = std::process::Command::new(&cli)
                        .args(["--dev-hide", item])
                        .status();
                }
            }
            let _ = self.set_multi_sz(IOCTL_SET_BLACKLIST, list);
            Ok(())
        }

        pub fn get_whitelist(&self) -> Result<Vec<String>, String> {
            if let Some(cli_apps) = cli_get_whitelist() {
                if !cli_apps.is_empty() {
                    return Ok(cli_apps);
                }
            }
            self.get_multi_sz(IOCTL_GET_WHITELIST)
        }

        pub fn set_whitelist(&self, list: &[String]) -> Result<(), String> {
            if let Some(cli) = find_hidhide_cli() {
                for app in list {
                    let _ = std::process::Command::new(&cli)
                        .args(["--app-reg", app])
                        .status();
                }
            }
            let _ = self.set_multi_sz(IOCTL_SET_WHITELIST, list);
            Ok(())
        }

        fn get_multi_sz(&self, ioctl: u32) -> Result<Vec<String>, String> {
            let mut buffer = vec![0u16; 8192];
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

            if ok != 0 {
                return Ok(multi_sz_to_string_list(&buffer));
            }

            Ok(Vec::new())
        }

        fn set_multi_sz(&self, ioctl: u32, list: &[String]) -> Result<(), String> {
            let multi_sz = string_list_to_multi_sz(list);
            let mut returned = 0u32;
            let _ = unsafe {
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
            Ok(())
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
        win::find_hidhide_cli().is_some() || win::HidHideHandle::open().is_ok()
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
        let cli_found = win::find_hidhide_cli().is_some();
        let handle_res = win::HidHideHandle::open();

        if !cli_found && handle_res.is_err() {
            return HidHideStatus {
                installed: false,
                active: false,
                whitelisted: false,
                hidden_devices: Vec::new(),
                app_path: current_exe,
            };
        }

        let active = if let Some(cli_state) = win::cli_get_cloak_state() {
            cli_state
        } else if let Ok(ref h) = handle_res {
            h.get_active().unwrap_or(false)
        } else {
            false
        };

        let hidden = if let Some(cli_devs) = win::cli_get_blacklist() {
            cli_devs
        } else if let Ok(ref h) = handle_res {
            h.get_blacklist().unwrap_or_default()
        } else {
            Vec::new()
        };

        let whitelist = if let Some(cli_apps) = win::cli_get_whitelist() {
            cli_apps
        } else if let Ok(ref h) = handle_res {
            h.get_whitelist().unwrap_or_default()
        } else {
            Vec::new()
        };

        let exe_lower = current_exe.to_lowercase();
        let dev_lower = win::dos_path_to_device_path(&current_exe).to_lowercase();

        let whitelisted = whitelist.iter().any(|w| {
            let w_lower = w.to_lowercase();
            w_lower == exe_lower
                || w_lower == dev_lower
                || exe_lower.ends_with(&w_lower)
                || w_lower.ends_with(&exe_lower)
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
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {e}"))?
            .to_string_lossy()
            .to_string();

        let device_path = win::dos_path_to_device_path(&current_exe);

        if let Some(cli) = win::find_hidhide_cli() {
            let _ = std::process::Command::new(&cli)
                .args(["--app-reg", &device_path])
                .status();
            let _ = std::process::Command::new(&cli)
                .args(["--app-reg", &current_exe])
                .status();
        }

        if let Ok(handle) = win::HidHideHandle::open() {
            let mut whitelist = handle.get_whitelist().unwrap_or_default();
            let dev_lower = device_path.to_lowercase();
            let exe_lower = current_exe.to_lowercase();

            let mut changed = false;
            if !whitelist.iter().any(|w| w.to_lowercase() == dev_lower) {
                whitelist.push(device_path);
                changed = true;
            }
            if !whitelist.iter().any(|w| w.to_lowercase() == exe_lower) {
                whitelist.push(current_exe);
                changed = true;
            }

            if changed {
                let _ = handle.set_whitelist(&whitelist);
            }
        }

        Ok(())
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

pub fn hide_device(raw_path: &str) -> Result<(), String> {
    let ids = extract_all_device_instance_ids(raw_path);
    if ids.is_empty() {
        return Err("Invalid device path".into());
    }

    #[cfg(windows)]
    {
        let _ = auto_whitelist_current_process();

        // 1. Hide via official CLI
        if let Some(cli) = win::find_hidhide_cli() {
            for id in &ids {
                let _ = std::process::Command::new(&cli)
                    .args(["--dev-hide", id, "--cloak-on"])
                    .status();
            }
        }

        // 2. Hide via IOCTL
        if let Ok(handle) = win::HidHideHandle::open() {
            let mut list = handle.get_blacklist().unwrap_or_default();
            let mut changed = false;
            for id in &ids {
                if !list.iter().any(|p| p.eq_ignore_ascii_case(id)) {
                    list.push(id.clone());
                    changed = true;
                }
            }
            if changed {
                let _ = handle.set_blacklist(&list);
            }
            let _ = handle.set_active(true);
        }

        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = raw_path;
        Ok(())
    }
}

pub fn unhide_device(raw_path: &str) -> Result<(), String> {
    let ids = extract_all_device_instance_ids(raw_path);

    #[cfg(windows)]
    {
        if let Some(cli) = win::find_hidhide_cli() {
            for id in &ids {
                let _ = std::process::Command::new(&cli)
                    .args(["--dev-unhide", id])
                    .status();
            }
        }

        if let Ok(handle) = win::HidHideHandle::open() {
            let mut list = handle.get_blacklist().unwrap_or_default();
            let initial_len = list.len();
            list.retain(|p| !ids.iter().any(|id| id.eq_ignore_ascii_case(p)) && !p.eq_ignore_ascii_case(raw_path));
            if list.len() != initial_len {
                let _ = handle.set_blacklist(&list);
            }
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
        if let Some(cli) = win::find_hidhide_cli() {
            let arg = if active { "--cloak-on" } else { "--cloak-off" };
            let _ = std::process::Command::new(cli).arg(arg).status();
        }

        if let Ok(handle) = win::HidHideHandle::open() {
            let _ = handle.set_active(active);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = active;
        Ok(())
    }
}

pub fn cloak_all_gaming_controllers() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        let _ = auto_whitelist_current_process();
        let mut hidden = Vec::new();

        if let Some(gaming_devs) = win::cli_get_gaming_devices() {
            if let Some(cli) = win::find_hidhide_cli() {
                for dev in &gaming_devs {
                    let _ = std::process::Command::new(&cli)
                        .args(["--dev-hide", dev])
                        .status();
                    hidden.push(dev.clone());
                }
                let _ = std::process::Command::new(&cli).arg("--cloak-on").status();
            }
        }

        let _ = set_active(true);
        Ok(hidden)
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

pub fn install_hidhide_driver(app: &tauri::AppHandle) -> Result<String, String> {
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
