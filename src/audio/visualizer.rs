use std::sync::{Arc, Mutex};

/// Number of frequency bands the visualizer renders.
pub const VIZ_BANDS: usize = 16;
/// Samples (mono-mixed) gathered per analysis window.
const WINDOW: usize = 1024;

/// Shared, lock-protected band energies (0.0..1.0+) written by the audio thread.
pub type SharedViz = Arc<Mutex<[f32; VIZ_BANDS]>>;

/// Conversion from any rodio sample type into normalized f32.
trait ToNormalizedF32 {
    fn to_normalized_f32(self) -> f32;
}

impl ToNormalizedF32 for i16 {
    fn to_normalized_f32(self) -> f32 {
        self as f32 / 32768.0
    }
}

impl ToNormalizedF32 for u16 {
    fn to_normalized_f32(self) -> f32 {
        (self as f32 - 32768.0) / 32768.0
    }
}

impl ToNormalizedF32 for f32 {
    fn to_normalized_f32(self) -> f32 {
        self.clamp(-1.0, 1.0)
    }
}

pub fn new_shared_viz() -> SharedViz {
    Arc::new(Mutex::new([0.0; VIZ_BANDS]))
}

/// Log-spaced Goertzel analysis frequencies (~55Hz .. 16kHz).
const BAND_FREQS: [f32; VIZ_BANDS] = [
    55.0, 80.0, 120.0, 180.0, 260.0, 380.0, 550.0, 800.0,
    1150.0, 1700.0, 2500.0, 3600.0, 5200.0, 7600.0, 11500.0, 15500.0,
];

/// Wraps any rodio audio Source and continuously analyzes the passing samples
/// into shared band energies so the on-screen spectrum reacts to real audio.
pub struct VisualizerSource<S> {
    inner: S,
    sample_rate: u32,
    mono: Vec<f32>,
    shared: SharedViz,
}

impl<S> VisualizerSource<S>
where
    S: rodio::Source,
    S::Item: rodio::Sample,
{
    pub fn new(inner: S, shared: SharedViz) -> Self {
        let sample_rate = inner.sample_rate();
        Self {
            inner,
            sample_rate: sample_rate.max(1),
            mono: Vec::with_capacity(WINDOW),
            shared,
        }
    }

    fn analyze_window(&self) {
        let n = self.mono.len();
        if n < WINDOW / 2 {
            return;
        }
        let mut levels = [0.0_f32; VIZ_BANDS];
        // Hann window pre-computed lazily per position
        for (band, freq) in BAND_FREQS.iter().enumerate() {
            let k = 2.0 * std::f32::consts::PI * freq / self.sample_rate as f32;
            let mut coeff_re = 0.0_f32;
            let mut coeff_im = 0.0_f32;
            for (i, &sample) in self.mono.iter().enumerate() {
                let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos();
                let s = sample * w;
                let phase = k * i as f32;
                coeff_re += s * phase.cos();
                coeff_im -= s * phase.sin();
            }
            let mag = (coeff_re * coeff_re + coeff_im * coeff_im).sqrt() / (n as f32 * 0.25);
            // Gentle high-frequency lift so treble bands stay visible
            let lift = 1.0 + band as f32 * 0.12;
            levels[band] = (mag * lift).clamp(0.0, 1.4);
        }
        if let Ok(mut slot) = self.shared.lock() {
            for (slot_v, new_v) in slot.iter_mut().zip(levels.iter()) {
                // Peak-hold with slow decay handled on the render side; here keep max attack
                *slot_v = (*slot_v * 0.35).max(*new_v);
            }
        }
    }
}

impl<S> Iterator for VisualizerSource<S>
where
    S: rodio::Source,
    S::Item: rodio::Sample + ToNormalizedF32,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<S::Item> {
        let sample = self.inner.next()?;
        // Mono mixdown accumulation across channel frames
        self.mono.push(sample.to_normalized_f32());
        if self.mono.len() >= WINDOW {
            self.analyze_window();
            self.mono.clear();
        }
        Some(sample)
    }
}

impl<S> rodio::Source for VisualizerSource<S>
where
    S: rodio::Source,
    S::Item: rodio::Sample + ToNormalizedF32,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }

    // CRITICAL: forward seeks to the wrapped decoder. Without this, the tap
    // makes every Sink::try_seek report "unsupported" and seeking (restart on
    // PREVIOUS press, scrubbing, quiz excerpts) silently stops working.
    fn try_seek(&mut self, pos: std::time::Duration) -> Result<(), rodio::source::SeekError> {
        self.mono.clear();
        self.inner.try_seek(pos)
    }
}
