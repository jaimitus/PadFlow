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
  const [avgError, setAvgError] = useState<number | null>(null);
  const [maxError, setMaxError] = useState<number | null>(null);
  const [centerDrift, setCenterDrift] = useState<number>(0);
  const [coveragePct, setCoveragePct] = useState<number>(0);

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const rawSamplesRef = useRef<number[]>(new Array(BUCKETS).fill(0));
  const restingSamplesRef = useRef<[number, number][]>([]);

  const profile = activeStick === "left" ? profileLeft : profileRight;

  const resetTest = useCallback(() => {
    rawSamplesRef.current = new Array(BUCKETS).fill(0);
    restingSamplesRef.current = [];
    setAvgError(null);
    setMaxError(null);
    setCenterDrift(0);
    setCoveragePct(0);
  }, []);

  useEffect(() => {
    resetTest();
  }, [activeStick, resetTest]);

  // Auto-calibrate handler
  const handleAutoCalibrate = () => {
    const drift = centerDrift;
    const recommended = Math.min(Math.max(Number((drift + 0.015).toFixed(3)), 0.02), 0.35);
    onApplyDeadzone(activeStick, recommended);
  };

  // Render loop
  useEffect(() => {
    const cv = canvasRef.current;
    const wrap = wrapRef.current;
    if (!cv || !wrap) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    let raf = 0;
    let size = 280;

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      size = Math.min(wrap.clientWidth, 320);
      cv.width = Math.floor(size * dpr);
      cv.height = Math.floor(size * dpr);
      cv.style.width = `${size}px`;
      cv.style.height = `${size}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);

    const tick = () => {
      const snap = getSnapshot();
      const raw = activeStick === "left" ? snap.rawLeft : snap.rawRight;
      const x = raw[0];
      const y = raw[1];
      const mag = Math.hypot(x, y);

      // Resting drift tracking (when stick magnitude < 0.25)
      if (mag < 0.25) {
        const rList = restingSamplesRef.current;
        rList.push([x, y]);
        if (rList.length > 60) rList.shift();
        const avgMag = rList.reduce((acc, p) => acc + Math.hypot(p[0], p[1]), 0) / rList.length;
        setCenterDrift(avgMag);
      }

      // Outer rim tracking (when stick magnitude > 0.45)
      if (mag > 0.45) {
        let angle = Math.atan2(y, x);
        if (angle < 0) angle += Math.PI * 2;
        const bucket = Math.floor((angle / (Math.PI * 2)) * BUCKETS) % BUCKETS;
        const current = rawSamplesRef.current[bucket];
        if (mag > current) {
          rawSamplesRef.current[bucket] = mag;
        }

        // Calculate statistics
        const samples = rawSamplesRef.current;
        const filled = samples.filter((v) => v > 0);
        if (filled.length >= 20) {
          const totalErr = filled.reduce((acc, v) => acc + Math.abs(v - 1.0), 0);
          const maxErr = filled.reduce((acc, v) => Math.max(acc, Math.abs(v - 1.0)), 0);
          setAvgError(Number(((totalErr / filled.length) * 100).toFixed(2)));
          setMaxError(Number((maxErr * 100).toFixed(2)));
          setCoveragePct(Math.round((filled.length / BUCKETS) * 100));
        }
      }

      // -------------------------------------------------------------
      // DRAW CANVAS
      // -------------------------------------------------------------
      ctx.clearRect(0, 0, size, size);

      const cx = size / 2;
      const cy = size / 2;
      const R = size * 0.42;

      // Dark radar background
      ctx.beginPath();
      ctx.arc(cx, cy, R, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(2, 6, 23, 0.75)";
      ctx.fill();

      // Concentric guide circles (25%, 50%, 75%, 100%)
      ctx.lineWidth = 1;
      for (let i = 1; i <= 4; i++) {
        ctx.beginPath();
        ctx.arc(cx, cy, (R * i) / 4, 0, Math.PI * 2);
        ctx.strokeStyle = i === 4 ? "rgba(148, 163, 184, 0.4)" : "rgba(148, 163, 184, 0.12)";
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
      ctx.strokeStyle = "rgba(148, 163, 184, 0.2)";
      ctx.stroke();

      // Current inner deadzone circle (Rose)
      ctx.beginPath();
      ctx.arc(cx, cy, Math.max(R * profile.innerDeadzone, 3), 0, Math.PI * 2);
      ctx.fillStyle = "rgba(244, 63, 94, 0.15)";
      ctx.fill();
      ctx.strokeStyle = "rgba(244, 63, 94, 0.6)";
      ctx.stroke();

      // Outer saturation circle (Sky)
      ctx.beginPath();
      ctx.arc(cx, cy, R * profile.outerDeadzone, 0, Math.PI * 2);
      ctx.strokeStyle = "rgba(56, 189, 248, 0.4)";
      ctx.stroke();

      // Draw recorded circular polygon
      const samples = rawSamplesRef.current;
      const pts: [number, number][] = [];
      for (let i = 0; i < BUCKETS; i++) {
        const rad = samples[i] > 0 ? samples[i] : 0;
        if (rad > 0) {
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
          avgError !== null && avgError < 8
            ? "34, 197, 94"
            : avgError !== null && avgError < 14
              ? "250, 204, 21"
              : "244, 63, 94";

        ctx.fillStyle = `rgba(${gradeColor}, 0.18)`;
        ctx.fill();
        ctx.strokeStyle = `rgb(${gradeColor})`;
        ctx.lineWidth = 1.8;
        ctx.shadowColor = `rgba(${gradeColor}, 0.6)`;
        ctx.shadowBlur = 8;
        ctx.stroke();
        ctx.shadowBlur = 0;
      }

      // Live stick marker
      const markerX = cx + x * R;
      const markerY = cy - y * R;

      ctx.beginPath();
      ctx.arc(markerX, markerY, 5.5, 0, Math.PI * 2);
      ctx.fillStyle = "#ffffff";
      ctx.shadowColor = "rgba(34, 211, 238, 0.9)";
      ctx.shadowBlur = 12;
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
  }, [activeStick, avgError, getSnapshot, profile.innerDeadzone, profile.outerDeadzone]);

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
              <p
                className={cn(
                  "mt-0.5 font-mono text-base font-bold tabular-nums",
                  avgError === null
                    ? "text-slate-500"
                    : avgError < 8
                      ? "text-emerald-400"
                      : avgError < 14
                        ? "text-amber-300"
                        : "text-rose-400",
                )}
              >
                {avgError !== null ? `${avgError}%` : "—"}
              </p>
              <span className="font-mono text-[8.5px] text-slate-500">
                {avgError === null
                  ? "Rotate stick..."
                  : maxError !== null
                    ? `Peak ${maxError}%`
                    : "Standard"}
              </span>
            </div>

            <div className="rounded-xl border border-white/8 bg-slate-950/60 p-3">
              <p className="font-mono text-[9px] uppercase tracking-wider text-slate-500">Center Drift</p>
              <p
                className={cn(
                  "mt-0.5 font-mono text-base font-bold tabular-nums",
                  centerDrift <= profile.innerDeadzone ? "text-emerald-400" : "text-rose-400",
                )}
              >
                {(centerDrift * 100).toFixed(1)}%
              </p>
              <span className="font-mono text-[8.5px] text-slate-500">
                {centerDrift <= profile.innerDeadzone ? "✓ Filtered by DZ" : "⚠️ Stick Drift"}
              </span>
            </div>

            <div className="rounded-xl border border-white/8 bg-slate-950/60 p-3 col-span-2 sm:col-span-1">
              <p className="font-mono text-[9px] uppercase tracking-wider text-slate-500">Coverage</p>
              <p className="mt-0.5 font-mono text-base font-bold text-slate-100 tabular-nums">
                {coveragePct}%
              </p>
              <span className="font-mono text-[8.5px] text-slate-500">360° perimeter scan</span>
            </div>
          </div>

          <div className="rounded-xl border border-white/8 bg-white/[0.02] p-3.5 flex flex-wrap items-center justify-between gap-3">
            <div>
              <p className="text-xs font-semibold text-slate-200">
                Recommended Deadzone: <span className="text-cyan-300 font-mono">{((centerDrift + 0.015) * 100).toFixed(1)}%</span>
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
