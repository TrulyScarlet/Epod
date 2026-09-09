use crate::audio::metadata::TrackMetadata;
use crate::library::model::{paths_match, Library, Song, VideoCategory, VideoItem};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use walkdir::WalkDir;

pub enum ScanProgress {
    Batch {
        songs: Vec<Song>,
        videos: Vec<VideoItem>,
    },
    Finished {
        total_songs: usize,
        #[allow(dead_code)]
        total_videos: usize,
        removed_paths: Vec<PathBuf>,
    },
}

pub struct AsyncScanner {
    pub receiver: Receiver<ScanProgress>,
}

impl AsyncScanner {
    pub fn start(sources: Vec<PathBuf>, cached_tracks: HashMap<PathBuf, (u64, u64)>) -> Self {
        let (sender, receiver) = channel();

        thread::spawn(move || {
            let audio_extensions = ["mp3", "flac", "m4a", "aac", "wav", "ogg", "aiff"];
            let video_extensions = ["mp4", "m4v", "mov", "mkv", "avi"];

            let mut batch_songs = Vec::new();
            let mut batch_videos = Vec::new();
            let mut total_songs = 0;
            let mut total_videos = 0;
            let mut seen_audio_paths = HashSet::new();

            let mut scan_dirs = sources;
            if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                let video_dir = PathBuf::from(&user_profile).join("Videos");
                if video_dir.exists() && !scan_dirs.contains(&video_dir) {
                    scan_dirs.push(video_dir);
                }
            }

            for dir in scan_dirs {
                if !dir.exists() {
                    continue;
                }

                for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
                            if audio_extensions.contains(&ext.as_str()) {
                                let path_buf = path.to_path_buf();
                                seen_audio_paths.insert(path_buf.clone());

                                let meta_stat = std::fs::metadata(path).ok();
                                let mtime_sec = meta_stat
                                    .as_ref()
                                    .and_then(|m| m.modified().ok())
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);
                                let file_size = meta_stat.as_ref().map(|m| m.len()).unwrap_or(0);

                                let cached_stat = cached_tracks.get(&path_buf).copied().or_else(|| {
                                    cached_tracks.iter().find_map(|(cp, stat)| {
                                        if paths_match(cp, path) {
                                            Some(*stat)
                                        } else {
                                            None
                                        }
                                    })
                                });

                                let is_cached_and_unchanged = if let Some((cmtime, csize)) = cached_stat {
                                    cmtime == mtime_sec && csize == file_size
                                } else {
                                    false
                                };

                                if is_cached_and_unchanged {
                                    continue;
                                }

                                let meta = match TrackMetadata::from_file(path) {
                                    Ok(m) => m,
                                    Err(_) => TrackMetadata::empty(path),
                                };

                                batch_songs.push(Song {
                                    id: 0, // will be assigned on merge
                                    title: meta.title,
                                    artist: meta.artist,
                                    album: meta.album,
                                    genre: meta.genre,
                                    composer: meta.composer,
                                    year: meta.year,
                                    track_number: meta.track_number,
                                    duration_sec: meta.duration_sec,
                                    bitrate_kbps: meta.bitrate_kbps,
                                    codec: meta.codec,
                                    file_path: Some(path_buf),
                                    rating: 0,
                                    play_count: 0,
                                    is_synthetic_demo: false,
                                    artwork_bytes: meta.artwork_bytes,
                                    lyrics: meta.lyrics,
                                    parsed_lyrics: meta.parsed_lyrics,
                                });
                                total_songs += 1;

                                if batch_songs.len() >= 100 {
                                    let _ = sender.send(ScanProgress::Batch {
                                        songs: std::mem::take(&mut batch_songs),
                                        videos: std::mem::take(&mut batch_videos),
                                    });
                                }
                            } else if video_extensions.contains(&ext.as_str()) {
                                let title = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown Video")
                                    .to_string();

                                let category = if title.to_lowercase().contains("movie") || title.to_lowercase().contains("film") {
                                    VideoCategory::Movie
                                } else if title.to_lowercase().contains("music") || title.to_lowercase().contains("mv") {
                                    VideoCategory::MusicVideo
                                } else if title.to_lowercase().contains("podcast") {
                                    VideoCategory::VideoPodcast
                                } else {
                                    VideoCategory::TVShow
                                };

                                batch_videos.push(VideoItem {
                                    id: 0,
                                    title,
                                    category,
                                    duration_sec: 300.0_f32,
                                    file_path: Some(path.to_path_buf()),
                                    resume_pos_sec: 0.0_f32,
                                    thumbnail_bytes: None,
                                });
                                total_videos += 1;
                            }
                        }
                    }
                }
            }

            if !batch_songs.is_empty() || !batch_videos.is_empty() {
                let _ = sender.send(ScanProgress::Batch {
                    songs: batch_songs,
                    videos: batch_videos,
                });
            }

            let mut removed_paths = Vec::new();
            for cached_path in cached_tracks.keys() {
                let was_seen = seen_audio_paths.iter().any(|p| paths_match(p, cached_path));
                if !was_seen && !cached_path.exists() {
                    removed_paths.push(cached_path.clone());
                }
            }

            let _ = sender.send(ScanProgress::Finished {
                total_songs,
                total_videos,
                removed_paths,
            });
        });

        Self { receiver }
    }
}

pub struct LibraryScanner;

impl LibraryScanner {
    pub fn scan_directory(library: &mut Library, root_path: &Path) -> (usize, usize) {
        let mut songs_added = 0;
        let mut videos_added = 0;

        let audio_extensions = ["mp3", "flac", "m4a", "aac", "wav", "ogg", "aiff"];
        let video_extensions = ["mp4", "m4v", "mov", "mkv", "avi"];

        for entry in WalkDir::new(root_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
                    if audio_extensions.contains(&ext.as_str()) {
                        if library.songs.iter().any(|s| s.file_path.as_deref() == Some(path)) {
                            continue;
                        }

                        let meta = match TrackMetadata::from_file(path) {
                            Ok(m) => m,
                            Err(_) => TrackMetadata::empty(path),
                        };

                        let song = Song {
                            id: library.songs.len(),
                            title: meta.title,
                            artist: meta.artist,
                            album: meta.album,
                            genre: meta.genre,
                            composer: meta.composer,
                            year: meta.year,
                            track_number: meta.track_number,
                            duration_sec: meta.duration_sec,
                            bitrate_kbps: meta.bitrate_kbps,
                            codec: meta.codec,
                            file_path: Some(path.to_path_buf()),
                            rating: 0,
                            play_count: 0,
                            is_synthetic_demo: false,
                            artwork_bytes: meta.artwork_bytes,
                            lyrics: meta.lyrics,
                            parsed_lyrics: meta.parsed_lyrics,
                        };

                        library.songs.push(song);
                        songs_added += 1;
                    } else if video_extensions.contains(&ext.as_str()) {
                        if library.videos.iter().any(|v| v.file_path.as_deref() == Some(path)) {
                            continue;
                        }

                        let title = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown Video")
                            .to_string();

                        let category = if title.to_lowercase().contains("movie") || title.to_lowercase().contains("film") {
                            VideoCategory::Movie
                        } else if title.to_lowercase().contains("music") || title.to_lowercase().contains("mv") {
                            VideoCategory::MusicVideo
                        } else if title.to_lowercase().contains("podcast") {
                            VideoCategory::VideoPodcast
                        } else {
                            VideoCategory::TVShow
                        };

                        library.videos.push(VideoItem {
                            id: library.videos.len(),
                            title,
                            category,
                            duration_sec: 300.0_f32,
                            file_path: Some(path.to_path_buf()),
                            resume_pos_sec: 0.0_f32,
                            thumbnail_bytes: None,
                        });
                        videos_added += 1;
                    }
                }
            }
        }

        if songs_added > 0 || videos_added > 0 {
            library.rebuild_indices();
            library.save_library_cache();
        }

        (songs_added, videos_added)
    }

    pub fn rescan_all_sources(library: &mut Library) -> (usize, usize) {
        let sources = library.music_sources.clone();
        let mut total_songs = 0;
        let mut total_vids = 0;

        for src in sources {
            if src.exists() {
                let (s, v) = Self::scan_directory(library, &src);
                total_songs += s;
                total_vids += v;
            }
        }

        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let video_dir = PathBuf::from(&user_profile).join("Videos");
            if video_dir.exists() {
                let (s, v) = Self::scan_directory(library, &video_dir);
                total_songs += s;
                total_vids += v;
            }
        }

        if total_songs > 0 || total_vids > 0 {
            library.save_library_cache();
        }

        (total_songs, total_vids)
    }
}
