import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';

// =====================================================================
// State
// =====================================================================

const state = {
  tab: 'browse',
  catalog: { packages: [], providers: [], counts: { updates: 0, security: 0, installed: 0 }, refreshing: false },
  filter: 'all',
  query: { text: '', source: null },
  selectedId: null,
  selectedIndex: null,
  openGroups: new Set(),
  toggledGroups: new Set(),
  shownPerGroup: new Map(),
  queueEntries: [],
  entryLogs: new Map(),
  logTail: [],
  logTailDirty: false,
  queueBusy: false,
};

const ROW_CHUNK = 60;
const $ = (id) => document.getElementById(id);

// =====================================================================
// Data layer
// =====================================================================

async function fetchJSON(url, options) {
  let response;
  try {
    response = await fetch(url, options);
  } catch (error) {
    markConnectivity(false);
    throw error;
  }
  if (!response.ok) {
    const text = await response.text().catch(() => '');
    markConnectivity(true);
    throw new Error(text || `${response.status} ${response.statusText}`);
  }
  markConnectivity(true);
  return response.json();
}

// --- connectivity: a frozen UI reads as broken; say what happened ----
let failedPolls = 0;
function markConnectivity(ok) {
  failedPolls = ok ? 0 : failedPolls + 1;
  const banner = $('offline');
  if (banner) banner.hidden = failedPolls < 2;
}

async function loadCatalog() {
  const data = await fetchJSON('/api/catalog');
  state.catalog = data;
  if (!Array.isArray(data.packages) || data.packages.length === 0) {
    renderSkeleton(data.refreshing
      ? 'assembling the catalog — first check running'
      : 'no packages yet');
    setTimeout(() => loadCatalog().catch(() => {}), 2500);
    return;
  }
  renderStats();
  renderChips();
  renderBrowse();
}

async function refreshCatalog() {
  toast('refreshing');
  $('refresh').classList.add('spin');
  await fetchJSON('/api/refresh', { method: 'POST' });
  const timer = setInterval(async () => {
    try {
      const data = await fetchJSON('/api/catalog');
      state.catalog = data;
      renderStats();
      if (!data.refreshing) {
        clearInterval(timer);
        $('refresh').classList.remove('spin');
        renderChips();
        renderBrowse();
        toast('catalog refreshed');
      }
    } catch { /* keep polling */ }
  }, 1500);
}

async function enqueue(action, ids) {
  if (ids.length === 0) { toast('nothing to queue'); return; }
  try {
    const result = await fetchJSON('/api/queue', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ action, ids }),
    });
    const total = result.queued + result.planned;
    toast(total > 0 ? `queued ${total} package${total === 1 ? '' : 's'}` : 'nothing to queue');
    renderQueueLoop();
  } catch (error) {
    toast(`queue failed: ${error.message}`, true);
  }
}

async function toggleFavorite(id) {
  const result = await fetchJSON('/api/favorites', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ id }),
  });
  const pkg = state.catalog.packages.find(p => p.id === id);
  if (pkg) pkg.is_favorite = result.is_favorite;
  toast(result.is_favorite ? 'starred' : 'unstarred');
  renderStats();
  renderChips();
  renderBrowse();
  if (state.selectedId === id) renderDetail(id);
}

async function retryFailed() {
  try {
    const result = await fetchJSON('/api/queue/retry', { method: 'POST' });
    toast(result.retried > 0 ? `retried ${result.retried} task${result.retried === 1 ? '' : 's'}` : 'no failed tasks to retry');
    renderQueueLoop();
  } catch (error) {
    toast(`retry failed: ${error.message}`, true);
  }
}

// =====================================================================
// Utilities
// =====================================================================

function escapeHTML(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function humanSize(bytes) {
  if (bytes == null) return '';
  const units = ['B', 'KiB', 'MiB', 'GiB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++; }
  return `${value.toFixed(value >= 10 || unit === 0 ? 0 : 2)} ${units[unit]}`;
}

/// One actionable line per failure. Policy refusals get the real fix
/// instead of pip's multi-sentence wall of text.
function errorBrief(entry) {
  const error = entry.error || '';
  const lower = error.toLowerCase();
  if (lower.includes('externally managed') || lower.includes('pep 668')) {
    const name = entry.package_name;
    return `python is distro-managed — try "pipx install ${name}" or "apt install python3-${name.replace(/[-_]/g, '-')}"`;
  }
  if (lower.includes('ebadengine') || lower.includes('runtime requirements')) {
    return 'this version needs a newer runtime (node/python) than installed';
  }
  return error.split('\n')[0].slice(0, 140);
}

function isSecurity(pkg) {
  return !!(pkg.is_security || pkg.update_category === 'Security');
}

let toastTimer = null;
function toast(message, isError = false) {
  const el = $('toast');
  el.textContent = message;
  el.classList.toggle('err', isError);
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { el.hidden = true; }, isError ? 5000 : 2400);
}

// =====================================================================
// Tabs
// =====================================================================

function setTab(tab) {
  state.tab = tab;
  for (const name of ['browse', 'galaxy', 'queue']) {
    $(`view-${name}`).hidden = name !== tab;
  }
  document.querySelectorAll('.tab').forEach(el => {
    el.classList.toggle('on', el.dataset.tab === tab);
  });
  visible = tab === 'galaxy';
  if (tab === 'galaxy') initGalaxy();
  if (tab === 'queue') renderQueueLoop();
}

document.querySelectorAll('.tab').forEach(el => {
  el.addEventListener('click', () => setTab(el.dataset.tab));
});

// =====================================================================
// Top bar: stats, chips, search
// =====================================================================

function renderStats() {
  const { counts, refreshing } = state.catalog;
  const favorites = state.catalog.packages.filter(p => p.is_favorite).length;
  const parts = [];
  if (refreshing) parts.push('<span class="stat note">refreshing…</span>');
  if (counts.security > 0) parts.push(`<span class="stat warn">⚠ <b>${counts.security}</b></span>`);
  if (counts.updates > 0) parts.push(`<span class="stat up">↑ <b>${counts.updates}</b></span>`);
  if (favorites > 0) parts.push(`<span class="stat">★ <b>${favorites}</b></span>`);
  if (parts.length === 0 && !refreshing) parts.push('<span class="stat ok">✓ up to date</span>');
  $('stats').innerHTML = parts.join('');
}

function renderChips() {
  const { counts, providers } = state.catalog;
  const chips = [
    { id: 'all', label: 'all' },
    { id: 'updates', label: `updates ${counts.updates || ''}`.trim() },
    { id: 'security', label: `security ${counts.security || ''}`.trim() },
    { id: 'favorites', label: '★ favorites' },
    ...providers.slice(0, 8).map(p => ({ id: `provider:${p.label}`, label: p.label })),
  ];
  const bar = $('chips');
  bar.innerHTML = '';
  for (const chip of chips) {
    const button = document.createElement('button');
    button.className = 'chip' + (state.filter === chip.id ? ' on' : '');
    button.textContent = chip.label;
    button.addEventListener('click', () => {
      state.filter = chip.id;
      state.shownPerGroup.clear();
      state.toggledGroups.clear();
      renderChips();
      renderBrowse();
    });
    bar.appendChild(button);
  }
  if (counts.updates > 0) {
    const all = document.createElement('button');
    all.className = 'chip cta';
    all.textContent = 'update all';
    all.addEventListener('click', () => {
      const ids = state.catalog.packages.filter(p => p.status === 'UpdateAvailable').map(p => p.id);
      enqueue('update', ids);
    });
    bar.appendChild(all);
  }
}

$('search').addEventListener('input', (event) => {
  const tokens = event.target.value.trim().split(/\s+/).filter(Boolean);
  state.query.source = null;
  const words = [];
  for (const token of tokens) {
    if (token.startsWith('src:') && token.length > 4) state.query.source = token.slice(4).toLowerCase();
    else words.push(token.toLowerCase());
  }
  state.query.text = words.join(' ');
  state.shownPerGroup.clear();
  state.toggledGroups.clear();
  renderBrowse();
});

$('refresh').addEventListener('click', refreshCatalog);
$('retry').addEventListener('click', retryFailed);

// =====================================================================
// Browse: grouped, chunked list
// =====================================================================

function packageMatches(pkg) {
  const { text, source } = state.query;
  if (source && !pkg.source.toLowerCase().includes(source)) return false;
  if (text) {
    const name = pkg.name.toLowerCase();
    const desc = (pkg.description || '').toLowerCase();
    if (!name.includes(text) && !desc.includes(text)) return false;
  }
  switch (state.filter) {
    case 'updates': return pkg.status === 'UpdateAvailable';
    case 'security': return pkg.status === 'UpdateAvailable' && isSecurity(pkg);
    case 'favorites': return !!pkg.is_favorite;
    default:
      if (state.filter.startsWith('provider:')) {
        return pkg.source.toLowerCase() === state.filter.slice(9);
      }
      return true;
  }
}

/// Groups for the current filter: security and updates first (open), the
/// installed long tail grouped by provider (collapsed until tapped). A
/// search query flattens everything into one ranked list — grouping is
/// for browsing, not finding.
function computeGroups() {
  const groups = [];
  const matched = [];
  state.catalog.packages.forEach((pkg, index) => {
    if (packageMatches(pkg)) matched.push({ pkg, index });
  });

  const searching = !!(state.query.text || state.query.source);
  if (searching) {
    const needle = state.query.text;
    const rank = ({ pkg }) => isSecurity(pkg) ? 0
      : pkg.status === 'UpdateAvailable' ? 1
      : pkg.is_favorite ? 2 : 3;
    matched.sort((a, b) => {
      const tier = rank(a) - rank(b);
      if (tier !== 0) return tier;
      if (needle) {
        const aPref = a.pkg.name.toLowerCase().startsWith(needle) ? 0 : 1;
        const bPref = b.pkg.name.toLowerCase().startsWith(needle) ? 0 : 1;
        if (aPref !== bPref) return aPref - bPref;
      }
      return a.pkg.name.localeCompare(b.pkg.name);
    });
    const label = state.query.source && !state.query.text
      ? state.query.source
      : 'results';
    groups.push({ key: 'results', label, cls: '', items: matched });
    return groups;
  }

  if (state.filter === 'favorites') {
    groups.push({ key: 'favorites', label: 'favorites', cls: '', items: matched });
    return groups;
  }
  if (state.filter.startsWith('provider:')) {
    const label = state.filter.slice(9);
    groups.push({ key: state.filter, label, cls: '', items: matched });
    return groups;
  }

  const scopeUpdates = state.filter === 'updates' || state.filter === 'security';
  const security = matched.filter(({ pkg }) => pkg.status === 'UpdateAvailable' && isSecurity(pkg));
  const updates = scopeUpdates
    ? matched.filter(({ pkg }) => pkg.status === 'UpdateAvailable' && !isSecurity(pkg))
    : matched.filter(({ pkg }) => pkg.status !== 'UpdateAvailable');

  if (security.length) groups.push({ key: 'security', label: 'security', cls: 'security', items: security });
  if (updates.length && state.filter !== 'security') {
    groups.push(scopeUpdates
      ? { key: 'updates', label: 'updates', cls: 'updates', items: updates }
      : { key: 'installed', label: 'installed', cls: '', items: updates });
  }

  // "all": also break installed out per provider, collapsed by default.
  if (!scopeUpdates && state.filter === 'all') {
    groups.length = 0;
    const sec = matched.filter(({ pkg }) => pkg.status === 'UpdateAvailable' && isSecurity(pkg));
    const upd = matched.filter(({ pkg }) => pkg.status === 'UpdateAvailable' && !isSecurity(pkg));
    const rest = matched.filter(({ pkg }) => pkg.status !== 'UpdateAvailable');
    if (sec.length) groups.push({ key: 'security', label: 'security', cls: 'security', items: sec });
    if (upd.length) groups.push({ key: 'updates', label: 'updates', cls: 'updates', items: upd });
    const byProvider = new Map();
    for (const item of rest) {
      const source = item.pkg.source.toLowerCase();
      if (!byProvider.has(source)) byProvider.set(source, []);
      byProvider.get(source).push(item);
    }
    const sorted = [...byProvider.entries()].sort((a, b) => b[1].length - a[1].length);
    for (const [source, items] of sorted) {
      groups.push({ key: `src:${source}`, label: source, cls: '', items });
    }
  }
  return groups;
}

function highlightName(name, needle) {
  if (!needle) return escapeHTML(name);
  const idx = name.toLowerCase().indexOf(needle);
  if (idx < 0) return escapeHTML(name);
  return escapeHTML(name.slice(0, idx))
    + '<span class="hl">' + escapeHTML(name.slice(idx, idx + needle.length)) + '</span>'
    + escapeHTML(name.slice(idx + needle.length));
}

function versionSub(pkg) {
  if (pkg.status === 'UpdateAvailable') {
    const next = pkg.available_version || '?';
    if (next === pkg.version) {
      return `${escapeHTML(pkg.version)} · new build`;
    }
    return `${escapeHTML(pkg.version)}<span class="arrow">→</span><span class="nv">${escapeHTML(next)}</span>`;
  }
  return escapeHTML(pkg.version);
}

function pkgRow({ pkg, index }, visualIndex = 0) {
  const row = document.createElement('div');
  row.className = 'pkg' + (state.selectedId === pkg.id ? ' sel' : '');
  // Cap the stagger so a 2500-row list doesn't animate for a minute.
  row.style.setProperty('--i', Math.min(visualIndex, 12));

  const security = isSecurity(pkg);
  const glyphClass = security ? 'security' : pkg.status === 'UpdateAvailable' ? 'updates' : pkg.is_favorite ? 'favorite' : '';
  const glyph = security ? '⚠' : pkg.status === 'UpdateAvailable' ? '↑' : pkg.is_favorite ? '★' : '·';

  const g = document.createElement('div');
  g.className = `pkg-glyph ${glyphClass}`;
  g.textContent = glyph;

  const main = document.createElement('div');
  main.className = 'pkg-main';
  const name = document.createElement('div');
  name.className = 'pkg-name';
  name.innerHTML = highlightName(pkg.name, state.query.text);
  const sub = document.createElement('div');
  sub.className = 'pkg-sub';
  const source = pkg.source.toLowerCase();
  const size = pkg.size != null ? ` · ${humanSize(pkg.size)}` : '';
  sub.innerHTML = `${escapeHTML(source)} · ${versionSub(pkg)}${size}`;
  main.appendChild(name);
  main.appendChild(sub);

  row.appendChild(g);
  row.appendChild(main);

  if (pkg.status === 'UpdateAvailable') {
    const quick = document.createElement('button');
    quick.className = 'pkg-quick';
    quick.textContent = 'update';
    quick.addEventListener('click', (event) => {
      event.stopPropagation();
      enqueue('update', [pkg.id]);
    });
    row.appendChild(quick);
  }

  row.addEventListener('click', () => {
    state.selectedId = pkg.id;
    state.selectedIndex = index;
    renderDetail(pkg.id);
    openSheet();
  });
  return row;
}

function renderBrowse() {
  const list = $('list');
  const skeleton = $('skeleton');
  if (skeleton) skeleton.remove();

  const groups = computeGroups();
  list.innerHTML = '';

  if (groups.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'list-empty';
    const searching = state.query.text || state.query.source;
    if (searching) {
      empty.innerHTML = `<div class="big">⌕</div>
        <div class="title">no matches</div>
        <div class="sub">try a shorter query, or esc to clear</div>`;
    } else if (state.filter === 'favorites') {
      empty.innerHTML = `<div class="big">★</div>
        <div class="title">no favorites yet</div>
        <div class="sub">tap any package, then star it</div>`;
    } else {
      empty.innerHTML = `<div class="big">✓</div>
        <div class="title">nothing here</div>
        <div class="sub">all clear</div>`;
    }
    list.appendChild(empty);
    return;
  }

  for (const group of groups) {
    const section = document.createElement('div');
    section.className = `group ${group.cls}`;

    // Default: search results, security and updates open; scoped filters
    // open everything; the installed-by-provider tail starts collapsed.
    // Explicit taps always win over defaults.
    const defaultsOpen = group.key === 'results'
      || state.filter !== 'all'
      || group.key === 'security' || group.key === 'updates';
    const isOpen = state.toggledGroups.has(group.key)
      ? state.openGroups.has(group.key)
      : defaultsOpen;

    const head = document.createElement('div');
    head.className = 'group-head';
    head.innerHTML = `<span class="g-label">${escapeHTML(group.label)}</span>
      <span class="g-count">${group.items.length}</span>
      <span class="g-chevron">▶</span>`;
    head.addEventListener('click', () => {
      state.toggledGroups.add(group.key);
      if (state.openGroups.has(group.key)) state.openGroups.delete(group.key);
      else state.openGroups.add(group.key);
      renderBrowse();
    });
    section.appendChild(head);

    if (isOpen) {
      section.classList.add('open');
      const shown = state.shownPerGroup.get(group.key) ?? ROW_CHUNK;
      const slice = group.items.slice(0, shown);
      slice.forEach((item, i) => section.appendChild(pkgRow(item, i)));
      if (shown < group.items.length) {
        const more = document.createElement('button');
        more.className = 'more-btn';
        more.textContent = `show ${Math.min(ROW_CHUNK, group.items.length - shown)} more of ${group.items.length}`;
        more.addEventListener('click', () => {
          state.shownPerGroup.set(group.key, shown + ROW_CHUNK);
          renderBrowse();
        });
        section.appendChild(more);
      }
    }
    list.appendChild(section);
  }
}

function renderSkeleton(label) {
  const list = $('list');
  list.innerHTML = '';
  const sk = document.createElement('div');
  sk.className = 'skeleton';
  sk.id = 'skeleton';
  for (let i = 0; i < 9; i++) {
    const row = document.createElement('div');
    row.className = 'sk-row';
    row.innerHTML = `
      <div class="sk-block" style="width:34px;height:34px;border-radius:10px;"></div>
      <div style="flex:1">
        <div class="sk-block" style="height:13px;width:${38 + (i * 13) % 45}%;margin-bottom:7px;"></div>
        <div class="sk-block" style="height:10px;width:${20 + (i * 7) % 30}%;"></div>
      </div>`;
    sk.appendChild(row);
  }
  if (label) {
    const note = document.createElement('div');
    note.style.cssText = 'text-align:center;color:var(--text-3);font-size:13px;padding:4px 0 14px;';
    note.textContent = label;
    sk.appendChild(note);
  }
  list.appendChild(sk);
}

// =====================================================================
// Detail sheet
// =====================================================================

function openSheet() {
  $('sheet').hidden = false;
  $('sheet-backdrop').hidden = false;
}
function closeSheet() {
  $('sheet').hidden = true;
  $('sheet-backdrop').hidden = true;
  state.selectedId = null;
  state.selectedIndex = null;
  hideHalo();
  document.querySelectorAll('.pkg.sel').forEach(el => el.classList.remove('sel'));
}

$('sheet-backdrop').addEventListener('click', closeSheet);
document.querySelector('.sheet-close').addEventListener('click', closeSheet);

function renderDetail(id) {
  const pkg = state.catalog.packages.find(p => p.id === id);
  if (!pkg) return;
  $('d-name').textContent = pkg.name;
  $('d-meta').textContent = `${pkg.source.toLowerCase()}${pkg.size ? ' · ' + humanSize(pkg.size) : ''}`;
  $('d-changelog').hidden = true;

  const versions = $('d-versions');
  if (pkg.status === 'UpdateAvailable') {
    const next = pkg.available_version || '?';
    const same = next === pkg.version;
    versions.innerHTML = `<span class="tag">${isSecurity(pkg) ? 'security update' : 'update available'}</span>`
      + (same
        ? `<span class="ok">${escapeHTML(pkg.version)} · new build</span>`
        : `${escapeHTML(pkg.version)}<span class="arrow">→</span><span class="nv">${escapeHTML(next)}</span>`);
  } else {
    versions.innerHTML = `<span class="tag">installed</span>${escapeHTML(pkg.version)}`;
  }

  $('d-desc').textContent = pkg.description || '';
  const facts = [];
  if (pkg.is_favorite) facts.push('★ favorite');
  if (pkg.homepage) facts.push(`<a href="${escapeHTML(pkg.homepage)}" target="_blank" rel="noopener">${escapeHTML(pkg.homepage)}</a>`);
  $('d-facts').innerHTML = facts.join(' · ');

  const actions = $('d-actions');
  actions.innerHTML = '';
  const make = (label, className, span2, onClick) => {
    const b = document.createElement('button');
    b.className = `btn ${className}${span2 ? ' span2' : ''}`;
    b.textContent = label;
    b.addEventListener('click', onClick);
    actions.appendChild(b);
    return b;
  };
  if (pkg.status === 'UpdateAvailable') {
    make('update', 'accent', false, () => { enqueue('update', [pkg.id]); closeSheet(); });
  }
  if (pkg.status === 'UpdateAvailable' || pkg.status === 'Installed') {
    // Destructive: arm-then-fire. The first tap re-labels the button and
    // gives the user a beat to back out; a stray thumb cannot remove a
    // package in one motion.
    const remove = make('remove', 'danger', false, null);
    let armed = false;
    let disarmTimer = null;
    remove.addEventListener('click', () => {
      if (!armed) {
        armed = true;
        remove.textContent = 'tap again to remove';
        remove.classList.add('armed');
        disarmTimer = setTimeout(() => {
          armed = false;
          remove.textContent = 'remove';
          remove.classList.remove('armed');
        }, 3000);
        return;
      }
      clearTimeout(disarmTimer);
      enqueue('remove', [pkg.id]);
      closeSheet();
    });
  }
  make(pkg.is_favorite ? 'unstar' : 'star', 'subtle', false, () => toggleFavorite(pkg.id));
  make('changelog', 'ghost', true, async () => {
    try {
      const data = await fetchJSON(`/api/changelog?id=${encodeURIComponent(pkg.id)}`);
      const box = $('d-changelog');
      box.textContent = data.changelog.slice(0, 6000);
      box.hidden = false;
    } catch (error) {
      toast(String(error.message).includes('no changelog') ? 'no changelog' : `changelog failed: ${error.message}`, true);
    }
  });
}

// =====================================================================
// Queue tab + SSE
// =====================================================================

function renderQueue() {
  const entries = state.queueEntries;
  const badge = $('queue-badge');
  const running = entries.filter(e => e.status === 'Running').length;
  const queued = entries.filter(e => e.status === 'Queued').length;
  const failed = entries.filter(e => e.status === 'Failed').length;
  const busy = running + queued;

  badge.hidden = busy + failed === 0;
  badge.classList.toggle('bad', failed > 0 && busy === 0);
  badge.textContent = busy > 0 ? busy : failed;

  if (state.queueBusy && busy === 0) setTimeout(loadCatalog, 2500);
  state.queueBusy = busy > 0;

  // Retry only makes sense when something failed; a dead button that
  // explains itself beats a toast that says "nothing to retry".
  const retry = $('retry');
  retry.disabled = failed === 0;
  retry.title = failed === 0 ? 'no failed tasks' : `retry ${failed} failed task${failed === 1 ? '' : 's'}`;

  const list = $('queue-list');
  list.innerHTML = '';
  const order = { Running: 0, Queued: 1, Failed: 2, Completed: 3, Cancelled: 4 };
  const shown = [...entries].sort((a, b) => (order[a.status] ?? 9) - (order[b.status] ?? 9)).slice(0, 50);
  if (shown.length === 0) {
    list.innerHTML = `<div class="list-empty">
      <div class="big">◌</div>
      <div class="title">the queue is empty</div>
      <div class="sub">updates you queue run here, with live output</div>
    </div>`;
  }
  for (const entry of shown) {
    const row = document.createElement('div');
    row.className = `q-entry ${entry.status === 'Running' ? 'running' : entry.status === 'Failed' ? 'failed' : entry.status === 'Completed' ? 'done' : ''}`;
    let sub;
    if (entry.status === 'Failed' && entry.error) {
      sub = `<div class="q-sub err-line">${escapeHTML(errorBrief(entry))}</div>`;
    } else if (entry.status === 'Running') {
      // The last output line IS the progress bar for package operations.
      const log = entryLog(entry.id);
      sub = log
        ? `<div class="q-sub log-line">${escapeHTML(log.slice(0, 90))}</div>`
        : `<div class="q-sub">${entry.action.toLowerCase()} · running</div>`;
    } else {
      sub = `<div class="q-sub">${entry.action.toLowerCase()} · ${entry.status.toLowerCase()}</div>`;
    }
    row.innerHTML = `<span class="q-dot"></span>
      <div class="q-main">
        <div class="q-name">${escapeHTML(entry.package_name)}</div>
        ${sub}
      </div>`;
    list.appendChild(row);
  }
  renderLogTail();
}

function entryLog(entryId) {
  const buffer = state.entryLogs.get(entryId);
  return buffer && buffer.length > 0 ? buffer[buffer.length - 1] : null;
}

function pushEntryLog(entryId, line) {
  let buffer = state.entryLogs.get(entryId);
  if (!buffer) { buffer = []; state.entryLogs.set(entryId, buffer); }
  buffer.push(line);
  if (buffer.length > 24) buffer.shift();

  const entry = state.queueEntries.find(e => e.id === entryId);
  const who = entry ? entry.package_name : entryId.slice(0, 8);
  state.logTail.push({ who, line, err: /\b(error|failed|fatal)\b/i.test(line) });
  if (state.logTail.length > 120) state.logTail.splice(0, state.logTail.length - 120);
  state.logTailDirty = true;
}

function renderLogTail() {
  if (!state.logTailDirty) return;
  state.logTailDirty = false;
  const wrap = $('queue-log-wrap');
  const box = $('queue-log');
  const lines = state.logTail.slice(-16);
  if (lines.length === 0) { wrap.hidden = true; return; }
  wrap.hidden = false;
  const nearBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 48;
  box.innerHTML = '';
  for (const item of lines) {
    const div = document.createElement('div');
    div.className = 'line' + (item.err ? ' err' : '');
    const who = document.createElement('span');
    who.className = 'who';
    who.textContent = `${item.who} `;
    div.appendChild(who);
    div.appendChild(document.createTextNode(item.line));
    box.appendChild(div);
  }
  if (nearBottom) box.scrollTop = box.scrollHeight;
}

function upsertQueueEntry(entry) {
  const index = state.queueEntries.findIndex(e => e.id === entry.id);
  if (index >= 0) state.queueEntries[index] = entry;
  else state.queueEntries.unshift(entry);
  if (state.queueEntries.length > 60) state.queueEntries.length = 60;
}

let queueTimer = null;
async function renderQueueLoop() {
  try {
    const data = await fetchJSON('/api/queue');
    state.queueEntries = data.entries ?? [];
    renderQueue();
  } catch { /* SSE carries the live updates */ }
  clearTimeout(queueTimer);
  queueTimer = setTimeout(renderQueueLoop, 15000);
}

function connectEventStream() {
  const source = new EventSource('/api/queue/stream');

  source.addEventListener('state', (event) => {
    state.queueEntries = JSON.parse(event.data).entries ?? [];
    renderQueue();
  });

  source.addEventListener('task', (event) => {
    const entry = JSON.parse(event.data);
    const wasRunning = state.queueEntries.some(e => e.id === entry.id && e.status === 'Running');
    upsertQueueEntry(entry);
    if (wasRunning && entry.status === 'Completed') {
      toast(`${entry.package_name} ${entry.action.toLowerCase()} done`);
    } else if (wasRunning && entry.status === 'Failed') {
      toast(`${entry.package_name} failed — see queue`, true);
    }
    renderQueue();
    if (entry.status === 'Completed' || entry.status === 'Failed') {
      const busy = state.queueEntries.some(e => e.status === 'Running' || e.status === 'Queued');
      if (!busy) setTimeout(loadCatalog, 1500);
    }
  });

  source.addEventListener('log', (event) => {
    const data = JSON.parse(event.data);
    if (data.entry_id && data.line) pushEntryLog(data.entry_id, data.line);
    renderQueue();
  });
}

// =====================================================================
// Galaxy tab — lazy, paused when hidden
// =====================================================================

const COLORS = {
  security: new THREE.Color('#e06060'),
  update: new THREE.Color('#e2b054'),
  favorite: new THREE.Color('#e8c97a'),
  installed: new THREE.Color('#8b94a6'),
  dust: new THREE.Color('#232b3a'),
};

let renderer, scene, camera, controls, composer, galaxy, points;
let galaxyReady = false;
let halo = null;
let positionArray, alphaArray;
let packageIndex = [];
let paused = false;
let visible = false;

const REDUCED_MOTION = matchMedia('(prefers-reduced-motion: reduce)').matches;
const UNIFORMS = { uTime: { value: 0 }, uScale: { value: innerHeight / 2 }, uMap: { value: null } };

function hash32(text) {
  let h = 2166136261;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function rng(seed) {
  let s = seed || 1;
  return () => {
    s ^= s << 13; s >>>= 0;
    s ^= s >> 17;
    s ^= s << 5; s >>>= 0;
    return s / 4294967296;
  };
}

function starTexture() {
  const c = document.createElement('canvas');
  c.width = c.height = 128;
  const g = c.getContext('2d');
  const core = g.createRadialGradient(64, 64, 0, 64, 64, 60);
  core.addColorStop(0.0, 'rgba(255,255,255,1)');
  core.addColorStop(0.12, 'rgba(255,255,255,0.95)');
  core.addColorStop(0.3, 'rgba(255,255,255,0.28)');
  core.addColorStop(0.6, 'rgba(255,255,255,0.05)');
  core.addColorStop(1.0, 'rgba(255,255,255,0)');
  g.fillStyle = core;
  g.fillRect(0, 0, 128, 128);
  g.globalCompositeOperation = 'lighter';
  for (const [x0, y0, x1, y1] of [[64, 6, 64, 122], [6, 64, 122, 64]]) {
    const spike = g.createLinearGradient(x0, y0, x1, y1);
    spike.addColorStop(0, 'rgba(255,255,255,0)');
    spike.addColorStop(0.5, 'rgba(255,255,255,0.55)');
    spike.addColorStop(1, 'rgba(255,255,255,0)');
    g.fillStyle = spike;
    g.fillRect(Math.min(x0, x1), Math.min(y0, y1), Math.abs(x1 - x0) || 3, Math.abs(y1 - y0) || 3);
  }
  const texture = new THREE.CanvasTexture(c);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

const POINT_MATERIAL = new THREE.ShaderMaterial({
  uniforms: UNIFORMS,
  transparent: true,
  depthWrite: false,
  blending: THREE.AdditiveBlending,
  vertexShader: /* glsl */ `
    attribute vec3 aColor;
    attribute float aSize;
    attribute float aSeed;
    attribute float aAlpha;
    attribute float aAngle;
    uniform float uTime;
    uniform float uScale;
    varying vec3 vColor;
    varying float vAlpha;
    varying float vAngle;
    void main() {
      vColor = aColor;
      vAlpha = aAlpha;
      vAngle = aAngle;
      vec4 mv = modelViewMatrix * vec4(position, 1.0);
      float twinkle = 0.82 + 0.28 * sin(uTime * 1.7 + aSeed * 6.2831);
      gl_PointSize = aSize * twinkle * (uScale / -mv.z);
      gl_Position = projectionMatrix * mv;
    }
  `,
  fragmentShader: /* glsl */ `
    uniform sampler2D uMap;
    varying vec3 vColor;
    varying float vAlpha;
    varying float vAngle;
    void main() {
      vec2 uv = gl_PointCoord - 0.5;
      float s = sin(vAngle), c = cos(vAngle);
      uv = mat2(c, -s, s, c) * uv + 0.5;
      vec4 tex = texture2D(uMap, uv);
      float center = 1.0 - smoothstep(0.0, 0.22, length(uv - 0.5));
      vec3 col = vColor * tex.rgb + vec3(center * 0.85 * tex.a);
      gl_FragColor = vec4(col, tex.a * vAlpha);
    }
  `,
});

function makePoints(count, build) {
  const geometry = new THREE.BufferGeometry();
  const position = new Float32Array(count * 3);
  const color = new Float32Array(count * 3);
  const size = new Float32Array(count);
  const seed = new Float32Array(count);
  const alpha = new Float32Array(count).fill(1);
  const angle = new Float32Array(count);
  build({ position, color, size, seed, alpha, angle });
  geometry.setAttribute('position', new THREE.BufferAttribute(position, 3));
  geometry.setAttribute('aColor', new THREE.BufferAttribute(color, 3));
  geometry.setAttribute('aSize', new THREE.BufferAttribute(size, 1));
  geometry.setAttribute('aSeed', new THREE.BufferAttribute(seed, 1));
  geometry.setAttribute('aAlpha', new THREE.BufferAttribute(alpha, 1));
  geometry.setAttribute('aAngle', new THREE.BufferAttribute(angle, 1));
  const pts = new THREE.Points(geometry, POINT_MATERIAL);
  pts.frustumCulled = false;
  return pts;
}

const ARMS = 4;
function placePackage(pkg) {
  const seed = hash32(pkg.id);
  const rand = rng(seed);
  const security = isSecurity(pkg);
  const isUpdate = pkg.status === 'UpdateAvailable';
  const tier = security ? 0 : isUpdate ? 1 : 2;
  const radius = [2.4, 7.5, 13.5][tier] + rand() * [3.4, 5.5, 12.5][tier];
  const armAngle = ((seed % ARMS) / ARMS) * Math.PI * 2;
  const angle = armAngle + radius * 0.28 + (rand() - 0.5) * (0.5 + radius * 0.055);
  const height = (rand() - 0.5) * (1.4 + radius * 0.10) * (tier === 2 ? 1.5 : 0.8)
    + (pkg.is_favorite ? 1.1 : 0);
  return new THREE.Vector3(Math.cos(angle) * radius, height, Math.sin(angle) * radius);
}

function pointColor(pkg) {
  if (isSecurity(pkg)) return COLORS.security;
  if (pkg.is_favorite) return COLORS.favorite;
  if (pkg.status === 'UpdateAvailable') return COLORS.update;
  return COLORS.installed;
}

function pointSize(pkg) {
  if (isSecurity(pkg)) return 3.8;
  if (pkg.is_favorite) return 3.3;
  if (pkg.status === 'UpdateAvailable') return 2.8;
  return 1.9;
}

function initGalaxy() {
  if (galaxyReady) { visible = true; return; }
  galaxyReady = true;
  visible = true;

  const canvas = $('scene');
  renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.setSize(innerWidth, innerHeight);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;

  scene = new THREE.Scene();
  scene.fog = new THREE.FogExp2(0x070a0f, 0.011);

  camera = new THREE.PerspectiveCamera(52, innerWidth / innerHeight, 0.1, 300);
  camera.position.set(9, 16, 30);

  controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.06;
  controls.minDistance = 6;
  controls.maxDistance = 90;
  controls.autoRotate = !REDUCED_MOTION;
  controls.autoRotateSpeed = 0.22;
  let idleTimer = null;
  controls.addEventListener('start', () => { controls.autoRotate = false; clearTimeout(idleTimer); });
  controls.addEventListener('end', () => {
    clearTimeout(idleTimer);
    if (!REDUCED_MOTION) idleTimer = setTimeout(() => { controls.autoRotate = true; }, 9000);
  });

  composer = new EffectComposer(renderer);
  composer.addPass(new RenderPass(scene, camera));
  composer.addPass(new UnrealBloomPass(new THREE.Vector2(innerWidth, innerHeight), 1.15, 0.55, 0.12));
  composer.addPass(new OutputPass());

  galaxy = new THREE.Group();
  scene.add(galaxy);

  UNIFORMS.uMap.value = starTexture();

  halo = new THREE.Sprite(new THREE.SpriteMaterial({
    map: (() => {
      const c = document.createElement('canvas');
      c.width = c.height = 128;
      const g = c.getContext('2d');
      g.strokeStyle = 'rgba(86,200,220,0.95)';
      g.lineWidth = 5;
      g.beginPath(); g.arc(64, 64, 50, 0, Math.PI * 2); g.stroke();
      g.strokeStyle = 'rgba(86,200,220,0.3)';
      g.lineWidth = 1.5;
      g.beginPath(); g.arc(64, 64, 60, 0, Math.PI * 2); g.stroke();
      const t = new THREE.CanvasTexture(c);
      t.colorSpace = THREE.SRGBColorSpace;
      return t;
    })(),
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  }));
  halo.visible = false;
  scene.add(halo);

  buildGalaxy();

  canvas.addEventListener('pointerdown', (event) => {
    pointerDownAt = { x: event.clientX, y: event.clientY, t: performance.now() };
  });
  canvas.addEventListener('pointerup', (event) => {
    if (!pointerDownAt) return;
    const moved = Math.hypot(event.clientX - pointerDownAt.x, event.clientY - pointerDownAt.y);
    const elapsed = performance.now() - pointerDownAt.t;
    pointerDownAt = null;
    if (moved > 8 || elapsed > 500) return;
    const rect = canvas.getBoundingClientRect();
    pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    raycaster.setFromCamera(pointer, camera);
    const hits = raycaster.intersectObject(points, false);
    const live = hits.filter(hit => alphaArray.array[hit.index] > 0.3);
    if (live.length === 0) { closeSheet(); return; }
    const index = live[0].index;
    const pkg = packageIndex[index];
    state.selectedId = pkg.id;
    state.selectedIndex = index;
    halo.visible = true;
    controls.autoRotate = false;
    renderDetail(pkg.id);
    openSheet();
  });

  addEventListener('resize', () => {
    if (!galaxyReady) return;
    camera.aspect = innerWidth / innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(innerWidth, innerHeight);
    composer.setSize(innerWidth, innerHeight);
    UNIFORMS.uScale.value = innerHeight / 2;
  });

  animate();
}

function buildGalaxy() {
  if (!galaxyReady) return;
  if (points) galaxy.remove(points);
  const packages = state.catalog.packages;
  packageIndex = packages;

  points = makePoints(packages.length, ({ position, color, size, seed, angle }) => {
    packages.forEach((pkg, i) => {
      const spot = placePackage(pkg);
      position[i * 3] = spot.x;
      position[i * 3 + 1] = spot.y;
      position[i * 3 + 2] = spot.z;
      const tint = pointColor(pkg);
      color[i * 3] = tint.r;
      color[i * 3 + 1] = tint.g;
      color[i * 3 + 2] = tint.b;
      size[i] = pointSize(pkg);
      const star = hash32(pkg.id);
      seed[i] = (star % 1000) / 1000;
      angle[i] = ((star >> 10) % 6283) / 1000.0;
    });
  });
  positionArray = points.geometry.getAttribute('position');
  alphaArray = points.geometry.getAttribute('aAlpha');
  galaxy.add(points);

  const existing = galaxy.getObjectByName('dust');
  if (existing) galaxy.remove(existing);
  const count = 1600;
  const dust = makePoints(count, ({ position, color, size, seed, angle }) => {
    const rand = rng(1234567);
    for (let i = 0; i < count; i++) {
      const radius = 6 + rand() * 34;
      const anglePos = rand() * Math.PI * 2;
      position[i * 3] = Math.cos(anglePos) * radius;
      position[i * 3 + 1] = (rand() - 0.5) * 16;
      position[i * 3 + 2] = Math.sin(anglePos) * radius;
      const shade = 0.55 + rand() * 0.45;
      color[i * 3] = COLORS.dust.r * shade;
      color[i * 3 + 1] = COLORS.dust.g * shade;
      color[i * 3 + 2] = COLORS.dust.b * shade * 1.25;
      size[i] = 0.5 + rand() * 0.8;
      seed[i] = rand();
      angle[i] = rand() * Math.PI;
    }
  });
  dust.name = 'dust';
  dust.renderOrder = -1;
  galaxy.add(dust);

  applyGalaxyFilter();
}

function applyGalaxyFilter() {
  if (!alphaArray) return;
  for (let i = 0; i < packageIndex.length; i++) {
    alphaArray.array[i] = packageMatches(packageIndex[i]) ? 1 : 0.045;
  }
  alphaArray.needsUpdate = true;
}

function hideHalo() {
  if (halo) halo.visible = false;
}

const raycaster = new THREE.Raycaster();
raycaster.params.Points.threshold = 0.55;
const pointer = new THREE.Vector2();
let pointerDownAt = null;

const clock = new THREE.Clock();
const focusTarget = new THREE.Vector3();
let focusEase = null;

function animate() {
  requestAnimationFrame(animate);
  // Render only while the galaxy tab is visible and the page is foreground.
  if (!visible || paused || document.hidden) return;
  const dt = clock.getDelta();
  const time = clock.elapsedTime;
  UNIFORMS.uTime.value = time;

  if (!REDUCED_MOTION) galaxy.rotation.y = time * 0.004;

  if (state.selectedIndex != null && positionArray) {
    focusTarget.fromBufferAttribute(positionArray, state.selectedIndex);
    galaxy.localToWorld(focusTarget);
    focusEase = focusEase || focusTarget.clone();
    focusEase.lerp(focusTarget, 0.12);
    halo.position.copy(focusEase);
    const pulse = 1 + 0.12 * Math.sin(time * 3.2);
    halo.scale.setScalar(2.6 * pulse);
    controls.target.lerp(focusEase, 0.05);
  }

  controls.update();
  composer.render();
}

// Pause the RAF work when leaving the tab — handled inside setTab via
// the `visible` flag.

// =====================================================================
// Boot
// =====================================================================

document.addEventListener('visibilitychange', () => { paused = document.hidden; });

renderSkeleton('assembling the catalog');
renderStats();
loadCatalog().catch(error => {
  renderSkeleton(`failed to load catalog: ${error.message}`);
});
connectEventStream();
renderQueueLoop();
