import { useMemo } from "react";
import type { GamepadInfo } from "../lib/types";
import { cn } from "../utils/cn";

interface Props {
  pad: GamepadInfo;
  selected: boolean;
  liveBattery: number;
  charging: boolean;
  activeProfileName?: string;
  onSelect: () => void;
  onLed: (rgb: [number, number, number]) => void;
  onRumble: () => void;
}

const SWATCHES: [number, number, number][] = [
  [0, 140, 255],
  [0, 255, 190],
  [168, 85, 247],
  [244, 63, 94],
  [250, 204, 21],
  [255, 255, 255],
];

const hex = ([r, g, b]: [number, number, number]) =>
  `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;

const fromHex = (v: string): [number, number, number] => [
  parseInt(v.slice(1, 3), 16),
  parseInt(v.slice(3, 5), 16),
  parseInt(v.slice(5, 7), 16),
];

export default function GamepadCard({
  pad,
  selected,
  liveBattery,
  charging,
  activeProfileName,
  onSelect,
  onLed,
  onRumble,
}: Props) {
  const battery = liveBattery >= 0 ? liveBattery : pad.battery;
  const rgb = pad.led;
  const glow = `rgb(${rgb[0]},${rgb[1]},${rgb[2]})`;

  const batteryColor = useMemo(() => {
    if (battery < 0) return "bg-slate-600";
    if (battery <= 15) return "bg-rose-500";
    if (battery <= 35) return "bg-amber-400";
    return "bg-emerald-400";
  }, [battery]);

  return (
    <div
      onClick={onSelect}
      className={cn(
        "group relative cursor-pointer overflow-hidden rounded-2xl border p-4 transition-all duration-200",
        selected
          ? "border-cyan-400/50 bg-cyan-400/[0.06] shadow-[0_0_30px_-12px] shadow-cyan-400/60"
          : "border-white/8 bg-white/[0.03] hover:border-white/20 hover:bg-white/[0.05]",
      )}
    >
      <div
        className="pointer-events-none absolute -right-10 -top-10 h-28 w-28 rounded-full blur-3xl transition-opacity"
        style={{ background: glow, opacity: selected ? 0.35 : 0.16 }}
      />

      <div className="relative flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <PadGlyph color={glow} />
            <h3 className="truncate text-sm font-semibold text-slate-100">{pad.name}</h3>
          </div>
          <div className="mt-1.5 flex flex-wrap items-center gap-1.5 font-mono text-[10px]">
            <Tag>{pad.kind === "dualSense" ? "PS5" : pad.kind === "dualShock4" ? "PS4" : "XINPUT"}</Tag>
            <Tag className={pad.connection === "usb" ? "text-emerald-300" : "text-sky-300"}>
              {pad.connection === "usb" ? "USB · WIRED" : "BLUETOOTH"}
            </Tag>
            <Tag>{pad.reportRateHz} Hz</Tag>
            {activeProfileName && (
              <Tag className="border-cyan-400/30 bg-cyan-400/10 text-cyan-200 font-bold">
                🎯 {activeProfileName}
              </Tag>
            )}
          </div>
        </div>
        <span
          className={cn(
            "shrink-0 rounded-full px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider",
            selected ? "bg-cyan-400/20 text-cyan-200" : "bg-white/5 text-slate-400",
          )}
        >
          {selected ? "active slot" : "mapped"}
        </span>
      </div>

      {/* battery */}
      <div className="relative mt-4">
        <div className="mb-1 flex items-center justify-between font-mono text-[10px] text-slate-400">
          <span>BATTERY</span>
          <span className="text-slate-200">
            {battery >= 0 ? `${battery}%` : "n/a"}
            {charging && <span className="ml-1 text-amber-300">⚡ charging</span>}
          </span>
        </div>
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/8">
          <div
            className={cn("h-full rounded-full transition-[width] duration-500", batteryColor)}
            style={{ width: `${Math.max(battery, 0)}%` }}
          />
        </div>
      </div>

      {/* lightbar & actions */}
      {pad.hasLightbar && (
        <div className="relative mt-4" onClick={(e) => e.stopPropagation()}>
          <div className="mb-2 flex items-center justify-between font-mono text-[10px] text-slate-400">
            <span>LIGHTBAR RGB</span>
            <span className="text-slate-300">{hex(rgb).toUpperCase()}</span>
          </div>
          <div
            className="mb-2 h-2 w-full rounded-full transition-all"
            style={{ background: glow, boxShadow: `0 0 18px ${glow}` }}
          />
          <div className="flex items-center gap-1.5">
            {SWATCHES.map((s) => (
              <button
                key={s.join()}
                onClick={() => onLed(s)}
                title={hex(s)}
                className={cn(
                  "h-6 w-6 rounded-md border transition-transform hover:scale-110",
                  hex(s) === hex(rgb) ? "border-white/80" : "border-white/10",
                )}
                style={{ background: `rgb(${s.join(",")})` }}
              />
            ))}
            <label className="relative ml-1 h-6 w-6 cursor-pointer overflow-hidden rounded-md border border-white/15 bg-gradient-to-br from-rose-500 via-amber-300 to-cyan-400">
              <input
                type="color"
                value={hex(rgb)}
                onChange={(e) => onLed(fromHex(e.target.value))}
                className="absolute inset-0 cursor-pointer opacity-0"
              />
            </label>

            <button
              onClick={onRumble}
              className="ml-auto rounded-md border border-white/10 bg-white/5 px-2.5 py-1 font-mono text-[10px] text-slate-300 transition-colors hover:border-cyan-400/40 hover:text-cyan-200"
            >
              TEST RUMBLE
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function Tag({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <span
      className={cn(
        "rounded border border-white/8 bg-white/5 px-1.5 py-0.5 uppercase tracking-wider text-slate-400",
        className,
      )}
    >
      {children}
    </span>
  );
}

function PadGlyph({ color }: { color: string }) {
  return (
    <svg viewBox="0 0 24 24" className="h-4 w-4 shrink-0" fill="none" stroke={color} strokeWidth={1.6}>
      <path d="M6 11h4M8 9v4M15.5 11.5h.01M18 13.5h.01" strokeLinecap="round" />
      <path d="M17.32 5H6.68a4 4 0 0 0-3.87 3l-1.5 6a3.3 3.3 0 0 0 5.9 2.6L8.5 15h7l1.29 1.6a3.3 3.3 0 0 0 5.9-2.6l-1.5-6a4 4 0 0 0-3.87-3Z" />
    </svg>
  );
}
