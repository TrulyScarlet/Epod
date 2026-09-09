use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle, Sink};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClickerSetting {
    Speaker,
    Headphones,
    Both,
    Off,
}

impl ClickerSetting {
    pub fn name(&self) -> &'static str {
        match self {
            ClickerSetting::Speaker => "Speaker",
            ClickerSetting::Headphones => "Headphones",
            ClickerSetting::Both => "Both",
            ClickerSetting::Off => "Off",
        }
    }
}

pub struct ClickerAudio {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    click_samples: Vec<f32>,
    button_click_samples: Vec<f32>,
}

impl ClickerAudio {
    pub fn new() -> Self {
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("Clicker audio output init failed: {}", e);
                (None, None)
            }
        };

        // Generate authentic iPod rotary click impulse (~4ms damped high-frequency tick)
        let sample_rate = 44100;
        let num_samples = (sample_rate as f32 * 0.0045) as usize; // 4.5ms
        let mut click_samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let decay = (-t * 1200.0).exp();
            let wave = (t * 2800.0 * 2.0 * std::f32::consts::PI).sin() * 0.7
                + (t * 5200.0 * 2.0 * std::f32::consts::PI).sin() * 0.3;
            click_samples.push(wave * decay * 0.45);
        }

        // Slightly deeper click for physical button press (~7ms)
        let btn_samples_len = (sample_rate as f32 * 0.007) as usize;
        let mut button_click_samples = Vec::with_capacity(btn_samples_len);
        for i in 0..btn_samples_len {
            let t = i as f32 / sample_rate as f32;
            let decay = (-t * 800.0).exp();
            let wave = (t * 1800.0 * 2.0 * std::f32::consts::PI).sin() * 0.7
                + (t * 3600.0 * 2.0 * std::f32::consts::PI).sin() * 0.3;
            button_click_samples.push(wave * decay * 0.6);
        }

        Self {
            _stream: stream,
            stream_handle,
            click_samples,
            button_click_samples,
        }
    }

    pub fn play_rotary_tick(&self, setting: ClickerSetting) {
        if setting == ClickerSetting::Off {
            return;
        }
        if let Some(handle) = &self.stream_handle {
            let buffer = SamplesBuffer::new(1, 44100, self.click_samples.clone());
            if let Ok(sink) = Sink::try_new(handle) {
                sink.set_volume(0.35);
                sink.append(buffer);
                sink.detach();
            }
        }
    }

    pub fn play_button_click(&self, setting: ClickerSetting) {
        if setting == ClickerSetting::Off {
            return;
        }
        if let Some(handle) = &self.stream_handle {
            let buffer = SamplesBuffer::new(1, 44100, self.button_click_samples.clone());
            if let Ok(sink) = Sink::try_new(handle) {
                sink.set_volume(0.5);
                sink.append(buffer);
                sink.detach();
            }
        }
    }
}
