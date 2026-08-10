import { useEffect, useRef } from "react";
import type { EngineStats } from "../lib/types";

interface Props {
  getStats: () => EngineStats;
}

/**
 * Real-time performance dashboard showing adaptive polling, batching,
 * AI optimization progress, and system resource usage.
 * Updated via rAF for smooth 60Hz rendering without React re-renders.
 */
export default function PerformanceDashboard({ getStats }: Props) {
  const pollHzRef = useRef<HTMLSpanElement | null>(null);
  const latencyRef = useRef<HTMLSpanElement | null>(null);
  const batchRef = useRef<HTMLSpanElement | null>(null);
  const cpuRef = useRef<HTMLDivElement | null>(null);
  const cpuTxtRef = useRef<HTMLSpanElement | null>(null);
  const aiProgressRef = useRef<HTMLDivElement | null>(null);
  const aiTxtRef = useRef<HTMLSpanElement | null>(null);
  const priorityRef = useRef<HTMLSpanElement | null>(null);
  const reportsBatchedRef = useRef<HTMLSpanElement | null>(null);

  useEffect(() => {
    let raf = 0;
    let lastUpdate = 0;

    const tick = (now: number) => {
      // Throttle updates to 10Hz for readability
      if (now - lastUpdate < 100) {
        raf = requestAnimationFrame(tick);
        return;
      }
      lastUpdate = now;

      const stats = getStats();

      // Update poll Hz display
      if (pollHzRef.current) {
        pollHzRef.current.textContent = `${stats.currentPollHz || stats.pollHz}`;
        pollHzRef.current.style.color = stats.adaptivePollingActive 
          ? 'rgb(34, 211, 238)' // cyan for adaptive
          : 'rgb(148, 163, 184)'; // slate for fixed
      }

      // Update latency
      if (latencyRef.current) {
        const ms = (stats.avgLatencyUs / 1000).toFixed(2);
        latencyRef.current.textContent = `${ms} ms`;
        latencyRef.current.style.color = stats.avgLatencyUs < 500
          ? 'rgb(34, 211, 238)' // green < 0.5ms
          : stats.avgLatencyUs < 1000
          ? 'rgb(240, 180, 0)' // amber < 1ms
          : 'rgb(255, 30, 20)'; // red > 1ms
      }

      // Update batch info
      if (batchRef.current) {
        if (stats.batchReportsActive) {
          batchRef.current.textContent = `${stats.batchSizeAvg.toFixed(1)} avg`;
          batchRef.current.className = "font-mono text-xs tabular-nums text-cyan-300";
        } else {
          batchRef.current.textContent = "disabled";
          batchRef.current.className = "font-mono text-xs tabular-nums text-slate-500";
        }
      }

      // Update reports batched counter
      if (reportsBatchedRef.current) {
        reportsBatchedRef.current.textContent = stats.reportsBatched.toLocaleString();
      }

      // Update CPU usage bar
      if (cpuRef.current && cpuTxtRef.current) {
        const cpuPct = Math.min(stats.cpuUsagePercent, 100);
        cpuRef.current.style.width = `${cpuPct}%`;
        cpuTxtRef.current.textContent = `${cpuPct.toFixed(1)}%`;
        
        if (cpuPct < 30) {
          cpuRef.current.style.background = 'linear-gradient(90deg, rgba(34,211,238,0.5), rgb(34,211,238))';
        } else if (cpuPct < 70) {
          cpuRef.current.style.background = 'linear-gradient(90deg, rgba(240,180,0,0.5), rgb(240,180,0))';
        } else {
          cpuRef.current.style.background = 'linear-gradient(90deg, rgba(255,30,20,0.5), rgb(255,30,20))';
        }
      }

      // Update AI optimization progress
      if (aiProgressRef.current && aiTxtRef.current) {
        if (stats.aiOptimizationActive) {
          const progress = stats.aiSamplesCollected / Math.max(stats.aiSamplesTarget, 1);
          const pct = Math.min(progress * 100, 100);
          
          aiProgressRef.current.style.width = `${pct}%`;
          aiProgressRef.current.style.background = stats.aiAnalysisComplete
            ? 'linear-gradient(90deg, rgba(168,85,247,0.5), rgb(168,85,247))'
            : 'linear-gradient(90deg, rgba(236,72,153,0.5), rgb(236,72,153))';
          
          if (stats.aiAnalysisComplete) {
            aiTxtRef.current.textContent = `Complete (${(stats.aiConfidenceScore * 100).toFixed(0)}% confidence)`;
            aiTxtRef.current.className = "font-mono text-xs text-fuchsia-300";
          } else {
            aiTxtRef.current.textContent = `${stats.aiSamplesCollected}/${stats.aiSamplesTarget} samples`;
            aiTxtRef.current.className = "font-mono text-xs text-pink-300";
          }
        } else {
          aiProgressRef.current.style.width = '0%';
          aiTxtRef.current.textContent = 'AI optimization disabled';
          aiTxtRef.current.className = "font-mono text-xs text-slate-500";
        }
      }

      // Update thread priority badge
      if (priorityRef.current) {
        priorityRef.current.textContent = stats.threadPriority || 'Normal';
        priorityRef.current.style.color = stats.threadPriority === 'Time Critical'
          ? 'rgb(34, 211, 238)'
          : 'rgb(148, 163, 184)';
      }

      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [getStats]);

  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.02] p-4">
      <h3 className="mb-3 text-xs font-semibold uppercase tracking-[0.18em] text-slate-300">
        Performance Dashboard <span className="ml-1 text-slate-600">v1.3.0</span>
      </h3>

      <div className="grid grid-cols-2 gap-3 mb-4">
        {/* Polling Rate */}
        <div className="rounded-lg border border-white/8 bg-white/[0.03] p-3">
          <p className="mb-1 font-mono text-[10px] text-slate-500">POLLING RATE</p>
          <div className="flex items-baseline gap-2">
            <span ref={pollHzRef} className="font-mono text-2xl font-bold text-cyan-300">0</span>
            <span className="text-xs text-slate-400">Hz</span>
          </div>
          <p className="mt-1 font-mono text-[9px] text-slate-600">adaptive: <span ref={priorityRef}>active</span></p>
        </div>

        {/* Latency */}
        <div className="rounded-lg border border-white/8 bg-white/[0.03] p-3">
          <p className="mb-1 font-mono text-[10px] text-slate-500">AVG LATENCY</p>
          <div className="flex items-baseline gap-2">
            <span ref={latencyRef} className="font-mono text-2xl font-bold text-emerald-300">0.00 ms</span>
          </div>
          <p className="mt-1 font-mono text-[9px] text-slate-600">target: &lt;1.0 ms</p>
        </div>

        {/* Batching */}
        <div className="rounded-lg border border-white/8 bg-white/[0.03] p-3">
          <p className="mb-1 font-mono text-[10px] text-slate-500">HID BATCHING</p>
          <div className="flex items-baseline gap-2">
            <span ref={batchRef} className="font-mono text-sm text-slate-300">disabled</span>
          </div>
          <p className="mt-1 font-mono text-[9px] text-slate-600">batched: <span ref={reportsBatchedRef}>0</span></p>
        </div>

        {/* Thread Priority */}
        <div className="rounded-lg border border-white/8 bg-white/[0.03] p-3">
          <p className="mb-1 font-mono text-[10px] text-slate-500">THREAD PRIORITY</p>
          <div className="flex items-baseline gap-2">
            <span ref={priorityRef} className="font-mono text-sm text-cyan-300">Normal</span>
          </div>
          <p className="mt-1 font-mono text-[9px] text-slate-600">real-time optimized</p>
        </div>
      </div>

      {/* CPU Usage Bar */}
      <div className="mb-4">
        <div className="mb-1 flex items-baseline justify-between font-mono text-[10px]">
          <span className="text-slate-400">CPU USAGE</span>
          <span ref={cpuTxtRef} className="tabular-nums text-slate-200">0.0%</span>
        </div>
        <div className="h-3 w-full overflow-hidden rounded-full bg-white/8">
          <div
            ref={cpuRef}
            className="h-full rounded-full transition-all duration-200"
            style={{
              width: '0%',
              background: 'linear-gradient(90deg, rgba(34,211,238,0.5), rgb(34,211,238))',
              boxShadow: '0 0 12px rgba(34,211,238,0.6)',
            }}
          />
        </div>
      </div>

      {/* AI Optimization Progress */}
      <div>
        <div className="mb-1 flex items-baseline justify-between font-mono text-[10px]">
          <span className="text-slate-400">AI CURVE OPTIMIZATION</span>
          <span ref={aiTxtRef} className="tabular-nums text-slate-500">disabled</span>
        </div>
        <div className="h-3 w-full overflow-hidden rounded-full bg-white/8">
          <div
            ref={aiProgressRef}
            className="h-full rounded-full transition-all duration-300"
            style={{
              width: '0%',
              background: 'linear-gradient(90deg, rgba(168,85,247,0.5), rgb(168,85,247))',
              boxShadow: '0 0 12px rgba(168,85,247,0.6)',
            }}
          />
        </div>
        <p className="mt-1 font-mono text-[9px] text-slate-600">
          collects input/output pairs during gameplay
        </p>
      </div>
    </div>
  );
}
