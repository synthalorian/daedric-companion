//! daedric-companion — Tauri 2 desktop dashboard backed by daedric-api.

mod store;

use daedric_api::{Client, DEFAULT_HOST};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use store::{Character, Contact, JournalEntry, Profile, Rumor, Store};
use tauri::{Emitter, Manager};

fn host_or_default(host: Option<String>) -> String {
    host.filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HOST.to_string())
}

#[derive(Debug, Serialize)]
pub struct PulseDto {
    players: Option<u32>,
    max_players: Option<u32>,
    latency_ms: Option<u64>,
    online: bool,
}

#[derive(Debug, Serialize)]
pub struct LauncherInfo {
    version: Option<String>,
    raw: String,
}

#[tauri::command]
async fn get_pulse(host: Option<String>) -> Result<PulseDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = daedric_api::pulse(&host_or_default(host));
        Ok(PulseDto {
            players: p.players,
            max_players: p.max_players,
            latency_ms: p.latency.map(|d| d.as_millis() as u64),
            online: p.online,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_news(host: Option<String>) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = Client::new(&host_or_default(host));
        client.news().map(|feed| feed.0).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_latest_launcher(host: Option<String>) -> Result<LauncherInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = Client::new(&host_or_default(host));
        let raw = client.latest_launcher().map_err(|e| e.to_string())?;
        let version = raw
            .lines()
            .find_map(|line| {
                line.trim()
                    .strip_prefix("version:")
                    .map(|v| v.trim().to_string())
            })
            .filter(|v| !v.is_empty());
        Ok(LauncherInfo { version, raw })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Serialize)]
pub struct CollectionSummary {
    slug: Option<String>,
    revision: Option<serde_json::Value>,
    /// Server sends counts as ints; older builds sent arrays — keep loose.
    mods_count: Option<serde_json::Value>,
    files_count: Option<serde_json::Value>,
    url: Option<String>,
    published_at: Option<String>,
    pinned_revision: Option<serde_json::Value>,
    file_set_sha256: Option<String>,
}

#[tauri::command]
async fn get_collection(host: Option<String>) -> Result<CollectionSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = Client::new(&host_or_default(host));
        let value = client.collection().map_err(|e| e.to_string())?;
        let get_str = |k: &str| value.get(k).and_then(|s| s.as_str()).map(|s| s.to_string());
        let count_of = |k: &str| -> Option<serde_json::Value> {
            match value.get(k) {
                Some(serde_json::Value::Array(a)) => Some(serde_json::Value::from(a.len())),
                other => other.cloned(),
            }
        };
        Ok(CollectionSummary {
            slug: get_str("slug"),
            revision: value.get("revision").cloned(),
            mods_count: count_of("mods"),
            files_count: count_of("files"),
            url: get_str("url"),
            published_at: get_str("publishedAt"),
            pinned_revision: value.get("pinnedRevision").cloned(),
            file_set_sha256: get_str("fileSetSha256"),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_ping_stats(
    host: Option<String>,
    count: Option<u32>,
) -> Result<daedric_api::PingStats, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = Client::new(&host_or_default(host));
        Ok(client.ping_stats(count.unwrap_or(5).clamp(1, 20)))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Read the installed overlay version marker: current path first, then the
/// launcher's legacy fallback. Mirrors `readInstalledVersion()` in the launcher.
fn read_installed_overlay(skyrim_root: &str) -> Option<String> {
    let root = std::path::Path::new(skyrim_root);
    let candidates = [
        root.join("DaedricData").join("overlay-version.json"),
        root.join(".daedric-overlay.json"),
    ];
    for path in candidates {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(ver) = v.get("version").and_then(|s| s.as_str()) {
                    if !ver.trim().is_empty() {
                        return Some(ver.to_string());
                    }
                }
            }
        }
    }
    None
}

#[derive(Debug, Serialize)]
pub struct UpdateReport {
    /// Overlay version marker on disk, if the install was found.
    installed_overlay: Option<String>,
    /// Server's current overlay version (only sent on change).
    server_overlay: Option<String>,
    /// Server's current launcher version (only sent on change).
    server_launcher: Option<String>,
    /// Server's newest launcher from the latest.yml feed.
    latest_launcher_feed: Option<String>,
    /// True when the server reports an overlay different from the installed one.
    overlay_update_available: bool,
    /// Human-readable verdict line for the panel.
    note: String,
}

/// Update check: local overlay marker vs the server's long-poll push channel.
/// `daedricWaitUpdate` holds ~25 s when nothing changed, so this command can
/// take a while on an up-to-date client — the UI shows a spinner.
#[tauri::command]
async fn check_updates(
    skyrim_root: Option<String>,
    host: Option<String>,
) -> Result<UpdateReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let host = host_or_default(host);
        let client = Client::new(&host);

        let installed = skyrim_root
            .filter(|r| !r.trim().is_empty())
            .and_then(|r| read_installed_overlay(&r));

        // latest.yml feed is fast — grab it first so a slow long-poll can't lose it.
        let feed_version = client
            .latest_launcher()
            .ok()
            .and_then(|raw| {
                raw.lines().find_map(|l| {
                    l.trim()
                        .strip_prefix("version:")
                        .map(|v| v.trim().to_string())
                })
            })
            .filter(|v| !v.is_empty());

        let check = client
            .check_update(installed.as_deref(), None)
            .map_err(|e| e.to_string())?;

        let overlay_update =
            matches!(
                (&check.overlay_version, &installed),
                (Some(server), Some(local)) if server != local
            ) || (check.change && installed.is_none() && check.overlay_version.is_some());

        let note = if check.change {
            if overlay_update {
                format!(
                    "server is on overlay {} — run the launcher to update.",
                    check.overlay_version.as_deref().unwrap_or("?")
                )
            } else {
                "server reports a change (launcher build) — check the launcher feed.".to_string()
            }
        } else if check.retry_in.is_some() {
            "no change on the push channel — client is up to date.".to_string()
        } else {
            "server returned no change marker — treat as up to date.".to_string()
        };

        Ok(UpdateReport {
            installed_overlay: installed,
            server_overlay: check.overlay_version,
            server_launcher: check.launcher_version,
            latest_launcher_feed: feed_version,
            overlay_update_available: overlay_update,
            note,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Watcher event emitted to the frontend when the server pushes a change.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WatcherEvent {
    /// "overlay" | "launcher" | "both"
    kind: String,
    overlay_version: Option<String>,
    launcher_version: Option<String>,
}

static WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Background update watcher: holds the `daedricWaitUpdate` long-poll in a
/// loop and emits `update-event` to the frontend whenever the server pushes a
/// new overlay or launcher build. Starts once per app run; returns false if
/// already running. Network errors back off 30 s and retry.
#[tauri::command]
fn start_update_watcher(
    app: tauri::AppHandle,
    skyrim_root: Option<String>,
    host: Option<String>,
) -> bool {
    if WATCHER_RUNNING.swap(true, Ordering::SeqCst) {
        return false;
    }
    let host = host_or_default(host);
    let mut overlay = skyrim_root
        .filter(|r| !r.trim().is_empty())
        .and_then(|r| read_installed_overlay(&r));
    let mut launcher: Option<String> = None;

    std::thread::spawn(move || {
        let client = Client::new(&host);
        loop {
            match client.check_update(overlay.as_deref(), launcher.as_deref()) {
                Ok(check) => {
                    if check.change {
                        let overlay_changed =
                            check.overlay_version.is_some() && check.overlay_version != overlay;
                        let launcher_changed =
                            check.launcher_version.is_some() && check.launcher_version != launcher;
                        if overlay_changed || launcher_changed {
                            let kind = match (overlay_changed, launcher_changed) {
                                (true, true) => "both",
                                (true, false) => "overlay",
                                _ => "launcher",
                            };
                            overlay = check.overlay_version.clone().or(overlay);
                            launcher = check.launcher_version.clone().or(launcher);
                            let _ = app.emit(
                                "update-event",
                                WatcherEvent {
                                    kind: kind.to_string(),
                                    overlay_version: overlay.clone(),
                                    launcher_version: launcher.clone(),
                                },
                            );
                        }
                    }
                    // No change: server already held ~25 s — loop right back.
                }
                Err(_) => std::thread::sleep(Duration::from_secs(30)),
            }
        }
    });
    true
}

// ---------- RP profile store commands ----------

#[tauri::command]
fn profile_load(state: tauri::State<Store>) -> Profile {
    state.profile()
}

#[tauri::command]
fn character_save(state: tauri::State<Store>, character: Character) -> Result<(), String> {
    state.save_character(character)
}

#[tauri::command]
fn contact_save(state: tauri::State<Store>, contact: Contact) -> Result<Contact, String> {
    state.save_contact(contact)
}

#[tauri::command]
fn contact_delete(state: tauri::State<Store>, id: String) -> Result<(), String> {
    state.delete_contact(&id)
}

#[tauri::command]
fn journal_add(
    state: tauri::State<Store>,
    title: String,
    location: String,
    body: String,
) -> Result<JournalEntry, String> {
    state.add_journal(title, location, body)
}

#[tauri::command]
fn journal_delete(state: tauri::State<Store>, id: String) -> Result<(), String> {
    state.delete_journal(&id)
}

#[tauri::command]
fn rumor_add(state: tauri::State<Store>, text: String, source: String) -> Result<Rumor, String> {
    state.add_rumor(text, source)
}

#[tauri::command]
fn rumor_toggle(state: tauri::State<Store>, id: String) -> Result<(), String> {
    state.toggle_rumor(&id)
}

#[tauri::command]
fn rumor_delete(state: tauri::State<Store>, id: String) -> Result<(), String> {
    state.delete_rumor(&id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            app.manage(Store::load(config_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pulse,
            get_news,
            get_latest_launcher,
            get_collection,
            get_ping_stats,
            check_updates,
            start_update_watcher,
            profile_load,
            character_save,
            contact_save,
            contact_delete,
            journal_add,
            journal_delete,
            rumor_add,
            rumor_toggle,
            rumor_delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running daedric-companion");
}
