import { invoke } from '@tauri-apps/api/core'
import type { Package, SettingsData, SourceInfo } from '../types'

interface BackendSourcePayload {
  id: string
  name: string
  description: string
  icon: string
  enabled: boolean
  available: boolean
}

export async function getBackendSources(): Promise<SourceInfo[]> {
  const backendSources = await invoke<BackendSourcePayload[]>('get_backend_sources')
  return backendSources.map((source) => ({
    id: source.id,
    name: source.name,
    icon: source.icon || '📦',
    enabled: source.enabled,
    available: source.available,
    description: source.description,
  }))
}

export async function loadSettings(): Promise<SettingsData> {
  return invoke<SettingsData>('load_settings')
}

export async function loadInstalledPackages(): Promise<Package[]> {
  return invoke<Package[]>('list_installed_packages')
}

export async function loadUpdates(): Promise<Package[]> {
  return invoke<Package[]>('check_updates')
}

export async function searchPackages(query: string): Promise<Package[]> {
  return invoke<Package[]>('search_packages', { query })
}

export async function installPackage(name: string, source: string): Promise<void> {
  await invoke('install_package', { name, source })
}

export async function removePackage(name: string, source: string): Promise<void> {
  await invoke('remove_package', { name, source })
}

export async function updatePackage(name: string, source: string): Promise<void> {
  await invoke('update_package', { name, source })
}

export async function updateAllPackages(): Promise<void> {
  await invoke('update_all_packages')
}

export async function persistSettings(settings: SettingsData): Promise<void> {
  await invoke('save_settings', { settings })
}

export async function cancelOperation(operationId: string): Promise<void> {
  await invoke('cancel_operation', { operation_id: operationId })
}
