//! EVE Online Intel Tool background service:
//! - Watches the user's `Chatlogs` folder using `FindFirstChangeNotificationW`
//!   + an event-driven wake handle (0% CPU while off/idle).
//! - Tail-reads UTF-16LE chatlogs safely while EVE Online keeps the file open.
//! - Plays custom alarm audio via WinRT `MediaPlayer` without creating a
//!   Windows media session (`CommandManager.SetIsEnabled(false)`).
//! - Opens native Windows file/folder pickers on an STA worker thread so the UI
//!   thread never blocks.

use crate::commands::Sound;
use crate::events::{AppEvent, EventSink};
use poni_core::eve_intel::{self, ChatMessage};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use windows::core::{w, HSTRING};
use windows::Foundation::{TypedEventHandler, Uri};
use windows::Media::Core::MediaSource;
use windows::Media::Playback::MediaPlayer;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    FindCloseChangeNotification, FindFirstChangeNotificationW, FindNextChangeNotification,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects, INFINITE};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, FOS_FILEMUSTEXIST, FOS_PATHMUSTEXIST,
    FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};

const INITIAL_HISTORY_BYTES: u64 = 16 * 1024;
const ACTIVE_FALLBACK_MS: u32 = 1500;
const IDLE_RETRY_MS: u32 = 5000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Watch {
    pub active: bool,
    pub channel: String,
    pub dir: PathBuf,
}

#[derive(Clone, Debug)]
pub enum EveIntelEvent {
    Session {
        filename: Option<String>,
        listener: Option<String>,
        history: Vec<ChatMessage>,
        error: Option<String>,
    },
    Messages(Vec<ChatMessage>),
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

struct TailState {
    channel: String,
    dir: PathBuf,
    file_path: Option<PathBuf>,
    filename: Option<String>,
    offset: u64,
    carry: Option<u8>,
    pending: String,
}

impl TailState {
    fn reset(&mut self, channel: String, dir: PathBuf) {
        self.channel = channel;
        self.dir = dir;
        self.file_path = None;
        self.filename = None;
        self.offset = 0;
        self.carry = None;
        self.pending.clear();
    }
}

fn worker(sink: EventSink, watch: Arc<Mutex<Watch>>, wake: HANDLE) {
    let mut state = TailState {
        channel: String::new(),
        dir: PathBuf::new(),
        file_path: None,
        filename: None,
        offset: 0,
        carry: None,
        pending: String::new(),
    };
    let mut watched_dir: Option<PathBuf> = None;
    let mut dir_notify: Option<HANDLE> = None;

    loop {
        let w = watch.lock().unwrap().clone();
        if !w.active || w.channel.trim().is_empty() {
            if state.file_path.is_some() || !state.channel.is_empty() {
                state.reset(String::new(), PathBuf::new());
            }
            close_dir_notify(&mut dir_notify, &mut watched_dir);
            unsafe {
                WaitForMultipleObjects(&[wake], false, INFINITE);
            }
            continue;
        }

        if state.channel != w.channel || state.dir != w.dir {
            state.reset(w.channel.clone(), w.dir.clone());
            close_dir_notify(&mut dir_notify, &mut watched_dir);
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
            if state.file_path.is_some() {
                ACTIVE_FALLBACK_MS
            } else {
                INFINITE
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

fn poll_chatlogs(sink: &EventSink, st: &mut TailState) {
    if !st.dir.is_dir() {
        if st.file_path.is_some() || st.filename.is_some() {
            st.file_path = None;
            st.filename = None;
        }
        sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
            filename: None,
            listener: None,
            history: Vec::new(),
            error: Some(format!("Folder not found: {}", st.dir.display())),
        }));
        return;
    }

    let entries = match fs::read_dir(&st.dir) {
        Ok(rd) => rd,
        Err(e) => {
            sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
                filename: None,
                listener: None,
                history: Vec::new(),
                error: Some(format!("Cannot read folder: {e}")),
            }));
            return;
        }
    };

    let names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let picked = eve_intel::pick_latest_session(names.iter().map(String::as_str), &st.channel);
    let Some(session) = picked else {
        if st.file_path.is_some() || st.filename.is_none() {
            st.file_path = None;
            st.filename = None;
            sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
                filename: None,
                listener: None,
                history: Vec::new(),
                error: Some(format!("Waiting for \"{}_*.txt\"", st.channel.trim())),
            }));
        }
        return;
    };

    let switched = st.filename.as_deref() != Some(&session.filename);
    if switched {
        let path = st.dir.join(&session.filename);
        let (listener, history, offset, carry) = read_initial_tail(&path);
        st.file_path = Some(path);
        st.filename = Some(session.filename.clone());
        st.offset = offset;
        st.carry = carry;
        st.pending.clear();
        sink.send(AppEvent::EveIntel(EveIntelEvent::Session {
            filename: Some(session.filename),
            listener,
            history,
            error: None,
        }));
        return;
    }

    let Some(path) = st.file_path.as_ref() else { return };
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

    let mut msgs = Vec::new();
    while let Some(pos) = st.pending.find('\n') {
        let line: String = st.pending.drain(..=pos).collect();
        if let Some(m) = eve_intel::parse_line(&line) {
            msgs.push(m);
        }
    }
    if !msgs.is_empty() {
        sink.send(AppEvent::EveIntel(EveIntelEvent::Messages(msgs)));
    }
}

fn read_initial_tail(path: &Path) -> (Option<String>, Vec<ChatMessage>, u64, Option<u8>) {
    let Ok(mut file) = File::open(path) else {
        return (None, Vec::new(), 0, None);
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    // Read header (first 2 KB) for Listener metadata.
    let mut head_buf = vec![0u8; (len.min(2048)) as usize];
    let head_read = file.read(&mut head_buf).unwrap_or(0);
    let mut head_carry = None;
    let head_text = eve_intel::decode_utf16le_chunk(&head_buf[..head_read], &mut head_carry);
    let listener = eve_intel::parse_header(&head_text).and_then(|h| (!h.listener.is_empty()).then_some(h.listener));

    // Read up to last 16 KB (aligned to even byte boundary) for recent feed lines.
    let start = len.saturating_sub(INITIAL_HISTORY_BYTES) & !1;
    if file.seek(SeekFrom::Start(start)).is_err() {
        return (listener, Vec::new(), len, None);
    }
    let mut tail = Vec::new();
    let read_len = file.read_to_end(&mut tail).unwrap_or(0);
    let mut carry = None;
    let text = eve_intel::decode_utf16le_chunk(&tail[..read_len], &mut carry);
    let lines: Vec<&str> = text.lines().collect();
    let skip_first = usize::from(start > 0 && !lines.is_empty());
    let mut history: Vec<ChatMessage> = lines.into_iter().skip(skip_first).filter_map(eve_intel::parse_line).filter(|m| !m.is_system).collect();
    if history.len() > 24 {
        history.drain(..history.len() - 24);
    }
    (listener, history, start + read_len as u64, carry)
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
