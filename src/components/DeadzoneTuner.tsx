import { CURVE_HINTS, CURVE_LABELS } from "../lib/curves";
import type { CurveKind, StickAxisProfile } from "../lib/types";
import { cn } from "../utils/cn";

interface Props {
  title: string;
  accent: string; // "r,g,b"
  profile: StickAxisProfile;
  onChange: (patch: Partial<StickAxisProfile>) => void;
}

const CURVES: CurveKind[] = ["linear", "exponential", "sCurve", "aggressive"];

export default function DeadzoneTuner({ title, accent, profile, onChange }: Props) {
  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-3 flex items-center justify-between">
        <h4 className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
          <span
            className="h-2 w-2 rounded-full"
            style={{ background: `rgb(${accent})`, boxShadow: `0 0 10px rgb(${accent})` }}
          />
          {title}
        </h4>
        <div className="flex gap-1">
          <Toggle
            active={profile.radial}
            onClick={() => onChange({ radial: !profile.radial })}
            label="RADIAL"
          />
          <Toggle
            active={profile.invertY}
            onClick={() => onChange({ invertY: !profile.invertY })}
            label="INV-Y"
          />
        </div>
      </div>

      <div className="mb-4 grid grid-cols-4 gap-1">
        {CURVES.map((c) => (
          <button
            key={c}
            onClick={() => onChange({ curve: c })}
            title={CURVE_HINTS[c]}
            className={cn(
              "rounded-lg border px-1 py-1.5 text-[10px] font-semibold uppercase tracking-wide transition-all",
              profile.curve === c
                ? "border-transparent text-slate-950"
                : "border-white/8 bg-white/5 text-slate-400 hover:text-slate-200",
            )}
            style={
              profile.curve === c
                ? { background: `rgb(${accent})`, boxShadow: `0 0 18px -4px rgb(${accent})` }
                : undefined
            }
          >
            {CURVE_LABELS[c]}
          </button>
        ))}
      </div>

      <p className="mb-4 font-mono text-[10px] leading-relaxed text-slate-500">
        {CURVE_HINTS[profile.curve]}
      </p>

      <Slider
        label="Inner deadzone"
        hint="kills stick drift at rest"
        value={profile.innerDeadzone}
        min={0}
        max={0.4}
        step={0.005}
        accent="244,63,94"
        format={(v) => `${(v * 100).toFixed(1)}%`}
        onChange={(v) =>
          onChange({ innerDeadzone: Math.min(v, profile.outerDeadzone - 0.05) })
        }
      />
      <Slider
        label="Outer deadzone"
        hint="magnitude that reaches 100% output"
        value={profile.outerDeadzone}
        min={0.5}
        max={1}
        step={0.005}
        accent="56,189,248"
        format={(v) => `${(v * 100).toFixed(1)}%`}
        onChange={(v) =>
          onChange({ outerDeadzone: Math.max(v, profile.innerDeadzone + 0.05) })
        }
      />
      <Slider
        label="Anti-deadzone"
        hint="compensates the in-game deadzone"
        value={profile.antiDeadzone}
        min={0}
        max={0.6}
        step={0.005}
        accent="250,204,21"
        format={(v) => `${(v * 100).toFixed(1)}%`}
        onChange={(v) => onChange({ antiDeadzone: v })}
      />
      <Slider
        label="Curve power"
        hint="steepness of the response ramp"
        value={profile.curvePower}
        min={0.5}
        max={4}
        step={0.01}
        accent={accent}
        format={(v) => v.toFixed(2)}
        disabled={profile.curve === "linear"}
        onChange={(v) => onChange({ curvePower: v })}
      />
      <Slider
        label="Sensitivity"
        hint="output gain multiplier"
        value={profile.sensitivity}
        min={0.25}
        max={3}
        step={0.01}
        accent="168,85,247"
        format={(v) => `${v.toFixed(2)}×`}
        onChange={(v) => onChange({ sensitivity: v })}
      />
    </div>
  );
}

function Toggle({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "rounded-md border px-2 py-0.5 font-mono text-[10px] tracking-wider transition-colors",
        active
          ? "border-cyan-400/40 bg-cyan-400/15 text-cyan-200"
          : "border-white/8 bg-white/5 text-slate-500 hover:text-slate-300",
      )}
    >
      {label}
    </button>
  );
}

interface SliderProps {
  label: string;
  hint: string;
  value: number;
  min: number;
  max: number;
  step: number;
  accent: string;
  format: (v: number) => string;
  onChange: (v: number) => void;
  disabled?: boolean;
}

function Slider({
  label,
  hint,
  value,
  min,
  max,
  step,
  accent,
  format,
  onChange,
  disabled,
}: SliderProps) {
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <div className={cn("mb-3.5", disabled && "opacity-40")}>
      <div className="mb-1 flex items-baseline justify-between">
        <span className="text-[11px] font-medium text-slate-300">{label}</span>
        <span className="font-mono text-[11px] tabular-nums text-slate-100">
          {format(value)}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="pf-range h-1.5 w-full cursor-pointer appearance-none rounded-full"
        style={{
          background: `linear-gradient(90deg, rgb(${accent}) 0%, rgb(${accent}) ${pct}%, rgba(255,255,255,0.09) ${pct}%, rgba(255,255,255,0.09) 100%)`,
          ["--pf-accent" as string]: `rgb(${accent})`,
        }}
      />
      <p className="mt-1 font-mono text-[9.5px] text-slate-600">{hint}</p>
    </div>
  );
}
