//! Pure logic for the EVE Online Intel Tool add-on: UTF-16LE chatlog
//! tail-decoding, session filename matching, line parsing, and alert filtering.

use crate::eve_map::EveMap;
use std::path::PathBuf;

/// Default keywords that suppress an alarm (e.g. a system reported clear).
pub const DEFAULT_IGNORE_WORDS: &str = "clr, clear";

/// Parsed header block from an EVE Online chatlog file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogHeader {
    pub channel_id: String,
    pub channel_name: String,
    pub listener: String,
}

/// One chat line parsed from an EVE Online chatlog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    /// Timestamp as written in the log, e.g. `2026.11.25 14:38:49`.
    pub timestamp: String,
    /// `HH:MM:SS` slice for compact display.
    pub time_short: String,
    pub sender: String,
    pub text: String,
    pub is_system: bool,
}

/// Metadata parsed from `<ChannelName>_<YYYYMMDD>_<HHMMSS>_<CharacterID>.txt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFile {
    pub filename: String,
    pub channel_name: String,
    /// `YYYYMMDD_HHMMSS`, lexicographically comparable for picking newest.
    pub stamp: String,
    pub char_id: String,
}

/// Default EVE chatlogs directory on Windows: `<USERPROFILE>\Documents\EVE\logs\Chatlogs`.
pub fn default_chatlogs_dir(user_profile: Option<&str>) -> Option<PathBuf> {
    let home = user_profile.map(str::trim).filter(|s| !s.is_empty())?;
    Some(PathBuf::from(home).join("Documents").join("EVE").join("logs").join("Chatlogs"))
}

/// Decode a streamed byte chunk of UTF-16LE text, carrying over any trailing
/// odd byte in `carry` so a multi-byte character split across reads is not corrupted.
/// Strips UTF-16 BOM (`\u{FEFF}`), which EVE inserts at file start and before each line.
pub fn decode_utf16le_chunk(bytes: &[u8], carry: &mut Option<u8>) -> String {
    let mut all = Vec::with_capacity(bytes.len() + usize::from(carry.is_some()));
    if let Some(b) = carry.take() {
        all.push(b);
    }
    all.extend_from_slice(bytes);
    if all.len() % 2 == 1 {
        *carry = all.pop();
    }
    let units = all.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
    char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .filter(|&c| c != '\u{FEFF}')
        .collect()
}

/// Parse an EVE chatlog filename `<ChannelName>_<YYYYMMDD>_<HHMMSS>_<CharacterID>.txt`.
pub fn parse_session_filename(filename: &str) -> Option<SessionFile> {
    let stem = filename.strip_suffix(".txt").or_else(|| filename.strip_suffix(".TXT"))?;
    let (rest, char_id) = stem.rsplit_once('_')?;
    if char_id.is_empty() || !char_id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (rest, time) = rest.rsplit_once('_')?;
    if time.len() != 6 || !time.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (channel_name, date) = rest.rsplit_once('_')?;
    if channel_name.is_empty() || date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(SessionFile {
        filename: filename.to_string(),
        channel_name: channel_name.to_string(),
        stamp: format!("{date}_{time}"),
        char_id: char_id.to_string(),
    })
}

/// True when `file_channel` matches the user-entered `configured_channel` (case-insensitive).
pub fn channel_matches(file_channel: &str, configured_channel: &str) -> bool {
    let want = configured_channel.trim();
    !want.is_empty() && file_channel.trim().eq_ignore_ascii_case(want)
}

/// Pick the newest session file matching `channel` by `YYYYMMDD_HHMMSS` timestamp.
pub fn pick_latest_session<'a>(filenames: impl IntoIterator<Item = &'a str>, channel: &str) -> Option<SessionFile> {
    filenames
        .into_iter()
        .filter_map(parse_session_filename)
        .filter(|s| channel_matches(&s.channel_name, channel))
        .max_by(|a, b| a.stamp.cmp(&b.stamp).then_with(|| a.filename.cmp(&b.filename)))
}

/// Parse the header block at the top of an EVE chatlog.
pub fn parse_header(text: &str) -> Option<LogHeader> {
    let (mut channel_id, mut channel_name, mut listener) = (String::new(), String::new(), String::new());
    for raw in text.lines() {
        let line = raw.trim().trim_start_matches('\u{FEFF}');
        if let Some(v) = line.strip_prefix("Channel ID:") {
            channel_id = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("Channel Name:") {
            channel_name = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("Listener:") {
            listener = v.trim().to_string();
        }
    }
    if channel_name.is_empty() && channel_id.is_empty() {
        None
    } else {
        Some(LogHeader { channel_id, channel_name, listener })
    }
}

/// Parse one EVE chatlog line: `[ YYYY.MM.DD HH:MM:SS ] Sender > Message`.
pub fn parse_line(raw: &str) -> Option<ChatMessage> {
    let line = raw.trim_matches(|c| c == '\r' || c == '\n' || c == '\u{FEFF}').trim();
    let after_open = line.strip_prefix('[')?;
    let (ts_raw, rest) = after_open.split_once(']')?;
    let timestamp = ts_raw.trim();
    let time_short = timestamp.split_whitespace().nth(1).unwrap_or(timestamp).to_string();
    let (sender_raw, msg_raw) = rest.split_once('>')?;
    let sender = sender_raw.trim().to_string();
    let text = msg_raw.trim().to_string();
    if sender.is_empty() || text.is_empty() {
        return None;
    }
    let is_system = sender.eq_ignore_ascii_case("EVE System");
    Some(ChatMessage { timestamp: timestamp.to_string(), time_short, sender, text, is_system })
}

/// True when `msg` should trigger an alert/alarm given comma-separated `ignore_words`.
pub fn should_alert(msg: &ChatMessage, ignore_words: &str) -> bool {
    if msg.is_system || msg.text.trim().is_empty() {
        return false;
    }
    let lower_words: Vec<String> = msg
        .text
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_lowercase())
        .collect();
    for raw_kw in ignore_words.split(',') {
        let kw = raw_kw.trim().to_ascii_lowercase();
        if kw.is_empty() {
            continue;
        }
        if msg.text.trim().eq_ignore_ascii_case(&kw) || lower_words.iter().any(|w| w == &kw) {
            return false;
        }
    }
    true
}

/// Parse `EVE System > Channel changed to Local : <SystemName>` from a `Local_*.txt` line.
pub fn parse_local_system_change(msg: &ChatMessage) -> Option<String> {
    if !msg.is_system {
        return None;
    }
    let rest = msg.text.trim().strip_prefix("Channel changed to Local :")?;
    let sys = rest.trim();
    if sys.is_empty() {
        None
    } else {
        Some(sys.to_string())
    }
}

/// True when `text` contains non-Latin script (Cyrillic, CJK, Hangul, Arabic, etc.).
pub fn has_non_latin_script(text: &str) -> bool {
    text.chars()
        .any(|c| !c.is_ascii() && c.is_alphabetic() && !('\u{00C0}'..='\u{024F}').contains(&c))
}

/// Validated and parsed intel extracted from a chat message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedIntel {
    pub systems: Vec<String>,
    pub system_ids: Vec<u16>,
    pub foreign_only: bool,
    pub tags: Vec<String>,
    pub ships: Vec<String>,
    pub pilots: Vec<String>,
}

impl ParsedIntel {
    /// Compact human-readable line for feed and collapsed-island notices.
    pub fn summary(&self) -> String {
        let sys = self.systems.join(", ");
        if self.foreign_only {
            return sys;
        }
        let mut parts = vec![sys];
        if !self.tags.is_empty() {
            parts.push(self.tags.join(" "));
        }
        if !self.ships.is_empty() {
            parts.push(self.ships.join(" "));
        }
        if !self.pilots.is_empty() {
            parts.push(self.pilots.join(" "));
        }
        parts.join(" · ")
    }
}

fn clean_token(tok: &str) -> &str {
    tok.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ',' | '.' | ':' | ';' | '!' | '?' | '"' | '\'' | '*' | '<' | '>'))
}

fn is_count_or_spike_tag(tok: &str) -> bool {
    let lower = tok.to_ascii_lowercase();
    if lower == "spike" || lower == "despike" {
        return true;
    }
    if let Some(rest) = lower.strip_prefix('+') {
        return !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit());
    }
    if let Some(rest) = lower.strip_suffix('+') {
        return !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit());
    }
    if let Some(rest) = lower.strip_prefix('x') {
        return !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit());
    }
    false
}

fn is_filler_word(lower: &str) -> bool {
    matches!(
        lower,
        "in" | "on" | "at" | "to" | "from" | "of" | "the" | "and" | "or" | "with" | "by" | "near" | "gate" | "plex" | "belt"
            | "station" | "citadel" | "fort" | "fortizar" | "keep" | "keepstar" | "astra" | "astrahus" | "ansi" | "jb" | "scan"
            | "dscan" | "nv" | "visual" | "grid" | "local" | "clr" | "clear" | "status" | "ping" | "warp" | "warping" | "+" | "-" | "/" | ">"
    )
}

fn is_ship_type(lower: &str) -> bool {
    let base = lower.strip_suffix('s').unwrap_or(lower);
    matches!(
        lower,
        "sabre" | "flycatcher" | "eris" | "heretic" | "stiletto" | "malediction" | "ares" | "crow" | "crusader" | "taranis" | "raptor"
            | "claw" | "interceptor" | "cept" | "ceptor" | "dictor" | "hic" | "hictor" | "broadword" | "broadsword" | "onyx" | "devoter"
            | "phobos" | "loki" | "tengu" | "proteus" | "legion" | "t3" | "t3c" | "muninn" | "eagle" | "cerberus" | "cerb" | "vagabond"
            | "ishtar" | "zealot" | "sacrilege" | "deimos" | "hac" | "rokh" | "megathron" | "mega" | "hyperion" | "raven" | "typhoon"
            | "tempest" | "maelstrom" | "machariel" | "nightmare" | "barghest" | "bhaalgorn" | "rattlesnake" | "vindicator" | "redeemer"
            | "sin" | "panther" | "widow" | "marshal" | "blops" | "marauder" | "paladin" | "golem" | "kronos" | "vargur" | "cyno"
            | "titan" | "avatar" | "erebus" | "leviathan" | "ragnarok" | "super" | "nyx" | "hel" | "aeon" | "wyvern" | "revenant"
            | "vendetta" | "carrier" | "thanatos" | "nidhoggur" | "archon" | "chimera" | "fax" | "apostle" | "minokawa" | "lif"
            | "ninazu" | "dread" | "revelation" | "naglfar" | "phoenix" | "moros" | "zirnitra" | "rorq" | "rorqual" | "orca"
            | "shuttle" | "pod" | "capsule" | "covops" | "astero" | "stratios" | "pacifier" | "enforcer" | "monitor" | "draugur"
            | "kikimora" | "vedmak" | "ikishava" | "ikitursa" | "leshak" | "dreka" | "drekavac" | "rodiva" | "zarmazd" | "nomen"
            | "omen" | "caracal" | "vexor" | "thorax" | "stabber" | "rupture" | "moa" | "maller" | "osprey" | "scythe" | "augoror"
            | "exequror" | "basi" | "basilisk" | "scimitar" | "scimi" | "guardian" | "oneiros" | "arazu" | "lachesis" | "lach" | "rapier"
            | "huginn" | "rook" | "falcon" | "pilgrim" | "curse" | "recon" | "hecate" | "jackdaw" | "svipul" | "confessor" | "t3d"
            | "kestrel" | "condor" | "atron" | "rifter" | "slasher" | "punisher" | "executioner" | "tristan" | "worm" | "garmur"
            | "daredevil" | "dramiel" | "succubus" | "cruor" | "catalyst" | "cormorant" | "thrasher" | "coercer" | "dragoon"
            | "algos" | "corax" | "talwar" | "sunesis" | "gnosis" | "praxis" | "tornado" | "talos" | "oracle" | "naga" | "ferox"
            | "drake" | "hurricane" | "cane" | "harbinger" | "myrmidon" | "brutix" | "prowler" | "viator" | "crane" | "prorator"
            | "dst" | "impel" | "bustard" | "mastodon" | "occam" | "occator" | "freighter" | "jumpfreighter" | "jf" | "ark" | "rhea"
            | "anshar" | "nomad"
    ) || matches!(base, "sabre" | "dictor" | "hic" | "titan" | "super" | "dread" | "carrier" | "fax" | "cyno" | "shuttle" | "pod" | "ceptor" | "t3c" | "hac" | "blops" | "marauder" | "recon")
}

/// Strictly validate and parse an intel chat message using `map`.
///
/// Rejects casual chat and abbreviated/invalid system names (e.g. `C-N`, `B1`).
/// For non-English messages (e.g. Cyrillic/CJK), extracts only valid system names.
pub fn validate_and_parse(msg: &ChatMessage, map: &EveMap) -> Option<ParsedIntel> {
    if msg.is_system || msg.text.trim().is_empty() {
        return None;
    }
    let raw_tokens: Vec<&str> = msg.text.split_whitespace().map(clean_token).filter(|t| !t.is_empty()).collect();
    if raw_tokens.is_empty() {
        return None;
    }

    let mut consumed = vec![false; raw_tokens.len()];
    let mut systems = Vec::new();
    let mut system_ids = Vec::new();

    let mut i = 0;
    while i < raw_tokens.len() {
        let mut matched = false;
        for width in (1..=3).rev() {
            if i + width > raw_tokens.len() {
                continue;
            }
            let phrase = raw_tokens[i..i + width].join(" ");
            if let Some(id) = map.lookup_exact(&phrase) {
                if !system_ids.contains(&id) {
                    system_ids.push(id);
                    systems.push(map.name(id).to_string());
                }
                for flag in &mut consumed[i..i + width] {
                    *flag = true;
                }
                i += width;
                matched = true;
                break;
            }
        }
        if !matched {
            i += 1;
        }
    }

    if system_ids.is_empty() {
        return None;
    }

    let foreign_only = has_non_latin_script(&msg.text);
    if foreign_only {
        return Some(ParsedIntel {
            systems,
            system_ids,
            foreign_only: true,
            tags: Vec::new(),
            ships: Vec::new(),
            pilots: Vec::new(),
        });
    }

    let mut tags = Vec::new();
    let mut ships = Vec::new();
    let mut pilots = Vec::new();
    for (idx, tok) in raw_tokens.iter().enumerate() {
        if consumed[idx] {
            continue;
        }
        let lower = tok.to_ascii_lowercase();
        if is_count_or_spike_tag(tok) {
            tags.push(tok.to_string());
        } else if is_ship_type(&lower) {
            ships.push(tok.to_string());
        } else if !is_filler_word(&lower) {
            pilots.push(tok.to_string());
        }
    }

    Some(ParsedIntel {
        systems,
        system_ids,
        foreign_only: false,
        tags,
        ships,
        pilots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16le_across_odd_byte_splits_and_strips_boms() {
        let sample = "\u{FEFF}[ 2026.11.25 14:38:49 ] spacefarmer83 > 4HS-CR\r\n";
        let bytes: Vec<u8> = sample.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let split = 7; // intentionally odd offset
        let mut carry = None;
        let first = decode_utf16le_chunk(&bytes[..split], &mut carry);
        assert!(carry.is_some());
        let second = decode_utf16le_chunk(&bytes[split..], &mut carry);
        assert!(carry.is_none());
        assert_eq!(format!("{first}{second}"), "[ 2026.11.25 14:38:49 ] spacefarmer83 > 4HS-CR\r\n");
    }

    #[test]
    fn parses_session_filenames_and_picks_newest() {
        let files = [
            "Local_20261125_143805_2123995636.txt",
            "I. Ftn Intel_20261125_094316_2123995636.txt",
            "I. Ftn Intel_20261125_143805_2123995636.txt",
            "notes.txt",
        ];
        let picked = pick_latest_session(files, "i. ftn intel").unwrap();
        assert_eq!(picked.filename, "I. Ftn Intel_20261125_143805_2123995636.txt");
        assert_eq!(picked.char_id, "2123995636");
        assert_eq!(picked.stamp, "20261125_143805");
        assert_eq!(pick_latest_session(files, ""), None);
    }

    #[test]
    fn parses_header_and_chat_lines_and_filters_ignored_words() {
        let header = "\nChannel ID: player_2fb\nChannel Name: I. Ftn Intel\nListener: Aleg LaZY Boy\n";
        let h = parse_header(header).unwrap();
        assert_eq!(h.channel_name, "I. Ftn Intel");
        assert_eq!(h.listener, "Aleg LaZY Boy");

        let motd = parse_line("\u{FEFF}[ 2026.11.25 14:38:14 ] EVE System > Channel MOTD: test\r\n").unwrap();
        assert!(motd.is_system);
        assert!(!should_alert(&motd, DEFAULT_IGNORE_WORDS));

        let clr = parse_line("\u{FEFF}[ 2026.11.25 14:39:00 ] pilot1 > B17O-R clr\r\n").unwrap();
        assert!(!should_alert(&clr, DEFAULT_IGNORE_WORDS));

        let intel = parse_line("\u{FEFF}[ 2026.11.25 14:39:25 ] max9892 > NightsongzZ  4HS-CR sabre\r\n").unwrap();
        assert_eq!(intel.time_short, "14:39:25");
        assert_eq!(intel.sender, "max9892");
        assert_eq!(intel.text, "NightsongzZ  4HS-CR sabre");
        assert!(should_alert(&intel, DEFAULT_IGNORE_WORDS));
    }

    #[test]
    fn validates_exact_systems_rejects_chatter_and_handles_foreign_text() {
        let map = EveMap::global();

        let local = parse_line("\u{FEFF}[ 2026.11.25 14:43:43 ] EVE System > Channel changed to Local : NY6-FH\r\n").unwrap();
        assert_eq!(parse_local_system_change(&local).as_deref(), Some("NY6-FH"));

        let short = parse_line("\u{FEFF}[ 2026.11.25 14:40:00 ] pilot1 > B1 spike\r\n").unwrap();
        assert_eq!(validate_and_parse(&short, map), None);

        let abbr = parse_line("\u{FEFF}[ 2026.11.25 14:40:01 ] pilot1 > C-N +10\r\n").unwrap();
        assert_eq!(validate_and_parse(&abbr, map), None);

        let chat = parse_line("\u{FEFF}[ 2026.11.25 14:40:02 ] pilot1 > anyone have ammo?\r\n").unwrap();
        assert_eq!(validate_and_parse(&chat, map), None);

        let valid = parse_line("\u{FEFF}[ 2026.11.25 14:40:03 ] pilot1 > B17O-R spike +10 NightsongzZ sabre\r\n").unwrap();
        let parsed = validate_and_parse(&valid, map).unwrap();
        assert_eq!(parsed.systems, vec!["B17O-R"]);
        assert!(!parsed.foreign_only);
        assert_eq!(parsed.tags, vec!["spike", "+10"]);
        assert_eq!(parsed.ships, vec!["sabre"]);
        assert_eq!(parsed.pilots, vec!["NightsongzZ"]);

        let ru = parse_line("\u{FEFF}[ 2026.11.25 14:40:04 ] pilot2 > 4HS-CR сабра +5\r\n").unwrap();
        let parsed_ru = validate_and_parse(&ru, map).unwrap();
        assert_eq!(parsed_ru.systems, vec!["4HS-CR"]);
        assert!(parsed_ru.foreign_only);
        assert!(parsed_ru.ships.is_empty());
        assert!(parsed_ru.tags.is_empty());

        let zh = parse_line("\u{FEFF}[ 2026.11.25 14:40:05 ] pilot3 > KVN-36 有敌人 10+\r\n").unwrap();
        let parsed_zh = validate_and_parse(&zh, map).unwrap();
        assert_eq!(parsed_zh.systems, vec!["KVN-36"]);
        assert!(parsed_zh.foreign_only);
        assert!(parsed_zh.ships.is_empty());
    }
}
