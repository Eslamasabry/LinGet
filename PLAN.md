# LinGet Roadmap

## Vision
Make LinGet the **unforgettable** Linux package manager—the one users recommend to friends, the one that makes system management feel like a superpower rather than a chore.

---

## Phase 1: Storage & Cleanup ✅ COMPLETED

### 1.1 Storage Analyzer View ✅
- New "Storage" nav item in sidebar
- Breakdown by source (bar chart): APT 12GB, Flatpak 8GB, npm 2GB...
- Top 20 largest packages list with size
- Sort packages by size in Library view

### 1.2 Cleanup Tools ✅
- "Cleanup" section in Storage view
- Detect orphaned packages per source
- Cache sizes per source (apt, flatpak, snap, dnf, pacman, pip, npm)
- One-click cleanup with confirmation dialog
- Shows exact items that will be deleted before action
- Space recoverable preview

### 1.3 Duplicate Detection
**Problem**: Same app installed from multiple sources (Firefox from apt AND flatpak).

**Features**:
- Detect duplicate app names across sources
- Suggest keeping one, removing others
- Show size comparison between duplicates

---

## Phase 2: System Health Dashboard ✅ COMPLETED

### 2.1 Health Score & Dashboard ✅
- Health view accessible from sidebar navigation
- Single health score (0-100) with visual indicator
- Actionable issue cards with severity levels (Critical/Warning/Good)
- Issues include: security updates, pending updates, recoverable space, orphaned packages
- Score factors: pending updates, orphaned packages, cache size
- Auto-refresh after cleanup/update operations
- Manual refresh button

**Files**: `src/ui/health_dashboard.rs`, `src/models/health.rs`

### 2.2 Before/After Preview ✅
- Pre-action confirmation dialog for Install/Remove/Update
- Shows packages affected with sizes
- Disk space change preview
- Supports new commands and services display
- Cancel/Confirm actions with appropriate styling

**Files**: `src/ui/widgets/action_preview.rs`

---

## Phase 3: Package Timeline & Undo ✅ COMPLETED

### 3.1 Operation History Timeline ✅
- "History" nav item in sidebar
- Timeline grouped by date (Today, Yesterday, older dates)
- All operations tracked: Install, Remove, Update, Downgrade, Cleanup
- **External change detection**: Tracks packages changed outside LinGet (via CLI)
- Filter by: All, Installs, Removes, Updates, Today, This Week
- Search history by package name
- Export history to JSON/CSV (framework in place)

**Files**: `src/models/history.rs`, `src/ui/history_view.rs`, `src/backend/history_tracker.rs`

### 3.2 Undo / Rollback ✅
- Undo button on reversible operations
- Version tracking (before/after) for rollback support
- Operations marked as "undone" in history
- Framework for actual undo execution (placeholder for now)

**UI Concept**:
```
Today
  └─ 14:32  Installed: neovim, ripgrep      [Uninstall]
  └─ 10:15  Updated: firefox (124 → 125)    [Downgrade]
Yesterday  
  └─ 18:00  Removed: libreoffice            [Reinstall]
  └─ 16:00  Installed (external): vim       [CLI badge]
```

**Files**: `src/ui/history_view.rs` (undo actions)

---

## Phase 4: Smart Recommendations 🎯 DIFFERENTIATOR

### 4.1 "You Might Also Like"
**Problem**: Users don't discover useful tools they'd love.

**Features**:
- Based on installed packages, suggest complementary tools:
  - "You have docker → try lazydocker, dive, ctop"
  - "You have neovim → try ripgrep, fzf, fd"
  - "You have python → try ipython, black, mypy"
- Data source: curated package relationships + community patterns
- "Dismiss" and "Not interested" options
- Show in package details and optionally on Home

**UI Concept**:
```
┌─────────────────────────────────────────────────┐
│ Recommended for you                             │
├─────────────────────────────────────────────────┤
│ Based on: docker, kubernetes                    │
│                                                 │
│ 📦 lazydocker    TUI for Docker        [Install]│
│ 📦 k9s           TUI for Kubernetes    [Install]│
│ 📦 dive          Analyze image layers  [Install]│
└─────────────────────────────────────────────────┘
```

**Files**: `src/models/recommendations.rs`, `src/ui/recommendations.rs`, `data/package_relations.json`

### 4.2 Trending & Popular
**Problem**: New users don't know what good packages exist.

**Features**:
- "Trending" section on Home/Discover view
- Popular packages by source (Flathub top apps, etc.)
- "Editor's picks" curated list
- Weekly/monthly highlights

---

## Phase 5: Command Palette & Quick Actions 🎯 POWER USERS

### 5.1 Command Palette (Ctrl+K) ✅ COMPLETED
**Problem**: Power users want keyboard-driven everything.

**Features**:
- `Ctrl+K` opens fuzzy-search command overlay
- Commands:
  - `update all` - Update all packages
  - `clean` - Clean all caches
  - `goto home/library/updates/storage/health/history/favorites` - Navigation
  - `refresh` - Refresh package list
  - `selection` - Toggle selection mode
  - `preferences` - Open preferences
  - `shortcuts` - Show keyboard shortcuts
  - `export/import` - Export/import packages
- Fuzzy matching on command labels and keywords
- Keyboard-only navigation (↑/↓, Enter, Escape)
- Styled overlay window with search entry

**Files**: `src/ui/command_palette.rs`

### 5.2 Keyboard-First UX
**Problem**: Mouse-only is slow for experienced users.

**Features**:
- Full keyboard navigation throughout app
- Vim-style bindings (optional):

| Shortcut     | Action                    |
|--------------|---------------------------|
| `/`          | Focus search              |
| `Ctrl+K`     | Command palette           |
| `j/k`        | Navigate list (vim mode)  |
| `↑/↓`        | Navigate list             |
| `Enter`      | Open details              |
| `i`          | Install selected          |
| `r`          | Remove selected           |
| `u`          | Update selected           |
| `Space`      | Toggle selection          |
| `Ctrl+a`     | Select all                |
| `Ctrl+Enter` | Confirm action            |
| `Escape`     | Close panel / Cancel      |
| `g h`        | Go to Home                |
| `g l`        | Go to Library             |
| `g u`        | Go to Updates             |
| `g s`        | Go to Storage             |

**Files**: `src/ui/keyboard.rs`, update all views

---

## Phase 6: Cross-Machine Sync 🎯 DIFFERENTIATOR

### 6.1 Package List Export/Import
**Problem**: Setting up a new machine means remembering what to install.

**Features**:
- Export installed packages to file (JSON/YAML)
- Import and install from file
- Diff view: "Your export has 12 packages not on this machine"
- Selective import: choose which to install
- Include/exclude by source

**Files**: `src/models/package_list.rs`, `src/ui/sync_dialog.rs`

### 6.2 Cloud Sync (Future)
**Problem**: Manual export/import is tedious for multiple machines.

**Features**:
- Sync package list via cloud (optional)
- See all your machines and their packages
- Push/pull changes
- Privacy-first: encrypted, optional

---

## Phase 7: Enhanced Package Insights

### 7.1 Rich Package Details ✅ COMPLETED
**Problem**: Users lack context to make informed decisions.

**Features**:
- Enhanced "Insights" section in details panel:
  - **Installed**: Human-readable install date (e.g., "2 months ago")
  - **Dependencies**: Package count with shared deps indicator
  - **Required by**: Reverse dependencies count
  - **Safe to remove**: Visual indicator (✓/⚠) based on reverse deps
  - **Config location**: Auto-detected config paths

**Implementation**:
- `src/models/insights.rs`: PackageInsights model with computed fields
- `src/backend/traits.rs`: Added `get_reverse_dependencies()` trait method
- `src/backend/apt.rs`: Implemented reverse deps via `apt-cache rdepends`
- `src/ui/package_details/component.rs`: Insights UI section with async loading

**Files**: `src/models/insights.rs`, `src/ui/package_details/component.rs`

### 7.2 Dependency Viewer
**Problem**: "Why can't I remove this?" - no visibility into dependencies.

**Features**:
- "Dependencies" tab in package details
- Show: "Required by" (reverse deps) and "Requires" (forward deps)
- Visual tree view
- Warning icon on packages that would break others
- Click to navigate to dependent package

### 7.3 Safe Remove Check
**Problem**: Removing a package might break others.

**Features**:
- Before remove, check dependents
- Warning dialog: "Removing X will break Y, Z (they depend on it)"
- Options: Force remove, Cancel, Remove cascade

---

## Phase 8: Update Intelligence

### 8.1 Update Categories
**Problem**: Security updates buried among cosmetic ones.

**Features**:
- Categorize updates: 🔴 Security, 🟡 Bugfix, 🟢 Feature, ⚪ Minor
- Visual badges in update list
- Filter by category
- "Security updates only" quick action
- Notification priority based on category

### 8.2 Changelog Preview
**Problem**: Users update blindly.

**Features**:
- "What's new" expandable in update rows
- Summarize: "2 security fixes, 5 bug fixes"
- Link to full release notes
- AI-generated summary (future)

### 8.3 Scheduled Updates
**Problem**: Updates interrupt workflow.

**Features**:
- "Schedule for later" on updates
- Presets: Tonight, Tomorrow morning, Custom time
- Background execution
- Notification on completion
- Auto-schedule option in preferences

---

## Phase 9: UI Polish & Delight

### 9.1 Animations & Transitions
**Problem**: App feels static and utilitarian.

**Features**:
- Smooth page transitions (slide/fade)
- List item animations (stagger on load)
- Button feedback (ripple, press states)
- Progress bars with shimmer effect
- Skeleton loaders everywhere (no blank states)
- Success/error animations (confetti on batch complete?)

### 9.2 Empty States with Personality
**Problem**: Empty screens feel broken.

**Features**:
- Friendly illustrations for empty states
- Helpful text:
  - No updates: "Your system is fresh! 🌿"
  - No favorites: "Star packages you love to find them here"
  - No search results: "Nothing found. Try a different term?"
- Suggested actions in empty states

### 9.3 Progress & Feedback
**Problem**: Operations feel like black boxes.

**Features**:
- ETA on long operations: "Installing... 2 min left"
- Step indicators: "Step 2/5: Downloading..."
- Operation speed: "Downloading at 5.2 MB/s"
- Sound feedback on completion (optional, configurable)
- Desktop notification on background completion

### 9.4 Theme Polish
**Problem**: Dark mode is often an afterthought.

**Features**:
- True dark mode (OLED-friendly option)
- Accent color customization
- Follow system theme
- High contrast mode for accessibility

---

## Implementation Priority

| Phase | Feature                 | User Impact | Effort | Priority |
|-------|-------------------------|-------------|--------|----------|
| 1.1   | Storage Analyzer        | High        | Medium | ✅ Done  |
| 1.2   | Cleanup Tools           | High        | Medium | ✅ Done  |
| 2.1   | Health Dashboard        | High        | Medium | ✅ Done  |
| 2.2   | Before/After Preview    | High        | Low    | ✅ Done  |
| 3.1   | Operation History       | High        | Medium | ✅ Done  |
| 3.2   | Undo/Rollback           | High        | Medium | ✅ Done  |
| 5.1   | Command Palette         | High        | Low    | ✅ Done  |
| 5.2   | Keyboard Navigation     | Medium      | Low    | **P1**   |
| 4.1   | Recommendations         | Medium      | Medium | P1       |
| 7.1   | Rich Package Details    | Medium      | Low    | ✅ Done  |
| 8.1   | Update Categories       | Medium      | Low    | P1       |
| 6.1   | Export/Import Sync      | Medium      | Low    | P1       |
| 7.2   | Dependency Viewer       | High        | High   | P2       |
| 7.3   | Safe Remove Check       | Medium      | Medium | P2       |
| 8.3   | Scheduled Updates       | Medium      | Medium | P2       |
| 1.3   | Duplicate Detection     | Low         | Low    | P2       |
| 8.2   | Changelog Preview       | Low         | Medium | P3       |
| 9.1   | Animations              | Medium      | Medium | P3       |
| 9.2   | Empty States            | Low         | Low    | P3       |
| 6.2   | Cloud Sync              | Medium      | High   | P3       |
| 4.2   | Trending/Popular        | Low         | Medium | P3       |

---

## Success Metrics

### Efficiency
- [ ] Users can assess system health in <5 seconds
- [ ] Users can reclaim >1GB disk space in <30 seconds
- [ ] Power users can perform any action in <3 keystrokes
- [ ] New machine setup from export in <5 minutes

### Safety
- [ ] Zero "why did this break?" surprises (before/after preview)
- [ ] 100% of destructive actions are undoable within 7 days
- [ ] Security updates visually distinct and one-click actionable

### Delight
- [ ] Users recommend LinGet to others (word of mouth)
- [ ] "This is how package management should work" sentiment
- [ ] Users prefer LinGet over CLI for daily tasks

---

## Design Principles

1. **Show, don't ask** - Display information upfront, don't hide behind clicks
2. **Undo over confirm** - Let users act fast, provide undo instead of friction
3. **Keyboard = mouse** - Every action accessible both ways
4. **Progressive disclosure** - Simple by default, power on demand
5. **No surprises** - Preview every change before it happens
6. **Speed is a feature** - Instant feedback, async everything
