import { useI18n } from "../lib/i18n";
import { cn } from "../utils/cn";

export interface GameMapping {
  exe: string;
  profileName: string;
}

interface Props {
  foreground: string | null;
  mappings: GameMapping[];
  onAssign: () => void;
  onRemove: (exe: string) => void;
}

export default function GameProfilesPanel({ foreground, mappings, onAssign, onRemove }: Props) {
  const { t } = useI18n();

  return (
    <section className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <div className="mb-2">
        <h3 className="text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
          🎮 {t("games.title")}
        </h3>
        <p className="mt-0.5 font-mono text-[9.5px] text-slate-500">{t("games.sub")}</p>
      </div>

      <div className="mb-3 flex items-center justify-between rounded-xl border border-white/6 bg-white/[0.02] px-3 py-2">
        <div className="min-w-0">
          <p className="font-mono text-[9.5px] uppercase tracking-wider text-slate-500">
            {t("games.current")}
          </p>
          <p className="truncate font-mono text-[11px] text-slate-200">
            {foreground ?? t("games.none")}
          </p>
        </div>
        <button
          onClick={onAssign}
          disabled={!foreground}
          className="ml-2 shrink-0 rounded-lg bg-gradient-to-r from-cyan-400 to-violet-500 px-3 py-1.5 font-mono text-[10px] font-bold text-slate-950 shadow-md shadow-cyan-400/20 transition-all hover:brightness-110 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
        >
          {t("games.assign")}
        </button>
      </div>

      {mappings.length === 0 ? (
        <p className="font-mono text-[10px] text-slate-600">{t("games.noMapping")}</p>
      ) : (
        <div className="space-y-1.5">
          {mappings.map((m) => (
            <div
              key={m.exe}
              className={cn(
                "flex items-center justify-between gap-2 rounded-lg border px-2.5 py-1.5",
                foreground === m.exe
                  ? "border-cyan-400/40 bg-cyan-400/[0.07]"
                  : "border-white/6 bg-white/[0.02]",
              )}
            >
              <div className="min-w-0">
                <p className="truncate font-mono text-[11px] text-slate-200">{m.exe}</p>
                <p className="truncate font-mono text-[9.5px] text-slate-500">
                  🎯 {m.profileName}
                </p>
              </div>
              {foreground === m.exe && (
                <span className="shrink-0 rounded-full bg-emerald-400/15 px-1.5 py-0.5 font-mono text-[9px] uppercase text-emerald-300">
                  {t("profiles.live")}
                </span>
              )}
              <button
                onClick={() => onRemove(m.exe)}
                title={t("games.remove")}
                className="shrink-0 rounded p-1 font-mono text-[10px] text-slate-500 transition-colors hover:bg-rose-500/20 hover:text-rose-300 cursor-pointer"
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
