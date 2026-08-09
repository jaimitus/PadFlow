import { useEffect, useRef } from "react";
import type { InputSnapshot, TriggerProfile } from "../lib/types";
import { cn } from "../utils/cn";

interface Props {
  triggerLeft: TriggerProfile;
  triggerRight: TriggerProfile;
  flipTriggers: boolean;
  onLeftChange: (patch: Partial<TriggerProfile>) => void;
  onRightChange: (patch: Partial<TriggerProfile>) => void;
  onFlipChange: (flip: boolean) => void;
  getSnapshot: () => InputSnapshot;
}

export default function TriggerTuner({
  triggerLeft,
  triggerRight,
  flipTriggers,
  onLeftChange,
  onRightChange,
  onFlipChange,
  getSnapshot,
}: Props) {
  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-3.5 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
            Trigger Matrix & Hair Triggers
          </h3>
          <p className="mt-0.5 font-mono text-[9.5px] text-slate-500">
            Independent L2/R2 deadzones · Instant digital hair triggers · Bumper swap
          </p>
        </div>

        <button
          onClick={() => onFlipChange(!flipTriggers)}
          className={cn(
            "flex items-center gap-1.5 rounded-lg border px-2.5 py-1 font-mono text-[10px] tracking-wider transition-all",
            flipTriggers
              ? "border-cyan-400/50 bg-cyan-400/15 text-cyan-200 shadow-sm shadow-cyan-400/20"
              : "border-white/8 bg-white/5 text-slate-400 hover:text-slate-200",
          )}
          title="Swaps L1/R1 bumpers with L2/R2 triggers for instant bumper aiming/shooting"
        >
          <span>⇄ FLIP BUMPERS & TRIGGERS</span>
          <span
            className={cn(
              "rounded px-1 text-[8.5px] font-bold uppercase",
              flipTriggers ? "bg-cyan-400 text-slate-950" : "bg-white/10 text-slate-400",
            )}
          >
            {flipTriggers ? "ON" : "OFF"}
          </span>
        </button>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <TriggerCard
          side="L2 → LT"
          accent="34,211,238"
          profile={triggerLeft}
          onChange={onLeftChange}
          getRawValue={() => getSnapshot().triggerLeft}
        />
        <TriggerCard
          side="R2 → RT"
          accent="168,85,247"
          profile={triggerRight}
          onChange={onRightChange}
          getRawValue={() => getSnapshot().triggerRight}
        />
      </div>
    </div>
  );
}

interface TriggerCardProps {
  side: string;
  accent: string;
  profile: TriggerProfile;
  onChange: (patch: Partial<TriggerProfile>) => void;
  getRawValue: () => number;
}

function TriggerCard({ side, accent, profile, onChange, getRawValue }: TriggerCardProps) {
  const meterRef = useRef<HTMLDivElement | null>(null);
  const rawTextRef = useRef<HTMLSpanElement | null>(null);

  useEffect(() => {
    let raf = 0;
    const tick = () => {
      const v = getRawValue();
      if (meterRef.current) {
        meterRef.current.style.width = `${Math.min(v * 100, 100)}%`;
      }
      if (rawTextRef.current) {
        rawTextRef.current.textContent = `${(v * 100).toFixed(0)}% (${Math.round(v * 255)})`;
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [getRawValue]);

  return (
    <div className="rounded-xl border border-white/6 bg-white/[0.02] p-3.5">
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span
            className="h-2 w-2 rounded-full"
            style={{ background: `rgb(${accent})`, boxShadow: `0 0 10px rgb(${accent})` }}
          />
          <h4 className="text-xs font-semibold uppercase tracking-wider text-slate-200">{side}</h4>
        </div>

        <button
          onClick={() => onChange({ hairTrigger: !profile.hairTrigger })}
          className={cn(
            "flex items-center gap-1 rounded-md border px-2 py-0.5 font-mono text-[9.5px] font-bold uppercase transition-all",
            profile.hairTrigger
              ? "border-amber-400/50 bg-amber-400/20 text-amber-200 shadow-sm shadow-amber-400/20"
              : "border-white/8 bg-white/5 text-slate-400 hover:text-slate-200",
          )}
        >
          <span>⚡ HAIR TRIGGER</span>
          <span
            className={cn(
              "rounded px-1 text-[8px]",
              profile.hairTrigger ? "bg-amber-400 text-slate-950 font-bold" : "bg-white/10 text-slate-400",
            )}
          >
            {profile.hairTrigger ? "ACTIVE" : "OFF"}
          </span>
        </button>
      </div>

      {/* Live Trigger Meter */}
      <div className="mb-3 rounded-lg border border-white/6 bg-slate-950/60 p-2">
        <div className="mb-1 flex items-baseline justify-between font-mono text-[9.5px]">
          <span className="text-slate-400">OUTPUT GAIN</span>
          <span ref={rawTextRef} className="tabular-nums text-slate-200">
            0% (0)
          </span>
        </div>
        <div className="relative h-2 w-full overflow-hidden rounded-full bg-white/8">
          {/* Inner deadzone marker */}
          <div
            className="absolute top-0 bottom-0 z-10 w-0.5 bg-rose-400/80"
            style={{ left: `${profile.innerDeadzone * 100}%` }}
            title="Inner threshold"
          />
          {/* Outer deadzone marker */}
          {!profile.hairTrigger && (
            <div
              className="absolute top-0 bottom-0 z-10 w-0.5 bg-sky-400/80"
              style={{ left: `${profile.outerDeadzone * 100}%` }}
              title="Outer threshold"
            />
          )}
          {/* Active fill */}
          <div
            ref={meterRef}
            className="h-full rounded-full transition-all duration-75"
            style={{
              width: "0%",
              background: profile.hairTrigger
                ? "linear-gradient(90deg, rgb(251,191,36), rgb(245,158,11))"
                : `linear-gradient(90deg, rgba(${accent},0.5), rgb(${accent}))`,
              boxShadow: `0 0 12px ${profile.hairTrigger ? "rgba(251,191,36,0.6)" : `rgba(${accent},0.6)`}`,
            }}
          />
        </div>
      </div>

      <p className="mb-3 font-mono text-[9.5px] leading-relaxed text-slate-500">
        {profile.hairTrigger
          ? "⚡ Digital Hair Trigger: fires instantly at 100% upon crossing the inner threshold with zero pull lag."
          : "Smooth analog ramp between inner threshold and 100% saturation point."}
      </p>

      {/* Sliders */}
      <div className="space-y-3">
        <div>
          <div className="mb-1 flex items-baseline justify-between">
            <span className="text-[11px] font-medium text-slate-300">Inner deadzone (Threshold)</span>
            <span className="font-mono text-[11px] tabular-nums text-slate-100">
              {(profile.innerDeadzone * 100).toFixed(1)}%
            </span>
          </div>
          <input
            type="range"
            min={0}
            max={0.4}
            step={0.005}
            value={profile.innerDeadzone}
            onChange={(e) =>
              onChange({
                innerDeadzone: Math.min(parseFloat(e.target.value), profile.outerDeadzone - 0.05),
              })
            }
            className="pf-range h-1.5 w-full cursor-pointer appearance-none rounded-full"
            style={{
              background: `linear-gradient(90deg, rgb(244,63,94) 0%, rgb(244,63,94) ${
                (profile.innerDeadzone / 0.4) * 100
              }%, rgba(255,255,255,0.09) ${(profile.innerDeadzone / 0.4) * 100}%, rgba(255,255,255,0.09) 100%)`,
              ["--pf-accent" as string]: "rgb(244,63,94)",
            }}
          />
          <p className="mt-0.5 font-mono text-[9px] text-slate-600">
            Initial pull required before input registers
          </p>
        </div>

        <div className={cn(profile.hairTrigger && "opacity-35")}>
          <div className="mb-1 flex items-baseline justify-between">
            <span className="text-[11px] font-medium text-slate-300">Outer deadzone (Saturation)</span>
            <span className="font-mono text-[11px] tabular-nums text-slate-100">
              {(profile.outerDeadzone * 100).toFixed(1)}%
            </span>
          </div>
          <input
            type="range"
            min={0.5}
            max={1.0}
            step={0.005}
            disabled={profile.hairTrigger}
            value={profile.outerDeadzone}
            onChange={(e) =>
              onChange({
                outerDeadzone: Math.max(parseFloat(e.target.value), profile.innerDeadzone + 0.05),
              })
            }
            className="pf-range h-1.5 w-full cursor-pointer appearance-none rounded-full"
            style={{
              background: `linear-gradient(90deg, rgb(56,189,248) 0%, rgb(56,189,248) ${
                ((profile.outerDeadzone - 0.5) / 0.5) * 100
              }%, rgba(255,255,255,0.09) ${((profile.outerDeadzone - 0.5) / 0.5) * 100}%, rgba(255,255,255,0.09) 100%)`,
              ["--pf-accent" as string]: "rgb(56,189,248)",
            }}
          />
          <p className="mt-0.5 font-mono text-[9px] text-slate-600">
            {profile.hairTrigger
              ? "Inactive in Hair Trigger mode (fires at 100% instantly)"
              : "Point where trigger reaches maximum 100% output"}
          </p>
        </div>
      </div>
    </div>
  );
}
