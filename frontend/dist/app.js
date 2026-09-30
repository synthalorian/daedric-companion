const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);

function host() {
  return $('host-input').value.trim() || 'play.daedriconline.com';
}

function setLoading(name, loading) {
  $('spinner-' + name).classList.toggle('hidden', !loading);
}

function setError(name, msg) {
  const el = $(name + '-error');
  if (msg) {
    el.textContent = msg;
    el.classList.remove('hidden');
  } else {
    el.classList.add('hidden');
  }
}

// ---------- toast ----------

let toastTimer = null;
function toast(msg) {
  const t = $('toast');
  t.textContent = msg;
  t.classList.remove('hidden');
  t.classList.add('toast-show');
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    t.classList.remove('toast-show');
    t.classList.add('hidden');
  }, 9000);
}

// ---------- pulse history + sparklines ----------

const HISTORY_KEY = 'daedric-pulse-history';
const HISTORY_CAP = 2880; // ~24 h at 30 s auto-refresh

function loadHistory() {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    const arr = raw ? JSON.parse(raw) : [];
    return Array.isArray(arr) ? arr : [];
  } catch {
    return [];
  }
}

function pushSample(sample) {
  const h = loadHistory();
  h.push(sample);
  while (h.length > HISTORY_CAP) h.shift();
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(h));
  } catch {
    /* storage full — history is a luxury, drop it */
  }
}

function drawSpark(canvasId, values, color, fill) {
  const canvas = $(canvasId);
  const ctx = canvas.getContext('2d');
  const w = canvas.width;
  const hgt = canvas.height;
  ctx.clearRect(0, 0, w, hgt);
  const clean = values.filter((v) => v != null);
  if (clean.length < 2) {
    ctx.fillStyle = 'rgba(143,149,156,0.4)';
    ctx.font = '9px monospace';
    ctx.fillText('gathering samples…', 6, hgt / 2 + 3);
    return;
  }
  const min = Math.min(...clean);
  const max = Math.max(...clean);
  const span = max - min || 1;
  const pad = 3;
  const step = w / (values.length - 1 || 1);
  const pt = (i, v) => [
    i * step,
    v == null ? hgt - pad : hgt - pad - ((v - min) / span) * (hgt - pad * 2),
  ];

  if (fill) {
    ctx.beginPath();
    ctx.moveTo(0, hgt);
    values.forEach((v, i) => {
      const [x, y] = pt(i, v);
      ctx.lineTo(x, y);
    });
    ctx.lineTo((values.length - 1) * step, hgt);
    ctx.closePath();
    ctx.fillStyle = fill;
    ctx.fill();
  }
  ctx.beginPath();
  let started = false;
  values.forEach((v, i) => {
    if (v == null) return;
    const [x, y] = pt(i, v);
    if (!started) {
      ctx.moveTo(x, y);
      started = true;
    } else {
      ctx.lineTo(x, y);
    }
  });
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.stroke();

  // min/max labels
  ctx.fillStyle = 'rgba(216,211,200,0.55)';
  ctx.font = '8px monospace';
  ctx.fillText(String(max), 2, 8);
  ctx.fillText(String(min), 2, hgt - 2);
}

function renderSparklines() {
  const h = loadHistory();
  drawSpark(
    'spark-players',
    h.map((s) => (s.online ? s.players : null)),
    '#d8d3c8',
    'rgba(143,149,156,0.15)'
  );
  drawSpark(
    'spark-latency',
    h.map((s) => s.latencyMs),
    '#c1121f',
    null
  );
}

// ---------- uptime tracking ----------

function fmtDuration(ms) {
  const mins = Math.floor(ms / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return mins + 'm';
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return hrs + 'h ' + (mins % 60) + 'm';
  return Math.floor(hrs / 24) + 'd ' + (hrs % 24) + 'h';
}

function renderUptime() {
  const h = loadHistory();
  if (h.length === 0) return;
  const nowOnline = h[h.length - 1].online;
  let since = h[h.length - 1].t;
  let flips = 0;
  for (let i = h.length - 2; i >= 0; i--) {
    if (h[i].online !== nowOnline) {
      flips++;
      if (flips === 1) since = h[i + 1].t;
      else break;
    }
  }
  const el = $('uptime-line');
  const span = fmtDuration(Date.now() - since);
  el.textContent = nowOnline
    ? `online for ${span}${flips > 1 ? ' · ' + (flips - 1) + ' outage(s) in history' : ''}`
    : `OFFLINE for ${span}`;
  el.classList.toggle('down', !nowOnline);
}

// ---------- pulse ----------

async function loadPulse() {
  setLoading('pulse', true);
  setError('pulse', null);
  try {
    const p = await invoke('get_pulse', { host: host() });
    const badge = $('pulse-badge');
    badge.textContent = p.online ? 'ONLINE' : 'OFFLINE';
    badge.className = 'badge ' + (p.online ? 'online' : 'offline');
    $('pulse-players').textContent =
      (p.players ?? '—') + '/' + (p.max_players ?? '—');
    $('pulse-latency').textContent =
      p.latency_ms != null ? p.latency_ms + ' ms' : '— ms';
    $('pulse-content').classList.remove('hidden');
    pushSample({
      t: Date.now(),
      online: p.online,
      players: p.players ?? null,
      maxPlayers: p.max_players ?? null,
      latencyMs: p.latency_ms ?? null,
    });
    renderSparklines();
    renderUptime();
  } catch (e) {
    $('pulse-content').classList.add('hidden');
    setError('pulse', String(e));
  } finally {
    setLoading('pulse', false);
  }
}

// ---------- connection quality ----------

async function loadPingStats() {
  const btn = $('ping-quality');
  const box = $('ping-stats');
  btn.disabled = true;
  btn.textContent = 'PINGING…';
  try {
    const s = await invoke('get_ping_stats', { host: host(), count: 5 });
    box.textContent =
      s.answered === 0
        ? `0/${s.sent} answered — server unreachable on UDP 7777`
        : `${s.answered}/${s.sent} answered · loss ${s.loss_pct}% · min ${s.min_ms} / avg ${s.avg_ms} / max ${s.max_ms} ms`;
    box.classList.remove('hidden');
  } catch (e) {
    box.textContent = 'quality check failed: ' + String(e);
    box.classList.remove('hidden');
  } finally {
    btn.disabled = false;
    btn.textContent = 'QUALITY (5 PINGS)';
  }
}

// ---------- news ----------

async function loadNews() {
  setLoading('news', true);
  setError('news', null);
  try {
    const feed = await invoke('get_news', { host: host() });
    const list = $('news-list');
    list.innerHTML = '';
    const posts = (feed && feed.posts) || [];
    for (const post of posts) {
      const div = document.createElement('div');
      div.className = 'news-post';
      const meta = document.createElement('div');
      meta.className = 'news-meta';
      const chip = document.createElement('span');
      chip.className = 'chip';
      chip.textContent = post.chip || 'NEWS';
      const date = document.createElement('span');
      date.className = 'news-date';
      date.textContent = post.date || '';
      meta.appendChild(chip);
      meta.appendChild(date);
      const ul = document.createElement('ul');
      ul.className = 'news-items';
      for (const item of post.items || []) {
        const li = document.createElement('li');
        li.textContent = item;
        ul.appendChild(li);
      }
      div.appendChild(meta);
      div.appendChild(ul);
      list.appendChild(div);
    }
    list.classList.remove('hidden');
  } catch (e) {
    setError('news', String(e));
  } finally {
    setLoading('news', false);
  }
}

// ---------- launcher ----------

async function loadLauncher() {
  setLoading('launcher', true);
  setError('launcher', null);
  try {
    const l = await invoke('get_latest_launcher', { host: host() });
    $('launcher-version').textContent = l.version || '—';
    $('launcher-note').textContent = l.raw || '';
    $('launcher-content').classList.remove('hidden');
  } catch (e) {
    setError('launcher', String(e));
  } finally {
    setLoading('launcher', false);
  }
}

// ---------- collection ----------

function countText(v) {
  if (v == null) return '—';
  return String(v);
}

async function loadCollection() {
  setLoading('collection', true);
  setError('collection', null);
  try {
    const c = await invoke('get_collection', { host: host() });
    $('collection-slug').textContent = c.slug || '—';
    $('collection-revision').textContent =
      (c.revision ?? '—') +
      (c.pinned_revision != null && c.pinned_revision !== c.revision
        ? ' (pinned ' + c.pinned_revision + ')'
        : '');
    $('collection-mods').textContent =
      countText(c.mods_count) + ' / ' + countText(c.files_count);
    $('collection-published').textContent = c.published_at
      ? new Date(c.published_at).toLocaleString()
      : '—';
    $('collection-sha').textContent = c.file_set_sha256
      ? c.file_set_sha256.slice(0, 20) + '…'
      : '—';
    const link = $('collection-link');
    if (c.url) {
      link.href = c.url;
      link.classList.remove('hidden');
    } else {
      link.classList.add('hidden');
    }
    $('collection-content').classList.remove('hidden');
  } catch (e) {
    setError('collection', String(e));
  } finally {
    setLoading('collection', false);
  }
}

// ---------- client update ----------

async function loadUpdates() {
  setLoading('update', true);
  setError('update', null);
  try {
    const u = await invoke('check_updates', {
      skyrimRoot: $('skyrim-root-input').value.trim(),
      host: host(),
    });
    const badge = $('update-badge');
    if (u.overlay_update_available) {
      badge.textContent = 'UPDATE AVAILABLE';
      badge.className = 'badge update-available';
    } else {
      badge.textContent = 'UP TO DATE';
      badge.className = 'badge online';
    }
    $('update-installed').textContent = u.installed_overlay || 'not found';
    $('update-server-overlay').textContent = u.server_overlay || '—';
    $('update-launcher-feed').textContent = u.latest_launcher_feed || '—';
    $('update-note').textContent = u.note || '';
    $('update-content').classList.remove('hidden');
  } catch (e) {
    $('update-content').classList.add('hidden');
    setError('update', String(e));
  } finally {
    setLoading('update', false);
  }
}

// ---------- update watcher (background push) ----------

async function startWatcher() {
  try {
    const started = await invoke('start_update_watcher', {
      skyrimRoot: $('skyrim-root-input').value.trim(),
      host: host(),
    });
    if (started) $('watcher-badge').classList.remove('hidden');
  } catch {
    /* watcher is best-effort */
  }
  await listen('update-event', (ev) => {
    const p = ev.payload || {};
    const what =
      p.kind === 'overlay'
        ? 'overlay ' + (p.overlay_version || '?')
        : p.kind === 'launcher'
          ? 'launcher ' + (p.launcher_version || '?')
          : 'overlay ' +
            (p.overlay_version || '?') +
            ' + launcher ' +
            (p.launcher_version || '?');
    toast('SERVER PUSHED NEW ' + what.toUpperCase() + ' — run the launcher');
    loadUpdates();
  });
}

// ---------- wiring ----------

function refreshAll() {
  loadPulse();
  loadNews();
  loadLauncher();
  loadCollection();
  loadUpdates();
}

let timer = null;
function armAutoRefresh() {
  if (timer) clearInterval(timer);
  timer = null;
  if ($('auto-refresh').checked) {
    timer = setInterval(loadPulse, 30000);
  }
}

$('refresh-pulse').addEventListener('click', loadPulse);
$('refresh-news').addEventListener('click', loadNews);
$('refresh-launcher').addEventListener('click', loadLauncher);
$('refresh-collection').addEventListener('click', loadCollection);
$('refresh-update').addEventListener('click', loadUpdates);
$('ping-quality').addEventListener('click', loadPingStats);
$('apply-host').addEventListener('click', refreshAll);
$('host-input').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') refreshAll();
});
$('auto-refresh').addEventListener('change', armAutoRefresh);

armAutoRefresh();
refreshAll();
startWatcher();
renderSparklines();
renderUptime();
