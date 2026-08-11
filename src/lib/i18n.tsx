// PadFlow i18n — lightweight ES/EN localization (v1.2.5).
// No external dependency: a React context + flat dictionaries + interpolation.
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

export type Lang = "en" | "es";

const STORAGE_KEY = "padflow-lang";

type Dict = Record<string, string>;

const EN: Dict = {
  // ---- header / chips ----------------------------------------------------
  "app.tagline": "DualShock 4 / DualSense → XInput bridge · ViGEmBus · HidHide Shield · <15 MB RAM",
  "chip.poll": "POLL",
  "chip.latency": "LATENCY",
  "chip.virtualPad": "VIRTUAL PAD",
  "chip.hidhide": "HIDHIDE",
  "chip.mode": "MODE",
  "chip.x360Online": "X360 ONLINE",
  "chip.offline": "OFFLINE",
  "chip.driverReady": "DRIVER READY 🛡️",
  "chip.notInstalled": "NOT INSTALLED",
  "chip.nativeHid": "NATIVE HID",
  "chip.webPreview": "WEB PREVIEW",
  "checkUpdate": "Check update",
  "checking": "Checking…",
  "updateReady": "Update v{version} ready",
  "tab.studio": "Studio",
  "tab.source": "Rust core",
  "stopEngine": "STOP ENGINE",
  "startEngine": "START ENGINE",
  "lang.toggle": "ES",

  // ---- banners -----------------------------------------------------------
  "banner.vigemTitle": "ViGEmBus Driver Required for Virtual Xbox 360 Pad",
  "banner.vigemText":
    "Input curves and live monitoring work, but games require ViGEmBus to receive mapped inputs.",
  "banner.installVigem": "🛠️ INSTALL VIGEMBUS DRIVER",
  "banner.adminTitle": "Administrator mode helps when HidHide rejects changes",
  "banner.adminText":
    "Cloaking usually works without it, but if HidHide refuses a write, restart as Administrator to force the registry fallback.",
  "banner.restartAsAdmin": "🔄 RESTART AS ADMINISTRATOR",
  "banner.hidhideTitle": "HidHide Driver Recommended — Prevent Double-Input Conflicts",
  "banner.hidhideText":
    "Hides physical DirectInput controllers from games so only the virtual Xbox pad is detected.",
  "banner.installHidhide": "🛡️ INSTALL HIDHIDE DRIVER",

  // ---- left column -------------------------------------------------------
  "section.controllers": "Controllers",
  "section.detected": "{n} detected",
  "noController": "No controller detected",
  "noControllerHint": "Connect a DualShock 4 / DualSense over USB or Bluetooth",
  "section.shield": "Anti-Double Input Shield",
  "shield.active": "ACTIVE 🛡️",
  "shield.paused": "PAUSED",
  "shield.notInstalled": "NOT INSTALLED",
  "shield.desc":
    "Hides physical PlayStation controllers from games so only the virtual Xbox 360 pad is detected — no more double-input.",
  "shield.driverNotDetected": "Driver not detected",
  "shield.global": "Global shield",
  "shield.globalSubActive": "firewall intercepting HID reports",
  "shield.globalSubPaused": "firewall paused — controllers visible",
  "shield.cloakAll": "🛡️ CLOAK ALL",
  "shield.uncloakAll": "◉ UNCLOAK ALL",
  "shield.whitelist": "Whitelist:",
  "shield.authorized": "PadFlow Authorized",
  "shield.notWhitelisted": "NOT WHITELISTED",
  "shield.hiddenDevices": "Hidden devices ({n})",
  "shield.nothingHidden": "Nothing hidden — games see your physical controllers directly.",
  "shield.autoCloak": "Auto-cloak on connect",
  "shield.autoCloakHint": "hide new PlayStation pads as they plug in",
  "shield.cloakStart": "Cloak on startup",
  "shield.cloakStartHint": "hide already-connected pads at launch (requires the global shield)",
  "shield.openClient": "⚙️ OPEN HIDHIDE CLIENT (OFFICIAL GUI)",
  "section.engineExtras": "Engine & Extras",
  "stat.reports": "Reports",
  "stat.peakLat": "Peak lat.",
  "stat.dropped": "Dropped",
  "stat.reconnects": "Reconnects",
  "rumble.intensity": "Rumble intensity",
  "turbo.label": "Turbo polling (1 kHz)",
  "turbo.hint": "pins HID loop to sub-millisecond thread priority",
  "touchpad.label": "Touchpad as Virtual Mouse",
  "touchpad.hint": "1-finger move/click · 2-finger scroll",
  "touchpad.sens": "Touchpad Sensitivity",
  "batteryLed.label": "Smart Battery Lightbar",
  "batteryLed.hint": "dynamic LED color (Green > 60%, Amber, Red)",

  // ---- right column ------------------------------------------------------
  "matrix.title": "Stick response matrix",
  "matrix.right": "input magnitude → virtual output",
  "curve.left": "Left stick · movement",
  "curve.right": "Right stick · aim",
  "tuner.left": "Left stick tuner",
  "tuner.right": "Right stick tuner",

  // ---- footer ------------------------------------------------------------
  "footer.about": "PadFlow v{version} · open source · github.com/jaimitus/PadFlow · Windows 10 / 11 · ViGEmBus 1.22+ · HidHide Support",
  "footer.activePad": "active pad:",

  // ---- update modal ------------------------------------------------------
  "update.available": "Update available",
  "update.fromTo": "PadFlow v{from} → v{to}",
  "update.released": "A new PadFlow release is published on GitHub.",
  "update.applying": "Applying update…",
  "update.downloading": "Downloading update…",
  "update.restart": "🔄 Restart PadFlow now",
  "update.downloadInstall": "⬇ Download & install",
  "update.viewRelease": "View release",
  "update.later": "Later",
  "update.notNow": "Not now",
  "update.keepOpen": "Keep PadFlow open while the update installs…",

  // ---- GamepadCard -------------------------------------------------------
  "conn.usb": "USB · WIRED",
  "conn.bt": "BLUETOOTH",
  "badge.cloaked": "🛡️ CLOAKED",
  "badge.visible": "VISIBLE",
  "slot.active": "active slot",
  "slot.mapped": "mapped",
  "hiddenFromGames": "HIDDEN FROM GAMES",
  "visibleToGames": "VISIBLE TO GAMES",
  "cloak.btn": "🙈 CLOAK",
  "uncloak.btn": "🛡️ UNCLOAK",
  "battery.label": "BATTERY",
  "battery.na": "n/a",
  "battery.charging": "⚡ charging",
  "lightbar.rgb": "LIGHTBAR RGB",
  "rumble.test": "TEST RUMBLE",

  // ---- DeadzoneTuner -----------------------------------------------------
  "dz.inner": "Inner deadzone",
  "dz.innerHint": "kills stick drift at rest",
  "dz.outer": "Outer deadzone",
  "dz.outerHint": "magnitude that reaches 100% output",
  "dz.anti": "Anti-deadzone",
  "dz.antiHint": "compensates the in-game deadzone",
  "dz.power": "Curve power",
  "dz.powerHint": "steepness of the response ramp",
  "dz.sens": "Sensitivity",
  "dz.sensHint": "output gain multiplier",

  // ---- TriggerTuner ------------------------------------------------------
  "trigger.title": "Trigger Matrix & Hair Triggers",
  "trigger.sub": "Independent L2/R2 deadzones · Instant digital hair triggers · Bumper swap",
  "trigger.flip": "⇄ FLIP BUMPERS & TRIGGERS",
  "trigger.on": "ON",
  "trigger.off": "OFF",
  "trigger.hair": "⚡ HAIR TRIGGER",
  "trigger.active": "ACTIVE",
  "trigger.outputGain": "OUTPUT GAIN",
  "trigger.digitalHint":
    "⚡ Digital Hair Trigger: fires instantly at 100% upon crossing the inner threshold with zero pull lag.",
  "trigger.analogHint": "Smooth analog ramp between inner threshold and 100% saturation point.",
  "trigger.inner": "Inner deadzone (Threshold)",
  "trigger.innerHint": "Initial pull required before input registers",
  "trigger.outer": "Outer deadzone (Saturation)",
  "trigger.outerHintActive": "Inactive in Hair Trigger mode (fires at 100% instantly)",
  "trigger.outerHint": "Point where trigger reaches maximum 100% output",
  "trigger.l2": "L2 → LT",
  "trigger.r2": "R2 → RT",
  "trigger.l2Flipped": "L1 → LT (FLIPPED)",
  "trigger.r2Flipped": "R1 → RT (FLIPPED)",

  // ---- LiveTelemetry -----------------------------------------------------
  "telemetry.title": "Live input map",
  "telemetry.psToX": "PS → XInput",
  "telemetry.gyroAccel": "GYRO / ACCEL",
  "telemetry.dpad": "D-PAD",
  "telemetry.touchpad": "TOUCHPAD",

  // ---- ProfileSelector ---------------------------------------------------
  "profiles.title": "Profiles & Presets",
  "profiles.save": "SAVE CURRENT",
  "profiles.reset": "RESET",
  "profiles.saveTitle": "Save Custom Profile",
  "profiles.placeholder": "Profile name (e.g. Apex Precision, Halo Smooth)...",
  "profiles.saveBtn": "Save",
  "profiles.live": "live",
  "profiles.active": "active",
  "profiles.userSaved": "User Saved Profiles ({n})",
  "profiles.exportTitle": "Copy config JSON to clipboard",
  "profiles.deleteTitle": "Delete profile",

  // ---- SourceExplorer ----------------------------------------------------
  "source.title": "Rust core · {n} files",
  "source.lines": "{n} lines",
  "source.copy": "COPY",
  "source.copied": "COPIED ✓",

  // ---- CircularityTester -------------------------------------------------
  "circ.title": "Stick Circularity Test",
  "circ.stickLeft": "Left",
  "circ.stickRight": "Right",
  "circ.avgError": "AVG ERROR",
  "circ.rotateHint": "Rotate stick along rim...",
  "circ.drift": "DRIFT",
  "circ.restingClean": "✓ Resting Clean",
  "circ.coverage": "COVERAGE",
  "circ.recDz": "REC. DZ",
  "circ.autoCal": "⚡ AUTO-CALIBRATE",
  "circ.correction": "Circularity correction",
  "circ.correctionHint": "remaps the physical ellipse toward a perfect circle (auto-measured)",

  // ---- StickCurveCanvas --------------------------------------------------
  "canvas.fps": "canvas {fps} fps",
  "canvas.hint": "drag ● handles → deadzones · drag inside plot ↕ → curve power",
  "curve.linear": "Linear",
  "curve.exponential": "Exponential",
  "curve.sCurve": "S-Curve",
  "curve.aggressive": "Aggressive",

  // ---- GyroPanel ---------------------------------------------------------
  "gyro.title": "Gyro Motion Control",
  "gyro.sub": "aim with the controller — mouse or right stick",
  "gyro.enabled": "Gyro enabled",
  "gyro.mode": "Gyro mode",
  "gyro.modeMouse": "Mouse",
  "gyro.modeStick": "Right stick",
  "gyro.sensitivity": "Sensitivity",
  "gyro.smoothing": "Smoothing",
  "gyro.invert": "Invert pitch",
  "gyro.recalibrate": "🎯 Recalibrate center",
  "gyro.recalHint": "hold the pad perfectly still, then tap",
  "gyro.restNote": "rest offset auto-captured while the pad is still",

  // ---- ButtonRemapper ----------------------------------------------------
  "remap.title": "Button Remapping",
  "remap.sub": "map any PS button to any XInput output",
  "remap.source": "PHYSICAL",
  "remap.target": "OUTPUT",
  "remap.reset": "RESET MAP",
  "remap.identity": "Default",
  "remap.none": "—",

  // ---- InputOscilloscope -------------------------------------------------
  "osc.title": "Input Oscilloscope",
  "osc.sub": "live strip-chart of sticks & triggers",
  "osc.legend": "left · right · LT · RT",

  // ---- GameProfilesPanel -------------------------------------------------
  "games.title": "Per-Game Profiles",
  "games.sub": "auto-switch when a mapped game is focused",
  "games.current": "Foreground app:",
  "games.none": "no game focused",
  "games.assign": "🎯 Assign current profile",
  "games.assigned": "Profile assigned to {exe}",
  "games.noMapping": "no mappings yet — launch your game, then assign",
  "games.remove": "Remove",
  "games.applied": "🎮 {exe} profile applied (per-game)",
  "games.restored": "↩️ Restored controller profile after {app}",

  // ---- SettingsPanel -----------------------------------------------------
  "settings.title": "Settings",
  "settings.behavior": "Behavior",
  "settings.startMinimized": "Start minimized to tray",
  "settings.minimizeToTray": "Minimize to tray on close",
  "settings.autostart": "Launch PadFlow at Windows startup",
  "settings.language": "Language / Idioma",
  "settings.diagnostic": "🩺 Diagnostic report",
  "settings.copyReport": "Copy report",
  "settings.reportCopied": "Diagnostic report copied to clipboard",
  "settings.close": "Close",
  "settings.about": "About",
  "settings.saved": "Settings saved",
};

const ES: Dict = {
  "app.tagline": "Puente DualShock 4 / DualSense → XInput · ViGEmBus · Escudo HidHide · <15 MB de RAM",
  "chip.poll": "POLL",
  "chip.latency": "LATENCIA",
  "chip.virtualPad": "MANDO VIRTUAL",
  "chip.hidhide": "HIDHIDE",
  "chip.mode": "MODO",
  "chip.x360Online": "X360 EN LÍNEA",
  "chip.offline": "SIN CONEXIÓN",
  "chip.driverReady": "DRIVER LISTO 🛡️",
  "chip.notInstalled": "NO INSTALADO",
  "chip.nativeHid": "HID NATIVO",
  "chip.webPreview": "VISTA WEB",
  "checkUpdate": "Comprobar actualización",
  "checking": "Comprobando…",
  "updateReady": "Actualización v{version} lista",
  "tab.studio": "Estudio",
  "tab.source": "Núcleo Rust",
  "stopEngine": "DETENER MOTOR",
  "startEngine": "INICIAR MOTOR",
  "lang.toggle": "EN",

  "banner.vigemTitle": "Driver ViGEmBus necesario para el mando virtual Xbox 360",
  "banner.vigemText":
    "Las curvas y la monitorización en vivo funcionan, pero los juegos necesitan ViGEmBus para recibir las entradas mapeadas.",
  "banner.installVigem": "🛠️ INSTALAR DRIVER VIGEMBUS",
  "banner.adminTitle": "El modo Administrador ayuda cuando HidHide rechaza cambios",
  "banner.adminText":
    "El cloaking suele funcionar sin él, pero si HidHide rechaza una escritura, reinicia como Administrador para forzar el fallback de registro.",
  "banner.restartAsAdmin": "🔄 REINICIAR COMO ADMINISTRADOR",
  "banner.hidhideTitle": "Driver HidHide recomendado — evita conflictos de doble entrada",
  "banner.hidhideText":
    "Oculta los mandos DirectInput físicos de los juegos para que solo se detecte el mando Xbox virtual.",
  "banner.installHidhide": "🛡️ INSTALAR DRIVER HIDHIDE",

  "section.controllers": "Mandos",
  "section.detected": "{n} detectados",
  "noController": "No se detectó ningún mando",
  "noControllerHint": "Conecta un DualShock 4 / DualSense por USB o Bluetooth",
  "section.shield": "Escudo Anti Doble Entrada",
  "shield.active": "ACTIVO 🛡️",
  "shield.paused": "PAUSADO",
  "shield.notInstalled": "NO INSTALADO",
  "shield.desc":
    "Oculta los mandos PlayStation físicos de los juegos para que solo se detecte el mando virtual Xbox 360 — adiós doble entrada.",
  "shield.driverNotDetected": "Driver no detectado",
  "shield.global": "Escudo global",
  "shield.globalSubActive": "cortafuegos interceptando reportes HID",
  "shield.globalSubPaused": "cortafuegos en pausa — mandos visibles",
  "shield.cloakAll": "🛡️ OCULTAR TODOS",
  "shield.uncloakAll": "◉ MOSTRAR TODOS",
  "shield.whitelist": "Lista blanca:",
  "shield.authorized": "PadFlow Autorizado",
  "shield.notWhitelisted": "NO EN LISTA BLANCA",
  "shield.hiddenDevices": "Dispositivos ocultos ({n})",
  "shield.nothingHidden": "Nada oculto — los juegos ven tus mandos físicos directamente.",
  "shield.autoCloak": "Auto-cloak al conectar",
  "shield.autoCloakHint": "oculta mandos PlayStation nuevos al conectarse",
  "shield.cloakStart": "Cloak al iniciar",
  "shield.cloakStartHint": "oculta al iniciar los mandos ya conectados (requiere el escudo global)",
  "shield.openClient": "⚙️ ABRIR CLIENTE HIDHIDE (GUI OFICIAL)",
  "section.engineExtras": "Motor y extras",
  "stat.reports": "Reportes",
  "stat.peakLat": "Pico lat.",
  "stat.dropped": "Perdidos",
  "stat.reconnects": "Reconexiones",
  "rumble.intensity": "Intensidad de vibración",
  "turbo.label": "Turbo polling (1 kHz)",
  "turbo.hint": "fija el bucle HID a prioridad de sub-milisegundo",
  "touchpad.label": "Touchpad como ratón virtual",
  "touchpad.hint": "1 dedo mover/clic · 2 dedos scroll",
  "touchpad.sens": "Sensibilidad del touchpad",
  "batteryLed.label": "Lightbar inteligente de batería",
  "batteryLed.hint": "color LED dinámico (Verde > 60%, Ámbar, Rojo)",

  "matrix.title": "Matriz de respuesta de sticks",
  "matrix.right": "magnitud de entrada → salida virtual",
  "curve.left": "Stick izquierdo · movimiento",
  "curve.right": "Stick derecho · puntería",
  "tuner.left": "Tuner stick izquierdo",
  "tuner.right": "Tuner stick derecho",

  "footer.about": "PadFlow v{version} · código abierto · github.com/jaimitus/PadFlow · Windows 10 / 11 · ViGEmBus 1.22+ · Soporte HidHide",
  "footer.activePad": "mando activo:",

  "update.available": "Actualización disponible",
  "update.fromTo": "PadFlow v{from} → v{to}",
  "update.released": "Se ha publicado una nueva versión de PadFlow en GitHub.",
  "update.applying": "Aplicando actualización…",
  "update.downloading": "Descargando actualización…",
  "update.restart": "🔄 Reiniciar PadFlow ahora",
  "update.downloadInstall": "⬇ Descargar e instalar",
  "update.viewRelease": "Ver release",
  "update.later": "Más tarde",
  "update.notNow": "Ahora no",
  "update.keepOpen": "Mantén PadFlow abierto mientras se instala…",

  "conn.usb": "USB · CABLEADO",
  "conn.bt": "BLUETOOTH",
  "badge.cloaked": "🛡️ OCULTO",
  "badge.visible": "VISIBLE",
  "slot.active": "slot activo",
  "slot.mapped": "mapeado",
  "hiddenFromGames": "OCULTO DE LOS JUEGOS",
  "visibleToGames": "VISIBLE PARA LOS JUEGOS",
  "cloak.btn": "🙈 OCULTAR",
  "uncloak.btn": "🛡️ MOSTRAR",
  "battery.label": "BATERÍA",
  "battery.na": "n/d",
  "battery.charging": "⚡ cargando",
  "lightbar.rgb": "LIGHTBAR RGB",
  "rumble.test": "PROBAR VIBRACIÓN",

  "dz.inner": "Deadzone interior",
  "dz.innerHint": "elimina el drift del stick en reposo",
  "dz.outer": "Deadzone exterior",
  "dz.outerHint": "magnitud que alcanza el 100% de salida",
  "dz.anti": "Anti-deadzone",
  "dz.antiHint": "compensa la deadzone del juego",
  "dz.power": "Potencia de curva",
  "dz.powerHint": "inclinación de la rampa de respuesta",
  "dz.sens": "Sensibilidad",
  "dz.sensHint": "multiplicador de ganancia de salida",

  "trigger.title": "Matriz de gatillos y hair triggers",
  "trigger.sub": "Deadzones L2/R2 independientes · Hair triggers digitales instantáneos · Intercambio de bumpers",
  "trigger.flip": "⇄ INTERCAMBIAR BUMPERS Y GATILLOS",
  "trigger.on": "ON",
  "trigger.off": "OFF",
  "trigger.hair": "⚡ HAIR TRIGGER",
  "trigger.active": "ACTIVO",
  "trigger.outputGain": "GANANCIA DE SALIDA",
  "trigger.digitalHint":
    "⚡ Hair trigger digital: dispara al 100% al cruzar el umbral interior sin retardo de recorrido.",
  "trigger.analogHint": "Rampa analógica suave entre el umbral interior y el punto de saturación al 100%.",
  "trigger.inner": "Deadzone interior (Umbral)",
  "trigger.innerHint": "Recorrido inicial necesario antes de registrar entrada",
  "trigger.outer": "Deadzone exterior (Saturación)",
  "trigger.outerHintActive": "Inactiva en modo hair trigger (dispara al 100% al instante)",
  "trigger.outerHint": "Punto donde el gatillo alcanza el 100% de salida",
  "trigger.l2": "L2 → LT",
  "trigger.r2": "R2 → RT",
  "trigger.l2Flipped": "L1 → LT (INVERTIDO)",
  "trigger.r2Flipped": "R1 → RT (INVERTIDO)",

  "telemetry.title": "Mapa de entrada en vivo",
  "telemetry.psToX": "PS → XInput",
  "telemetry.gyroAccel": "GYRO / ACCEL",
  "telemetry.dpad": "CRUZETA",
  "telemetry.touchpad": "TOUCHPAD",

  "profiles.title": "Perfiles y presets",
  "profiles.save": "GUARDAR ACTUAL",
  "profiles.reset": "REINICIAR",
  "profiles.saveTitle": "Guardar perfil personalizado",
  "profiles.placeholder": "Nombre del perfil (p. ej. Precisión Apex, Suave Halo)…",
  "profiles.saveBtn": "Guardar",
  "profiles.live": "en vivo",
  "profiles.active": "activo",
  "profiles.userSaved": "Perfiles guardados ({n})",
  "profiles.exportTitle": "Copiar JSON de configuración al portapapeles",
  "profiles.deleteTitle": "Eliminar perfil",

  "source.title": "Núcleo Rust · {n} archivos",
  "source.lines": "{n} líneas",
  "source.copy": "COPIAR",
  "source.copied": "COPIADO ✓",

  "circ.title": "Test de circularidad del stick",
  "circ.stickLeft": "Izquierdo",
  "circ.stickRight": "Derecho",
  "circ.avgError": "ERROR MEDIO",
  "circ.rotateHint": "Gira el stick por el borde...",
  "circ.drift": "DRIFT",
  "circ.restingClean": "✓ Reposo limpio",
  "circ.coverage": "COBERTURA",
  "circ.recDz": "DZ REC.",
  "circ.autoCal": "⚡ AUTO-CALIBRAR",
  "circ.correction": "Corrección de circularidad",
  "circ.correctionHint": "remodela la elipse física hacia un círculo perfecto (medida automática)",

  "canvas.fps": "canvas {fps} fps",
  "canvas.hint": "arrastra ● tiradores → deadzones · arrastra dentro ↕ → potencia de curva",
  "curve.linear": "Lineal",
  "curve.exponential": "Exponencial",
  "curve.sCurve": "Curva S",
  "curve.aggressive": "Agresiva",

  "gyro.title": "Control de movimiento gyro",
  "gyro.sub": "apunta con el mando — ratón o stick derecho",
  "gyro.enabled": "Gyro activado",
  "gyro.mode": "Modo gyro",
  "gyro.modeMouse": "Ratón",
  "gyro.modeStick": "Stick derecho",
  "gyro.sensitivity": "Sensibilidad",
  "gyro.smoothing": "Suavizado",
  "gyro.invert": "Invertir cabeceo",
  "gyro.recalibrate": "🎯 Recalibrar centro",
  "gyro.recalHint": "mantén el mando quieto y pulsa",
  "gyro.restNote": "offset de reposo capturado automáticamente con el mando quieto",

  "remap.title": "Remapeo de botones",
  "remap.sub": "mapea cualquier botón PS a cualquier salida XInput",
  "remap.source": "FÍSICO",
  "remap.target": "SALIDA",
  "remap.reset": "REINICIAR MAPA",
  "remap.identity": "Predeterminado",
  "remap.none": "—",

  "osc.title": "Osciloscopio de entrada",
  "osc.sub": "gráfico en vivo de sticks y gatillos",
  "osc.legend": "izquierdo · derecho · LT · RT",

  "games.title": "Perfiles por juego",
  "games.sub": "cambio automático cuando un juego mapeado tiene el foco",
  "games.current": "App en foco:",
  "games.none": "ningún juego en foco",
  "games.assign": "🎯 Asignar perfil actual",
  "games.assigned": "Perfil asignado a {exe}",
  "games.noMapping": "sin mapeos aún — abre tu juego y asigna",
  "games.remove": "Quitar",
  "games.applied": "🎮 Perfil de {exe} aplicado (por juego)",
  "games.restored": "↩️ Perfil del mando restaurado tras {app}",

  "settings.title": "Ajustes",
  "settings.behavior": "Comportamiento",
  "settings.startMinimized": "Iniciar minimizado en la bandeja",
  "settings.minimizeToTray": "Minimizar a la bandeja al cerrar",
  "settings.autostart": "Iniciar PadFlow con Windows",
  "settings.language": "Idioma / Language",
  "settings.diagnostic": "🩺 Informe de diagnóstico",
  "settings.copyReport": "Copiar informe",
  "settings.reportCopied": "Informe copiado al portapapeles",
  "settings.close": "Cerrar",
  "settings.about": "Acerca de",
  "settings.saved": "Ajustes guardados",
};

const DICTS: Record<Lang, Dict> = { en: EN, es: ES };

interface I18nCtx {
  lang: Lang;
  setLang: (l: Lang) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}

const Ctx = createContext<I18nCtx | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved === "en" || saved === "es") return saved;
      const nav = navigator.language || "";
      if (nav.toLowerCase().startsWith("es")) return "es";
    }
    return "en";
  });

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  const setLang = useCallback((l: Lang) => {
    setLangState(l);
    try {
      localStorage.setItem(STORAGE_KEY, l);
    } catch {
      /* storage unavailable */
    }
  }, []);

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>) => {
      let s = DICTS[lang][key] ?? DICTS.en[key] ?? key;
      if (vars) {
        for (const [k, v] of Object.entries(vars)) {
          s = s.split(`{${k}}`).join(String(v));
        }
      }
      return s;
    },
    [lang],
  );

  const value = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useI18n(): I18nCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useI18n must be used inside <I18nProvider>");
  return ctx;
}
