import { useI18n } from "../lib/i18n";
import type { Lang } from "../lib/i18n";
import { APP_VERSION } from "../lib/version";
import { cn } from "../utils/cn";

export interface AppSettings {
  startMinimized: boolean;
  minimizeToTray: boolean;
  autostart: boolean;
}

interface Props {
  settings: AppSettings;
  onToggle: (key: keyof AppSettings) => void;
  lang: Lang;
  onLangChange: (l: Lang) => void;
  onCopyDiagnostic: () => Promise<void>;
  onClose: () => void;
}

export default function SettingsPanel({
  settings,
  onToggle,
  lang,
  onLangChange,
  onCopyDiagnostic,
  onClose,
}: Props) {
  const { t } = useI18n();

  const rows: { key: keyof AppSettings; label: string; hint: string }[] = [
    { key: "startMinimized", label: t("settings.startMinimized"), hint: "PadFlow" },
    { key: "minimizeToTray", label: t("settings.minimizeToTray"), hint: "PadFlow" },
    { key: "autostart", label: t("settings.autostart"), hint: "Windows" },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md overflow-hidden rounded-2xl border border-cyan-400/25 bg-[#0a0e18] shadow-[0_0_70px_-12px] shadow-cyan-400/25">
        <div className="pointer-events-none absolute -top-24 left-1/2 h-48 w-72 -translate-x-1/2 rounded-full bg-cyan-400/15 blur-[70px]" />
        <div className="relative p-5">
          <div className="mb-4 flex items-start justify-between">
            <div>
              <h3 className="text-sm font-bold uppercase tracking-[0.18em] text-white">
                ⚙️ {t("settings.title")}
              </h3>
              <p className="mt-0.5 font-mono text-[10px] text-slate-500">
                {t("settings.about")} · PadFlow v{APP_VERSION}
              </p>
            </div>
            <button
              onClick={onClose}
              className="rounded-md px-2 py-1 text-slate-400 transition-colors hover:text-white cursor-pointer"
            >
              ✕
            </button>
          </div>

          {/* language */}
          <div className="mb-4">
            <p className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
              {t("settings.language")}
            </p>
            <div className="flex rounded-lg border border-white/8 bg-white/5 p-0.5">
              {(["en", "es"] as Lang[]).map((l) => (
                <button
                  key={l}
                  onClick={() => onLangChange(l)}
                  className={cn(
                    "flex-1 rounded-md px-3 py-1.5 font-mono text-[11px] font-semibold uppercase tracking-wider transition-colors",
                    lang === l ? "bg-cyan-400 text-slate-950" : "text-slate-500 hover:text-slate-300",
                  )}
                >
                  {l === "en" ? "English" : "Español"}
                </button>
              ))}
            </div>
          </div>

          {/* behavior */}
          <p className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
            {t("settings.behavior")}
          </p>
          <div className="space-y-2.5">
            {rows.map((row) => (
              <div key={row.key} className="flex items-center justify-between">
                <div>
                  <p className="text-[11px] text-slate-200">{row.label}</p>
                  <p className="font-mono text-[9.5px] text-slate-600">{row.hint}</p>
                </div>
                <button
                  onClick={() => onToggle(row.key)}
                  aria-label={row.label}
                  className={cn(
                    "relative h-5 w-9 rounded-full transition-colors",
                    settings[row.key] ? "bg-cyan-400" : "bg-white/12",
                  )}
                >
                  <span
                    className={cn(
                      "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                      settings[row.key] ? "left-[18px]" : "left-0.5",
                    )}
                  />
                </button>
              </div>
            ))}
          </div>

          {/* diagnostic */}
          <div className="mt-4 border-t border-white/8 pt-3.5">
            <p className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
              {t("settings.diagnostic")}
            </p>
            <button
              onClick={onCopyDiagnostic}
              className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 font-mono text-[10px] font-bold text-slate-300 transition-all hover:border-emerald-400/40 hover:text-emerald-200 cursor-pointer"
            >
              📋 {t("settings.copyReport")}
            </button>
          </div>

          <div className="mt-5 flex justify-end">
            <button
              onClick={onClose}
              className="rounded-xl bg-gradient-to-r from-cyan-400 to-violet-500 px-4 py-2 text-xs font-bold text-slate-950 transition-all hover:brightness-110 cursor-pointer"
            >
              {t("settings.close")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
