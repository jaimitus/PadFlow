import { useEffect, useRef, useState } from "react";
import type { StickProfileConfig } from "../lib/types";

interface SampleBuffer {
  inputs: number[];
  outputs: number[];
  timestamps: number[];
}

interface AISuggestion {
  curveType: "linear" | "exponential" | "sCurve" | "aggressive";
  confidence: number;
  reason: string;
}

export default function AICurveAnalyzer({
  profile,
  onUpdateProfile,
}: {
  profile: StickProfileConfig;
  onUpdateProfile: (updates: Partial<StickProfileConfig>) => void;
}) {
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [samplesCollected, setSamplesCollected] = useState(0);
  const [suggestion, setSuggestion] = useState<AISuggestion | null>(null);
  const bufferRef = useRef<SampleBuffer>({
    inputs: [],
    outputs: [],
    timestamps: [],
  });
  const maxSamples = 500;

  useEffect(() => {
    if (!profile.aiCurveOptimization) {
      setSuggestion(null);
      setSamplesCollected(0);
      bufferRef.current = { inputs: [], outputs: [], timestamps: [] };
      return;
    }

    const collectSamples = () => {
      // In a real implementation, this would receive data from the engine
      // For now, we simulate collection status
      setSamplesCollected((prev) => {
        if (prev >= maxSamples) {
          analyzeAndSuggest();
          return prev;
        }
        return prev + 1;
      });
    };

    const interval = setInterval(collectSamples, 100);
    return () => clearInterval(interval);
  }, [profile.aiCurveOptimization]);

  const analyzeAndSuggest = () => {
    setIsAnalyzing(true);
    
    // Simulated AI analysis - in production this would use actual input patterns
    setTimeout(() => {
      const suggestions: AISuggestion[] = [
        {
          curveType: "exponential",
          confidence: 0.87,
          reason: "Detected precise micro-adjustments in center zone. Exponential curve improves aim stability.",
        },
        {
          curveType: "sCurve",
          confidence: 0.72,
          reason: "Balanced gameplay with both precision and flick shots. S-Curve offers best compromise.",
        },
      ];
      
      const best = suggestions[0];
      setSuggestion(best);
      setIsAnalyzing(false);
    }, 1500);
  };

  const applySuggestion = () => {
    if (!suggestion) return;
    
    onUpdateProfile({
      right: {
        ...profile.right,
        curve: suggestion.curveType,
        curvePower: suggestion.curveType === "exponential" ? 1.9 : 2.0,
      },
      left: {
        ...profile.left,
        curve: suggestion.curveType,
        curvePower: suggestion.curveType === "exponential" ? 1.7 : 1.8,
      },
    });
    
    setSuggestion(null);
    setSamplesCollected(0);
  };

  if (!profile.aiCurveOptimization) {
    return null;
  }

  return (
    <div className="mt-3 rounded-xl border border-violet-500/30 bg-violet-500/10 p-3">
      <div className="mb-2 flex items-center gap-2">
        <span className="text-[10px] font-bold uppercase tracking-wider text-violet-300">
          AI Curve Optimization
        </span>
        {isAnalyzing && (
          <span className="animate-pulse text-[9px] text-violet-400">Analyzing...</span>
        )}
      </div>

      {!suggestion && samplesCollected < maxSamples && (
        <div className="space-y-1">
          <div className="flex justify-between text-[9px] text-slate-400">
            <span>Collecting gameplay samples</span>
            <span>{Math.min(samplesCollected, maxSamples)} / {maxSamples}</span>
          </div>
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
            <div
              className="h-full bg-gradient-to-r from-violet-500 to-fuchsia-500 transition-all duration-300"
              style={{ width: `${(samplesCollected / maxSamples) * 100}%` }}
            />
          </div>
          <p className="text-[8px] text-slate-500">
            Play normally for {Math.max(0, Math.ceil((maxSamples - samplesCollected) / 10))}s more
          </p>
        </div>
      )}

      {suggestion && (
        <div className="space-y-2">
          <div className="flex items-start gap-2">
            <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-violet-500/20 text-[10px] text-violet-300">
              ✨
            </div>
            <div>
              <p className="text-[10px] font-medium text-slate-200">
                Recommended: <span className="text-violet-300">{suggestion.curveType}</span> curve
              </p>
              <p className="text-[8px] text-slate-400">{suggestion.reason}</p>
              <p className="mt-0.5 text-[8px] text-violet-400">
                Confidence: {(suggestion.confidence * 100).toFixed(0)}%
              </p>
            </div>
          </div>
          
          <div className="flex gap-2 pt-1">
            <button
              onClick={applySuggestion}
              className="flex-1 rounded-md bg-violet-500 px-2 py-1 text-[9px] font-medium text-white hover:bg-violet-600 transition-colors"
            >
              Apply Recommendation
            </button>
            <button
              onClick={() => {
                setSuggestion(null);
                setSamplesCollected(0);
              }}
              className="rounded-md bg-white/10 px-2 py-1 text-[9px] text-slate-300 hover:bg-white/20 transition-colors"
            >
              Dismiss
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
