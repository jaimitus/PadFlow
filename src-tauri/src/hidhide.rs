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

/// Extracts all variations of hardware IDs for a controller (composite collections and base container).
pub fn extract_all_device_instance_ids(path: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let norm = normalize_device_instance_path(path);
    if !norm.is_empty() {
        ids.push(norm.clone());
    }

    // Add base HID container if &COL is present
    if let Some(col_idx) = norm.find("&COL") {
        if let Some(next_slash) = norm[col_idx..].find('\\') {
            let base_hid = format!("{}{}", &norm[..col_idx], &norm[col_idx + next_slash..]);
            if !ids.contains(&base_hid) {
                ids.push(base_hid);
            }
        }
    }

    // Add USB prefix variation if HID is present
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
// Windows In-Process Native Engine (Zero Process Spawning, Zero Console Popups)
// ---------------------------------------------------------------------------

#[cfg(windows)]
pub mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HANDLE = *mut std::ffi::c_void;
    type HKEY = *mut std::ffi::c_void;

    const HKEY_LOCAL_MACHINE: HKEY = 0x80000002usize as HKEY;
    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const FILE_SHARE_DELETE: u32 = 0x00000004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x00000080;

    const KEY_READ: u32 = 0x20019;
    const KEY_WRITE: u32 = 0x20006;
    const REG_DWORD: u32 = 4;
    const REG_MULTI_SZ: u32 = 7;

    const PARAMS_KEY: &str = r"SYSTEM\CurrentControlSet\Services\HidHide\Parameters";

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

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hKey: HKEY,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut HKEY,
        ) -> i32;

        fn RegCreateKeyExW(
            hKey: HKEY,
            lpSubKey: *const u16,
            Reserved: u32,
            lpClass: *mut u16,
            dwOptions: u32,
            samDesired: u32,
            lpSecurityAttributes: *mut std::ffi::c_void,
            phkResult: *mut HKEY,
            lpdwDisposition: *mut u32,
        ) -> i32;

        fn RegQueryValueExW(
            hKey: HKEY,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;

        fn RegSetValueExW(
            hKey: HKEY,
            lpValueName: *const u16,
            Reserved: u32,
            dwType: u32,
            lpData: *const u8,
            cbData: u32,
        ) -> i32;

        fn RegCloseKey(hKey: HKEY) -> i32;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn dos_path_to_device_path(dos_path: &str) -> String {
        let clean = dos_path.trim();
        if clean.starts_with(r"\Device\") {
            return clean.to_string();
        }

        if clean.len() >= 2 && clean.as_bytes()[1] == b':' {
            let drive = &clean[..2];
            let rest = &clean[2..];
            let drive_wide = to_wide(drive);

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

    pub fn is_service_installed() -> bool {
        let sub = to_wide(r"SYSTEM\CurrentControlSet\Services\HidHide");
        let mut key: HKEY = std::ptr::null_mut();
        let ret = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub.as_ptr(), 0, KEY_READ, &mut key) };
        if ret == 0 {
            unsafe { RegCloseKey(key) };
            true
        } else {
            false
        }
    }

    pub fn reg_get_active() -> bool {
        let sub = to_wide(PARAMS_KEY);
        let val_name = to_wide("Active");
        let mut key: HKEY = std::ptr::null_mut();
        let ret = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub.as_ptr(), 0, KEY_READ, &mut key) };
        if ret != 0 {
            return false;
        }

        let mut val = 0u32;
        let mut val_type = 0u32;
        let mut size = 4u32;
        let ret_val = unsafe {
            RegQueryValueExW(
                key,
                val_name.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                &mut val as *mut _ as *mut u8,
                &mut size,
            )
        };
        unsafe { RegCloseKey(key) };
        ret_val == 0 && val == 1
    }

    pub fn reg_set_active(active: bool) -> Result<(), String> {
        let sub = to_wide(PARAMS_KEY);
        let val_name = to_wide("Active");
        let mut key: HKEY = std::ptr::null_mut();
        let ret = unsafe {
            RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                sub.as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE | KEY_READ,
                std::ptr::null_mut(),
                &mut key,
                std::ptr::null_mut(),
            )
        };
        if ret != 0 {
            return Err(format!("Registry access denied (code {ret})"));
        }

        let val: u32 = if active { 1 } else { 0 };
        let ret_val = unsafe {
            RegSetValueExW(
                key,
                val_name.as_ptr(),
                0,
                REG_DWORD,
                &val as *const _ as *const u8,
                4,
            )
        };
        unsafe { RegCloseKey(key) };
        if ret_val == 0 {
            Ok(())
        } else {
            Err(format!("Cannot write Active value (code {ret_val})"))
        }
    }

    pub fn reg_get_multi_sz(name: &str) -> Vec<String> {
        let sub = to_wide(PARAMS_KEY);
        let val_name = to_wide(name);
        let mut key: HKEY = std::ptr::null_mut();
        let ret = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub.as_ptr(), 0, KEY_READ, &mut key) };
        if ret != 0 {
            return Vec::new();
        }

        let mut size = 0u32;
        let mut val_type = 0u32;
        let _ = unsafe {
            RegQueryValueExW(
                key,
                val_name.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                std::ptr::null_mut(),
                &mut size,
            )
        };

        if size == 0 {
            unsafe { RegCloseKey(key) };
            return Vec::new();
        }

        let mut buf = vec![0u8; size as usize + 4];
        let ret_val = unsafe {
            RegQueryValueExW(
                key,
                val_name.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                buf.as_mut_ptr(),
                &mut size,
            )
        };
        unsafe { RegCloseKey(key) };

        if ret_val == 0 {
            let u16_slice: &[u16] = unsafe {
                std::slice::from_raw_parts(buf.as_ptr() as *const u16, size as usize / 2)
            };
            multi_sz_to_string_list(u16_slice)
        } else {
            Vec::new()
        }
    }

    pub fn reg_set_multi_sz(name: &str, list: &[String]) -> Result<(), String> {
        let sub = to_wide(PARAMS_KEY);
        let val_name = to_wide(name);
        let mut key: HKEY = std::ptr::null_mut();
        let ret = unsafe {
            RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                sub.as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE | KEY_READ,
                std::ptr::null_mut(),
                &mut key,
                std::ptr::null_mut(),
            )
        };
        if ret != 0 {
            return Err(format!("Registry access denied for {name} (code {ret})"));
        }

        let multi_sz = string_list_to_multi_sz(list);
        let ret_val = unsafe {
            RegSetValueExW(
                key,
                val_name.as_ptr(),
                0,
                REG_MULTI_SZ,
                multi_sz.as_ptr() as *const u8,
                (multi_sz.len() * 2) as u32,
            )
        };
        unsafe { RegCloseKey(key) };

        if ret_val == 0 {
            Ok(())
        } else {
            Err(format!("Cannot write {name} (code {ret_val})"))
        }
    }

    pub struct HidHideHandle(HANDLE);

    impl HidHideHandle {
        pub fn open() -> Result<Self, String> {
            let path_wide = to_wide(r"\\.\HidHide");

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
                    return Err("HidHide driver not installed or control device not accessible".into());
                }

                Ok(Self(handle))
            }
        }

        pub fn set_active(&self, active: bool) {
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
        }

        pub fn set_blacklist(&self, list: &[String]) {
            let multi_sz = string_list_to_multi_sz(list);
            let mut returned = 0u32;
            let _ = unsafe {
                DeviceIoControl(
                    self.0,
                    IOCTL_SET_BLACKLIST,
                    multi_sz.as_ptr() as *const _,
                    (multi_sz.len() * 2) as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
        }

        pub fn set_whitelist(&self, list: &[String]) {
            let multi_sz = string_list_to_multi_sz(list);
            let mut returned = 0u32;
            let _ = unsafe {
                DeviceIoControl(
                    self.0,
                    IOCTL_SET_WHITELIST,
                    multi_sz.as_ptr() as *const _,
                    (multi_sz.len() * 2) as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
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
// High Level Public API (Zero Subprocess Overheads)
// ---------------------------------------------------------------------------

pub fn is_installed() -> bool {
    #[cfg(windows)]
    {
        win::is_service_installed() || win::HidHideHandle::open().is_ok()
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
        if !is_installed() {
            return HidHideStatus {
                installed: false,
                active: false,
                whitelisted: false,
                hidden_devices: Vec::new(),
                app_path: current_exe,
            };
        }

        let active = win::reg_get_active();
        let hidden = win::reg_get_multi_sz("BlacklistedDeviceInstancePaths");
        let whitelist = win::reg_get_multi_sz("WhitelistedFullImageNames");

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
        let mut whitelist = win::reg_get_multi_sz("WhitelistedFullImageNames");
        let dev_lower = device_path.to_lowercase();
        let exe_lower = current_exe.to_lowercase();

        let mut changed = false;
        if !whitelist.iter().any(|w| w.to_lowercase() == dev_lower) {
            whitelist.push(device_path.clone());
            changed = true;
        }
        if !whitelist.iter().any(|w| w.to_lowercase() == exe_lower) {
            whitelist.push(current_exe.clone());
            changed = true;
        }

        if changed {
            let _ = win::reg_set_multi_sz("WhitelistedFullImageNames", &whitelist);
            if let Ok(handle) = win::HidHideHandle::open() {
                handle.set_whitelist(&whitelist);
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

        let mut list = win::reg_get_multi_sz("BlacklistedDeviceInstancePaths");
        let mut changed = false;
        for id in &ids {
            if !list.iter().any(|p| p.eq_ignore_ascii_case(id)) {
                list.push(id.clone());
                changed = true;
            }
        }

        if changed {
            let _ = win::reg_set_multi_sz("BlacklistedDeviceInstancePaths", &list);
            if let Ok(handle) = win::HidHideHandle::open() {
                handle.set_blacklist(&list);
            }
        }

        let _ = win::reg_set_active(true);
        if let Ok(handle) = win::HidHideHandle::open() {
            handle.set_active(true);
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
        let mut list = win::reg_get_multi_sz("BlacklistedDeviceInstancePaths");
        let initial_len = list.len();
        list.retain(|p| !ids.iter().any(|id| id.eq_ignore_ascii_case(p)) && !p.eq_ignore_ascii_case(raw_path));

        if list.len() != initial_len {
            let _ = win::reg_set_multi_sz("BlacklistedDeviceInstancePaths", &list);
            if let Ok(handle) = win::HidHideHandle::open() {
                handle.set_blacklist(&list);
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
        let _ = win::reg_set_active(active);
        if let Ok(handle) = win::HidHideHandle::open() {
            handle.set_active(active);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = active;
        Ok(())
    }
}

pub fn uncloak_all_controllers() -> Result<HidHideStatus, String> {
    #[cfg(windows)]
    {
        let _ = win::reg_set_multi_sz("BlacklistedDeviceInstancePaths", &[]);
        let _ = win::reg_set_active(false);

        if let Ok(handle) = win::HidHideHandle::open() {
            handle.set_blacklist(&[]);
            handle.set_active(false);
        }

        Ok(get_status())
    }
    #[cfg(not(windows))]
    {
        Ok(get_status())
    }
}

pub fn install_hidhide_driver(app: &tauri::AppHandle) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use tauri::Manager;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let candidate_names = [
        "HidHide_Setup.exe",
        "HidHide_1.5.230_x64.exe",
        "HidHideMSI.msi",
        "HidHide.exe",
    ];

    let mut found_installer: Option<PathBuf> = None;

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

    if found_installer.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                for name in &candidate_names {
                    let p1 = parent.join("resources").join(name);
                    let p2 = parent.join(name);
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
        }
    }

    let target_path = if let Some(local_path) = found_installer {
        local_path
    } else {
        let temp_exe = std::env::temp_dir().join("HidHide_Setup.exe");
        let dl_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; $ProgressPreference = 'SilentlyContinue'; (New-Object System.Net.WebClient).DownloadFile('https://github.com/nefarius/HidHide/releases/download/v1.5.230.0/HidHide_1.5.230_x64.exe', '{}')",
            temp_exe.to_string_lossy().replace('\'', "''")
        );
        let mut dl = std::process::Command::new("powershell");
        #[cfg(windows)]
        dl.creation_flags(CREATE_NO_WINDOW);

        let dl_status = dl
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
            let mut dl2 = std::process::Command::new("powershell");
            #[cfg(windows)]
            dl2.creation_flags(CREATE_NO_WINDOW);

            let fallback_status = dl2
                .args(["-Command", &dl_fallback])
                .status()
                .map_err(|e| format!("failed to download HidHide installer: {e}"))?;

            if !fallback_status.success() || !temp_exe.exists() {
                return Err("Failed to download HidHide installer from GitHub releases. Please check internet connection or place HidHide_Setup.exe in src-tauri/resources/".into());
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

    let mut install_proc = std::process::Command::new("powershell");
    #[cfg(windows)]
    install_proc.creation_flags(CREATE_NO_WINDOW);

    let status = install_proc
        .args(["-Command", &install_cmd])
        .status()
        .map_err(|e| format!("failed to run HidHide installer: {e}"))?;

    let mut start_svc = std::process::Command::new("powershell");
    #[cfg(windows)]
    start_svc.creation_flags(CREATE_NO_WINDOW);
    let _ = start_svc
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
