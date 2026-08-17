import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';

// ---------------------------------------------------------------------
// Palette — same identity as the terminal UI
// ---------------------------------------------------------------------

const COLORS = {
  security: new THREE.Color('#e05a5a'),
  update: new THREE.Color('#e6b450'),
  favorite: new THREE.Color('#f0c674'),
  installed: new THREE.Color('#4a5568'),
  dust: new THREE.Color('#232b3a'),
  accent: new THREE.Color('#5ec8dc'),
};

const REDUCED_MOTION = matchMedia('(prefers-reduced-motion: reduce)').matches;

// ---------------------------------------------------------------------
// State
// ---------------------------------------------------------------------

const state = {
  catalog: { packages: [], providers: [], counts: { updates: 0, security: 0, installed: 0 }, refreshing: false },
  filter: 'all',
  query: { text: '', source: null },
  selectedId: null,
  selectedIndex: null,
  points: null,          // THREE.Points for packages
  packageIndex: [],      // package meta per point index
  positionArray: null,
  alphaArray: null,
  focus: null,           // Vector3 the camera eases toward
  queueBusy: false,
  // Live queue state, driven by SSE with a slow poll backstop.
  queueEntries: [],
  entryLogs: new Map(),  // entry id -> recent output lines
  logTail: [],           // { who, line, err } global tail for the drawer
  logTailDirty: false,
  fly: null,             // camera ease {from, to, t}
};

// ---------------------------------------------------------------------
// Data layer
// ---------------------------------------------------------------------

async function fetchJSON(url, options) {
  const response = await fetch(url, options);
  if (!response.ok) {
    const text = await response.text().catch(() => '');
    throw new Error(text || `${response.status} ${response.statusText}`);
  }
  return response.json();
}

async function loadCatalog() {
  const data = await fetchJSON('/api/catalog');
  state.catalog = data;
  // Cold start with no cache yet: keep the boot screen and retry rather
  // than painting an empty galaxy that reads as "up to date".
  if (!Array.isArray(data.packages) || data.packages.length === 0) {
    $('boot-text').textContent = data.refreshing
      ? 'assembling the catalog — first check running'
      : 'catalog is empty — tap ↻ to check sources';
    setTimeout(() => loadCatalog().catch(() => {}), 2500);
    return;
  }
  buildGalaxy();
  renderStats();
  renderChips();
  applyFilters();
  hideBoot();
}

async function refreshCatalog() {
  toast('refreshing catalog');
  await fetchJSON('/api/refresh', { method: 'POST' });
  pollRefreshing();
}

function pollRefreshing() {
  const timer = setInterval(async () => {
    try {
      const data = await fetchJSON('/api/catalog');
      state.catalog = data;
      renderStats();
      if (!data.refreshing) {
        clearInterval(timer);
        buildGalaxy();
        renderChips();
        applyFilters();
        if (state.selectedId) renderDetail(state.selectedId);
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
    toast(total > 0 ? `queued ${total} package(s)` : 'nothing to queue');
    pollQueueSoon();
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
  buildGalaxy();
  applyFilters();
  renderStats();
  if (state.selectedId === id) renderDetail(id);
}

async function retryFailed() {
  try {
    const result = await fetchJSON('/api/queue/retry', { method: 'POST' });
    toast(result.retried > 0 ? `retried ${result.retried} task(s)` : 'no failed tasks to retry');
    pollQueueSoon();
  } catch (error) {
    toast(`retry failed: ${error.message}`, true);
  }
}

// ---------------------------------------------------------------------
// Galaxy layout — deterministic spiral keyed by package id
// ---------------------------------------------------------------------

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

const ARMS = 4;
function placePackage(pkg) {
  const seed = hash32(pkg.id);
  const rand = rng(seed);
  const security = pkg.is_security || pkg.update_category === 'Security';
  const isUpdate = pkg.status === 'UpdateAvailable';

  // Priority rings: security at the core, updates around it, the installed
  // long tail forming the rim. Favorites sit slightly above the plane.
  const tier = security ? 0 : isUpdate ? 1 : 2;
  const radiusBase = [2.4, 7.5, 13.5][tier];
  const radiusSpread = [3.4, 5.5, 12.5][tier];
  const radius = radiusBase + rand() * radiusSpread;

  const arm = seed % ARMS;
  const armAngle = (arm / ARMS) * Math.PI * 2;
  const spin = radius * 0.28;
  const jitter = (rand() - 0.5) * (0.5 + radius * 0.055);
  const angle = armAngle + spin + jitter;

  const height = (rand() - 0.5) * (1.4 + radius * 0.10) * (tier === 2 ? 1.5 : 0.8)
    + (pkg.is_favorite ? 1.1 : 0);

  return new THREE.Vector3(
    Math.cos(angle) * radius,
    height,
    Math.sin(angle) * radius,
  );
}

function pointColor(pkg) {
  const security = pkg.is_security || pkg.update_category === 'Security';
  if (security) return COLORS.security;
  if (pkg.is_favorite) return COLORS.favorite;
  if (pkg.status === 'UpdateAvailable') return COLORS.update;
  return COLORS.installed;
}

function pointSize(pkg) {
  const security = pkg.is_security || pkg.update_category === 'Security';
  if (security) return 3.8;
  if (pkg.is_favorite) return 3.3;
  if (pkg.status === 'UpdateAvailable') return 2.8;
  return 1.9;
}

// ---------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------

const canvas = document.getElementById('scene');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.setSize(innerWidth, innerHeight);
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.05;

const scene = new THREE.Scene();
scene.fog = new THREE.FogExp2(0x05070d, 0.011);

const camera = new THREE.PerspectiveCamera(52, innerWidth / innerHeight, 0.1, 300);
camera.position.set(9, 16, 30);

const controls = new OrbitControls(camera, canvas);
controls.enableDamping = true;
controls.dampingFactor = 0.06;
controls.minDistance = 6;
controls.maxDistance = 90;
controls.autoRotate = !REDUCED_MOTION;
controls.autoRotateSpeed = 0.22;
let idleTimer = null;
controls.addEventListener('start', () => {
  controls.autoRotate = false;
  clearTimeout(idleTimer);
});
controls.addEventListener('end', () => {
  clearTimeout(idleTimer);
  if (!REDUCED_MOTION) idleTimer = setTimeout(() => { controls.autoRotate = true; }, 9000);
});

const composer = new EffectComposer(renderer);
composer.addPass(new RenderPass(scene, camera));
const bloom = new UnrealBloomPass(new THREE.Vector2(innerWidth, innerHeight), 1.15, 0.55, 0.12);
composer.addPass(bloom);
composer.addPass(new OutputPass());

const galaxy = new THREE.Group();
scene.add(galaxy);

const UNIFORMS = {
  uTime: { value: 0 },
  uScale: { value: innerHeight / 2 },
  uMap: { value: starTexture() },
};

/// A real star sprite: hot gaussian core with four diffraction spikes,
/// drawn once on a canvas — plain round points read as noise at this size.
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
  for (const [x0, y0, x1, y1] of [
    [64, 6, 64, 122],   // vertical spike
    [6, 64, 122, 64],   // horizontal spike
  ]) {
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
      // Whitening hot center over the tinted corona gives each star a
      // real photographic core instead of a flat disc.
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
  const points = new THREE.Points(geometry, POINT_MATERIAL);
  points.frustumCulled = false;
  return points;
}

function buildDust() {
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
}

function buildGalaxy() {
  if (state.points) galaxy.remove(state.points);
  const packages = state.catalog.packages;
  state.packageIndex = packages;
  state.selectedId = null;
  state.selectedIndex = null;
  halo.visible = false;
  state.focus = null;

  state.points = makePoints(packages.length, ({ position, color, size, seed, angle }) => {
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
      angle[i] = ((star >> 10) % 6283) / 1000.0; // 0..2π-ish
    });
  });
  state.positionArray = state.points.geometry.getAttribute('position');
  state.alphaArray = state.points.geometry.getAttribute('aAlpha');
  galaxy.add(state.points);
  buildDust();
}

// ---------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------

const raycaster = new THREE.Raycaster();
raycaster.params.Points.threshold = 0.55;
const pointer = new THREE.Vector2();
let pointerDownAt = null;

canvas.addEventListener('pointerdown', (event) => {
  pointerDownAt = { x: event.clientX, y: event.clientY, t: performance.now() };
});

canvas.addEventListener('pointerup', (event) => {
  if (!pointerDownAt) return;
  const moved = Math.hypot(event.clientX - pointerDownAt.x, event.clientY - pointerDownAt.y);
  const elapsed = performance.now() - pointerDownAt.t;
  pointerDownAt = null;
  if (moved > 8 || elapsed > 500) return; // it was a drag, not a tap

  pointer.x = (event.clientX / innerWidth) * 2 - 1;
  pointer.y = -(event.clientY / innerHeight) * 2 + 1;
  raycaster.setFromCamera(pointer, camera);
  const hits = raycaster.intersectObject(state.points, false);
  const visible = hits.filter(hit => state.alphaArray.array[hit.index] > 0.3);
  if (visible.length === 0) { clearSelection(); return; }
  select(state.packageIndex[visible[0].index], visible[0].index);
});

function ringTexture() {
  const c = document.createElement('canvas');
  c.width = c.height = 128;
  const g = c.getContext('2d');
  g.strokeStyle = 'rgba(94,200,220,0.95)';
  g.lineWidth = 5;
  g.beginPath(); g.arc(64, 64, 50, 0, Math.PI * 2); g.stroke();
  g.strokeStyle = 'rgba(94,200,220,0.3)';
  g.lineWidth = 1.5;
  g.beginPath(); g.arc(64, 64, 60, 0, Math.PI * 2); g.stroke();
  const texture = new THREE.CanvasTexture(c);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

const halo = new THREE.Sprite(new THREE.SpriteMaterial({
  map: ringTexture(),
  transparent: true,
  depthWrite: false,
  blending: THREE.AdditiveBlending,
}));
halo.visible = false;
scene.add(halo);

function selectedWorldPosition(target) {
  if (state.selectedIndex == null) return false;
  target.fromBufferAttribute(state.positionArray, state.selectedIndex);
  galaxy.localToWorld(target);
  return true;
}

function select(pkg, index) {
  state.selectedId = pkg.id;
  state.selectedIndex = index;
  halo.visible = true;
  controls.autoRotate = false;
  renderDetail(pkg.id);
  showPanel('detail');
}

/// Selects a package and eases the camera to it — used by the results sheet.
function flyToPackage(index) {
  const pkg = state.packageIndex[index];
  if (!pkg) return;
  state.selectedId = pkg.id;
  state.selectedIndex = index;
  halo.visible = true;
  controls.autoRotate = false;

  const world = new THREE.Vector3().fromBufferAttribute(state.positionArray, index);
  galaxy.localToWorld(world);
  state.focus = world.clone();
  controls.target.copy(world);

  // Stop at a comfortable distance along the current viewing direction.
  const dir = camera.position.clone().sub(controls.target).normalize();
  if (dir.lengthSq() < 0.5) dir.set(0.4, 0.5, 0.8).normalize();
  state.fly = {
    from: camera.position.clone(),
    to: world.clone().add(dir.multiplyScalar(9)),
    t: 0,
  };
  renderDetail(pkg.id);
  showPanel('detail');
}

function clearSelection() {
  state.selectedId = null;
  state.selectedIndex = null;
  halo.visible = false;
  state.focus = null;
  hidePanel('detail');
}

// ---------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------

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
    case 'security': return pkg.status === 'UpdateAvailable' && (pkg.is_security || pkg.update_category === 'Security');
    case 'favorites': return !!pkg.is_favorite;
    default:
      if (state.filter.startsWith('provider:')) {
        return pkg.source.toLowerCase() === state.filter.slice(9);
      }
      return true;
  }
}

function applyFilters() {
  if (!state.alphaArray) return;
  const packages = state.packageIndex;
  for (let i = 0; i < packages.length; i++) {
    state.alphaArray.array[i] = packageMatches(packages[i]) ? 1 : 0.045;
  }
  state.alphaArray.needsUpdate = true;
  renderResults();
}

// ---------------------------------------------------------------------
// Results sheet — the galaxy is the overview, this is how you find things
// ---------------------------------------------------------------------

const RESULT_LIMIT = 150;

function resultRank(pkg) {
  if (pkg.is_security || pkg.update_category === 'Security') return 0;
  if (pkg.status === 'UpdateAvailable') return 1;
  if (pkg.is_favorite) return 2;
  return 3;
}

function resultRows() {
  const rows = [];
  state.catalog.packages.forEach((pkg, index) => {
    if (packageMatches(pkg)) rows.push({ pkg, index });
  });
  const needle = state.query.text;
  rows.sort((a, b) => {
    // Prefix matches float to the top within each tier.
    const rankDiff = resultRank(a.pkg) - resultRank(b.pkg);
    if (rankDiff !== 0) return rankDiff;
    if (needle) {
      const aPrefix = a.pkg.name.toLowerCase().startsWith(needle) ? 0 : 1;
      const bPrefix = b.pkg.name.toLowerCase().startsWith(needle) ? 0 : 1;
      if (aPrefix !== bPrefix) return aPrefix - bPrefix;
    }
    return a.pkg.name.localeCompare(b.pkg.name);
  });
  return rows;
}

function highlightName(name, needle) {
  if (!needle) return escapeHTML(name);
  const idx = name.toLowerCase().indexOf(needle);
  if (idx < 0) return escapeHTML(name);
  return escapeHTML(name.slice(0, idx))
    + '<span class="mark">' + escapeHTML(name.slice(idx, idx + needle.length)) + '</span>'
    + escapeHTML(name.slice(idx + needle.length));
}

function resultSubLine(pkg) {
  const source = pkg.source.toLowerCase();
  const size = pkg.size != null ? ' · ' + humanSize(pkg.size) : '';
  if (pkg.status === 'UpdateAvailable') {
    const next = pkg.available_version || '?';
    const version = next === pkg.version
      ? `${pkg.version} · new build`
      : `${pkg.version}<span class="arrow">→</span><span class="new">${next}</span>`;
    return `${source} · ${version}${size}`;
  }
  return `${source} · ${pkg.version}${size}`;
}

function renderResults() {
  const sheet = $('results');
  if (!sheet || sheet.classList.contains('hidden')) return;

  const rows = resultRows();
  const total = rows.length;
  const shown = Math.min(total, RESULT_LIMIT);
  $('results-count').textContent = total === 0
    ? 'no matches'
    : shown === total
      ? `${total} package${total === 1 ? '' : 's'}`
      : `${shown} of ${total} packages`;

  const list = $('results-list');
  list.innerHTML = '';
  for (const { pkg, index } of rows.slice(0, RESULT_LIMIT)) {
    const security = pkg.is_security || pkg.update_category === 'Security';
    const glyphClass = security ? 'security' : pkg.status === 'UpdateAvailable' ? 'update' : pkg.is_favorite ? 'favorite' : 'installed';
    const glyph = security ? '⚠' : pkg.status === 'UpdateAvailable' ? '↑' : pkg.is_favorite ? '★' : '·';

    const row = document.createElement('div');
    row.className = 'r-row' + (state.selectedId === pkg.id ? ' active' : '');

    const g = document.createElement('span');
    g.className = `r-glyph ${glyphClass}`;
    g.textContent = glyph;

    const main = document.createElement('div');
    main.className = 'r-main';
    const name = document.createElement('div');
    name.className = 'r-name';
    name.innerHTML = highlightName(pkg.name, state.query.text);
    const sub = document.createElement('div');
    sub.className = 'r-sub';
    sub.innerHTML = resultSubLine(pkg);
    main.appendChild(name);
    main.appendChild(sub);

    row.appendChild(g);
    row.appendChild(main);

    if (pkg.status === 'UpdateAvailable') {
      const update = document.createElement('button');
      update.className = 'r-act accent';
      update.textContent = '↑';
      update.title = 'queue update';
      update.addEventListener('click', (event) => {
        event.stopPropagation();
        enqueue('update', [pkg.id]);
      });
      row.appendChild(update);
    }

    row.addEventListener('click', () => {
      flyToPackage(index);
      hidePanel('results');
    });
    list.appendChild(row);
  }
}

function openResults() {
  showPanel('results');
  renderResults();
}

// ---------------------------------------------------------------------
// HUD rendering
// ---------------------------------------------------------------------

const $ = (id) => document.getElementById(id);

function renderStats() {
  const { counts, refreshing } = state.catalog;
  const favorites = state.catalog.packages.filter(p => p.is_favorite).length;
  const parts = [];
  if (refreshing) parts.push('<span class="stat note">refreshing…</span>');
  if (counts.security > 0) parts.push(`<span class="stat security">⚠ <b>${counts.security}</b> security</span>`);
  if (counts.updates > 0) parts.push(`<span class="stat updates">↑ <b>${counts.updates}</b> updates</span>`);
  if (favorites > 0) parts.push(`<span class="stat favorites">★ <b>${favorites}</b></span>`);
  if (parts.length === 0 && !refreshing) parts.push('<span class="stat dim" style="color:var(--green)">✓ up to date</span>');
  $('stats').innerHTML = parts.join('');
}

function renderChips() {
  const chips = [
    { id: 'all', label: 'all' },
    { id: 'updates', label: `updates ${state.catalog.counts.updates || ''}`.trim() },
    { id: 'security', label: `security ${state.catalog.counts.security || ''}`.trim() },
    { id: 'favorites', label: '★' },
    ...state.catalog.providers.slice(0, 8).map(p => ({ id: `provider:${p.label}`, label: p.label })),
    { id: 'update-all', label: '↑ update all', action: true },
  ];
  const bar = $('chips');
  bar.innerHTML = '';
  for (const chip of chips) {
    const button = document.createElement('button');
    button.className = 'chip' + (state.filter === chip.id ? ' on' : '') + (chip.action ? ' accent' : '');
    button.textContent = chip.label;
    button.addEventListener('click', () => {
      if (chip.action) {
        const ids = state.catalog.packages.filter(p => p.status === 'UpdateAvailable').map(p => p.id);
        enqueue('update', ids);
        return;
      }
      state.filter = chip.id;
      renderResults();
      bar.querySelectorAll('.chip').forEach(el => el.classList.remove('on'));
      button.classList.add('on');
      applyFilters();
    });
    bar.appendChild(button);
  }
}

function humanSize(bytes) {
  if (bytes == null) return '';
  const units = ['B', 'KiB', 'MiB', 'GiB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++; }
  return `${value.toFixed(value >= 10 || unit === 0 ? 0 : 2)} ${units[unit]}`;
}

function renderDetail(id) {
  const pkg = state.catalog.packages.find(p => p.id === id);
  if (!pkg) return;
  $('d-name').textContent = pkg.name;
  $('d-meta').textContent = `${pkg.source.toLowerCase()}${pkg.size ? ' · ' + humanSize(pkg.size) : ''}`;
  $('d-desc').textContent = pkg.description || '';
  $('d-changelog').classList.add('hidden');

  const versions = $('d-versions');
  if (pkg.status === 'UpdateAvailable') {
    const next = pkg.available_version || '?';
    const same = next === pkg.version;
    versions.innerHTML = same
      ? `<span class="same">${pkg.version} · new build</span>`
      : `<span style="color:var(--dim)">${pkg.version}</span><span class="arrow">→</span><span class="new">${next}</span>`;
  } else {
    versions.innerHTML = `<span style="color:var(--dim)">${pkg.version}</span>`;
  }

  const facts = [];
  if (pkg.is_security || pkg.update_category === 'Security') facts.push('security update');
  if (pkg.homepage) facts.push(pkg.homepage);
  $('d-facts').textContent = facts.join(' · ');

  const actions = $('d-actions');
  actions.innerHTML = '';
  if (pkg.status === 'UpdateAvailable') {
    addButton(actions, '↑ update', 'accent', () => enqueue('update', [pkg.id]));
  }
  if (pkg.status === 'UpdateAvailable' || pkg.status === 'Installed') {
    addButton(actions, 'remove', 'danger', () => enqueue('remove', [pkg.id]));
  }
  addButton(actions, pkg.is_favorite ? '★ unstar' : '☆ star', '', () => toggleFavorite(pkg.id));
  addButton(actions, 'changelog', 'ghost', async () => {
    try {
      const data = await fetchJSON(`/api/changelog?id=${encodeURIComponent(pkg.id)}`);
      const box = $('d-changelog');
      box.textContent = data.changelog.slice(0, 4000);
      box.classList.remove('hidden');
    } catch (error) {
      toast(String(error.message).includes('no changelog') ? 'no changelog' : `changelog failed: ${error.message}`, true);
    }
  });
}

function addButton(parent, label, className, onClick) {
  const button = document.createElement('button');
  button.textContent = label;
  if (className) button.className = className;
  button.addEventListener('click', onClick);
  parent.appendChild(button);
}

// ---------------------------------------------------------------------
// Queue drawer + live event stream
// ---------------------------------------------------------------------

function renderQueue() {
  const entries = state.queueEntries;
  const badge = $('queue-badge');
  const running = entries.filter(e => e.status === 'Running').length;
  const queued = entries.filter(e => e.status === 'Queued').length;
  const failed = entries.filter(e => e.status === 'Failed').length;
  const busy = running + queued;

  badge.classList.toggle('hidden', busy + failed === 0);
  badge.classList.toggle('bad', failed > 0 && busy === 0);
  badge.textContent = busy > 0 ? busy : (failed > 0 ? failed : '');

  if (state.queueBusy && busy === 0) {
    // Queue just drained: pick up the new versions.
    setTimeout(loadCatalog, 2500);
  }
  state.queueBusy = busy > 0;

  const list = $('queue-list');
  list.innerHTML = '';
  const order = { Running: 0, Queued: 1, Failed: 2, Completed: 3, Cancelled: 4 };
  const shown = [...entries].sort((a, b) => (order[a.status] ?? 9) - (order[b.status] ?? 9)).slice(0, 40);
  if (shown.length === 0) {
    list.innerHTML = '<div style="color:var(--faint);font-size:13px">the queue is empty</div>';
    renderLogTail();
    return;
  }
  for (const entry of shown) {
    const row = document.createElement('div');
    row.className = `q-entry ${entry.status === 'Running' ? 'running' : entry.status === 'Failed' ? 'failed' : entry.status === 'Completed' ? 'done' : ''}`;
    row.innerHTML = `<span class="q-dot"></span><span class="q-name">${escapeHTML(entry.package_name)}</span><span class="q-action">${entry.action.toLowerCase()}</span>`;
    list.appendChild(row);
    if (entry.error) {
      const err = document.createElement('div');
      err.className = 'q-error';
      err.textContent = errorBrief(entry);
      list.appendChild(err);
    }
    const log = entryLog(entry.id);
    if (entry.status === 'Running' && log) {
      const line = document.createElement('div');
      line.className = 'q-log';
      line.textContent = `↳ ${log}`;
      list.appendChild(line);
    }
  }
  renderLogTail();
}

/// Latest buffered output line for a task, if any.
function entryLog(entryId) {
  const buffer = state.entryLogs.get(entryId);
  return buffer && buffer.length > 0 ? buffer[buffer.length - 1] : null;
}

function pushEntryLog(entryId, line) {
  let buffer = state.entryLogs.get(entryId);
  if (!buffer) { buffer = []; state.entryLogs.set(entryId, buffer); }
  buffer.push(line);
  if (buffer.length > 24) buffer.shift();

  // Resolve the display name once, then append to the global tail.
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
  const lines = state.logTail.slice(-14);
  if (lines.length === 0) { wrap.classList.add('hidden'); return; }
  wrap.classList.remove('hidden');
  const nearBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 40;
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
  if (entry.status !== 'Running' && entry.status !== 'Queued') {
    // Keep terminal entries around briefly for context, trimmed hard so
    // the drawer never grows unbounded.
    if (state.queueEntries.length > 60) state.queueEntries.length = 60;
  }
}

function connectEventStream() {
  const source = new EventSource('/api/queue/stream');

  source.addEventListener('state', (event) => {
    const data = JSON.parse(event.data);
    state.queueEntries = data.entries ?? [];
    renderQueue();
  });

  source.addEventListener('task', (event) => {
    // WebQueueEvent is serde-tagged, so entry fields arrive flattened
    // alongside the discriminant.
    const entry = JSON.parse(event.data);
    const wasRunning = state.queueEntries.some(e => e.id === entry.id && e.status === 'Running');
    upsertQueueEntry(entry);
    if (wasRunning && entry.status === 'Completed') {
      toast(`${entry.package_name} ${entry.action.toLowerCase()} done`);
    } else if (wasRunning && entry.status === 'Failed') {
      toast(`${entry.package_name} failed — open the queue`, true);
    }
    renderQueue();
    // Refresh catalog when the queue drains so versions update.
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

function escapeHTML(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

let queueTimer = null;
function pollQueueSoon() { renderQueueLoop(); setTimeout(renderQueueLoop, 1500); }
async function renderQueueLoop() {
  try {
    const data = await fetchJSON('/api/queue');
    state.queueEntries = data.entries ?? [];
    renderQueue();
  } catch { /* transient; SSE carries the live updates */ }
  clearTimeout(queueTimer);
  // SSE is the live path; this poll only backstops a dropped connection.
  queueTimer = setTimeout(renderQueueLoop, 15000);
}

// ---------------------------------------------------------------------
// Panels, toast, boot
// ---------------------------------------------------------------------

function showPanel(id) { $(id).classList.remove('hidden'); }
function hidePanel(id) { $(id).classList.add('hidden'); }

document.querySelectorAll('[data-close]').forEach(button => {
  button.addEventListener('click', () => hidePanel(button.dataset.close));
});

let toastTimer = null;
function toast(message, isError = false) {
  const el = $('toast');
  el.textContent = message;
  el.classList.toggle('err', isError);
  el.classList.remove('hidden');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.add('hidden'), isError ? 5000 : 2600);
}

function hideBoot() {
  const boot = $('boot');
  boot.classList.add('gone');
  setTimeout(() => boot.remove(), 600);
}

// ---------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------

$('search').addEventListener('input', (event) => {
  const tokens = event.target.value.trim().split(/\s+/).filter(Boolean);
  state.query.source = null;
  const words = [];
  for (const token of tokens) {
    if (token.startsWith('src:') && token.length > 4) state.query.source = token.slice(4).toLowerCase();
    else words.push(token.toLowerCase());
  }
  state.query.text = words.join(' ');
  applyFilters();
  // Typing opens the results sheet — dimming dots alone is not finding
  // anything, the list is.
  if (state.query.text || state.query.source) openResults();
});

$('refresh').addEventListener('click', refreshCatalog);
$('list-btn').addEventListener('click', () => {
  const open = !$('results').classList.contains('hidden');
  if (open) hidePanel('results');
  else openResults();
});
$('queue-btn').addEventListener('click', () => {
  const open = !$('queue-drawer').classList.contains('hidden');
  if (open) hidePanel('queue-drawer');
  else { showPanel('queue-drawer'); pollQueueSoon(); }
});
$('retry').addEventListener('click', retryFailed);

// ---------------------------------------------------------------------
// Frame loop
// ---------------------------------------------------------------------

const clock = new THREE.Clock();
let paused = false;
document.addEventListener('visibilitychange', () => { paused = document.hidden; });

const focusTarget = new THREE.Vector3();

function animate() {
  requestAnimationFrame(animate);
  if (paused) return;
  const dt = clock.getDelta();
  const time = clock.elapsedTime;
  UNIFORMS.uTime.value = time;

  if (!REDUCED_MOTION) galaxy.rotation.y = time * 0.004;

  if (state.fly) {
    state.fly.t = Math.min(1, state.fly.t + dt / 0.9);
    const k = state.fly.t * state.fly.t * (3 - 2 * state.fly.t);
    camera.position.lerpVectors(state.fly.from, state.fly.to, k);
    if (state.fly.t >= 1) state.fly = null;
  }

  // The halo and camera focus track the selected point as the galaxy turns.
  if (selectedWorldPosition(focusTarget)) {
    state.focus = state.focus || focusTarget.clone();
    state.focus.lerp(focusTarget, 0.12);
    halo.position.copy(state.focus);
    const pulse = 1 + 0.12 * Math.sin(time * 3.2);
    halo.scale.setScalar(2.6 * pulse);
    controls.target.lerp(state.focus, 0.05);
  }

  controls.update();
  composer.render();
}

addEventListener('resize', () => {
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(innerWidth, innerHeight);
  composer.setSize(innerWidth, innerHeight);
  UNIFORMS.uScale.value = innerHeight / 2;
});

// ---------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------

animate();
loadCatalog().catch(error => {
  $('boot-text').textContent = `failed to load catalog: ${error.message}`;
});
connectEventStream();
renderQueueLoop();
