# LinGet Tauri UI - Parallel Development Plan

## Overview

Build a **parallel, modern Tauri-based frontend** for LinGet while maintaining the existing GTK4 version. Users can choose which UI they prefer.

```
┌─────────────────────────────────────────────────────────────────┐
│                        LinGet Project                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────────────┐         ┌─────────────────────────────┐   │
│   │  Current        │         │  New (Parallel)             │   │
│   │  GTK4/Libadwaita│         │  Tauri + Modern Web UI      │   │
│   │  • Legacy       │         │  • Modern UX                │   │
│   │  • Rigid        │         │  • Customizable             │   │
│   │  • GNOME-only   │         │  • Cross-platform capable   │   │
│   └─────────────────┘         └─────────────────────────────┘   │
│                                                                 │
│              ┌─────────────────────────────────┐                │
│              │   Shared Rust Backend           │                │
│              │   (All package backends)        │                │
│              └─────────────────────────────────┘                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Project Structure

```
linget/
├── Cargo.toml                    # Workspace root
├── linget-gtk/                   # Existing GTK4 UI (renamed)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── app.rs
│   │   ├── backend/              # Backend implementations
│   │   ├── ui/                   # Relm4 components
│   │   └── models/
│   └── assets/
├── linget-tauri/                 # NEW: Tauri UI
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs               # Tauri entry point
│   │   ├── api/                  # Rust API layer (tauri commands)
│   │   ├── window.rs             # Window configuration
│   │   └── menu.rs               # Menu configuration
│   ├── ui/                       # Web frontend
│   │   ├── index.html
│   │   ├── src/
│   │   │   ├── main.ts           # App entry
│   │   │   ├── App.tsx           # Root component
│   │   │   ├── components/       # UI components
│   │   │   ├── pages/            # Page components
│   │   │   ├── store/            # State management
│   │   │   ├── services/         # API services
│   │   │   ├── hooks/            # Custom React hooks
│   │   │   ├── utils/            # Utilities
│   │   │   └── styles/           # Global styles
│   │   ├── package.json
│   │   ├── tailwind.config.js
│   │   └── vite.config.ts
│   ├── icons/                    # App icons
│   └── tauri.conf.json           # Tauri configuration
├── packages/
│   ├── backend-core/             # SHARED: Core backend logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── traits.rs         # PackageBackend trait
│   │       ├── mod.rs
│   │       ├── streaming.rs      # Command streaming
│   │       ├── pkexec.rs         # Privilege escalation
│   │       └── types.rs          # Shared types
│   ├── models/                   # SHARED: Data models
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── package.rs
│   │       ├── config.rs
│   │       └── ...
│   └── cli/                      # SHARED: CLI (refactored)
├── Cargo.toml                    # Workspace manifest
└── README.md
```

---

## Phased Implementation Plan

### Phase 1: Backend Extraction (Week 1)

**Goal**: Extract shared backend into reusable crate

#### 1.1 Create backend-core crate
```
File: packages/backend-core/Cargo.toml

[package]
name = "linget-backend-core"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
which = "5"
# ... existing deps
```

#### 1.2 Extract shared modules
```rust
// packages/backend-core/src/traits.rs

use async_trait::async_trait;
use anyhow::Result;
use crate::types::{Package, PackageSource, PackageStatus};

#[async_trait]
pub trait PackageBackend: Send + Sync {
    fn is_available() -> bool;
    fn source(&self) -> PackageSource;
    
    async fn list_installed(&self) -> Result<Vec<Package>>;
    async fn list_available(&self) -> Result<Vec<Package>>;
    async fn search(&self, query: &str) -> Result<Vec<Package>>;
    async fn check_updates(&self) -> Result<Vec<Package>>;
    
    async fn install(&self, name: &str) -> Result<()>;
    async fn remove(&self, name: &str) -> Result<()>;
    async fn update(&self, name: &str) -> Result<()>;
    
    // Optional methods with defaults
    async fn get_info(&self, name: &str) -> Result<Option<Package>> { ... }
    async fn get_cache_size(&self) -> Result<u64> { Ok(0) }
    async fn cleanup_cache(&self) -> Result<u64> { Ok(0) }
}
```

#### 1.3 Migrate existing backends
```
Files to extract:
- src/backend/traits.rs → packages/backend-core/src/traits.rs
- src/backend/streaming.rs → packages/backend-core/src/streaming.rs
- src/backend/pkexec.rs → packages/backend-core/src/pkexec.rs
- src/backend/apt.rs → packages/backend-core/src/backends/apt.rs
- src/backend/flatpak.rs → packages/backend-core/src/backends/flatpak.rs
- ... all 17 backends
```

#### 1.4 Update GTK version to use shared crate
```rust
// linget-gtk/src/backend/mod.rs

pub use linget_backend_core::traits::PackageBackend;
pub use linget_backend_core::backends::{AptBackend, FlatpakBackend, ...};
```

---

### Phase 2: Tauri Setup (Week 2)

**Goal**: Set up Tauri project structure

#### 2.1 Initialize Tauri app
```bash
cd linget-tauri
cargo create-tauri-app --template react-ts
```

#### 2.2 Configure Tauri
```json
// linget-tauri/src-tauri/tauri.conf.json
{
  "productName": "linget",
  "version": "0.1.0",
  "identifier": "com.linget.manager",
  "build": {
    "beforeDevCommand": "cd ui && npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "cd ui && npm run build"
  },
  "package": {
    "productName": "LinGet",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "open": true
      },
      "window": {
        "all": true
      },
      "process": {
        "all": true
      }
    },
    "bundle": {
      "active": true,
      "targets": "deb",
      "deb": {
        "depends": ["libwebkit2gtk-4.0", "libgtk-3"]
      }
    },
    "security": {
      "csp": null
    }
  }
}
```

#### 2.3 Create Rust API layer
```rust
// linget-tauri/src/api/mod.rs

use tauri::State;
use linget_backend_core::{PackageManager, Config};

pub struct AppState {
    pub package_manager: PackageManager,
    pub config: Config,
}

#[tauri::command]
pub async fn list_installed_sources(
    state: State<'_, AppState>,
) -> Result<Vec<PackageSource>, String> {
    Ok(state.package_manager.list_sources().await)
}

#[tauri::command]
pub async fn get_packages(
    state: State<'_, AppState>,
    source: Option<PackageSource>,
) -> Result<Vec<Package>, String> {
    match source {
        Some(s) => state.package_manager.list_by_source(s).await,
        None => state.package_manager.list_all_installed().await,
    }
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_package(
    state: State<'_, AppState>,
    name: String,
    source: PackageSource,
) -> Result<(), String> {
    state.package_manager.install(&name, source).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_package(
    state: State<'_, AppState>,
    name: String,
    source: PackageSource,
) -> Result<(), String> {
    state.package_manager.remove(&name, source).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_package(
    state: State<'_, AppState>,
    name: String,
    source: PackageSource,
) -> Result<(), String> {
    state.package_manager.update(&name, source).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_packages(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Package>, String> {
    state.package_manager.search(&query).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_updates(
    state: State<'_, AppState>,
) -> Result<Vec<Package>, String> {
    state.package_manager.check_all_updates().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_config(
    state: State<'_, AppState>,
) -> Result<Config, String> {
    Ok(state.config.clone())
}

#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    state.config = config;
    state.config.save().map_err(|e| e.to_string())
}
```

---

### Phase 3: Frontend Architecture (Week 2-3)

**Goal**: Build modern React frontend

#### 3.1 Tech Stack
```
UI Framework:      React 18 + TypeScript
Build Tool:        Vite
Styling:           Tailwind CSS + shadcn/ui
State Management:  Zustand
Routing:           React Router 6
Icons:             Lucide React
Data Fetching:     TanStack Query (React Query)
Animations:        Framer Motion
```

#### 3.2 Project Structure (Frontend)
```
ui/src/
├── main.tsx              # Entry point
├── App.tsx               # Root component
├── routes.tsx            # Route definitions
├── components/
│   ├── layout/
│   │   ├── Layout.tsx
│   │   ├── Sidebar.tsx
│   │   ├── Header.tsx
│   │   └── SidebarItem.tsx
│   ├── common/
│   │   ├── Button.tsx
│   │   ├── Card.tsx
│   │   ├── Dialog.tsx
│   │   ├── Input.tsx
│   │   ├── Select.tsx
│   │   └── Loading.tsx
│   ├── packages/
│   │   ├── PackageCard.tsx
│   │   ├── PackageList.tsx
│   │   └── PackageBadge.tsx
│   └── sources/
│       └── SourceIcon.tsx
├── pages/
│   ├── Overview.tsx      # Dashboard
│   ├── Installed.tsx     # Installed packages
│   ├── Updates.tsx       # Available updates
│   ├── Browse.tsx        # Browse/search
│   └── Settings.tsx      # Settings
├── store/
│   ├── usePackageStore.ts
│   ├── useConfigStore.ts
│   └── useUIStore.ts
├── services/
│   ├── api.ts            # Tauri API client
│   └── packages.ts       # Package operations
├── hooks/
│   ├── usePackages.ts
│   ├── useUpdates.ts
│   └── useSources.ts
├── utils/
│   ├── format.ts         # Formatters
│   └── helpers.ts
└── styles/
    ├── globals.css
    └── tailwind.css
```

#### 3.3 State Management (Zustand)
```typescript
// ui/src/store/usePackageStore.ts

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api';
import { Package, PackageSource } from '../../types';

interface PackageState {
  packages: Package[];
  filteredPackages: Package[];
  selectedSource: PackageSource | null;
  searchQuery: string;
  isLoading: boolean;
  error: string | null;
  
  // Actions
  fetchPackages: (source?: PackageSource) => Promise<void>;
  setSelectedSource: (source: PackageSource | null) => void;
  setSearchQuery: (query: string) => void;
  installPackage: (name: string, source: PackageSource) => Promise<void>;
  removePackage: (name: string, source: PackageSource) => Promise<void>;
  updatePackage: (name: string, source: PackageSource) => Promise<void>;
  refresh: () => Promise<void>;
}

export const usePackageStore = create<PackageState>((set, get) => ({
  packages: [],
  filteredPackages: [],
  selectedSource: null,
  searchQuery: '',
  isLoading: false,
  error: null,
  
  fetchPackages: async (source) => {
    set({ isLoading: true, error: null });
    try {
      const packages = await invoke<Package[]>('get_packages', { source });
      set({ packages, filteredPackages: packages, isLoading: false });
    } catch (error) {
      set({ error: (error as Error).message, isLoading: false });
    }
  },
  
  setSelectedSource: (source) => {
    set({ selectedSource: source });
    get().setSearchQuery(get().searchQuery); // Trigger filter
  },
  
  setSearchQuery: (query) => {
    const { packages, selectedSource } = get();
    let filtered = packages;
    
    if (selectedSource) {
      filtered = filtered.filter(p => p.source === selectedSource);
    }
    
    if (query) {
      const lower = query.toLowerCase();
      filtered = filtered.filter(p => 
        p.name.toLowerCase().includes(lower) ||
        p.description?.toLowerCase().includes(lower)
      );
    }
    
    set({ searchQuery: query, filteredPackages: filtered });
  },
  
  installPackage: async (name, source) => {
    await invoke('install_package', { name, source });
    await get().fetchPackages();
  },
  
  removePackage: async (name, source) => {
    await invoke('remove_package', { name, source });
    await get().fetchPackages();
  },
  
  updatePackage: async (name, source) => {
    await invoke('update_package', { name, source });
    await get().fetchPackages();
  },
  
  refresh: async () => {
    await get().fetchPackages(get().selectedSource || undefined);
  },
}));
```

---

### Phase 4: UI Components (Week 3-4)

**Goal**: Build all UI components

#### 4.1 Layout Components
```typescript
// ui/src/components/layout/Layout.tsx

import { useState } from 'react';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { Outlet } from 'react-router-dom';

export function Layout() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  
  return (
    <div className="flex h-screen bg-gray-50">
      <Sidebar open={sidebarOpen} onToggle={() => setSidebarOpen(!sidebarOpen)} />
      <div className="flex-1 flex flex-col overflow-hidden">
        <Header onMenuClick={() => setSidebarOpen(!sidebarOpen)} />
        <main className="flex-1 overflow-auto p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
```

```typescript
// ui/src/components/layout/Sidebar.tsx

import { Package, RefreshCw, Search, Settings, Globe } from 'lucide-react';
import { SidebarItem } from './SidebarItem';
import { usePackageStore } from '../../store/usePackageStore';
import { PackageSource } from '../../types';

const SOURCES = [
  { source: PackageSource::Apt, label: 'APT', icon: '🟠' },
  { source: PackageSource::Flatpak, label: 'Flatpak', icon: '🟣' },
  { source: PackageSource::Snap, label: 'Snap', icon: '🟡' },
  { source: PackageSource::Npm, label: 'npm', icon: '🔴' },
  { source: PackageSource::Pip, label: 'pip', icon: '🔵' },
  { source: PackageSource::Cargo, label: 'Cargo', icon: '🟤' },
];

export function Sidebar({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const { selectedSource, setSelectedSource } = usePackageStore();
  
  return (
    <aside className={`${open ? 'w-64' : 'w-16'} transition-all duration-300 bg-white border-r border-gray-200 flex flex-col`}>
      <div className="p-4 border-b border-gray-200">
        <h1 className="text-xl font-bold text-gray-900">LinGet</h1>
      </div>
      
      <nav className="flex-1 py-4">
        <SidebarItem 
          icon={<Package size={20} />}
          label="Installed"
          to="/installed"
        />
        <SidebarItem 
          icon={<RefreshCw size={20} />}
          label="Updates"
          to="/updates"
        />
        <SidebarItem 
          icon={<Search size={20} />}
          label="Browse"
          to="/browse"
        />
        
        <div className="px-4 py-2 mt-4 mb-1">
          <span className="text-xs font-semibold text-gray-400 uppercase">Sources</span>
        </div>
        
        {SOURCES.map(({ source, label, icon }) => (
          <SidebarItem
            key={source}
            icon={<span>{icon}</span>}
            label={label}
            active={selectedSource === source}
            onClick={() => setSelectedSource(
              selectedSource === source ? null : source
            )}
          />
        ))}
      </nav>
      
      <div className="p-4 border-t border-gray-200">
        <SidebarItem
          icon={<Settings size={20} />}
          label="Settings"
          to="/settings"
        />
      </div>
    </aside>
  );
}
```

#### 4.2 Package Components
```typescript
// ui/src/components/packages/PackageCard.tsx

import { Package } from '../../types';
import { Badge } from '../common/Badge';
import { Install, Trash2, RefreshCw, ExternalLink } from 'lucide-react';
import { useState } from 'react';

interface PackageCardProps {
  package: Package;
  onInstall?: () => void;
  onRemove?: () => void;
  onUpdate?: () => void;
}

export function PackageCard({ pkg, onInstall, onRemove, onUpdate }: PackageCardProps) {
  const [showDetails, setShowDetails] = useState(false);
  
  const statusBadge = {
    [PackageStatus::Installed]: { color: 'green', label: 'Installed' },
    [PackageStatus::NotInstalled]: { color: 'gray', label: 'Available' },
    [PackageStatus::UpdateAvailable]: { color: 'yellow', label: 'Update' },
  }[pkg.status];
  
  return (
    <div className="bg-white rounded-xl border border-gray-200 p-4 hover:shadow-lg transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <h3 className="font-semibold text-gray-900">{pkg.name}</h3>
            <Badge color={statusBadge.color}>{statusBadge.label}</Badge>
          </div>
          <p className="text-sm text-gray-500 mt-1">{pkg.version}</p>
          {pkg.description && (
            <p className="text-sm text-gray-600 mt-2 line-clamp-2">{pkg.description}</p>
          )}
        </div>
        
        <div className="flex items-center gap-2">
          {pkg.status === 'update_available' && onUpdate && (
            <button
              onClick={onUpdate}
              className="p-2 text-blue-600 hover:bg-blue-50 rounded-lg"
              title="Update"
            >
              <RefreshCw size={18} />
            </button>
          )}
          {pkg.status === 'not_installed' && onInstall && (
            <button
              onClick={onInstall}
              className="p-2 text-green-600 hover:bg-green-50 rounded-lg"
              title="Install"
            >
              <Install size={18} />
            </button>
          )}
          {pkg.status === 'installed' && onRemove && (
            <button
              onClick={onRemove}
              className="p-2 text-red-600 hover:bg-red-50 rounded-lg"
              title="Remove"
            >
              <Trash2 size={18} />
            </button>
          )}
        </div>
      </div>
      
      {showDetails && (
        <div className="mt-4 pt-4 border-t border-gray-100">
          {/* Extended package info */}
        </div>
      )}
    </div>
  );
}
```

#### 4.3 Pages
```typescript
// ui/src/pages/Installed.tsx

import { useEffect } from 'react';
import { usePackageStore } from '../store/usePackageStore';
import { PackageCard } from '../components/packages/PackageCard';
import { Loading } from '../components/common/Loading';

export function Installed() {
  const { 
    filteredPackages, 
    isLoading, 
    fetchPackages, 
    removePackage 
  } = usePackageStore();
  
  useEffect(() => {
    fetchPackages();
  }, []);
  
  if (isLoading && filteredPackages.length === 0) {
    return <Loading />;
  }
  
  return (
    <div>
      <h2 className="text-2xl font-bold text-gray-900 mb-6">Installed Packages</h2>
      
      {filteredPackages.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-gray-500">No packages found</p>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredPackages.map((pkg) => (
            <PackageCard
              key={`${pkg.source}-${pkg.name}`}
              package={pkg}
              onRemove={() => removePackage(pkg.name, pkg.source)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
```

```typescript
// ui/src/pages/Updates.tsx

import { useEffect, useState } from 'react';
import { usePackageStore } from '../store/usePackageStore';
import { PackageCard } from '../components/packages/PackageCard';
import { Loading } from '../components/common/Loading';
import { invoke } from '@tauri-apps/api';
import { Button } from '../components/common/Button';

export function Updates() {
  const { 
    filteredPackages, 
    isLoading, 
    fetchPackages,
    updatePackage 
  } = usePackageStore();
  const [updating, setUpdating] = useState<string | null>(null);
  
  const updates = filteredPackages.filter(p => p.status === 'update_available');
  
  const handleUpdateAll = async () => {
    for (const pkg of updates) {
      setUpdating(`${pkg.source}-${pkg.name}`);
      await updatePackage(pkg.name, pkg.source);
    }
    setUpdating(null);
    fetchPackages();
  };
  
  if (isLoading && updates.length === 0) {
    return <Loading />;
  }
  
  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold text-gray-900">
          {updates.length} Updates Available
        </h2>
        {updates.length > 0 && (
          <Button onClick={handleUpdateAll} loading={updating !== null}>
            Update All
          </Button>
        )}
      </div>
      
      {updates.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-gray-500">Your system is up to date!</p>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {updates.map((pkg) => (
            <PackageCard
              key={`${pkg.source}-${pkg.name}`}
              package={pkg}
              onUpdate={() => updatePackage(pkg.name, pkg.source)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
```

---

### Phase 5: Features (Week 4-5)

**Goal**: Implement all features

#### 5.1 Search
```typescript
// ui/src/pages/Browse.tsx

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api';
import { Search, Loader2 } from 'lucide-react';
import { Package } from '../types';
import { PackageCard } from '../components/packages/PackageCard';

export function Browse() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Package[]>([]);
  const [loading, setLoading] = useState(false);
  const [searched, setSearched] = useState(false);
  
  const handleSearch = async () => {
    if (!query.trim()) return;
    
    setLoading(true);
    setSearched(true);
    try {
      const packages = await invoke<Package[]>('search_packages', { query });
      setResults(packages);
    } finally {
      setLoading(false);
    }
  };
  
  return (
    <div>
      <h2 className="text-2xl font-bold text-gray-900 mb-6">Browse Packages</h2>
      
      <div className="flex gap-2 mb-6">
        <div className="flex-1 relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" size={20} />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="Search packages..."
            className="w-full pl-10 pr-4 py-3 border border-gray-300 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>
        <button
          onClick={handleSearch}
          disabled={loading}
          className="px-6 py-3 bg-blue-600 text-white rounded-xl hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? <Loader2 className="animate-spin" size={20} /> : 'Search'}
        </button>
      </div>
      
      {searched && (
        <div>
          <p className="text-sm text-gray-500 mb-4">{results.length} results</p>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {results.map((pkg) => (
              <PackageCard key={`${pkg.source}-${pkg.name}`} package={pkg} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

#### 5.2 Settings Page
```typescript
// ui/src/pages/Settings.tsx

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api';
import { Config } from '../types';
import { Button } from '../components/common/Button';
import { Card } from '../components/common/Card';

export function Settings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [saving, setSaving] = useState(false);
  
  useEffect(() => {
    invoke<Config>('get_config').then(setConfig);
  }, []);
  
  const handleSave = async () => {
    setSaving(true);
    await invoke('update_config', { config });
    setSaving(false);
  };
  
  if (!config) return null;
  
  return (
    <div className="max-w-2xl">
      <h2 className="text-2xl font-bold text-gray-900 mb-6">Settings</h2>
      
      <Card className="mb-6">
        <h3 className="font-semibold text-gray-900 mb-4">Appearance</h3>
        <label className="flex items-center gap-3">
          <input
            type="checkbox"
            checked={config.theme.dark}
            onChange={(e) => setConfig({ ...config, theme: { ...config.theme, dark: e.target.checked } })}
            className="w-5 h-5 rounded"
          />
          <span>Dark mode</span>
        </label>
      </Card>
      
      <Card className="mb-6">
        <h3 className="font-semibold text-gray-900 mb-4">Package Sources</h3>
        <div className="space-y-2">
          {config.sources.map((source) => (
            <label key={source.name} className="flex items-center gap-3">
              <input
                type="checkbox"
                checked={source.enabled}
                onChange={(e) => {
                  const sources = config.sources.map(s =>
                    s.name === source.name ? { ...s, enabled: e.target.checked } : s
                  );
                  setConfig({ ...config, sources });
                }}
                className="w-5 h-5 rounded"
              />
              <span>{source.name}</span>
            </label>
          ))}
        </div>
      </Card>
      
      <Button onClick={handleSave} loading={saving}>
        Save Settings
      </Button>
    </div>
  );
}
```

---

### Phase 6: Polish (Week 5-6)

**Goal**: Polish UI/UX

#### 6.1 Animations (Framer Motion)
```typescript
// ui/src/components/common/PageTransition.tsx

import { motion } from 'framer-motion';

export function PageTransition({ children }: { children: React.ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.2 }}
    >
      {children}
    </motion.div>
  );
}
```

#### 6.2 Toast Notifications
```typescript
// ui/src/hooks/useToast.ts

import { create } from 'zustand';

interface Toast {
  id: string;
  type: 'success' | 'error' | 'info';
  message: string;
}

interface ToastState {
  toasts: Toast[];
  addToast: (type: Toast['type'], message: string) => void;
  removeToast: (id: string) => void;
}

export const useToast = create<ToastState>((set) => ({
  toasts: [],
  
  addToast: (type, message) => {
    const id = Math.random().toString(36).slice(2);
    set((state) => ({ toasts: [...state.toasts, { id, type, message }] }));
    setTimeout(() => {
      set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
    }, 5000);
  },
  
  removeToast: (id) => set((state) => ({
    toasts: state.toasts.filter((t) => t.id !== id)
  })),
}));
```

---

## File Manifest

### Rust Files to Create/Modify

```
linget-tauri/src/
├── main.rs                    # NEW: Tauri entry
├── api/
│   ├── mod.rs                 # NEW: API module
│   ├── packages.rs            # NEW: Package commands
│   ├── config.rs              # NEW: Config commands
│   └── sources.rs             # NEW: Source commands
├── window.rs                  # NEW: Window config
└── menu.rs                    # NEW: Menu config

packages/backend-core/
├── Cargo.toml                 # NEW
└── src/
    ├── lib.rs                 # NEW
    ├── traits.rs              # MIGRATED from linget-gtk
    ├── types.rs               # MIGRATED from models/
    ├── streaming.rs           # MIGRATED from backend/
    ├── pkexec.rs              # MIGRATED from backend/
    └── backends/
        ├── mod.rs             # NEW
        ├── apt.rs             # MIGRATED
        ├── flatpak.rs         # MIGRATED
        ├── snap.rs            # MIGRATED
        ├── npm.rs             # MIGRATED
        ├── pip.rs             # MIGRATED
        ├── cargo.rs           # MIGRATED
        ├── brew.rs            # MIGRATED
        ├── dnf.rs             # MIGRATED
        ├── zypper.rs          # MIGRATED
        ├── pacman.rs          # MIGRATED
        ├── aur.rs             # MIGRATED
        ├── deb.rs             # MIGRATED
        ├── dart.rs            # MIGRATED
        ├── guix.rs            # MIGRATED
        ├── nix.rs             # MIGRATED
        ├── xbps.rs            # MIGRATED
        ├── eopkg.rs           # MIGRATED
        └── appimage.rs        # MIGRATED
```

### Frontend Files to Create

```
ui/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── src/
    ├── main.tsx
    ├── App.tsx
    ├── routes.tsx
    ├── types.ts               # Shared types
    ├── components/
    │   ├── layout/
    │   │   ├── Layout.tsx
    │   │   ├── Sidebar.tsx
    │   │   ├── Header.tsx
    │   │   └── SidebarItem.tsx
    │   ├── common/
    │   │   ├── Button.tsx
    │   │   ├── Card.tsx
    │   │   ├── Dialog.tsx
    │   │   ├── Input.tsx
    │   │   ├── Select.tsx
    │   │   ├── Badge.tsx
    │   │   ├── Loading.tsx
    │   │   └── Toast.tsx
    │   ├── packages/
    │   │   ├── PackageCard.tsx
    │   │   ├── PackageList.tsx
    │   │   └── PackageBadge.tsx
    │   └── sources/
    │       └── SourceIcon.tsx
    ├── pages/
    │   ├── Overview.tsx
    │   ├── Installed.tsx
    │   ├── Updates.tsx
    │   ├── Browse.tsx
    │   └── Settings.tsx
    ├── store/
    │   ├── usePackageStore.ts
    │   ├── useConfigStore.ts
    │   └── useUIStore.ts
    ├── services/
    │   ├── api.ts
    │   └── packages.ts
    ├── hooks/
    │   ├── usePackages.ts
    │   ├── useUpdates.ts
    │   └── useToast.ts
    ├── utils/
    │   ├── format.ts
    │   └── helpers.ts
    └── styles/
        ├── globals.css
        └── tailwind.css
```

---

## Effort Estimate

| Phase       | Effort   | Files | Key Outputs              |
|-------------|----------|-------|--------------------------|
| Phase 1     | 1 week   | 15    | backend-core crate       |
| Phase 2     | 1 week   | 10    | Tauri setup + API layer  |
| Phase 3     | 1 week   | 20    | Frontend architecture    |
| Phase 4     | 1.5 week | 25    | All UI components        |
| Phase 5     | 1 week   | 15    | All features             |
| Phase 6     | 1 week   | 10    | Polish + animations      |
| **Total**   | **6.5 weeks** | **~100 files** | **Production-ready** |

---

## Installation & Build Commands

```bash
# Development
cd linget-tauri/ui && npm install
cd linget-tauri && cargo tauri dev

# Production build
cd linget-tauri/ui && npm run build
cd linget-tauri && cargo tauri build

# Build Debian package
cargo tauri build --bundles deb
```

---

## Dependencies

### Rust Dependencies
```
# Cargo.toml (workspace)
[workspace]
members = [
    "linget-gtk",
    "linget-tauri",
    "packages/backend-core",
    "packages/models",
    "packages/cli",
]

[patch.crates-io]
tauri = { git = "https://github.com/tauri-apps/tauri", branch = "v1" }
```

### Frontend Dependencies
```json
// ui/package.json
{
  "dependencies": {
    "react": "^18.2",
    "react-dom": "^18.2",
    "react-router-dom": "^6.20",
    "zustand": "^4.4",
    "@tanstack/react-query": "^5.8",
    "framer-motion": "^10.16",
    "lucide-react": "^0.294",
    "clsx": "^2.0",
    "tailwind-merge": "^2.1"
  },
  "devDependencies": {
    "@types/react": "^18.2",
    "@types/react-dom": "^18.2",
    "@vitejs/plugin-react": "^4.2",
    "autoprefixer": "^10.4",
    "postcss": "^8.4",
    "tailwindcss": "^3.3",
    "typescript": "^5.3",
    "vite": "^5.0"
  }
}
```

---

## Migration Checklist

- [ ] Create workspace Cargo.toml
- [ ] Extract backend-core crate
- [ ] Extract models crate
- [ ] Migrate all 17 backends to backend-core
- [ ] Update linget-gtk to use shared crates
- [ ] Set up linget-tauri project
- [ ] Create Tauri API layer
- [ ] Build React frontend architecture
- [ ] Implement all UI components
- [ ] Implement all pages
- [ ] Add animations and polish
- [ ] Test all features
- [ ] Build Debian package
- [ ] Create installation instructions
- [ ] Update README with both versions

---

## Next Steps

1. **Start Phase 1**: Extract backend-core crate
2. **Decide**: Which version gets the `linget` CLI name? (GTK stays, Tauri becomes `linget-v2` or `linget-modern`?)
3. **Prototype**: Create minimal Tauri app with one backend to validate approach

Ready to begin implementation?
