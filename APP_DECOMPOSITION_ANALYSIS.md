# LinGet Tauri UI - App.tsx Decomposition Analysis

## Current State Summary
- **File**: `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/App.tsx`
- **Size**: 1,830 lines (64 KB)
- **Architecture**: Monolithic single-component structure
- **Status**: All UI logic, state management, and event handling mixed into AppContent component

---

## Section 1: Current Responsibilities & State Domains

### AppContent Component (Lines 1240-1813)
Primary responsibilities:
1. **Package Management** - Load, filter, search packages
2. **UI State** - Modal visibility, tabs, dialogs
3. **Settings Management** - Dark mode, source toggle, refresh settings
4. **Event System** - Listen to Tauri events (operation progress, logs)
5. **Navigation** - Route handling with React Router
6. **Operations Tracking** - Monitor install/remove/update tasks
7. **Toast Notifications** - Display user feedback

### State Domains (14 useState hooks in AppContent)

| Domain | State Variables | Lines | Purpose |
|--------|-----------------|-------|---------|
| **Package Data** | `packages`, `loading` | 1243-1244 | Main package list & loading state |
| **UI Visibility** | `showShortcuts`, `showAbout`, `showTaskHub` | 1246-1248 | Modal/dialog toggles |
| **Sources** | `sources`, `settings` | 1250-1256 | Package sources + associated settings |
| **Confirmation** | `confirmDialog` | 1258-1262 | Package action confirmation modal |
| **Bulk Operations** | `updatingAll`, `updateProgress` | 1264-1265 | Batch update tracking |
| **Operations** | `runningOperations` | 1267 | Active install/remove/update tasks |
| **Notifications** | `toasts` | 1245 | Toast messages |

---

## Section 2: Page/View Sections

### Four Page Components (All Defined in Same File)

#### 1. **InstalledPage** (Lines 772-870)
**Responsibilities**:
- Display installed packages grid
- Source filtering (selector buttons)
- Package counts per source
- Empty state handling

**Props**:
```typescript
packages: Package[]
loading: boolean
onRequestConfirm: (action, pkg) => void
loadPackages: () => void
sources: SourceInfo[]
```

**Internal State**:
- `selectedSource: string` - Active filter

**Reused Components**:
- `PackageCard` (repeated)
- `PageTransition`
- `SkeletonCard`
- `EmptyState`

---

#### 2. **UpdatesPage** (Lines 872-950)
**Responsibilities**:
- Display packages with available updates
- "Update All" button with progress
- Empty state when all up-to-date

**Props**:
```typescript
packages: Package[]
loading: boolean
onRequestConfirm: (action, pkg) => void
onUpdateAll: () => void
updatingAll: boolean
updateProgress: number
```

**Reused Components**:
- `PackageCard` (repeated)
- `PageTransition`
- `SkeletonList`
- `EmptyState`

---

#### 3. **BrowsePage** (Lines 953-1060)
**Responsibilities**:
- Search input field
- Invoke backend search via Tauri
- Keyboard shortcut handling (/)
- Search results display

**Props**:
```typescript
loading: boolean
onRequestConfirm: (action, pkg) => void
```

**Internal State**:
- `searchQuery: string`
- `results: Package[]`
- `searched: boolean`
- `isSearching: boolean`
- `inputRef: RefObject<HTMLInputElement>`

**Event Handlers**:
- `handleSearch` - Async search invocation
- Global keydown listener for "/" focus

**Reused Components**:
- `PackageCard` (repeated)
- `PageTransition`
- `SkeletonList`
- `EmptyState`

---

#### 4. **SettingsPage** (Lines 1063-1238)
**Responsibilities**:
- Dark mode toggle
- Source enable/disable toggles
- Auto-refresh settings
- Refresh interval selector
- Shortcuts display
- About button

**Props**:
```typescript
settings: SettingsData
onSettingsChange: (updates) => void
onSave: () => void
sources: SourceInfo[]
onToggleSource: (id) => void
onShowAbout: () => void
```

**No Internal State** (all lifted to AppContent)

---

## Section 3: Reusable UI Chunks / Shared Components

### Layout Components

#### **PageTransition** (Lines 121-133)
```typescript
Props: { children: React.ReactNode }
Purpose: Framer Motion wrapper for page animations
Used: By all 4 page components
```
**Exact Code Region**: Lines 121-133
**Risk**: LOW - Pure presentation, no logic

---

#### **EmptyState** (Lines 134-160)
```typescript
Props: {
  icon: React.ElementType
  title: string
  description: string
  action?: React.ReactNode
}
Purpose: Consistent empty state UI across pages
Used: By InstalledPage, UpdatesPage, BrowsePage
```
**Exact Code Region**: Lines 134-160
**Risk**: LOW - Pure UI, no logic

---

### Loading Components

#### **SkeletonCard** (Lines 161-174)
```typescript
Purpose: Grid-based loading skeleton (3-column layout)
Used: By InstalledPage
```
**Exact Code Region**: Lines 161-174
**Risk**: LOW - Pure UI, no state/logic

---

#### **SkeletonList** (Lines 175-190)
```typescript
Purpose: List-based loading skeleton (rows)
Used: By UpdatesPage, BrowsePage
```
**Exact Code Region**: Lines 175-190
**Risk**: LOW - Pure UI, no state/logic

---

### Modal/Dialog Components

#### **ConfirmDialog** (Lines 202-267)
```typescript
Props: {
  isOpen: boolean
  title: string
  message: string
  confirmText: string
  cancelText: string
  confirmStyle: 'primary' | 'danger'
  onConfirm: () => void
  onCancel: () => void
}
Purpose: Generic confirmation dialog
Used: AppContent (1806-1811)
```
**Exact Code Region**: Lines 202-267
**Risk**: VERY LOW - Pure presentation

---

#### **TaskHub** (Lines 269-362)
```typescript
Props: {
  operations: RunningOperation[]
  onCancelOperation: (id: string) => void
}
Purpose: Display running operations with logs
State: Handles auto-scroll via useRef
```
**Exact Code Region**: Lines 269-362
**Risk**: VERY LOW - Only manages internal scroll

---

#### **ShortcutsModal** (Lines 683-722)
```typescript
Props: { onClose: () => void }
Purpose: Display keyboard shortcuts list
```
**Exact Code Region**: Lines 683-722
**Risk**: LOW - Pure UI with SHORTCUTS constant

---

#### **AboutModal** (Lines 723-771)
```typescript
Props: { onClose: () => void }
Purpose: Display about/version info
```
**Exact Code Region**: Lines 723-771
**Risk**: LOW - Pure UI

---

### Card Component

#### **PackageCard** (Lines 369-647)
```typescript
Props: {
  pkg: Package
  onRequestConfirm: (action, pkg) => void
}
State: showDetails (modal visibility)
```
**Exact Code Region**: Lines 369-647 (279 lines!)
**Risk**: MEDIUM - Has internal state + calls parent callback
**Subcomponents**: Detail modal, action buttons
**Details Modal**: Lines 465-640 (inline JSX for package details)

---

### Toast System

#### **ToastContainer** (Lines 648-682)
```typescript
Props: {
  toasts: Toast[]
  onDismiss: (id: number) => void
}
Purpose: Display/manage toast notifications
```
**Exact Code Region**: Lines 648-682
**Risk**: LOW - Pure UI with callback

---

---

## Section 4: Data Types & Interfaces (Lines 16-87)

Located at top of file:
```typescript
interface Package {}           // Lines 16-30
interface Toast {}              // Lines 31-35
interface OperationLog {}       // Lines 37-43
interface RunningOperation {}   // Lines 45-54
interface OperationProgressPayload {}  // Lines 56-62
interface OperationLogPayload {}       // Lines 64-70
interface BatchUpdateProgressPayload {} // Lines 71-75
interface SourceInfo {}         // Lines 77-85
interface SettingsData {}       // Lines 87-92

const DEFAULT_SOURCES = [...]   // Lines 94-109
const SHORTCUTS = [...]         // Lines 110-119
```

**Extraction Plan**: Move to `src/types/index.ts`

---

## Section 5: Service/API Functions in AppContent (Lines 1277-1551)

### **loadSources()** (Lines 1277-1312)
- Invokes: `get_backend_sources`
- Updates: `sources` state
- Merges backend data with local defaults

**Extraction**: `src/hooks/usePackageSources.ts`

---

### **loadSettings()** (Lines 1314-1334)
- Invokes: `load_settings`
- Updates: `settings` state
- Syncs enabled sources

**Extraction**: `src/hooks/useSettings.ts`

---

### **loadPackages()** (Lines 1336-1355)
- Invokes: `list_installed_packages` OR `check_updates`
- Updates: `packages` state
- Tab-dependent logic

**Extraction**: `src/hooks/usePackageList.ts`

---

### **handleAction()** (Lines 1357-1374)
- Install/Remove/Update action dispatch
- Invokes: `install_package`, `remove_package`, `update_package`
- Shows toast + reloads packages

**Extraction**: `src/services/packageActions.ts`

---

### **handleUpdateAll()** (Lines 1376-1393)
- Batch update orchestration
- Progress tracking
- Toast feedback

**Extraction**: `src/services/packageActions.ts` (same as above)

---

### **saveSettings()** (Lines 1523-1531)
- Invokes: `save_settings`
- Error handling + toast

**Extraction**: `src/hooks/useSettings.ts`

---

### **toggleSource()** (Lines 1533-1548)
- Updates both `sources` + `settings.enabled_sources`
- Validates source availability

**Extraction**: `src/hooks/useSettings.ts`

---

### **cancelOperation()** (Lines 1550-1558)
- Invokes: `cancel_operation`
- Removes from UI

**Extraction**: `src/services/operationService.ts`

---

### **getConfirmConfig()** (Lines 1560-1589)
- Maps action → dialog text + style
- Pure function

**Extraction**: `src/utils/confirmDialogConfig.ts`

---

## Section 6: Event Listeners & Effects (Lines 1404-1506)

### **Effect 1: Initialize** (Lines 1404-1407)
```typescript
useEffect(() => {
  loadSources()
  loadSettings()
}, [])
```
**Purpose**: Load sources & settings on mount
**Extraction**: No change (in AppContent), consolidate loads

---

### **Effect 2: Keyboard Shortcuts** (Lines 1409-1426)
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // Handle ?, t, Escape
  }
  window.addEventListener('keydown', handleKeyDown)
  return () => window.removeEventListener('keydown', handleKeyDown)
}, [])
```
**Purpose**: Global keyboard shortcuts
**Extraction**: `src/hooks/useGlobalKeyboardShortcuts.ts`

---

### **Effect 3: Tauri Event Listeners** (Lines 1428-1500)
```typescript
useEffect(() => {
  Promise.all([
    listen<OperationProgressPayload>('operation-progress', ...),
    listen<OperationLogPayload>('operation-log', ...),
    listen<BatchUpdateProgressPayload>('batch-update-progress', ...),
    listen('batch-update-completed', ...)
  ])
  return () => cleanup
}, [])
```
**Purpose**: Subscribe to backend events
**Extraction**: `src/hooks/useTauriEventListeners.ts`

---

### **Effect 4: Page Load** (Lines 1502-1506)
```typescript
useEffect(() => {
  if (currentPage !== 'settings') {
    loadPackages(currentPage as 'installed' | 'updates' | 'browse')
  }
}, [currentPage])
```
**Purpose**: Reload packages when page changes
**No extraction** - Keep in AppContent (coordinates multiple hooks)

---

## Section 7: Sidebar Navigation (Lines 1593-1681)

**Hardcoded in AppContent JSX** (Lines 1601-1656)
- 4 nav buttons (installed, updates, browse, settings)
- Task Hub button
- Shortcuts button
- Active tab styling
- Package counts

**Extraction Plan**: `src/components/Sidebar.tsx`

---

## Section 8: Header/Toolbar (Lines 1683-1715)

**Hardcoded in AppContent JSX** (Lines 1684-1715)
- Page title
- Subtitle (counts, hints)
- Refresh button
- Conditional rendering based on activeTab

**Extraction Plan**: `src/components/Header.tsx`

---

---

# DECOMPOSITION PLAN: CLEANEST EXTRACTION PATH

## Phase 1: LOW-RISK EXTRACTIONS (Week 1)

### 1.1 Extract Types → `src/types/index.ts`
**Files**: All interfaces (16 items)
**Lines**: 16-119 (constants too)
**Behavior Preserved**: ✅ 100% - No logic
**Risk**: ⚠️ VERY LOW

**Actions**:
```bash
# Create src/types/index.ts with:
# - Package, Toast, OperationLog, RunningOperation
# - OperationProgressPayload, OperationLogPayload, BatchUpdateProgressPayload
# - SourceInfo, SettingsData
# - DEFAULT_SOURCES, SHORTCUTS constants

# Update App.tsx imports at line 1
```

**Verification**: TypeScript compilation only

---

### 1.2 Extract Utilities → `src/utils/confirmDialogConfig.ts`
**Function**: `getConfirmConfig()`
**Lines**: 1560-1589
**Dependencies**: `confirmDialog` state from AppContent
**Behavior**: Maps action string → dialog config
**Risk**: ⚠️ VERY LOW

**After Extraction**:
```typescript
// src/utils/confirmDialogConfig.ts
export function getConfirmConfig(action: string, pkg: Package | null) {
  // ... pure function
}

// In AppContent:
getConfirmConfig(confirmDialog.action, confirmDialog.pkg)
```

**Verification**: Check all 3 cases (install/update/remove) work

---

### 1.3 Extract Layout Components → `src/components/shared/`
**Components**:
- `PageTransition` → `src/components/shared/PageTransition.tsx` (lines 121-133)
- `EmptyState` → `src/components/shared/EmptyState.tsx` (lines 134-160)
- `SkeletonCard` → `src/components/shared/SkeletonCard.tsx` (lines 161-174)
- `SkeletonList` → `src/components/shared/SkeletonList.tsx` (lines 175-190)

**Behavior**: ✅ 100% preserved - Pure UI
**Risk**: ⚠️ VERY LOW

**Verification**:
- InstalledPage still shows loading state
- UpdatesPage & BrowsePage show skeletons
- All pages transition smoothly

---

### 1.4 Extract Modal Components → `src/components/modals/`
**Components**:
- `ConfirmDialog` → `src/components/modals/ConfirmDialog.tsx` (lines 202-267)
- `ShortcutsModal` → `src/components/modals/ShortcutsModal.tsx` (lines 683-722)
- `AboutModal` → `src/components/modals/AboutModal.tsx` (lines 723-771)

**Behavior**: ✅ 100% preserved - Pure UI
**Risk**: ⚠️ VERY LOW

**Verification**: 
- Modals open/close
- Dialog callbacks still work
- Keyboard handling in ShortcutsModal

---

### 1.5 Extract Toast System → `src/components/ToastContainer.tsx`
**Component**: `ToastContainer` (lines 648-682)
**Behavior**: Pure UI presentation
**Risk**: ⚠️ VERY LOW

**Verification**: Toasts appear/disappear on timers

---

### 1.6 Extract TaskHub → `src/components/TaskHub.tsx`
**Component**: `TaskHub` (lines 269-362)
**State**: Internal `useRef` for scroll
**Behavior**: Auto-scroll on operations change ✅
**Risk**: ⚠️ VERY LOW

**Verification**:
- Operations appear in list
- Scroll to bottom on new ops
- Progress bars animate
- Logs display

---

## Phase 2: MEDIUM-RISK EXTRACTIONS (Week 2)

### 2.1 Extract Pages → `src/pages/`

#### **InstalledPage** → `src/pages/InstalledPage.tsx`
**Lines**: 772-870
**State**: `selectedSource` (stays internal)
**Behavior**: ✅ Preserved
**Risk**: 🟡 LOW-MEDIUM

**Files**:
```
src/pages/InstalledPage.tsx
```

**Verification**:
- Filter by source works
- Counts display correctly
- Empty state shows
- Refresh loads packages

---

#### **UpdatesPage** → `src/pages/UpdatesPage.tsx`
**Lines**: 872-950
**Behavior**: ✅ Preserved
**Risk**: 🟡 LOW-MEDIUM

**Verification**:
- Update All button works
- Progress displays
- Up-to-date message shows

---

#### **BrowsePage** → `src/pages/BrowsePage.tsx`
**Lines**: 953-1060
**State**: `searchQuery`, `results`, `searched`, `isSearching`
**Effect**: Global "/" keyboard shortcut (stays internal)
**Risk**: 🟡 LOW-MEDIUM

**Verification**:
- Search input works
- Enter/button triggers search
- "/" focus works
- Results display

---

#### **SettingsPage** → `src/pages/SettingsPage.tsx`
**Lines**: 1063-1238
**Behavior**: ✅ Preserved (all callbacks passed from AppContent)
**Risk**: ⚠️ VERY LOW - No internal state

**Verification**:
- Toggles work
- Select dropdowns work
- About button fires callback

---

### 2.2 Extract Layout Components → `src/components/layout/`

#### **Sidebar** → `src/components/layout/Sidebar.tsx`
**Extracted from**: Lines 1593-1681
**Dependencies**:
- `activeTab` (string)
- `navigate` (useNavigate hook)
- `updateCount`, `installedCount` (numbers)
- `runningOperations.length` (number)
- `setShowShortcuts`, `setShowTaskHub` (callbacks)

**Risk**: 🟡 MEDIUM

```typescript
interface SidebarProps {
  activeTab: string
  navigate: NavigateFunction
  updateCount: number
  installedCount: number
  runningOperationsCount: number
  onShowShortcuts: () => void
  onShowTaskHub: () => void
}
```

**Verification**:
- Nav buttons highlight correctly
- Click navigation works
- Counts display
- Task Hub badge shows

---

#### **Header** → `src/components/layout/Header.tsx`
**Extracted from**: Lines 1683-1715
**Dependencies**:
- `activeTab` (string)
- `updateCount`, `installedCount` (numbers)
- `loading` (boolean)
- `onRefresh` (callback)

**Risk**: ⚠️ LOW

```typescript
interface HeaderProps {
  activeTab: string
  updateCount: number
  installedCount: number
  loading: boolean
  onRefresh: () => void
}
```

**Verification**:
- Title changes per tab
- Subtitle shows correct counts
- Refresh button works

---

### 2.3 Extract PackageCard → `src/components/PackageCard.tsx`
**Lines**: 369-647
**State**: `showDetails` (internal modal)
**Complexity**: ⚠️ HIGHEST - 279 lines with detail modal
**Risk**: 🔴 MEDIUM-HIGH

**Sub-components**:
- Detail modal UI (lines 465-640)
- Action buttons
- Status badges
- Metadata display

**Extraction Strategy**:
1. Extract detail modal inner content → `src/components/PackageDetailsModal.tsx`
2. Keep `showDetails` state in PackageCard
3. Extract button styles → utilities

**Verification**:
- Card renders correctly
- Click opens detail modal
- Action buttons work
- Status badges display
- Close modal works

---

## Phase 3: ARCHITECTURE CHANGES (Week 3)

### 3.1 Create Service Layer → `src/services/`

#### `src/services/tauri.ts`
**Consolidates**: All `invoke()` calls
```typescript
export async function installPackage(name: string, source: string)
export async function removePackage(name: string, source: string)
export async function updatePackage(name: string, source: string)
export async function updateAllPackages()
export async function searchPackages(query: string): Promise<Package[]>
export async function listInstalledPackages(): Promise<Package[]>
export async function checkUpdates(): Promise<Package[]>
export async function getBackendSources()
export async function loadSettings()
export async function saveSettings(settings: SettingsData)
export async function cancelOperation(id: string)
```

**Files to Update**:
- AppContent
- BrowsePage
- (Any extracted functions)

**Risk**: 🟡 MEDIUM - Single point of failure for API

---

### 3.2 Create Custom Hooks → `src/hooks/`

#### `src/hooks/usePackageList.ts`
```typescript
export function usePackageList() {
  const [packages, setPackages] = useState<Package[]>([])
  const [loading, setLoading] = useState(true)
  
  const loadPackages = useCallback(async (tab: 'installed' | 'updates') => {
    // ... logic from lines 1336-1355
  }, [])
  
  return { packages, loading, loadPackages }
}
```

**Usage in AppContent**: Reduces 2 useState + loadPackages function

---

#### `src/hooks/useSettings.ts`
```typescript
export function useSettings() {
  const [settings, setSettings] = useState<SettingsData>(DEFAULT_SETTINGS)
  
  const loadSettings = useCallback(async () => { ... }, [])
  const saveSettings = useCallback(async () => { ... }, [])
  const toggleSource = useCallback((id: string) => { ... }, [])
  
  return { settings, loadSettings, saveSettings, toggleSource }
}
```

**Usage in AppContent**: Reduces 1 useState + 3 functions

---

#### `src/hooks/useSources.ts`
```typescript
export function useSources() {
  const [sources, setSources] = useState<SourceInfo[]>(DEFAULT_SOURCES)
  
  const loadSources = useCallback(async () => {
    // ... logic from lines 1277-1312
  }, [])
  
  return { sources, loadSources }
}
```

---

#### `src/hooks/useToastNotifications.ts`
```typescript
export function useToastNotifications() {
  const [toasts, setToasts] = useState<Toast[]>([])
  
  const showToast = useCallback((type: Toast['type'], message: string) => {
    // ... logic from lines 1269-1275
  }, [])
  
  const dismissToast = useCallback((id: number) => {
    setToasts(prev => prev.filter(t => t.id !== id))
  }, [])
  
  return { toasts, showToast, dismissToast }
}
```

---

#### `src/hooks/useTauriEventListeners.ts`
```typescript
export function useTauriEventListeners(
  onOperationProgress: (data: OperationProgressPayload) => void,
  onOperationLog: (data: OperationLogPayload) => void,
  onBatchUpdateProgress: (data: BatchUpdateProgressPayload) => void,
  onBatchUpdateCompleted: () => void
) {
  useEffect(() => {
    // ... logic from lines 1428-1500
  }, [onOperationProgress, onOperationLog, onBatchUpdateProgress, onBatchUpdateCompleted])
}
```

---

#### `src/hooks/useGlobalKeyboardShortcuts.ts`
```typescript
export function useGlobalKeyboardShortcuts(
  onShowShortcuts: () => void,
  onToggleTaskHub: (open: boolean) => void
) {
  useEffect(() => {
    // ... logic from lines 1409-1426
  }, [onShowShortcuts, onToggleTaskHub])
}
```

---

#### `src/hooks/useRunningOperations.ts`
```typescript
export function useRunningOperations() {
  const [runningOperations, setRunningOperations] = useState<RunningOperation[]>([])
  
  const addOperation = useCallback((op: RunningOperation) => { ... }, [])
  const updateOperation = useCallback((id: string, updates: Partial<RunningOperation>) => { ... }, [])
  const removeOperation = useCallback((id: string) => { ... }, [])
  const cancelOperation = useCallback(async (id: string) => { ... }, [])
  
  return { runningOperations, addOperation, updateOperation, removeOperation, cancelOperation }
}
```

---

### 3.3 Final AppContent (Simplified)
**New AppContent will**:
1. Use 6 custom hooks (packages, settings, sources, toasts, operations, shortcuts)
2. Handle event listener setup
3. Manage modal visibility (3 useState)
4. Render layout + route pages

**New Size**: ~250-300 lines (vs 573 before extraction)

---

---

# EXTRACTION ROADMAP: RISK & IMPACT

| Phase | Task | Lines | Risk | Impact | Time |
|-------|------|-------|------|--------|------|
| **1** | Types → `src/types/index.ts` | 16-119 | ⚠️ VERY LOW | Enables other extractions | 30m |
| **1** | Utils → `src/utils/confirmDialogConfig.ts` | 1560-1589 | ⚠️ VERY LOW | Cleaner AppContent | 20m |
| **1** | Shared UI → `src/components/shared/` | 121-190 | ⚠️ VERY LOW | Reuse across pages | 45m |
| **1** | Modals → `src/components/modals/` | 202-771 | ⚠️ VERY LOW | Cleaner AppContent | 45m |
| **1** | Toast → `src/components/ToastContainer.tsx` | 648-682 | ⚠️ VERY LOW | Isolates notification logic | 15m |
| **1** | TaskHub → `src/components/TaskHub.tsx` | 269-362 | ⚠️ VERY LOW | Monitoring UI separation | 20m |
| **2** | Pages → `src/pages/` (4 files) | 772-1238 | 🟡 LOW-MEDIUM | Core feature separation | 2h |
| **2** | Layout → `src/components/layout/` | 1593-1715 | 🟡 MEDIUM | Navigation architecture | 1h |
| **2** | PackageCard → `src/components/PackageCard.tsx` | 369-647 | 🔴 MEDIUM-HIGH | Reusable card system | 1.5h |
| **3** | Services → `src/services/tauri.ts` | All invokes | 🟡 MEDIUM | API encapsulation | 1h |
| **3** | Hooks → `src/hooks/` (6 files) | All logic | 🟡 MEDIUM | State management | 3h |

**Total Time**: ~10 hours
**Total LOC Reduction**: 1,830 → 250-300 (82% reduction in AppContent)

---

# LOW-RISK FIRST STEPS (RECOMMENDED ORDER)

## Step 1: Extract Types (30 min)
Create `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/types/index.ts`
- Copy interfaces 16-119
- Update App.tsx import (line 1)
- Verify: `npm run build`

---

## Step 2: Extract Confirm Dialog Config (20 min)
Create `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/utils/confirmDialogConfig.ts`
- Copy `getConfirmConfig()` logic
- Change to pure function: `(action, pkg) => config`
- Update App.tsx call (line 1808)
- Verify: All 3 dialog types work (install/update/remove)

---

## Step 3: Extract Shared UI Components (45 min)
Create:
- `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/shared/PageTransition.tsx`
- `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/shared/EmptyState.tsx`
- `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/shared/SkeletonCard.tsx`
- `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/shared/SkeletonList.tsx`

**Verify**:
- InstalledPage loads + shows cards
- UpdatesPage loads + shows list
- BrowsePage loading state
- All pages transition smoothly

---

## Step 4: Extract Modal Components (45 min)
Create:
- `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/modals/ConfirmDialog.tsx`
- `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/modals/ShortcutsModal.tsx`
- `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/modals/AboutModal.tsx`

**Verify**:
- Open confirmation → confirm → closes ✅
- Open shortcuts → close ✅
- Open about → close ✅

---

## Step 5: Extract Toast System (15 min)
Create `/home/eslam/Storage/Code/LinGet/linget-tauri/ui/src/components/ToastContainer.tsx`

**Verify**:
- Success/error/warning toast appears
- Auto-dismisses after 4 seconds
- Manual dismiss works

---

## After Step 5: App.tsx Size Reduction
- Current: 1,830 lines
- After Phase 1: ~1,300 lines (29% reduction)
- Safe to run tests + deploy

---

# BEHAVIOR PRESERVATION CHECKLIST

After each extraction, verify:

### UI Behavior
- [ ] All pages render
- [ ] All modals open/close
- [ ] All buttons work
- [ ] All loading states show

### State Management
- [ ] Package list loads on tab change
- [ ] Settings persist
- [ ] Sources toggle correctly
- [ ] Toasts appear + dismiss

### Events
- [ ] Keyboard shortcuts work (?, t, /, Escape)
- [ ] Package operations complete
- [ ] Task Hub updates in real-time
- [ ] Progress bars animate

### Navigation
- [ ] Tab navigation works
- [ ] URL updates correctly
- [ ] Back button works

---

# FINAL ARCHITECTURE (After Phase 3)

```
src/
├── types/
│   └── index.ts          # All interfaces & constants
├── utils/
│   └── confirmDialogConfig.ts
├── services/
│   └── tauri.ts          # All Tauri invoke() calls
├── hooks/
│   ├── usePackageList.ts
│   ├── useSettings.ts
│   ├── useSources.ts
│   ├── useToastNotifications.ts
│   ├── useTauriEventListeners.ts
│   ├── useGlobalKeyboardShortcuts.ts
│   └── useRunningOperations.ts
├── components/
│   ├── shared/
│   │   ├── PageTransition.tsx
│   │   ├── EmptyState.tsx
│   │   ├── SkeletonCard.tsx
│   │   └── SkeletonList.tsx
│   ├── modals/
│   │   ├── ConfirmDialog.tsx
│   │   ├── ShortcutsModal.tsx
│   │   └── AboutModal.tsx
│   ├── layout/
│   │   ├── Sidebar.tsx
│   │   └── Header.tsx
│   ├── TaskHub.tsx
│   ├── ToastContainer.tsx
│   └── PackageCard.tsx
├── pages/
│   ├── InstalledPage.tsx
│   ├── UpdatesPage.tsx
│   ├── BrowsePage.tsx
│   └── SettingsPage.tsx
├── App.tsx               # ~250 lines (orchestration only)
└── main.tsx
```

