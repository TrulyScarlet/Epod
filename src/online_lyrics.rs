use crate::audio::player::AudioPlayer;
use crate::library::model::Library;
use crate::state::AppState;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_CACHE_ENTRIES: usize = 1000;
const TARGET_CACHE_ENTRIES: usize = 800;
const HTTP_TIMEOUT_SECS: u64 = 5;
const MAX_BODY_BYTES: u64 = 256 * 1024; // 256 KB cap
const MAX_TRANSIENT_RETRIES: u32 = 3;

/// High-level status of the online lyrics backend, suitable for UI / designer display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OnlineLyricsStatus {
    Disabled,
    Idle,
    Fetching,
    Ready,
    NotFound,
    Error,
}

impl OnlineLyricsStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            OnlineLyricsStatus::Disabled => "Disabled",
            OnlineLyricsStatus::Idle => "Idle",
            OnlineLyricsStatus::Fetching => "Fetching…",
            OnlineLyricsStatus::Ready => "Lyrics Available",
            OnlineLyricsStatus::NotFound => "Lyrics Not Found",
            OnlineLyricsStatus::Error => "Network Error",
        }
    }
}

/// Strict identity key for track requests and cache indexing.
/// Uses full metadata + file path, never unstable song indexes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TrackKey {
    pub file_path: Option<PathBuf>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_sec: u32,
}

impl TrackKey {
    pub fn cache_filename(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.file_path.hash(&mut hasher);
        self.title.hash(&mut hasher);
        self.artist.hash(&mut hasher);
        self.album.hash(&mut hasher);
        self.duration_sec.hash(&mut hasher);
        format!("{:016x}.json", hasher.finish())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CacheOutcome {
    Found { lyrics: String, is_synced: bool },
    NotFound,
    Instrumental,
    Mismatch,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheRecord {
    pub key: TrackKey,
    pub outcome: CacheOutcome,
    pub cached_at_epoch_sec: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TransportResponse {
    Success(String),
    NotFound,
    RateLimited { retry_after_secs: Option<u64> },
    ServerError(u16),
    ClientError(u16),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TransportError {
    Network(String),
    Timeout,
    Other(String),
}

pub trait LyricsTransport: Send + Sync + 'static {
    fn fetch(
        &self,
        track_name: &str,
        artist_name: &str,
        album_name: &str,
        duration_sec: u32,
    ) -> Result<TransportResponse, TransportError>;
}

pub struct UreqLyricsTransport {
    user_agent: String,
}

impl Default for UreqLyricsTransport {
    fn default() -> Self {
        Self {
            user_agent: format!(
                "Epod/{} (https://github.com/TrulyScarlet/Epod)",
                env!("CARGO_PKG_VERSION")
            ),
        }
    }
}

impl LyricsTransport for UreqLyricsTransport {
    fn fetch(
        &self,
        track_name: &str,
        artist_name: &str,
        album_name: &str,
        duration_sec: u32,
    ) -> Result<TransportResponse, TransportError> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .timeout_read(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .user_agent(&self.user_agent)
            .build();

        let response = agent
            .get("https://lrclib.net/api/get")
            .query("track_name", track_name)
            .query("artist_name", artist_name)
            .query("album_name", album_name)
            .query("duration", &duration_sec.to_string())
            .call();

        match response {
            Ok(resp) => {
                let mut reader = resp.into_reader().take(MAX_BODY_BYTES + 1);
                let mut body = String::new();
                if let Err(e) = reader.read_to_string(&mut body) {
                    return Err(TransportError::Network(e.to_string()));
                }
                if body.len() as u64 > MAX_BODY_BYTES {
                    return Err(TransportError::Other(format!(
                        "Response exceeded maximum body size limit of {} bytes",
                        MAX_BODY_BYTES
                    )));
                }
                Ok(TransportResponse::Success(body))
            }
            Err(ureq::Error::Status(404, _)) => Ok(TransportResponse::NotFound),
            Err(ureq::Error::Status(429, resp)) => {
                let retry_after = resp
                    .header("Retry-After")
                    .and_then(|h| h.trim().parse::<u64>().ok());
                Ok(TransportResponse::RateLimited {
                    retry_after_secs: retry_after,
                })
            }
            Err(ureq::Error::Status(code, _)) if (500..=599).contains(&code) => {
                Ok(TransportResponse::ServerError(code))
            }
            Err(ureq::Error::Status(code, _)) => Ok(TransportResponse::ClientError(code)),
            Err(ureq::Error::Transport(t)) => Err(TransportError::Network(t.to_string())),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LrclibResponse {
    pub track_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: Option<bool>,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

pub fn validate_and_extract_lyrics(body: &str, key: &TrackKey) -> Result<CacheOutcome, String> {
    let resp: LrclibResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;

    if resp.instrumental == Some(true) {
        return Ok(CacheOutcome::Instrumental);
    }

    // Require actual trackName, artistName, and duration to be present before accepting
    let Some(ref tn) = resp.track_name else {
        return Ok(CacheOutcome::Mismatch);
    };
    let Some(ref an) = resp.artist_name else {
        return Ok(CacheOutcome::Mismatch);
    };
    let Some(dur) = resp.duration else {
        return Ok(CacheOutcome::Mismatch);
    };

    // Conservative exact duration check: within 2.0 seconds
    if (dur - key.duration_sec as f64).abs() > 2.0 {
        return Ok(CacheOutcome::Mismatch);
    }

    // Exact track name and artist name match
    if !tn.trim().eq_ignore_ascii_case(key.title.trim()) {
        return Ok(CacheOutcome::Mismatch);
    }
    if !an.trim().eq_ignore_ascii_case(key.artist.trim()) {
        return Ok(CacheOutcome::Mismatch);
    }

    // If response provides albumName and request album is not empty / Unknown Album, require exact match
    let req_album = key.album.trim();
    if !req_album.is_empty() && !req_album.eq_ignore_ascii_case("Unknown Album") {
        if let Some(ref resp_album) = resp.album_name.as_ref().filter(|s| !s.trim().is_empty()) {
            if !resp_album.trim().eq_ignore_ascii_case(req_album) {
                return Ok(CacheOutcome::Mismatch);
            }
        }
    }

    // Prefer synced lyrics, fall back to plain lyrics
    if let Some(synced) = resp.synced_lyrics.filter(|s| !s.trim().is_empty()) {
        return Ok(CacheOutcome::Found {
            lyrics: synced,
            is_synced: true,
        });
    }

    if let Some(plain) = resp.plain_lyrics.filter(|s| !s.trim().is_empty()) {
        return Ok(CacheOutcome::Found {
            lyrics: plain,
            is_synced: false,
        });
    }

    Ok(CacheOutcome::NotFound)
}

pub fn default_cache_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app_data)
                .join("epod")
                .join("lyrics_cache");
        }
    }
    if let Ok(cache_home) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home).join("epod").join("lyrics_cache");
    }
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        return PathBuf::from(home)
            .join(".cache")
            .join("epod")
            .join("lyrics_cache");
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.join(".epod_lyrics_cache");
        }
    }
    PathBuf::from(".epod_lyrics_cache")
}

pub const NEGATIVE_CACHE_TTL_SECS: u64 = 6 * 3600; // 6 hours max

pub fn read_from_cache(cache_dir: &Path, key: &TrackKey) -> Option<CacheOutcome> {
    let file_path = cache_dir.join(key.cache_filename());
    let bytes = std::fs::read(&file_path).ok()?;
    let record: CacheRecord = serde_json::from_slice(&bytes).ok()?;
    if &record.key == key {
        let is_negative = matches!(
            record.outcome,
            CacheOutcome::NotFound | CacheOutcome::Instrumental | CacheOutcome::Mismatch
        );
        if is_negative {
            let now_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if now_secs.saturating_sub(record.cached_at_epoch_sec) > NEGATIVE_CACHE_TTL_SECS {
                let _ = std::fs::remove_file(&file_path);
                return None;
            }
        }
        Some(record.outcome)
    } else {
        None
    }
}

pub fn write_to_cache(cache_dir: &Path, key: &TrackKey, outcome: &CacheOutcome) {
    let _ = std::fs::create_dir_all(cache_dir);
    let file_path = cache_dir.join(key.cache_filename());
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let record = CacheRecord {
        key: key.clone(),
        outcome: outcome.clone(),
        cached_at_epoch_sec: now_secs,
    };
    if let Ok(bytes) = serde_json::to_vec(&record) {
        let _ = std::fs::write(&file_path, bytes);
    }
    prune_cache_if_needed(cache_dir, MAX_CACHE_ENTRIES, TARGET_CACHE_ENTRIES);
}

pub fn prune_cache_if_needed(cache_dir: &Path, max_entries: usize, target_entries: usize) {
    let read_dir = match std::fs::read_dir(cache_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    let mut files = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH);
            files.push((path, modified));
        }
    }
    if files.len() > max_entries {
        files.sort_by_key(|(_, m)| *m);
        let to_remove = files.len().saturating_sub(target_entries);
        for (path, _) in files.into_iter().take(to_remove) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Worker-local cancellation token used to invalidate queued/in-flight jobs across track changes
/// or disable/re-enable toggles.
#[derive(Clone)]
pub struct CancellationToken(Arc<AtomicBool>);

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }

    pub fn cancel(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        !self.0.load(Ordering::Relaxed)
    }
}

enum WorkerCommand {
    Fetch {
        key: TrackKey,
        song_id: usize,
        token: CancellationToken,
        enabled_flag: Arc<AtomicBool>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum WorkerOutcome {
    Found {
        lyrics: String,
        #[allow(dead_code)]
        is_synced: bool,
    },
    NotFound,
    Instrumental,
    Mismatch,
    TransientError,
    DeterministicError(#[allow(dead_code)] u16),
}

pub struct WorkerResponse {
    pub key: TrackKey,
    pub song_id: usize,
    pub outcome: WorkerOutcome,
}

struct WorkerState {
    tx: Sender<WorkerCommand>,
    resp_rx: Receiver<WorkerResponse>,
    _thread: std::thread::JoinHandle<()>,
}

pub struct OnlineLyricsService {
    worker: Option<WorkerState>,
    cache_dir: PathBuf,
    active_key: Option<TrackKey>,
    current_token: Option<CancellationToken>,
    transport: Arc<dyn LyricsTransport>,
    last_enabled: bool,
    retry_count: u32,
    next_retry_time: Option<Instant>,
}

impl Default for OnlineLyricsService {
    fn default() -> Self {
        Self::new()
    }
}

impl OnlineLyricsService {
    pub fn new() -> Self {
        Self::with_transport_and_cache(
            Arc::new(UreqLyricsTransport::default()),
            default_cache_dir(),
        )
    }

    pub fn with_transport_and_cache(
        transport: Arc<dyn LyricsTransport>,
        cache_dir: PathBuf,
    ) -> Self {
        Self {
            worker: None,
            cache_dir,
            active_key: None,
            current_token: None,
            transport,
            last_enabled: false,
            retry_count: 0,
            next_retry_time: None,
        }
    }

    fn ensure_worker(&mut self) -> &mut WorkerState {
        if self.worker.is_none() {
            let (cmd_tx, cmd_rx) = channel::<WorkerCommand>();
            let (resp_tx, resp_rx) = channel::<WorkerResponse>();
            let transport = Arc::clone(&self.transport);
            let cache_dir = self.cache_dir.clone();

            let handle = std::thread::spawn(move || {
                let mut backoff_until: Option<Instant> = None;

                while let Ok(mut cmd) = cmd_rx.recv() {
                    // Coalesce commands: if newer fetch commands arrived while worker was busy,
                    // advance immediately to the latest command to avoid backlog.
                    while let Ok(newer_cmd) = cmd_rx.try_recv() {
                        cmd = newer_cmd;
                    }

                    match cmd {
                        WorkerCommand::Shutdown => break,
                        WorkerCommand::Fetch {
                            key,
                            song_id,
                            token,
                            enabled_flag,
                        } => {
                            // Gate check 1: before disk cache
                            if token.is_cancelled() || !enabled_flag.load(Ordering::Relaxed) {
                                continue;
                            }

                            // 1. Check disk cache
                            if let Some(cached) = read_from_cache(&cache_dir, &key) {
                                if token.is_cancelled() || !enabled_flag.load(Ordering::Relaxed) {
                                    continue;
                                }
                                let outcome = match cached {
                                    CacheOutcome::Found { lyrics, is_synced } => {
                                        WorkerOutcome::Found { lyrics, is_synced }
                                    }
                                    CacheOutcome::NotFound => WorkerOutcome::NotFound,
                                    CacheOutcome::Instrumental => WorkerOutcome::Instrumental,
                                    CacheOutcome::Mismatch => WorkerOutcome::Mismatch,
                                };
                                let _ = resp_tx.send(WorkerResponse {
                                    key,
                                    song_id,
                                    outcome,
                                });
                                continue;
                            }

                            // 2. Check transient backoff timer
                            let now = Instant::now();
                            if let Some(until) = backoff_until {
                                if now < until {
                                    let _ = resp_tx.send(WorkerResponse {
                                        key,
                                        song_id,
                                        outcome: WorkerOutcome::TransientError,
                                    });
                                    continue;
                                }
                            }

                            // Gate check 2: recheck before making HTTP network call
                            if token.is_cancelled() || !enabled_flag.load(Ordering::Relaxed) {
                                continue;
                            }

                            // 3. Make HTTP request via transport off UI thread
                            let (worker_outcome, cache_outcome) = match transport.fetch(
                                &key.title,
                                &key.artist,
                                &key.album,
                                key.duration_sec,
                            ) {
                                Ok(TransportResponse::Success(body)) => {
                                    match validate_and_extract_lyrics(&body, &key) {
                                        Ok(outcome) => {
                                            let w_out = match &outcome {
                                                CacheOutcome::Found { lyrics, is_synced } => {
                                                    WorkerOutcome::Found {
                                                        lyrics: lyrics.clone(),
                                                        is_synced: *is_synced,
                                                    }
                                                }
                                                CacheOutcome::NotFound => WorkerOutcome::NotFound,
                                                CacheOutcome::Instrumental => {
                                                    WorkerOutcome::Instrumental
                                                }
                                                CacheOutcome::Mismatch => WorkerOutcome::Mismatch,
                                            };
                                            (w_out, Some(outcome))
                                        }
                                        Err(_) => (WorkerOutcome::TransientError, None),
                                    }
                                }
                                Ok(TransportResponse::NotFound) => {
                                    (WorkerOutcome::NotFound, Some(CacheOutcome::NotFound))
                                }
                                Ok(TransportResponse::RateLimited { retry_after_secs }) => {
                                    let wait_secs = retry_after_secs.unwrap_or(15).max(1);
                                    backoff_until =
                                        Some(Instant::now() + Duration::from_secs(wait_secs));
                                    (WorkerOutcome::TransientError, None)
                                }
                                Ok(TransportResponse::ServerError(_)) => {
                                    backoff_until = Some(Instant::now() + Duration::from_secs(5));
                                    (WorkerOutcome::TransientError, None)
                                }
                                Ok(TransportResponse::ClientError(code)) => {
                                    // 4xx other than 404/429 is deterministic error; no backoff, no retry
                                    (WorkerOutcome::DeterministicError(code), None)
                                }
                                Err(_) => {
                                    backoff_until =
                                        Some(Instant::now() + Duration::from_millis(500));
                                    (WorkerOutcome::TransientError, None)
                                }
                            };

                            // Gate check 3: recheck before writing cache to disk
                            if token.is_cancelled() || !enabled_flag.load(Ordering::Relaxed) {
                                continue;
                            }

                            // 4. Bounded disk cache storage (positive and negative responses)
                            if let Some(ref c_out) = cache_outcome {
                                write_to_cache(&cache_dir, &key, c_out);
                            }

                            // Gate check 4: recheck before transmitting response to main thread
                            if token.is_cancelled() || !enabled_flag.load(Ordering::Relaxed) {
                                continue;
                            }

                            let _ = resp_tx.send(WorkerResponse {
                                key,
                                song_id,
                                outcome: worker_outcome,
                            });
                        }
                    }
                }
            });

            self.worker = Some(WorkerState {
                tx: cmd_tx,
                resp_rx,
                _thread: handle,
            });
        }
        self.worker.as_mut().unwrap()
    }

    pub fn shutdown_worker(&mut self) {
        if let Some(token) = self.current_token.take() {
            token.cancel();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.tx.send(WorkerCommand::Shutdown);
        }
        self.active_key = None;
        self.retry_count = 0;
        self.next_retry_time = None;
    }

    fn drain_responses(&mut self, state: &mut AppState, library: &mut Library) {
        if let Some(ref worker) = self.worker {
            loop {
                match worker.resp_rx.try_recv() {
                    Ok(resp) => {
                        // Stale check 1: Still enabled?
                        if !state.online_lyrics_enabled {
                            continue;
                        }

                        // Stale check 2: Matches current track key?
                        if self.active_key.as_ref() != Some(&resp.key) {
                            continue;
                        }

                        // Stale check 3: Song identity matches library entry?
                        let Some(song) = library.songs.get_mut(resp.song_id) else {
                            continue;
                        };
                        if song.file_path != resp.key.file_path
                            || !song.title.eq_ignore_ascii_case(&resp.key.title)
                            || !song.artist.eq_ignore_ascii_case(&resp.key.artist)
                        {
                            continue;
                        }

                        // Stale check 4: Has local lyrics arrived in the meantime?
                        if song.lyrics.as_ref().is_some_and(|l| !l.trim().is_empty()) {
                            state.online_lyrics_status = OnlineLyricsStatus::Ready;
                            continue;
                        }

                        // Apply outcome
                        match resp.outcome {
                            WorkerOutcome::Found { lyrics, .. } => {
                                self.retry_count = 0;
                                self.next_retry_time = None;
                                song.parsed_lyrics =
                                    crate::audio::metadata::parse_lrc_or_plain_lyrics(&lyrics);
                                song.lyrics = Some(lyrics);
                                state.online_lyrics_status = OnlineLyricsStatus::Ready;
                            }
                            WorkerOutcome::NotFound
                            | WorkerOutcome::Instrumental
                            | WorkerOutcome::Mismatch => {
                                self.retry_count = 0;
                                self.next_retry_time = None;
                                state.online_lyrics_status = OnlineLyricsStatus::NotFound;
                            }
                            WorkerOutcome::TransientError => {
                                state.online_lyrics_status = OnlineLyricsStatus::Error;
                                if self.retry_count < MAX_TRANSIENT_RETRIES {
                                    let delay_secs = 2 * (self.retry_count as u64 + 1);
                                    self.next_retry_time =
                                        Some(Instant::now() + Duration::from_secs(delay_secs));
                                } else {
                                    self.next_retry_time = None;
                                }
                            }
                            WorkerOutcome::DeterministicError(_) => {
                                self.retry_count = 0;
                                self.next_retry_time = None;
                                state.online_lyrics_status = OnlineLyricsStatus::Error;
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }
    }

    /// Central nonblocking current-track lyrics poll called on every frame.
    /// Lazy worker: only runs when online lyrics are enabled and current track lacks local lyrics.
    pub fn poll(&mut self, state: &mut AppState, library: &mut Library, _player: &AudioPlayer) {
        // Synchronize state flag
        if state.online_lyrics_enabled != self.last_enabled {
            self.last_enabled = state.online_lyrics_enabled;
            state
                .online_lyrics_enabled_flag
                .store(state.online_lyrics_enabled, Ordering::Relaxed);
            if !state.online_lyrics_enabled {
                self.shutdown_worker();
                state.online_lyrics_status = OnlineLyricsStatus::Disabled;
                return;
            }
        }

        // Hard gate: zero new LRCLIB work when disabled
        if !state.online_lyrics_enabled {
            if self.worker.is_some() {
                self.shutdown_worker();
            }
            if self.active_key.is_some() {
                self.active_key = None;
                if let Some(tok) = self.current_token.take() {
                    tok.cancel();
                }
            }
            state.online_lyrics_status = OnlineLyricsStatus::Disabled;
            return;
        }

        // Determine currently playing song from the queue
        let current_song_id = match state.current_queue.get(state.current_queue_idx) {
            Some(&id) => id,
            None => {
                if self.active_key.is_some() {
                    self.active_key = None;
                    if let Some(tok) = self.current_token.take() {
                        tok.cancel();
                    }
                    self.retry_count = 0;
                    self.next_retry_time = None;
                }
                state.online_lyrics_status = OnlineLyricsStatus::Idle;
                self.drain_responses(state, library);
                return;
            }
        };

        let current_song = match library.songs.get(current_song_id) {
            Some(s) => s,
            None => {
                self.active_key = None;
                return;
            }
        };

        // Real track requirement (no synthetic demo tracks, real audio file path required)
        if current_song.is_synthetic_demo || current_song.file_path.is_none() {
            if self.active_key.is_some() {
                self.active_key = None;
                if let Some(tok) = self.current_token.take() {
                    tok.cancel();
                }
                self.retry_count = 0;
                self.next_retry_time = None;
            }
            state.online_lyrics_status = OnlineLyricsStatus::Idle;
            self.drain_responses(state, library);
            return;
        }

        let key = TrackKey {
            file_path: current_song.file_path.clone(),
            title: current_song.title.clone(),
            artist: current_song.artist.clone(),
            album: current_song.album.clone(),
            duration_sec: current_song.duration_sec.round() as u32,
        };

        // Local / embedded lyrics takes precedence: never overwrite local lyrics
        if let Some(ref lyrics) = current_song.lyrics {
            if !lyrics.trim().is_empty() {
                if self.active_key.as_ref() != Some(&key) {
                    self.active_key = Some(key);
                    if let Some(tok) = self.current_token.take() {
                        tok.cancel();
                    }
                    self.retry_count = 0;
                    self.next_retry_time = None;
                }
                state.online_lyrics_status = OnlineLyricsStatus::Ready;
                self.drain_responses(state, library);
                return;
            }
        }

        // Require valid track title and artist
        if current_song.title.trim().is_empty()
            || current_song.artist.trim().is_empty()
            || current_song.artist.eq_ignore_ascii_case("Unknown Artist")
        {
            if self.active_key.as_ref() != Some(&key) {
                self.active_key = Some(key);
                if let Some(tok) = self.current_token.take() {
                    tok.cancel();
                }
                self.retry_count = 0;
                self.next_retry_time = None;
            }
            state.online_lyrics_status = OnlineLyricsStatus::NotFound;
            self.drain_responses(state, library);
            return;
        }

        // If track changed or uninitialized, submit fresh worker request with new cancellation token
        if self.active_key.as_ref() != Some(&key) {
            self.active_key = Some(key.clone());
            if let Some(tok) = self.current_token.take() {
                tok.cancel();
            }
            self.retry_count = 0;
            self.next_retry_time = None;

            let token = CancellationToken::new();
            self.current_token = Some(token.clone());
            let enabled_flag = state.online_lyrics_enabled_flag.clone();
            let worker = self.ensure_worker();
            let _ = worker.tx.send(WorkerCommand::Fetch {
                key: key.clone(),
                song_id: current_song_id,
                token,
                enabled_flag,
            });
            state.online_lyrics_status = OnlineLyricsStatus::Fetching;
        } else if state.online_lyrics_status == OnlineLyricsStatus::Error {
            // Bounded timed retry for same active_key on transient error without busy-loop
            if let Some(retry_at) = self.next_retry_time {
                if Instant::now() >= retry_at && self.retry_count < MAX_TRANSIENT_RETRIES {
                    self.retry_count += 1;
                    self.next_retry_time = None;

                    let token = CancellationToken::new();
                    self.current_token = Some(token.clone());
                    let enabled_flag = state.online_lyrics_enabled_flag.clone();
                    let worker = self.ensure_worker();
                    let _ = worker.tx.send(WorkerCommand::Fetch {
                        key: key.clone(),
                        song_id: current_song_id,
                        token,
                        enabled_flag,
                    });
                    state.online_lyrics_status = OnlineLyricsStatus::Fetching;
                }
            }
        }

        // Drain responses from the worker off-thread channel
        self.drain_responses(state, library);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::model::Song;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    struct MockTransport {
        call_count: AtomicUsize,
        response_builder: Mutex<
            Box<
                dyn Fn(&str, &str, &str, u32) -> Result<TransportResponse, TransportError>
                    + Send
                    + Sync,
            >,
        >,
    }

    impl MockTransport {
        fn new<F>(f: F) -> Self
        where
            F: Fn(&str, &str, &str, u32) -> Result<TransportResponse, TransportError>
                + Send
                + Sync
                + 'static,
        {
            Self {
                call_count: AtomicUsize::new(0),
                response_builder: Mutex::new(Box::new(f)),
            }
        }
    }

    impl LyricsTransport for MockTransport {
        fn fetch(
            &self,
            track_name: &str,
            artist_name: &str,
            album_name: &str,
            duration_sec: u32,
        ) -> Result<TransportResponse, TransportError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let builder = self.response_builder.lock().unwrap();
            builder(track_name, artist_name, album_name, duration_sec)
        }
    }

    fn create_test_song(id: usize, title: &str, artist: &str, album: &str, duration: f32) -> Song {
        Song {
            id,
            file_path: Some(PathBuf::from(format!("/music/{}_{}.mp3", artist, title))),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: "Rock".to_string(),
            composer: "Composer".to_string(),
            year: Some(2024),
            track_number: Some(1),
            duration_sec: duration,
            bitrate_kbps: Some(320),
            codec: Some("mp3".to_string()),
            rating: 0,
            play_count: 0,
            artwork_bytes: None,
            lyrics: None,
            parsed_lyrics: Vec::new(),
            is_synthetic_demo: false,
        }
    }

    #[test]
    fn test_disabled_zero_calls() {
        let temp_dir = tempfile::tempdir().unwrap();
        let transport = Arc::new(MockTransport::new(|_, _, _, _| {
            Ok(TransportResponse::Success("{}".to_string()))
        }));

        let mut service = OnlineLyricsService::with_transport_and_cache(
            transport.clone(),
            temp_dir.path().to_path_buf(),
        );

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(false);

        let mut library = Library::new();
        library.songs = vec![create_test_song(0, "Song A", "Artist A", "Album A", 180.0)];
        state.current_queue = vec![0];
        state.current_queue_idx = 0;

        let player = AudioPlayer::new();

        // Run poll multiple times
        for _ in 0..5 {
            service.poll(&mut state, &mut library, &player);
        }

        assert_eq!(transport.call_count.load(Ordering::SeqCst), 0);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Disabled);
        assert!(library.songs[0].lyrics.is_none());
    }

    #[test]
    fn test_enabled_and_cache_reuse() {
        let temp_dir = tempfile::tempdir().unwrap();
        let json_body = serde_json::json!({
            "trackName": "Song A",
            "artistName": "Artist A",
            "albumName": "Album A",
            "duration": 180.0,
            "instrumental": false,
            "syncedLyrics": "[00:10.00]Hello world",
            "plainLyrics": "Hello world"
        })
        .to_string();

        let transport = Arc::new(MockTransport::new(move |_, _, _, _| {
            Ok(TransportResponse::Success(json_body.clone()))
        }));

        let mut service = OnlineLyricsService::with_transport_and_cache(
            transport.clone(),
            temp_dir.path().to_path_buf(),
        );

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let mut library = Library::new();
        library.songs = vec![create_test_song(0, "Song A", "Artist A", "Album A", 180.0)];
        state.current_queue = vec![0];
        state.current_queue_idx = 0;
        let player = AudioPlayer::new();

        // Poll until lyrics received from background worker
        for _ in 0..100 {
            service.poll(&mut state, &mut library, &player);
            if library.songs[0].lyrics.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(transport.call_count.load(Ordering::SeqCst), 1);
        assert_eq!(
            library.songs[0].lyrics.as_deref(),
            Some("[00:10.00]Hello world")
        );
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Ready);

        // Now simulate a fresh service session on the same track with cleared song memory
        library.songs[0].lyrics = None;
        library.songs[0].parsed_lyrics.clear();

        let mut second_service = OnlineLyricsService::with_transport_and_cache(
            transport.clone(),
            temp_dir.path().to_path_buf(),
        );

        for _ in 0..100 {
            second_service.poll(&mut state, &mut library, &player);
            if library.songs[0].lyrics.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        // Call count MUST remain 1 because it hit the disk cache!
        assert_eq!(transport.call_count.load(Ordering::SeqCst), 1);
        assert_eq!(
            library.songs[0].lyrics.as_deref(),
            Some("[00:10.00]Hello world")
        );
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Ready);
    }

    #[test]
    fn test_synced_preferred_over_plain() {
        let key = TrackKey {
            file_path: Some(PathBuf::from("test.mp3")),
            title: "Test".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_sec: 200,
        };

        let body = serde_json::json!({
            "trackName": "Test",
            "artistName": "Artist",
            "duration": 200.0,
            "instrumental": false,
            "syncedLyrics": "[00:01.00]Synced lyric",
            "plainLyrics": "Plain lyric"
        })
        .to_string();

        let outcome = validate_and_extract_lyrics(&body, &key).unwrap();
        assert_eq!(
            outcome,
            CacheOutcome::Found {
                lyrics: "[00:01.00]Synced lyric".to_string(),
                is_synced: true,
            }
        );
    }

    #[test]
    fn test_plain_fallback_when_no_synced() {
        let key = TrackKey {
            file_path: Some(PathBuf::from("test.mp3")),
            title: "Test".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_sec: 200,
        };

        let body = serde_json::json!({
            "trackName": "Test",
            "artistName": "Artist",
            "duration": 200.0,
            "instrumental": false,
            "syncedLyrics": null,
            "plainLyrics": "Plain lyric line 1\nPlain lyric line 2"
        })
        .to_string();

        let outcome = validate_and_extract_lyrics(&body, &key).unwrap();
        assert_eq!(
            outcome,
            CacheOutcome::Found {
                lyrics: "Plain lyric line 1\nPlain lyric line 2".to_string(),
                is_synced: false,
            }
        );
    }

    #[test]
    fn test_instrumental_and_404_outcomes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let key = TrackKey {
            file_path: Some(PathBuf::from("test.mp3")),
            title: "Instrumental Song".to_string(),
            artist: "Band".to_string(),
            album: "Album".to_string(),
            duration_sec: 150,
        };

        let inst_body = serde_json::json!({
            "trackName": "Instrumental Song",
            "artistName": "Band",
            "duration": 150.0,
            "instrumental": true,
            "syncedLyrics": null,
            "plainLyrics": null
        })
        .to_string();

        let inst_outcome = validate_and_extract_lyrics(&inst_body, &key).unwrap();
        assert_eq!(inst_outcome, CacheOutcome::Instrumental);

        write_to_cache(temp_dir.path(), &key, &inst_outcome);
        let cached = read_from_cache(temp_dir.path(), &key);
        assert_eq!(cached, Some(CacheOutcome::Instrumental));

        // 404 Not Found outcome
        let not_found_key = TrackKey {
            file_path: Some(PathBuf::from("404.mp3")),
            title: "Unknown".to_string(),
            artist: "Nobody".to_string(),
            album: "None".to_string(),
            duration_sec: 120,
        };
        write_to_cache(temp_dir.path(), &not_found_key, &CacheOutcome::NotFound);
        assert_eq!(
            read_from_cache(temp_dir.path(), &not_found_key),
            Some(CacheOutcome::NotFound)
        );
    }

    #[test]
    fn test_duration_and_name_mismatch_rejected() {
        let key = TrackKey {
            file_path: Some(PathBuf::from("test.mp3")),
            title: "Real Title".to_string(),
            artist: "Real Artist".to_string(),
            album: "Album".to_string(),
            duration_sec: 180,
        };

        // Duration difference > 2.0s
        let dur_mismatch = serde_json::json!({
            "trackName": "Real Title",
            "artistName": "Real Artist",
            "duration": 185.0, // 5 seconds diff
            "syncedLyrics": "Lyrics"
        })
        .to_string();
        assert_eq!(
            validate_and_extract_lyrics(&dur_mismatch, &key).unwrap(),
            CacheOutcome::Mismatch
        );

        // Name mismatch
        let name_mismatch = serde_json::json!({
            "trackName": "Wrong Title",
            "artistName": "Real Artist",
            "duration": 180.0,
            "syncedLyrics": "Lyrics"
        })
        .to_string();
        assert_eq!(
            validate_and_extract_lyrics(&name_mismatch, &key).unwrap(),
            CacheOutcome::Mismatch
        );
    }

    #[test]
    fn test_stale_result_protection() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Transport that delays Song A response
        let transport = Arc::new(MockTransport::new(|title, _, _, _| {
            if title == "Song A" {
                std::thread::sleep(Duration::from_millis(50));
                let body = serde_json::json!({
                    "trackName": "Song A",
                    "artistName": "Artist A",
                    "duration": 180.0,
                    "syncedLyrics": "[00:05.00]Song A Lyrics"
                })
                .to_string();
                Ok(TransportResponse::Success(body))
            } else {
                let body = serde_json::json!({
                    "trackName": "Song B",
                    "artistName": "Artist B",
                    "duration": 200.0,
                    "syncedLyrics": "[00:05.00]Song B Lyrics"
                })
                .to_string();
                Ok(TransportResponse::Success(body))
            }
        }));

        let mut service =
            OnlineLyricsService::with_transport_and_cache(transport, temp_dir.path().to_path_buf());

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let mut library = Library::new();
        library.songs = vec![
            create_test_song(0, "Song A", "Artist A", "Album A", 180.0),
            create_test_song(1, "Song B", "Artist B", "Album B", 200.0),
        ];

        // Start Song A
        state.current_queue = vec![0, 1];
        state.current_queue_idx = 0;
        let player = AudioPlayer::new();

        service.poll(&mut state, &mut library, &player);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Fetching);

        // Immediately switch to Song B while Song A is in-flight
        state.current_queue_idx = 1;
        service.poll(&mut state, &mut library, &player);

        // Wait for worker to finish
        for _ in 0..100 {
            service.poll(&mut state, &mut library, &player);
            if library.songs[1].lyrics.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        // Song B should have Song B lyrics, NOT Song A lyrics!
        assert_eq!(
            library.songs[1].lyrics.as_deref(),
            Some("[00:05.00]Song B Lyrics")
        );
    }

    #[test]
    fn test_preserve_local_lyrics() {
        let temp_dir = tempfile::tempdir().unwrap();
        let transport = Arc::new(MockTransport::new(|_, _, _, _| {
            panic!("Transport must never be called when local lyrics exist!");
        }));

        let mut service =
            OnlineLyricsService::with_transport_and_cache(transport, temp_dir.path().to_path_buf());

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let mut song = create_test_song(0, "Song A", "Artist A", "Album A", 180.0);
        song.lyrics = Some("Local Embedded Lyrics".to_string());

        let mut library = Library::new();
        library.songs = vec![song];
        state.current_queue = vec![0];
        state.current_queue_idx = 0;
        let player = AudioPlayer::new();

        service.poll(&mut state, &mut library, &player);

        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Ready);
        assert_eq!(
            library.songs[0].lyrics.as_deref(),
            Some("Local Embedded Lyrics")
        );
    }

    #[test]
    fn test_bounded_disk_cache_pruning() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache_path = temp_dir.path();

        for i in 0..15 {
            let key = TrackKey {
                file_path: Some(PathBuf::from(format!("track_{}.mp3", i))),
                title: format!("Title {}", i),
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                duration_sec: 100 + i as u32,
            };
            write_to_cache(cache_path, &key, &CacheOutcome::NotFound);
            std::thread::sleep(Duration::from_millis(5));
        }

        // Prune down to 10 entries when max is 12
        prune_cache_if_needed(cache_path, 12, 10);

        let count = std::fs::read_dir(cache_path)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .count();

        assert_eq!(count, 10);
    }

    #[test]
    fn test_disabled_during_inflight() {
        let temp_dir = tempfile::tempdir().unwrap();
        let transport = Arc::new(MockTransport::new(|_, _, _, _| {
            std::thread::sleep(Duration::from_millis(30));
            let body = serde_json::json!({
                "trackName": "Song A",
                "artistName": "Artist A",
                "duration": 180.0,
                "syncedLyrics": "[00:01.00]Lyrics"
            })
            .to_string();
            Ok(TransportResponse::Success(body))
        }));

        let mut service =
            OnlineLyricsService::with_transport_and_cache(transport, temp_dir.path().to_path_buf());

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let mut library = Library::new();
        library.songs = vec![create_test_song(0, "Song A", "Artist A", "Album A", 180.0)];
        state.current_queue = vec![0];
        state.current_queue_idx = 0;
        let player = AudioPlayer::new();

        // Trigger in-flight fetch
        service.poll(&mut state, &mut library, &player);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Fetching);

        // User disables online lyrics while worker is running
        state.set_online_lyrics_enabled(false);
        service.poll(&mut state, &mut library, &player);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Disabled);

        // Wait for worker thread to finish
        std::thread::sleep(Duration::from_millis(50));
        service.poll(&mut state, &mut library, &player);

        // Lyrics must NOT be applied to the song
        assert!(library.songs[0].lyrics.is_none());
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Disabled);
    }

    #[test]
    fn test_disable_reenable_does_not_reactivate_old_worker_jobs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let executed_call = Arc::new(AtomicBool::new(false));
        let executed_clone = Arc::clone(&executed_call);

        let transport = Arc::new(MockTransport::new(move |_, _, _, _| {
            executed_clone.store(true, Ordering::SeqCst);
            let body = serde_json::json!({
                "trackName": "Song A",
                "artistName": "Artist A",
                "duration": 180.0,
                "syncedLyrics": "[00:01.00]Lyrics"
            })
            .to_string();
            Ok(TransportResponse::Success(body))
        }));

        let mut service =
            OnlineLyricsService::with_transport_and_cache(transport, temp_dir.path().to_path_buf());

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let mut library = Library::new();
        library.songs = vec![
            create_test_song(0, "Song A", "Artist A", "Album A", 180.0),
            create_test_song(1, "Song B", "Artist B", "Album B", 200.0),
        ];
        state.current_queue = vec![0, 1];
        state.current_queue_idx = 0;
        let player = AudioPlayer::new();

        // Start fetch on Song A
        service.poll(&mut state, &mut library, &player);

        // Disable immediately
        state.set_online_lyrics_enabled(false);
        service.poll(&mut state, &mut library, &player);

        // Re-enable immediately but switch to Song B with local lyrics
        state.set_online_lyrics_enabled(true);
        library.songs[1].lyrics = Some("Song B local lyrics".to_string());
        state.current_queue_idx = 1;
        service.poll(&mut state, &mut library, &player);

        std::thread::sleep(Duration::from_millis(50));
        service.poll(&mut state, &mut library, &player);

        // Old Song A job token was cancelled, so Song A must not have lyrics
        assert!(library.songs[0].lyrics.is_none());
        assert_eq!(
            library.songs[1].lyrics.as_deref(),
            Some("Song B local lyrics")
        );
    }

    #[test]
    fn test_a_to_b_local_to_a_transition() {
        let temp_dir = tempfile::tempdir().unwrap();
        let transport = Arc::new(MockTransport::new(|title, _, _, _| {
            let body = serde_json::json!({
                "trackName": title,
                "artistName": "Artist",
                "duration": 180.0,
                "syncedLyrics": format!("[00:01.00]Fetched lyrics for {}", title)
            })
            .to_string();
            Ok(TransportResponse::Success(body))
        }));

        let mut service = OnlineLyricsService::with_transport_and_cache(
            transport.clone(),
            temp_dir.path().to_path_buf(),
        );

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let song_a = create_test_song(0, "Song A", "Artist", "Album", 180.0);
        let mut song_b = create_test_song(1, "Song B", "Artist", "Album", 180.0);
        song_b.lyrics = Some("Local Lyrics for Song B".to_string());

        let mut library = Library::new();
        library.songs = vec![song_a, song_b];
        state.current_queue = vec![0, 1];
        state.current_queue_idx = 0; // Song A
        let player = AudioPlayer::new();

        // 1. Play Song A: worker starts
        service.poll(&mut state, &mut library, &player);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Fetching);

        // 2. Switch to Song B (has local lyrics) before Song A response completes
        state.current_queue_idx = 1;
        service.poll(&mut state, &mut library, &player);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Ready);

        // 3. Switch back to Song A (A -> B -> A)
        state.current_queue_idx = 0;
        service.poll(&mut state, &mut library, &player);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Fetching);

        // Wait for Song A to arrive
        for _ in 0..100 {
            service.poll(&mut state, &mut library, &player);
            if library.songs[0].lyrics.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(
            library.songs[0].lyrics.as_deref(),
            Some("[00:01.00]Fetched lyrics for Song A")
        );
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Ready);
    }

    #[test]
    fn test_transient_error_bounded_retry() {
        let temp_dir = tempfile::tempdir().unwrap();
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = Arc::clone(&attempts);

        let transport = Arc::new(MockTransport::new(move |_, _, _, _| {
            let count = attempts_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                // First call fails with transient error
                Err(TransportError::Network("Temporary glitch".to_string()))
            } else {
                // Subsequent call succeeds
                let body = serde_json::json!({
                    "trackName": "Song A",
                    "artistName": "Artist A",
                    "duration": 180.0,
                    "syncedLyrics": "[00:01.00]Recovered Lyrics"
                })
                .to_string();
                Ok(TransportResponse::Success(body))
            }
        }));

        let mut service = OnlineLyricsService::with_transport_and_cache(
            transport.clone(),
            temp_dir.path().to_path_buf(),
        );

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let mut library = Library::new();
        library.songs = vec![create_test_song(0, "Song A", "Artist A", "Album A", 180.0)];
        state.current_queue = vec![0];
        state.current_queue_idx = 0;
        let player = AudioPlayer::new();

        // 1. First poll initiates fetch
        service.poll(&mut state, &mut library, &player);

        // Wait for first response (error)
        for _ in 0..50 {
            service.poll(&mut state, &mut library, &player);
            if state.online_lyrics_status == OnlineLyricsStatus::Error {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Error);
        assert_eq!(transport.call_count.load(Ordering::SeqCst), 1);

        // 2. Poll immediately: retry timer is in the future, so no call made
        service.poll(&mut state, &mut library, &player);
        assert_eq!(transport.call_count.load(Ordering::SeqCst), 1);

        // 3. Fast-forward retry timer by overriding next_retry_time to past and wait for worker backoff window
        std::thread::sleep(Duration::from_millis(550));
        service.next_retry_time = Some(Instant::now() - Duration::from_secs(1));

        // 4. Poll triggers retry
        service.poll(&mut state, &mut library, &player);
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Fetching);

        // Wait for recovery response
        for _ in 0..50 {
            service.poll(&mut state, &mut library, &player);
            if library.songs[0].lyrics.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(transport.call_count.load(Ordering::SeqCst), 2);
        assert_eq!(
            library.songs[0].lyrics.as_deref(),
            Some("[00:01.00]Recovered Lyrics")
        );
        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Ready);
    }

    #[test]
    fn test_manual_repair_honors_gate() {
        let mut song = create_test_song(0, "Song A", "Artist A", "Album A", 180.0);
        song.lyrics = None;

        // Gate disabled: online_lyrics_enabled = false
        let flag = Arc::new(AtomicBool::new(false));
        let rx = crate::metadata_fetcher::start_metadata_repair(&[song], flag);

        // With lyrics disabled and no other missing metadata, 0 requests should be queued
        let res = rx.recv_timeout(Duration::from_millis(200));
        assert!(
            res.is_err(),
            "No repair result should be returned when only lyrics are missing and gate is disabled"
        );
    }

    #[test]
    fn test_response_with_both_name_and_track_name_parses() {
        let key = TrackKey {
            file_path: Some(PathBuf::from("test.mp3")),
            title: "Track Title".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_sec: 180,
        };

        let body = serde_json::json!({
            "id": 12345,
            "name": "Track Title",
            "trackName": "Track Title",
            "artistName": "Artist",
            "albumName": "Album",
            "duration": 180.0,
            "instrumental": false,
            "syncedLyrics": "[00:01.00]Valid lyrics",
            "plainLyrics": "Valid lyrics"
        })
        .to_string();

        let outcome = validate_and_extract_lyrics(&body, &key).expect("Should parse cleanly");
        assert_eq!(
            outcome,
            CacheOutcome::Found {
                lyrics: "[00:01.00]Valid lyrics".to_string(),
                is_synced: true,
            }
        );
    }

    #[test]
    fn test_negative_cache_expiry_and_positive_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache_path = temp_dir.path();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let key_neg = TrackKey {
            file_path: Some(PathBuf::from("neg.mp3")),
            title: "NotFound".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_sec: 180,
        };

        // 1. Write negative cache that is 7 hours old (> 6h limit)
        let record_expired = CacheRecord {
            key: key_neg.clone(),
            outcome: CacheOutcome::NotFound,
            cached_at_epoch_sec: now.saturating_sub(7 * 3600),
        };
        let file_path = cache_path.join(key_neg.cache_filename());
        std::fs::write(&file_path, serde_json::to_vec(&record_expired).unwrap()).unwrap();
        assert!(file_path.exists());

        // Read must return None and delete the expired file
        assert_eq!(read_from_cache(cache_path, &key_neg), None);
        assert!(
            !file_path.exists(),
            "Expired negative cache file must be removed"
        );

        // 2. Write negative cache that is 2 hours old (< 6h limit)
        let record_valid = CacheRecord {
            key: key_neg.clone(),
            outcome: CacheOutcome::NotFound,
            cached_at_epoch_sec: now.saturating_sub(2 * 3600),
        };
        std::fs::write(&file_path, serde_json::to_vec(&record_valid).unwrap()).unwrap();
        assert_eq!(
            read_from_cache(cache_path, &key_neg),
            Some(CacheOutcome::NotFound)
        );

        // 3. Write positive cache that is 100 hours old: must persist!
        let key_pos = TrackKey {
            file_path: Some(PathBuf::from("pos.mp3")),
            title: "Found".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_sec: 180,
        };
        let record_pos = CacheRecord {
            key: key_pos.clone(),
            outcome: CacheOutcome::Found {
                lyrics: "Lyrics".to_string(),
                is_synced: false,
            },
            cached_at_epoch_sec: now.saturating_sub(100 * 3600),
        };
        let pos_file = cache_path.join(key_pos.cache_filename());
        std::fs::write(&pos_file, serde_json::to_vec(&record_pos).unwrap()).unwrap();
        assert_eq!(
            read_from_cache(cache_path, &key_pos),
            Some(CacheOutcome::Found {
                lyrics: "Lyrics".to_string(),
                is_synced: false,
            })
        );
    }

    #[test]
    fn test_client_error_is_deterministic_without_retries() {
        let temp_dir = tempfile::tempdir().unwrap();
        let transport = Arc::new(MockTransport::new(|_, _, _, _| {
            // Documented 4xx client error (e.g. 400 Bad Request)
            Ok(TransportResponse::ClientError(400))
        }));

        let mut service = OnlineLyricsService::with_transport_and_cache(
            transport.clone(),
            temp_dir.path().to_path_buf(),
        );

        let mut state = AppState::new();
        state.set_online_lyrics_enabled(true);

        let mut library = Library::new();
        library.songs = vec![create_test_song(0, "Song A", "Artist A", "Album A", 180.0)];
        state.current_queue = vec![0];
        state.current_queue_idx = 0;
        let player = AudioPlayer::new();

        service.poll(&mut state, &mut library, &player);

        for _ in 0..50 {
            service.poll(&mut state, &mut library, &player);
            if state.online_lyrics_status == OnlineLyricsStatus::Error {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(state.online_lyrics_status, OnlineLyricsStatus::Error);
        assert_eq!(transport.call_count.load(Ordering::SeqCst), 1);
        // Client errors must NOT schedule timed retries
        assert!(service.next_retry_time.is_none());
        assert_eq!(service.retry_count, 0);
    }

    #[test]
    fn test_album_mismatch_rejected() {
        let key = TrackKey {
            file_path: Some(PathBuf::from("test.mp3")),
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: "Expected Album".to_string(),
            duration_sec: 180,
        };

        let body = serde_json::json!({
            "trackName": "Song",
            "artistName": "Artist",
            "albumName": "Different Album",
            "duration": 180.0,
            "syncedLyrics": "[00:01.00]Lyrics"
        })
        .to_string();

        assert_eq!(
            validate_and_extract_lyrics(&body, &key).unwrap(),
            CacheOutcome::Mismatch
        );
    }

    #[test]
    fn test_body_cap_overflow_detected() {
        // Simulated body exceeding MAX_BODY_BYTES (256 KB)
        let oversized = vec![b'a'; (MAX_BODY_BYTES + 10) as usize];
        let mut reader = std::io::Cursor::new(oversized).take(MAX_BODY_BYTES + 1);
        let mut body = String::new();
        reader.read_to_string(&mut body).unwrap();
        assert!(
            body.len() as u64 > MAX_BODY_BYTES,
            "Overflow must be detected"
        );
    }
}
