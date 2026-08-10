# Build & Deployment Guide - Native Driver

This guide covers building, packaging, and deploying the native Rust driver alongside your Electron app.

## 🏗️ Build Process Overview

```
┌─────────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Build Rust Driver  │ --> │  Copy to dist/   │ --> │  Package with    │
│  (cargo build)      │     │  or resources/   │     │  electron-builder│
└─────────────────────┘     └──────────────────┘     └─────────────────┘
         │                           │                        │
         v                           v                        v
   mouse_driver_daemon         Platform-specific        Single installer
   (.exe, .so, .dylib)       binaries in app bundle   for end users
```

## 📦 Step-by-Step Build Instructions

### 1. Build Native Driver for All Platforms

#### Windows (x64)
```bash
cd mouse_driver
cargo build --release --target x86_64-pc-windows-msvc
```

Output: `target/x86_64-pc-windows-msvc/release/mouse_driver_daemon.exe`

#### Linux (x64)
```bash
cd mouse_driver
cargo build --release --target x86_64-unknown-linux-gnu
```

Output: `target/x86_64-unknown-linux-gnu/release/mouse_driver_daemon`

#### macOS (Intel & Apple Silicon)
```bash
# Intel
cargo build --release --target x86_64-apple-darwin

# Apple Silicon (M1/M2)
cargo build --release --target aarch64-apple-darwin

# Universal binary (both architectures)
lipo -create \
  target/x86_64-apple-darwin/release/mouse_driver_daemon \
  target/aarch64-apple-darwin/release/mouse_driver_daemon \
  -output target/universal-macos/mouse_driver_daemon
```

### 2. Cross-Compilation Setup (Optional)

For building all platforms from a single machine:

#### Install Cross-Compilation Targets
```bash
# Windows from Linux
rustup target add x86_64-pc-windows-msvc

# macOS from Linux (requires OSXCross)
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

#### Use Cargo Cross (Docker-based)
```bash
cargo install cargo-cross

# Build for Windows
cargo cross build --release --target x86_64-pc-windows-msvc

# Build for macOS
cargo cross build --release --target x86_64-apple-darwin
```

### 3. Integrate with Electron Build

Update your `package.json`:

```json
{
  "scripts": {
    "build:driver:win": "cd mouse_driver && cargo build --release --target x86_64-pc-windows-msvc",
    "build:driver:linux": "cd mouse_driver && cargo build --release --target x86_64-unknown-linux-gnu",
    "build:driver:mac": "cd mouse_driver && cargo build --release --target x86_64-apple-darwin && cargo build --release --target aarch64-apple-darwin",
    "build:driver:all": "npm run build:driver:win && npm run build:driver:linux && npm run build:driver:mac",
    "copy:driver:win": "mkdirp dist/native/win && copy mouse_driver\\target\\x86_64-pc-windows-msvc\\release\\mouse_driver_daemon.exe dist\\native\\win\\",
    "copy:driver:linux": "mkdirp dist/native/linux && cp mouse_driver/target/x86_64-unknown-linux-gnu/release/mouse_driver_daemon dist/native/linux/",
    "copy:driver:mac": "mkdirp dist/native/mac && cp mouse_driver/target/*/release/mouse_driver_daemon dist/native/mac/",
    "build:all": "npm run build:driver:all && npm run copy:driver:all && npm run build:electron",
    "build:electron": "tsc && vite build"
  },
  "build": {
    "appId": "com.padflow.app",
    "productName": "PadFlow",
    "extraResources": [
      "dist/native/**/*"
    ],
    "win": {
      "target": ["nsis", "portable"],
      "artifactName": "${productName}-${version}-Setup.${ext}"
    },
    "linux": {
      "target": ["AppImage", "deb"],
      "category": "Utility"
    },
    "mac": {
      "target": ["dmg", "zip"],
      "entitlements": "build/entitlements.mac.plist"
    }
  }
}
```

### 4. Create Build Script (Recommended)

Create `scripts/build-native.js`:

```javascript
const { execSync } = require('child_process');
const fs = require('fs-extra');
const path = require('path');

const PLATFORMS = {
  win32: {
    target: 'x86_64-pc-windows-msvc',
    binary: 'mouse_driver_daemon.exe',
    dest: 'dist/native/win'
  },
  linux: {
    target: 'x86_64-unknown-linux-gnu',
    binary: 'mouse_driver_daemon',
    dest: 'dist/native/linux'
  },
  darwin: {
    targets: ['x86_64-apple-darwin', 'aarch64-apple-darwin'],
    binary: 'mouse_driver_daemon',
    dest: 'dist/native/mac'
  }
};

async function build() {
  const platform = process.platform;
  const config = PLATFORMS[platform];
  
  if (!config) {
    console.error(`Unsupported platform: ${platform}`);
    process.exit(1);
  }

  console.log(`🦀 Building native driver for ${platform}...`);

  if (platform === 'darwin') {
    // Build both architectures on macOS
    for (const target of config.targets) {
      console.log(`Building for ${target}...`);
      execSync(`cargo build --release --target ${target}`, {
        cwd: 'mouse_driver',
        stdio: 'inherit'
      });
    }
    
    // Create universal binary
    fs.ensureDirSync(config.dest);
    execSync(`lipo -create \
      mouse_driver/target/x86_64-apple-darwin/release/${config.binary} \
      mouse_driver/target/aarch64-apple-darwin/release/${config.binary} \
      -output ${config.dest}/${config.binary}`);
  } else {
    execSync(`cargo build --release --target ${config.target}`, {
      cwd: 'mouse_driver',
      stdio: 'inherit'
    });
    
    // Copy binary
    fs.ensureDirSync(config.dest);
    fs.copySync(
      `mouse_driver/target/${config.target}/release/${config.binary}`,
      `${config.dest}/${config.binary}`
    );
  }

  console.log('✅ Native driver built successfully!');
}

build().catch(console.error);
```

Add to package.json:
```json
{
  "scripts": {
    "build:native": "node scripts/build-native.js"
  }
}
```

## 🚀 Deployment Strategies

### Option A: Bundle with Installer (Recommended)

Include the native driver in your Electron app package:

```typescript
// main.ts - Auto-start daemon
import { spawn } from 'child_process';
import { app } from 'electron';

function startNativeDriver() {
  const platform = process.platform;
  const daemonName = platform === 'win32' 
    ? 'mouse_driver_daemon.exe' 
    : 'mouse_driver_daemon';
  
  const daemonPath = path.join(
    process.resourcesPath || __dirname,
    'native',
    platform,
    daemonName
  );

  const daemon = spawn(daemonPath, [], {
    stdio: 'ignore',
    detached: false
  });

  app.on('will-quit', () => {
    daemon.kill();
  });
}
```

### Option B: Separate System Service

Install as a system service for elevated privileges:

#### Windows (PowerShell Installer)
```powershell
# install-service.ps1
$daemonPath = "C:\Program Files\PadFlow\native\win\mouse_driver_daemon.exe"
$serviceName = "PadFlowNativeDriver"

New-Service -Name $serviceName `
  -BinaryPathName "`"$daemonPath`"" `
  -DisplayName "PadFlow Native Driver Service" `
  -Description "High-performance HID driver for PadFlow" `
  -StartupType Automatic

Start-Service $serviceName
```

#### Linux (systemd Service)
```ini
# /etc/systemd/system/padflow-driver.service
[Unit]
Description=PadFlow Native Mouse Driver
After=graphical.target

[Service]
Type=simple
ExecStart=/opt/padflow/native/linux/mouse_driver_daemon
Restart=on-failure
User=root
CapabilityBoundingSet=CAP_SYS_ADMIN CAP_NET_ADMIN
AmbientCapabilities=CAP_SYS_ADMIN CAP_NET_ADMIN

[Install]
WantedBy=multi-user.target
```

Install:
```bash
sudo systemctl enable padflow-driver
sudo systemctl start padflow-driver
```

#### macOS (LaunchDaemon)
```xml
<!-- /Library/LaunchDaemons/com.padflow.driver.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" 
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.padflow.driver</string>
  <key>ProgramArguments</key>
  <array>
    <string>/Applications/PadFlow.app/Contents/Resources/native/mac/mouse_driver_daemon</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/var/log/padflow-driver.log</string>
  <key>StandardErrorPath</key>
  <string>/var/log/padflow-driver.err</string>
</dict>
</plist>
```

Install:
```bash
sudo launchctl load /Library/LaunchDaemons/com.padflow.driver.plist
```

## 🔐 Code Signing & Notarization

### Windows (Authenticode)

```bash
# Purchase certificate from Sectigo, DigiCert, etc.
signtool sign /f certificate.pfx /p password /tr http://timestamp.digicert.com /td sha256 /fd sha256 \
  dist/native/win/mouse_driver_daemon.exe
```

### macOS (Notarization)

```bash
# Sign binary
codesign --force --sign "Developer ID Application: Your Name" \
  --options runtime \
  --entitlements build/entitlements.mac.plist \
  dist/native/mac/mouse_driver_daemon

# Notarize
xcrun notarytool submit dist/native/mac/mouse_driver_daemon \
  --apple-id "your@email.com" \
  --password "@keychain:AC_PASSWORD" \
  --team-id "YOUR_TEAM_ID" \
  --wait

# Staple ticket
xcrun stapler staple dist/native/mac/mouse_driver_daemon
```

### Linux (No signing required, but consider GPG)

```bash
# Sign release tarball
gpg --detach-sign --armor dist/native/linux/mouse_driver_daemon.tar.gz
```

## 📊 Binary Size Optimization

Reduce binary size in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = 'abort'
```

Additional size reduction:
```bash
# Strip symbols manually
strip target/release/mouse_driver_daemon

# Use UPX compression (optional, may trigger antivirus)
upx --best target/release/mouse_driver_daemon
```

Typical sizes:
- Debug: ~50 MB
- Release (optimized): ~2-5 MB
- Release + UPX: ~1-2 MB

## 🧪 Testing Before Deployment

```bash
# Test locally before packaging
npm run build:native
npm run build:electron
npm run start

# Verify daemon starts
ps aux | grep mouse_driver_daemon

# Check IPC communication
netstat -an | grep mouse_driver  # Windows
ls -la /tmp/mouse_driver.sock    # Linux/macOS
```

## 📝 Release Checklist

- [ ] Build native driver for all target platforms
- [ ] Run tests: `cargo test --release`
- [ ] Sign binaries (Windows/macOS)
- [ ] Notarize macOS binary
- [ ] Test IPC communication on each platform
- [ ] Verify auto-start functionality
- [ ] Test clean uninstall
- [ ] Update README with release notes
- [ ] Create GitHub release with binaries
- [ ] Announce on Discord/community

---

**🎉 Ready to ship!** Your zero-dependency native driver is packaged and ready for distribution.
