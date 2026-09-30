//! daedric-api — client for the Daedric Online server surface.
//!
//! Field-decoded from the launcher (daedric-companion/docs/recon-launcher-internals.md):
//! - `GET :3000/rp-status.json` → `{players, maxPlayers}` (drives the launcher UI)
//! - `GET :3000/launcher/news.json` → news feed
//! - `GET :3000/launcher/latest.yml` → electron-updater feed (version/file/sha512)
//! - UDP 7777: RakNet unconnected ping (33 bytes) → latency + online
//! - `POST :3000/rpc/daedricWaitUpdate` → long-poll push channel (~25 s hold)
//!
//! Connect by NAME, never a raw IP: `play.daedriconline.com`.

use serde::Deserialize;
use std::time::{Duration, Instant};

pub mod ping;
pub use ping::{raknet_ping, raknet_ping_stats, PingStats};

pub const DEFAULT_HOST: &str = "play.daedriconline.com";
pub const API_PORT: u16 = 3000;
pub const GAME_PORT: u16 = 7777;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("http: {0}")]
    Http(#[from] ureq::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerStatus {
    #[serde(default)]
    pub players: u32,
    #[serde(default, rename = "maxPlayers")]
    pub max_players: u32,
}

/// One news entry — shape is server-defined; keep it loose.
#[derive(Debug, Clone, Deserialize)]
pub struct NewsFeed(pub serde_json::Value);

pub struct Client {
    host: String,
    agent: ureq::Agent,
}

impl Default for Client {
    fn default() -> Self {
        Self::new(DEFAULT_HOST)
    }
}

impl Client {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(5))
                .build(),
        }
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = format!("http://{}:{}{}", self.host, API_PORT, path);
        let body = self.agent.get(&url).call()?.into_string()?;
        Ok(serde_json::from_str(&body)?)
    }

    /// Server status + player count (the ONLINE 437/650 UI).
    pub fn status(&self) -> Result<ServerStatus, ApiError> {
        self.get_json("/rp-status.json")
    }

    /// News feed for the UI.
    pub fn news(&self) -> Result<NewsFeed, ApiError> {
        self.get_json("/launcher/news.json")
    }

    /// electron-updater style feed: version/file/sha512/notes (raw YAML text).
    pub fn latest_launcher(&self) -> Result<String, ApiError> {
        let url = format!("http://{}:{}/launcher/latest.yml", self.host, API_PORT);
        Ok(self.agent.get(&url).call()?.into_string()?)
    }

    /// Server-published required collection manifest (raw JSON).
    pub fn collection(&self) -> Result<serde_json::Value, ApiError> {
        self.get_json("/launcher/collection.json")
    }

    /// Long-poll push channel. Server holds ~25 s; returns change info or retry.
    /// `overlay_version` / `launcher_version` are the client's current versions.
    pub fn wait_update(
        &self,
        overlay_version: &str,
        launcher_version: &str,
    ) -> Result<serde_json::Value, ApiError> {
        let url = format!("http://{}:{}/rpc/daedricWaitUpdate", self.host, API_PORT);
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(40)) // launcher uses 40 s
            .build();
        let body = agent
            .post(&url)
            .set("content-type", "application/json")
            .send_json(ureq::json!({
                "payload": {
                    "overlayVersion": overlay_version,
                    "launcherVersion": launcher_version,
                }
            }))?
            .into_string()?;
        Ok(serde_json::from_str(&body)?)
    }

    /// RakNet ping on the game port: online + latency in one packet.
    pub fn ping(&self) -> Result<Duration, ApiError> {
        Ok(raknet_ping(&self.host, GAME_PORT, Duration::from_secs(3))?)
    }

    /// Multi-ping quality sample: min/avg/max + packet loss over `count` probes.
    pub fn ping_stats(&self, count: u32) -> PingStats {
        raknet_ping_stats(&self.host, GAME_PORT, count, Duration::from_secs(2))
    }
}

/// Status + latency in one call — the companion's main dashboard datum.
pub fn pulse(host: &str) -> Pulse {
    let client = Client::new(host);
    let status = client.status().ok();
    let started = Instant::now();
    let latency = client.ping().ok();
    Pulse {
        players: status.as_ref().map(|s| s.players),
        max_players: status.as_ref().map(|s| s.max_players),
        latency,
        online: status.is_some() || latency.is_some(),
        checked_in: started.elapsed(),
    }
}

/// Typed result of a `daedricWaitUpdate` long-poll probe.
///
/// The server answers `{change, overlayVersion, launcherVersion}` when either
/// version differs from its current build, or `{retryIn}` after holding ~25 s
/// when nothing changed (i.e. the client is up to date).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateCheck {
    pub change: bool,
    pub overlay_version: Option<String>,
    pub launcher_version: Option<String>,
    /// Raw `retryIn` value — units are server-defined, kept loose.
    pub retry_in: Option<serde_json::Value>,
}

impl Client {
    /// Long-poll update check. Pass the client's current versions
    /// (`None` = unknown). Blocks up to ~25 s when there is no change;
    /// the underlying HTTP timeout is 40 s, matching the launcher.
    pub fn check_update(
        &self,
        overlay_version: Option<&str>,
        launcher_version: Option<&str>,
    ) -> Result<UpdateCheck, ApiError> {
        let url = format!("http://{}:{}/rpc/daedricWaitUpdate", self.host, API_PORT);
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(40))
            .build();
        let body = agent
            .post(&url)
            .set("content-type", "application/json")
            .send_json(ureq::json!({
                "payload": {
                    "overlayVersion": overlay_version,
                    "launcherVersion": launcher_version,
                }
            }))?
            .into_string()?;
        let v: serde_json::Value = serde_json::from_str(&body)?;
        Ok(UpdateCheck {
            change: v.get("change").and_then(|c| c.as_bool()).unwrap_or(false),
            overlay_version: v
                .get("overlayVersion")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
            launcher_version: v
                .get("launcherVersion")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
            retry_in: v.get("retryIn").cloned(),
        })
    }
}

#[derive(Debug)]
pub struct Pulse {
    pub players: Option<u32>,
    pub max_players: Option<u32>,
    pub latency: Option<Duration>,
    pub online: bool,
    pub checked_in: Duration,
}
