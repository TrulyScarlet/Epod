use crate::library::model::Song;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrackListeningStats {
    pub key: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    #[serde(default)]
    pub play_count: u64,
    #[serde(default)]
    pub listened_seconds: f64,
    #[serde(default)]
    pub last_played_unix: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ListeningStats {
    #[serde(default)]
    pub tracks: HashMap<String, TrackListeningStats>,
    #[serde(default)]
    pub total_listened_seconds: f64,
    #[serde(default)]
    pub total_plays: u64,
}

#[derive(Debug, Clone)]
pub struct ListeningSession {
    pub key: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_sec: f32,
    pub listened_sec: f32,
    pub counted: bool,
}

impl ListeningSession {
    pub fn from_song(song: &Song) -> Self {
        Self {
            key: song_stats_key(song),
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            duration_sec: song.duration_sec,
            listened_sec: 0.0,
            counted: false,
        }
    }

    pub fn qualifying_seconds(&self) -> f32 {
        (self.duration_sec * 0.5).min(30.0).max(5.0)
    }
}

pub fn song_stats_key(song: &Song) -> String {
    if let Some(ref path) = song.file_path {
        return path.to_string_lossy().replace('/', "\\").to_lowercase();
    }
    format!(
        "{}|{}|{}",
        song.title.trim().to_lowercase(),
        song.artist.trim().to_lowercase(),
        song.album.trim().to_lowercase()
    )
}

impl ListeningStats {
    fn paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                paths.push(parent.join("epod_stats.json"));
            }
        }
        paths.push(PathBuf::from("epod_stats.json"));
        paths.push(PathBuf::from("target/release/epod_stats.json"));
        paths
    }

    pub fn load() -> Self {
        for path in Self::paths() {
            if let Ok(file) = std::fs::File::open(path) {
                if let Ok(stats) = serde_json::from_reader::<_, Self>(file) {
                    return stats;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                if let Ok(file) = std::fs::File::create(parent.join("epod_stats.json")) {
                    let _ = serde_json::to_writer_pretty(file, self);
                }
            }
        }
        if let Ok(file) = std::fs::File::create("epod_stats.json") {
            let _ = serde_json::to_writer_pretty(file, self);
        }
    }

    fn ensure_track(&mut self, session: &ListeningSession) -> &mut TrackListeningStats {
        self.tracks.entry(session.key.clone()).or_insert_with(|| TrackListeningStats {
            key: session.key.clone(),
            title: session.title.clone(),
            artist: session.artist.clone(),
            album: session.album.clone(),
            play_count: 0,
            listened_seconds: 0.0,
            last_played_unix: 0,
        })
    }

    pub fn add_listening_time(&mut self, session: &ListeningSession, seconds: f32) {
        if seconds <= 0.0 {
            return;
        }
        let seconds = seconds as f64;
        self.total_listened_seconds += seconds;
        let track = self.ensure_track(session);
        track.listened_seconds += seconds;
        track.title = session.title.clone();
        track.artist = session.artist.clone();
        track.album = session.album.clone();
    }

    pub fn record_play(&mut self, session: &ListeningSession) {
        self.total_plays += 1;
        let track = self.ensure_track(session);
        track.play_count += 1;
        track.last_played_unix = chrono::Utc::now().timestamp();
    }

    pub fn top_tracks(&self) -> Vec<&TrackListeningStats> {
        let mut tracks: Vec<_> = self.tracks.values().filter(|t| t.play_count > 0 || t.listened_seconds > 0.0).collect();
        tracks.sort_by(|a, b| {
            b.play_count
                .cmp(&a.play_count)
                .then_with(|| b.listened_seconds.partial_cmp(&a.listened_seconds).unwrap_or(std::cmp::Ordering::Equal))
        });
        tracks
    }

    pub fn aggregate_artists(&self) -> Vec<(String, u64, f64)> {
        let mut map: HashMap<String, (u64, f64)> = HashMap::new();
        for track in self.tracks.values() {
            let entry = map.entry(track.artist.clone()).or_default();
            entry.0 += track.play_count;
            entry.1 += track.listened_seconds;
        }
        let mut rows: Vec<_> = map.into_iter().map(|(name, (plays, seconds))| (name, plays, seconds)).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)));
        rows
    }

    pub fn aggregate_albums(&self) -> Vec<(String, u64, f64)> {
        let mut map: HashMap<String, (u64, f64)> = HashMap::new();
        for track in self.tracks.values() {
            let entry = map.entry(track.album.clone()).or_default();
            entry.0 += track.play_count;
            entry.1 += track.listened_seconds;
        }
        let mut rows: Vec<_> = map.into_iter().map(|(name, (plays, seconds))| (name, plays, seconds)).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)));
        rows
    }
}
