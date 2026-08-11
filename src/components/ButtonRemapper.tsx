import { IDENTITY_BUTTON_MAP } from "../lib/curves";
import { useI18n } from "../lib/i18n";
import { BUTTON_LAYOUT } from "../lib/types";
import { cn } from "../utils/cn";

interface Props {
  buttonMap: number[];
  onChange: (map: number[]) => void;
}

/** Sources/targets exposed by the remapper (the 14 physical PS buttons). */
const ROWS = BUTTON_LAYOUT.map((b) => ({
  bit: Math.round(Math.log2(b.mask)),
  label: b.label,
  xbox: b.xbox,
}));

export default function ButtonRemapper({ buttonMap, onChange }: Props) {
  const { t } = useI18n();
  const map = buttonMap.length === 16 ? buttonMap : IDENTITY_BUTTON_MAP;

  const setMapping = (src: number, dst: number) => {
    const next = [...map];
    // Avoid creating duplicate targets: clear any other source pointing at dst.
    for (let i = 0; i < next.length; i++) {
      if (i !== src && next[i] === dst) next[i] = i;
    }
    next[src] = dst;
    onChange(next);
  };

  return (
    <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
            🔘 {t("remap.title")}
          </h3>
          <p className="mt-0.5 font-mono text-[9.5px] text-slate-500">{t("remap.sub")}</p>
        </div>
        <button
          onClick={() => onChange(IDENTITY_BUTTON_MAP)}
          className="rounded-md border border-white/8 bg-white/5 px-2.5 py-1 font-mono text-[10px] text-slate-400 transition-colors hover:text-slate-100 cursor-pointer"
        >
          {t("remap.reset")}
        </button>
      </div>

      <div className="mb-1.5 grid grid-cols-[1fr_auto_1fr] gap-2 font-mono text-[9px] uppercase tracking-wider text-slate-500">
        <span>{t("remap.source")}</span>
        <span className="text-slate-700">→</span>
        <span>{t("remap.target")}</span>
      </div>

      <div className="space-y-1.5">
        {ROWS.map((row) => {
          const target = map[row.bit] ?? row.bit;
          const targetRow = ROWS.find((r) => r.bit === target);
          const isCustom = target !== row.bit;
          return (
            <div
              key={row.bit}
              className="grid grid-cols-[1fr_auto_1fr] items-center gap-2 rounded-lg border border-white/6 bg-white/[0.02] px-2 py-1.5"
            >
              <div className="flex items-center gap-2">
                <span className="flex h-6 w-6 items-center justify-center rounded-md bg-white/5 font-mono text-[11px] font-bold text-slate-100">
                  {row.label}
                </span>
                <span className="font-mono text-[9px] text-slate-500">{row.xbox}</span>
              </div>
              <span className={cn("font-mono text-[11px]", isCustom ? "text-cyan-300" : "text-slate-700")}>
                →
              </span>
              <div className="flex items-center justify-end gap-2">
                <select
                  value={target}
                  onChange={(e) => setMapping(row.bit, parseInt(e.target.value, 10))}
                  className={cn(
                    "rounded-md border bg-slate-950 px-1.5 py-1 font-mono text-[10px] focus:outline-none",
                    isCustom
                      ? "border-cyan-400/40 text-cyan-200"
                      : "border-white/10 text-slate-400",
                  )}
                >
                  {ROWS.map((r) => (
                    <option key={r.bit} value={r.bit}>
                      {r.label} → {r.xbox}
                    </option>
                  ))}
                </select>
                {isCustom && (
                  <span className="rounded bg-cyan-400/15 px-1.5 py-0.5 font-mono text-[9px] font-bold text-cyan-300">
                    {targetRow?.xbox ?? "?"}
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
