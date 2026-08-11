import { useEffect, useRef } from "react";
import { useI18n } from "../lib/i18n";
import type { InputSnapshot } from "../lib/types";

interface Props {
  getSnapshot: () => InputSnapshot;
}

const WINDOW_MS = 5000; // 5 s of history
const CHANNELS = [
  { key: "lx" as const, color: "34,211,238", label: "LX" },
  { key: "rx" as const, color: "168,85,247", label: "RX" },
  { key: "lt" as const, color: "250,204,21", label: "LT" },
  { key: "rt" as const, color: "74,222,128", label: "RT" },
];

export default function InputOscilloscope({ getSnapshot }: Props) {
  const { t } = useI18n();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const cv = canvasRef.current;
    const wrap = wrapRef.current;
    if (!cv || !wrap) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    const history: { t: number; v: Record<string, number> }[] = [];
    let raf = 0;
    let w = 0;
    let h = 0;
    const height = 180;

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
      const s = getSnapshot();
      const now = performance.now();
      history.push({
        t: now,
        v: {
          lx: s.left[0],
          rx: s.right[0],
          lt: s.triggerLeft,
          rt: s.triggerRight,
        },
      });
      while (history.length > 0 && now - history[0].t > WINDOW_MS) history.shift();

      ctx.clearRect(0, 0, w, h);

      // background + grid
      ctx.fillStyle = "rgba(2,6,23,0.5)";
      ctx.beginPath();
      ctx.roundRect ? ctx.roundRect(0, 0, w, h, 12) : ctx.rect(0, 0, w, h);
      ctx.fill();
      ctx.strokeStyle = "rgba(148,163,184,0.12)";
      ctx.lineWidth = 1;
      for (let i = 0; i <= 4; i++) {
        const y = (h / 4) * i;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
      }
      // center line (zero)
      ctx.setLineDash([3, 3]);
      ctx.strokeStyle = "rgba(148,163,184,0.25)";
      ctx.beginPath();
      ctx.moveTo(0, h / 2);
      ctx.lineTo(w, h / 2);
      ctx.stroke();
      ctx.setLineDash([]);

      if (history.length > 1) {
        for (const ch of CHANNELS) {
          ctx.beginPath();
          let first = true;
          for (const p of history) {
            const x = w - ((now - p.t) / WINDOW_MS) * w;
            const y = h / 2 - (p.v[ch.key] ?? 0) * (h / 2 - 6);
            if (first) {
              ctx.moveTo(x, y);
              first = false;
            } else {
              ctx.lineTo(x, y);
            }
          }
          ctx.strokeStyle = `rgb(${ch.color})`;
          ctx.lineWidth = 1.6;
          ctx.shadowColor = `rgba(${ch.color},0.5)`;
          ctx.shadowBlur = 6;
          ctx.stroke();
          ctx.shadowBlur = 0;
        }
      }

      // legend
      ctx.font = "600 9px ui-monospace, Menlo, monospace";
      let lx = 8;
      for (const ch of CHANNELS) {
        const label = `${ch.label} ${CHANNELS.length}`;
        const wpx = ctx.measureText(label).width;
        ctx.fillStyle = `rgb(${ch.color})`;
        ctx.fillText(ch.label, lx, 14);
        lx += wpx + 10;
      }

      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, [getSnapshot]);

  return (
    <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-2 flex items-baseline justify-between">
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
            📈 {t("osc.title")}
          </h3>
          <p className="mt-0.5 font-mono text-[9.5px] text-slate-500">{t("osc.sub")}</p>
        </div>
        <span className="font-mono text-[9px] text-slate-600">{t("osc.legend")}</span>
      </div>
      <div ref={wrapRef} className="w-full">
        <canvas ref={canvasRef} className="block w-full" />
      </div>
    </section>
  );
}
