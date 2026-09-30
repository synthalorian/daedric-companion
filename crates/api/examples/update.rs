//! Live update-check probe against play.daedriconline.com.
//!
//! usage: update [host] [overlay_version]
//! defaults: synth's field machine (host + installed overlay marker).
//!
//! WARNING: the server holds ~25 s when nothing changed — a slow run is the
//! up-to-date path, not a hang.

use daedric_api::{Client, DEFAULT_HOST};

fn read_installed_overlay() -> Option<String> {
    let root = std::path::Path::new(
        "/home/synth/.local/share/Steam/steamapps/common/Skyrim Special Edition",
    );
    for path in [
        root.join("DaedricData").join("overlay-version.json"),
        root.join(".daedric-overlay.json"),
    ] {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(ver) = v.get("version").and_then(|s| s.as_str()) {
                    return Some(ver.to_string());
                }
            }
        }
    }
    None
}

fn main() {
    let host = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_HOST.to_string());
    let overlay = std::env::args().nth(2).or_else(read_installed_overlay);
    println!("host: {host}");
    println!(
        "installed overlay: {}",
        overlay.as_deref().unwrap_or("none")
    );

    let client = Client::new(&host);
    let t = std::time::Instant::now();
    match client.check_update(overlay.as_deref(), None) {
        Ok(check) => {
            println!("answered in {:?}", t.elapsed());
            println!("  change:           {}", check.change);
            println!(
                "  overlay_version:  {}",
                check.overlay_version.as_deref().unwrap_or("—")
            );
            println!(
                "  launcher_version: {}",
                check.launcher_version.as_deref().unwrap_or("—")
            );
            println!(
                "  retry_in:         {}",
                check
                    .retry_in
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".to_string())
            );
            match (&check.overlay_version, &overlay) {
                (Some(server), Some(local)) if server != local => {
                    println!("  VERDICT: overlay update available ({local} -> {server})");
                }
                _ if check.change => println!("  VERDICT: change flagged, overlay unchanged"),
                _ => println!("  VERDICT: up to date"),
            }
        }
        Err(e) => println!("check failed: {e}"),
    }
}
