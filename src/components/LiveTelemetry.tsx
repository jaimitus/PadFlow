import { useEffect, useRef } from "react";
import { BUTTON_LAYOUT } from "../lib/types";
import type { InputSnapshot } from "../lib/types";

interface Props {
  getSnapshot: () => InputSnapshot;
  flipTriggers?: boolean;
}

/**
 * Imperative, ref-driven telemetry strip: buttons, triggers, touchpad and
 * motion. Updated in a single rAF pass so React never re-renders at 60 Hz.
 */
export default function LiveTelemetry({ getSnapshot, flipTriggers = false }: Props) {
  const btnRefs = useRef<(HTMLDivElement | null)[]>([]);
  const ltRef = useRef<HTMLDivElement | null>(null);
  const rtRef = useRef<HTMLDivElement | null>(null);
  const ltTxt = useRef<HTMLSpanElement | null>(null);
  const rtTxt = useRef<HTMLSpanElement | null>(null);
  const touchRef = useRef<HTMLCanvasElement | null>(null);
  const gyroRef = useRef<HTMLSpanElement | null>(null);
  const dpadRefs = useRef<(HTMLDivElement | null)[]>([]);

  useEffect(() => {
    let raf = 0;
    const tick = () => {
      const s = getSnapshot();
      BUTTON_LAYOUT.forEach((b, i) => {
        const el = btnRefs.current[i];
        if (!el) return;
        const on = (s.buttons & b.mask) !== 0;
        el.dataset.on = on ? "1" : "0";
      });
      const dirs = [0, 2, 4, 6];
      const active = new Set<number>();
      if (s.dpad <= 7) {
        active.add(s.dpad % 8);
        if (s.dpad % 2 === 1) {
          active.add((s.dpad - 1) % 8);
          active.add((s.dpad + 1) % 8);
        }
      }
      dirs.forEach((d, i) => {
        const el = dpadRefs.current[i];
        if (el) el.dataset.on = active.has(d) ? "1" : "0";
      });
      if (ltRef.current) ltRef.current.style.width = `${s.triggerLeft * 100}%`;
      if (rtRef.current) rtRef.current.style.width = `${s.triggerRight * 100}%`;
      if (ltTxt.current) ltTxt.current.textContent = `${Math.round(s.triggerLeft * 255)}`;
      if (rtTxt.current) rtTxt.current.textContent = `${Math.round(s.triggerRight * 255)}`;
      if (gyroRef.current)
        gyroRef.current.textContent = `${s.gyro.map((v) => v.toFixed(2).padStart(5, " ")).join(" ")}  |  ${s.accel
          .map((v) => v.toFixed(2).padStart(5, " "))
          .join(" ")}`;

      const cv = touchRef.current;
      if (cv) {
        const ctx = cv.getContext("2d");
        if (ctx) {
          const w = cv.width;
          const h = cv.height;
          ctx.clearRect(0, 0, w, h);
          ctx.strokeStyle = "rgba(148,163,184,0.22)";
          ctx.lineWidth = 1;
          ctx.strokeRect(0.5, 0.5, w - 1, h - 1);
          ctx.strokeStyle = "rgba(148,163,184,0.10)";
          ctx.beginPath();
          ctx.moveTo(w / 2, 0);
          ctx.lineTo(w / 2, h);
          ctx.stroke();
          s.touchPoints.forEach((p, i) => {
            ctx.beginPath();
            ctx.arc(p[0] * w, p[1] * h, 7, 0, Math.PI * 2);
            ctx.fillStyle = i === 0 ? "rgba(34,211,238,0.85)" : "rgba(168,85,247,0.85)";
            ctx.shadowColor = ctx.fillStyle as string;
            ctx.shadowBlur = 12;
            ctx.fill();
            ctx.shadowBlur = 0;
          });
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [getSnapshot]);

  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <h3 className="mb-3 text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
        Live input map <span className="ml-1 text-slate-600">PS → XInput</span>
      </h3>

      <div className="grid grid-cols-7 gap-1.5">
        {BUTTON_LAYOUT.map((b, i) => (
          <div
            key={b.label}
            ref={(el) => {
              btnRefs.current[i] = el;
            }}
            data-on="0"
            className="pf-btn flex flex-col items-center justify-center rounded-lg border border-white/8 bg-white/[0.03] py-1.5"
          >
            <span className="text-[12px] leading-none text-slate-200">{b.label}</span>
            <span className="mt-0.5 font-mono text-[8px] leading-none text-slate-500">
              {b.xbox}
            </span>
          </div>
        ))}
      </div>

      <div className="mt-4 flex items-start gap-4">
        <div className="flex-1 space-y-2.5">
          <TriggerBar
            name={flipTriggers ? "L1 → LT (FLIPPED)" : "L2 → LT"}
            barRef={ltRef}
            txtRef={ltTxt}
            color="34,211,238"
          />
          <TriggerBar
            name={flipTriggers ? "R1 → RT (FLIPPED)" : "R2 → RT"}
            barRef={rtRef}
            txtRef={rtTxt}
            color="168,85,247"
          />
          <div className="pt-1">
            <p className="mb-1 font-mono text-[10px] text-slate-500">GYRO / ACCEL</p>
            <span
              ref={gyroRef}
              className="block whitespace-pre font-mono text-[10px] tabular-nums text-slate-300"
            >
              0.00 0.00 0.00 | 0.00 0.00 0.98
            </span>
          </div>
        </div>

        <div>
          <p className="mb-1 font-mono text-[10px] text-slate-500">D-PAD</p>
          <div className="grid h-[70px] w-[70px] grid-cols-3 grid-rows-3 gap-0.5">
            <span />
            <div ref={(el) => { dpadRefs.current[0] = el; }} data-on="0" className="pf-btn rounded-sm border border-white/8 bg-white/[0.03]" />
            <span />
            <div ref={(el) => { dpadRefs.current[3] = el; }} data-on="0" className="pf-btn rounded-sm border border-white/8 bg-white/[0.03]" />
            <span />
            <div ref={(el) => { dpadRefs.current[1] = el; }} data-on="0" className="pf-btn rounded-sm border border-white/8 bg-white/[0.03]" />
            <span />
            <div ref={(el) => { dpadRefs.current[2] = el; }} data-on="0" className="pf-btn rounded-sm border border-white/8 bg-white/[0.03]" />
            <span />
          </div>
        </div>

        <div>
          <p className="mb-1 font-mono text-[10px] text-slate-500">TOUCHPAD</p>
          <canvas
            ref={touchRef}
            width={150}
            height={70}
            className="rounded-md bg-slate-950/60"
          />
        </div>
      </div>
    </div>
  );
}

function TriggerBar({
  name,
  barRef,
  txtRef,
  color,
}: {
  name: string;
  barRef: React.RefObject<HTMLDivElement | null>;
  txtRef: React.RefObject<HTMLSpanElement | null>;
  color: string;
}) {
  return (
    <div>
      <div className="mb-1 flex items-baseline justify-between font-mono text-[10px]">
        <span className="text-slate-400">{name}</span>
        <span ref={txtRef} className="tabular-nums text-slate-200">
          0
        </span>
      </div>
      <div className="h-2 w-full overflow-hidden rounded-full bg-white/8">
        <div
          ref={barRef}
          className="h-full rounded-full"
          style={{
            width: "0%",
            background: `linear-gradient(90deg, rgba(${color},0.5), rgb(${color}))`,
            boxShadow: `0 0 12px rgba(${color},0.6)`,
          }}
        />
      </div>
    </div>
  );
}
