fn main() {
    let mut windows_attrs = tauri_build::WindowsAttributes::new();

    // Release builds embed a requireAdministrator manifest so Windows asks for
    // elevation at launch — HidHide writes, registry access and driver helpers
    // then always run elevated (no in-app elevation banner needed).
    // Dev builds keep the default manifest so `tauri dev` doesn't trigger UAC.
    if std::env::var("PROFILE").map(|p| p == "release").unwrap_or(false) {
        windows_attrs = windows_attrs.app_manifest(include_str!("windows-app.manifest"));
    }

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows_attrs))
        .expect("failed to run tauri build script");
}
