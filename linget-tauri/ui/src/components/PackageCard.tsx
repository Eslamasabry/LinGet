import { motion } from 'framer-motion'
import { Clock, Download, ExternalLink, RotateCcw, Search, Shield, Trash2 } from 'lucide-react'
import type { Package } from '../types'

export function PackageCard({
  pkg,
  viewMode,
  onAction,
}: {
  pkg: Package
  viewMode: 'grid' | 'list'
  onAction: (action: 'install' | 'remove' | 'update', pkg: Package) => void
}) {
  const actionButton =
    pkg.status === 'installed' ? (
      <button
        onClick={() => onAction('remove', pkg)}
        className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors flex items-center gap-2"
      >
        <Trash2 size={16} />
        Remove
      </button>
    ) : pkg.status === 'update_available' ? (
      <button
        onClick={() => onAction('update', pkg)}
        className="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg transition-colors flex items-center gap-2"
      >
        <RotateCcw size={16} />
        Update
      </button>
    ) : (
      <button
        onClick={() => onAction('install', pkg)}
        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors flex items-center gap-2"
      >
        <Download size={16} />
        Install
      </button>
    )

  if (viewMode === 'list') {
    return (
      <motion.div
        initial={{ opacity: 0, x: -20 }}
        animate={{ opacity: 1, x: 0 }}
        className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-4 hover:border-[#3F3F46] transition-all"
      >
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 bg-[#1A1A1A] rounded-lg flex items-center justify-center text-xl">{pkg.source === 'APT' ? '🟢' : pkg.source === 'Flatpak' ? '🟣' : pkg.source === 'Snap' ? '🔵' : pkg.source === 'npm' ? '🟡' : pkg.source === 'pip' ? '🐍' : pkg.source === 'cargo' ? '🦀' : '📦'}</div>

          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-3 mb-1">
              <h3 className="font-semibold text-white truncate">{pkg.name}</h3>
              <span className="px-2 py-0.5 bg-[#1A1A1A] text-xs text-gray-400 rounded">{pkg.source}</span>
              {pkg.status === 'update_available' && (
                <span className="px-2 py-0.5 bg-yellow-900/30 text-xs text-yellow-400 rounded">Update available</span>
              )}
            </div>
            <p className="text-sm text-gray-500 line-clamp-1">{pkg.description || 'No description available'}</p>
          </div>

          <div className="flex items-center gap-3">
            <div className="text-right">
              <div className="text-sm text-white">{pkg.version}</div>
              {pkg.available_version && <div className="text-xs text-gray-500">→ {pkg.available_version}</div>}
            </div>
            {actionButton}
          </div>
        </div>
      </motion.div>
    )
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      whileHover={{ y: -2 }}
      className="bg-[#0A0A0A] rounded-xl border border-[#27272A] p-5 hover:border-[#3F3F46] transition-all group"
    >
      <div className="flex items-start justify-between mb-4">
        <div className="w-12 h-12 bg-[#1A1A1A] rounded-xl flex items-center justify-center text-2xl">
          {pkg.source === 'APT' ? '🟢' : pkg.source === 'Flatpak' ? '🟣' : pkg.source === 'Snap' ? '🔵' : pkg.source === 'npm' ? '🟡' : pkg.source === 'pip' ? '🐍' : pkg.source === 'cargo' ? '🦀' : '📦'}
        </div>
        <span className={`px-2.5 py-1 text-xs rounded-full ${pkg.status === 'installed' ? 'bg-green-900/30 text-green-400' : pkg.status === 'update_available' ? 'bg-yellow-900/30 text-yellow-400' : 'bg-[#1A1A1A] text-gray-400'}`}>
          {pkg.status === 'installed' ? 'Installed' : pkg.status === 'update_available' ? 'Update Available' : 'Available'}
        </span>
      </div>

      <h3 className="text-lg font-semibold text-white mb-1 group-hover:text-blue-400 transition-colors">{pkg.name}</h3>
      <p className="text-sm text-gray-500 mb-4 line-clamp-2 min-h-[2.5rem]">{pkg.description || 'No description available'}</p>

      <div className="space-y-2 mb-4">
        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-500">Version</span>
          <span className="text-white font-mono text-xs">{pkg.version}</span>
        </div>
        {pkg.available_version && (
          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-500">Latest</span>
            <span className="text-green-400 font-mono text-xs">{pkg.available_version}</span>
          </div>
        )}
        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-500">Source</span>
          <span className="text-gray-300">{pkg.source}</span>
        </div>
        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-500">Size</span>
          <span className="text-gray-300">{pkg.size_display}</span>
        </div>
      </div>

      {(pkg.homepage || pkg.license || pkg.maintainer || pkg.dependencies) && (
        <div className="border-t border-[#1A1A1A] pt-3 mb-4 space-y-2">
          {pkg.homepage && (
            <div className="flex items-center gap-2 text-xs">
              <ExternalLink size={12} className="text-gray-500" />
              <a
                href={pkg.homepage}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-400 hover:text-blue-300 truncate"
              >
                {pkg.homepage}
              </a>
            </div>
          )}
          {pkg.license && (
            <div className="flex items-center gap-2 text-xs text-gray-400">
              <Shield size={12} className="text-gray-500" />
              <span>{pkg.license}</span>
            </div>
          )}
          {pkg.maintainer && (
            <div className="flex items-center gap-2 text-xs text-gray-400">
              <Clock size={12} className="text-gray-500" />
              <span>{pkg.maintainer}</span>
            </div>
          )}
          {pkg.dependencies && pkg.dependencies.length > 0 && (
            <div className="flex items-center gap-2 text-xs text-gray-400">
              <Search size={12} className="text-gray-500" />
              <span>{pkg.dependencies.length} dependencies</span>
            </div>
          )}
        </div>
      )}

      {actionButton}
    </motion.div>
  )
}
