//! Field check: hit the live server once and print everything.

use daedric_api::{pulse, Client, DEFAULT_HOST};

fn main() {
    let host = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_HOST.into());
    let client = Client::new(&host);

    println!("== status ==");
    match client.status() {
        Ok(s) => println!("  {}/{} players", s.players, s.max_players),
        Err(e) => println!("  failed: {e}"),
    }

    println!("== raknet ping (udp 7777) ==");
    match client.ping() {
        Ok(d) => println!("  {d:?}"),
        Err(e) => println!("  failed: {e}"),
    }

    println!("== news ==");
    match client.news() {
        Ok(n) => {
            let s = serde_json::to_string_pretty(&n.0).unwrap();
            println!("  {}", &s[..s.len().min(600)]);
        }
        Err(e) => println!("  failed: {e}"),
    }

    println!("== latest.yml ==");
    match client.latest_launcher() {
        Ok(y) => println!("  {}", &y[..y.len().min(400)]),
        Err(e) => println!("  failed: {e}"),
    }

    println!("== pulse ==");
    let p = pulse(&host);
    println!(
        "  online={} players={:?}/{:?} latency={:?} (checked in {:?})",
        p.online, p.players, p.max_players, p.latency, p.checked_in
    );
}
