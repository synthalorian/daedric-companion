const { invoke } = window.__TAURI__.core;

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
  } catch (e) {
    $('pulse-content').classList.add('hidden');
    setError('pulse', String(e));
  } finally {
    setLoading('pulse', false);
  }
}

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

async function loadCollection() {
  setLoading('collection', true);
  setError('collection', null);
  try {
    const c = await invoke('get_collection', { host: host() });
    $('collection-slug').textContent = c.slug || '—';
    $('collection-revision').textContent = c.revision ?? '—';
    $('collection-mods').textContent = c.mods_count ?? '—';
    $('collection-content').classList.remove('hidden');
  } catch (e) {
    setError('collection', String(e));
  } finally {
    setLoading('collection', false);
  }
}

function refreshAll() {
  loadPulse();
  loadNews();
  loadLauncher();
  loadCollection();
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
$('apply-host').addEventListener('click', refreshAll);
$('host-input').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') refreshAll();
});
$('auto-refresh').addEventListener('change', armAutoRefresh);

armAutoRefresh();
refreshAll();
