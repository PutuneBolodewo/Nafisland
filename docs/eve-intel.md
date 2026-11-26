# EVE Online Intel Tool Add-on Plan

> **Branch:** `feat/eve-intel-addon`  
> **Status:** Planning (Phase 1 design)

## 1. Overview

**EVE Intel** is a built-in add-on (`poni_core::addons::EVE_INTEL`, no external download) that monitors live EVE Online chat logs on Windows and triggers collapsed-island alerts (`Notice`) plus custom audio alarms when intel is reported.

It follows the repository's **"Keep it light"** rules (`AGENTS.md`):
- Does nothing while the add-on is switched off (no thread, no watcher, no tab).
- Uses event-driven directory watching (`ReadDirectoryChangesW`) with zero busy-polling at rest.
- Keeps custom sound playback isolated from Windows media controls (`CommandManager.SetIsEnabled(false)`), so alarms never hijack the Music pill.

---

## 2. How EVE Online Chatlogs Work (Verified Spec)

Observed directly from `C:\Users\Acer Gk\Documents\EVE\logs\Chatlogs`:

- **Filename format:**  
  `<ChannelName>_<YYYYMMDD>_<HHMMSS>_<CharacterID>.txt`  
  Example: `I. Ftn Intel_20261125_143805_2123995636.txt`
  - Every time the EVE client starts or a character logs in, a new session file is generated with a new timestamp.
  - Therefore, the user configures the **Channel Name** (e.g. `I. Ftn Intel`), and the watcher automatically resolves the newest `.txt` file matching `<ChannelName>_<YYYYMMDD>_<HHMMSS>_<CharacterID>.txt`.
- **Encoding & line format:**
  - Files are encoded in **UTF-16LE** starting with a BOM (`0xFF 0xFE`).
  - Header block includes `Channel ID`, `Channel Name`, `Listener` (character name), and `Session started`.
  - Each chat line begins with an extra inline UTF-16 BOM (`\u{FEFF}`):
    ```text
    \ufeff[ 2026.11.25 14:38:49 ] spacefarmer83 > 4HS-CR  Dominique Meyers  NightsongzZ
    ```
  - `Local_*.txt` logs system changes as:
    ```text
    \ufeff[ 2026.11.25 14:43:43 ] EVE System > Channel changed to Local : NY6-FH
    ```
- **Windows file-sharing behavior while EVE is running:**
  - EVE holds an open write handle on the active session `.txt` file. File Explorer's preview pane often shows no preview until flushed/closed, whereas shared read handles succeed.
  - On Windows, Rust's `std::fs::OpenOptions::new().read(true)` opens files with `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`, allowing non-blocking tail-reading without locking EVE out.
  - **Safe tailing rules:**
    1. Maintain a `u64` byte offset per watched file; on file rollover (new session timestamp), switch to the newest matching file and start at its end (or skip the header/MOTD).
    2. Only read/decode complete **2-byte UTF-16LE code units** (`available_bytes & !1`).
    3. Buffer incomplete trailing lines until `\n` arrives so mid-write flushes never emit truncated messages.
    4. Strip `\u{FEFF}` and `\r` when parsing each line.

---

## 3. UI & UX Design

### A. Expanded Island Page (`crates/poni/src/pages/eve_intel.rs`)

Standard island content area (`~584 × 256` DIPs), split into **2 columns**:

| Left Column (Controls & Config) | Right Column (Live Intel Feed) |
|---|---|
| 1. **Small EVE SSO button** at the top (`Connect SSO` / character badge when signed in; Phase 1 uses public data / placeholder flow). | 1. **Header status** (watched channel status, matched `.txt` filename/session time, mute/active state). |
| 2. **Intel Channel input** below it (`ui.text_input_row`): user manually types the channel name (e.g. `I. Ftn Intel`). **No automatic channel list detection in Phase 1** — the watcher resolves `<ChannelName>_*.txt` from the Chatlogs folder. | 2. **Scrollable recent intel list** (`wheel_in`) showing latest parsed lines (`[HH:MM:SS] Sender: Message`) with highlight on alert triggers. |
| 3. **Alarm Settings button** (`ui.pill`): opens **Advanced Settings** directly on the **EVE Intel** section (`open_settings_window(Some(Section::EveIntel))`). | |

### B. Collapsed Island Alerts (`Notice`)

When an intel message matches trigger rules (and passes cooldown):
- Push a `Notice` to the island (`AppState::notice`), showing the channel/system snippet and reporter.
- Clicking the notice opens the `PageKind::EveIntel` tab.
- Respects full-screen game settings (`held` notice / show-over-games behavior already in `app.rs`).

### C. Custom Sound & Alarm Settings (`crates/poni/src/settings_window.rs`)

All alarm and sound customization lives exclusively in **Advanced Settings → `Section::EveIntel`** (hidden when the EVE Intel add-on is off, matching `Section::Caffeine` / `Section::Prayer`):
- **Chatlogs folder path:** defaults to `%USERPROFILE%\Documents\EVE\logs\Chatlogs` (editable/resettable).
- **Alarm enabled:** switch to enable/mute audio alerts independently of the visual feed.
- **Alert sound:** System Chime (`services::sound`) or **Custom Audio File** (`.wav` / `.mp3` / `.m4a` path) + **Test** button.
- **Cooldown (seconds):** prevents repeated sound spam for rapid consecutive lines (e.g. 0 s, 5 s, 10 s, 30 s).
- **Ignore words:** comma-separated keywords that suppress alarms (default e.g. `clr, clear`).
- **Collapsed island pop-up:** switch to show/hide `Notice` banners while collapsed.

---

## 4. SSO & Public Data Roadmap

1. **Phase 1 — Local Chatlogs + Public Data (No Auth Required):**
   - Full chatlog monitoring, intel parsing, custom sound alarm, and collapsed island alerts work without logging in.
   - The small SSO button in the left column represents the character link state (shows "Public mode" / "EVE SSO (Soon)" or opens the PKCE connect flow once a registered `Client ID` is configured).
   - Optional public ESI lookups (no token needed) via `services::http::Client::uncached`, e.g., resolving character/system names or public route jumps with local caching.
2. **Phase 2 — EVE SSO v2 (OAuth2 + PKCE):**
   - Native desktop flow: generate `code_verifier` + `S256` `code_challenge` + `state`, open `https://login.eveonline.com/v2/oauth/authorize/` in the system browser, capture callback on `http://127.0.0.1:<ephemeral_port>/callback`, exchange code at `https://login.eveonline.com/v2/oauth/token` via `services::http::Client::private`.
   - Seal the V2 `refresh_token` with Windows DPAPI via `services::secret` (`%LOCALAPPDATA%\Poni\eve-sso.bin`) — never stored in `settings.json` or logs.
   - Future scopes: `esi-location.read_location.v1`, `esi-ui.open_window.v1`.

---

## 5. Proposed Module Architecture

### `crates/poni-core` (Pure Logic + Unit Tests)
- `src/addons.rs`: Add `pub const EVE_INTEL: Addon` (`id: "eve_intel"`, `files: &[]`, `pro: false`) and include in `ALL`.
- `src/settings.rs`: Add `pub eve_intel: EveIntelSettings` (`channel: String`, `chatlogs_dir: Option<String>`, `alarm_on: bool`, `sound_path: Option<String>`, `cooldown_s: u32`, `ignore_words: String`, `popup_notice: bool`).
- `src/eve_intel.rs` (new):
  - `decode_utf16le_chunk(bytes: &[u8], carry: &mut Option<u8>) -> String` (handles odd-byte boundaries and `\u{FEFF}`).
  - `parse_header(text: &str) -> Option<LogHeader>` (`Channel Name`, `Listener`, `Channel ID`).
  - `parse_line(line: &str) -> Option<ChatMessage>` (timestamp, sender, text).
  - `match_channel_filename(filename: &str, channel: &str) -> Option<SessionFile>` (parses `<Channel>_<YYYYMMDD>_<HHMMSS>_<CharID>.txt` and picks the latest session).
  - `should_alert(msg: &ChatMessage, cfg: &EveIntelSettings) -> bool` (skips `EVE System` MOTD/join noise and `ignore_words`).

### `crates/poni` (Windows Services & UI)
- `src/services/eve_intel.rs` (new):
  - Background thread started lazily only when `EVE_INTEL` is switched on (`Services::set_eve_intel_watch`).
  - Watches `Chatlogs` directory via `ReadDirectoryChangesW` (`FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_SIZE`) with a light fallback tick only while an active session is open, sleeping at 0% CPU when idle or locked/display off.
  - Plays custom alarm audio using `WinRT MediaPlayer` with `CommandManager.SetIsEnabled(false)` (following `services/adzan.rs`), falling back to `services::sound::play(Sound::Chime)`.
- `src/events.rs` & `src/commands.rs`:
  - `AppEvent::EveIntel(EveIntelEvent)` (watched file changed, new parsed messages, SSO state).
  - `Command::EveIntel(EveIntelCmd)` (update channel, test sound, start/disconnect SSO) + `Command::OpenSettingsSection(Section)`.
- `src/state.rs`:
  - `eve_intel_on(&self) -> bool`, `AppState::eve_intel` (ring buffer of recent messages, active `.txt` filename, last alert timestamp), `PageUiState::eve_channel_input`.
- `src/pages/mod.rs` & `src/pages/eve_intel.rs` (new):
  - `PageKind::EveIntel` tab, 2-column layout (Left: small SSO button + channel input + Alarm Settings button; Right: live intel feed).
- `src/settings_window.rs`:
  - `Section::EveIntel` with Chatlogs folder, Alarm toggle, Sound mode/file path + Test button, Cooldown, Ignore words, and Island pop-up toggle.

---

## 6. Implementation Phases & Verification

1. **Phase 1A (`poni-core`):** Implement `eve_intel.rs` parser/matcher, `EveIntelSettings`, and `EVE_INTEL` add-on registration + unit tests (`cargo test -p poni-core`) using sanitized fixtures based on real UTF-16LE chatlogs.
2. **Phase 1B (`services/eve_intel`):** Implement Windows `Chatlogs` directory watcher + tail reader + custom `MediaPlayer` alarm playback + wire into `Services`, `events.rs`, `commands.rs`, `app.rs`.
3. **Phase 1C (`UI`):** Implement 2-column `pages/eve_intel.rs`, collapsed `Notice` alert, and `Section::EveIntel` in `settings_window.rs`. Verify compilation with `cargo check` / `cargo test` and smoke-test live chatlog appending while EVE Online is running.
4. **Phase 2 (SSO):** Implement EVE SSO v2 PKCE loopback flow + DPAPI token storage (`services/secret.rs`) + public/authenticated ESI integration.
