use crate::audio::metadata::LyricLine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Song {
    pub id: usize,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub composer: String,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub duration_sec: f32,
    #[serde(default)]
    pub bitrate_kbps: Option<u32>,
    #[serde(default)]
    pub codec: Option<String>,
    pub file_path: Option<PathBuf>,
    pub rating: u8, // 0 to 5 stars
    pub play_count: u32,
    pub is_synthetic_demo: bool,
    #[serde(skip)]
    pub artwork_bytes: Option<Vec<u8>>,
    pub lyrics: Option<String>,
    #[serde(skip)]
    pub parsed_lyrics: Vec<LyricLine>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub year: Option<u32>,
    pub song_ids: Vec<usize>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub artwork_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Artist {
    pub name: String,
    pub song_ids: Vec<usize>,
    pub album_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlaylistSongEntry {
    #[serde(default)]
    pub file_path: Option<PathBuf>,
    pub title: String,
    pub artist: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedTrack {
    pub file_path: PathBuf,
    pub mtime_sec: u64,
    pub file_size: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub composer: String,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub duration_sec: f32,
    pub bitrate_kbps: Option<u32>,
    pub codec: Option<String>,
    pub lyrics: Option<String>,
}

pub fn paths_match(p1: &Path, p2: &Path) -> bool {
    if p1 == p2 {
        return true;
    }
    let s1 = p1.to_string_lossy().replace('/', "\\").to_lowercase();
    let s2 = p2.to_string_lossy().replace('/', "\\").to_lowercase();
    let s1_clean = s1.strip_prefix("\\\\?\\").unwrap_or(&s1);
    let s2_clean = s2.strip_prefix("\\\\?\\").unwrap_or(&s2);
    if s1_clean == s2_clean {
        return true;
    }
    if let (Ok(c1), Ok(c2)) = (p1.canonicalize(), p2.canonicalize()) {
        if c1 == c2 {
            return true;
        }
        let cs1 = c1.to_string_lossy().replace('/', "\\").to_lowercase();
        let cs2 = c2.to_string_lossy().replace('/', "\\").to_lowercase();
        let cs1_clean = cs1.strip_prefix("\\\\?\\").unwrap_or(&cs1);
        let cs2_clean = cs2.strip_prefix("\\\\?\\").unwrap_or(&cs2);
        if cs1_clean == cs2_clean {
            return true;
        }
    }
    if let (Some(n1), Some(n2)) = (p1.file_name(), p2.file_name()) {
        if n1.to_string_lossy().eq_ignore_ascii_case(&n2.to_string_lossy()) {
            return true;
        }
    }
    false
}

fn default_playlist_id() -> String {
    format!("pl_{}", chrono::Utc::now().timestamp_millis())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Playlist {
    #[serde(default = "default_playlist_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub song_ids: Vec<usize>,
    #[serde(default)]
    pub song_entries: Vec<PlaylistSongEntry>,
    pub is_smart: bool,
    #[serde(default)]
    pub custom_cover_path: Option<PathBuf>,
    #[serde(default)]
    pub description: Option<String>,
}

impl Playlist {
    pub fn new(name: String) -> Self {
        Self {
            id: default_playlist_id(),
            name,
            song_ids: Vec::new(),
            song_entries: Vec::new(),
            is_smart: false,
            custom_cover_path: None,
            description: None,
        }
    }

    pub fn contains_song(&self, song_id: usize, library_songs: &[Song]) -> bool {
        if self.song_ids.contains(&song_id) {
            return true;
        }
        if let Some(song) = library_songs.get(song_id) {
            self.song_entries.iter().any(|entry| {
                if let (Some(ref p1), Some(ref p2)) = (&entry.file_path, &song.file_path) {
                    if paths_match(p1, p2) {
                        return true;
                    }
                }
                (entry.title.eq_ignore_ascii_case(&song.title) && entry.artist.eq_ignore_ascii_case(&song.artist))
                    || (entry.title.eq_ignore_ascii_case(&song.title) && entry.artist.to_lowercase().replace(' ', "") == song.artist.to_lowercase().replace(' ', ""))
            })
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VideoCategory {
    Movie,
    MusicVideo,
    TVShow,
    VideoPodcast,
}

impl VideoCategory {
    pub fn name(&self) -> &'static str {
        match self {
            VideoCategory::Movie => "Movies",
            VideoCategory::MusicVideo => "Music Videos",
            VideoCategory::TVShow => "TV Shows",
            VideoCategory::VideoPodcast => "Video Podcasts",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoItem {
    pub id: usize,
    pub title: String,
    pub category: VideoCategory,
    pub duration_sec: f32,
    pub file_path: Option<PathBuf>,
    pub resume_pos_sec: f32,
    #[serde(skip)]
    #[allow(dead_code)]
    pub thumbnail_bytes: Option<Vec<u8>>,
}

pub struct Library {
    pub songs: Vec<Song>,
    pub sorted_song_indices: Vec<usize>, // A-Z sorted indices by song title
    pub albums: HashMap<String, Album>, // key: "AlbumName - ArtistName"
    pub artists: HashMap<String, Artist>,
    pub genres: HashMap<String, Vec<usize>>,
    pub composers: HashMap<String, Vec<usize>>,
    pub playlists: Vec<Playlist>,
    pub user_playlists: Vec<Playlist>,
    pub on_the_go_ids: Vec<usize>,
    pub videos: Vec<VideoItem>,
    pub music_sources: Vec<PathBuf>,
    pub cached_track_stats: HashMap<PathBuf, (u64, u64)>,
}

impl Library {
    pub fn new() -> Self {
        let mut lib = Self {
            songs: Vec::new(),
            sorted_song_indices: Vec::new(),
            albums: HashMap::new(),
            artists: HashMap::new(),
            genres: HashMap::new(),
            composers: HashMap::new(),
            playlists: Vec::new(),
            user_playlists: Vec::new(),
            on_the_go_ids: Vec::new(),
            videos: Vec::new(),
            music_sources: Vec::new(),
            cached_track_stats: HashMap::new(),
        };

        lib.load_sources_config();
        lib.load_playlists_config();
        let cache_loaded = lib.load_library_cache();
        if !cache_loaded || lib.songs.is_empty() {
            lib.populate_default_demo_content();
        } else {
            lib.populate_default_demo_videos();
        }
        lib.rebuild_indices();
        lib
    }

    pub fn populate_default_demo_videos(&mut self) {
        if !self.videos.is_empty() {
            return;
        }
        self.videos = vec![
            VideoItem {
                id: 0,
                title: "Epod 5th Gen - Keynote Introduction".to_string(),
                category: VideoCategory::Movie,
                duration_sec: 145.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
            VideoItem {
                id: 1,
                title: "Neon Skyline (Official Music Video)".to_string(),
                category: VideoCategory::MusicVideo,
                duration_sec: 184.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
            VideoItem {
                id: 2,
                title: "Cyber Odyssey: Episode 1".to_string(),
                category: VideoCategory::TVShow,
                duration_sec: 320.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
            VideoItem {
                id: 3,
                title: "Retro Tech Podcast #42 - Video Epods".to_string(),
                category: VideoCategory::VideoPodcast,
                duration_sec: 480.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
        ];
    }

    pub fn populate_default_demo_content(&mut self) {
        let demo_songs = vec![
            (
                "Neon Skyline",
                "RetroWave Collective",
                "Synthwave Memories (2005)",
                "Electronic",
                "K. Vance",
                2005,
                1,
                184.0_f32,
                "[00:00.00] (Synth Intro)\n[00:08.50] Driving through the neon glow\n[00:15.20] Midnight highways down below\n[00:23.10] City lights reflect your eyes\n[00:30.40] Electric dreams beneath the skies\n[00:38.00] (Chorus)\n[00:39.50] Neon skyline, take me away\n[00:46.00] Into the twilight of yesterday\n[00:54.00] Feel the rhythm in the digital stream\n[01:02.00] Living inside a 5th gen dream",
            ),
            (
                "Summer Drive 2005",
                "The California Horizon",
                "Pacific Coast Sessions",
                "Indie Rock",
                "J. Miller",
                2005,
                2,
                210.0_f32,
                "[00:00.00] (Guitar Riff)\n[00:10.00] Top down on Highway 1\n[00:18.00] Chasing after the setting sun\n[00:26.50] Click wheel spinning track by track\n[00:34.00] We're never ever turning back\n[00:42.00] (Chorus)\n[00:43.00] Summer breezes in our hair\n[00:50.00] Music floating through the air",
            ),
            (
                "Midnight Groove",
                "Tokyo Jazz Club",
                "Late Night Shibuya",
                "Jazz",
                "T. Sato",
                2004,
                3,
                195.0_f32,
                "[00:00.00] (Upright Bass Solo)\n[00:12.00] Raindrops on the cafe window\n[00:24.00] Saxophone plays soft and slow\n[00:36.00] Smooth espresso, midnight tea\n[00:48.00] Lost in Shibuya harmony",
            ),
            (
                "Digital Orbit",
                "Silicon Pulse",
                "Pure Algorithm",
                "Dance",
                "A. Turing",
                2005,
                4,
                168.0_f32,
                "[00:00.00] (Electronic Kick)\n[00:14.00] Pulse... Pulse... Pulse...\n[00:28.00] Synchronize your frequency\n[00:42.00] Welcome to the digital reality",
            ),
            (
                "Acoustic Horizon",
                "Clara & The Strings",
                "Unplugged Moments",
                "Acoustic",
                "Clara Evans",
                2006,
                5,
                225.0_f32,
                "[00:00.00] (Fingerpicked Acoustic Guitar)\n[00:14.00] Morning dew upon the grass\n[00:27.00] Watching winter shadows pass\n[00:40.00] Gentle whispers in the breeze\n[00:53.00] Rest your mind and be at ease",
            ),
        ];

        for (idx, (title, artist, album, genre, comp, yr, trk, dur, lyr)) in demo_songs.into_iter().enumerate() {
            let parsed_lyr = crate::audio::metadata::parse_lrc_or_plain_lyrics(lyr);
            self.songs.push(Song {
                id: self.songs.len(),
                title: title.to_string(),
                artist: artist.to_string(),
                album: album.to_string(),
                genre: genre.to_string(),
                composer: comp.to_string(),
                year: Some(yr),
                track_number: Some(trk),
                duration_sec: dur,
                bitrate_kbps: None,
                codec: Some("Demo Synth".to_string()),
                file_path: None,
                rating: if idx == 0 { 5 } else if idx == 1 { 4 } else { 3 },
                play_count: (5 - idx as u32) * 8,
                is_synthetic_demo: true,
                artwork_bytes: None,
                lyrics: Some(lyr.to_string()),
                parsed_lyrics: lyr.lines().map(|s| {
                    LyricLine {
                        timestamp_sec: 0.0_f32,
                        text: s.to_string(),
                    }
                }).collect(),
            });
            self.songs[idx].parsed_lyrics = parsed_lyr;
        }

        // Built-in Demo Videos
        self.videos = vec![
            VideoItem {
                id: 0,
                title: "Epod 5th Gen - Keynote Introduction".to_string(),
                category: VideoCategory::Movie,
                duration_sec: 145.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
            VideoItem {
                id: 1,
                title: "Neon Skyline (Official Music Video)".to_string(),
                category: VideoCategory::MusicVideo,
                duration_sec: 184.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
            VideoItem {
                id: 2,
                title: "Cyber Odyssey: Episode 1".to_string(),
                category: VideoCategory::TVShow,
                duration_sec: 320.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
            VideoItem {
                id: 3,
                title: "Retro Tech Podcast #42 - Video Epods".to_string(),
                category: VideoCategory::VideoPodcast,
                duration_sec: 480.0_f32,
                file_path: None,
                resume_pos_sec: 0.0_f32,
                thumbnail_bytes: None,
            },
        ];
    }

    pub fn rebuild_indices(&mut self) {
        self.albums.clear();
        self.artists.clear();
        self.genres.clear();
        self.composers.clear();

        // 1. Build A-Z Alphabetical Song Index
        let mut sorted_ids: Vec<usize> = (0..self.songs.len()).collect();
        sorted_ids.sort_by(|&a, &b| {
            self.songs[a].title.to_lowercase().cmp(&self.songs[b].title.to_lowercase())
        });
        self.sorted_song_indices = sorted_ids;

        // 2. Index Albums, Artists, Genres, Composers
        for (song_idx, song) in self.songs.iter().enumerate() {
            let album_key = format!("{} - {}", song.album, song.artist);
            let album = self.albums.entry(album_key).or_insert_with(|| Album {
                name: song.album.clone(),
                artist: song.artist.clone(),
                year: song.year,
                song_ids: Vec::new(),
                artwork_bytes: song.artwork_bytes.clone(),
            });
            album.song_ids.push(song_idx);
            if album.artwork_bytes.is_none() && song.artwork_bytes.is_some() {
                album.artwork_bytes = song.artwork_bytes.clone();
            }

            let artist = self.artists.entry(song.artist.clone()).or_insert_with(|| Artist {
                name: song.artist.clone(),
                song_ids: Vec::new(),
                album_names: Vec::new(),
            });
            artist.song_ids.push(song_idx);
            if !artist.album_names.contains(&song.album) {
                artist.album_names.push(song.album.clone());
            }

            self.genres
                .entry(song.genre.clone())
                .or_default()
                .push(song_idx);

            self.composers
                .entry(song.composer.clone())
                .or_default()
                .push(song_idx);
        }

        // We only retain custom user playlists
        self.user_playlists.retain(|pl| pl.id != "smart_all_songs" && pl.id != "smart_recently_played" && pl.id != "smart_top_25" && pl.id != "smart_top_rated" && pl.id != "smart_90s" && pl.id != "smart_on_the_go");
        let mut all_playlists = Vec::new();
        let has_real_songs = self.songs.iter().any(|s| !s.is_synthetic_demo);

        // Sanitize & append user playlists
        for user_pl in &mut self.user_playlists {
            user_pl.is_smart = false;

            // 1. If song_entries is empty but song_ids has entries, populate song_entries
            for &sid in &user_pl.song_ids {
                if let Some(song) = self.songs.get(sid) {
                    let entry = PlaylistSongEntry {
                        file_path: song.file_path.clone(),
                        title: song.title.clone(),
                        artist: song.artist.clone(),
                    };
                    if !user_pl.song_entries.iter().any(|e| {
                        if let (Some(ref p1), Some(ref p2)) = (&e.file_path, &entry.file_path) {
                            if paths_match(p1, p2) {
                                return true;
                            }
                        }
                        e.title.trim().eq_ignore_ascii_case(entry.title.trim())
                            && e.artist.trim().eq_ignore_ascii_case(entry.artist.trim())
                    }) {
                        user_pl.song_entries.push(entry);
                    }
                }
            }

            // 2. Resolve song_entries into current runtime song_ids
            if !user_pl.song_entries.is_empty() {
                let mut resolved_ids = Vec::new();
                for entry in &user_pl.song_entries {
                    let matched_id = self.songs.iter().position(|s| {
                        if let (Some(ref p1), Some(ref p2)) = (&entry.file_path, &s.file_path) {
                            if paths_match(p1, p2) {
                                return true;
                            }
                        }
                        (s.title.trim().eq_ignore_ascii_case(entry.title.trim())
                            && s.artist.trim().eq_ignore_ascii_case(entry.artist.trim()))
                            || (s.title.trim().eq_ignore_ascii_case(entry.title.trim())
                                && s.artist.to_lowercase().replace(' ', "")
                                    == entry.artist.to_lowercase().replace(' ', ""))
                    });

                    if let Some(id) = matched_id {
                        if !resolved_ids.contains(&id) {
                            resolved_ids.push(id);
                        }
                    }
                }

                if !resolved_ids.is_empty() || has_real_songs {
                    user_pl.song_ids = resolved_ids;
                }
            }

            all_playlists.push(user_pl.clone());
        }

        self.playlists = all_playlists;
    }

    fn library_cache_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                paths.push(parent.join("epod_library_cache.json"));
            }
        }
        paths.push(PathBuf::from("epod_library_cache.json"));
        paths.push(PathBuf::from("target/release/epod_library_cache.json"));
        paths
    }

    pub fn load_library_cache(&mut self) -> bool {
        for path in Self::library_cache_paths() {
            if path.exists() {
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(cached_tracks) = serde_json::from_reader::<_, Vec<CachedTrack>>(file) {
                        if !cached_tracks.is_empty() {
                            for track in cached_tracks {
                                self.cached_track_stats.insert(
                                    track.file_path.clone(),
                                    (track.mtime_sec, track.file_size),
                                );
                                let parsed_lyrics = if let Some(ref lyrics_text) = track.lyrics {
                                    crate::audio::metadata::parse_lrc_or_plain_lyrics(lyrics_text)
                                } else {
                                    Vec::new()
                                };
                                self.songs.push(Song {
                                    id: self.songs.len(),
                                    title: track.title,
                                    artist: track.artist,
                                    album: track.album,
                                    genre: track.genre,
                                    composer: track.composer,
                                    year: track.year,
                                    track_number: track.track_number,
                                    duration_sec: track.duration_sec,
                                    bitrate_kbps: track.bitrate_kbps,
                                    codec: track.codec,
                                    file_path: Some(track.file_path),
                                    rating: 0,
                                    play_count: 0,
                                    is_synthetic_demo: false,
                                    artwork_bytes: None,
                                    lyrics: track.lyrics,
                                    parsed_lyrics,
                                });
                            }
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn save_library_cache(&self) {
        let cached_tracks: Vec<CachedTrack> = self
            .songs
            .iter()
            .filter(|s| !s.is_synthetic_demo && s.file_path.is_some())
            .map(|s| {
                let p = s.file_path.as_ref().unwrap();
                let (mtime_sec, file_size) = self
                    .cached_track_stats
                    .get(p)
                    .copied()
                    .unwrap_or_else(|| {
                        if let Ok(meta) = std::fs::metadata(p) {
                            let mtime = meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            (mtime, meta.len())
                        } else {
                            (0, 0)
                        }
                    });
                CachedTrack {
                    file_path: p.clone(),
                    mtime_sec,
                    file_size,
                    title: s.title.clone(),
                    artist: s.artist.clone(),
                    album: s.album.clone(),
                    genre: s.genre.clone(),
                    composer: s.composer.clone(),
                    year: s.year,
                    track_number: s.track_number,
                    duration_sec: s.duration_sec,
                    bitrate_kbps: s.bitrate_kbps,
                    codec: s.codec.clone(),
                    lyrics: s.lyrics.clone(),
                }
            })
            .collect();

        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let path = parent.join("epod_library_cache.json");
                if let Ok(file) = std::fs::File::create(path) {
                    let _ = serde_json::to_writer_pretty(file, &cached_tracks);
                }
            }
        }
        if let Ok(file) = std::fs::File::create("epod_library_cache.json") {
            let _ = serde_json::to_writer_pretty(file, &cached_tracks);
        }
    }

    pub fn cached_tracks_map(&self) -> HashMap<PathBuf, (u64, u64)> {
        let mut map = self.cached_track_stats.clone();
        for s in &self.songs {
            if let Some(ref path) = s.file_path {
                map.entry(path.clone()).or_insert_with(|| {
                    if let Ok(meta) = std::fs::metadata(path) {
                        let mtime = meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        (mtime, meta.len())
                    } else {
                        (0, 0)
                    }
                });
            }
        }
        map
    }

    pub fn add_source_folder(&mut self, path: PathBuf) {
        if !self.music_sources.contains(&path) {
            self.music_sources.push(path);
            self.save_sources_config();
        }
    }

    pub fn remove_source_folder(&mut self, index: usize) {
        if index < self.music_sources.len() {
            self.music_sources.remove(index);
            self.save_sources_config();
        }
    }

    fn playlists_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                paths.push(parent.join("epod_playlists.json"));
            }
        }
        paths.push(PathBuf::from("epod_playlists.json"));
        paths.push(PathBuf::from("target/release/epod_playlists.json"));
        paths
    }

    pub fn load_playlists_config(&mut self) {
        for path in Self::playlists_config_paths() {
            if path.exists() {
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(mut user_pls) = serde_json::from_reader::<_, Vec<Playlist>>(file) {
                        user_pls.retain(|pl| {
                            pl.id != "smart_all_songs"
                                && pl.id != "smart_recently_played"
                                && pl.id != "smart_top_25"
                                && pl.id != "smart_top_rated"
                                && pl.id != "smart_90s"
                                && pl.id != "smart_on_the_go"
                        });
                        if !user_pls.is_empty() {
                            self.user_playlists = user_pls;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn save_playlists_config(&self) {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let path = parent.join("epod_playlists.json");
                if let Ok(file) = std::fs::File::create(path) {
                    let _ = serde_json::to_writer_pretty(file, &self.user_playlists);
                }
            }
        }
        if let Ok(file) = std::fs::File::create("epod_playlists.json") {
            let _ = serde_json::to_writer_pretty(file, &self.user_playlists);
        }
    }

    pub fn create_user_playlist(&mut self, name: String) -> usize {
        let new_pl = Playlist::new(name);
        self.user_playlists.push(new_pl);
        self.save_playlists_config();
        self.rebuild_indices();
        self.playlists.len().saturating_sub(1)
    }

    pub fn delete_playlist_by_idx(&mut self, playlist_idx: usize) -> bool {
        if let Some(pl) = self.playlists.get(playlist_idx) {
            let target_id = pl.id.clone();
            self.user_playlists.retain(|p| p.id != target_id);
            self.save_playlists_config();
            self.rebuild_indices();
            return true;
        }
        false
    }

    pub fn rename_playlist_by_idx(&mut self, playlist_idx: usize, new_name: String) -> bool {
        if let Some(pl) = self.playlists.get(playlist_idx) {
            let target_id = pl.id.clone();
            if let Some(upl) = self.user_playlists.iter_mut().find(|p| p.id == target_id) {
                upl.name = new_name;
                self.save_playlists_config();
                self.rebuild_indices();
                return true;
            } else if pl.id == "smart_on_the_go" {
                // Can't rename system on-the-go
            }
        }
        false
    }

    pub fn set_playlist_cover_by_idx(&mut self, playlist_idx: usize, cover_path: Option<PathBuf>) -> bool {
        if let Some(pl) = self.playlists.get(playlist_idx) {
            let target_id = pl.id.clone();
            if let Some(upl) = self.user_playlists.iter_mut().find(|p| p.id == target_id) {
                upl.custom_cover_path = cover_path.clone();
                self.save_playlists_config();
                self.rebuild_indices();
                return true;
            } else if let Some(tpl) = self.playlists.get_mut(playlist_idx) {
                tpl.custom_cover_path = cover_path;
                return true;
            }
        }
        false
    }

    pub fn toggle_song_in_playlist(&mut self, playlist_idx: usize, song_id: usize) {
        if let Some(song) = self.songs.get(song_id) {
            let entry = PlaylistSongEntry {
                file_path: song.file_path.clone(),
                title: song.title.clone(),
                artist: song.artist.clone(),
            };
            if let Some(pl) = self.playlists.get(playlist_idx) {
                let target_id = pl.id.clone();
                if let Some(upl) = self.user_playlists.iter_mut().find(|p| p.id == target_id) {
                    if let Some(pos) = upl.song_entries.iter().position(|e| {
                        if let (Some(ref p1), Some(ref p2)) = (&e.file_path, &entry.file_path) {
                            if paths_match(p1, p2) {
                                return true;
                            }
                        }
                        (e.title.eq_ignore_ascii_case(&entry.title) && e.artist.eq_ignore_ascii_case(&entry.artist))
                            || (e.title.eq_ignore_ascii_case(&entry.title) && e.artist.to_lowercase().replace(' ', "") == entry.artist.to_lowercase().replace(' ', ""))
                    }) {
                        upl.song_entries.remove(pos);
                    } else {
                        upl.song_entries.push(entry);
                    }
                    self.save_playlists_config();
                    self.rebuild_indices();
                } else if target_id == "smart_on_the_go" {
                    if let Some(pos) = self.on_the_go_ids.iter().position(|&id| id == song_id) {
                        self.on_the_go_ids.remove(pos);
                    } else {
                        self.on_the_go_ids.push(song_id);
                    }
                    self.rebuild_indices();
                }
            }
        }
    }

    fn sources_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                paths.push(parent.join("epod_sources.json"));
            }
        }
        paths.push(PathBuf::from("epod_sources.json"));
        paths.push(PathBuf::from("target/release/epod_sources.json"));
        paths
    }

    pub fn load_sources_config(&mut self) {
        for path in Self::sources_config_paths() {
            if path.exists() {
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(sources) = serde_json::from_reader::<_, Vec<PathBuf>>(file) {
                        if !sources.is_empty() {
                            self.music_sources = sources;
                            return;
                        }
                    }
                }
            }
        }
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let music_dir = PathBuf::from(&user_profile).join("Music");
            if music_dir.exists() && !self.music_sources.contains(&music_dir) {
                self.music_sources.push(music_dir);
            }
        }
    }

    pub fn save_sources_config(&self) {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let path = parent.join("epod_sources.json");
                if let Ok(file) = std::fs::File::create(path) {
                    let _ = serde_json::to_writer_pretty(file, &self.music_sources);
                }
            }
        }
        if let Ok(file) = std::fs::File::create("epod_sources.json") {
            let _ = serde_json::to_writer_pretty(file, &self.music_sources);
        }
    }
}
