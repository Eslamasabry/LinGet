export interface Package {
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

export interface Toast {
  id: number
  type: 'success' | 'error' | 'info' | 'warning'
  message: string
}

export interface OperationLog {
  id: string
  timestamp: Date
  type: 'info' | 'success' | 'error' | 'warning'
  message: string
  name?: string
}

export interface RunningOperation {
  id: string
  name: string
  type: 'install' | 'remove' | 'update'
  status: 'started' | 'running' | 'completed' | 'failed'
  progress: number
  message: string
  logs: OperationLog[]
  startTime: Date
}

export interface OperationProgressPayload {
  operation_id: string
  name: string
  status: RunningOperation['status']
  progress: number
  message: string
}

export interface OperationLogPayload {
  operation_id: string
  name: string
  line: string
  is_error: boolean
}

export interface BatchUpdateProgressPayload {
  completed: number
  total: number
  progress: number
}

export interface BatchUpdateItemPayload {
  name: string
  source: string
  status: 'updated' | 'failed'
  error?: string | null
}

export interface SourceInfo {
  id: string
  name: string
  icon: string
  enabled: boolean
  available?: boolean
  description?: string
  count?: number
}

export interface SettingsData {
  dark_mode: boolean
  auto_refresh: boolean
  refresh_interval: number
  enabled_sources: string[]
}

export interface Shortcut {
  key: string
  action: string
}

export type PackageAction = 'install' | 'remove' | 'update'
export type AppTab = 'installed' | 'updates' | 'browse' | 'settings'

export interface ConfirmDialogState {
  isOpen: boolean
  action: PackageAction | ''
  pkg: Package | null
}
