import { useI18n } from "../lib/i18n";
import type { GyroMode, StickProfileConfig } from "../lib/types";
import { cn } from "../utils/cn";

interface Props {
  config: StickProfileConfig;
  onChange: (patch: Partial<StickProfileConfig>) => void;
  hasGyro: boolean;
  onRecalibrate: () => void;
}

export default function GyroPanel({ config, onChange, hasGyro, onRecalibrate }: Props) {
  const { t } = useI18n();
  const enabled = config.gyroEnabled && hasGyro;

  return (
    <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-2 flex items-center justify-between">
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
            🌀 {t("gyro.title")}
          </h3>
          <p className="mt-0.5 font-mono text-[9.5px] text-slate-500">{t("gyro.sub")}</p>
        </div>
        <button
          onClick={() => onChange({ gyroEnabled: !config.gyroEnabled })}
          disabled={!hasGyro}
          aria-label="Toggle gyro"
          className={cn(
            "relative h-5 w-9 shrink-0 rounded-full transition-colors",
            enabled ? "bg-cyan-400" : "bg-white/12",
            !hasGyro && "opacity-40 cursor-not-allowed",
          )}
        >
          <span
            className={cn(
              "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
              enabled ? "left-[18px]" : "left-0.5",
            )}
          />
        </button>
      </div>

      {!hasGyro ? (
        <p className="font-mono text-[10px] text-slate-500">
          {t("gyro.restNote")} — <span className="text-amber-300/80">n/a</span>
        </p>
      ) : (
        <div className={cn("space-y-3", !enabled && "pointer-events-none opacity-40")}>
          {/* mode */}
          <div className="flex items-center justify-between">
            <span className="text-[11px] text-slate-300">{t("gyro.mode")}</span>
            <div className="flex rounded-lg border border-white/8 bg-white/5 p-0.5">
              {(["mouse", "rightStick"] as GyroMode[]).map((m) => (
                <button
                  key={m}
                  onClick={() => onChange({ gyroMode: m })}
                  className={cn(
                    "rounded-md px-2.5 py-1 font-mono text-[10px] uppercase tracking-wider transition-colors",
                    config.gyroMode === m ? "bg-white/10 text-white" : "text-slate-500 hover:text-slate-300",
                  )}
                >
                  {m === "mouse" ? t("gyro.modeMouse") : t("gyro.modeStick")}
                </button>
              ))}
            </div>
          </div>

          {/* sensitivity */}
          <div>
            <div className="mb-1 flex items-baseline justify-between">
              <span className="text-[11px] text-slate-300">{t("gyro.sensitivity")}</span>
              <span className="font-mono text-[11px] tabular-nums text-slate-100">
                {config.gyroSensitivity.toFixed(2)}×
              </span>
            </div>
            <input
              type="range"
              min={0.1}
              max={8}
              step={0.05}
              value={config.gyroSensitivity}
              onChange={(e) => onChange({ gyroSensitivity: parseFloat(e.target.value) })}
              className="pf-range h-1.5 w-full cursor-pointer appearance-none rounded-full"
              style={{
                background: `linear-gradient(90deg, rgb(34,211,238) 0%, rgb(34,211,238) ${((config.gyroSensitivity - 0.1) / 7.9) * 100}%, rgba(255,255,255,0.09) ${((config.gyroSensitivity - 0.1) / 7.9) * 100}%, rgba(255,255,255,0.09) 100%)`,
                ["--pf-accent" as string]: "rgb(34,211,238)",
              }}
            />
          </div>

          {/* smoothing */}
          <div>
            <div className="mb-1 flex items-baseline justify-between">
              <span className="text-[11px] text-slate-300">{t("gyro.smoothing")}</span>
              <span className="font-mono text-[11px] tabular-nums text-slate-100">
                {(config.gyroSmoothing * 100).toFixed(0)}%
              </span>
            </div>
            <input
              type="range"
              min={0}
              max={0.95}
              step={0.05}
              value={config.gyroSmoothing}
              onChange={(e) => onChange({ gyroSmoothing: parseFloat(e.target.value) })}
              className="pf-range h-1.5 w-full cursor-pointer appearance-none rounded-full"
              style={{
                background: `linear-gradient(90deg, rgb(168,85,247) 0%, rgb(168,85,247) ${(config.gyroSmoothing / 0.95) * 100}%, rgba(255,255,255,0.09) ${(config.gyroSmoothing / 0.95) * 100}%, rgba(255,255,255,0.09) 100%)`,
                ["--pf-accent" as string]: "rgb(168,85,247)",
              }}
            />
          </div>

          {/* invert + recalibrate */}
          <div className="flex items-center justify-between border-t border-white/6 pt-2.5">
            <div>
              <p className="text-[11px] text-slate-300">{t("gyro.invert")}</p>
              <p className="font-mono text-[9.5px] text-slate-600">{t("gyro.recalHint")}</p>
            </div>
            <button
              onClick={() => onChange({ gyroInvert: !config.gyroInvert })}
              className={cn(
                "relative h-5 w-9 rounded-full transition-colors",
                config.gyroInvert ? "bg-violet-400" : "bg-white/12",
              )}
            >
              <span
                className={cn(
                  "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                  config.gyroInvert ? "left-[18px]" : "left-0.5",
                )}
              />
            </button>
          </div>

          <button
            onClick={onRecalibrate}
            className="w-full rounded-lg border border-cyan-400/30 bg-cyan-400/10 px-3 py-2 font-mono text-[10px] font-bold text-cyan-200 transition-all hover:bg-cyan-400/20 cursor-pointer"
          >
            {t("gyro.recalibrate")}
          </button>

          <p className="font-mono text-[9px] leading-relaxed text-slate-600">{t("gyro.restNote")}</p>
        </div>
      )}
    </section>
  );
}
