//! daedric-companion — Tauri 2 desktop dashboard backed by daedric-api.

use daedric_api::{Client, DEFAULT_HOST};
use serde::Serialize;

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

#[derive(Debug, Serialize)]
pub struct CollectionSummary {
    slug: Option<String>,
    revision: Option<serde_json::Value>,
    mods_count: Option<usize>,
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

#[tauri::command]
async fn get_collection(host: Option<String>) -> Result<CollectionSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = Client::new(&host_or_default(host));
        let value = client.collection().map_err(|e| e.to_string())?;
        let slug = value
            .get("slug")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let revision = value.get("revision").cloned();
        let mods_count = value
            .get("mods")
            .and_then(|m| m.as_array())
            .map(|a| a.len());
        Ok(CollectionSummary {
            slug,
            revision,
            mods_count,
        })
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_pulse,
            get_news,
            get_latest_launcher,
            get_collection,
            check_updates
        ])
        .run(tauri::generate_context!())
        .expect("error while running daedric-companion");
}
