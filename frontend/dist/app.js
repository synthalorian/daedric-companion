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

// ---------- tab navigation ----------

document.querySelectorAll('.tab').forEach((tab) => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach((t) => t.classList.remove('active'));
    tab.classList.add('active');
    document.querySelectorAll('.view').forEach((v) => v.classList.add('hidden'));
    $('view-' + tab.dataset.view).classList.remove('hidden');
  });
});

// ---------- RP profile store ----------

let profile = null;

async function loadProfile() {
  try {
    profile = await invoke('profile_load');
    renderCharacter();
    renderContacts();
    renderJournal();
    renderRumors();
  } catch (e) {
    toast('profile load failed: ' + String(e));
  }
}

// ---------- character sheet ----------

const CH_TEXT = [
  'name', 'race', 'sex', 'birthsign', 'deity', 'ingame-date',
  'faction', 'rank', 'occupation', 'appearance', 'personality',
  'backstory', 'notes',
];
const CH_NUM = ['level', 'health', 'magicka', 'stamina'];

function renderCharacter() {
  if (!profile) return;
  const c = profile.character || {};
  CH_TEXT.forEach((f) => {
    $('ch-' + f).value = c[f.replace(/-/g, '_')] || '';
  });
  CH_NUM.forEach((f) => {
    $('ch-' + f).value = c[f] ?? '';
  });
  renderSkillRows(c.skills || []);
}

function renderSkillRows(skills) {
  const wrap = $('skill-rows');
  wrap.innerHTML = '';
  skills.forEach((s, i) => {
    const row = document.createElement('div');
    row.className = 'skill-row';
    const name = document.createElement('input');
    name.type = 'text';
    name.value = s.name || '';
    name.placeholder = 'skill';
    name.spellcheck = false;
    const level = document.createElement('input');
    level.type = 'number';
    level.min = '0';
    level.max = '100';
    level.value = s.level ?? 0;
    const rm = document.createElement('button');
    rm.className = 'btn small danger';
    rm.textContent = '×';
    rm.addEventListener('click', () => row.remove());
    row.appendChild(name);
    row.appendChild(level);
    row.appendChild(rm);
    wrap.appendChild(row);
    name.dataset.idx = i;
  });
}

$('skill-add').addEventListener('click', () => {
  const rows = $('skill-rows');
  const current = [...rows.querySelectorAll('.skill-row')].map((r) => ({
    name: r.querySelector('input[type=text]').value,
    level: parseInt(r.querySelector('input[type=number]').value, 10) || 0,
  }));
  current.push({ name: '', level: 0 });
  renderSkillRows(current);
});

function collectSkills() {
  return [...$('skill-rows').querySelectorAll('.skill-row')]
    .map((r) => ({
      name: r.querySelector('input[type=text]').value.trim(),
      level: parseInt(r.querySelector('input[type=number]').value, 10) || 0,
    }))
    .filter((s) => s.name.length > 0);
}

$('character-save').addEventListener('click', async () => {
  const character = {};
  CH_TEXT.forEach((f) => {
    character[f.replace(/-/g, '_')] = $('ch-' + f).value.trim();
  });
  CH_NUM.forEach((f) => {
    const v = $('ch-' + f).value;
    character[f] = v === '' ? null : parseInt(v, 10);
  });
  character.skills = collectSkills();
  try {
    await invoke('character_save', { character });
    profile.character = character;
    const flash = $('character-saved');
    flash.classList.remove('hidden');
    setTimeout(() => flash.classList.add('hidden'), 2500);
  } catch (e) {
    toast('save failed: ' + String(e));
  }
});

// ---------- contacts ----------

const REL_LABELS = { '-2': 'HATED', '-1': 'COLD', 0: 'STRANGER', 1: 'WARM', 2: 'SWORN' };
let editingContactId = null;

function openContactEditor(contact) {
  editingContactId = contact ? contact.id : null;
  $('contact-editor-title').textContent = contact ? 'EDIT CONTACT' : 'NEW CONTACT';
  $('ct-name').value = contact?.name || '';
  $('ct-race').value = contact?.race || '';
  $('ct-faction').value = contact?.faction || '';
  $('ct-role').value = contact?.role || '';
  $('ct-met-at').value = contact?.met_at || '';
  $('ct-first-met').value = contact?.first_met || '';
  $('ct-last-seen').value = contact?.last_seen || '';
  $('ct-relationship').value = String(contact?.relationship ?? 0);
  $('ct-alive').checked = contact ? contact.alive : true;
  $('ct-notes').value = contact?.notes || '';
  $('contact-editor').classList.remove('hidden');
  $('ct-name').focus();
}

function closeContactEditor() {
  editingContactId = null;
  $('contact-editor').classList.add('hidden');
}

$('contact-new').addEventListener('click', () => openContactEditor(null));
$('contact-cancel').addEventListener('click', closeContactEditor);

$('contact-save').addEventListener('click', async () => {
  const contact = {
    id: editingContactId || '',
    name: $('ct-name').value.trim(),
    race: $('ct-race').value.trim(),
    faction: $('ct-faction').value.trim(),
    role: $('ct-role').value.trim(),
    met_at: $('ct-met-at').value.trim(),
    first_met: $('ct-first-met').value.trim(),
    last_seen: $('ct-last-seen').value.trim(),
    relationship: parseInt($('ct-relationship').value, 10),
    alive: $('ct-alive').checked,
    notes: $('ct-notes').value.trim(),
  };
  if (!contact.name) {
    toast('a contact needs a name');
    return;
  }
  try {
    await invoke('contact_save', { contact });
    await loadProfile();
    closeContactEditor();
  } catch (e) {
    toast('save failed: ' + String(e));
  }
});

$('contact-search').addEventListener('input', renderContacts);

function renderContacts() {
  if (!profile) return;
  const q = ($('contact-search').value || '').toLowerCase();
  const list = $('contact-list');
  list.innerHTML = '';
  const contacts = (profile.contacts || []).filter((c) => {
    if (!q) return true;
    return [c.name, c.race, c.faction, c.role, c.met_at, c.notes]
      .join(' ')
      .toLowerCase()
      .includes(q);
  });
  if (contacts.length === 0) {
    list.innerHTML = '<p class="empty-note">no souls recorded yet — meet someone worth remembering.</p>';
    return;
  }
  contacts.forEach((c) => {
    const card = document.createElement('div');
    card.className = 'card contact-card' + (c.alive ? '' : ' dead');

    const head = document.createElement('div');
    head.className = 'card-head';
    const name = document.createElement('span');
    name.className = 'card-title';
    name.textContent = c.name + (c.alive ? '' : ' †');
    const rel = document.createElement('span');
    rel.className = 'rel-badge rel-' + c.relationship;
    rel.textContent = REL_LABELS[c.relationship] ?? 'STRANGER';
    head.appendChild(name);
    head.appendChild(rel);
    card.appendChild(head);

    const metaBits = [c.race, c.role, c.faction].filter(Boolean);
    if (metaBits.length) {
      const meta = document.createElement('div');
      meta.className = 'card-meta';
      meta.textContent = metaBits.join(' · ');
      card.appendChild(meta);
    }
    const whereBits = [
      c.met_at && 'met at ' + c.met_at,
      c.first_met && 'first met ' + c.first_met,
      c.last_seen && 'last seen ' + c.last_seen,
    ].filter(Boolean);
    if (whereBits.length) {
      const where = document.createElement('div');
      where.className = 'card-meta dim';
      where.textContent = whereBits.join(' · ');
      card.appendChild(where);
    }
    if (c.notes) {
      const notes = document.createElement('p');
      notes.className = 'card-body';
      notes.textContent = c.notes;
      card.appendChild(notes);
    }

    const actions = document.createElement('div');
    actions.className = 'card-actions';
    const edit = document.createElement('button');
    edit.className = 'btn small';
    edit.textContent = 'EDIT';
    edit.addEventListener('click', () => openContactEditor(c));
    const del = document.createElement('button');
    del.className = 'btn small danger';
    del.textContent = 'FORGET';
    del.addEventListener('click', async () => {
      await invoke('contact_delete', { id: c.id });
      await loadProfile();
    });
    actions.appendChild(edit);
    actions.appendChild(del);
    card.appendChild(actions);
    list.appendChild(card);
  });
}

// ---------- chronicle ----------

$('journal-new').addEventListener('click', () => {
  $('jn-title').value = '';
  $('jn-location').value = '';
  $('jn-body').value = '';
  $('journal-editor').classList.remove('hidden');
  $('jn-title').focus();
});
$('journal-cancel').addEventListener('click', () => {
  $('journal-editor').classList.add('hidden');
});
$('journal-save').addEventListener('click', async () => {
  const title = $('jn-title').value.trim();
  if (!title) {
    toast('an entry needs a title');
    return;
  }
  try {
    await invoke('journal_add', {
      title,
      location: $('jn-location').value.trim(),
      body: $('jn-body').value.trim(),
    });
    $('journal-editor').classList.add('hidden');
    await loadProfile();
  } catch (e) {
    toast('could not write it down: ' + String(e));
  }
});

function renderJournal() {
  if (!profile) return;
  const list = $('journal-list');
  list.innerHTML = '';
  const entries = profile.journal || [];
  if (entries.length === 0) {
    list.innerHTML = '<p class="empty-note">the chronicle is empty — the road has not been walked yet.</p>';
    return;
  }
  entries.forEach((e) => {
    const card = document.createElement('div');
    card.className = 'card journal-card';
    const head = document.createElement('div');
    head.className = 'card-head';
    const title = document.createElement('span');
    title.className = 'card-title';
    title.textContent = e.title;
    const when = document.createElement('span');
    when.className = 'card-meta dim';
    when.textContent =
      new Date(e.created).toLocaleString() + (e.location ? ' · ' + e.location : '');
    head.appendChild(title);
    head.appendChild(when);
    card.appendChild(head);
    if (e.body) {
      const body = document.createElement('p');
      body.className = 'card-body';
      body.textContent = e.body;
      card.appendChild(body);
    }
    const actions = document.createElement('div');
    actions.className = 'card-actions';
    const del = document.createElement('button');
    del.className = 'btn small danger';
    del.textContent = 'BURN';
    del.addEventListener('click', async () => {
      await invoke('journal_delete', { id: e.id });
      await loadProfile();
    });
    actions.appendChild(del);
    card.appendChild(actions);
    list.appendChild(card);
  });
}

// ---------- rumors ----------

$('rumor-add').addEventListener('click', async () => {
  const text = $('rm-text').value.trim();
  if (!text) return;
  try {
    await invoke('rumor_add', { text, source: $('rm-source').value.trim() });
    $('rm-text').value = '';
    $('rm-source').value = '';
    await loadProfile();
  } catch (e) {
    toast('rumor lost: ' + String(e));
  }
});

function renderRumors() {
  if (!profile) return;
  const list = $('rumor-list');
  list.innerHTML = '';
  const rumors = profile.rumors || [];
  if (rumors.length === 0) {
    list.innerHTML = '<p class="empty-note">no whispers on the wind.</p>';
    return;
  }
  rumors.forEach((r) => {
    const card = document.createElement('div');
    card.className = 'card rumor-card' + (r.done ? ' done' : '');
    const label = document.createElement('label');
    label.className = 'rumor-check';
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = r.done;
    cb.addEventListener('change', async () => {
      await invoke('rumor_toggle', { id: r.id });
      await loadProfile();
    });
    const text = document.createElement('span');
    text.className = 'rumor-text';
    text.textContent = r.text;
    label.appendChild(cb);
    label.appendChild(text);
    card.appendChild(label);
    const meta = document.createElement('div');
    meta.className = 'card-meta dim';
    meta.textContent =
      (r.source ? 'heard from ' + r.source + ' · ' : '') +
      new Date(r.created).toLocaleDateString();
    card.appendChild(meta);
    const del = document.createElement('button');
    del.className = 'btn small danger';
    del.textContent = '×';
    del.addEventListener('click', async () => {
      await invoke('rumor_delete', { id: r.id });
      await loadProfile();
    });
    card.appendChild(del);
    list.appendChild(card);
  });
}

// ---------- dice tray ----------

const diceHistory = [];

function rollDie(sides) {
  return 1 + Math.floor(Math.random() * sides);
}

function doRoll(count, sides) {
  const rolls = [];
  for (let i = 0; i < count; i++) rolls.push(rollDie(sides));
  const total = rolls.reduce((a, b) => a + b, 0);
  const label = `${count}d${sides}`;
  diceHistory.unshift({ label, rolls, total });
  if (diceHistory.length > 30) diceHistory.pop();
  renderDice(label, rolls, total);
}

function renderDice(label, rolls, total) {
  const res = $('dice-result');
  res.classList.remove('hidden');
  res.innerHTML = '';
  const big = document.createElement('span');
  big.className = 'dice-total';
  big.textContent = total;
  const detail = document.createElement('span');
  detail.className = 'dice-detail';
  detail.textContent = label + (rolls.length > 1 ? ' → ' + rolls.join(' + ') : '');
  res.appendChild(big);
  res.appendChild(detail);

  const hist = $('dice-history');
  hist.innerHTML = '';
  diceHistory.forEach((h) => {
    const li = document.createElement('li');
    li.textContent =
      h.label + ' = ' + h.total + (h.rolls.length > 1 ? '  (' + h.rolls.join(', ') + ')' : '');
    hist.appendChild(li);
  });
}

document.querySelectorAll('.die').forEach((btn) => {
  btn.addEventListener('click', () => doRoll(1, parseInt(btn.dataset.sides, 10)));
});
$('dice-roll-custom').addEventListener('click', () => {
  const count = Math.max(1, Math.min(20, parseInt($('dice-count').value, 10) || 1));
  const sides = Math.max(2, Math.min(1000, parseInt($('dice-sides').value, 10) || 20));
  doRoll(count, sides);
});
$('dice-clear').addEventListener('click', () => {
  diceHistory.length = 0;
  $('dice-history').innerHTML = '';
  $('dice-result').classList.add('hidden');
});

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
loadProfile();
