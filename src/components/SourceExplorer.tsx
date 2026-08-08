import { useMemo, useState } from "react";
import cargoToml from "../../src-tauri/Cargo.toml?raw";
import gamepadRs from "../../src-tauri/src/input/gamepad.rs?raw";
import commandsRs from "../../src-tauri/src/commands.rs?raw";
import libRs from "../../src-tauri/src/lib.rs?raw";
import mainRs from "../../src-tauri/src/main.rs?raw";
import tauriConf from "../../src-tauri/tauri.conf.json?raw";
import { cn } from "../utils/cn";

const FILES: { path: string; lang: string; body: string; note: string }[] = [
  {
    path: "src-tauri/Cargo.toml",
    lang: "toml",
    body: cargoToml,
    note: "Locked dependency set — tauri 2 (tray-icon), vigem-client, hidapi, gilrs, tokio.",
  },
  {
    path: "src-tauri/src/input/gamepad.rs",
    lang: "rust",
    body: gamepadRs,
    note: "DS4 / DualSense HID parsing, curve maths, lightbar + rumble reports, ViGEm mapping, hot-plug supervisor.",
  },
  {
    path: "src-tauri/src/commands.rs",
    lang: "rust",
    body: commandsRs,
    note: "Every #[tauri::command] with Result<T, String> error handling.",
  },
  {
    path: "src-tauri/src/lib.rs",
    lang: "rust",
    body: libRs,
    note: "Tauri v2 builder, tray icon + menu, battery tooltip, auto-start of the realtime loop.",
  },
  {
    path: "src-tauri/src/main.rs",
    lang: "rust",
    body: mainRs,
    note: "Windows subsystem entry point.",
  },
  {
    path: "src-tauri/tauri.conf.json",
    lang: "json",
    body: tauriConf,
    note: "Window, CSP, tray and NSIS/MSI bundle configuration.",
  },
];

export default function SourceExplorer() {
  const [active, setActive] = useState(1);
  const [copied, setCopied] = useState(false);
  const file = FILES[active];
  const lines = useMemo(() => file.body.split("\n"), [file.body]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(file.body);
      setCopied(true);
      setTimeout(() => setCopied(false), 1400);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="grid gap-4 lg:grid-cols-[260px_minmax(0,1fr)]">
      <div className="space-y-1.5">
        <p className="mb-2 font-mono text-[10px] uppercase tracking-[0.2em] text-slate-500">
          Rust core · {FILES.length} files
        </p>
        {FILES.map((f, i) => (
          <button
            key={f.path}
            onClick={() => setActive(i)}
            className={cn(
              "w-full rounded-lg border px-3 py-2 text-left transition-all",
              i === active
                ? "border-cyan-400/40 bg-cyan-400/[0.08]"
                : "border-white/8 bg-white/[0.02] hover:border-white/16",
            )}
          >
            <div className="flex items-center justify-between gap-2">
              <span className="truncate font-mono text-[11px] text-slate-200">
                {f.path.replace("src-tauri/", "")}
              </span>
              <span className="shrink-0 font-mono text-[9px] uppercase text-slate-500">
                {f.lang}
              </span>
            </div>
            <p className="mt-0.5 font-mono text-[9px] leading-snug text-slate-500">
              {f.body.split("\n").length} lines
            </p>
          </button>
        ))}
      </div>

      <div className="min-w-0 overflow-hidden rounded-2xl border border-white/8 bg-slate-950/70">
        <div className="flex items-center justify-between border-b border-white/8 px-4 py-2.5">
          <div className="min-w-0">
            <p className="truncate font-mono text-[11px] text-slate-200">{file.path}</p>
            <p className="truncate font-mono text-[9.5px] text-slate-500">{file.note}</p>
          </div>
          <button
            onClick={copy}
            className="shrink-0 rounded-md border border-white/10 bg-white/5 px-2.5 py-1 font-mono text-[10px] text-slate-300 hover:border-cyan-400/40 hover:text-cyan-200"
          >
            {copied ? "COPIED ✓" : "COPY"}
          </button>
        </div>
        <div className="max-h-[62vh] overflow-auto">
          <pre className="min-w-full p-4 font-mono text-[11px] leading-[1.55]">
            {lines.map((l, i) => (
              <div key={i} className="flex">
                <span className="mr-4 w-9 shrink-0 select-none text-right text-slate-700">
                  {i + 1}
                </span>
                <code className={cn("whitespace-pre", tint(l))}>{l || " "}</code>
              </div>
            ))}
          </pre>
        </div>
      </div>
    </div>
  );
}

function tint(line: string): string {
  const t = line.trim();
  if (t.startsWith("//") || t.startsWith("#!") || t.startsWith("/*") || t.startsWith("*"))
    return "text-slate-600";
  if (t.startsWith("#[") || t.startsWith("#!["))
    return "text-amber-300/80";
  if (/^(pub |fn |impl |struct |enum |use |mod |let |match |const |static )/.test(t))
    return "text-cyan-200/90";
  if (/^\[/.test(t)) return "text-violet-300/90";
  return "text-slate-300";
}
