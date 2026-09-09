use crate::library::model::Song;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Number of concurrent network workers used during a repair run.
const WORKER_THREADS: usize = 4;

#[derive(Debug)]
pub struct MetadataRepairResult {
    pub song_id: usize,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork_bytes: Option<Vec<u8>>,
    pub lyrics: Option<String>,
}

#[derive(Clone)]
struct RepairRequest {
    song_id: usize,
    title: String,
    artist: String,
    album: String,
    duration_sec: f32,
    file_path: Option<PathBuf>,
    missing_artist: bool,
    missing_album: bool,
    missing_art: bool,
    missing_lyrics: bool,
}

/// Cached iTunes lookup result shared between identical tracks.
#[derive(Clone)]
struct LookupHit {
    artist: Option<String>,
    album: Option<String>,
    artwork: Option<Vec<u8>>,
}

pub fn start_metadata_repair(songs: &[Song]) -> Receiver<MetadataRepairResult> {
    let requests: Vec<_> = songs
        .iter()
        .filter_map(|song| {
            let missing_artist = song.artist.trim().is_empty() || song.artist.eq_ignore_ascii_case("Unknown Artist");
            let missing_album = song.album.trim().is_empty() || song.album.eq_ignore_ascii_case("Unknown Album");
            let missing_art = song.artwork_bytes.is_none() && !song.is_synthetic_demo;
            let missing_lyrics = song.lyrics.as_ref().is_none_or(|l| l.trim().is_empty()) && !song.is_synthetic_demo;
            if !(missing_artist || missing_album || missing_art || missing_lyrics) {
                return None;
            }
            Some(RepairRequest {
                song_id: song.id,
                title: song.title.clone(),
                artist: song.artist.clone(),
                album: song.album.clone(),
                duration_sec: song.duration_sec,
                file_path: song.file_path.clone(),
                missing_artist,
                missing_album,
                missing_art,
                missing_lyrics,
            })
        })
        .collect();

    let (tx, rx) = channel();
    let queue = Arc::new(Mutex::new(requests.into_iter()));
    // Dedupe cache so duplicate tracks (same title+artist) reuse one network hit
    let itunes_cache: Arc<Mutex<HashMap<String, LookupHit>>> = Arc::new(Mutex::new(HashMap::new()));

    for _ in 0..WORKER_THREADS.max(1) {
        let tx = tx.clone();
        let queue = Arc::clone(&queue);
        let itunes_cache = Arc::clone(&itunes_cache);
        std::thread::spawn(move || loop {
            let next = {
                let Ok(mut q) = queue.lock() else { break };
                q.next()
            };
            let Some(request) = next else { break };
            let result = repair_one(&request, &itunes_cache);
            let _ = tx.send(result);
        });
    }
    rx
}

fn repair_one(req: &RepairRequest, itunes_cache: &Arc<Mutex<HashMap<String, LookupHit>>>) -> MetadataRepairResult {
    let mut artist = None;
    let mut album = None;
    let mut artwork_bytes = None;

    let known_artist = if req.missing_artist { "" } else { req.artist.as_str() };
    let search_query = format!("{} {}", req.title, known_artist);
    let cache_key = search_query.to_lowercase();

    // Skip the network entirely when an identical lookup already succeeded
    let cached = itunes_cache.lock().ok().and_then(|c| c.get(&cache_key).cloned());
    let hit = if let Some(hit) = cached {
        Some(hit)
    } else {
        lookup_itunes(&search_query).map(|hit| {
            if let Ok(mut c) = itunes_cache.lock() {
                c.insert(cache_key.clone(), hit.clone());
            }
            hit
        })
    };

    if let Some(hit) = hit {
        if req.missing_artist {
            artist = hit.artist;
        }
        if req.missing_album {
            album = hit.album;
        }
        if req.missing_art {
            artwork_bytes = hit.artwork;
        }
    }

    let resolved_artist = artist.as_deref().unwrap_or(&req.artist);
    let resolved_album = album.as_deref().unwrap_or(&req.album);
    let mut lyrics = None;
    if req.missing_lyrics && !resolved_artist.eq_ignore_ascii_case("Unknown Artist") {
        let exact_url = format!(
            "https://lrclib.net/api/get?artist_name={}&track_name={}&album_name={}&duration={}",
            urlencode(resolved_artist),
            urlencode(&req.title),
            urlencode(resolved_album),
            req.duration_sec.round() as u32
        );
        lyrics = fetch_lyrics(&exact_url);
        if lyrics.is_none() {
            let search_url = format!(
                "https://lrclib.net/api/search?q={}",
                urlencode(&format!("{} {}", req.title, resolved_artist))
            );
            if let Some(json) = curl_json(&search_url) {
                if let Some(item) = json.as_array().and_then(|a| a.first()) {
                    lyrics = item.get("syncedLyrics").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(str::to_string)
                        .or_else(|| item.get("plainLyrics").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(str::to_string));
                }
            }
        }
    }

    persist_sidecars(req, artist.as_deref(), album.as_deref(), artwork_bytes.as_deref(), lyrics.as_deref());

    MetadataRepairResult {
        song_id: req.song_id,
        artist,
        album,
        artwork_bytes,
        lyrics,
    }
}

fn lookup_itunes(search_query: &str) -> Option<LookupHit> {
    let json = curl_json(&format!(
        "https://itunes.apple.com/search?term={}&entity=song&limit=1",
        urlencode(search_query)
    ))?;
    let item = json.get("results").and_then(|v| v.as_array()).and_then(|a| a.first())?.clone();
    let artist = item.get("artistName").and_then(|v| v.as_str()).map(str::to_string);
    let album = item.get("collectionName").and_then(|v| v.as_str()).map(str::to_string);
    let artwork = item
        .get("artworkUrl100")
        .and_then(|v| v.as_str())
        .map(|url| url.replace("100x100bb", "600x600bb"))
        .and_then(|hd| curl_bytes(&hd));
    Some(LookupHit { artist, album, artwork })
}

fn persist_sidecars(req: &RepairRequest, artist: Option<&str>, album: Option<&str>, art: Option<&[u8]>, lyrics: Option<&str>) {
    let Some(ref audio_path) = req.file_path else { return };
    let Some(parent) = audio_path.parent() else { return };
    let stem = audio_path.file_stem().and_then(|s| s.to_str()).unwrap_or("track");

    if artist.is_some() || album.is_some() {
        let sidecar_path = parent.join(format!("{}.epod.json", stem));
        // Merge with any existing override file so prior fields are preserved
        let mut override_json = std::fs::read(&sidecar_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = override_json.as_object_mut() {
            if let Some(a) = artist {
                obj.insert("artist".into(), serde_json::Value::String(a.to_string()));
            }
            if let Some(al) = album {
                obj.insert("album".into(), serde_json::Value::String(al.to_string()));
            }
        }
        let _ = std::fs::write(sidecar_path, override_json.to_string());
    }
    if let Some(bytes) = art {
        let _ = std::fs::write(parent.join(format!("{}.cover.jpg", stem)), bytes);
    }
    if let Some(text) = lyrics {
        let _ = std::fs::write(audio_path.with_extension("lrc"), text);
    }
}

fn curl_json(url: &str) -> Option<serde_json::Value> {
    let bytes = curl_bytes(url)?;
    serde_json::from_slice(&bytes).ok()
}

fn curl_bytes(url: &str) -> Option<Vec<u8>> {
    let mut command = Command::new("curl.exe");
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = command
        .args([
            "-L",
            "-s",
            "--fail",
            "--max-time",
            "10",
            "-A",
            "Epod/1.0 (metadata repair; contact: local user)",
            url,
        ])
        .output()
        .ok()?;
    if output.status.success() && !output.stdout.is_empty() {
        Some(output.stdout)
    } else {
        None
    }
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

// Small local helper kept private: fetch synced/plain lyrics from an LRCLIB endpoint.
fn fetch_lyrics(url: &str) -> Option<String> {
    let json = curl_json(url)?;
    json.get("syncedLyrics").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(str::to_string)
        .or_else(|| json.get("plainLyrics").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(str::to_string))
}
