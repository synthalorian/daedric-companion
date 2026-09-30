# Daedric Online Launcher — Internal Recon (for third-party companion app)

**Source analyzed:** `C:\Program Files\DaedricOnline\resources\app.asar` (Wine prefix
`/home/synth/Games/umu/umu-489830/drive_c/...`), launcher **version 1.3.60** (from
`package.json`). Electron app, esbuild-bundled (`dist/main.js` = main process,
`dist/renderer.js` = UI). Read-only analysis; nothing modified.

**Live settings file (user machine):**
`C:\users\steamuser\AppData\Roaming\Daedric Online\settings.json` — confirms the
defaults below plus live values: `profileId: 4158`, `discordClientId: "1523305822791139388"`,
`voiceLastGoodPort: 7778`, `termsAcceptedVersion: 5`. `voice.log` (262 KB) also lives in
that Roaming dir.

---

## 1. Mod-verification manifest schema

There are **three** distinct manifests. Two ship as real asar files; the richest one is
embedded inside the bundled `main.js`.

### 1a. `/data/modlist.json` (asar file) — the required-mod list

Top-level shape:

```json
{
  "_source": "Daedric Online (ptmvzi) rev 22 — 115 mods / 124 files; order+categories+special fields from modlist-overrides.json",
  "game": "skyrimspecialedition",
  "collectionSlug": "ptmvzi",
  "collectionRevision": 22,
  "collectionUrl": "https://www.nexusmods.com/games/skyrimspecialedition/collections/ptmvzi/revisions/22",
  "mods": [ /* 115 entries */ ]
}
```

Mod entries — verbatim:

```json
{
  "modId": 35785,
  "name": "Expressive Facegen Morphs SE",
  "author": "Niroku",
  "category": "Character & Appearance",
  "files": [
    {
      "fileId": 151138,
      "name": "Expressive Facegen Morphs SE",
      "version": "1.0",
      "sizeBytes": 2628697,
      "sizeMB": 2.5,
      "optional": false
    }
  ]
}
```

**CBBE 3BA, verbatim (modId 30174 / fileId 600100):**

```json
{
  "modId": 30174,
  "name": "CBBE 3BA (3BBB)",
  "author": "Acro",
  "category": "Character & Appearance",
  "files": [
    {
      "fileId": 600100,
      "name": "CBBE 3BA (3BBB)",
      "version": "2.48",
      "sizeBytes": 346505352,
      "sizeMB": 330.5,
      "optional": false
    }
  ],
  "detect": [
    "3BBB.esp"
  ]
}
```

(`detect` = plugin filename the launcher uses to detect that the mod is installed;
present on only some entries.)

### 1b. `data/collection-lock.json` — archive pins (md5/size/name) — embedded in main.js

Not a real asar file; it is baked into `dist/main.js` as an esbuild CommonJS module.
Top-level keys: `schema: "daedric-collection/1"`, `slug: "ptmvzi"`, `revision: 22`,
`url`, `revisionCreatedAt`, `fileSetSha256`, `generatedAt`, `generator: {tool, export,
exportSha256, overlayVersion: "4.99.608"}`, `gameVersions: ["1.6.1170.0"]`,
`launcherPolicy: {collectionEnforce: "enforce"}`, then **`mods: [...]` (240 entries)**,
**`files: {...}` (242 plugin entries)**, **`dlls: {...}` (26 entries)**, per-release
`_comment` keys, and `unattributed: []`.

Mod entry schema (verbatim, first entry):

```js
{
  modId: 183,
  fileId: 778318,
  name: "Feminine Khajiit Textures (Grey Cat and Leopard)",
  file: "Feminine Grey Cat and Leopard (CBBE) 2K",
  logicalFilename: "Feminine Grey Cat and Leopard (CBBE) 2K",
  version: "4.0",
  archiveMd5: "3d6d36062c75399234d3a4d33d095185",
  archiveSize: 46887898,
  archiveName: "Feminine Grey Cat and Leopard (CBBE) 2K 183 4.0 2026-07-18T23-25Z miUDjF7DV.7z",
  archiveStem: "Feminine Grey Cat and Leopard (CBBE) 2K 183 4.0 2026-07-18T23-25Z miUDjF7DV",
  archiveFormat: "7z",
  uri: null,
  category: "MAIN",
  fomod: true,
  choicesSha: "ebda6095d6f1a1a1abcbfbb9ea9b0f82276562284045531cbcaae6f9be63a00d",
  provides: {
    plugins: [],
    bsas: []
  }
}
```

**CBBE 3BA, verbatim:**

```js
{
  modId: 30174,
  fileId: 600100,
  name: "CBBE 3BA (3BBB)",
  file: "CBBE 3BA (3BBB)",
  logicalFilename: "CBBE 3BA (3BBB)",
  version: "2.48",
  archiveMd5: "714f3d83b296c66b9f0a64c7089b3666",
  archiveSize: 346505352,
  archiveName: "CBBE 3BA (3BBB)-30174-2-48-1740765899.zip",
  archiveStem: "CBBE 3BA (3BBB)-30174-2-48-1740765899",
  archiveFormat: "zip",
  uri: "CBBE 3BA (3BBB)-30174-2-48-1740765899.zip",
  category: "MAIN",
  fomod: true,
  choicesSha: "6ad169111ed821e954139f926a263139c17ed73f118a18e875583a0f58986fce",
  provides: {
    plugins: [
      "3BBB-RaceMenuMorphs.esp",
      "3BBB.esp",
      "RaceMenuMorphsCBBE.esp",
      "SOSPhysicsManager.esp"
    ],
    bsas: []
  }
}
```

`category` values seen: `MAIN`, `OPTIONAL`, `OLD_VERSION`, `ARCHIVED`.

### Per-file plugin hash map (`files:` in collection-lock) — esp/esl name → entry

Verbatim entries:

```js
"RaceMenuMorphsCBBE.esp": {
  sha256: "1abd6b7176b16a479bd2ea8fc8844c622588083eef7fc3e810a5aa1f62014d29",
  size: 697,
  role: "collection",
  modId: 30174,
  fileId: 600100,
  entry: "15 RaceMenuMorphs/00 RaceMenuMorphs - CBBE/RaceMenuMorphsCBBE.esp"
},
"Precision.esp": {
  sha256: "48bea576dadadb93f3af3701baf634e870fc4e0ce6c6ee94fa5f2072d2520790",
  size: 783,
  role: "collection",
  modId: 72347,
  fileId: 351862,
  entry: "Precision.esp"
},
"PipeSmokingSE.esp": {
  sha256: "f5e7b08db6b56db7035a4769c82e7134745bd8a2db29d6f547615df0909d40dc",
  size: 4262,
  role: "overlay",
  modId: 13061,
  fileId: 36966,
  entry: "Pipe Smoking SE/data/PipeSmokingSE.esp"
}
```

`role` ∈ {`collection`, `overlay`}. Map keys are **case-sensitive as shipped** (e.g.
`"unofficial skyrim special edition patch.esp"` is lowercase). `entry` is the path
inside the archive.

### `dlls:` map (SKSE plugin DLLs)

DLLs may have **multiple accepted hashes**:

```js
"actorlimitfix.dll": {
  sha256: [
    "b28de705613aceefd16d44bfc4fa21288b644be5a7decaf9c5603690b1d89a23"
  ],
  origin: "collection",
  size: 563712,
  mods: [ { modId: 32349, fileId: 368385, name: "Actor Limit Fix" } ],
  from: { b28de7...: [ "archive 32349:368385 SKSE/Plugins/ActorLimitFix.dll", "reference install" ] }
}
```

### 1c. `data/build-manifest.json` — final on-disk plugin hashes (152 plugins)

Also embedded in main.js. `_source`: "sha256 of every required non-RP plugin PLUS
required loose UI files (RaceMenu chargen swfs); the launcher hash-checks each at Play".
Entries are `{sha256, size}` only (no modId), e.g.:

```js
"3BBB.esp": {
  sha256: "a1aa2b0a08a1ce77ddb56ab48483f5a9978c671cf508d771e2a2b2f78fd0244e",
  size: 3288
}
```

### 1d. `/data/fomod-choices.json` (asar file) — FOMOD installer selections

Keyed by stringified Nexus modId:

```json
"57339": {
  "module": "Faster HDT-SMP",
  "selections": [
    { "step": "Introduction", "group": "Introduction", "plugins": ["Introduction"] },
    { "step": "AVX", "group": "AVX", "plugins": ["AVX (recommended)"] },
    { "step": "Thanks", "group": "", "plugins": ["Thanks"] }
  ],
  "savedAt": "2026-08-17T08:20:11.403Z",
  "_note": "AVX variant — hash-matched to the dll samis deployed (58bde4e2…)..."
}
```

### Where each manifest lives in the asar (for a Rust parser)

asar layout: 4×u32-LE header (`[4, header_block_size, header_string_size, json_length]`
= `[4, 142144, 142140, 142133]` in this build), JSON directory at byte 16, file data
starting at `8 + header_block_size` = byte **142152**. Directory entries store
`offset` (relative to data start) and `size` as decimal strings.

| File | Abs. offset in app.asar | Size |
|---|---|---|
| `/data/fomod-choices.json` | 9,028,172 (`0x89C24C`) | 6,853 |
| `/data/modlist.json` | 9,035,025 (`0x89DD11`) | 66,030 |
| `/dist/main.js` | 10,416,152 (`0x9EF018`) | 933,306 |
| `/dist/preload.js` | 11,351,592 | 6,126 |
| `/dist/renderer.js` | 11,357,718 | 1,040,763 |
| `/dist/voice.js` | 12,490,697 | 20,032 |
| `/package.json` | 12,512,633 | 561 |

Offsets shift per launcher release — **parse the asar header JSON rather than hardcoding**.
For `collection-lock.json` and `build-manifest.json` (which have no asar directory
entry), locate them inside `dist/main.js` by searching for the marker strings
`"data/collection-lock.json"` and `"data/build-manifest.json"`; each is an esbuild
`__commonJS` module body of the form `module2.exports = { ...JS object literal... };`
terminated by `\n// <next module path>`. Note: it's a JS object literal (unquoted keys),
not strict JSON — a lenient parser (or a JS-engine eval in a sandbox) is needed. The
`files:` map begins ~offset 171,364 within main.js and `dlls:` at ~388,752 in this build.

---

## 2. Network surface

### Game / account server — `play.daedriconline.com`

From `src/main/settings.ts` DEFAULTS:

```js
serverHost: "play.daedriconline.com",   // "Connect by NAME, never a raw IP"
serverPort: 7777,
```

Trusted-host list (`src/shared/trustedKeys.ts`):

```js
var TRUSTED_KEYS = {
  keys: [],
  servers: {
    prod: { hosts: ["play.daedriconline.com", "playdaedric.ddns.net"] },
    dev: { hosts: ["devdaedric.ddns.net", "100.101.27.72"] }
  },
  revoked: []
};
```

**Port map (all on the server host):**

| Port | Proto | Purpose |
|---|---|---|
| 7777 | UDP | RakNet game (skymp) — launcher pings it with a 33-byte RakNet unconnected ping (`0x01`, timestamp, magic `00ffff00fefefefefdfdfdfd12345678`, client guid `aabbccddeeff0011`) to measure latency/online |
| 7778 | UDP + WS | Proximity **voice server**. Primary: UDP with custom framing (`MAGIC = 218`, `TYPE_CONTROL = 0`, `TYPE_VOICE_UP = 1`, `TYPE_VOICE_DOWN = 2`). Fallback: `ws://<host>:7778` (`const ws = new wrapper_default(\`ws://${st.host}:${st.port}\`)`), handshake `{t:"hello", token, red:true}` retried every ~3 s |
| 7780 | (local) | `CONTROL_PORT` — launcher's local voice control channel to the game client |
| 3000 | HTTP | Launcher services API (update feed, news, status, RPC, overlay downloads, collection manifest) |

**HTTP endpoints on port 3000** (`http://play.daedriconline.com:3000`):

```js
// server status + player count (drives the ONLINE / 437/650 UI)
http.get({ host, port: 3e3, path: "/rp-status.json", timeout: 3e3 }, ...)
//   → { players: j.players ?? 0, maxPlayers: j.maxPlayers ?? 0 }

// JSON fetch helper used for:
fetchServerJson(host, "/launcher/collection.json")   // server-published required collection (raw manifest; sha256'd for build reports)
fetchServerJson(host, "/launcher/news.json")         // news feed for the UI
httpGetText(host, "/launcher/latest.yml")            // electron-updater style feed: version/file/sha512/notes

// overlay (server-side client files) downloads:
"/overlay/manifest.json"
"/overlay/release.json"
"/overlay/release.addendum.json"
`/overlay/files/${f.sha256}`                         // content-addressed file fetch, 30s timeout

// diag agent (opt-in watchdog) download:
"/launcher/diag-agent.exe" + "/launcher/diag-agent.exe.sha256"

// launcher self-update download:
`/launcher/${encodeURIComponent(yml.file)}`          // NSIS installer, sha512-verified, staged then run

// renderer also references: http://play.daedriconline.com:3000/launcher/latest.json
```

**JSON-RPC on port 3000** — `POST /rpc/<name>`, body `{"payload": {...}}`, expects a
JSON body back (explicitly guards against HTML/captive-portal responses):

```js
path: `/rpc/${rpcName}`, method: "POST",
headers: { "content-type": "application/json", ... }
```

RPC names used:

- `daedricRegister` — `{ token /*uuid accountToken*/, hwid }` → `{ profileId }`
- `daedricResolveProfile` — `{ token, discordId, hwid }` → `{ profileId }`
- `daedricDiscordLink` — `{ token, accessToken /*discord oauth*/ }`
- `daedricWaitUpdate` — `{ overlayVersion, launcherVersion }`, **long-poll (server
  holds ~25 s, client timeout 40 s)**; returns `{change, overlayVersion,
  launcherVersion}` or `{retryIn}` — this is the push channel behind the UI's
  `clientfiles:changed` / `collection:changed` events
- `daedricBuildReport` — `{ token, manifestSha /*sha256 of /launcher/collection.json*/,
  modsOk, launcher, ...telemetry }`

### Discord OAuth (account link) — PKCE, no client secret

```js
var REDIRECT_PORT = 53682;
var REDIRECT_URI = `http://127.0.0.1:${REDIRECT_PORT}/callback`;   // loopback, NOT daedric://
var LINK_TIMEOUT_MS = 2 * 60 * 1e3;
...
const authUrl = `https://discord.com/oauth2/authorize?client_id=${encodeURIComponent(clientId)}&response_type=code&redirect_uri=${encodeURIComponent(REDIRECT_URI)}&scope=identify&state=${state2}&code_challenge=${challenge}&code_challenge_method=S256`;
shell.openExternal(authUrl);
// then POST https://discord.com/api/oauth2/token  (grant_type=authorization_code, code_verifier)
// then rpcPost(host, "daedricDiscordLink", { token, accessToken })
```

- **Client ID (public, baked in): `1523305822791139388`** (`discordClientId` default in settings.ts)
- Scope: `identify` only. Redirect: localhost callback on **127.0.0.1:53682/callback**.
- The resulting Discord access token goes to the game server; the launcher stores only
  `discordId`, `discordUsername`, `discordAvatar` in settings.json.

### Nexus Mods API

```js
https2.get({ host: "api.nexusmods.com", path: pathName,
  headers: { apikey: <user nexusApiKey>, "user-agent": "DaedricOnlineLauncher/0.1" } }, ...)
// used paths:
//  /v1/games/skyrimspecialedition/mods/{modId}/files/{fileId}/download_link.json   (Premium one-click install)
//  (also /v1 user validation for the api key)
```

Collection page referenced: `https://www.nexusmods.com/games/skyrimspecialedition/collections/ptmvzi/revisions/22`.
Non-premium users are told to use "Mod Manager Download" on Nexus (the launcher
registers as an nxm/Mod-Manager-Download target — archives land in
`<game>\DaedricData\downloads\`).

### Other outbound

- `https://cdn.discordapp.com/avatars/<id>/<hash>.png?size=64` (avatar display; embed fallback `cdn.discordapp.com/embed/avatars/...`)
- `https://daedriconline.com/{terms,privacy,credits}.html`, `mailto:privacy@daedriconline.com`
- `https://discord.gg/FkDqP8Rg4f` (community invite)
- `https://github.com/skyrim-multiplayer/skymp` (attribution)
- `steam://` repair URLs via `launch:openRepairUrl` (downgrader flow)
- **Self-update**: electron-updater with a **generic provider** — `resources/app-update.yml`:
  ```yaml
  provider: generic
  url: http://play.daedriconline.com:3000/launcher
  updaterCacheDirName: daedric-launcher-updater
  ```
  Polls `/launcher/latest.yml` at startup +4 s and on an interval, sha512-verifies the
  staged NSIS installer, runs it elevated if needed (`resources/elevate.exe` shipped
  beside the asar). Update state also pushed via the `daedricWaitUpdate` long-poll.

---

## 3. Companion-app-relevant extras

### settings.json schema (Roaming\Daedric Online\settings.json)

Written atomically-ish (`JSON.stringify(merged, null, 2)`), mtime-cached. Full default
set from `src/main/settings.ts`:

```js
{
  gamePath: "",                          // Skyrim SE root; validated by SkyrimSE.exe + skse64_loader.exe presence
  serverHost: "play.daedriconline.com",
  serverPort: 7777,
  profileId: 0,                          // server-assigned player number
  sessionToken: "",                      // "v1.<profileId>.<ms>.<rand16>.<hex64>"
  profileDiscordId: "",
  accountToken: "",                      // client-generated UUID (crypto.randomUUID)
  nexusApiKey: "",
  discordClientId: "1523305822791139388",
  discordId: "", discordUsername: "", discordAvatar: "",
  communityShaders: false,               // opt-in, costs FPS
  watchdogEnabled: false, diagEnabled: false, diagMirrorAll: false,
  diagLogFile: "Diagnostics.log", diagOnlyEvents: "", diagOnlyServices: "", diagSkipEvents: "",
  enableProfiling: false, profilingDurationMs: 20000,
  voiceEnabled: true,
  voicePttKey: 47,        // V
  voiceModeKey: 56,       // Left Alt (whisper/normal/yell)
  bindNametagsKey: 59,    // F1
  bindHudKey: 60,         // F2
  bindReportCodesKey: 61, // F3
  bindChatFocusKey: 64,   // F6
  bindCreateWheelKey: 35, // H
  voiceInputDeviceId: "", voiceOutputDeviceId: "",
  voiceInputVolume: 1, voiceVolume: 1,
  voiceLastGoodHost: "", voiceLastGoodPort: 0,
  voiceSinkEnabled: true,
  termsAcceptedVersion: 0,               // current TERMS_VERSION = 5 ("17 September 2026")
  termsAcceptedAt: ""
}
```

### Files/dirs the launcher owns (under `<gamePath>`)

- `DaedricData/downloads/` — downloaded mod archives (`<modId>-<fileId>.archive`)
- `DaedricData/quarantine/` — foreign plugins/client files moved out of the way at PLAY
- `DaedricData/backups/`, `DaedricData/vanilla-backup/`
- `DaedricData/installed-mods.json` (install ledger), `DaedricData/overlay-version.json`,
  `DaedricData/overlay-manifest.json` (cache), `DaedricData/release-state.json`,
  `DaedricData/release-lock.<server>.json`
- Legacy marker: `<game>\.daedric-overlay.json`
- Writes `Data\Plugins.txt`-style load order (`*`-prefixed lines, CRLF)
- Reads/writes Skyrim INIs for the perf profile apply/revert; can disable/enable the
  skymp client files ("play-elsewhere" toggle)

### Version strings in this build

- Launcher: **1.3.60** (`package.json`)
- Collection: Nexus slug `ptmvzi` revision **22**, `fileSetSha256:
  7aa1b0d39a5422dd03f4ec2f05815d8338470cd43d0806499b30943b3be5f4e7`,
  `generatedAt: 2026-09-17T13:47:02.805Z`
- Server overlay (client files): **4.99.608**
- Supported game version: **1.6.1170.0** only (there is a Steam-depot downgrader flow
  with per-depot console commands for players on newer EXEs)
- Terms: `TERMS_VERSION = 5`, updated "17 September 2026"

### IPC surface (for reference; renderer bridge `window.daedric`)

`settings:get/save/validateGamePath/applyPerfProfile/revertPerfProfile`, `server:ping`,
`revert:plan/run`, `skymp:state/disable/enable`, `game:launch/stop/getState`,
`verify:run` (+`verify:progress`), `mods:status`, `collection:status`,
`overlay:check/install`, `discord:link/unlink`, `terms:status/accept`,
`selfupdate:check/install/version`, `news:get`, `shaders:clearCache`,
`downgrade:*`, `diag:lastSession/openDumps`, plus pushed events `mods:status`,
`clientfiles:changed`, `collection:changed`, `launch:quarantined`, `launch:restored`.

### Practical notes for a companion app

- **Server status without the launcher**: `GET http://play.daedriconline.com:3000/rp-status.json`
  → `{"players":N,"maxPlayers":M}`; pair with a RakNet UDP ping on 7777 for latency.
- **News**: `GET :3000/launcher/news.json`. **Update feed**: `GET :3000/launcher/latest.yml`.
- **Voice presence**: UDP `MAGIC=218` framing on 7778, or WS `ws://host:7778` hello
  `{t:"hello", token, red:true}` — token is the settings.json `sessionToken`.
- Long-poll `POST :3000/rpc/daedricWaitUpdate` with `{payload:{overlayVersion,launcherVersion}}`
  for push notifications of new client builds/collection revisions.
- Mod verification: hash installed `Data\*.esp/.esl` against `files:` (case-sensitive
  keys) / `build-manifest.json` `plugins:`; verify archives in `DaedricData/downloads`
  by md5+size against `collection-lock.json` `mods[]`.
