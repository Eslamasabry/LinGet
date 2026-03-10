import { useCallback, useEffect, useMemo, useRef, useState, type ElementType } from 'react'
import { HashRouter, Route, Routes, useLocation, useNavigate } from 'react-router-dom'
import { AnimatePresence } from 'framer-motion'
import {
  Grid3X3,
  Info,
  Keyboard,
  List,
  Package as PackageIcon,
  RefreshCw,
  Search,
  Settings,
  Terminal,
} from 'lucide-react'
import { ConfirmDialog, AboutModal, ShortcutsModal } from './components/modals'
import { TaskHubModal } from './components/TaskHub'
import { ToastContainer, PageTransition } from './components/shared'
import { DEFAULT_SOURCES } from './constants'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import { useOperationEvents } from './hooks/useOperationEvents'
import { useToasts } from './hooks/useToasts'
import { BrowsePage } from './pages/BrowsePage'
import { InstalledPage } from './pages/InstalledPage'
import { SettingsPage } from './pages/SettingsPage'
import { UpdatesPage } from './pages/UpdatesPage'
import {
  cancelOperation,
  getBackendSources,
  installPackage,
  loadInstalledPackages,
  loadSettings,
  loadUpdates,
  persistSettings,
  removePackage,
  searchPackages,
  updateAllPackages,
  updatePackage,
} from './services/tauri'
import type { AppTab, ConfirmDialogState, Package, RunningOperation, SettingsData, SourceInfo } from './types'

const NAV_ITEMS: Array<{ id: AppTab; label: string; icon: ElementType; path: string }> = [
  { id: 'installed', label: 'Installed', icon: List, path: '/' },
  { id: 'updates', label: 'Updates', icon: RefreshCw, path: '/updates' },
  { id: 'browse', label: 'Browse', icon: Search, path: '/browse' },
  { id: 'settings', label: 'Settings', icon: Settings, path: '/settings' },
]

const DEFAULT_SETTINGS: SettingsData = {
  dark_mode: true,
  auto_refresh: true,
  refresh_interval: 3600,
  enabled_sources: DEFAULT_SOURCES.filter((source) => source.enabled).map((source) => source.id),
}

function tabFromPath(pathname: string): AppTab {
  switch (pathname) {
    case '/updates':
      return 'updates'
    case '/browse':
      return 'browse'
    case '/settings':
      return 'settings'
    default:
      return 'installed'
  }
}

function actionLabel(action: ConfirmDialogState['action']) {
  return action === 'install' ? 'Install' : action === 'remove' ? 'Remove' : 'Update'
}

function actionVerb(type: RunningOperation['type']) {
  return type === 'install' ? 'installation' : type === 'remove' ? 'removal' : 'update'
}

function mergeSources(settings: SettingsData, backendSources: SourceInfo[]): SourceInfo[] {
  const enabledSet = new Set(settings.enabled_sources)
  return backendSources.map((source) => ({
    ...source,
    enabled: enabledSet.size > 0 ? enabledSet.has(source.id) : source.enabled,
  }))
}

function AppShell() {
  const location = useLocation()
  const navigate = useNavigate()
  const activeTab = tabFromPath(location.pathname)
  const { toasts, showToast, dismissToast } = useToasts()

  const [packages, setPackages] = useState<Package[]>([])
  const [updates, setUpdates] = useState<Package[]>([])
  const [browsePackages, setBrowsePackages] = useState<Package[]>([])
  const [sources, setSources] = useState<SourceInfo[]>(DEFAULT_SOURCES)
  const [settings, setSettings] = useState<SettingsData>(DEFAULT_SETTINGS)
  const [runningOperations, setRunningOperations] = useState<RunningOperation[]>([])
  const [loading, setLoading] = useState(true)
  const [searching, setSearching] = useState(false)
  const [saving, setSaving] = useState(false)
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid')
  const [searchQuery, setSearchQuery] = useState('')
  const [sourceFilter, setSourceFilter] = useState<string>('all')
  const [updateProgress, setUpdateProgress] = useState(0)
  const [showTaskHub, setShowTaskHub] = useState(false)
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [showAbout, setShowAbout] = useState(false)
  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState>({
    isOpen: false,
    action: '',
    pkg: null,
  })
  const operationStatusRef = useRef<Record<string, RunningOperation['status']>>({})

  const loadPackageData = useCallback(async () => {
    const [installed, availableUpdates] = await Promise.all([loadInstalledPackages(), loadUpdates()])
    setPackages(installed)
    setUpdates(availableUpdates)
  }, [])

  const loadSettingsData = useCallback(async () => {
    const [settingsData, backendSources] = await Promise.all([loadSettings(), getBackendSources()])
    const normalizedSettings = {
      ...DEFAULT_SETTINGS,
      ...settingsData,
      enabled_sources:
        settingsData.enabled_sources && settingsData.enabled_sources.length > 0
          ? settingsData.enabled_sources
          : backendSources.filter((source) => source.enabled).map((source) => source.id),
    }

    setSettings(normalizedSettings)
    setSources(mergeSources(normalizedSettings, backendSources))
  }, [])

  const bootstrap = useCallback(async () => {
    setLoading(true)
    try {
      await Promise.all([loadPackageData(), loadSettingsData()])
    } catch (error) {
      console.error('Failed to bootstrap Tauri UI', error)
      showToast('error', 'Failed to load package data')
    } finally {
      setLoading(false)
    }
  }, [loadPackageData, loadSettingsData, showToast])

  const refreshActiveTab = useCallback(async () => {
    try {
      if (activeTab === 'settings') {
        await loadSettingsData()
      } else if (activeTab === 'browse') {
        if (!searchQuery.trim()) {
          setBrowsePackages([])
          return
        }
        setSearching(true)
        setBrowsePackages(await searchPackages(searchQuery.trim()))
        setSearching(false)
      } else {
        setLoading(true)
        await loadPackageData()
      }
    } catch (error) {
      console.error('Failed to refresh active tab', error)
      showToast('error', 'Refresh failed')
    } finally {
      setLoading(false)
      setSearching(false)
    }
  }, [activeTab, loadPackageData, loadSettingsData, searchQuery, showToast])

  useOperationEvents({ setRunningOperations, setUpdateProgress })

  useKeyboardShortcuts({
    onShowShortcuts: () => setShowShortcuts(true),
    onToggleTaskHub: () => setShowTaskHub((current) => !current),
    onCloseOverlays: () => {
      setShowShortcuts(false)
      setShowAbout(false)
      setShowTaskHub(false)
      setConfirmDialog((current) => ({ ...current, isOpen: false }))
    },
  })

  useEffect(() => {
    void bootstrap()
  }, [bootstrap])

  useEffect(() => {
    if (activeTab !== 'browse') {
      return
    }

    if (!searchQuery.trim()) {
      setBrowsePackages([])
      setSearching(false)
      return
    }

    setSearching(true)
    const timeout = window.setTimeout(async () => {
      try {
        setBrowsePackages(await searchPackages(searchQuery.trim()))
      } catch (error) {
        console.error('Search failed', error)
        showToast('error', 'Search failed')
      } finally {
        setSearching(false)
      }
    }, 250)

    return () => window.clearTimeout(timeout)
  }, [activeTab, searchQuery, showToast])

  useEffect(() => {
    let shouldReload = false
    const nextStatusMap: Record<string, RunningOperation['status']> = {}

    runningOperations.forEach((operation) => {
      const previousStatus = operationStatusRef.current[operation.id]
      nextStatusMap[operation.id] = operation.status

      if (previousStatus && previousStatus !== operation.status) {
        if (operation.status === 'completed') {
          showToast('success', `${operation.name} ${actionVerb(operation.type)} completed`)
          shouldReload = true
        } else if (operation.status === 'failed') {
          showToast('error', `${operation.name} ${actionVerb(operation.type)} failed`)
        }
      }
    })

    operationStatusRef.current = nextStatusMap

    if (shouldReload) {
      void loadPackageData()
    }
  }, [loadPackageData, runningOperations, showToast])

  const filteredPackages = useMemo(
    () =>
      packages.filter((pkg) => {
        const matchesQuery =
          !searchQuery ||
          pkg.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          pkg.description.toLowerCase().includes(searchQuery.toLowerCase())
        const matchesSource = sourceFilter === 'all' || pkg.source === sourceFilter
        return matchesQuery && matchesSource
      }),
    [packages, searchQuery, sourceFilter],
  )

  const filteredUpdates = useMemo(
    () => updates.filter((pkg) => sourceFilter === 'all' || pkg.source === sourceFilter),
    [sourceFilter, updates],
  )

  const filteredBrowsePackages = useMemo(
    () => browsePackages.filter((pkg) => sourceFilter === 'all' || pkg.source === sourceFilter),
    [browsePackages, sourceFilter],
  )

  const openConfirm = useCallback((action: 'install' | 'remove' | 'update', pkg: Package) => {
    setConfirmDialog({ isOpen: true, action, pkg })
  }, [])

  const closeConfirm = useCallback(() => {
    setConfirmDialog({ isOpen: false, action: '', pkg: null })
  }, [])

  const executeConfirmedAction = useCallback(async () => {
    const { action, pkg } = confirmDialog
    if (!pkg || !action) {
      return
    }

    closeConfirm()

    try {
      if (action === 'install') {
        await installPackage(pkg.name, pkg.source)
      } else if (action === 'remove') {
        await removePackage(pkg.name, pkg.source)
      } else {
        await updatePackage(pkg.name, pkg.source)
      }
      showToast('info', `${actionLabel(action)} started for ${pkg.name}`)
    } catch (error) {
      console.error(`Failed to ${action} package`, error)
      showToast('error', `${actionLabel(action)} failed for ${pkg.name}`)
    }
  }, [closeConfirm, confirmDialog, showToast])

  const handleUpdateAll = useCallback(async () => {
    try {
      setUpdateProgress(0)
      await updateAllPackages()
      showToast('info', 'Update all started')
    } catch (error) {
      console.error('Failed to start update all', error)
      showToast('error', 'Failed to start update all')
    }
  }, [showToast])

  const handleCancelOperation = useCallback(
    async (operationId: string) => {
      try {
        await cancelOperation(operationId)
        showToast('warning', 'Cancellation requested')
      } catch (error) {
        console.error('Failed to cancel operation', error)
        showToast('error', 'Failed to cancel operation')
      }
    },
    [showToast],
  )

  const handleSettingsChange = useCallback((patch: Partial<SettingsData>) => {
    setSettings((current) => ({ ...current, ...patch }))
  }, [])

  const handleSourceToggle = useCallback((sourceId: string) => {
    setSources((current) =>
      current.map((source) =>
        source.id === sourceId && source.available !== false ? { ...source, enabled: !source.enabled } : source,
      ),
    )

    setSettings((current) => {
      const enabled = current.enabled_sources.includes(sourceId)
        ? current.enabled_sources.filter((id) => id !== sourceId)
        : [...current.enabled_sources, sourceId]
      return { ...current, enabled_sources: enabled }
    })
  }, [])

  const handleSaveSettings = useCallback(async () => {
    setSaving(true)
    try {
      const enabledSources = sources.filter((source) => source.enabled && source.available !== false).map((source) => source.id)
      const nextSettings = { ...settings, enabled_sources: enabledSources }
      await persistSettings(nextSettings)
      setSettings(nextSettings)
      showToast('success', 'Settings saved')
      await loadSettingsData()
      await loadPackageData()
    } catch (error) {
      console.error('Failed to save settings', error)
      showToast('error', 'Failed to save settings')
    } finally {
      setSaving(false)
    }
  }, [loadPackageData, loadSettingsData, settings, showToast, sources])

  const totalEnabledSources = sources.filter((source) => source.enabled && source.available !== false).length

  return (
    <div className="min-h-screen bg-[#050505] text-white flex">
      <aside className="w-64 border-r border-[#18181B] bg-[#080808] p-4 flex flex-col">
        <div className="flex items-center gap-3 mb-6 px-2">
          <div className="w-10 h-10 rounded-xl bg-blue-600/20 border border-blue-500/20 flex items-center justify-center">
            <PackageIcon size={20} className="text-blue-400" />
          </div>
          <div>
            <h1 className="font-semibold text-white">LinGet</h1>
            <p className="text-xs text-gray-500">Unified package manager</p>
          </div>
        </div>

        <nav className="space-y-1 mb-6">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon
            const active = activeTab === item.id
            return (
              <button
                key={item.id}
                onClick={() => navigate(item.path)}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors ${
                  active ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white hover:bg-[#111111]'
                }`}
              >
                <Icon size={18} />
                <span className="font-medium">{item.label}</span>
              </button>
            )
          })}
        </nav>

        <div className="space-y-2 mb-6">
          <div className="px-2 flex items-center justify-between text-xs text-gray-500 uppercase tracking-wide">
            <span>Sources</span>
            <span>{totalEnabledSources} enabled</span>
          </div>
          <div className="space-y-1 max-h-[18rem] overflow-y-auto pr-1">
            {sources.map((source) => (
              <div key={source.id} className="flex items-center justify-between px-3 py-2 rounded-lg bg-[#0F0F10] border border-[#18181B]">
                <div className="min-w-0">
                  <div className="flex items-center gap-2 text-sm text-gray-200">
                    <span>{source.icon}</span>
                    <span className="truncate">{source.name}</span>
                  </div>
                  <div className="text-xs text-gray-500 truncate">
                    {source.available === false ? 'Not detected on this system' : source.description}
                  </div>
                </div>
                <span
                  className={`ml-2 h-2.5 w-2.5 rounded-full ${
                    source.available === false ? 'bg-red-500/70' : source.enabled ? 'bg-green-500' : 'bg-gray-600'
                  }`}
                />
              </div>
            ))}
          </div>
        </div>

        <div className="mt-auto space-y-2">
          <button
            onClick={() => setShowTaskHub(true)}
            className="w-full flex items-center justify-between px-3 py-2.5 rounded-lg bg-[#111111] border border-[#1F1F22] text-gray-300 hover:text-white hover:border-[#2A2A2E] transition-colors"
          >
            <span className="flex items-center gap-3">
              <Terminal size={17} />
              Task Hub
            </span>
            <span className="text-xs text-gray-500">{runningOperations.length}</span>
          </button>
          <button
            onClick={() => setShowAbout(true)}
            className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#111111] transition-colors"
          >
            <Info size={17} />
            About
          </button>
          <button
            onClick={() => setShowShortcuts(true)}
            className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#111111] transition-colors"
          >
            <Keyboard size={17} />
            Shortcuts
          </button>
        </div>
      </aside>

      <main className="flex-1 flex flex-col min-w-0">
        <header className="border-b border-[#18181B] bg-[#080808]/95 backdrop-blur px-6 py-4">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div>
              <h2 className="text-2xl font-semibold text-white capitalize">{activeTab === 'browse' ? 'Discover packages' : activeTab}</h2>
              <p className="text-sm text-gray-500">
                {activeTab === 'installed'
                  ? `${packages.length} installed packages available locally.`
                  : activeTab === 'updates'
                    ? `${updates.length} packages currently have updates.`
                    : activeTab === 'browse'
                      ? 'Search all enabled providers for software.'
                      : 'Adjust behavior, theme, and enabled sources.'}
              </p>
            </div>

            <div className="flex flex-col gap-3 md:flex-row md:items-center">
              {activeTab !== 'settings' && (
                <div className="flex items-center gap-2 bg-[#0F0F10] border border-[#18181B] rounded-lg px-3 py-2 min-w-[18rem]">
                  <Search size={16} className="text-gray-500" />
                  <input
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                    placeholder={activeTab === 'browse' ? 'Search across sources...' : 'Filter visible packages...'}
                    className="bg-transparent outline-none text-sm text-white placeholder:text-gray-600 flex-1"
                  />
                </div>
              )}

              <select
                value={sourceFilter}
                onChange={(event) => setSourceFilter(event.target.value)}
                className="bg-[#0F0F10] border border-[#18181B] rounded-lg px-3 py-2 text-sm text-gray-300 outline-none"
              >
                <option value="all">All sources</option>
                {sources
                  .filter((source) => source.available !== false)
                  .map((source) => (
                    <option key={source.id} value={source.name}>
                      {source.name}
                    </option>
                  ))}
              </select>

              {activeTab !== 'settings' && (
                <div className="flex items-center rounded-lg border border-[#18181B] overflow-hidden">
                  <button
                    onClick={() => setViewMode('grid')}
                    className={`px-3 py-2 transition-colors ${
                      viewMode === 'grid' ? 'bg-blue-600 text-white' : 'bg-[#0F0F10] text-gray-500 hover:text-white'
                    }`}
                    title="Grid view"
                  >
                    <Grid3X3 size={16} />
                  </button>
                  <button
                    onClick={() => setViewMode('list')}
                    className={`px-3 py-2 transition-colors ${
                      viewMode === 'list' ? 'bg-blue-600 text-white' : 'bg-[#0F0F10] text-gray-500 hover:text-white'
                    }`}
                    title="List view"
                  >
                    <List size={16} />
                  </button>
                </div>
              )}

              <button
                onClick={() => void refreshActiveTab()}
                className="px-4 py-2 bg-[#111111] border border-[#1F1F22] rounded-lg text-gray-300 hover:text-white hover:border-[#2A2A2E] transition-colors flex items-center gap-2"
              >
                <RefreshCw size={16} />
                Refresh
              </button>
            </div>
          </div>
        </header>

        <section className="px-6 py-4 border-b border-[#111114] bg-[#070707]">
          <div className="grid gap-3 md:grid-cols-3">
            <div className="rounded-xl border border-[#18181B] bg-[#0B0B0C] p-4">
              <div className="text-xs uppercase tracking-wide text-gray-500 mb-1">Installed</div>
              <div className="text-2xl font-semibold text-white">{packages.length}</div>
            </div>
            <div className="rounded-xl border border-[#18181B] bg-[#0B0B0C] p-4">
              <div className="text-xs uppercase tracking-wide text-gray-500 mb-1">Updates</div>
              <div className="text-2xl font-semibold text-white">{updates.length}</div>
            </div>
            <div className="rounded-xl border border-[#18181B] bg-[#0B0B0C] p-4">
              <div className="text-xs uppercase tracking-wide text-gray-500 mb-1">Running tasks</div>
              <div className="text-2xl font-semibold text-white">{runningOperations.length}</div>
            </div>
          </div>
        </section>

        <section className="flex-1 overflow-y-auto px-6 py-6">
          <AnimatePresence mode="wait">
            <Routes location={location} key={location.pathname}>
              <Route
                path="/"
                element={
                  <PageTransition>
                    <InstalledPage
                      loading={loading}
                      viewMode={viewMode}
                      filteredPackages={filteredPackages}
                      onAction={openConfirm}
                    />
                  </PageTransition>
                }
              />
              <Route
                path="/updates"
                element={
                  <PageTransition>
                    <UpdatesPage
                      loading={loading}
                      viewMode={viewMode}
                      filteredUpdates={filteredUpdates}
                      updateProgress={updateProgress}
                      onAction={openConfirm}
                      onUpdateAll={handleUpdateAll}
                    />
                  </PageTransition>
                }
              />
              <Route
                path="/browse"
                element={
                  <PageTransition>
                    <BrowsePage
                      loading={searching}
                      viewMode={viewMode}
                      searchQuery={searchQuery}
                      filteredBrowsePackages={filteredBrowsePackages}
                      onAction={openConfirm}
                    />
                  </PageTransition>
                }
              />
              <Route
                path="/settings"
                element={
                  <PageTransition>
                    <SettingsPage
                      sources={sources}
                      settings={settings}
                      loading={loading}
                      saving={saving}
                      onSettingsChange={handleSettingsChange}
                      onSourceToggle={handleSourceToggle}
                      onSave={handleSaveSettings}
                    />
                  </PageTransition>
                }
              />
            </Routes>
          </AnimatePresence>
        </section>
      </main>

      <TaskHubModal
        isOpen={showTaskHub}
        onClose={() => setShowTaskHub(false)}
        operations={runningOperations}
        onCancelOperation={handleCancelOperation}
      />

      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={`${actionLabel(confirmDialog.action)} ${confirmDialog.pkg?.name ?? ''}`.trim()}
        message={
          confirmDialog.pkg
            ? `${actionLabel(confirmDialog.action)} ${confirmDialog.pkg.name} from ${confirmDialog.pkg.source}?`
            : ''
        }
        confirmText={actionLabel(confirmDialog.action)}
        cancelText="Cancel"
        confirmStyle={confirmDialog.action === 'remove' ? 'danger' : 'primary'}
        onConfirm={() => void executeConfirmedAction()}
        onCancel={closeConfirm}
      />

      <AnimatePresence>
        {showShortcuts && <ShortcutsModal onClose={() => setShowShortcuts(false)} />}
        {showAbout && <AboutModal onClose={() => setShowAbout(false)} />}
      </AnimatePresence>

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  )
}

export default function App() {
  return (
    <HashRouter>
      <AppShell />
    </HashRouter>
  )
}
