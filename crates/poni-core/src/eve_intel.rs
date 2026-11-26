//! Pure logic for the EVE Online Intel Tool add-on: UTF-16LE chatlog
//! tail-decoding, session filename matching, line parsing, and alert filtering.

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
}
