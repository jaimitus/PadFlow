import { useCallback, useEffect, useRef, useState } from "react";
import type { InputSnapshot, StickAxisProfile } from "../lib/types";
import { cn } from "../utils/cn";

interface Props {
  profileLeft: StickAxisProfile;
  profileRight: StickAxisProfile;
  onApplyDeadzone: (side: "left" | "right", recommendedInner: number) => void;
  getSnapshot: () => InputSnapshot;
}

const BUCKETS = 360;

export default function CircularityTester({
  profileLeft,
  profileRight,
  onApplyDeadzone,
  getSnapshot,
}: Props) {
  const [activeStick, setActiveStick] = useState<"left" | "right">("right");

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const profileRef = useRef(profileRight);
  profileRef.current = activeStick === "left" ? profileLeft : profileRight;

  const rawSamplesRef = useRef<number[]>(new Array(BUCKETS).fill(0));
  const restingSamplesRef = useRef<[number, number][]>([]);
  const lastDriftValRef = useRef<number>(0);
  const lastUpdateRef = useRef<number>(0);

  // DOM Refs for throttled 0-rerender telemetry text updates
  const avgErrTxtRef = useRef<HTMLParagraphElement | null>(null);
  const avgErrSubRef = useRef<HTMLSpanElement | null>(null);
  const driftTxtRef = useRef<HTMLParagraphElement | null>(null);
  const driftSubRef = useRef<HTMLSpanElement | null>(null);
  const coverageTxtRef = useRef<HTMLParagraphElement | null>(null);
  const recDzTxtRef = useRef<HTMLSpanElement | null>(null);

  const resetTest = useCallback(() => {
    rawSamplesRef.current = new Array(BUCKETS).fill(0);
    restingSamplesRef.current = [];
    lastDriftValRef.current = 0;

    if (avgErrTxtRef.current) avgErrTxtRef.current.textContent = "—";
    if (avgErrSubRef.current) avgErrSubRef.current.textContent = "Rotate stick along rim...";
    if (driftTxtRef.current) driftTxtRef.current.textContent = "0.0%";
    if (driftSubRef.current) driftSubRef.current.textContent = "✓ Resting Clean";
    if (coverageTxtRef.current) coverageTxtRef.current.textContent = "0%";
    if (recDzTxtRef.current) recDzTxtRef.current.textContent = "3.0%";
  }, []);

  useEffect(() => {
    resetTest();
  }, [activeStick, resetTest]);

  // Auto-calibrate handler
  const handleAutoCalibrate = () => {
    const drift = lastDriftValRef.current;
    const recommended = Math.min(Math.max(Number((drift + 0.015).toFixed(3)), 0.02), 0.35);
    onApplyDeadzone(activeStick, recommended);
  };

  // Dedicated rAF loop — completely decoupled from React render cycle
  useEffect(() => {
    const cv = canvasRef.current;
    const wrap = wrapRef.current;
    if (!cv || !wrap) return;
    const ctx = cv.getContext("2d", { alpha: true });
    if (!ctx) return;

    let raf = 0;
    let size = 280;

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      size = Math.min(wrap.clientWidth, 300);
      cv.width = Math.floor(size * dpr);
      cv.height = Math.floor(size * dpr);
      cv.style.width = `${size}px`;
      cv.style.height = `${size}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);

    let lastAvgErr = 0;

    const tick = () => {
      const snap = getSnapshot();
      const raw = activeStick === "left" ? snap.rawLeft : snap.rawRight;
      const x = raw[0];
      const y = raw[1];
      const mag = Math.hypot(x, y);
      const now = performance.now();
      const prof = profileRef.current;

      // 1. Resting drift calculation
      if (mag < 0.25) {
        const rList = restingSamplesRef.current;
        rList.push([x, y]);
        if (rList.length > 60) rList.shift();
        const avgMag = rList.reduce((acc, p) => acc + Math.hypot(p[0], p[1]), 0) / rList.length;
        lastDriftValRef.current = avgMag;
      }

      // 2. Outer boundary circular tracking
      if (mag > 0.45) {
        let angle = Math.atan2(y, x);
        if (angle < 0) angle += Math.PI * 2;
        const bucket = Math.floor((angle / (Math.PI * 2)) * BUCKETS) % BUCKETS;
        const current = rawSamplesRef.current[bucket];
        if (mag > current) {
          rawSamplesRef.current[bucket] = mag;
        }
      }

      // 3. Throttled UI text updates (4 Hz / every 250ms) without React re-renders
      if (now - lastUpdateRef.current > 200) {
        lastUpdateRef.current = now;
        const samples = rawSamplesRef.current;
        const filled = samples.filter((v) => v > 0);
        const drift = lastDriftValRef.current;

        if (filled.length >= 10) {
          const totalErr = filled.reduce((acc, v) => acc + Math.abs(v - 1.0), 0);
          const maxErr = filled.reduce((acc, v) => Math.max(acc, Math.abs(v - 1.0)), 0);
          lastAvgErr = (totalErr / filled.length) * 100;
          const avgErrStr = lastAvgErr.toFixed(2);
          const maxErrStr = (maxErr * 100).toFixed(1);
          const covStr = `${Math.round((filled.length / BUCKETS) * 100)}%`;

          if (avgErrTxtRef.current) {
            avgErrTxtRef.current.textContent = `${avgErrStr}%`;
            avgErrTxtRef.current.className = cn(
              "mt-0.5 font-mono text-base font-bold tabular-nums",
              lastAvgErr < 8 ? "text-emerald-400" : lastAvgErr < 14 ? "text-amber-300" : "text-rose-400",
            );
          }
          if (avgErrSubRef.current) {
            avgErrSubRef.current.textContent =
              lastAvgErr < 8 ? "✓ Pro Grade (<8%)" : lastAvgErr < 14 ? `Peak +${maxErrStr}%` : "High Deviation";
          }
          if (coverageTxtRef.current) {
            coverageTxtRef.current.textContent = covStr;
          }
        }

        if (driftTxtRef.current) {
          driftTxtRef.current.textContent = `${(drift * 100).toFixed(1)}%`;
          driftTxtRef.current.className = cn(
            "mt-0.5 font-mono text-base font-bold tabular-nums",
            drift <= prof.innerDeadzone ? "text-emerald-400" : "text-rose-400",
          );
        }
        if (driftSubRef.current) {
          driftSubRef.current.textContent =
            drift <= prof.innerDeadzone ? "✓ Filtered by Deadzone" : "⚠️ Hardware Drift Detected";
        }
        if (recDzTxtRef.current) {
          recDzTxtRef.current.textContent = `${((drift + 0.015) * 100).toFixed(1)}%`;
        }
      }

      // -------------------------------------------------------------
      // DRAW CANVAS (Sub-millisecond direct 2D drawing)
      // -------------------------------------------------------------
      ctx.clearRect(0, 0, size, size);

      const cx = size / 2;
      const cy = size / 2;
      const R = size * 0.41;

      // Radar background
      ctx.beginPath();
      ctx.arc(cx, cy, R, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(2, 6, 23, 0.75)";
      ctx.fill();

      // Concentric guide circles (25%, 50%, 75%, 100%)
      ctx.lineWidth = 1;
      for (let i = 1; i <= 4; i++) {
        ctx.beginPath();
        ctx.arc(cx, cy, (R * i) / 4, 0, Math.PI * 2);
        ctx.strokeStyle = i === 4 ? "rgba(148, 163, 184, 0.35)" : "rgba(148, 163, 184, 0.10)";
        if (i === 4) ctx.setLineDash([4, 4]);
        ctx.stroke();
        ctx.setLineDash([]);
      }

      // Crosshair axes
      ctx.beginPath();
      ctx.moveTo(cx - R, cy);
      ctx.lineTo(cx + R, cy);
      ctx.moveTo(cx, cy - R);
      ctx.lineTo(cx, cy + R);
      ctx.strokeStyle = "rgba(148, 163, 184, 0.18)";
      ctx.stroke();

      // Current inner deadzone circle (Rose)
      ctx.beginPath();
      ctx.arc(cx, cy, Math.max(R * prof.innerDeadzone, 3), 0, Math.PI * 2);
      ctx.fillStyle = "rgba(244, 63, 94, 0.12)";
      ctx.fill();
      ctx.strokeStyle = "rgba(244, 63, 94, 0.5)";
      ctx.stroke();

      // Outer saturation circle (Sky)
      ctx.beginPath();
      ctx.arc(cx, cy, R * prof.outerDeadzone, 0, Math.PI * 2);
      ctx.strokeStyle = "rgba(56, 189, 248, 0.35)";
      ctx.stroke();

      // Draw recorded circular points
      const samples = rawSamplesRef.current;
      const pts: [number, number][] = [];
      for (let i = 0; i < BUCKETS; i++) {
        const rad = samples[i];
        if (rad > 0.1) {
          const a = (i / BUCKETS) * Math.PI * 2;
          pts.push([cx + Math.cos(a) * R * rad, cy - Math.sin(a) * R * rad]);
        }
      }

      if (pts.length > 5) {
        ctx.beginPath();
        pts.forEach(([px, py], idx) => {
          if (idx === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        });
        ctx.closePath();

        const gradeColor =
          lastAvgErr < 8 ? "34, 197, 94" : lastAvgErr < 14 ? "250, 204, 21" : "244, 63, 94";

        ctx.fillStyle = `rgba(${gradeColor}, 0.16)`;
        ctx.fill();
        ctx.strokeStyle = `rgb(${gradeColor})`;
        ctx.lineWidth = 1.8;
        ctx.shadowColor = `rgba(${gradeColor}, 0.5)`;
        ctx.shadowBlur = 6;
        ctx.stroke();
        ctx.shadowBlur = 0;
      }

      // Live stick marker
      const markerX = cx + x * R;
      const markerY = cy - y * R;

      ctx.beginPath();
      ctx.arc(markerX, markerY, 5, 0, Math.PI * 2);
      ctx.fillStyle = "#ffffff";
      ctx.shadowColor = "rgba(34, 211, 238, 0.9)";
      ctx.shadowBlur = 10;
      ctx.fill();
      ctx.shadowBlur = 0;
      ctx.strokeStyle = "rgb(34, 211, 238)";
      ctx.lineWidth = 2;
      ctx.stroke();

      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, [activeStick, getSnapshot]);

  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-3.5 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
            Stick Circularity & Drift Benchmark
          </h3>
          <p className="mt-0.5 font-mono text-[9.5px] text-slate-500">
            Rotate stick in complete circles along the outer rim to benchmark circularity error
          </p>
        </div>

        <div className="flex items-center gap-1.5">
          <div className="flex rounded-lg border border-white/8 bg-white/5 p-0.5">
            <button
              onClick={() => setActiveStick("left")}
              className={cn(
                "rounded-md px-2.5 py-0.5 font-mono text-[9.5px] uppercase tracking-wider transition-colors",
                activeStick === "left" ? "bg-cyan-400/20 text-cyan-200 font-bold" : "text-slate-500 hover:text-slate-300",
              )}
            >
              Left Stick
            </button>
            <button
              onClick={() => setActiveStick("right")}
              className={cn(
                "rounded-md px-2.5 py-0.5 font-mono text-[9.5px] uppercase tracking-wider transition-colors",
                activeStick === "right" ? "bg-violet-400/20 text-violet-200 font-bold" : "text-slate-500 hover:text-slate-300",
              )}
            >
              Right Stick
            </button>
          </div>

          <button
            onClick={resetTest}
            className="rounded-md border border-white/8 bg-white/5 px-2.5 py-1 font-mono text-[9.5px] text-slate-400 hover:text-slate-200 transition-colors"
          >
            RESET
          </button>
        </div>
      </div>

      <div className="grid items-center gap-5 lg:grid-cols-[auto_minmax(0,1fr)]">
        {/* Radar Canvas */}
        <div ref={wrapRef} className="flex justify-center">
          <canvas ref={canvasRef} className="block touch-none select-none rounded-xl" />
        </div>

        {/* Metrics and Calibration Action */}
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
            <div className="rounded-xl border border-white/8 bg-slate-950/60 p-3">
              <p className="font-mono text-[9px] uppercase tracking-wider text-slate-500">Avg Error</p>
              <p ref={avgErrTxtRef} className="mt-0.5 font-mono text-base font-bold text-slate-500 tabular-nums">
                —
              </p>
              <span ref={avgErrSubRef} className="font-mono text-[8.5px] text-slate-500">
                Rotate stick along rim...
              </span>
            </div>

            <div className="rounded-xl border border-white/8 bg-slate-950/60 p-3">
              <p className="font-mono text-[9px] uppercase tracking-wider text-slate-500">Center Drift</p>
              <p ref={driftTxtRef} className="mt-0.5 font-mono text-base font-bold text-emerald-400 tabular-nums">
                0.0%
              </p>
              <span ref={driftSubRef} className="font-mono text-[8.5px] text-slate-500">
                ✓ Resting Clean
              </span>
            </div>

            <div className="rounded-xl border border-white/8 bg-slate-950/60 p-3 col-span-2 sm:col-span-1">
              <p className="font-mono text-[9px] uppercase tracking-wider text-slate-500">Coverage</p>
              <p ref={coverageTxtRef} className="mt-0.5 font-mono text-base font-bold text-slate-100 tabular-nums">
                0%
              </p>
              <span className="font-mono text-[8.5px] text-slate-500">360° perimeter scan</span>
            </div>
          </div>

          <div className="rounded-xl border border-white/8 bg-white/[0.02] p-3.5 flex flex-wrap items-center justify-between gap-3">
            <div>
              <p className="text-xs font-semibold text-slate-200">
                Recommended Deadzone: <span ref={recDzTxtRef} className="text-cyan-300 font-mono">3.0%</span>
              </p>
              <p className="mt-0.5 font-mono text-[9.5px] text-slate-500">
                Automatically suppresses resting hardware drift with a clean 1.5% safety margin.
              </p>
            </div>

            <button
              onClick={handleAutoCalibrate}
              className="rounded-lg bg-gradient-to-r from-cyan-400 to-violet-500 px-3.5 py-1.5 font-mono text-[10px] font-bold uppercase text-slate-950 shadow-md shadow-cyan-400/20 hover:brightness-110 transition-all"
            >
              🎯 Apply Optimal Deadzone
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
