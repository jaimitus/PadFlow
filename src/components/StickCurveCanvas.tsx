import { useCallback, useEffect, useRef, useState } from "react";
import { CURVE_LABELS, shapeMagnitude } from "../lib/curves";
import type { StickAxisProfile } from "../lib/types";

export interface StickLiveSample {
  raw: [number, number];
  shaped: [number, number];
}

interface Props {
  label: string;
  accent: string; // "r,g,b"
  profile: StickAxisProfile;
  /** Called while dragging handles on the canvas. */
  onChange: (patch: Partial<StickAxisProfile>) => void;
  /** Pull-based so the 60 FPS canvas never triggers React re-renders. */
  getSample: () => StickLiveSample;
  height?: number;
}

type DragTarget = "inner" | "outer" | "power" | null;

const PAD = { l: 44, r: 18, t: 18, b: 30 };

export default function StickCurveCanvas({
  label,
  accent,
  profile,
  onChange,
  getSample,
  height = 260,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const profileRef = useRef(profile);
  const dragRef = useRef<DragTarget>(null);
  const hoverRef = useRef<DragTarget>(null);
  const trailRef = useRef<{ x: number; y: number; a: number }[]>([]);
  const fpsRef = useRef({ frames: 0, last: performance.now(), value: 60 });
  const [fps, setFps] = useState(60);
  const [hint, setHint] = useState<string>("");

  profileRef.current = profile;

  // ---- geometry helpers ----------------------------------------------------
  const plot = (w: number, h: number) => ({
    x0: PAD.l,
    y0: h - PAD.b,
    w: w - PAD.l - PAD.r,
    h: h - PAD.t - PAD.b,
  });

  // ---- pointer interaction -------------------------------------------------
  const pickTarget = useCallback(
    (px: number, py: number, w: number, h: number): DragTarget => {
      const g = plot(w, h);
      const p = profileRef.current;
      const ix = g.x0 + p.innerDeadzone * g.w;
      const ox = g.x0 + p.outerDeadzone * g.w;
      if (Math.abs(px - ix) < 9) return "inner";
      if (Math.abs(px - ox) < 9) return "outer";
      if (px > g.x0 && px < g.x0 + g.w && py > PAD.t && py < g.y0) return "power";
      return null;
    },
    [],
  );

  const applyDrag = useCallback(
    (target: DragTarget, px: number, py: number, w: number, h: number) => {
      const g = plot(w, h);
      const p = profileRef.current;
      const tx = Math.max(0, Math.min(1, (px - g.x0) / g.w));
      const ty = Math.max(0, Math.min(1, 1 - (py - PAD.t) / g.h));
      if (target === "inner") {
        onChange({
          innerDeadzone: Math.max(0, Math.min(tx, p.outerDeadzone - 0.05)),
        });
        setHint(`Inner deadzone ${(tx * 100).toFixed(1)}%`);
      } else if (target === "outer") {
        onChange({
          outerDeadzone: Math.max(p.innerDeadzone + 0.05, Math.min(1, tx)),
        });
        setHint(`Outer deadzone ${(tx * 100).toFixed(1)}%`);
      } else if (target === "power") {
        // Drag above the diagonal ⇒ faster ramp, below ⇒ slower ramp.
        const ref = Math.max(0.02, tx);
        let power = Math.log(Math.max(ty, 0.001)) / Math.log(ref);
        if (p.curve === "aggressive") {
          power = Math.log(Math.max(1 - ty, 0.001)) / Math.log(Math.max(1 - ref, 0.001));
        }
        const clamped = Math.max(0.5, Math.min(4, power));
        onChange({ curvePower: Number(clamped.toFixed(2)) });
        setHint(`${CURVE_LABELS[p.curve]} power ${clamped.toFixed(2)}`);
      }
    },
    [onChange],
  );

  useEffect(() => {
    const cv = canvasRef.current;
    if (!cv) return;
    const rectOf = () => cv.getBoundingClientRect();

    const onDown = (e: PointerEvent) => {
      const r = rectOf();
      const t = pickTarget(e.clientX - r.left, e.clientY - r.top, r.width, r.height);
      if (!t) return;
      dragRef.current = t;
      cv.setPointerCapture(e.pointerId);
      applyDrag(t, e.clientX - r.left, e.clientY - r.top, r.width, r.height);
    };
    const onMove = (e: PointerEvent) => {
      const r = rectOf();
      const px = e.clientX - r.left;
      const py = e.clientY - r.top;
      hoverRef.current = pickTarget(px, py, r.width, r.height);
      cv.style.cursor =
        hoverRef.current === "inner" || hoverRef.current === "outer"
          ? "ew-resize"
          : hoverRef.current === "power"
            ? "ns-resize"
            : "default";
      if (dragRef.current) applyDrag(dragRef.current, px, py, r.width, r.height);
    };
    const onUp = (e: PointerEvent) => {
      dragRef.current = null;
      try {
        cv.releasePointerCapture(e.pointerId);
      } catch {
        /* pointer already released */
      }
      setTimeout(() => setHint(""), 900);
    };

    cv.addEventListener("pointerdown", onDown);
    cv.addEventListener("pointermove", onMove);
    cv.addEventListener("pointerup", onUp);
    cv.addEventListener("pointercancel", onUp);
    return () => {
      cv.removeEventListener("pointerdown", onDown);
      cv.removeEventListener("pointermove", onMove);
      cv.removeEventListener("pointerup", onUp);
      cv.removeEventListener("pointercancel", onUp);
    };
  }, [applyDrag, pickTarget]);

  // ---- render loop (60 FPS, rAF driven) ------------------------------------
  useEffect(() => {
    const cv = canvasRef.current;
    const wrap = wrapRef.current;
    if (!cv || !wrap) return;
    const ctx = cv.getContext("2d", { alpha: true });
    if (!ctx) return;

    let raf = 0;
    let w = 0;
    let h = 0;

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = wrap.clientWidth;
      h = height;
      cv.width = Math.floor(w * dpr);
      cv.height = Math.floor(h * dpr);
      cv.style.width = `${w}px`;
      cv.style.height = `${h}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);

    const draw = () => {
      const p = profileRef.current;
      const g = plot(w, h);
      const sample = getSample();
      const rawMag = Math.min(1, Math.hypot(sample.raw[0], sample.raw[1]));
      const outMag = Math.min(1, Math.hypot(sample.shaped[0], sample.shaped[1]));

      ctx.clearRect(0, 0, w, h);

      // background
      const bg = ctx.createLinearGradient(0, 0, 0, h);
      bg.addColorStop(0, "rgba(255,255,255,0.035)");
      bg.addColorStop(1, "rgba(255,255,255,0.008)");
      ctx.fillStyle = bg;
      roundRect(ctx, 1, 1, w - 2, h - 2, 14);
      ctx.fill();

      // grid
      ctx.strokeStyle = "rgba(148,163,184,0.13)";
      ctx.lineWidth = 1;
      ctx.font = "10px ui-monospace, SFMono-Regular, Menlo, monospace";
      ctx.fillStyle = "rgba(148,163,184,0.55)";
      for (let i = 0; i <= 4; i++) {
        const y = PAD.t + (g.h * i) / 4;
        ctx.beginPath();
        ctx.moveTo(g.x0, y);
        ctx.lineTo(g.x0 + g.w, y);
        ctx.stroke();
        ctx.textAlign = "right";
        ctx.fillText(`${100 - i * 25}%`, g.x0 - 8, y + 3);
      }
      for (let i = 0; i <= 4; i++) {
        const x = g.x0 + (g.w * i) / 4;
        ctx.beginPath();
        ctx.moveTo(x, PAD.t);
        ctx.lineTo(x, g.y0);
        ctx.stroke();
        ctx.textAlign = "center";
        ctx.fillText(`${i * 25}`, x, g.y0 + 16);
      }

      // reference diagonal (1:1)
      ctx.setLineDash([4, 4]);
      ctx.strokeStyle = "rgba(148,163,184,0.35)";
      ctx.beginPath();
      ctx.moveTo(g.x0, g.y0);
      ctx.lineTo(g.x0 + g.w, PAD.t);
      ctx.stroke();
      ctx.setLineDash([]);

      // deadzone bands
      ctx.fillStyle = "rgba(244,63,94,0.14)";
      ctx.fillRect(g.x0, PAD.t, p.innerDeadzone * g.w, g.h);
      ctx.fillStyle = "rgba(56,189,248,0.10)";
      ctx.fillRect(
        g.x0 + p.outerDeadzone * g.w,
        PAD.t,
        Math.max(0, (1 - p.outerDeadzone) * g.w),
        g.h,
      );

      // curve
      const steps = 160;
      const pts: [number, number][] = [];
      for (let i = 0; i <= steps; i++) {
        const t = i / steps;
        const v = shapeMagnitude(t, p);
        pts.push([g.x0 + t * g.w, g.y0 - v * g.h]);
      }
      // area under curve
      const area = ctx.createLinearGradient(0, PAD.t, 0, g.y0);
      area.addColorStop(0, `rgba(${accent},0.30)`);
      area.addColorStop(1, `rgba(${accent},0.02)`);
      ctx.beginPath();
      ctx.moveTo(g.x0, g.y0);
      pts.forEach(([x, y]) => ctx.lineTo(x, y));
      ctx.lineTo(g.x0 + g.w, g.y0);
      ctx.closePath();
      ctx.fillStyle = area;
      ctx.fill();

      ctx.beginPath();
      pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
      ctx.strokeStyle = `rgb(${accent})`;
      ctx.lineWidth = 2.25;
      ctx.shadowColor = `rgba(${accent},0.65)`;
      ctx.shadowBlur = 12;
      ctx.stroke();
      ctx.shadowBlur = 0;

      // deadzone handles
      const drawHandle = (t: number, color: string, id: DragTarget) => {
        const x = g.x0 + t * g.w;
        const active = hoverRef.current === id || dragRef.current === id;
        ctx.strokeStyle = color;
        ctx.lineWidth = active ? 2 : 1.25;
        ctx.beginPath();
        ctx.moveTo(x, PAD.t);
        ctx.lineTo(x, g.y0);
        ctx.stroke();
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(x, PAD.t + 6, active ? 5.5 : 4, 0, Math.PI * 2);
        ctx.fill();
      };
      drawHandle(p.innerDeadzone, "rgba(244,63,94,0.95)", "inner");
      drawHandle(p.outerDeadzone, "rgba(56,189,248,0.95)", "outer");

      // live trail
      const lx = g.x0 + rawMag * g.w;
      const ly = g.y0 - outMag * g.h;
      const trail = trailRef.current;
      trail.push({ x: lx, y: ly, a: 1 });
      if (trail.length > 26) trail.shift();
      trail.forEach((pt, i) => {
        pt.a = (i + 1) / trail.length;
        ctx.beginPath();
        ctx.arc(pt.x, pt.y, 1.6 + pt.a * 2, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${accent},${pt.a * 0.35})`;
        ctx.fill();
      });

      // live marker
      ctx.beginPath();
      ctx.arc(lx, ly, 6.5, 0, Math.PI * 2);
      ctx.fillStyle = "#fff";
      ctx.shadowColor = `rgba(${accent},0.9)`;
      ctx.shadowBlur = 16;
      ctx.fill();
      ctx.shadowBlur = 0;
      ctx.strokeStyle = `rgb(${accent})`;
      ctx.lineWidth = 2;
      ctx.stroke();

      // crosshair guides
      ctx.setLineDash([2, 3]);
      ctx.strokeStyle = "rgba(255,255,255,0.28)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(g.x0, ly);
      ctx.lineTo(lx, ly);
      ctx.moveTo(lx, g.y0);
      ctx.lineTo(lx, ly);
      ctx.stroke();
      ctx.setLineDash([]);

      // XY radar inset
      const R = 42;
      const cx = g.x0 + g.w - R - 10;
      const cy = PAD.t + R + 8;
      ctx.beginPath();
      ctx.arc(cx, cy, R, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(2,6,23,0.65)";
      ctx.fill();
      ctx.strokeStyle = "rgba(148,163,184,0.28)";
      ctx.lineWidth = 1;
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx, cy, Math.max(2, R * p.innerDeadzone), 0, Math.PI * 2);
      ctx.strokeStyle = "rgba(244,63,94,0.6)";
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx, cy, R * p.outerDeadzone, 0, Math.PI * 2);
      ctx.strokeStyle = "rgba(56,189,248,0.45)";
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(cx - R, cy);
      ctx.lineTo(cx + R, cy);
      ctx.moveTo(cx, cy - R);
      ctx.lineTo(cx, cy + R);
      ctx.strokeStyle = "rgba(148,163,184,0.18)";
      ctx.stroke();
      // raw ghost
      ctx.beginPath();
      ctx.arc(cx + sample.raw[0] * R, cy - sample.raw[1] * R, 3.2, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(148,163,184,0.75)";
      ctx.fill();
      // shaped
      ctx.beginPath();
      ctx.arc(cx + sample.shaped[0] * R, cy - sample.shaped[1] * R, 4.2, 0, Math.PI * 2);
      ctx.fillStyle = `rgb(${accent})`;
      ctx.shadowColor = `rgba(${accent},0.9)`;
      ctx.shadowBlur = 10;
      ctx.fill();
      ctx.shadowBlur = 0;

      // readouts
      ctx.font = "600 10px ui-monospace, SFMono-Regular, Menlo, monospace";
      ctx.textAlign = "left";
      ctx.fillStyle = "rgba(226,232,240,0.75)";
      ctx.fillText(
        `IN ${(rawMag * 100).toFixed(0).padStart(3, " ")}%   OUT ${(outMag * 100)
          .toFixed(0)
          .padStart(3, " ")}%`,
        g.x0 + 6,
        PAD.t + 14,
      );

      // fps counter
      const f = fpsRef.current;
      f.frames++;
      const now = performance.now();
      if (now - f.last >= 500) {
        f.value = Math.round((f.frames * 1000) / (now - f.last));
        f.frames = 0;
        f.last = now;
        setFps(f.value);
      }

      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, [accent, getSample, height]);

  return (
    <div className="relative">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span
            className="h-2 w-2 rounded-full"
            style={{ background: `rgb(${accent})`, boxShadow: `0 0 10px rgb(${accent})` }}
          />
          <h4 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
            {label}
          </h4>
          <span className="rounded bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-slate-400">
            {CURVE_LABELS[profile.curve]} · p{profile.curvePower.toFixed(2)}
          </span>
        </div>
        <span className="font-mono text-[10px] text-slate-500">
          {hint || `canvas ${fps} fps`}
        </span>
      </div>
      <div ref={wrapRef} className="w-full">
        <canvas ref={canvasRef} className="block w-full touch-none select-none" />
      </div>
      <p className="mt-1.5 font-mono text-[10px] text-slate-500">
        drag ● handles → deadzones · drag inside plot ↕ → curve power
      </p>
    </div>
  );
}

function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}
