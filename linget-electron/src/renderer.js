const sources = [
  { id: 'arch', name: 'Arch Linux [core]', count: 89, icon: 'A', color: '#2e9dff' },
  { id: 'aur', name: 'AUR', count: 42, icon: 'pkg', color: '#f7f7f7' },
  { id: 'flathub', name: 'Flathub', count: 31, icon: 'F', color: '#f7f7f7' },
  { id: 'snap', name: 'Snap Store', count: 12, icon: 'S', color: '#f7f7f7' },
  { id: 'cargo', name: 'Cargo (crates.io)', count: 18, icon: 'R', color: '#f7f7f7' },
  { id: 'npm', name: 'npm Registry', count: 9, icon: 'npm', color: '#f7f7f7' },
  { id: 'dockerhub', name: 'Docker Hub', count: 4, icon: 'D', color: '#f7f7f7' },
  { id: 'github', name: 'GitHub Releases', count: 2, icon: 'GH', color: '#f7f7f7' },
  { id: 'all', name: 'All Sources', count: 207, icon: 'db', color: '#f7f7f7', separated: true },
];

const filters = [
  { id: 'all', label: 'All', count: 207 },
  { id: 'apps', label: 'Apps', count: 86 },
  { id: 'libraries', label: 'Libraries', count: 57 },
  { id: 'security', label: 'Security', count: 7, danger: true },
  { id: 'duplicates', label: 'Duplicates', count: 12 },
  { id: 'favorites', label: 'Favorites', count: 9 },
];

const packages = [
  {
    name: 'tailscale',
    description: 'Zero-config VPN for teams',
    current: '1.62.2-1',
    next: '1.64.1-1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Update',
    icon: 'grid',
    category: 'apps',
    installed: '1.62.2-1',
    available: '1.64.1-1',
    repo: 'extra',
    license: 'BSD-3-Clause',
    download: '12.4 MiB',
    installedSize: '43.7 MiB',
    changelogVersion: '1.64.1',
    changelog: [
      'MagicDNS: split DNS improvements',
      'DERP: better region selection',
      'Admin: device expiry controls',
      'Fix: route advertisement stability',
    ],
    moreChanges: 8,
    dependencies: ['libc', 'libtap', 'iptables-nft', 'ca-certificates', 'resolvconf (opt)'],
    failedAt: '2024-05-20 09:14',
  },
  {
    name: 'linux',
    description: 'The Linux kernel',
    current: '6.8.9.arch1',
    next: '6.8.10.arch1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Update',
    icon: 'linux',
    category: 'libraries',
    installed: '6.8.9.arch1',
    available: '6.8.10.arch1',
    repo: 'core',
    license: 'GPL-2.0-only',
    download: '134.6 MiB',
    installedSize: '211.8 MiB',
    changelogVersion: '6.8.10',
    changelog: ['Stable branch patch release', 'Scheduler fixes', 'Network driver updates', 'Btrfs writeback correction'],
    moreChanges: 18,
    dependencies: ['coreutils', 'kmod', 'mkinitcpio', 'linux-firmware'],
  },
  {
    name: 'openssh',
    description: 'SSH protocol suite',
    current: '9.7p1',
    next: '9.8p1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Security',
    icon: 'boxes',
    category: 'security',
    installed: '9.7p1',
    available: '9.8p1',
    repo: 'core',
    license: 'BSD',
    download: '1.1 MiB',
    installedSize: '5.8 MiB',
    changelogVersion: '9.8p1',
    changelog: ['Security hardening update', 'sshd startup validation', 'Agent forwarding checks', 'Protocol compatibility fixes'],
    moreChanges: 6,
    dependencies: ['glibc', 'openssl', 'krb5', 'libedit'],
  },
  {
    name: 'glibc',
    description: 'GNU C Library',
    current: '2.39-2',
    next: '2.39-3',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Security',
    icon: 'crescent',
    category: 'security',
    installed: '2.39-2',
    available: '2.39-3',
    repo: 'core',
    license: 'LGPL-2.1-or-later',
    download: '10.7 MiB',
    installedSize: '49.2 MiB',
    changelogVersion: '2.39-3',
    changelog: ['Resolver security update', 'Locale data refresh', 'Thread cancellation fixes', 'Loader diagnostics cleanup'],
    moreChanges: 12,
    dependencies: ['linux-api-headers', 'tzdata', 'filesystem'],
  },
  {
    name: 'firefox',
    description: 'Web browser',
    current: '124.0.2-1',
    next: '125.0',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Update',
    icon: 'fire',
    category: 'apps',
    installed: '124.0.2-1',
    available: '125.0',
    repo: 'extra',
    license: 'MPL-2.0',
    download: '68.2 MiB',
    installedSize: '256.1 MiB',
    changelogVersion: '125.0',
    changelog: ['PDF viewer improvements', 'Wayland clipboard fix', 'Developer tools refresh', 'Security advisory rollup'],
    moreChanges: 14,
    dependencies: ['gtk3', 'libvpx', 'nss', 'dbus-glib'],
  },
  {
    name: 'vscode',
    description: 'Code editor',
    current: '1.89.1-1',
    next: '1.90.0-1',
    source: 'AUR',
    sourceId: 'aur',
    status: 'Update',
    icon: 'code',
    category: 'apps',
    installed: '1.89.1-1',
    available: '1.90.0-1',
    repo: 'aur',
    license: 'MIT',
    download: '91.4 MiB',
    installedSize: '343.3 MiB',
    changelogVersion: '1.90.0',
    changelog: ['Profiles sync refinements', 'Terminal accessibility fixes', 'Editor sticky scroll update', 'Extension host diagnostics'],
    moreChanges: 20,
    dependencies: ['electron', 'ripgrep', 'libsecret', 'gtk3'],
  },
  {
    name: 'docker',
    description: 'Container platform',
    current: '26.1.3-1',
    next: '26.1.4-1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Update',
    icon: 'ship',
    category: 'apps',
    installed: '26.1.3-1',
    available: '26.1.4-1',
    repo: 'extra',
    license: 'Apache-2.0',
    download: '28.5 MiB',
    installedSize: '128.8 MiB',
    changelogVersion: '26.1.4',
    changelog: ['BuildKit refresh', 'Overlay network fix', 'Credential helper update', 'Containerd compatibility'],
    moreChanges: 9,
    dependencies: ['containerd', 'runc', 'iptables-nft', 'bridge-utils'],
  },
  {
    name: 'rust',
    description: 'Systems programming language',
    current: '1.77.1-1',
    next: '1.78.0-1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Update',
    icon: 'gear',
    category: 'libraries',
    installed: '1.77.1-1',
    available: '1.78.0-1',
    repo: 'extra',
    license: 'MIT OR Apache-2.0',
    download: '82.0 MiB',
    installedSize: '381.0 MiB',
    changelogVersion: '1.78.0',
    changelog: ['Compiler diagnostics update', 'Cargo resolver fixes', 'Stabilized API additions', 'LLVM patch refresh'],
    moreChanges: 11,
    dependencies: ['llvm-libs', 'curl', 'libgit2', 'gcc-libs'],
  },
  {
    name: 'openssl',
    description: 'TLS/SSL and crypto library',
    current: '3.2.1-2',
    next: '3.2.2-1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Security',
    icon: 'lock',
    category: 'security',
    installed: '3.2.1-2',
    available: '3.2.2-1',
    repo: 'core',
    license: 'Apache-2.0',
    download: '4.9 MiB',
    installedSize: '11.8 MiB',
    changelogVersion: '3.2.2',
    changelog: ['Security advisory fixes', 'Certificate parser update', 'Provider cleanup', 'FIPS module correction'],
    moreChanges: 7,
    dependencies: ['glibc', 'ca-certificates'],
  },
  {
    name: 'nodejs',
    description: 'JavaScript runtime',
    current: '20.11.1-1',
    next: '20.12.0-1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Update',
    icon: 'node',
    category: 'libraries',
    installed: '20.11.1-1',
    available: '20.12.0-1',
    repo: 'extra',
    license: 'MIT',
    download: '13.6 MiB',
    installedSize: '52.9 MiB',
    changelogVersion: '20.12.0',
    changelog: ['V8 engine update', 'Permission model fixes', 'npm compatibility bump', 'Test runner improvements'],
    moreChanges: 10,
    dependencies: ['icu', 'libuv', 'openssl', 'zlib'],
  },
  {
    name: 'git',
    description: 'Distributed VCS',
    current: '2.44.0-1',
    next: '2.45.0-1',
    source: 'Arch Linux',
    sourceId: 'arch',
    status: 'Update',
    icon: 'git',
    category: 'apps',
    installed: '2.44.0-1',
    available: '2.45.0-1',
    repo: 'extra',
    license: 'GPL-2.0-only',
    download: '6.8 MiB',
    installedSize: '39.4 MiB',
    changelogVersion: '2.45.0',
    changelog: ['Sparse checkout fixes', 'Protocol v2 refinements', 'Credential helper cleanup', 'Rebase diagnostics'],
    moreChanges: 8,
    dependencies: ['curl', 'pcre2', 'expat', 'perl'],
  },
  {
    name: 'cargo-audit',
    description: 'Security audit for Rust',
    current: '0.20.1-1',
    next: '0.21.0-1',
    source: 'AUR',
    sourceId: 'aur',
    status: 'Update',
    icon: 'audit',
    category: 'security',
    installed: '0.20.1-1',
    available: '0.21.0-1',
    repo: 'aur',
    license: 'Apache-2.0',
    download: '3.3 MiB',
    installedSize: '12.2 MiB',
    changelogVersion: '0.21.0',
    changelog: ['Advisory DB sync updates', 'Lockfile parser fixes', 'JSON report improvements', 'Exit-code compatibility'],
    moreChanges: 5,
    dependencies: ['rust', 'openssl', 'sqlite'],
  },
];

const state = {
  selectedPackage: 'tailscale',
  selectedFilter: 'all',
  selectedSource: 'arch',
  queued: new Set(),
  ignored: new Set(),
  favorites: new Set(['tailscale', 'firefox', 'vscode']),
  searchOpen: false,
  query: '',
};

const sourceList = document.querySelector('#sourceList');
const filterGrid = document.querySelector('#filterGrid');
const packageList = document.querySelector('#packageList');
const detailsPanel = document.querySelector('#detailsPanel');
const resultCount = document.querySelector('#resultCount');
const toastRegion = document.querySelector('#toastRegion');

function iconMarkup(pkg) {
  return `<span class="pkg-icon ${pkg.icon}" aria-hidden="true">${iconText(pkg.icon)}</span>`;
}

function iconText(icon) {
  switch (icon) {
    case 'grid': return '&#8759;';
    case 'linux': return 'L';
    case 'boxes': return '[]';
    case 'crescent': return 'C';
    case 'fire': return 'Fx';
    case 'code': return '&lt;/&gt;';
    case 'ship': return 'D';
    case 'gear': return '*';
    case 'lock': return '#';
    case 'node': return 'JS';
    case 'git': return 'G';
    case 'audit': return '@';
    default: return '?';
  }
}

function sourceIcon(source) {
  return `<span class="source-icon" style="--source-color: ${source.color}">${source.icon}</span>`;
}

function filteredPackages() {
  return packages.filter((pkg) => {
    if (state.selectedFilter === 'security' && pkg.status !== 'Security') return false;
    if (state.selectedFilter === 'apps' && pkg.category !== 'apps') return false;
    if (state.selectedFilter === 'libraries' && pkg.category !== 'libraries') return false;
    if (state.selectedFilter === 'favorites' && !state.favorites.has(pkg.name)) return false;
    if (state.selectedFilter === 'duplicates' && !['nodejs', 'docker', 'vscode'].includes(pkg.name)) return false;
    if (state.ignored.has(pkg.name)) return false;
    if (state.query && !`${pkg.name} ${pkg.description} ${pkg.source}`.toLowerCase().includes(state.query)) return false;
    return true;
  });
}

function renderSources() {
  sourceList.innerHTML = sources.map((source) => `
    <button class="source-row ${source.id === state.selectedSource ? 'active' : ''} ${source.separated ? 'separated' : ''}" data-source="${source.id}">
      ${sourceIcon(source)}
      <span class="source-name">${source.name}</span>
      <span class="count-dot"></span>
      <span class="source-count">${source.count}</span>
    </button>
  `).join('');
}

function renderFilters() {
  filterGrid.innerHTML = filters.map((filter) => `
    <button class="filter-tile ${filter.id === state.selectedFilter ? 'active' : ''} ${filter.danger ? 'danger' : ''}" data-filter="${filter.id}">
      <span>${filter.label}</span>
      <strong>${filter.count}</strong>
    </button>
  `).join('');
}

function renderPackages() {
  const rows = filteredPackages();
  if (!rows.some((pkg) => pkg.name === state.selectedPackage) && rows.length > 0) {
    state.selectedPackage = rows[0].name;
  }

  packageList.innerHTML = rows.map((pkg) => {
    const queued = state.queued.has(pkg.name);
    const isSelected = pkg.name === state.selectedPackage;
    const statusClass = pkg.status === 'Security' ? 'security' : 'update';
    return `
      <button class="package-row ${isSelected ? 'selected' : ''}" data-package="${pkg.name}" role="option" aria-selected="${isSelected}">
        <span class="pkg-cell pkg-main">
          ${iconMarkup(pkg)}
          <span class="pkg-copy">
            <strong>${pkg.name}</strong>
            <small>${pkg.description}</small>
          </span>
        </span>
        <span class="pkg-cell version-cell">
          <span>${pkg.current}</span>
          <span class="arrow">&rarr;</span>
          <span>${pkg.next}</span>
        </span>
        <span class="pkg-cell source-cell">${pkg.source}</span>
        <span class="pkg-cell status-cell">
          <span class="status-pill ${queued ? 'queued' : statusClass}">${queued ? 'Queued' : pkg.status}</span>
        </span>
        <span class="pkg-cell action-cell">
          <span class="download-button" aria-hidden="true">&#8681;</span>
          <span class="more-button" aria-hidden="true">&#8942;</span>
        </span>
      </button>
    `;
  }).join('');

  resultCount.textContent = rows.length === 0 ? 'Showing 0 of 207' : `Showing 1-${rows.length} of 207`;
  renderDetails();
}

function renderDetails() {
  const pkg = packages.find((item) => item.name === state.selectedPackage) || packages[0];
  const queued = state.queued.has(pkg.name);
  detailsPanel.innerHTML = `
    <div class="detail-title">
      ${iconMarkup(pkg)}
      <div>
        <h2>${pkg.name}</h2>
        <p>${pkg.description}</p>
      </div>
    </div>

    <div class="detail-section metrics">
      <dl>
        <div><dt>Installed</dt><dd>${pkg.installed}</dd></div>
        <div><dt>Available</dt><dd>${pkg.available}</dd></div>
        <div><dt>Repository</dt><dd>${pkg.repo}</dd></div>
        <div><dt>License</dt><dd>${pkg.license}</dd></div>
        <div><dt>Download</dt><dd>${pkg.download}</dd></div>
        <div><dt>Installed Size</dt><dd>${pkg.installedSize}</dd></div>
      </dl>
    </div>

    <div class="detail-section">
      <h3>Changelog (${pkg.changelogVersion})</h3>
      <ul>
        ${pkg.changelog.map((item) => `<li>${item}</li>`).join('')}
      </ul>
      <p class="muted-line">... and ${pkg.moreChanges} more changes</p>
    </div>

    <div class="detail-section">
      <h3>Dependencies</h3>
      <ul>
        ${pkg.dependencies.map((item) => `<li>${item}</li>`).join('')}
      </ul>
    </div>

    <div class="detail-section recommendation">
      <h3>Recommended Action</h3>
      <button class="recommend-card" data-recommend="${pkg.name}">
        <span class="big-download">&#8681;</span>
        <span>
          <strong>${queued ? 'Queued' : `Update ${pkg.name}`}</strong>
          <small>${pkg.installed} <span class="arrow">&rarr;</span> ${pkg.available}</small>
        </span>
      </button>
      ${pkg.failedAt ? `
        <p class="failure-note">1 failed attempt on ${pkg.failedAt}</p>
        <p class="hint-note">Check the Queue tab for details.</p>
      ` : `
        <p class="hint-note">No failed attempts recorded for this package.</p>
      `}
    </div>
  `;
}

function showToast(message, tone = 'info') {
  const toast = document.createElement('div');
  toast.className = `toast ${tone}`;
  toast.textContent = message;
  toastRegion.appendChild(toast);
  setTimeout(() => {
    toast.classList.add('leaving');
    setTimeout(() => toast.remove(), 220);
  }, 1800);
}

function queueSelected() {
  state.queued.add(state.selectedPackage);
  renderPackages();
  showToast(`Queued ${state.selectedPackage} for update`, 'info');
}

function updateAll() {
  filteredPackages().forEach((pkg) => state.queued.add(pkg.name));
  renderPackages();
  showToast('Queued visible updates', 'security');
}

function ignoreSelected() {
  state.ignored.add(state.selectedPackage);
  const nextVisible = filteredPackages()[0];
  if (nextVisible) {
    state.selectedPackage = nextVisible.name;
  }
  renderPackages();
  showToast('Package ignored for this session', 'danger');
}

function toggleFavorite() {
  if (state.favorites.has(state.selectedPackage)) {
    state.favorites.delete(state.selectedPackage);
    showToast('Removed from favorites', 'info');
  } else {
    state.favorites.add(state.selectedPackage);
    showToast('Added to favorites', 'info');
  }
  renderPackages();
}

function moveSelection(delta) {
  const rows = filteredPackages();
  const currentIndex = rows.findIndex((pkg) => pkg.name === state.selectedPackage);
  if (currentIndex === -1 || rows.length === 0) return;
  const nextIndex = Math.max(0, Math.min(rows.length - 1, currentIndex + delta));
  state.selectedPackage = rows[nextIndex].name;
  renderPackages();
}

function openSearch() {
  const query = window.prompt('Search packages', state.query || '');
  if (query !== null) {
    state.query = query.trim().toLowerCase();
    renderPackages();
    showToast(state.query ? `Filtered by "${state.query}"` : 'Search cleared', 'info');
  }
}

function handleCommand(command) {
  switch (command) {
    case 'details':
      showToast(`Showing details for ${state.selectedPackage}`, 'info');
      break;
    case 'update':
      queueSelected();
      break;
    case 'update-all':
      updateAll();
      break;
    case 'ignore':
      ignoreSelected();
      break;
    case 'favorite':
      toggleFavorite();
      break;
    case 'repos':
      showToast('Repository view is mocked in this Electron prototype', 'info');
      break;
    case 'search':
      openSearch();
      break;
    case 'filters':
      showToast('Use the filter tiles above the update table', 'info');
      break;
    case 'quit':
      if (window.lingetShell) {
        window.lingetShell.close();
      }
      break;
    case 'help':
      showToast('Use arrow keys, Enter, u, U, i, *, /, f, and q', 'info');
      break;
    default:
      break;
  }
}

sourceList.addEventListener('click', (event) => {
  const row = event.target.closest('[data-source]');
  if (!row) return;
  state.selectedSource = row.dataset.source;
  renderSources();
  renderPackages();
});

filterGrid.addEventListener('click', (event) => {
  const tile = event.target.closest('[data-filter]');
  if (!tile) return;
  state.selectedFilter = tile.dataset.filter;
  renderFilters();
  renderPackages();
});

packageList.addEventListener('click', (event) => {
  const row = event.target.closest('[data-package]');
  if (!row) return;
  state.selectedPackage = row.dataset.package;
  renderPackages();
});

detailsPanel.addEventListener('click', (event) => {
  const action = event.target.closest('[data-recommend]');
  if (!action) return;
  state.selectedPackage = action.dataset.recommend;
  queueSelected();
});

document.querySelector('.keybar').addEventListener('click', (event) => {
  const button = event.target.closest('[data-command]');
  if (!button) return;
  handleCommand(button.dataset.command);
});

document.addEventListener('keydown', (event) => {
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    moveSelection(1);
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    moveSelection(-1);
  } else if (event.key === 'Enter') {
    handleCommand('details');
  } else if (event.key === 'u') {
    handleCommand('update');
  } else if (event.key === 'U') {
    handleCommand('update-all');
  } else if (event.key === 'i') {
    handleCommand('ignore');
  } else if (event.key === '*') {
    handleCommand('favorite');
  } else if (event.key === '/') {
    event.preventDefault();
    handleCommand('search');
  } else if (event.key === 'f') {
    handleCommand('filters');
  } else if (event.key === 'q') {
    handleCommand('quit');
  } else if (event.key === '?') {
    handleCommand('help');
  }
});

function renderClock() {
  const now = new Date();
  document.querySelector('#clock').textContent = now.toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}

renderSources();
renderFilters();
renderPackages();
renderClock();
setInterval(renderClock, 30000);
