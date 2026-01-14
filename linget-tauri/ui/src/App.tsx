import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { BrowserRouter, Routes, Route, useLocation } from 'react-router-dom'
import { motion, AnimatePresence } from 'framer-motion'
import {
  Package, RefreshCw, Search, Settings, List,
  Trash2, Download, X, Check, AlertCircle,
  Terminal, Sun, Moon, Zap, Globe, Layers,
  BookOpen, ToggleLeft, ToggleRight, Sparkles,
  AlertTriangle, Keyboard, RotateCcw,
  CheckCircle, Clock, Shield, User, ExternalLink, ChevronRight
} from 'lucide-react'

interface Package {
  name: string
  version: string
  available_version?: string
  description: string
  source: string
  status: 'installed' | 'update_available' | 'not_installed' | 'installing' | 'removing' | 'updating'
  size?: number
  size_display: string
  homepage?: string
  license?: string
  maintainer?: string
  dependencies?: string[]
}

interface Toast {
  id: number
  type: 'success' | 'error' | 'info' | 'warning'
  message: string
}

interface SourceInfo {
  id: string
  name: string
  icon: string
  enabled: boolean
  count?: number
}

const SOURCES: SourceInfo[] = [
  { id: 'APT', name: 'APT', icon: '🟢', enabled: true },
  { id: 'Flatpak', name: 'Flatpak', icon: '🟣', enabled: true },
  { id: 'Snap', name: 'Snap', icon: '🔵', enabled: true },
  { id: 'npm', name: 'npm', icon: '🟡', enabled: true },
  { id: 'pip', name: 'pip', icon: '🐍', enabled: true },
  { id: 'pipx', name: 'pipx', icon: '🐍', enabled: true },
  { id: 'cargo', name: 'Cargo', icon: '🦀', enabled: true },
  { id: 'brew', name: 'Homebrew', icon: '🍺', enabled: true },
  { id: 'dnf', name: 'DNF', icon: '🔴', enabled: true },
  { id: 'pacman', name: 'Pacman', icon: '📦', enabled: true },
  { id: 'zypper', name: 'Zypper', icon: '🟠', enabled: true },
]

const SHORTCUTS = [
  { key: 'r', action: 'Refresh' },
  { key: '/', action: 'Search' },
  { key: 'i', action: 'Installed' },
  { key: 'u', action: 'Updates' },
  { key: 'b', action: 'Browse' },
  { key: 's', action: 'Settings' },
  { key: '?', action: 'Shortcuts' },
]

function PageTransition({ children }: { children: React.ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -10 }}
      transition={{ duration: 0.2 }}
    >
      {children}
    </motion.div>
  )
}

// Empty state component with illustration
function EmptyState({ 
  icon: Icon, 
  title, 
  description, 
  action 
}: { 
  icon: React.ElementType
  title: string
  description: string
  action?: React.ReactNode
}) {
  return (
    <motion.div 
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      className="flex flex-col items-center justify-center h-64"
    >
      <div className="p-4 bg-[#0A0A0A] rounded-full mb-4">
        <Icon size={48} className="text-gray-600" />
      </div>
      <h3 className="text-lg font-medium text-white mb-1">{title}</h3>
      <p className="text-sm text-gray-500 text-center max-w-sm">{description}</p>
      {action && <div className="mt-4">{action}</div>}
    </motion.div>
  )
}

// Skeleton loader
function SkeletonCard() {
  return (
    <div className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-4 animate-pulse">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="h-4 bg-[#1A1A1A] rounded w-1/2 mb-2"></div>
          <div className="h-3 bg-[#1A1A1A] rounded w-1/3 mb-3"></div>
          <div className="h-3 bg-[#1A1A1A] rounded w-3/4"></div>
        </div>
      </div>
    </div>
  )
}

function SkeletonList() {
  return (
    <div className="space-y-3">
      {[...Array(5)].map((_, i) => (
        <div key={i} className="flex items-center gap-3 p-3 bg-[#0A0A0A] rounded-lg animate-pulse">
          <div className="w-10 h-10 bg-[#1A1A1A] rounded-lg"></div>
          <div className="flex-1">
            <div className="h-4 bg-[#1A1A1A] rounded w-1/3 mb-2"></div>
            <div className="h-3 bg-[#1A1A1A] rounded w-1/4"></div>
          </div>
        </div>
      ))}
    </div>
  )
}

// Package card with hover effects
function PackageCard({ pkg, onAction }: { pkg: Package; onAction: (action: string, pkg: Package) => void }) {
  const [showDetails, setShowDetails] = useState(false)
  
  return (
    <>
      <motion.div 
        whileHover={{ scale: 1.01, borderColor: '#3F3F46' }}
        whileTap={{ scale: 0.99 }}
        className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-4 transition-all cursor-pointer group"
        onClick={() => setShowDetails(true)}
      >
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <h3 className="font-semibold text-white truncate group-hover:text-blue-400 transition-colors">{pkg.name}</h3>
              {pkg.status === 'update_available' && (
                <span className="px-1.5 py-0.5 text-xs bg-blue-600/20 text-blue-400 rounded flex items-center gap-1">
                  <Sparkles size={10} />
                  Update
                </span>
              )}
            </div>
            <p className="text-sm text-gray-400 mt-1 flex items-center gap-2">
              {pkg.status === 'update_available' ? (
                <>
                  <span className="text-gray-500">{pkg.version}</span>
                  <ChevronRight size={12} className="text-gray-600" />
                  <span className="text-yellow-400">{pkg.available_version}</span>
                </>
              ) : (
                pkg.version
              )}
            </p>
            <p className="text-sm text-gray-500 mt-2 line-clamp-2">{pkg.description || 'No description'}</p>
          </div>
          <span className={`px-2 py-1 text-xs rounded-full ml-3 ${
            pkg.status === 'installed' ? 'bg-green-900/30 text-green-400 border border-green-800' :
            pkg.status === 'update_available' ? 'bg-yellow-900/30 text-yellow-400 border border-yellow-800' :
            pkg.status === 'installing' ? 'bg-blue-900/30 text-blue-400 border border-blue-800' :
            pkg.status === 'removing' ? 'bg-red-900/30 text-red-400 border border-red-800' :
            pkg.status === 'updating' ? 'bg-blue-900/30 text-blue-400 border border-blue-800' :
            'bg-gray-800 text-gray-400 border border-gray-700'
          }`}>
            {pkg.status === 'installed' && <Check size={10} className="inline mr-1" />}
            {pkg.status === 'update_available' && pkg.available_version}
            {pkg.status === 'installing' && 'Installing...'}
            {pkg.status === 'removing' && 'Removing...'}
            {pkg.status === 'updating' && 'Updating...'}
            {pkg.status === 'not_installed' && 'Available'}
          </span>
        </div>
        
        <div className="flex items-center justify-between mt-3 pt-3 border-t border-[#27272A]">
          <div className="flex items-center gap-3">
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <Package size={12} />
              {pkg.source}
            </span>
            {pkg.size && (
              <span className="text-xs text-gray-600">{pkg.size_display}</span>
            )}
          </div>
          <div className="flex items-center gap-1">
            {pkg.status === 'installed' && (
              <>
                <button 
                  onClick={(e) => { e.stopPropagation(); onAction('update', pkg) }}
                  className="p-1.5 text-gray-400 hover:text-yellow-400 hover:bg-[#1A1A1A] rounded transition-colors"
                  title="Update (U)"
                >
                  <RotateCcw size={14} />
                </button>
                <button 
                  onClick={(e) => { e.stopPropagation(); onAction('remove', pkg) }}
                  className="p-1.5 text-gray-400 hover:text-red-400 hover:bg-[#1A1A1A] rounded transition-colors"
                  title="Remove"
                >
                  <Trash2 size={14} />
                </button>
              </>
            )}
            {pkg.status === 'update_available' && (
              <button 
                onClick={(e) => { e.stopPropagation(); onAction('update', pkg) }}
                className="px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors flex items-center gap-1"
              >
                <Download size={12} />
                Update
              </button>
            )}
            {pkg.status === 'not_installed' && (
              <button 
                onClick={(e) => { e.stopPropagation(); onAction('install', pkg) }}
                className="px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors flex items-center gap-1"
              >
                <Download size={12} />
                Install
              </button>
            )}
          </div>
        </div>
      </motion.div>

      {/* Package Details Modal */}
      <AnimatePresence>
        {showDetails && (
          <motion.div 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
            onClick={() => setShowDetails(false)}
          >
            <motion.div 
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              className="bg-[#0A0A0A] rounded-xl border border-[#27272A] max-w-lg w-full max-h-[85vh] overflow-hidden shadow-2xl"
              onClick={e => e.stopPropagation()}
            >
              <div className="p-6 border-b border-[#27272A]">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <h2 className="text-xl font-bold text-white">{pkg.name}</h2>
                      <span className="px-2 py-0.5 text-xs bg-[#1A1A1A] text-gray-400 rounded border border-[#27272A]">
                        {pkg.source}
                      </span>
                    </div>
                    <p className="text-sm text-gray-500 mt-1 flex items-center gap-2">
                      <Clock size={14} />
                      {pkg.status === 'update_available' ? `${pkg.version} → ${pkg.available_version}` : pkg.version}
                    </p>
                  </div>
                  <button 
                    onClick={() => setShowDetails(false)}
                    className="p-2 text-gray-400 hover:text-white hover:bg-[#1A1A1A] rounded-lg transition-colors"
                  >
                    <X size={20} />
                  </button>
                </div>
              </div>
              
              <div className="p-6 space-y-4 overflow-y-auto max-h-96">
                <div>
                  <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-2 flex items-center gap-1">
                    <BookOpen size={12} />
                    Description
                  </h3>
                  <p className="text-gray-300">{pkg.description || 'No description available'}</p>
                </div>
                
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-1">Status</h3>
                    <span className={`inline-flex items-center gap-1 px-2 py-1 text-xs rounded-full ${
                      pkg.status === 'installed' ? 'bg-green-900/30 text-green-400' :
                      pkg.status === 'update_available' ? 'bg-yellow-900/30 text-yellow-400' :
                      'bg-gray-800 text-gray-400'
                    }`}>
                      {pkg.status === 'installed' && <Check size={12} />}
                      {pkg.status === 'installed' ? 'Installed' :
                       pkg.status === 'update_available' ? 'Update Available' :
                       'Not Installed'}
                    </span>
                  </div>
                  
                  {pkg.size && (
                    <div>
                      <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-1">Size</h3>
                      <p className="text-gray-300">{pkg.size_display}</p>
                    </div>
                  )}
                </div>
                
                {pkg.homepage && (
                  <div>
                    <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-1 flex items-center gap-1">
                      <ExternalLink size={12} />
                      Homepage
                    </h3>
                    <a 
                      href={pkg.homepage} 
                      target="_blank" 
                      rel="noopener noreferrer"
                      className="text-blue-400 hover:underline flex items-center gap-1 text-sm"
                    >
                      {pkg.homepage}
                    </a>
                  </div>
                )}
                
                {pkg.license && (
                  <div>
                    <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-1 flex items-center gap-1">
                      <Shield size={12} />
                      License
                    </h3>
                    <p className="text-gray-300">{pkg.license}</p>
                  </div>
                )}
                
                {pkg.maintainer && (
                  <div>
                    <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-1 flex items-center gap-1">
                      <User size={12} />
                      Maintainer
                    </h3>
                    <p className="text-gray-300">{pkg.maintainer}</p>
                  </div>
                )}
                
                {pkg.dependencies && pkg.dependencies.length > 0 && (
                  <div>
                    <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-2 flex items-center gap-1">
                      <Layers size={12} />
                      Dependencies ({pkg.dependencies.length})
                    </h3>
                    <div className="flex flex-wrap gap-1">
                      {pkg.dependencies.slice(0, 15).map((dep, i) => (
                        <span key={i} className="px-2 py-1 text-xs bg-[#1A1A1A] text-gray-400 rounded border border-[#27272A]">
                          {dep}
                        </span>
                      ))}
                      {pkg.dependencies.length > 15 && (
                        <span className="px-2 py-1 text-xs bg-[#1A1A1A] text-gray-500 rounded border border-[#27272A]">
                          +{pkg.dependencies.length - 15} more
                        </span>
                      )}
                    </div>
                  </div>
                )}
              </div>
              
              <div className="p-6 border-t border-[#27272A] flex gap-3">
                {pkg.status === 'installed' && (
                  <>
                    <button 
                      onClick={() => { onAction('update', pkg); setShowDetails(false); }}
                      className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
                    >
                      <RotateCcw size={16} />
                      Update
                    </button>
                    <button 
                      onClick={() => { onAction('remove', pkg); setShowDetails(false); }}
                      className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-[#1A1A1A] text-red-400 border border-[#27272A] rounded-lg hover:bg-[#27272A] transition-colors font-medium"
                    >
                      <Trash2 size={16} />
                      Remove
                    </button>
                  </>
                )}
                {pkg.status === 'update_available' && (
                  <button 
                    onClick={() => { onAction('update', pkg); setShowDetails(false); }}
                    className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
                  >
                    <Download size={16} />
                    Update to {pkg.available_version}
                  </button>
                )}
                {pkg.status === 'not_installed' && (
                  <button 
                    onClick={() => { onAction('install', pkg); setShowDetails(false); }}
                    className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
                  >
                    <Download size={16} />
                    Install
                  </button>
                )}
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  )
}

// Toast notifications
function ToastContainer({ toasts, onDismiss }: { toasts: Toast[]; onDismiss: (id: number) => void }) {
  return (
    <div className="fixed bottom-4 right-4 z-50 space-y-2">
      <AnimatePresence>
        {toasts.map(toast => (
          <motion.div
            key={toast.id}
            initial={{ opacity: 0, x: 50 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 50 }}
            className={`px-4 py-3 rounded-lg border flex items-center gap-3 min-w-[320px] shadow-lg ${
              toast.type === 'success' ? 'bg-[#022c22] border-green-800 text-green-400' :
              toast.type === 'error' ? 'bg-[#450a0a] border-red-800 text-red-400' :
              toast.type === 'warning' ? 'bg-[#451a03] border-yellow-800 text-yellow-400' :
              'bg-[#0c4a6e] border-blue-800 text-blue-400'
            }`}
          >
            {toast.type === 'success' ? <Check size={18} /> :
             toast.type === 'error' ? <AlertCircle size={18} /> :
             toast.type === 'warning' ? <AlertTriangle size={18} /> :
             <Terminal size={18} />}
            <span className="flex-1 text-sm">{toast.message}</span>
            <button 
              onClick={() => onDismiss(toast.id)} 
              className="text-gray-400 hover:text-white transition-colors"
            >
              <X size={14} />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  )
}

// Keyboard shortcuts modal
function ShortcutsModal({ onClose }: { onClose: () => void }) {
  return (
    <motion.div 
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
      onClick={onClose}
    >
      <motion.div 
        initial={{ scale: 0.95 }}
        animate={{ scale: 1 }}
        exit={{ scale: 0.95 }}
        className="bg-[#0A0A0A] rounded-xl border border-[#27272A] max-w-sm w-full p-6"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Keyboard size={20} />
            Keyboard Shortcuts
          </h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white">
            <X size={20} />
          </button>
        </div>
        <div className="space-y-2">
          {SHORTCUTS.map(shortcut => (
            <div key={shortcut.key} className="flex items-center justify-between py-2">
              <span className="text-gray-300">{shortcut.action}</span>
              <kbd className="px-2 py-1 text-xs bg-[#1A1A1A] text-gray-400 rounded border border-[#27272A]">
                {shortcut.key}
              </kbd>
            </div>
          ))}
        </div>
      </motion.div>
    </motion.div>
  )
}

// Installed page
function InstalledPage({ packages, loading, onAction, loadPackages }: { 
  packages: Package[]; 
  loading: boolean; 
  onAction: (action: string, pkg: Package) => void;
  loadPackages: () => void;
}) {
  const [selectedSource, setSelectedSource] = useState('all')
  
  const filtered = selectedSource === 'all' 
    ? packages 
    : packages.filter(p => p.source === selectedSource)
  
  // Get source counts
  const sourceCounts = SOURCES.reduce((acc, s) => {
    acc[s.id] = packages.filter(p => p.source === s.id).length
    return acc
  }, {} as Record<string, number>)
  
  const installedCount = packages.filter(p => p.status === 'installed').length
  
  if (loading) {
    return (
      <PageTransition>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {[...Array(6)].map((_, i) => <SkeletonCard key={i} />)}
        </div>
      </PageTransition>
    )
  }
  
  if (filtered.length === 0) {
    return (
      <PageTransition>
        <EmptyState
          icon={Package}
          title="No packages found"
          description="Click Refresh to load your installed packages"
          action={
            <button
              onClick={loadPackages}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center gap-2"
            >
              <RefreshCw size={16} />
              Refresh
            </button>
          }
        />
      </PageTransition>
    )
  }
  
  return (
    <PageTransition>
      <div>
        {/* Source filter pills */}
        <div className="flex flex-wrap gap-2 mb-6">
          <button
            onClick={() => setSelectedSource('all')}
            className={`px-3 py-1.5 text-sm rounded-lg transition-all ${
              selectedSource === 'all'
                ? 'bg-blue-600 text-white'
                : 'bg-[#1A1A1A] text-gray-400 hover:bg-[#27272A]'
            }`}
          >
            All ({installedCount})
          </button>
          {SOURCES.filter(s => s.enabled && sourceCounts[s.id] > 0).map(source => (
            <button
              key={source.id}
              onClick={() => setSelectedSource(source.id)}
              className={`px-3 py-1.5 text-sm rounded-lg transition-all ${
                selectedSource === source.id
                  ? 'bg-blue-600 text-white'
                  : 'bg-[#1A1A1A] text-gray-400 hover:bg-[#27272A]'
              }`}
            >
              {source.icon} {source.name} ({sourceCounts[source.id]})
            </button>
          ))}
        </div>
        
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filtered.map((pkg) => (
            <PackageCard 
              key={`${pkg.source}-${pkg.name}`} 
              pkg={pkg} 
              onAction={onAction} 
            />
          ))}
        </div>
      </div>
    </PageTransition>
  )
}

// Updates page
function UpdatesPage({ packages, loading, onAction, onUpdateAll }: { 
  packages: Package[]; 
  loading: boolean; 
  onAction: (action: string, pkg: Package) => void;
  onUpdateAll: () => void;
}) {
  const updates = packages.filter(p => p.status === 'update_available')
  
  if (loading) {
    return (
      <PageTransition>
        <SkeletonList />
      </PageTransition>
    )
  }
  
  return (
    <PageTransition>
      <div>
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-xl font-semibold text-white flex items-center gap-2">
              <Sparkles size={20} className="text-yellow-500" />
              Available Updates
            </h2>
            <p className="text-sm text-gray-500 mt-1">
              {updates.length > 0 ? `${updates.length} packages can be updated` : 'Your system is up to date!'}
            </p>
          </div>
          {updates.length > 0 && (
            <button
              onClick={onUpdateAll}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
            >
              <Download size={16} />
              Update All ({updates.length})
            </button>
          )}
        </div>
        
        {updates.length === 0 ? (
          <EmptyState
            icon={CheckCircle}
            title="All up to date!"
            description="Your system is running the latest versions of all packages."
          />
        ) : (
          <div className="space-y-3">
            {updates.map((pkg) => (
              <PackageCard 
                key={`${pkg.source}-${pkg.name}`} 
                pkg={pkg} 
                onAction={onAction} 
              />
            ))}
          </div>
        )}
      </div>
    </PageTransition>
  )
}

// Browse page with real search
function BrowsePage({ loading, onAction }: { loading: boolean; onAction: (action: string, pkg: Package) => void }) {
  const [searchQuery, setSearchQuery] = useState('')
  const [results, setResults] = useState<Package[]>([])
  const [searched, setSearched] = useState(false)
  const [isSearching, setIsSearching] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  
  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) return
    
    setIsSearching(true)
    setSearched(true)
    try {
      const result = await invoke<Package[]>('search_packages', { query: searchQuery })
      setResults(result)
    } catch (e) {
      console.error('Search failed:', e)
      setResults([])
    }
    setIsSearching(false)
  }, [searchQuery])
  
  // Handle keyboard shortcut
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === '/' && document.activeElement !== inputRef.current) {
        e.preventDefault()
        inputRef.current?.focus()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])
  
  if (loading) {
    return (
      <PageTransition>
        <SkeletonList />
      </PageTransition>
    )
  }
  
  return (
    <PageTransition>
      <div>
        {/* Search */}
        <div className="relative mb-6">
          <Search size={20} className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-500" />
          <input
            ref={inputRef}
            type="text"
            placeholder="Search packages... (Press /)"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            className="w-full bg-[#1A1A1A] border border-[#27272A] rounded-lg pl-12 pr-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
          />
          {isSearching && (
            <div className="absolute right-4 top-1/2 -translate-y-1/2">
              <RefreshCw size={18} className="text-blue-500 animate-spin" />
            </div>
          )}
        </div>
        
        {!searched ? (
          <EmptyState
            icon={Search}
            title="Search packages"
            description="Search across all your package sources to find new software to install."
          />
        ) : results.length === 0 ? (
          <EmptyState
            icon={Package}
            title="No results found"
            description={`No packages matching "${searchQuery}"`}
            action={
              <button
                onClick={() => { setSearchQuery(''); setSearched(false); }}
                className="px-4 py-2 bg-[#1A1A1A] text-gray-400 rounded-lg hover:bg-[#27272A] transition-colors"
              >
                Clear search
              </button>
            }
          />
        ) : (
          <>
            <p className="text-sm text-gray-500 mb-4 flex items-center gap-2">
              <CheckCircle size={14} className="text-green-500" />
              {results.length} results for "{searchQuery}"
            </p>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {results.map((pkg) => (
                <PackageCard 
                  key={`${pkg.source}-${pkg.name}`} 
                  pkg={pkg} 
                  onAction={onAction} 
                />
              ))}
            </div>
          </>
        )}
      </div>
    </PageTransition>
  )
}

// Settings page
function SettingsPage({ onThemeChange }: { onThemeChange: (dark: boolean) => void }) {
  const [darkMode, setDarkMode] = useState(true)
  const [autoRefresh, setAutoRefresh] = useState(false)
  const [refreshInterval, setRefreshInterval] = useState(5)
  const [sources, setSources] = useState(SOURCES)
  
  const toggleSource = (id: string) => {
    setSources(prev => prev.map(s => 
      s.id === id ? { ...s, enabled: !s.enabled } : s
    ))
  }
  
  return (
    <PageTransition>
      <div className="max-w-2xl space-y-6">
        <h2 className="text-xl font-semibold text-white">Settings</h2>
        
        {/* Appearance */}
        <div className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-6">
          <h3 className="text-lg font-medium text-white mb-4 flex items-center gap-2">
            <Sun size={18} />
            Appearance
          </h3>
          
          <div className="flex items-center justify-between py-3 border-b border-[#27272A]">
            <div>
              <p className="text-white">Dark Mode</p>
              <p className="text-sm text-gray-500">Use dark theme (OLED optimized)</p>
            </div>
            <button 
              onClick={() => { setDarkMode(!darkMode); onThemeChange(!darkMode); }}
              className={`p-2 rounded-lg transition-colors ${
                darkMode ? 'bg-blue-600 text-white' : 'bg-[#1A1A1A] text-gray-400'
              }`}
            >
              {darkMode ? <Moon size={20} /> : <Sun size={20} />}
            </button>
          </div>
        </div>
        
        {/* Package Sources */}
        <div className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-6">
          <h3 className="text-lg font-medium text-white mb-4 flex items-center gap-2">
            <Globe size={18} />
            Package Sources
          </h3>
          
          <div className="space-y-2">
            {sources.map(source => (
              <div key={source.id} className="flex items-center justify-between py-2">
                <div className="flex items-center gap-3">
                  <span className="text-lg">{source.icon}</span>
                  <span className="text-white">{source.name}</span>
                </div>
                <button 
                  onClick={() => toggleSource(source.id)}
                  className={`transition-colors ${
                    source.enabled ? 'text-blue-400' : 'text-gray-600'
                  }`}
                >
                  {source.enabled ? <ToggleRight size={24} /> : <ToggleLeft size={24} />}
                </button>
              </div>
            ))}
          </div>
        </div>
        
        {/* Behavior */}
        <div className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-6">
          <h3 className="text-lg font-medium text-white mb-4 flex items-center gap-2">
            <Zap size={18} />
            Behavior
          </h3>
          
          <div className="space-y-4">
            <div className="flex items-center justify-between py-2">
              <div>
                <p className="text-white">Auto Refresh</p>
                <p className="text-sm text-gray-500">Check for updates on startup</p>
              </div>
              <button 
                onClick={() => setAutoRefresh(!autoRefresh)}
                className={`p-2 rounded-lg transition-colors ${
                  autoRefresh ? 'bg-blue-600 text-white' : 'bg-[#1A1A1A] text-gray-400'
                }`}
              >
                {autoRefresh ? <ToggleRight size={24} /> : <ToggleLeft size={24} />}
              </button>
            </div>
            
            {autoRefresh && (
              <div className="flex items-center justify-between py-2">
                <div>
                  <p className="text-white">Refresh Interval</p>
                  <p className="text-sm text-gray-500">Minutes between auto-refresh</p>
                </div>
                <select
                  value={refreshInterval}
                  onChange={(e) => setRefreshInterval(Number(e.target.value))}
                  className="bg-[#1A1A1A] border border-[#27272A] rounded-lg px-3 py-2 text-white"
                >
                  <option value={1}>1 minute</option>
                  <option value={5}>5 minutes</option>
                  <option value={15}>15 minutes</option>
                  <option value={30}>30 minutes</option>
                </select>
              </div>
            )}
          </div>
        </div>
        
        {/* Keyboard Shortcuts */}
        <div className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-6">
          <h3 className="text-lg font-medium text-white mb-4 flex items-center gap-2">
            <Keyboard size={18} />
            Keyboard Shortcuts
          </h3>
          <div className="grid grid-cols-2 gap-2">
            {SHORTCUTS.slice(0, 6).map(shortcut => (
              <div key={shortcut.key} className="flex items-center justify-between py-2">
                <span className="text-gray-400">{shortcut.action}</span>
                <kbd className="px-2 py-1 text-xs bg-[#1A1A1A] text-gray-400 rounded border border-[#27272A]">
                  {shortcut.key}
                </kbd>
              </div>
            ))}
          </div>
        </div>
        
        {/* About */}
        <div className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-6">
          <h3 className="text-lg font-medium text-white mb-4 flex items-center gap-2">
            <BookOpen size={18} />
            About
          </h3>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-500">Version</span>
              <span className="text-white">0.1.0</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-500">Backends</span>
              <span className="text-white">{sources.filter(s => s.enabled).length} enabled</span>
            </div>
          </div>
        </div>
      </div>
    </PageTransition>
  )
}

// Main App component
function AppContent() {
  const location = useLocation()
  const [packages, setPackages] = useState<Package[]>([])
  const [loading, setLoading] = useState(true)
  const [toasts, setToasts] = useState<Toast[]>([])
  const [showShortcuts, setShowShortcuts] = useState(false)

  const showToast = useCallback((type: Toast['type'], message: string) => {
    const id = Date.now()
    setToasts(prev => [...prev, { id, type, message }])
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id))
    }, 4000)
  }, [])

  async function loadPackages(tab: 'installed' | 'updates' | 'browse') {
    setLoading(true)
    try {
      let result: Package[] = []
      
      if (tab === 'installed') {
        result = await invoke<Package[]>('list_installed_packages')
      } else if (tab === 'updates') {
        result = await invoke<Package[]>('check_updates')
      } else {
        result = []
      }
      
      setPackages(result)
    } catch (e) {
      console.error('Failed to load packages:', e)
      showToast('error', 'Failed to load packages')
    }
    setLoading(false)
  }

  async function handleAction(action: string, pkg: Package) {
    try {
      if (action === 'install') {
        await invoke('install_package', { name: pkg.name, source: pkg.source })
        showToast('success', `Installing ${pkg.name}...`)
      } else if (action === 'remove') {
        await invoke('remove_package', { name: pkg.name, source: pkg.source })
        showToast('success', `Removing ${pkg.name}...`)
      } else if (action === 'update') {
        await invoke('update_package', { name: pkg.name, source: pkg.source })
        showToast('success', `Updating ${pkg.name}...`)
      }
      setTimeout(() => loadPackages('installed'), 2000)
    } catch (e) {
      console.error(`Failed to ${action} package:`, e)
      showToast('error', `Failed to ${action} ${pkg.name}`)
    }
  }

  async function handleUpdateAll() {
    const updates = packages.filter(p => p.status === 'update_available')
    for (const pkg of updates) {
      showToast('info', `Updating ${pkg.name}...`)
      await handleAction('update', pkg)
    }
    showToast('success', 'All packages updated!')
    loadPackages('installed')
  }

  const activeTab = location.pathname.replace('/', '') || 'installed'
  const updateCount = packages.filter(p => p.status === 'update_available').length
  const installedCount = packages.filter(p => p.status === 'installed').length

  let currentPage = 'installed'
  if (activeTab === 'updates') currentPage = 'updates'
  else if (activeTab === 'browse') currentPage = 'browse'
  else if (activeTab === 'settings') currentPage = 'settings'

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === '?' && !e.ctrlKey && !e.metaKey) {
        e.preventDefault()
        setShowShortcuts(true)
      }
      if (e.key === 'Escape') {
        setShowShortcuts(false)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  useEffect(() => {
    if (currentPage !== 'settings') {
      loadPackages(currentPage as 'installed' | 'updates' | 'browse')
    }
  }, [currentPage])

  return (
    <div className="flex h-screen bg-black">
      {/* Sidebar */}
      <aside className="w-64 bg-[#0A0A0A] border-r border-[#27272A] flex flex-col">
        <div className="p-4 border-b border-[#27272A]">
          <h1 className="text-xl font-bold text-white flex items-center gap-2">
            <Package className="text-blue-500" />
            LinGet
          </h1>
        </div>
        
        <nav className="flex-1 py-4 space-y-1 px-2">
          <button
            onClick={() => {}}
            className={`w-full flex items-center gap-3 px-3 py-2.5 text-left rounded-lg transition-colors ${
              activeTab === 'installed' 
                ? 'bg-[#1A1A1A] text-blue-400 border-l-2 border-blue-500' 
                : 'text-gray-300 hover:bg-[#1A1A1A]'
            }`}
          >
            <List size={20} />
            <span className="flex-1">Installed</span>
            <span className="text-xs bg-[#1A1A1A] px-2 py-0.5 rounded-full text-gray-400">
              {installedCount}
            </span>
          </button>
          
          <button
            onClick={() => {}}
            className={`w-full flex items-center gap-3 px-3 py-2.5 text-left rounded-lg transition-colors ${
              activeTab === 'updates' 
                ? 'bg-[#1A1A1A] text-yellow-400 border-l-2 border-yellow-500' 
                : 'text-gray-300 hover:bg-[#1A1A1A]'
            }`}
          >
            <Sparkles size={20} />
            <span className="flex-1">Updates</span>
            {updateCount > 0 && (
              <span className="text-xs bg-yellow-600/20 px-2 py-0.5 rounded-full text-yellow-400">
                {updateCount}
              </span>
            )}
          </button>
          
          <button
            onClick={() => {}}
            className={`w-full flex items-center gap-3 px-3 py-2.5 text-left rounded-lg transition-colors ${
              activeTab === 'browse' 
                ? 'bg-[#1A1A1A] text-green-400 border-l-2 border-green-500' 
                : 'text-gray-300 hover:bg-[#1A1A1A]'
            }`}
          >
            <Search size={20} />
            <span className="flex-1">Browse</span>
          </button>
          
          <button
            onClick={() => {}}
            className={`w-full flex items-center gap-3 px-3 py-2.5 text-left rounded-lg transition-colors ${
              activeTab === 'settings' 
                ? 'bg-[#1A1A1A] text-purple-400 border-l-2 border-purple-500' 
                : 'text-gray-300 hover:bg-[#1A1A1A]'
            }`}
          >
            <Settings size={20} />
            <span className="flex-1">Settings</span>
          </button>
        </nav>
        
        <div className="p-4 border-t border-[#27272A]">
          <button
            onClick={() => setShowShortcuts(true)}
            className="w-full flex items-center gap-3 px-3 py-2.5 text-left text-gray-400 hover:bg-[#1A1A1A] rounded-lg transition-colors text-sm"
          >
            <Keyboard size={16} />
            <span>Shortcuts</span>
            <kbd className="ml-auto px-1.5 py-0.5 text-xs bg-[#1A1A1A] rounded">?</kbd>
          </button>
        </div>
      </aside>
      
      {/* Main Content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        <header className="bg-[#0A0A0A] border-b border-[#27272A] px-6 py-4">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xl font-semibold text-white">
                {activeTab === 'installed' && 'Installed Packages'}
                {activeTab === 'updates' && 'Available Updates'}
                {activeTab === 'browse' && 'Browse Packages'}
                {activeTab === 'settings' && 'Settings'}
              </h2>
              {activeTab !== 'settings' && (
                <p className="text-sm text-gray-500 mt-1">
                  {activeTab === 'updates' && updateCount > 0 
                    ? `${updateCount} packages can be updated` 
                    : activeTab === 'installed' 
                      ? `${installedCount} packages installed`
                      : 'Search and discover new packages'}
                </p>
              )}
            </div>
            
            {activeTab !== 'settings' && (
              <button
                onClick={() => loadPackages(currentPage as 'installed' | 'updates' | 'browse')}
                disabled={loading}
                className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 transition-colors"
              >
                <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
                Refresh
              </button>
            )}
          </div>
        </header>
        
        <div className="flex-1 overflow-auto p-6">
          <AnimatePresence mode="wait">
            {activeTab === 'installed' && (
              <InstalledPage 
                key="installed"
                packages={packages} 
                loading={loading} 
                onAction={handleAction}
                loadPackages={() => loadPackages('installed')}
              />
            )}
            {activeTab === 'updates' && (
              <UpdatesPage 
                key="updates"
                packages={packages} 
                loading={loading} 
                onAction={handleAction}
                onUpdateAll={handleUpdateAll}
              />
            )}
            {activeTab === 'browse' && (
              <BrowsePage 
                key="browse"
                loading={loading} 
                onAction={handleAction}
              />
            )}
            {activeTab === 'settings' && (
              <SettingsPage 
                key="settings"
                onThemeChange={() => {}}
              />
            )}
          </AnimatePresence>
        </div>
      </main>

      <ToastContainer toasts={toasts} onDismiss={(id) => setToasts(prev => prev.filter(t => t.id !== id))} />
      
      <AnimatePresence>
        {showShortcuts && <ShortcutsModal onClose={() => setShowShortcuts(false)} />}
      </AnimatePresence>
    </div>
  )
}

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<AppContent />} />
        <Route path="/installed" element={<AppContent />} />
        <Route path="/updates" element={<AppContent />} />
        <Route path="/browse" element={<AppContent />} />
        <Route path="/settings" element={<AppContent />} />
      </Routes>
    </BrowserRouter>
  )
}

export default App
