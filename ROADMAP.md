# LinGet Roadmap: Version 0 → Production Ready

## Context
LinGet is a Python TUI package manager using Textual. Currently at v0 (prototype), it has basic functionality but critical gaps in task execution, error handling, and data freshness.

---

## PHASE 1: Critical Fixes (Steps 1-15)

### Step 1: Auto-Refresh After Tasks
**Problem**: Package list is stale after install/update/remove operations
**Solution**: Call `fetch_packages()` after task completes with status "done"
**Acceptance**: Select a package → Install it → List refreshes to show new status

### Step 2: Concurrent Task Execution
**Problem**: Only one task runs at a time
**Solution**: Use `asyncio.gather()` or track multiple running tasks
**Acceptance**: Queue multiple installs → All run simultaneously

### Step 3: Task Cancellation
**Problem**: Cannot cancel running tasks
**Solution**: Store process reference, add cancel button/keybinding, send SIGTERM
**Acceptance**: Long-running task → Press Escape → Task cancels

### Step 4: Real Progress Tracking
**Problem**: Fake 1.5% increments per log line
**Solution**: Parse actual download bytes from apt/flatpak output, or use indeterminate progress
**Acceptance**: Progress bar reflects actual download/install activity

### Step 5: Task Retry Mechanism
**Problem**: Failed tasks stay failed, no way to retry
**Solution**: Add "Retry" action for failed tasks, track error type
**Acceptance**: Failed task → Press 'R' → Task re-queues

### Step 6: Error Classification
**Problem**: All errors show same "Failed" message
**Solution**: Classify errors (auth/canceled, network, not found, conflict)
**Acceptance**: Cancel pkexec → Shows "Authentication cancelled" not generic error

### Step 7: Fix Flatpak Update Command
**Problem**: `flatpak update <name>` wrong for per-app updates
**Solution**: Use `flatpak update` (global) or verify correct per-app syntax
**Acceptance**: Update Flatpak app → Command succeeds

### Step 8: Add pkexec Auth Feedback
**Problem**: No indication password prompt is showing
**Solution**: Update status to "Waiting for authorization..." when pkexec prompts
**Acceptance**: Install apt package → Status shows auth wait

### Step 9: Fix Cargo Privilege Handling
**Problem**: `cargo uninstall` may need sudo on some systems
**Solution**: Wrap cargo uninstall with pkexec when removing system crates
**Acceptance**: Uninstall cargo crate → Sudo prompt appears if needed

### Step 10: Loading State Feedback
**Problem**: "Initializing..." never changes during fetch
**Solution**: Update loading message to show current backend being queried
**Acceptance**: Refresh → See "Fetching APT packages...", "Fetching Flatpak..."

### Step 11: Fix Search Focus Cursor Position
**Problem**: Cursor ends up at wrong position when focusing search
**Solution**: Use `Input.cursor_position = len(value)` properly after focus
**Acceptance**: Press '/' → Cursor at end of existing text

### Step 12: Handle Empty Search Results
**Problem**: No feedback when search yields no results
**Solution**: Show "No packages match your search" in table area
**Acceptance**: Search for "xyznonexistent" → Clear "no results" message

### Step 13: Fix InfoPanel When No Package Selected
**Problem**: InfoPanel shows placeholder but may error on some interactions
**Solution**: Ensure all event handlers guard against null package
**Acceptance**: Launch app → Select nothing → Press 'i' → "No package selected" notification

### Step 14: Handle Dpkg Lock Conflicts
**Problem**: No handling when apt is locked by another process
**Solution**: Detect lock errors, show "apt is locked by another process" with retry option
**Acceptance**: Another apt process running → Update fails gracefully

### Step 15: Graceful Network Failure Handling
**Problem**: Network timeouts/failures crash or silent-fail
**Solution**: Wrap network commands in timeout, catch specific network errors
**Acceptance**: Disconnect network → Refresh → Shows "Network error" not crash

---

## PHASE 2: Core Functionality Completion (Steps 16-35)

### Step 16: Implement "Search for New" Mode
**Problem**: `mode-search` shows NOT_INSTALLED but doesn't search catalogs
**Solution**: Query package repos (apt-cache search, etc.) for new packages
**Acceptance**: Select "Search for New" tab → See installable packages

### Step 17: Package Dependency Info
**Problem**: No way to see what a package depends on or is required by
**Solution**: Add 'd' keybinding to show dependencies in InfoPanel
**Acceptance**: Select package → Press 'd' → See depends/dependents

### Step 18: Repository Management - APT Sources
**Problem**: Cannot add/remove APT repositories
**Solution**: Integrate with `add-apt-repository` command
**Acceptance**: UI to add PPAs and view current sources

### Step 19: Changelog Viewing
**Problem**: No way to see package changelogs
**Solution**: Use `apt changelog <package>` command
**Acceptance**: Select package → Press 'C' → See changelog in log panel

### Step 20: Bulk Selection
**Problem**: Can only select one package at a time
**Solution**: Add checkbox column for multi-select
**Acceptance**: Check 5 packages → Bulk install all

### Step 21: Bulk Operations
**Problem**: No batch install/remove
**Solution**: When multiple selected, action applies to all
**Acceptance**: Select 3 packages → Press 'i' → All install

### Step 22: Favorites System
**Problem**: No way to mark packages as favorites
**Solution**: Add star icon, filter by favorites
**Acceptance**: Star a package → "Favorites" filter shows it

### Step 23: Package Size Display
**Problem**: Size field is empty for most packages
**Solution**: Fetch size from package manager output
**Acceptance**: See actual download/install sizes

### Step 24: Confirmation for Remove
**Problem**: Remove executes immediately, no confirmation
**Solution**: Show confirmation dialog for remove actions
**Acceptance**: Press 'r' on package → Confirm dialog appears

### Step 25: Undo Recent Actions
**Problem**: No undo for accidental installs/removes
**Solution**: Keep action history, add undo command
**Acceptance**: Install package → Press 'z' → Package removed

### Step 26: Task History Persistence
**Problem**: Task history lost on app restart
**Solution**: Save completed/failed tasks to JSON file
**Acceptance**: Complete task → Restart app → History preserved

### Step 27: System Cache Cleanup
**Problem**: No way to clean package manager caches
**Solution**: Add "Clean Cache" button/action per backend
**Acceptance**: Press 'X' → Clean apt/flatpak/cargo caches

### Step 28: Orphan Package Detection
**Problem**: No detection of packages installed as dependencies but no longer needed
**Solution**: Run apt autoremove, show orphan list
**Acceptance**: View orphans → Option to auto-remove

### Step 29: Lock File Status Display
**Problem**: No indication of dpkg/apt lock status
**Solution**: Check lock files before operations, show warning if locked
**Acceptance**: See lock indicator in status bar

### Step 30: Package Version History
**Problem**: Don't know what versions are available
**Solution**: Query version history from apt/flatpak
**Acceptance**: See "Available versions: 1.0, 1.1, 1.2" in info panel

### Step 31: Pin/Priority Packages
**Problem**: Cannot pin package versions
**Solution**: Integrate apt pinning, show pinned status
**Acceptance**: Pin a package → Stays at current version

### Step 32: Download Speed Display
**Problem**: No download speed indicator
**Solution**: Parse download rate from subprocess output
**Acceptance**: See "2.5 MB/s" during downloads

### Step 33: Time Remaining Estimate
**Problem**: No ETA for operations
**Solution**: Calculate based on download speed and size
**Acceptance**: See "About 2 minutes remaining"

### Step 34: Background Refresh
**Problem**: Must manually refresh to see updates
**Solution**: Auto-refresh every N minutes (configurable)
**Acceptance**: Enable setting → Updates appear automatically

### Step 35: Offline Mode Detection
**Problem**: App doesn't detect offline state
**Solution**: Check network before remote operations, disable remote sources
**Acceptance**: Go offline → Remote sources grayed out, local operations work

---

## PHASE 3: Multi-Source Completeness (Steps 36-50)

### Step 36: Flatpak Remote Management
**Problem**: Cannot add/remove flatpak remotes
**Solution**: Integrate `flatpak remote-add/remote-delete`
**Acceptance**: Add Flathub remote → Appears in source list

### Step 37: Flatpak Update All
**Problem**: Only per-app update works
**Solution**: "Update All" button for flatpak
**Acceptance**: Press "Update All Flatpaks"

### Step 38: Snap Support
**Problem**: No snap backend
**Solution**: Add `snap list` and `snap install/remove`
**Acceptance**: See snap packages alongside others

### Step 39: GUI Package Managers
**Problem**: No support for software centers
**Solution**: Integrate with gnome-software, muon
**Acceptance**: Launch GUI installer for package

### Step 40: AUR Support (Arch)
**Problem**: No AUR helper integration
**Solution**: Add yay/pacaur/paru support
**Acceptance**: Install AUR packages

### Step 41: DNF/YUM Support (Fedora/RHEL)
**Problem**: No RPM backend
**Solution**: Add `dnf list/install/remove`
**Acceptance**: Manage RPM packages

### Step 42: Zypper Support (SUSE)
**Problem**: No Zypper backend
**Solution**: Add `zypper info/install/remove`
**Acceptance**: Manage Zypper packages

### Step 43: Homebrew Support (macOS)
**Problem**: No Homebrew backend
**Solution**: Add `brew list/install/uninstall`
**Acceptance**: Mac users can manage brew packages

### Step 44: Chocolatey Support (Windows)
**Problem**: No Windows package manager
**Solution**: Add `choco list/install/uninstall`
**Acceptance**: Windows users can manage choco packages

### Step 45: Winget Support (Windows)
**Problem**: No native Windows package manager
**Solution**: Add `winget list/install/uninstall`
**Acceptance**: Windows Store apps manageable

### Step 46: Nix Support
**Problem**: No Nix package manager
**Solution**: Add `nix-env` integration
**Acceptance**: Manage Nix packages

### Step 47: Guix Support
**Problem**: No Guix package manager
**Solution**: Add `guix package` integration
**Acceptance**: Manage Guix packages

### Step 48: Source-Specific Search
**Problem**: Only searches within installed packages
**Solution**: Per-source search across repos (apt-cache, flatpak search, etc.)
**Acceptance**: Search "rust" → See all available rust packages

### Step 49: Unified Search Results
**Problem**: Search results don't show source for same package
**Solution**: Deduplicate across sources, show "Available in: apt, pip, cargo"
**Acceptance**: Search "python" → See python from multiple sources

### Step 50: Cross-Source Dependencies
**Problem**: Can't see when npm depends on system package
**Solution**: Show cross-manager dependencies
**Acceptance**: "This npm package requires libpng-dev (apt)"

---

## PHASE 4: UI/UX Polish (Steps 51-70)

### Step 51: Dark/Light Theme Toggle
**Problem**: Fixed monokai theme
**Solution**: Add theme switcher with multiple themes
**Acceptance**: Press 'T' → Theme cycles

### Step 52: Custom Color Schemes
**Problem**: Cannot customize colors
**Solution**: Add color configuration file
**Acceptance**: User can define custom theme

### Step 53: Animated Progress Indicators
**Problem**: Progress bars are static
**Solution**: Add pulse animation for indeterminate progress
**Acceptance**: Spinner/pulse during operations

### Step 54: Keyboard Shortcut Help Overlay
**Problem**: Must memorize all shortcuts
**Solution**: Press '?' to show help overlay
**Acceptance**: See all shortcuts in modal

### Step 55: Command Palette
**Problem**: Too many shortcuts to remember
**Solution**: Ctrl+P opens command palette
**Acceptance**: Fuzzy search for commands

### Step 56: Toast Notification History
**Problem**: Notifications disappear quickly
**Solution**: Notification center/queue
**Acceptance**: Press 'N' to see notification history

### Step 57: Empty State Illustrations
**Problem**: Empty states show plain text
**Solution**: Add ASCII art / unicode illustrations
**Acceptance**: See nice empty state when no packages

### Step 58: Status Bar Improvements
**Problem**: Minimal status bar
**Solution**: Show connection status, last refresh time, task count
**Acceptance**: Status bar shows meaningful info

### Step 59: Context Menus
**Problem**: All actions via keyboard
**Solution**: Right-click context menus
**Acceptance**: Right-click package → See actions

### Step 60: Drag and Drop
**Problem**: Cannot reorder or rearrange
**Solution**: Drag packages to queue or favorites
**Acceptance**: Drag to prioritize task queue

### Step 61: Column Sorting
**Problem**: Fixed column order
**Solution**: Click column headers to sort
**Acceptance**: Click "Version" → Sort by version

### Step 62: Column Resizing
**Problem**: Fixed column widths
**Solution**: Drag column borders
**Acceptance**: Resize columns to taste

### Step 63: Variable Row Height
**Problem**: Descriptions truncated
**Solution**: Expand row to show full description
**Acceptance**: Select row → Expands to show details

### Step 64: Fuzzy Search
**Problem**: Exact substring matching only
**Solution**: Use fuzzy matching (fzf-style)
**Acceptance**: Search "bt" matches "bit" "boot" "about"

### Step 65: Search History
**Problem**: Can't reuse recent searches
**Solution**: Show recent searches dropdown
**Acceptance**: Click search → See recent searches

### Step 66: Task Dependencies
**Problem**: No dependency ordering
**Solution**: Auto-order tasks based on dependencies
**Acceptance**: Queue apt install → Auto-waits for flatpak install

### Step 67: Animated Transitions
**Problem**: Instant UI updates
**Solution**: Smooth transitions between states
**Acceptance**: Filter change → Smooth scroll

### Step 68: Sound Effects (Optional)
**Problem**: No audio feedback
**Solution**: Optional success/error sounds
**Acceptance**: Enable in settings → Hear completion sound

### Step 69: Desktop Notifications
**Problem**: Only in-app notifications
**Solution**: Use system notifications via notify-send
**Acceptance**: Get system notification on task complete

### Step 70: Package Previews
**Problem**: Full info only in InfoPanel
**Solution**: Hover preview popup
**Acceptance**: Hover row → See quick preview

---

## PHASE 5: Data & Persistence (Steps 71-80)

### Step 71: Settings Persistence
**Problem**: Settings reset on restart
**Solution**: Save to ~/.config/linget/settings.json
**Acceptance**: Set theme → Restart → Theme preserved

### Step 72: Package List Caching
**Problem**: Fetch on every launch
**Solution**: Cache package lists, refresh in background
**Acceptance**: Instant launch → Background refresh

### Step 73: User Profiles
**Problem**: One config for all users
**Solution**: Multiple named profiles
**Acceptance**: Create "work" and "home" profiles

### Step 74: Backup/Export Package Lists
**Problem**: No way to export installed packages
**Solution**: Export to JSON/CSV
**Acceptance**: Export → "my-packages.json"

### Step 75: Import Package Lists
**Problem**: Cannot restore from backup
**Solution**: Import from JSON/CSV
**Acceptance**: Import → Bulk install from list

### Step 76: Sync Across Machines
**Problem**: No cloud sync
**Solution**: Optional sync via GitHub Gist or similar
**Acceptance**: Sync settings across machines

### Step 77: Package Recommendations
**Problem**: No suggestions
**Solution**: "You installed X, you might like Y"
**Acceptance**: Based on common pairings

### Step 78: Usage Statistics
**Problem**: No tracking of habits
**Solution**: Track most installed/removed packages
**Acceptance**: See "Most installed: rust, node, python"

### Step 79: Security Vulnerability Check
**Problem**: No CVE checking
**Solution**: Integrate with security APIs
**Acceptance**: Red warning on vulnerable packages

### Step 80: License Compliance Report
**Problem**: No license tracking
**Solution**: Generate license report
**Acceptance**: Export license compliance doc

---

## PHASE 6: Integration & Ecosystem (Steps 81-90)

### Step 81: Plugin System
**Problem**: Hard to extend
**Solution**: Plugin API for custom backends
**Acceptance**: Community plugins available

### Step 82: Script Hooks
**Problem**: No pre/post install hooks
**Solution**: Execute scripts on events
**Acceptance**: Pre-install hook → Custom validation

### Step 83: API Server Mode
**Problem**: TUI only
**Solution**: Run as HTTP API server
**Acceptance**: curl localhost:8080/packages

### Step 84: Web UI Mode
**Problem**: No web interface
**Solution**: Optional web interface
**Acceptance**: Browser-based package manager

### Step 85: Mobile Companion App
**Problem**: No mobile access
**Solution**: Flutter companion app
**Acceptance**: Manage packages from phone

### Step 86: CI/CD Integration
**Problem**: No automation support
**Solution**: CLI for CI/CD pipelines
**Acceptance**: `linget install --ci package`

### Step 87: Shell Completion
**Problem**: No bash/zsh completion
**Solution**: Generate completion scripts
**Acceptance**: `linget in<TAB>` → `linget install`

### Step 88: Config File Editor UI
**Problem**: Manual config editing
**Solution**: UI for editing settings
**Acceptance**: Settings → UI to change values

### Step 89: Docker/Container Support
**Problem**: Cannot manage containers
**Solution**: Integrate with Docker/Podman
**Acceptance**: Install packages inside containers

### Step 90: Cloud Instance Management
**Problem**: Can't manage remote machines
**Solution**: SSH-based remote management
**Acceptance**: Manage packages on VPS

---

## PHASE 7: Performance & Reliability (Steps 91-100)

### Step 91: Startup Time Optimization
**Problem**: Slow startup with many packages
**Solution**: Lazy loading, parallel fetching
**Acceptance**: <1s startup with 1000 packages

### Step 92: Memory Usage Reduction
**Problem**: High memory with large lists
**Solution**: Virtual scrolling, pagination
**Acceptance**: 10k packages → Low memory

### Step 93: Concurrent Refresh
**Problem**: Sequential backend queries
**Solution**: Parallel fetch all backends
**Acceptance**: Fetch all sources simultaneously

### Step 94: Query Performance
**Problem**: Slow filtering with many packages
**Solution**: Indexed search, compiled regex
**Acceptance**: Filter 10k packages → Instant

### Step 95: Crash Recovery
**Problem**: Crash loses running tasks
**Solution**: Persist task queue to disk
**Acceptance**: Crash → Resume tasks on restart

### Step 96: Integrity Checks
**Problem**: No self-verification
**Solution**: Checksums, consistency checks
**Acceptance**: Detect corrupted cache

### Step 97: Stress Testing
**Problem**: Unknown limits
**Solution**: Test with 100k packages
**Acceptance**: Handle extreme loads gracefully

### Step 98: Fuzz Testing
**Problem**: Unknown edge cases
**Solution**: Fuzz package name parsing
**Acceptance**: Discover edge case bugs

### Step 99: Benchmark Suite
**Problem**: No performance regression detection
**Solution**: Track metrics over time
**Acceptance**: CI tracks performance

### Step 100: Production Hardening
**Problem**: Prototype code quality
**Solution**: Code review, testing, documentation
**Acceptance**: Production-ready v1.0

---

## Prioritization Notes

### Immediate (Steps 1-15)
These fix critical functionality gaps that make the app unreliable or unusable.

### Short-term (Steps 16-35)
These complete the core feature set to make LinGet competitive with existing tools.

### Mid-term (Steps 36-70)
These expand platform support and polish the UX.

### Long-term (Steps 71-100)
These make LinGet a mature, enterprise-ready product.

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Task success rate | >95% |
| User satisfaction | >4.5/5 |
| Startup time | <2s |
| Memory usage | <100MB baseline |
| Package sources | 10+ |
| Test coverage | >80% |

---

*Generated: 2026-03-29*
*Version: 0 → 1.0 Roadmap*
