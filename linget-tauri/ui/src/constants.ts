import type { Shortcut, SourceInfo } from './types'

export const DEFAULT_SOURCES: SourceInfo[] = [
  { id: 'APT', name: 'APT', icon: '🟢', enabled: true, description: 'System packages (Debian/Ubuntu)' },
  { id: 'Flatpak', name: 'Flatpak', icon: '🟣', enabled: true, description: 'Sandboxed applications' },
  { id: 'Snap', name: 'Snap', icon: '🔵', enabled: true, description: 'Ubuntu Snap packages' },
  { id: 'npm', name: 'npm', icon: '🟡', enabled: true, description: 'Node.js packages' },
  { id: 'pip', name: 'pip', icon: '🐍', enabled: true, description: 'Python packages' },
  { id: 'pipx', name: 'pipx', icon: '🐍', enabled: true, description: 'Python app packages (pipx)' },
  { id: 'cargo', name: 'Cargo', icon: '🦀', enabled: true, description: 'Rust crates (cargo install)' },
  { id: 'brew', name: 'Homebrew', icon: '🍺', enabled: true, description: 'Linuxbrew packages' },
  { id: 'dnf', name: 'DNF', icon: '🔴', enabled: true, description: 'Fedora/RHEL packages' },
  { id: 'pacman', name: 'Pacman', icon: '📦', enabled: true, description: 'Arch Linux packages' },
  { id: 'zypper', name: 'Zypper', icon: '🟠', enabled: true, description: 'openSUSE packages' },
  { id: 'conda', name: 'Conda', icon: '🟢', enabled: true, description: 'Conda packages' },
  { id: 'mamba', name: 'Mamba', icon: '🟢', enabled: true, description: 'Mamba packages' },
]

export const SHORTCUTS: Shortcut[] = [
  { key: 'r', action: 'Refresh' },
  { key: '/', action: 'Search' },
  { key: 'i', action: 'Installed' },
  { key: 'u', action: 'Updates' },
  { key: 'b', action: 'Browse' },
  { key: 's', action: 'Settings' },
  { key: '?', action: 'Shortcuts' },
  { key: 't', action: 'Task Hub' },
]
