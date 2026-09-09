use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub timestamp_sec: f32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TrackMetadata {
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
    pub artwork_bytes: Option<Vec<u8>>,
    pub lyrics: Option<String>,
    pub parsed_lyrics: Vec<LyricLine>,
}

impl TrackMetadata {
    pub fn empty(path: &Path) -> Self {
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Track")
            .to_string();

        let artwork_bytes = find_local_cover_art(path);
        let codec = path.extension().and_then(|e| e.to_str()).map(|e| e.to_uppercase());

        Self {
            title: file_stem,
            artist: "Unknown Artist".to_string(),
            album: "Unknown Album".to_string(),
            genre: "Music".to_string(),
            composer: "Unknown Composer".to_string(),
            year: None,
            track_number: None,
            duration_sec: 180.0,
            bitrate_kbps: None,
            codec,
            artwork_bytes,
            lyrics: None,
            parsed_lyrics: Vec::new(),
        }
    }

    pub fn extract_artwork(path: &Path) -> Option<Vec<u8>> {
        if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read().map_err(Into::into)) {
            let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
            if let Some(t) = tag {
                if let Some(pic) = t.pictures().first() {
                    return Some(pic.data().to_vec());
                }
            }
            for other_tag in tagged_file.tags() {
                if let Some(pic) = other_tag.pictures().first() {
                    return Some(pic.data().to_vec());
                }
            }
        }
        find_local_cover_art(path)
    }

    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let tagged_file = Probe::open(path)?.read()?;
        let properties = tagged_file.properties();
        let duration_sec = properties.duration().as_secs_f32();

        // Extract bitrate (lofty returns kbps directly) and format/codec
        let bitrate_kbps = properties
            .audio_bitrate()
            .or_else(|| properties.overall_bitrate())
            .map(|val| if val > 10_000 { val / 1000 } else { val })
            .filter(|&k| k > 0)
            .or_else(|| {
                // Accurate fallback: calculate kbps from file size and track duration
                if duration_sec > 0.0 {
                    if let Ok(file_meta) = std::fs::metadata(path) {
                        let total_bits = (file_meta.len() as f64) * 8.0;
                        let kbps = (total_bits / (duration_sec as f64 * 1000.0)).round() as u32;
                        if (32..=40_000).contains(&kbps) {
                            return Some(kbps);
                        }
                    }
                }
                None
            });

        let codec = file_type_codec(tagged_file.file_type())
            .or_else(|| path.extension().and_then(|e| e.to_str()).map(|e| e.to_uppercase()));

        let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

        let title = tag
            .and_then(|t| t.title().map(|s| s.to_string()))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Title")
                    .to_string()
            });

        let mut artist = tag
            .and_then(|t| t.artist().map(|s| s.to_string()))
            .unwrap_or_else(|| "Unknown Artist".to_string());

        let mut album = tag
            .and_then(|t| t.album().map(|s| s.to_string()))
            .unwrap_or_else(|| "Unknown Album".to_string());

        // Apply Epod's persistent online metadata sidecar when native tags are incomplete.
        if let Some(parent) = path.parent() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let sidecar = parent.join(format!("{}.epod.json", stem));
                if let Ok(bytes) = std::fs::read(sidecar) {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        if let Some(value) = json.get("artist").and_then(|v| v.as_str()) {
                            if !value.trim().is_empty() { artist = value.to_string(); }
                        }
                        if let Some(value) = json.get("album").and_then(|v| v.as_str()) {
                            if !value.trim().is_empty() { album = value.to_string(); }
                        }
                    }
                }
            }
        }

        let genre = tag
            .and_then(|t| t.genre().map(|s| s.to_string()))
            .unwrap_or_else(|| "Music".to_string());

        let composer = tag
            .and_then(|t| t.get_string(&ItemKey::Composer).map(|s| s.to_string()))
            .unwrap_or_else(|| "Unknown Composer".to_string());

        let year = tag.and_then(|t| t.year());
        let track_number = tag.and_then(|t| t.track());

        // Extract album artwork from all possible tag locations
        let mut artwork_bytes = None;
        if let Some(t) = tag {
            if let Some(pic) = t.pictures().first() {
                artwork_bytes = Some(pic.data().to_vec());
            }
        }

        // If no embedded picture found in primary tag, check other tags
        if artwork_bytes.is_none() {
            for other_tag in tagged_file.tags() {
                if let Some(pic) = other_tag.pictures().first() {
                    artwork_bytes = Some(pic.data().to_vec());
                    break;
                }
            }
        }

        // If still no embedded picture, check local folder for cover images
        if artwork_bytes.is_none() {
            artwork_bytes = find_local_cover_art(path);
        }

        // Extract lyrics
        let mut raw_lyrics = tag
            .and_then(|t| t.get_string(&ItemKey::Lyrics).map(|s| s.to_string()));

        // If no embedded lyrics, check for .lrc file in the same directory
        if raw_lyrics.is_none() {
            let lrc_path = path.with_extension("lrc");
            if lrc_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&lrc_path) {
                    raw_lyrics = Some(content);
                }
            }
        }

        let parsed_lyrics = if let Some(ref lyrics_text) = raw_lyrics {
            parse_lrc_or_plain_lyrics(lyrics_text)
        } else {
            Vec::new()
        };

        Ok(Self {
            title,
            artist,
            album,
            genre,
            composer,
            year,
            track_number,
            duration_sec: if duration_sec > 0.0 { duration_sec } else { 180.0 },
            bitrate_kbps,
            codec,
            artwork_bytes,
            lyrics: raw_lyrics,
            parsed_lyrics,
        })
    }
}

/// Map lofty's FileType to a friendly codec / format label.
fn file_type_codec(ft: lofty::file::FileType) -> Option<String> {
    use lofty::file::FileType;
    let label = match ft {
        FileType::Mpeg => "MP3",
        FileType::Flac => "FLAC",
        FileType::Mp4 => "M4A",
        FileType::Aac => "AAC",
        FileType::Wav => "WAV",
        FileType::Vorbis => "OGG",
        FileType::Opus => "OPUS",
        FileType::Aiff => "AIFF",
        FileType::Ape => "APE",
        FileType::WavPack => "WV",
        FileType::Mpc => "MPC",
        FileType::Speex => "SPX",
        FileType::Custom(s) => s,
        _ => return None,
    };
    Some(label.to_string())
}

pub fn find_local_cover_art(path: &Path) -> Option<Vec<u8>> {
    if let Some(parent) = path.parent() {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let per_track_cover = parent.join(format!("{}.cover.jpg", stem));
            if let Ok(bytes) = std::fs::read(per_track_cover) {
                return Some(bytes);
            }
        }
        let candidates = [
            "cover.jpg",
            "cover.jpeg",
            "cover.png",
            "cover.webp",
            "folder.jpg",
            "folder.jpeg",
            "folder.png",
            "folder.webp",
            "front.jpg",
            "front.jpeg",
            "front.png",
            "albumart.jpg",
            "albumart.png",
            "album.jpg",
            "album.png",
            "art.jpg",
            "art.png",
        ];
        for cand in candidates {
            let candidate_path = parent.join(cand);
            if candidate_path.is_file() {
                if let Ok(bytes) = std::fs::read(&candidate_path) {
                    return Some(bytes);
                }
            }
        }
    }
    None
}

pub fn parse_lrc_or_plain_lyrics(text: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    let mut is_lrc = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.len() >= 8 {
            if let Some(end_bracket) = trimmed.find(']') {
                let time_str = &trimmed[1..end_bracket];
                if let Some(seconds) = parse_lrc_timestamp(time_str) {
                    let lyric_text = trimmed[end_bracket + 1..].trim().to_string();
                    lines.push(LyricLine {
                        timestamp_sec: seconds,
                        text: lyric_text,
                    });
                    is_lrc = true;
                    continue;
                }
            }
        }
        if !is_lrc && !trimmed.is_empty() {
            lines.push(LyricLine {
                timestamp_sec: 0.0,
                text: trimmed.to_string(),
            });
        }
    }

    if is_lrc {
        lines.sort_by(|a, b| a.timestamp_sec.partial_cmp(&b.timestamp_sec).unwrap());
    }

    lines
}

fn parse_lrc_timestamp(s: &str) -> Option<f32> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let mins: f32 = parts[0].parse().ok()?;
        let secs: f32 = parts[1].parse().ok()?;
        return Some(mins * 60.0 + secs);
    }
    None
}
