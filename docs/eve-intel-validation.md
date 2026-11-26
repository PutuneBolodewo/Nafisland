# EVE Intel Tool — Validation, Multi-Character & Proximity Plan

> **Branch:** `feat/eve-intel-addon`  
> **Builds on:** [docs/eve-intel.md](eve-intel.md)  
> **Status:** Planning (Phase 2 — Validation & Proximity)

## 1. Overview & Goals

Extend the EVE Online Intel Tool add-on from single-channel raw tailing into **validated, multi-character proximity-aware intel**:

1. **Active Client Detection (`• N active`):** Detect how many EVE Online clients (`exefile.exe`) are running, map them to active characters' `Local_*.txt` logs, and display `• N active` in the `LIVE INTEL FEED` header (without showing character names).
2. **Automatic Local System Tracking:** Read each active character's `Local_*.txt` (`Channel changed to Local : <SystemName>`) to maintain each character's live solar system.
3. **Failover Single-Channel Intel Watcher:** Monitor **one** active intel channel `.txt` belonging to an online character; if that character goes offline while another remains online, automatically switch to an online character's session file.
4. **Offline K-Space Stargate Graph (`0–5` Jumps):** Use a compact `zstd`-compressed stargate topology (~40–60 KB) to compute shortest stargate jump distance from **every active character** to reported systems in `< 0.1 ms` offline.
5. **Strict Intel Validation & Parsing:**
   - Reject casual chat and abbreviated/invalid system names (e.g. `C-N`, `B1`) — only **exact** known EVE solar system names are processed.
   - Extract structured tags (`system`, `pilot/other`, `spike`/`despike`, `+10`/`10+`, `shipType`) for English/ASCII lines.
   - For non-English lines (e.g. Russian/Cyrillic, Chinese/CJK), extract **only the valid solar system name(s)** and skip other text/ship parsing.
6. **Radius-Filtered Alarms (`0–5` Jumps in Advanced Settings):** Trigger audio alarm + collapsed island `Notice` **only** when a valid reported system is within the configured `0–5` jump radius of at least one active character (nearest systems list stays internal, not shown in the UI).

---

## 2. Active Character & `Local` Tracking Design

### A. Detecting Running EVE Clients (`exefile.exe`)
- Windows `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` (via `Win32_System_Diagnostics_ToolHelp`, already enabled in `crates/poni/Cargo.toml`) counts running `exefile.exe` processes and records their PIDs + process creation times (`GetProcessTimes`).
- **Count shown in UI:** `• N active` in the `LIVE INTEL FEED` header (green dot if `N > 0`, dim dot if `0`).
- **Lightweight scheduling ("Keep it light"):**
  - When the add-on is off, nothing runs.
  - When the add-on is on, the watcher checks `exefile.exe` count on wake / `Chatlogs` directory events / a light periodic tick (e.g., every `2 s` only while `exefile.exe > 0` or every `5 s` while waiting for EVE to start, sleeping at `INFINITE` when session is locked or display is off).

### B. Mapping Running Clients to `Local_*.txt`
Because EVE leaves historical `Local_<YYYYMMDD>_<HHMMSS>_<CharacterID>.txt` files on disk after logout without writing a "logged out" line:
1. Scan `Chatlogs` for `Local_*.txt` parsed via `parse_session_filename`.
2. Group by `CharacterID`, keeping each character's **newest** `Local` file (`stamp`).
3. Sort those per-character `Local` sessions newest-first and take at most **`N` = count of running `exefile.exe` processes**, prioritizing files modified/created within the active `exefile.exe` lifetimes.
   - If `N == 0`: `active_chars` is empty (`• 0 active`), no local systems are active, and no alarms fire.
4. Tail-read each active character's `Local_*.txt` (UTF-16LE, safe shared read) and parse:
   ```text
   EVE System > Channel changed to Local : <SystemName>
   ```
   to maintain `Vec<ActiveChar { char_id, system_name, system_id }>`.

### C. Single-Channel Intel Failover (Multibox Deduplication)
To avoid duplicate messages when multiple characters sit in the same intel channel:
- From all `<IntelChannel>_<YYYYMMDD>_<HHMMSS>_<CharacterID>.txt` files matching the user's configured channel:
  1. Prefer files whose `char_id` is in the currently active `active_chars` set.
  2. Pick the newest session (`stamp`) among those online characters.
  3. If the currently watched `char_id` goes offline (drops out of `active_chars`) and another online character has a matching `<IntelChannel>_...txt`, **automatically switch** to that online character's file and seek to its current end (so historical lines are not re-alerted).
  4. If `N == 0` (all characters offline), pause intel tailing / alarms while keeping `• 0 active`.

---

## 3. Compact K-Space Stargate Map (`poni-core`)

### A. Static Data Pipeline (`tools/make_eve_map.py`)
- Input: CCP / Fuzzwork SDE dumps (`mapSolarSystems.csv`, `mapSolarSystemJumps.csv`).
- Output: `crates/poni-core/data/eve-kspace.bin.zst` (~40–60 KB `zstd` compressed; decompressed at runtime once when EVE Intel add-on starts via `ruzstd`, already in `poni-core`).
- Binary layout (little-endian, compact):
  - `u16 system_count`
  - For each system `0..system_count`: `u8 name_len`, `[u8; name_len]` (canonical UTF-8 name, e.g. `4HS-CR`, `B17O-R`, `Jita`)
  - `u32 edge_count`
  - `edge_count` undirected edges as `(u16 from_idx, u16 to_idx)`
- Only k-space systems (and systems with stargate connections) are indexed for BFS; wormhole systems have no fixed gates.

### B. Runtime Graph & BFS (`poni-core::eve_map`)
- Built lazily once when the EVE Intel add-on is enabled (`OnceLock` / worker state); unloaded or never initialized when the add-on is off.
- Hash map / sorted table for **exact** case-insensitive system lookup (`lookup_exact(token) -> Option<u16>`).
- `nearest_within(origin_ids: &[u16], target_id: u16, max_jumps: u8) -> Option<u8>`:
  - Bounded BFS (`depth <= max_jumps`, `max_jumps <= 5`) from each active character's `system_id` (or multi-source BFS from all active chars' systems at once!).
  - Multi-source BFS finds `min_jumps` across all active characters in a single tiny queue pass (`< 0.05 ms`).
  - Also exposes `systems_within(origin_id: u16, max_jumps: u8)` for internal per-character 5-jump neighborhood caching.

---

## 4. Strict Intel Validation & Message Parsing (`poni-core::eve_intel`)

### A. Filtering Non-Intel & Abbreviated Systems
1. Ignore `EVE System` lines (`Channel MOTD`, etc.).
2. Tokenize message and search for **exact** matches in `EveMap::lookup_exact`:
   - Check single tokens (stripped of surrounding punctuation like `,`, `.`, `:`, `(`, `)`, `[`, `]`) and 2–3 word sequences (for multi-word systems like `Old Man Star`).
   - **No prefix / partial matching:** `C-N` or `B1` will **not** match `C-N4OD` or `B17O-R` unless `C-N` / `B1` is literally an exact full system name in the SDE.
   - **No valid system found → drop/do not process:** Casual conversation and invalid/abbreviated lines are ignored completely (no feed entry, no alarm, no pop-up).

### B. Foreign Language Detection vs. Structured English Parsing
1. **Non-English Script Detection (`has_non_latin_script(text)`):**
   - Detect characters in Cyrillic (`\u{0400}..=\u{04FF}`), CJK (`\u{4E00}..=\u{9FFF}`, `\u{3400}..=\u{4DBF}`), Hangul, Arabic, etc.
   - If detected: **extract only the valid system name(s)** (`ParsedIntel { systems, mode: ForeignSystemOnly, .. }`). Do **not** parse ship types, spike keywords, or pilot names from the foreign text.
2. **English / ASCII Structured Parsing:**
   - **System(s):** exact matches from `EveMap` (`4HS-CR`, `B17O-R`).
   - **Alert tags (`spike` / `despike` / count):** detect `spike`, `despike`, and numeric gang counts like `+10`, `10+`, `x5` (`^\+\d+$` or `^\d+\+$`).
   - **Ship types:** match tokens against a compact normalized ship-type set (interdictors, interceptors, HACs, recons, T3Cs, capitals, blops, cynos, etc. — e.g. `sabre`, `flycatcher`, `eris`, `heretic`, `stiletto`, `malediction`, `ares`, `crow`, `muninn`, `eagle`, `cerberus`, `rokh`, `loki`, `tengu`, `proteus`, `legion`, `cyno`, `titan`, `super`, `fax`, `dread`, `carrier`, `rorq`, `shuttle`, `capsule`, `pod`, `covops`, `dictor`, `hic`, `blops`, `marauder`).
   - **Pilots / Other:** remaining non-filler tokens (excluding common intel prepositions/noise like `on`, `in`, `at`, `gate`, `scan`, `dscan`, `nv`, `visual`).
3. **Clear / Ignore Words:**
   - If `should_alert` matches configured `ignore_words` (e.g. `clr`, `clear`): suppress alarm & collapsed pop-up (can still log/skip according to existing ignore rule; if `despike` is treated as informational or alertable, keep `clr, clear` as default suppressors).

---

## 5. UI & Settings Updates

### A. Expanded Island (`crates/poni/src/pages/eve_intel.rs`)
- **Left Column:** Unchanged structure (small `EVE SSO · Public` button, manual Intel Channel input + `Watch` button, matched `.txt` status, `Alarm Settings` + `Test` buttons).
- **Right Column (`LIVE INTEL FEED`):**
  - Header shows `LIVE INTEL FEED` on the left and **`• N active`** on the right (green dot when `N > 0`, dim when `0`; no character names).
  - Internal 5-system proximity list is **not** shown in the UI.
  - Each validated feed row shows: `[HH:MM:SS]`, nearest jump badge (e.g. `0j`, `2j`, `>5j`), parsed system name(s), and (for English lines) extracted tags/ships/pilots, or system-only summary for foreign-language lines.

### B. Advanced Settings (`crates/poni/src/settings_window.rs` & `poni-core/src/settings.rs`)
- Add `pub alert_jumps: u8` (default `5`, clamped `0..=5`) to `EveIntelSettings`.
- In `Section::EveIntel` → **Alerts** group, add stepper:
  - **Label:** `Detection radius`
  - **Sub:** `Alert for reports within this many jumps of an active character`
  - **Values:** `0 jumps (same system)` / `1 jump` .. `5 jumps` (`step = 1`, range `0..=5`).

### C. Alert Trigger Condition
When a validated intel line arrives:
1. Check `ignore_words` (`clr, clear`).
2. Compute `min_jumps = EveMap::nearest_within(&active_char_systems, &reported_systems, settings.alert_jumps)`.
3. **Only if `min_jumps.is_some()`** (meaning at least one active character is within `0..=alert_jumps` of a reported system):
   - Show collapsed island `Notice` (if `popup_notice` is on and island is collapsed), displaying e.g. `EVE Intel · 4HS-CR (2j)` and the parsed summary.
   - Play alarm sound via `AlarmPlayer` (if `alarm_on` is true and `cooldown_s` elapsed).

---

## 6. Implementation Steps & Verification

1. **Map Builder (`tools/make_eve_map.py` + `crates/poni-core/data/eve-kspace.bin.zst`):** Generate compressed k-space topology from Fuzzwork SDE CSVs and add `crates/poni-core/src/eve_map.rs` with lookup + BFS unit tests (`4HS-CR`, `B17O-R`, etc.).
2. **Core Parser & Settings (`crates/poni-core/src/eve_intel.rs`, `settings.rs`):** Add `Local` system-change parser, strict exact-system validation (reject `C-N`, `B1`, casual chat), English structured parser (`spike`/`despike`/`+10`/`10+`/`shipType`/pilots), non-Latin system-only extraction, and `alert_jumps: u8` (`0..=5`).
3. **Windows Service (`crates/poni/src/services/eve_intel.rs`):** Add `exefile.exe` process counter (`ToolHelp32`), active `Local_*.txt` mapping per active character, single-channel failover watcher across online characters, and distance evaluation.
4. **UI & Settings Wiring (`pages/eve_intel.rs`, `settings_window.rs`, `state.rs`, `app.rs`):** Show `• N active` in feed header, render validated parsed rows + jump badges, add `0–5` jump radius stepper in Advanced Settings, and gate alarm/`Notice` strictly on `min_jumps <= alert_jumps`.
5. **Verification:** `cargo test -p poni-core`, `cargo test -p poni`, `cargo build`, plus live test with real EVE `Local_*.txt` and intel channel logs.
