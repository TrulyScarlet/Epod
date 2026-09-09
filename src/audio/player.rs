use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::visualizer::{new_shared_viz, SharedViz, VisualizerSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShuffleMode {
    Off,
    Songs,
    Albums,
}

impl ShuffleMode {
    pub fn name(&self) -> &'static str {
        match self {
            ShuffleMode::Off => "Off",
            ShuffleMode::Songs => "Songs",
            ShuffleMode::Albums => "Albums",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RepeatMode {
    Off,
    One,
    All,
}

impl RepeatMode {
    pub fn name(&self) -> &'static str {
        match self {
            RepeatMode::Off => "Off",
            RepeatMode::One => "One",
            RepeatMode::All => "All",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EqPreset {
    Off,
    Custom,
    Acoustic,
    BassBooster,
    BassReducer,
    Classical,
    Dance,
    Deep,
    Electronic,
    Flat,
    HipHop,
    Jazz,
    Latin,
    Loudness,
    Lounge,
    Piano,
    Pop,
    RnB,
    Rock,
    SmallSpeakers,
    SpokenWord,
    TrebleBooster,
    TrebleReducer,
    VocalBooster,
}

impl EqPreset {
    pub const ALL: [EqPreset; 24] = [
        EqPreset::Off,
        EqPreset::Custom,
        EqPreset::Acoustic,
        EqPreset::BassBooster,
        EqPreset::BassReducer,
        EqPreset::Classical,
        EqPreset::Dance,
        EqPreset::Deep,
        EqPreset::Electronic,
        EqPreset::Flat,
        EqPreset::HipHop,
        EqPreset::Jazz,
        EqPreset::Latin,
        EqPreset::Loudness,
        EqPreset::Lounge,
        EqPreset::Piano,
        EqPreset::Pop,
        EqPreset::RnB,
        EqPreset::Rock,
        EqPreset::SmallSpeakers,
        EqPreset::SpokenWord,
        EqPreset::TrebleBooster,
        EqPreset::TrebleReducer,
        EqPreset::VocalBooster,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            EqPreset::Off => "Off",
            EqPreset::Custom => "Custom (10-Band EQ)",
            EqPreset::Acoustic => "Acoustic",
            EqPreset::BassBooster => "Bass Booster",
            EqPreset::BassReducer => "Bass Reducer",
            EqPreset::Classical => "Classical",
            EqPreset::Dance => "Dance",
            EqPreset::Deep => "Deep",
            EqPreset::Electronic => "Electronic",
            EqPreset::Flat => "Flat",
            EqPreset::HipHop => "Hip-Hop",
            EqPreset::Jazz => "Jazz",
            EqPreset::Latin => "Latin",
            EqPreset::Loudness => "Loudness",
            EqPreset::Lounge => "Lounge",
            EqPreset::Piano => "Piano",
            EqPreset::Pop => "Pop",
            EqPreset::RnB => "R&B",
            EqPreset::Rock => "Rock",
            EqPreset::SmallSpeakers => "Small Speakers",
            EqPreset::SpokenWord => "Spoken Word",
            EqPreset::TrebleBooster => "Treble Booster",
            EqPreset::TrebleReducer => "Treble Reducer",
            EqPreset::VocalBooster => "Vocal Booster",
        }
    }

    pub fn gains(&self, custom_bands: &[f32; 10]) -> (f32, f32, f32) {
        match self {
            EqPreset::Custom => {
                // 10 Bands:
                // 0: 32Hz, 1: 64Hz, 2: 125Hz, 3: 250Hz -> Bass
                // 4: 500Hz, 5: 1kHz, 6: 2kHz -> Midrange
                // 7: 4kHz, 8: 8kHz, 9: 16kHz -> Treble
                let bass_db = (custom_bands[0] * 0.25) + (custom_bands[1] * 0.3) + (custom_bands[2] * 0.25) + (custom_bands[3] * 0.2);
                let mid_db = (custom_bands[4] * 0.3) + (custom_bands[5] * 0.4) + (custom_bands[6] * 0.3);
                let treble_db = (custom_bands[7] * 0.3) + (custom_bands[8] * 0.4) + (custom_bands[9] * 0.3);
                let db_to_linear = |db: f32| 10.0_f32.powf(db / 20.0);
                (db_to_linear(bass_db), db_to_linear(mid_db), db_to_linear(treble_db))
            }
            EqPreset::Off | EqPreset::Flat => (1.0, 1.0, 1.0),
            EqPreset::Acoustic => (1.15, 1.05, 1.25),
            EqPreset::BassBooster => (1.60, 1.0, 0.90),
            EqPreset::BassReducer => (0.55, 1.0, 1.10),
            EqPreset::Classical => (1.20, 0.95, 1.30),
            EqPreset::Dance => (1.45, 1.10, 1.35),
            EqPreset::Deep => (1.50, 0.90, 0.85),
            EqPreset::Electronic => (1.35, 1.15, 1.40),
            EqPreset::HipHop => (1.55, 1.05, 1.15),
            EqPreset::Jazz => (1.10, 1.10, 1.25),
            EqPreset::Latin => (1.15, 1.00, 1.30),
            EqPreset::Loudness => (1.40, 0.85, 1.35),
            EqPreset::Lounge => (1.20, 1.15, 0.95),
            EqPreset::Piano => (1.10, 1.20, 1.25),
            EqPreset::Pop => (1.20, 1.25, 1.15),
            EqPreset::RnB => (1.40, 1.10, 1.20),
            EqPreset::Rock => (1.35, 1.05, 1.40),
            EqPreset::SmallSpeakers => (0.75, 1.30, 1.35),
            EqPreset::SpokenWord => (0.60, 1.40, 0.90),
            EqPreset::TrebleBooster => (0.90, 1.0, 1.55),
            EqPreset::TrebleReducer => (1.10, 1.0, 0.60),
            EqPreset::VocalBooster => (0.80, 1.45, 1.15),
        }
    }
}

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    fading_sink: Option<Sink>,
    crossfade_elapsed_sec: f32,
    crossfade_triggered: bool,
    pub crossfade_seconds: u8,
    pub is_playing: bool,
    pub volume: f32, // 0.0 to 1.0 (32 fine steps = step is 0.03125)
    pub current_time_sec: f32,
    pub total_duration_sec: f32,
    pub eq: EqPreset,
    pub custom_eq_bands: [f32; 10], // 32, 64, 125, 250, 500, 1k, 2k, 4k, 8k, 16k (-12dB to +12dB)
    pub sound_check: bool,
    pub shuffle: ShuffleMode,
    pub repeat: RepeatMode,
    /// Live spectrum band energies (0..~1.4) fed by the real audio analysis tap.
    pub viz_bands: SharedViz,
    last_update: Instant,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("Audio player output init failed: {}", e);
                (None, None)
            }
        };

        Self {
            _stream: stream,
            stream_handle,
            sink: None,
            fading_sink: None,
            crossfade_elapsed_sec: 0.0,
            crossfade_triggered: false,
            crossfade_seconds: 0,
            is_playing: false,
            volume: 0.6875, // 22/32 default volume
            current_time_sec: 0.0,
            total_duration_sec: 0.0,
            eq: EqPreset::Off,
            custom_eq_bands: [0.0; 10],
            sound_check: false,
            shuffle: ShuffleMode::Off,
            repeat: RepeatMode::Off,
            viz_bands: new_shared_viz(),
            last_update: Instant::now(),
        }
    }

    // Perceptual logarithmic volume calculation (extra-steep low end so the
    // first volume bars are whisper-quiet)
    pub fn effective_volume(&self) -> f32 {
        if self.volume <= 0.001 {
            0.0
        } else {
            self.volume.powf(3.4)
        }
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        let (bass_gain, mid_gain, treble_gain) = self.eq.gains(&self.custom_eq_bands);
        let eq_factor = (bass_gain + mid_gain + treble_gain) / 3.0;
        let final_gain = (self.effective_volume() * eq_factor).clamp(0.0, 2.0);
        if let Some(ref sink) = self.sink {
            sink.set_volume(final_gain);
        }
        if let Some(ref sink) = self.fading_sink {
            sink.set_volume(final_gain);
        }
    }

    pub fn play_file(&mut self, path: PathBuf, duration_sec: f32) {
        let mut old_sink = if self.crossfade_seconds > 0 && self.is_playing {
            self.sink.take()
        } else {
            self.stop();
            None
        };
        self.total_duration_sec = duration_sec;
        self.current_time_sec = 0.0;
        self.crossfade_triggered = false;

        if let Some(ref handle) = self.stream_handle {
            match File::open(&path) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    match Decoder::new(reader) {
                        Ok(source) => {
                            match Sink::try_new(handle) {
                                Ok(sink) => {
                                    let (b, m, tr) = self.eq.gains(&self.custom_eq_bands);
                                    let eq_factor = (b + m + tr) / 3.0;
                                    let final_gain = (self.effective_volume() * eq_factor).clamp(0.0, 2.0);
                                    sink.set_volume(if old_sink.is_some() { 0.0 } else { final_gain });
                                    // Tap the decoded stream for live spectrum analysis
                                    sink.append(VisualizerSource::new(source, Arc::clone(&self.viz_bands)));
                                    self.sink = Some(sink);
                                    self.fading_sink = old_sink.take();
                                    self.crossfade_elapsed_sec = 0.0;
                                    self.is_playing = true;
                                    self.last_update = Instant::now();
                                    return;
                                }
                                Err(e) => eprintln!("Failed to create Rodio Sink: {}", e),
                            }
                        }
                        Err(e) => eprintln!("Failed to decode audio file {:?}: {}", path, e),
                    }
                }
                Err(e) => eprintln!("Failed to open audio file {:?}: {}", path, e),
            }
        }
        // If the replacement failed, keep the outgoing song alive.
        if let Some(old) = old_sink {
            self.sink = Some(old);
            self.is_playing = true;
        } else {
            self.is_playing = false;
            self.sink = None;
        }
    }

    pub fn play_synthetic_demo(&mut self, track_idx: usize, duration_sec: f32) {
        self.stop();
        self.total_duration_sec = duration_sec;
        self.current_time_sec = 0.0;

        if let Some(ref handle) = self.stream_handle {
            let samples = generate_demo_music(track_idx, duration_sec, &self.eq, &self.custom_eq_bands);
            let buffer = SamplesBuffer::new(2, 44100, samples);
            if let Ok(sink) = Sink::try_new(handle) {
                let final_gain = (self.effective_volume() * 0.75).clamp(0.0, 2.0);
                sink.set_volume(final_gain);
                sink.append(VisualizerSource::new(buffer, Arc::clone(&self.viz_bands)));
                self.sink = Some(sink);
                self.is_playing = true;
                self.last_update = Instant::now();
                return;
            }
        }

        self.is_playing = false;
        self.sink = None;
    }

    pub fn toggle_play_pause(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.resume();
        }
    }

    pub fn pause(&mut self) {
        if self.is_playing {
            if let Some(ref sink) = self.sink {
                sink.pause();
            }
            self.is_playing = false;
        }
    }

    pub fn resume(&mut self) {
        if !self.is_playing {
            if let Some(ref sink) = self.sink {
                sink.play();
            }
            self.is_playing = true;
            self.last_update = Instant::now();
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        if let Some(sink) = self.fading_sink.take() {
            sink.stop();
        }
        self.crossfade_elapsed_sec = 0.0;
        self.crossfade_triggered = false;
        self.is_playing = false;
        self.current_time_sec = 0.0;
    }

    pub fn seek_to(&mut self, target_sec: f32) {
        let clamped = target_sec.clamp(0.0, self.total_duration_sec);
        if let Some(ref sink) = self.sink {
            if sink.try_seek(Duration::from_secs_f32(clamped)).is_err() {
                return;
            }
        }
        self.current_time_sec = clamped;
        self.last_update = Instant::now();
    }

    pub fn should_start_crossfade(&mut self) -> bool {
        if self.crossfade_seconds == 0
            || self.crossfade_triggered
            || !self.is_playing
            || self.total_duration_sec <= self.crossfade_seconds as f32
        {
            return false;
        }
        if self.current_time_sec >= self.total_duration_sec - self.crossfade_seconds as f32 {
            self.crossfade_triggered = true;
            true
        } else {
            false
        }
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        if self.is_playing {
            if self.fading_sink.is_some() && self.crossfade_seconds > 0 {
                self.crossfade_elapsed_sec += dt;
                let p = (self.crossfade_elapsed_sec / self.crossfade_seconds as f32).clamp(0.0, 1.0);
                let target = self.effective_volume();
                if let Some(ref sink) = self.sink {
                    sink.set_volume(target * p);
                }
                if let Some(ref old) = self.fading_sink {
                    old.set_volume(target * (1.0 - p));
                }
                if p >= 1.0 {
                    if let Some(old) = self.fading_sink.take() {
                        old.stop();
                    }
                    if let Some(ref sink) = self.sink {
                        sink.set_volume(target);
                    }
                }
            }
            self.current_time_sec += dt;
            if self.total_duration_sec > 0.0 && self.current_time_sec >= self.total_duration_sec {
                self.current_time_sec = self.total_duration_sec;
                self.is_playing = false;
                return true;
            }
        }
        false
    }
}

// Procedural musical demo track synthesizer
pub fn generate_demo_music(track_idx: usize, duration_sec: f32, eq: &EqPreset, custom_bands: &[f32; 10]) -> Vec<f32> {
    let sample_rate = 44100;
    let gen_dur = duration_sec.min(120.0);
    let total_samples = (sample_rate as f32 * gen_dur) as usize;
    let mut buffer = Vec::with_capacity(total_samples * 2);

    let (bass_gain, mid_gain, treble_gain) = eq.gains(custom_bands);

    let bpm = match track_idx % 4 {
        0 => 120.0,
        1 => 96.0,
        2 => 140.0,
        _ => 80.0,
    };

    let beat_dur = 60.0 / bpm;
    let midi_to_freq = |m: f32| 440.0 * 2.0_f32.powf((m - 69.0) / 12.0);

    let melody_scale = match track_idx % 4 {
        0 => [60.0, 63.0, 65.0, 67.0, 70.0, 72.0, 75.0, 79.0],
        1 => [60.0, 62.0, 64.0, 67.0, 69.0, 72.0, 74.0, 76.0],
        2 => [58.0, 61.0, 63.0, 65.0, 68.0, 70.0, 73.0, 75.0],
        _ => [57.0, 60.0, 62.0, 64.0, 67.0, 69.0, 72.0, 76.0],
    };

    let bass_scale = match track_idx % 4 {
        0 => [36.0, 39.0, 41.0, 43.0],
        1 => [36.0, 38.0, 40.0, 43.0],
        2 => [34.0, 37.0, 39.0, 41.0],
        _ => [33.0, 36.0, 38.0, 40.0],
    };

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let beat = t / beat_dur;
        let beat_frac = beat.fract();
        let beat_num = beat.floor() as usize;

        // Kick Drum
        let kick_env = (-beat_frac * 28.0).exp();
        let kick_pitch = 140.0 * (-beat_frac * 25.0).exp() + 45.0;
        let kick = (beat_frac * kick_pitch * 2.0 * std::f32::consts::PI).sin() * kick_env * 0.45 * bass_gain;

        // Snare
        let snare_active = (beat_num % 2 == 1) as i32 as f32;
        let snare_env = (-beat_frac * 18.0).exp() * snare_active;
        let snare_noise = ((i * 1103515245 + 12345) as f32 / 2147483648.0 - 0.5) * 0.28 * snare_env * treble_gain;

        // Bassline
        let bass_note = bass_scale[(beat_num / 4) % bass_scale.len()];
        let bass_freq = midi_to_freq(bass_note);
        let bass_env = (-beat_frac * 4.0).exp();
        let bass_wave = ((t * bass_freq * 2.0 * std::f32::consts::PI).sin()
            + 0.5 * (t * bass_freq * 4.0 * std::f32::consts::PI).sin()) * bass_env * 0.25 * bass_gain;

        // Arpeggiated Melody
        let mel_step = (beat * 4.0).floor() as usize;
        let mel_frac = (beat * 4.0).fract();
        let mel_note = melody_scale[mel_step % melody_scale.len()];
        let mel_freq = midi_to_freq(mel_note);
        let mel_env = (-mel_frac * 7.0).exp();
        let mel_wave = (t * mel_freq * 2.0 * std::f32::consts::PI).sin() * mel_env * 0.18 * mid_gain;

        let left = (kick + snare_noise + bass_wave * 0.9 + mel_wave * 1.1).clamp(-1.0, 1.0);
        let right = (kick + snare_noise + bass_wave * 1.1 + mel_wave * 0.9).clamp(-1.0, 1.0);

        buffer.push(left);
        buffer.push(right);
    }

    buffer
}
