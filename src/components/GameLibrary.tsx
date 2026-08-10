import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GameProfile, DetectedGame, StickProfileConfig } from '../types';
import { Card } from './ui/Card';
import { Button } from './ui/Button';
import { Badge } from './ui/Badge';

interface GameLibraryProps {
  onProfileApplied?: (game: GameProfile) => void;
}

export function GameLibrary({ onProfileApplied }: GameLibraryProps) {
  const [profiles, setProfiles] = useState<GameProfile[]>([]);
  const [detectedGames, setDetectedGames] = useState<DetectedGame[]>([]);
  const [autoSwitchEnabled, setAutoSwitchEnabled] = useState(true);
  const [loading, setLoading] = useState(false);
  const [selectedGame, setSelectedGame] = useState<GameProfile | null>(null);
  const [showAddModal, setShowAddModal] = useState(false);

  // Load game profiles on mount
  useEffect(() => {
    loadProfiles();
    checkAutoSwitch();
    
    // Set up polling for detected games
    const interval = setInterval(async () => {
      try {
        const detected: DetectedGame[] = await invoke('scan_for_games');
        setDetectedGames(detected);
      } catch (error) {
        console.error('Failed to scan for games:', error);
      }
    }, 2000); // Poll every 2 seconds

    return () => clearInterval(interval);
  }, []);

  async function loadProfiles() {
    try {
      setLoading(true);
      const result: GameProfile[] = await invoke('get_game_profiles');
      setProfiles(result);
    } catch (error) {
      console.error('Failed to load game profiles:', error);
    } finally {
      setLoading(false);
    }
  }

  async function checkAutoSwitch() {
    try {
      const enabled: bool = await invoke('is_auto_switch_enabled');
      setAutoSwitchEnabled(enabled);
    } catch (error) {
      console.error('Failed to check auto-switch status:', error);
    }
  }

  async function toggleAutoSwitch() {
    try {
      await invoke('set_auto_switch_enabled', { enabled: !autoSwitchEnabled });
      setAutoSwitchEnabled(!autoSwitchEnabled);
    } catch (error) {
      console.error('Failed to toggle auto-switch:', error);
    }
  }

  async function handleRemoveProfile(executableName: string) {
    if (!confirm(`Remove profile for ${executableName}?`)) return;
    
    try {
      await invoke('remove_game_profile', { executableName });
      await loadProfiles();
    } catch (error) {
      console.error('Failed to remove profile:', error);
    }
  }

  async function handleApplyProfile(game: GameProfile) {
    try {
      await invoke('update_stick_profile', {
        profileData: game.recommendedProfile,
        padId: null,
      });
      onProfileApplied?.(game);
    } catch (error) {
      console.error('Failed to apply profile:', error);
    }
  }

  const currentlyRunningGames = detectedGames.filter(g => g.profileApplied);

  return (
    <div className="space-y-6">
      {/* Header with Auto-Switch Toggle */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white">Game Library</h2>
          <p className="text-sm text-gray-400">
            Manage game-specific profiles and automatic detection
          </p>
        </div>
        
        <div className="flex items-center gap-3">
          <Badge variant={autoSwitchEnabled ? 'success' : 'secondary'}>
            {autoSwitchEnabled ? 'Auto-Switch ON' : 'Auto-Switch OFF'}
          </Badge>
          
          <Button
            variant={autoSwitchEnabled ? 'outline' : 'primary'}
            size="sm"
            onClick={toggleAutoSwitch}
          >
            {autoSwitchEnabled ? 'Disable' : 'Enable'} Auto-Detect
          </Button>
          
          <Button variant="primary" size="sm" onClick={() => setShowAddModal(true)}>
            + Add Game
          </Button>
        </div>
      </div>

      {/* Currently Running Games */}
      {currentlyRunningGames.length > 0 && (
        <Card className="p-4 bg-gradient-to-r from-emerald-900/30 to-teal-900/30 border-emerald-500/30">
          <h3 className="text-lg font-semibold text-emerald-400 mb-3">
            🎮 Currently Running
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {currentlyRunningGames.map((game) => (
              <div
                key={game.processId}
                className="bg-emerald-950/50 rounded-lg p-3 border border-emerald-500/20"
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="font-medium text-white">{game.gameTitle}</span>
                  <Badge variant="success">Running</Badge>
                </div>
                <p className="text-xs text-gray-400 font-mono">{game.executableName}</p>
                <p className="text-xs text-gray-500 mt-1">
                  PID: {game.processId}
                </p>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Game Profiles List */}
      <Card className="p-4">
        <h3 className="text-lg font-semibold text-white mb-4">
          Saved Profiles ({profiles.length})
        </h3>
        
        {loading ? (
          <div className="text-center py-8 text-gray-400">Loading...</div>
        ) : profiles.length === 0 ? (
          <div className="text-center py-8 text-gray-400">
            No game profiles yet. Click "Add Game" to create one.
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {profiles.map((profile) => (
              <div
                key={profile.gameId}
                className={`rounded-lg p-4 border transition-all cursor-pointer ${
                  selectedGame?.gameId === profile.gameId
                    ? 'bg-violet-900/30 border-violet-500/50'
                    : 'bg-gray-800/50 border-gray-700/50 hover:border-gray-600'
                }`}
                onClick={() => setSelectedGame(profile)}
              >
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <h4 className="font-semibold text-white">{profile.gameTitle}</h4>
                    <p className="text-xs text-gray-400 font-mono mt-1">
                      {profile.executableName}
                    </p>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemoveProfile(profile.executableName);
                    }}
                    className="text-gray-500 hover:text-red-400 transition-colors"
                  >
                    ✕
                  </button>
                </div>

                <div className="space-y-2 text-xs">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Polling:</span>
                    <span className="text-white font-mono">
                      {profile.recommendedProfile.adaptivePolling ? 'Adaptive' : 'Fixed'}{' '}
                      {profile.recommendedProfile.targetPollHz} Hz
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Batching:</span>
                    <span className={profile.recommendedProfile.batchReports ? 'text-emerald-400' : 'text-gray-400'}>
                      {profile.recommendedProfile.batchReports ? 'ON' : 'OFF'}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">AI Optimization:</span>
                    <span className={profile.aiCurveOptimization ? 'text-violet-400' : 'text-gray-400'}>
                      {profile.aiCurveOptimization ? 'Enabled' : 'Disabled'}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Battery Saver:</span>
                    <span className={profile.batterySaverRecommended ? 'text-amber-400' : 'text-gray-400'}>
                      {profile.batterySaverRecommended ? `@${profile.batteryThreshold}%` : 'Not recommended'}
                    </span>
                  </div>
                </div>

                {profile.lastPlayed && (
                  <div className="mt-3 pt-3 border-t border-gray-700/50">
                    <p className="text-xs text-gray-500">
                      Last played: {new Date(profile.lastPlayed * 1000).toLocaleDateString()}
                    </p>
                    <p className="text-xs text-gray-500">
                      Play time: {Math.floor(profile.playTimeSeconds / 3600)}h {Math.floor((profile.playTimeSeconds % 3600) / 60)}m
                    </p>
                  </div>
                )}

                <Button
                  variant="primary"
                  size="sm"
                  className="w-full mt-3"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleApplyProfile(profile);
                  }}
                >
                  Apply Profile
                </Button>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Add Game Modal */}
      {showAddModal && (
        <AddGameModal
          onClose={() => setShowAddModal(false)}
          onAdd={async (profile) => {
            try {
              await invoke('add_game_profile', { profile });
              await loadProfiles();
              setShowAddModal(false);
            } catch (error) {
              console.error('Failed to add game profile:', error);
            }
          }}
        />
      )}
    </div>
  );
}

// Add Game Modal Component
interface AddGameModalProps {
  onClose: () => void;
  onAdd: (profile: GameProfile) => Promise<void>;
}

function AddGameModal({ onClose, onAdd }: AddGameModalProps) {
  const [formData, setFormData] = useState<Partial<GameProfile>>({
    gameId: '',
    executableName: '',
    gameTitle: '',
    aiCurveOptimization: true,
    batterySaverRecommended: false,
    batteryThreshold: 30,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    const profile: GameProfile = {
      gameId: formData.gameId || formData.executableName.replace('.exe', ''),
      executableName: formData.executableName!,
      gameTitle: formData.gameTitle || formData.executableName,
      recommendedProfile: {
        adaptivePolling: true,
        targetPollHz: 1000,
        batchReports: false,
        batterySaver: false,
        aiCurveOptimization: formData.aiCurveOptimization ?? true,
      } as StickProfileConfig,
      aiCurveOptimization: formData.aiCurveOptimization ?? true,
      batterySaverRecommended: formData.batterySaverRecommended ?? false,
      batteryThreshold: formData.batteryThreshold ?? 30,
      iconPath: null,
      lastPlayed: null,
      playTimeSeconds: 0,
    };

    await onAdd(profile);
  };

  return (
    <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
      <div className="bg-gray-900 rounded-lg p-6 w-full max-w-md border border-gray-700">
        <h3 className="text-xl font-bold text-white mb-4">Add Game Profile</h3>
        
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm text-gray-400 mb-1">Executable Name</label>
            <input
              type="text"
              required
              placeholder="e.g., apex_legends.exe"
              className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white"
              value={formData.executableName}
              onChange={(e) => setFormData({ ...formData, executableName: e.target.value })}
            />
          </div>
          
          <div>
            <label className="block text-sm text-gray-400 mb-1">Game Title</label>
            <input
              type="text"
              placeholder="e.g., Apex Legends"
              className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white"
              value={formData.gameTitle}
              onChange={(e) => setFormData({ ...formData, gameTitle: e.target.value })}
            />
          </div>
          
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="ai-opt"
              checked={formData.aiCurveOptimization}
              onChange={(e) => setFormData({ ...formData, aiCurveOptimization: e.target.checked })}
              className="rounded"
            />
            <label htmlFor="ai-opt" className="text-sm text-gray-300">
              Enable AI Curve Optimization
            </label>
          </div>
          
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="battery-saver"
              checked={formData.batterySaverRecommended}
              onChange={(e) => setFormData({ ...formData, batterySaverRecommended: e.target.checked })}
              className="rounded"
            />
            <label htmlFor="battery-saver" className="text-sm text-gray-300">
              Recommend Battery Saver
            </label>
          </div>
          
          {formData.batterySaverRecommended && (
            <div>
              <label className="block text-sm text-gray-400 mb-1">
                Battery Threshold (%)
              </label>
              <input
                type="number"
                min="10"
                max="50"
                value={formData.batteryThreshold}
                onChange={(e) => setFormData({ ...formData, batteryThreshold: parseInt(e.target.value) })}
                className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white"
              />
            </div>
          )}
          
          <div className="flex gap-3 pt-4">
            <Button type="button" variant="secondary" onClick={onClose} className="flex-1">
              Cancel
            </Button>
            <Button type="submit" variant="primary" className="flex-1">
              Add Game
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
