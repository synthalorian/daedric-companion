# Daedric Companion

Desktop companion for the [Daedric Online](https://daedriconline.com) Skyrim SE roleplay server.

It watches the live server (player count, latency, news, launcher feed, collection summary, overlay update channel) and keeps a local war journal: character sheets, contacts, chronicle, rumors, dice, a septim purse, faction standings, play-session clock, and a Tamrielic date helper.

Nothing here talks to your game process. The journal is a JSON file on your machine.

## Build

Requires Rust and the Tauri 2 CLI.

```bash
cargo test -p daedric-companion
npx --yes @tauri-apps/cli@2 build
```

The frontend is static HTML/CSS/JS in `frontend/dist`. No bundler.

## Server probes

`daedric-api` talks to `play.daedriconline.com` by name (HTTP `:3000`, RakNet UDP `7777`). There is no TLS on `:3000`.

```bash
cargo run -p daedric-api --example pulse
DAEDRIC_SKYRIM="/path/to/Skyrim Special Edition" cargo run -p daedric-api --example update
```

The update probe can sit for ~25 seconds when the client is current. That is the server holding the long-poll, not a hang.

## Journal

The profile lives at the app config dir (`daedric-companion/profile.json`). Dates you type are freeform — the server runs on Tamrielic dates, not epoch timestamps. Each character owns their own contacts, chronicle, rumors, purse, factions, and sessions.

## License

Apache-2.0. See [LICENSE](LICENSE).
