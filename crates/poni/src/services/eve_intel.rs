//! EVE Online Intel Tool background service:
//! - Counts running `exefile.exe` clients via ToolHelp32 (`• N active`).
//! - Maps running clients to active `Local_*.txt` logs per `CharacterID` and
//!   tracks each character's current k-space solar system.
//! - Monitors **one** active intel channel file owned by an online character,
//!   failing over automatically to another online character if one logs off.
//! - Validates incoming lines against `EveMap` (rejecting casual chat and
//!   abbreviated systems like `C-N` or `B1`) and computes stargate jump distance.
//! - Plays custom alarm audio via WinRT `MediaPlayer` without creating a
//!   Windows media session (`CommandManager.SetIsEnabled(false)`).

use crate::commands::Sound;
use crate::events::{AppEvent, EventSink};
use poni_core::eve_intel::{self, ChatMessage, ParsedIntel, SessionFile};
use poni_core::eve_map::EveMap;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows::core::{w, HSTRING};
use windows::Foundation::{TypedEventHandler, Uri};
use windows::Media::Core::MediaSource;
use windows::Media::Playback::MediaPlayer;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    FindCloseChangeNotification, FindFirstChangeNotificationW, FindNextChangeNotification,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    CreateEventW, GetProcessTimes, OpenProcess, SetEvent, WaitForMultipleObjects, INFINITE, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, FOS_FILEMUSTEXIST, FOS_PATHMUSTEXIST,
    FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};

const INITIAL_HISTORY_BYTES: u64 = 16 * 1024;
const ACTIVE_FALLBACK_MS: u32 = 1500;
const IDLE_RETRY_MS: u32 = 3000;
const MAX_TRACKED_JUMPS: u8 = 15;
const UNIX_EPOCH_IN_FILETIME_100NS: u64 = 116_444_736_000_000_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Watch {
    pub active: bool,
    pub channel: String,
    pub dir: PathBuf,
    pub ignore_words: String,
}

#[derive(Clone, Debug)]
pub struct IntelItem {
    pub time_short: String,
    pub sender: String,
    pub parsed: ParsedIntel,
    pub min_jumps: Option<u8>,
    pub ignored: bool,
}

#[derive(Clone, Debug)]
pub enum EveIntelEvent {
    Session {
        active_clients: usize,
        filename: Option<String>,
        history: Vec<IntelItem>,
        error: Option<String>,
    },
    ActiveClients(usize),
    Intel(Vec<IntelItem>),
    PickedSound(String),
    PickedDir(String),
}

pub fn resolved_dir(custom: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = custom.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let profile = std::env::var("USERPROFILE").ok();
    eve_intel::default_chatlogs_dir(profile.as_deref())
}

struct SendHandle(HANDLE);
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

pub struct EveIntelService {
    watch: Arc<Mutex<Watch>>,
    wake: SendHandle,
}

impl EveIntelService {
    pub fn start(sink: EventSink) -> Result<Self, String> {
        let wake = unsafe { CreateEventW(None, false, false, None) }.map_err(|e| format!("CreateEventW: {e}"))?;
        let watch = Arc::new(Mutex::new(Watch::default()));
        let w_clone = Arc::clone(&watch);
        let wake_raw = wake.0 as isize;
        std::thread::Builder::new()
            .name("poni-eve-intel".into())
            .spawn(move || worker(sink, w_clone, HANDLE(wake_raw as *mut _)))
            .map_err(|e| format!("spawn: {e}"))?;
        Ok(Self { watch, wake: SendHandle(wake) })
    }

    pub fn set_watch(&self, next: Watch) {
        let mut cur = self.watch.lock().unwrap();
        if *cur == next {
            return;
        }
        *cur = next;
        unsafe {
            let _ = SetEvent(self.wake.0);
        }
    }
}

impl Drop for EveIntelService {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.wake.0);
        }
    }
}

struct LocalTail {
    filename: String,
    offset: u64,
    carry: Option<u8>,
    pending: String,
    system_id: Option<u16>,
}

struct TailState {
    channel: String,
    dir: PathBuf,
    ignore_words: String,
    active_clients: usize,
    locals: HashMap<String, LocalTail>,
    intel_file: Option<PathBuf>,
    intel_filename: Option<String>,
    intel_char_id: Option<String>,
    offset: u64,
    carry: Option<u8>,
    pending: String,
    last_error: Option<String>,
}

impl TailState {
    fn new() -> Self {
        Self {
            channel: String::new(),
            dir: PathBuf::new(),
            ignore_words: String::new(),
            active_clients: usize::MAX,
            locals: HashMap::new(),
            intel_file: None,
            intel_filename: None,
            intel_char_id: None,
            offset: 0,
            carry: None,
            pending: String::new(),
            last_error: None,
        }
    }

    fn reset(&mut self, channel: String, dir: PathBuf, ignore_words: String) {
        self.channel = channel;
        self.dir = dir;
        self.ignore_words = ignore_words;
        self.active_clients = usize::MAX;
        self.locals.clear();
        self.intel_file = None;
        self.intel_filename = None;
        self.intel_char_id = None;
        self.offset = 0;
        self.carry = None;
        self.pending.clear();
        self.last_error = None;
    }
}

fn worker(sink: EventSink, watch: Arc<Mutex<Watch>>, wake: HANDLE) {
    let mut state = TailState::new();
    let mut watched_dir: Option<PathBuf> = None;
    let mut dir_notify: Option<HANDLE> = None;

    loop {
        let w = watch.lock().unwrap().clone();
        if !w.active || w.channel.trim().is_empty() {
            if state.intel_file.is_some() || !state.channel.is_empty() {
                state.reset(String::new(), PathBuf::new(), String::new());
            }
            close_dir_notify(&mut dir_notify, &mut watched_dir);
            unsafe {
                WaitForMultipleObjects(&[wake], false, INFINITE);
            }
            continue;
        }

        if state.channel != w.channel || state.dir != w.dir {
            state.reset(w.channel.clone(), w.dir.clone(), w.ignore_words.clone());
            close_dir_notify(&mut dir_notify, &mut watched_dir);
        } else {
            state.ignore_words = w.ignore_words.clone();
        }

        if watched_dir.as_ref() != Some(&w.dir) {
            close_dir_notify(&mut dir_notify, &mut watched_dir);
            if w.dir.is_dir() {
                let wide = HSTRING::from(w.dir.as_os_str());
                let h = unsafe {
                    FindFirstChangeNotificationW(
                        &wide,
                        false,
                        FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_SIZE,
                    )
                };
                if let Ok(handle) = h {
                    if handle != INVALID_HANDLE_VALUE {
                        dir_notify = Some(handle);
                        watched_dir = Some(w.dir.clone());
                    }
                }
            }
        }

        poll_chatlogs(&sink, &mut state);

        let timeout = if dir_notify.is_some() {
            if state.active_clients > 0 || state.intel_file.is_some() {
                ACTIVE_FALLBACK_MS
            } else {
                IDLE_RETRY_MS
            }
        } else {
            IDLE_RETRY_MS
        };

        let wait_idx = match dir_notify {
            Some(dh) => unsafe { WaitForMultipleObjects(&[wake, dh], false, timeout) },
            None => unsafe { WaitForMultipleObjects(&[wake], false, timeout) },
        };
        if wait_idx.0 == WAIT_OBJECT_0.0 + 1 {
            if let Some(dh) = dir_notify {
                unsafe {
                    let _ = FindNextChangeNotification(dh);
                }
            }
        }
    }
}

fn close_dir_notify(dir_notify: &mut Option<HANDLE>, watched_dir: &mut Option<PathBuf>) {
    if let Some(h) = dir_notify.take() {
        unsafe {
            let _ = FindCloseChangeNotification(h);
        }
    }
    *watched_dir = None;
}

struct RunningClients {
    count: usize,
    earliest_start: Option<SystemTime>,
}

fn filetime_to_system(ft: FILETIME) -> Option<SystemTime> {
    let ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    if ticks < UNIX_EPOCH_IN_FILETIME_100NS {
        return None;
    }
    let delta_ns = (ticks - UNIX_EPOCH_IN_FILETIME_100NS) * 100;
    Some(UNIX_EPOCH + Duration::from_nanos(delta_ns))
}

fn running_eve_clients() -> RunningClients {
    let mut count = 0usize;
    let mut earliest_start: Option<SystemTime> = None;
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return RunningClients { count: 0, earliest_start: None };
        };
        let mut pe = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut pe).is_ok() {
            loop {
                let end = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len());
                let exe = String::from_utf16_lossy(&pe.szExeFile[..end]);
                if exe.eq_ignore_ascii_case("exefile.exe") {
                    count += 1;
                    if let Ok(proc_h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pe.th32ProcessID) {
                        let (mut create, mut exit, mut kernel, mut user) = Default::default();
                        if GetProcessTimes(proc_h, &mut create, &mut exit, &mut kernel, &mut user).is_ok() {
                            if let Some(t) = filetime_to_system(create) {
                                earliest_start = Some(match earliest_start {
                                    Some(prev) => prev.min(t),
                                    None => t,
                                });
                            }
                        }
                        let _ = CloseHandle(proc_h);
                    }
                }
                if Process32NextW(snap, &mut pe).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    RunningClients { count, earliest_start }
}

fn poll_chatlogs(sink: &EventSink, st: &mut TailState) {
    let clients = running_eve_clients();
    let map = EveMap::global();

    if !st.dir.is_dir() {
        let err = format!("Folder not found: {}", st.dir.display());
        if st.last_error.as_deref() != Some(&err) || st.active_clients != clients.count {
            st.intel_file = None;
            st.intel_filename = None;
            st.intel_char_id = None;
            st.locals.clear();
            st.active_clients = clients.count;
            st.last_error = Some(err.clone());
            sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
                active_clients: clients.count,
                filename: None,
                history: Vec::new(),
                error: Some(err),
            }));
        }
        return;
    }

    let Ok(entries) = fs::read_dir(&st.dir) else {
        return;
    };

    let mut local_by_char: HashMap<String, (SessionFile, SystemTime)> = HashMap::new();
    let mut intel_sessions: Vec<(SessionFile, SystemTime)> = Vec::new();
    let cutoff = clients
        .earliest_start
        .map(|t| t.checked_sub(Duration::from_secs(15)).unwrap_or(UNIX_EPOCH));

    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else { continue };
        let Some(sf) = eve_intel::parse_session_filename(&name) else { continue };
        let modified = entry.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);

        if sf.channel_name.eq_ignore_ascii_case("Local") {
            if let Some(cut) = cutoff {
                if modified < cut {
                    continue;
                }
            }
            match local_by_char.get(&sf.char_id) {
                Some((cur, _)) if cur.stamp >= sf.stamp => {}
                _ => {
                    local_by_char.insert(sf.char_id.clone(), (sf, modified));
                }
            }
        } else if eve_intel::channel_matches(&sf.channel_name, &st.channel) {
            intel_sessions.push((sf, modified));
        }
    }

    // Map active `exefile.exe` processes to the newest unique `CharacterID` `Local` logs.
    let mut active_locals: Vec<(SessionFile, SystemTime)> = if clients.count == 0 {
        Vec::new()
    } else {
        let mut v: Vec<_> = local_by_char.into_values().collect();
        v.sort_by(|a, b| b.0.stamp.cmp(&a.0.stamp).then_with(|| b.1.cmp(&a.1)));
        v.truncate(clients.count);
        v
    };

    let active_char_ids: HashSet<String> = active_locals.iter().map(|(sf, _)| sf.char_id.clone()).collect();
    st.locals.retain(|cid, _| active_char_ids.contains(cid));

    for (sf, _) in active_locals.drain(..) {
        update_local_tail(&st.dir, &sf, &mut st.locals, map);
    }

    let active_system_ids: Vec<u16> = st.locals.values().filter_map(|l| l.system_id).collect();

    if clients.count == 0 {
        if st.active_clients != 0 || st.intel_file.is_some() {
            st.active_clients = 0;
            st.intel_file = None;
            st.intel_filename = None;
            st.intel_char_id = None;
            st.last_error = Some("EVE Online not running".into());
            sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
                active_clients: 0,
                filename: None,
                history: Vec::new(),
                error: st.last_error.clone(),
            }));
        }
        return;
    }

    if st.active_clients != clients.count {
        st.active_clients = clients.count;
        sink.send(AppEvent::EveIntel(EveIntelEvent::ActiveClients(clients.count)));
    }

    // Pick 1 active intel file from an online character (automatic failover when one goes offline).
    let picked = pick_online_intel_session(&intel_sessions, &active_char_ids, st.intel_char_id.as_deref());
    let Some(session) = picked else {
        let err = format!("Waiting for \"{}_*.txt\"", st.channel.trim());
        if st.intel_file.is_some() || st.last_error.as_deref() != Some(&err) {
            st.intel_file = None;
            st.intel_filename = None;
            st.intel_char_id = None;
            st.last_error = Some(err.clone());
            sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
                active_clients: clients.count,
                filename: None,
                history: Vec::new(),
                error: Some(err),
            }));
        }
        return;
    };

    let switched = st.intel_filename.as_deref() != Some(&session.filename);
    if switched {
        let path = st.dir.join(&session.filename);
        let (history, offset, carry) = read_initial_intel_tail(&path, map, &active_system_ids, &st.ignore_words);
        st.intel_file = Some(path);
        st.intel_filename = Some(session.filename.clone());
        st.intel_char_id = Some(session.char_id);
        st.offset = offset;
        st.carry = carry;
        st.pending.clear();
        st.last_error = None;
        sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
            active_clients: clients.count,
            filename: Some(session.filename),
            history,
            error: None,
        }));
        return;
    }

    let Some(path) = st.intel_file.as_ref() else { return };
    let Ok(mut file) = File::open(path) else { return };
    let Ok(meta) = file.metadata() else { return };
    let len = meta.len();
    if len <= st.offset {
        return;
    }
    if file.seek(SeekFrom::Start(st.offset)).is_err() {
        return;
    }
    let mut buf = Vec::with_capacity((len - st.offset).min(256 * 1024) as usize);
    if file.read_to_end(&mut buf).is_err() || buf.is_empty() {
        return;
    }
    st.offset += buf.len() as u64;
    let decoded = eve_intel::decode_utf16le_chunk(&buf, &mut st.carry);
    st.pending.push_str(&decoded);

    let mut items = Vec::new();
    while let Some(pos) = st.pending.find('\n') {
        let line: String = st.pending.drain(..=pos).collect();
        if let Some(msg) = eve_intel::parse_line(&line) {
            if let Some(item) = to_intel_item(&msg, map, &active_system_ids, &st.ignore_words) {
                items.push(item);
            }
        }
    }
    if !items.is_empty() {
        sink.send(AppEvent::EveIntel(EveIntelEvent::Intel(items)));
    }
}

fn pick_online_intel_session(
    sessions: &[(SessionFile, SystemTime)],
    active_char_ids: &HashSet<String>,
    current_char_id: Option<&str>,
) -> Option<SessionFile> {
    if sessions.is_empty() {
        return None;
    }
    // Keep watching current online character's newest file if still active.
    if let Some(cid) = current_char_id {
        if active_char_ids.contains(cid) {
            if let Some((sf, _)) = sessions
                .iter()
                .filter(|(sf, _)| sf.char_id == cid)
                .max_by(|a, b| a.0.stamp.cmp(&b.0.stamp).then_with(|| a.1.cmp(&b.1)))
            {
                return Some(sf.clone());
            }
        }
    }
    // Failover: pick newest intel file belonging to any currently online character.
    if let Some((sf, _)) = sessions
        .iter()
        .filter(|(sf, _)| active_char_ids.contains(&sf.char_id))
        .max_by(|a, b| a.0.stamp.cmp(&b.0.stamp).then_with(|| a.1.cmp(&b.1)))
    {
        return Some(sf.clone());
    }
    // Fallback if Local hasn't been created yet while client is loading into space.
    sessions
        .iter()
        .max_by(|a, b| a.0.stamp.cmp(&b.0.stamp).then_with(|| a.1.cmp(&b.1)))
        .map(|(sf, _)| sf.clone())
}

fn update_local_tail(dir: &Path, sf: &SessionFile, locals: &mut HashMap<String, LocalTail>, map: &EveMap) {
    let path = dir.join(&sf.filename);
    let Ok(mut file) = File::open(&path) else { return };
    let Ok(meta) = file.metadata() else { return };
    let len = meta.len();

    let need_init = match locals.get(&sf.char_id) {
        Some(cur) => cur.filename != sf.filename,
        None => true,
    };

    if need_init {
        let mut buf = Vec::with_capacity(len.min(256 * 1024) as usize);
        let read_len = file.read_to_end(&mut buf).unwrap_or(0);
        let mut carry = None;
        let text = eve_intel::decode_utf16le_chunk(&buf[..read_len], &mut carry);
        let mut system_id = None;
        for line in text.lines() {
            if let Some(msg) = eve_intel::parse_line(line) {
                if let Some(sys) = eve_intel::parse_local_system_change(&msg) {
                    if let Some(id) = map.lookup_exact(&sys) {
                        system_id = Some(id);
                    }
                }
            }
        }
        locals.insert(
            sf.char_id.clone(),
            LocalTail {
                filename: sf.filename.clone(),
                offset: read_len as u64,
                carry,
                pending: String::new(),
                system_id,
            },
        );
        return;
    }

    let Some(lt) = locals.get_mut(&sf.char_id) else { return };
    if len <= lt.offset {
        return;
    }
    if file.seek(SeekFrom::Start(lt.offset)).is_err() {
        return;
    }
    let mut buf = Vec::with_capacity((len - lt.offset).min(64 * 1024) as usize);
    if file.read_to_end(&mut buf).is_err() || buf.is_empty() {
        return;
    }
    lt.offset += buf.len() as u64;
    let decoded = eve_intel::decode_utf16le_chunk(&buf, &mut lt.carry);
    lt.pending.push_str(&decoded);
    while let Some(pos) = lt.pending.find('\n') {
        let line: String = lt.pending.drain(..=pos).collect();
        if let Some(msg) = eve_intel::parse_line(&line) {
            if let Some(sys) = eve_intel::parse_local_system_change(&msg) {
                if let Some(id) = map.lookup_exact(&sys) {
                    lt.system_id = Some(id);
                }
            }
        }
    }
}

fn to_intel_item(msg: &ChatMessage, map: &EveMap, active_system_ids: &[u16], ignore_words: &str) -> Option<IntelItem> {
    let parsed = eve_intel::validate_and_parse(msg, map)?;
    let min_jumps = map.nearest_within(active_system_ids, &parsed.system_ids, MAX_TRACKED_JUMPS);
    let ignored = !eve_intel::should_alert(msg, ignore_words);
    Some(IntelItem {
        time_short: msg.time_short.clone(),
        sender: msg.sender.clone(),
        parsed,
        min_jumps,
        ignored,
    })
}

fn read_initial_intel_tail(
    path: &Path,
    map: &EveMap,
    active_system_ids: &[u16],
    ignore_words: &str,
) -> (Vec<IntelItem>, u64, Option<u8>) {
    let Ok(mut file) = File::open(path) else {
        return (Vec::new(), 0, None);
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(INITIAL_HISTORY_BYTES) & !1;
    if file.seek(SeekFrom::Start(start)).is_err() {
        return (Vec::new(), len, None);
    }
    let mut tail = Vec::new();
    let read_len = file.read_to_end(&mut tail).unwrap_or(0);
    let mut carry = None;
    let text = eve_intel::decode_utf16le_chunk(&tail[..read_len], &mut carry);
    let lines: Vec<&str> = text.lines().collect();
    let skip_first = usize::from(start > 0 && !lines.is_empty());
    let mut history: Vec<IntelItem> = lines
        .into_iter()
        .skip(skip_first)
        .filter_map(eve_intel::parse_line)
        .filter_map(|m| to_intel_item(&m, map, active_system_ids, ignore_words))
        .collect();
    if history.len() > 24 {
        history.drain(..history.len() - 24);
    }
    (history, start + read_len as u64, carry)
}

/// Non-media-session alarm audio player (same WinRT pattern as `AdzanPlayer`).
#[derive(Default)]
pub struct AlarmPlayer {
    player: Option<MediaPlayer>,
}

impl AlarmPlayer {
    pub fn play(&mut self, custom_path: Option<&str>, sink: &EventSink) {
        let Some(p) = custom_path.map(str::trim).filter(|s| !s.is_empty()) else {
            super::sound::play(Sound::Chime);
            return;
        };
        if self.play_file(Path::new(p), sink).is_err() {
            super::sound::play(Sound::Chime);
        }
    }

    fn play_file(&mut self, path: &Path, sink: &EventSink) -> Result<(), String> {
        self.stop();
        let uri = Uri::CreateUri(&HSTRING::from(super::adzan::file_uri(path))).map_err(|e| format!("bad uri ({e})"))?;
        let player = MediaPlayer::new().map_err(|e| format!("no media player ({e})"))?;
        if let Ok(cm) = player.CommandManager() {
            let _ = cm.SetIsEnabled(false);
        }
        let source = MediaSource::CreateFromUri(&uri).map_err(|e| format!("open failed ({e})"))?;
        player.SetSource(&source).map_err(|e| format!("set source ({e})"))?;
        let ended = sink.clone();
        let _ = player.MediaEnded(&TypedEventHandler::new(move |_, _| {
            ended.send(AppEvent::EveAlarmEnded);
            Ok(())
        }));
        let failed = sink.clone();
        let _ = player.MediaFailed(&TypedEventHandler::new(move |_, _| {
            crate::pwarn!("eve_intel: alarm playback failed");
            failed.send(AppEvent::EveAlarmEnded);
            Ok(())
        }));
        player.Play().map_err(|e| format!("play ({e})"))?;
        self.player = Some(player);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(p) = self.player.take() {
            let _ = p.Pause();
            let _ = p.Close();
        }
    }
}

/// Spawn an STA thread to open the native Windows file picker for a custom alarm sound.
pub fn pick_sound_async(sink: EventSink) {
    let _ = std::thread::Builder::new().name("poni-eve-pick-sound".into()).spawn(move || {
        if let Some(path) = open_dialog(false, None) {
            sink.send(AppEvent::EveIntel(EveIntelEvent::PickedSound(path)));
        }
    });
}

/// Spawn an STA thread to open the native Windows folder picker for the Chatlogs folder.
pub fn pick_dir_async(sink: EventSink, initial: Option<String>) {
    let _ = std::thread::Builder::new().name("poni-eve-pick-dir".into()).spawn(move || {
        if let Some(path) = open_dialog(true, initial.as_deref()) {
            sink.send(AppEvent::EveIntel(EveIntelEvent::PickedDir(path)));
        }
    });
}

fn open_dialog(folder: bool, initial: Option<&str>) -> Option<String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let need_uninit = hr.is_ok();
        let res = (|| -> windows::core::Result<String> {
            let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            let mut opts = dlg.GetOptions()?;
            opts |= FOS_PATHMUSTEXIST;
            if folder {
                opts |= FOS_PICKFOLDERS;
                dlg.SetTitle(w!("Choose EVE Online Chatlogs folder"))?;
            } else {
                opts |= FOS_FILEMUSTEXIST;
                dlg.SetTitle(w!("Choose EVE Intel alarm sound"))?;
                let specs = [
                    COMDLG_FILTERSPEC { pszName: w!("Audio files"), pszSpec: w!("*.wav;*.mp3;*.m4a;*.ogg;*.wma") },
                    COMDLG_FILTERSPEC { pszName: w!("All files"), pszSpec: w!("*.*") },
                ];
                let _ = dlg.SetFileTypes(&specs);
            }
            if let Some(dir) = initial.filter(|s| !s.trim().is_empty()) {
                let wide = HSTRING::from(dir);
                if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&wide, None) {
                    let _ = dlg.SetFolder(&item);
                }
            }
            dlg.SetOptions(opts)?;
            dlg.Show(None)?;
            let item = dlg.GetResult()?;
            let pwstr = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let s = pwstr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(pwstr.0 as *const _));
            Ok(s)
        })()
        .ok()
        .filter(|s| !s.is_empty());
        if need_uninit {
            CoUninitialize();
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intel_failover_switches_to_another_online_character() {
        let sf = |name: &str, ch: &str, stamp: &str, cid: &str| {
            (
                SessionFile {
                    filename: name.into(),
                    channel_name: ch.into(),
                    stamp: stamp.into(),
                    char_id: cid.into(),
                },
                UNIX_EPOCH,
            )
        };
        let sessions = vec![
            sf("I. Ftn Intel_20261125_143805_111.txt", "I. Ftn Intel", "20261125_143805", "111"),
            sf("I. Ftn Intel_20261125_143900_222.txt", "I. Ftn Intel", "20261125_143900", "222"),
        ];
        // Both online, keeping current 111.
        let both: HashSet<String> = ["111".into(), "222".into()].into_iter().collect();
        let kept = pick_online_intel_session(&sessions, &both, Some("111")).unwrap();
        assert_eq!(kept.char_id, "111");

        // 111 goes offline -> automatically switches to online 222.
        let only_222: HashSet<String> = ["222".into()].into_iter().collect();
        let failover = pick_online_intel_session(&sessions, &only_222, Some("111")).unwrap();
        assert_eq!(failover.char_id, "222");
    }
}
