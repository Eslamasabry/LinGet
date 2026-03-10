import { Check, Loader2, Moon, RefreshCw, Save, Settings as SettingsIcon, Sun } from 'lucide-react'
import type { SettingsData, SourceInfo } from '../types'

export function SettingsPage({
  sources,
  settings,
  loading,
  saving,
  onSettingsChange,
  onSourceToggle,
  onSave,
}: {
  sources: SourceInfo[]
  settings: SettingsData
  loading: boolean
  saving: boolean
  onSettingsChange: (patch: Partial<SettingsData>) => void
  onSourceToggle: (sourceId: string) => void
  onSave: () => void
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 size={32} className="animate-spin text-blue-500" />
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">Settings</h2>
        <p className="text-gray-500">Configure LinGet to match your workflow.</p>
      </div>

      <div className="bg-[#0A0A0A] rounded-xl border border-[#27272A] p-6">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <SettingsIcon size={20} />
          General
        </h3>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h4 className="font-medium text-white">Theme</h4>
              <p className="text-sm text-gray-500">Choose your preferred appearance</p>
            </div>
            <button
              onClick={() => onSettingsChange({ dark_mode: !settings.dark_mode })}
              className="flex items-center gap-2 px-4 py-2 bg-[#1A1A1A] text-gray-300 rounded-lg hover:bg-[#27272A] transition-colors"
            >
              {settings.dark_mode ? <Moon size={16} /> : <Sun size={16} />}
              {settings.dark_mode ? 'Dark' : 'Light'}
            </button>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <h4 className="font-medium text-white">Auto Refresh</h4>
              <p className="text-sm text-gray-500">Automatically check for package updates</p>
            </div>
            <button
              onClick={() => onSettingsChange({ auto_refresh: !settings.auto_refresh })}
              className={`relative w-12 h-6 rounded-full transition-colors ${
                settings.auto_refresh ? 'bg-blue-600' : 'bg-[#27272A]'
              }`}
            >
              <div
                className={`absolute top-1 w-4 h-4 bg-white rounded-full transition-transform ${
                  settings.auto_refresh ? 'translate-x-7' : 'translate-x-1'
                }`}
              />
            </button>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <h4 className="font-medium text-white">Refresh Interval</h4>
              <p className="text-sm text-gray-500">How often to refresh package data</p>
            </div>
            <select
              value={settings.refresh_interval}
              onChange={(event) => onSettingsChange({ refresh_interval: Number(event.target.value) })}
              className="px-4 py-2 bg-[#1A1A1A] text-white rounded-lg border border-[#27272A] focus:border-blue-500 focus:outline-none"
            >
              <option value={300}>5 minutes</option>
              <option value={900}>15 minutes</option>
              <option value={1800}>30 minutes</option>
              <option value={3600}>1 hour</option>
              <option value={21600}>6 hours</option>
              <option value={43200}>12 hours</option>
            </select>
          </div>
        </div>
      </div>

      <div className="bg-[#0A0A0A] rounded-xl border border-[#27272A] p-6">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <RefreshCw size={20} />
          Package Sources
        </h3>

        <div className="space-y-3">
          {sources.map((source) => (
            <div key={source.id} className="flex items-center justify-between p-4 bg-[#111111] rounded-lg">
              <div className="flex items-center gap-3">
                <span className="text-2xl">{source.icon}</span>
                <div>
                  <h4 className="font-medium text-white flex items-center gap-2">
                    {source.name}
                    {source.available === false && (
                      <span className="px-2 py-0.5 text-xs bg-red-900/30 text-red-400 rounded-full">
                        Unavailable
                      </span>
                    )}
                  </h4>
                  <p className="text-sm text-gray-500">{source.description}</p>
                </div>
              </div>
              <button
                onClick={() => onSourceToggle(source.id)}
                disabled={source.available === false}
                className={`relative w-12 h-6 rounded-full transition-colors ${
                  source.available === false
                    ? 'bg-[#1A1A1A] cursor-not-allowed opacity-60'
                    : source.enabled
                      ? 'bg-blue-600'
                      : 'bg-[#27272A]'
                }`}
              >
                <div
                  className={`absolute top-1 w-4 h-4 bg-white rounded-full transition-transform ${
                    source.enabled ? 'translate-x-7' : 'translate-x-1'
                  }`}
                />
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="flex justify-end">
        <button
          onClick={onSave}
          disabled={saving}
          className="px-6 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-800 text-white rounded-lg transition-colors flex items-center gap-2 font-medium"
        >
          {saving ? <Loader2 size={18} className="animate-spin" /> : <Save size={18} />}
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
      </div>

      <div className="bg-blue-900/20 rounded-lg border border-blue-900/30 p-4">
        <div className="flex items-start gap-3">
          <Check size={20} className="text-blue-400 mt-0.5" />
          <div>
            <h4 className="font-medium text-white mb-1">Settings are persisted locally</h4>
            <p className="text-sm text-gray-400">
              Your preferences and enabled sources are saved by the Tauri backend for future launches.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
