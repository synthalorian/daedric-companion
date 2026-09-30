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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_pulse,
            get_news,
            get_latest_launcher,
            get_collection
        ])
        .run(tauri::generate_context!())
        .expect("error while running daedric-companion");
}
