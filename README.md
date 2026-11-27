<h1 align="center">Nafisland</h1>

<h3 align="center">A living notch for Windows — lightweight Dynamic Island in Rust with an EVE Online Intel Tool</h3>

<p align="center">
  An open-source living notch that hangs at the top of your screen, shows live media, timers, system HUDs, and real-time EVE Online intel alerts, and expands on hover into quick utility tabs.
</p>

<p align="center">
  <sub>Forked from <a href="https://github.com/lucacadalora/oyenza-studio">Oyenza Studio</a> &nbsp;·&nbsp; Built with Rust (Win32 + Direct2D + WinRT) &nbsp;·&nbsp; <a href="LICENSE">MIT License</a></sub>
</p>

---

## Origin & White-Label Config

This project (**PulauH / Nafisland**) is a fork of **[Oyenza Studio](https://github.com/lucacadalora/oyenza-studio)**. Upstream account/cloud integrations were removed and replaced with centralized white-label configuration in [`brand.toml`](brand.toml), new themes, and custom add-ons tailored for daily Windows use and EVE Online multiboxing.

---

## Features

### EVE Online Intel Tool (Add-on)
Enable **EVE Intel** in the **Add-ons (`+`)** tab to monitor live EVE Online chatlogs with near-zero CPU usage at rest:

- **Safe Live Chatlog Tailing:** Watches `%USERPROFILE%\Documents\EVE\logs\Chatlogs` via `FindFirstChangeNotificationW` and decodes EVE's UTF-16LE logs with `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` while the game keeps session files open.
- **Multibox Active Client Detection (`• N active`):** Counts running `exefile.exe` processes, displays `• N active` in the `LIVE INTEL FEED` header (without exposing character names), and maps active clients to their latest `Local_*.txt` logs to track each character's current system.
- **Single-Channel Failover:** Monitors one active intel channel file belonging to an online character and automatically switches to another online character's session file if one client logs off.
- **Offline K-Space Stargate Map (`0–5` Jumps):** Uses a compact `zstd`-compressed New Eden topology (`5,485` systems, `6,991` stargate edges in `crates/poni-core/data/eve-kspace.bin.zst`) for instant offline jump-distance checks across all active characters.
- **Strict Intel Validation & Multi-Language Filtering:**
  - Rejects casual chatter and invalid/abbreviated system names (such as `C-N` or `B1`); only exact valid EVE solar system names are processed.
  - Extracts structured intel (`system`, `spike`/`despike`, `+10`/`10+`, `shipType`, pilot names) for English/ASCII messages.
  - Detects non-Latin scripts (Chinese, Russian, etc.) and extracts only the valid solar system name(s).
- **Custom Alarms & Advanced Settings:**
  - Two-column expanded page: left side has a compact `EVE SSO · Public` button (PKCE SSO planned), manual channel name input, matched `.txt` status, and **Alarm Settings** shortcut; right side shows the live validated intel feed with jump badges.
  - **Advanced Settings → EVE Intel** lets you configure the Chatlogs folder, **Detection radius (`0–5 jumps`)**, custom alarm sound (`.wav`, `.mp3`, `.m4a` played via WinRT `MediaPlayer` without hijacking Windows media controls), cooldown, `clr`/`clear` ignore filter, and collapsed island pop-up alerts.

> Design & validation specs: [docs/eve-intel.md](docs/eve-intel.md) · [docs/eve-intel-validation.md](docs/eve-intel-validation.md)

### Music & Synced Lyrics
- Controls for Spotify, Apple Music, browser players (YouTube, etc.), and any player exposed through Windows GSMTC media controls.
- Synced lyrics from [LRCLIB](https://lrclib.net) (plus optional Apple Music web token), with pronunciation/romaji under Japanese, Korean, and Chinese lines.
- Displays the current sung line on the collapsed island and supports scrubbing via the progress bar.

### Themes (Including Horizon)
- **Classic:** Pure black island anchored to the top edge of the screen.
- **Horizon:** Dynamic sky gradient, sun/moon position, and stars synced to the time of day.
- **Glass:** Frosted/refractive backdrop inspired by Liquid Glass.
- **Pill:** Floating capsule below the top edge with status-bar idle mode.
- **Diorama:** Pixel-art day/night scene inside the resting pill (`crates/studio-diorama`).
- **Mascot:** Warm theme with animated mascot peeking over the island.

### Everyday Pages & Pop-Ups
- **Today:** Date overview and calendar view.
- **Shelf & Phone Drop:** Drag and drop files onto the island, or transfer files from your phone over local Wi-Fi via QR code (`poni_core::phonedrop`).
- **Tasks & Focus:** Quick to-do list and Pomodoro focus timer that continues counting on the collapsed island.
- **HUD Pop-Ups:** Brief collapsed overlays for volume changes, charger/battery state, and Bluetooth connections.
- **Show Over Games:** Optional FPS, GPU, and CPU readout over borderless full-screen games.
- **Built-in Add-ons (`+`):** Toggle **System** (live CPU, GPU, memory, drives, network, power), **Screen recorder** integration, and **EVE Intel**.

---

## Light by Design

- **Native Rust:** Win32 + Direct2D + WinRT with no WebView or heavyweight runtime.
- **Zero-Frame Idle:** Renders on vblank (`60–240 Hz`) only while animating or interacting; sleeps when idle, locked, or display is off.
- **Local Storage:** Settings live in `%APPDATA%\Poni\settings.json`, app data in `%APPDATA%\Poni\data.json`, and logs/cache in `%LOCALAPPDATA%\Poni\`.

---

## Building

Requires **Rust (stable MSVC toolchain)** on Windows 10/11:

```powershell
# Run core & app unit tests
cargo test -p poni-core
cargo test -p poni

# Build release binary (target\release\poni.exe)
cargo build -p poni --release
```

To rebuild the compressed EVE k-space stargate graph from Fuzzwork SDE CSV dumps (`zstd` CLI required in `PATH`):

```powershell
python tools/make_eve_map.py
```

To customize product naming, vendor metadata, or external endpoints, edit [`brand.toml`](brand.toml) and rebuild. See [`ARCHITECTURE.md`](ARCHITECTURE.md) for crate layout, thread model, and UI contracts.

---

## Credits & License

- Forked from **[Oyenza Studio](https://github.com/lucacadalora/oyenza-studio)** (MIT License).
- EVE Online universe system & stargate topology sourced from [Fuzzwork SDE](https://www.fuzzwork.co.uk/dump/).
- Synced lyrics powered by [LRCLIB](https://lrclib.net).
- Built with [windows-rs](https://github.com/microsoft/windows-rs), [serde](https://serde.rs), [qrcodegen](https://github.com/nayuki/QR-Code-generator), [ruzstd](https://github.com/KillingSpark/zstd-rs), [vibrato](https://github.com/daac-tools/vibrato), and [str0m](https://github.com/algesten/str0m).

Licensed under the [MIT License](LICENSE). See [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt) for third-party notices.
