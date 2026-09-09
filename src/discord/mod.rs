use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiscordDisplayMode {
    AppOnly,
    SongTitle,
    Artist,
    SongAndArtist,
}

impl DiscordDisplayMode {
    #[allow(dead_code)]
    pub const ALL: [DiscordDisplayMode; 4] = [
        DiscordDisplayMode::AppOnly,
        DiscordDisplayMode::SongTitle,
        DiscordDisplayMode::Artist,
        DiscordDisplayMode::SongAndArtist,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            DiscordDisplayMode::AppOnly => "App Only",
            DiscordDisplayMode::SongTitle => "Song Title",
            DiscordDisplayMode::Artist => "Artist",
            DiscordDisplayMode::SongAndArtist => "Song - Artist",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscordTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub lookup_title: String,
    pub lookup_artist: String,
    /// Discord StatusDisplayType: 0 = app name, 1 = state, 2 = details.
    pub status_display_type: u8,
    pub is_playing: bool,
    pub elapsed_sec: f32,
    pub duration_sec: f32,
    pub client_id: String,
}

pub struct DiscordRpc {
    sender: Sender<Option<DiscordTrackInfo>>,
    pub enabled: bool,
    pub display_mode: DiscordDisplayMode,
    pub client_id: String,
}

impl DiscordRpc {
    pub fn new(enabled: bool, display_mode: DiscordDisplayMode, client_id: String) -> Self {
        let (sender, receiver) = channel::<Option<DiscordTrackInfo>>();

        thread::spawn(move || {
            discord_worker_loop(receiver);
        });

        Self {
            sender,
            enabled,
            display_mode,
            client_id,
        }
    }

    pub fn update_presence(&self, track_info: Option<DiscordTrackInfo>) {
        if !self.enabled {
            let _ = self.sender.send(None);
            return;
        }

        if let Some(mut info) = track_info {
            let original_title = info.title.clone();
            let original_artist = info.artist.clone();
            let (details, state_text, status_display_type) = match self.display_mode {
                DiscordDisplayMode::AppOnly => (
                    "Epod Music Player".to_string(),
                    if info.is_playing { "Listening to music" } else { "Paused" }.to_string(),
                    0,
                ),
                DiscordDisplayMode::SongTitle => (
                    original_title.clone(),
                    if info.is_playing { original_artist.clone() } else { format!("{} • Paused", original_artist) },
                    2,
                ),
                DiscordDisplayMode::Artist => (
                    original_title.clone(),
                    if info.is_playing { original_artist.clone() } else { format!("{} • Paused", original_artist) },
                    1,
                ),
                DiscordDisplayMode::SongAndArtist => (
                    format!("{} — {}", original_title, original_artist),
                    if info.is_playing { original_artist.clone() } else { format!("{} • Paused", original_artist) },
                    2,
                ),
            };

            info.lookup_title = original_title;
            info.lookup_artist = original_artist;
            info.title = details;
            info.artist = state_text;
            info.status_display_type = status_display_type;
            info.client_id = self.client_id.clone();

            let _ = self.sender.send(Some(info));
        } else {
            let idle_info = DiscordTrackInfo {
                title: "Epod (5th Gen)".to_string(),
                artist: "Browsing Library".to_string(),
                album: String::new(),
                lookup_title: String::new(),
                lookup_artist: String::new(),
                status_display_type: 0,
                is_playing: false,
                elapsed_sec: 0.0,
                duration_sec: 0.0,
                client_id: self.client_id.clone(),
            };
            let _ = self.sender.send(Some(idle_info));
        }
    }
}

// Discord IPC Worker Loop
fn discord_worker_loop(receiver: Receiver<Option<DiscordTrackInfo>>) {
    let mut current_client_id = String::new();
    let mut current_pipe: Option<std::fs::File> = None;
    let mut last_info: Option<DiscordTrackInfo> = None;
    let mut nonce_counter: u64 = 1;
    let mut art_url_cache: HashMap<(String, String), String> = HashMap::new();

    loop {
        let mut got_update = false;
        while let Ok(msg) = receiver.try_recv() {
            last_info = msg;
            got_update = true;
        }

        let target_client_id = if let Some(ref info) = last_info {
            if !info.client_id.trim().is_empty() {
                info.client_id.trim().to_string()
            } else {
                "1540631275717656648".to_string()
            }
        } else {
            "1540631275717656648".to_string()
        };

        if current_client_id != target_client_id {
            current_pipe = None;
            current_client_id = target_client_id.clone();
        }

        if current_pipe.is_none() {
            current_pipe = try_connect_discord(&current_client_id);
            if current_pipe.is_some() {
                got_update = true;
            }
        }

        if let Some(ref mut pipe) = current_pipe {
            if got_update {
                if let Some(ref info) = last_info {
                    nonce_counter = nonce_counter.wrapping_add(1);

                    // Look up cover art URL
                    let lookup_key = (info.lookup_title.clone(), info.lookup_artist.clone());
                    let art_url = if let Some(cached) = art_url_cache.get(&lookup_key) {
                        if cached.is_empty() { None } else { Some(cached.clone()) }
                    } else if info.is_playing {
                        let fetched = fetch_artwork_url(&info.lookup_title, &info.lookup_artist);
                        art_url_cache.insert(lookup_key, fetched.clone().unwrap_or_default());
                        fetched
                    } else {
                        None
                    };

                    if send_activity_frame(pipe, info, art_url.as_deref(), nonce_counter).is_err() {
                        current_pipe = None;
                    }
                } else {
                    nonce_counter = nonce_counter.wrapping_add(1);
                    if clear_activity_frame(pipe, nonce_counter).is_err() {
                        current_pipe = None;
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}

fn fetch_artwork_url(title: &str, artist: &str) -> Option<String> {
    // Built-in procedural demo tracks
    let t_lower = title.to_lowercase();
    if t_lower.contains("neon skyline") {
        return Some("https://images.unsplash.com/photo-1509198397868-475647b2a1e5?w=500&auto=format&fit=crop".to_string());
    } else if t_lower.contains("summer drive") {
        return Some("https://images.unsplash.com/photo-1469854523086-cc02fe5d8800?w=500&auto=format&fit=crop".to_string());
    } else if t_lower.contains("midnight groove") {
        return Some("https://images.unsplash.com/photo-1511192336575-5a79af67a629?w=500&auto=format&fit=crop".to_string());
    } else if t_lower.contains("digital orbit") {
        return Some("https://images.unsplash.com/photo-1518770660439-4636190af475?w=500&auto=format&fit=crop".to_string());
    } else if t_lower.contains("acoustic horizon") {
        return Some("https://images.unsplash.com/photo-1510915361894-db8b60106cb1?w=500&auto=format&fit=crop".to_string());
    }

    // Clean search terms
    let clean_title = title.split(" - ").next().unwrap_or(title);
    let clean_artist = if artist.starts_with("by ") {
        &artist[3..]
    } else if artist.ends_with(" (Paused)") {
        &artist[..artist.len() - 9]
    } else {
        artist
    };

    let query = format!("{} {}", clean_title, clean_artist);
    let encoded_query = urlencode(&query);

    let mut cmd = Command::new("curl.exe");
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = cmd
        .args([
            "-s",
            "--max-time", "2",
            &format!("https://itunes.apple.com/search?term={}&entity=song&limit=1", encoded_query),
        ])
        .output()
        .ok()?;

    let body = String::from_utf8_lossy(&output.stdout);
    if let Some(pos) = body.find("\"artworkUrl100\":\"") {
        let after = &body[pos + 17..];
        if let Some(end) = after.find('"') {
            let raw_url = &after[..end];
            let hd_url = raw_url.replace("100x100bb.jpg", "512x512bb.jpg");
            return Some(hd_url);
        }
    }

    None
}

fn urlencode(s: &str) -> String {
    s.chars().map(|c| match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        ' ' => "+".to_string(),
        _ => format!("%{:02X}", c as u32),
    }).collect()
}

fn try_connect_discord(client_id: &str) -> Option<std::fs::File> {
    for i in 0..10 {
        let pipe_path = format!(r"\\.\pipe\discord-ipc-{}", i);
        if let Ok(mut file) = OpenOptions::new().read(true).write(true).open(&pipe_path) {
            let handshake_json = format!(r#"{{"v":1,"client_id":"{}"}}"#, client_id);
            if send_ipc_packet(&mut file, 0, &handshake_json).is_ok() {
                let mut header = [0u8; 8];
                if file.read_exact(&mut header).is_ok() {
                    let resp_op = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
                    let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
                    let mut payload = vec![0u8; length];
                    if file.read_exact(&mut payload).is_ok() && resp_op == 1 {
                        return Some(file);
                    }
                }
            }
        }
    }
    None
}

fn send_activity_frame(
    file: &mut std::fs::File,
    info: &DiscordTrackInfo,
    art_url: Option<&str>,
    nonce: u64,
) -> std::io::Result<()> {
    let pid = std::process::id();
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let timestamps_json = if info.is_playing && info.duration_sec > 0.0 {
        let elapsed = info.elapsed_sec.max(0.0) as u64;
        let start_epoch = now_epoch.saturating_sub(elapsed);
        let remaining = (info.duration_sec - info.elapsed_sec).max(0.0) as u64;
        let end_epoch = now_epoch + remaining;
        format!(r#","timestamps":{{"start":{},"end":{}}}"#, start_epoch, end_epoch)
    } else {
        String::new()
    };

    let assets_json = if let Some(url) = art_url {
        format!(r#","assets":{{"large_image":"{}","large_text":"{}"}}"#, escape_json(url), escape_json(&info.album))
    } else {
        r#","assets":{"large_text":"Epod"}"#.to_string()
    };

    let payload = format!(
        r#"{{"cmd":"SET_ACTIVITY","args":{{"pid":{},"activity":{{"name":"Epod","type":2,"status_display_type":{},"details":"{}","state":"{}"{}{}}}}},"nonce":"{}"}}"#,
        pid,
        info.status_display_type,
        escape_json(&info.title),
        escape_json(&info.artist),
        timestamps_json,
        assets_json,
        nonce
    );

    send_ipc_packet(file, 1, &payload)?;

    let mut header = [0u8; 8];
    if file.read_exact(&mut header).is_ok() {
        let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        let mut buf = vec![0u8; length];
        let _ = file.read_exact(&mut buf);
    }

    Ok(())
}

fn clear_activity_frame(file: &mut std::fs::File, nonce: u64) -> std::io::Result<()> {
    let pid = std::process::id();
    let payload = format!(r#"{{"cmd":"SET_ACTIVITY","args":{{"pid":{},"activity":null}},"nonce":"{}"}}"#, pid, nonce);
    send_ipc_packet(file, 1, &payload)?;

    let mut header = [0u8; 8];
    if file.read_exact(&mut header).is_ok() {
        let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        let mut buf = vec![0u8; length];
        let _ = file.read_exact(&mut buf);
    }
    Ok(())
}

fn send_ipc_packet(file: &mut std::fs::File, opcode: u32, json_payload: &str) -> std::io::Result<()> {
    let bytes = json_payload.as_bytes();
    let len = bytes.len() as u32;

    file.write_all(&opcode.to_le_bytes())?;
    file.write_all(&len.to_le_bytes())?;
    file.write_all(bytes)?;
    file.flush()?;
    Ok(())
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', "")
}
