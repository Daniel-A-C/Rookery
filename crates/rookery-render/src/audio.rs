//! Opt-in, mute-by-default audio subsystem & procedural soundscapes.
//!
//! Provides ambient soundscapes and subtle interaction SFX for the catalog apps.
//! Strictly silent by default; consumes 0% audio CPU when muted, paused, or occluded.
//! Gracefully handles headless environments / systems without audio hardware.

use std::f32::consts::PI;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use rookery_core::settings::AudioSettings;
use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE: u32 = 44_100;

/// Discrete sound effect cues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundEffect {
    /// Pruning a branch in Living Rookery (crisp woody snip)
    Prune,
    /// Bird chirping or snack scatter in Living Rookery
    BirdChirp,
    /// Planting a sapling seed into the soil
    PlantSeed,
    /// Water ripple or pebble in Koi Pond
    WaterRipple,
    /// Scattering food pellets for koi
    FeedKoi,
    /// Gravitational pulse in Cosmic Nebula
    NebulaPulse,
    /// Shifting celestial color palette
    PaletteCycle,
    /// Shuttle launch thruster burn in Orbital Haven
    ShuttleLaunch,
    /// Constructing / locking an orbital station module
    ModuleConstruct,
    /// Subspace sensor scan & telemetry ping in Orbital Haven
    OrbitalScan,
    /// Puddle splash in Shinjuku Rain
    RainSplash,
    /// Elevated commuter train pass-by in Shinjuku Rain
    TrainPass,
    /// Distant rolling thunder in Shinjuku Rain
    ThunderRumble,
    /// Neon storefront sign click & hum in Shinjuku Rain
    SignToggle,
}

enum AudioCommand {
    UpdateSettings(AudioSettings),
    SetInteractive(bool),
    SetPaused(bool),
    SwitchApp(String),
    PlaySfx(SoundEffect),
    Shutdown,
}

/// Thread-safe, cloneable audio handle for triggering SFX and controlling playback.
#[derive(Clone)]
pub struct AudioEngine {
    tx: Sender<AudioCommand>,
    cached_settings: Arc<Mutex<AudioSettings>>,
    interactive: Arc<AtomicBool>,
    _worker: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl AudioEngine {
    /// Spawn the background audio engine thread with initial settings and active app.
    pub fn new(initial_settings: AudioSettings, initial_app: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        let cached_settings = Arc::new(Mutex::new(initial_settings.clamped()));
        let interactive = Arc::new(AtomicBool::new(false));

        let initial_app = initial_app.to_string();
        let settings_clone = initial_settings.clamped();

        let handle = std::thread::Builder::new()
            .name("rookery-audio".into())
            .spawn(move || {
                run_audio_worker(rx, settings_clone, initial_app);
            })
            .ok();

        Self {
            tx,
            cached_settings,
            interactive,
            _worker: Arc::new(Mutex::new(handle)),
        }
    }

    /// Update audio settings (volume, enabled, direct_mode_only).
    pub fn update_settings(&self, settings: AudioSettings) {
        let clamped = settings.clamped();
        if let Ok(mut g) = self.cached_settings.lock() {
            *g = clamped;
        }
        let _ = self.tx.send(AudioCommand::UpdateSettings(clamped));
    }

    /// Notify the audio system of a Direct Interaction Mode transition.
    pub fn set_interactive(&self, interactive: bool) {
        self.interactive.store(interactive, Ordering::Relaxed);
        let _ = self.tx.send(AudioCommand::SetInteractive(interactive));
    }

    /// Pause or resume audio (e.g. when occluded, display asleep, or user paused).
    pub fn set_paused(&self, paused: bool) {
        let _ = self.tx.send(AudioCommand::SetPaused(paused));
    }

    /// Switch the active ambient soundscape when switching apps.
    pub fn switch_app(&self, app_id: &str) {
        let _ = self.tx.send(AudioCommand::SwitchApp(app_id.to_string()));
    }

    /// Play an instantaneous sound effect.
    pub fn play_sfx(&self, sfx: SoundEffect) {
        let _ = self.tx.send(AudioCommand::PlaySfx(sfx));
    }

    /// Return the currently cached settings.
    pub fn settings(&self) -> AudioSettings {
        self.cached_settings.lock().map(|g| *g).unwrap_or_default()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        // Only trigger shutdown if this is the last reference
        if Arc::strong_count(&self._worker) == 1 {
            let _ = self.tx.send(AudioCommand::Shutdown);
        }
    }
}

struct AudioWorkerState {
    device_sink: Option<MixerDeviceSink>,
    ambient_player: Option<Player>,
    sfx_players: Vec<Player>,
    settings: AudioSettings,
    current_app: String,
    interactive: bool,
    paused: bool,
}

impl AudioWorkerState {
    fn should_play(&self) -> bool {
        if !self.settings.enabled || self.paused {
            return false;
        }
        if self.settings.direct_mode_only && !self.interactive {
            return false;
        }
        true
    }

    fn sync_ambient(&mut self) {
        if !self.should_play() {
            if let Some(ref player) = self.ambient_player {
                player.pause();
            }
            return;
        }

        let Some(ref sink) = self.device_sink else {
            return;
        };

        let player = if let Some(ref p) = self.ambient_player {
            p
        } else {
            let p = Player::connect_new(sink.mixer());
            self.ambient_player = Some(p);
            self.ambient_player.as_ref().unwrap()
        };

        // Target volume: ambient sounds
        let effective_vol = (self.settings.volume * 0.75).clamp(0.0, 1.0);
        player.set_volume(effective_vol);

        if player.empty() {
            let loop_samples = synth_ambient_loop(&self.current_app);
            let channels = NonZeroU16::new(1).unwrap();
            let sample_rate = NonZeroU32::new(SAMPLE_RATE).unwrap();
            let source = SamplesBuffer::new(channels, sample_rate, loop_samples).repeat_infinite();
            player.append(source);
        }

        player.play();
        log::info!(
            "Audio ambient soundscape active: app='{}', volume={effective_vol:.2}",
            self.current_app
        );
    }

    fn change_app(&mut self, new_app: String) {
        if self.current_app == new_app {
            return;
        }
        self.current_app = new_app;
        if let Some(player) = self.ambient_player.take() {
            player.stop();
        }
        self.sync_ambient();
    }

    fn play_sfx(&mut self, sfx: SoundEffect) {
        if !self.should_play() {
            return;
        }
        let Some(ref sink) = self.device_sink else {
            return;
        };

        // Clean up finished players
        self.sfx_players.retain(|p| !p.empty());
        if self.sfx_players.len() >= 8 {
            return;
        }

        let player = Player::connect_new(sink.mixer());
        let sfx_vol = self.settings.volume.clamp(0.0, 1.0);
        player.set_volume(sfx_vol);
        let samples = synth_sfx(sfx);
        let channels = NonZeroU16::new(1).unwrap();
        let sample_rate = NonZeroU32::new(SAMPLE_RATE).unwrap();
        player.append(SamplesBuffer::new(channels, sample_rate, samples));
        player.detach();
        log::debug!("Audio SFX played: {sfx:?}");
    }
}

fn open_audio_sink() -> Option<MixerDeviceSink> {
    // 1. Try default device with system default buffer size (handles macOS CoreAudio limits)
    if let Ok(builder) = DeviceSinkBuilder::from_default_device()
        && let Ok(sink) = builder
            .with_buffer_size(rodio::cpal::BufferSize::Default)
            .open_stream()
    {
        log::info!(
            "Audio device opened with default buffer size: {:?}",
            sink.config()
        );
        return Some(sink);
    }
    // 2. Fall back to open_default_sink
    if let Ok(sink) = DeviceSinkBuilder::open_default_sink() {
        log::info!(
            "Audio device opened with open_default_sink: {:?}",
            sink.config()
        );
        return Some(sink);
    }
    // 3. Fall back to open_sink_or_fallback on default device
    if let Ok(builder) = DeviceSinkBuilder::from_default_device()
        && let Ok(sink) = builder.open_sink_or_fallback()
    {
        log::info!(
            "Audio device opened with fallback config: {:?}",
            sink.config()
        );
        return Some(sink);
    }
    None
}

fn run_audio_worker(
    rx: Receiver<AudioCommand>,
    initial_settings: AudioSettings,
    initial_app: String,
) {
    let device_sink = match open_audio_sink() {
        Some(s) => Some(s),
        None => {
            log::info!("Audio output device unavailable; running audio subsystem in silent mode");
            None
        }
    };

    let mut state = AudioWorkerState {
        device_sink,
        ambient_player: None,
        sfx_players: Vec::new(),
        settings: initial_settings,
        current_app: initial_app,
        interactive: false,
        paused: false,
    };

    // Initialize initial ambient state (stays paused if mute-by-default)
    state.sync_ambient();

    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCommand::UpdateSettings(s) => {
                state.settings = s;
                state.sync_ambient();
            }
            AudioCommand::SetInteractive(inter) => {
                state.interactive = inter;
                state.sync_ambient();
            }
            AudioCommand::SetPaused(p) => {
                state.paused = p;
                state.sync_ambient();
            }
            AudioCommand::SwitchApp(app) => {
                state.change_app(app);
            }
            AudioCommand::PlaySfx(sfx) => {
                state.play_sfx(sfx);
            }
            AudioCommand::Shutdown => break,
        }
    }
}

// -----------------------------------------------------------------------------
// Procedural Sound Synthesis
// -----------------------------------------------------------------------------

/// Synthesize a seamless looping ambient buffer (~3.0 seconds, 44.1 kHz).
pub fn synth_ambient_loop(app_id: &str) -> Vec<f32> {
    let duration_secs = 3.0;
    let total_samples = (duration_secs * SAMPLE_RATE as f64) as usize;
    let mut buffer = vec![0.0f32; total_samples];

    match app_id {
        "living-rookery" => {
            // Wind through pine needles: filtered noise with slow breeze modulation
            let mut lcg: u32 = 1234567;
            let mut filter_state = 0.0f32;
            for (i, sample) in buffer.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let white = ((lcg >> 16) as f32 / 32768.0) - 1.0;

                // Modulate filter cutoff with slow sinusoidal breeze (0.2 Hz & 0.13 Hz)
                let breeze =
                    0.5 + 0.3 * (2.0 * PI * 0.2 * t).sin() + 0.2 * (2.0 * PI * 0.13 * t).sin();
                let alpha = 0.04 + 0.06 * breeze;
                filter_state += alpha * (white - filter_state);

                // Add delicate high-frequency harmonic whistle
                let whistle =
                    0.03 * (2.0 * PI * 680.0 * t).sin() * (2.0 * PI * 0.18 * t).sin().abs();
                *sample = (filter_state * 0.8 + whistle) * 0.3;
            }
        }
        "koi-pond" => {
            // Tranquil water caustics: gentle fluid ripples and bubbling stream
            let mut lcg: u32 = 7654321;
            let mut s1 = 0.0f32;
            let mut s2 = 0.0f32;
            for (i, sample) in buffer.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = ((lcg >> 16) as f32 / 32768.0) - 1.0;

                let flow_mod = 0.5 + 0.5 * (2.0 * PI * 0.25 * t).sin();
                s1 += (0.05 + 0.03 * flow_mod) * (noise - s1);
                s2 += 0.02 * (s1 - s2);

                // Resonant fluid cavity sines
                let ripple1 =
                    0.08 * (2.0 * PI * (340.0 + 20.0 * (2.0 * PI * 0.4 * t).sin()) * t).sin();
                let ripple2 =
                    0.05 * (2.0 * PI * (460.0 + 15.0 * (2.0 * PI * 0.3 * t).cos()) * t).sin();

                *sample = (s2 * 0.5 + ripple1 + ripple2) * 0.25;
            }
        }
        "cosmic-nebula" => {
            // Deep cosmic drone: warm sub-bass and celestial harmonic resonance
            for (i, sample) in buffer.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sub = 0.30 * (2.0 * PI * 45.0 * t).sin();
                let fundamental = 0.40 * (2.0 * PI * 90.0 * t).sin();
                let fifth = 0.20 * (2.0 * PI * 135.0 * t + 0.4).sin();
                let octave = 0.12 * (2.0 * PI * 180.0 * t + 0.8).sin();
                let shimmer =
                    0.06 * (2.0 * PI * (360.0 + 2.0 * (2.0 * PI * 0.15 * t).sin()) * t).sin();

                // Gentle slow undulating envelope
                let breath = 0.85 + 0.15 * (2.0 * PI * 0.1 * t).sin();
                *sample = (sub + fundamental + fifth + octave + shimmer) * breath * 0.25;
            }
        }
        "orbital-haven" => {
            // Space station environmental tone: warm 55 Hz reactor hum + harmonic chord + life-support wash
            let mut lcg: u32 = 9876543;
            let mut s = 0.0f32;
            for (i, sample) in buffer.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = ((lcg >> 16) as f32 / 32768.0) - 1.0;
                s += 0.018 * (noise - s);

                let hum = 0.32 * (2.0 * PI * 55.0 * t).sin();
                let fifth = 0.16 * (2.0 * PI * 82.5 * t + 0.3).sin();
                let octave = 0.12 * (2.0 * PI * 110.0 * t).sin();
                let telemetry =
                    0.03 * (2.0 * PI * 440.0 * t).sin() * (2.0 * PI * 0.33 * t).sin().max(0.0);
                *sample = (s * 0.45 + hum + fifth + octave + telemetry) * 0.22;
            }
        }
        "shinjuku-rain" => {
            // Heavy city downpour: dual-band filtered rain patter + warm awning/city resonance
            let mut lcg: u32 = 2468135;
            let mut low = 0.0f32;
            let mut mid = 0.0f32;
            for (i, sample) in buffer.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let white = ((lcg >> 16) as f32 / 32768.0) - 1.0;
                let gust = 0.78 + 0.22 * (2.0 * PI * 0.16 * t).sin();
                low += 0.025 * (white - low);
                mid += (0.14 * gust) * (white - mid);
                let city_hum = 0.03 * (2.0 * PI * 60.0 * t).sin();
                *sample = ((low * 0.55 + mid * 0.45) * 0.36 + city_hum).clamp(-0.95, 0.95);
            }
        }
        _ => {
            // Default warm ambient room tone
            for (i, sample) in buffer.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let drone =
                    0.35 * (2.0 * PI * 70.0 * t).sin() + 0.20 * (2.0 * PI * 105.0 * t).sin();
                *sample = drone * 0.2;
            }
        }
    }

    // Apply seamless crossfade at loop boundary (0.4s window)
    apply_seamless_crossfade(&mut buffer, (0.4 * SAMPLE_RATE as f64) as usize);
    buffer
}

/// Apply a crossfade to eliminate seams when looping buffer.
fn apply_seamless_crossfade(buffer: &mut [f32], crossfade_len: usize) {
    let len = buffer.len();
    if len <= crossfade_len * 2 {
        return;
    }
    for i in 0..crossfade_len {
        let t = i as f32 / crossfade_len as f32;
        let head_idx = i;
        let tail_idx = len - crossfade_len + i;
        let head_val = buffer[head_idx];
        let tail_val = buffer[tail_idx];
        // Smooth raised-cosine (equal power) blend
        let w = 0.5 * (1.0 - (PI * t).cos());
        let blended = tail_val * (1.0 - w) + head_val * w;
        buffer[head_idx] = blended;
        buffer[tail_idx] = blended;
    }
}

/// Synthesize a discrete sound effect into a PCM buffer.
pub fn synth_sfx(sfx: SoundEffect) -> Vec<f32> {
    match sfx {
        SoundEffect::Prune => {
            // Crisp woody snip: sharp noise transient + wooden click decay (0.12s)
            let len = (0.12 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut lcg: u32 = 42;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let white = ((lcg >> 16) as f32 / 32768.0) - 1.0;

                let noise_env = (-45.0 * t).exp();
                let click = (2.0 * PI * 280.0 * t).sin() * (-65.0 * t).exp();
                *sample = (white * 0.7 * noise_env + click * 0.5) * 0.45;
            }
            out
        }
        SoundEffect::BirdChirp => {
            // Frequency-modulated sweet avian chirp (0.18s)
            let len = (0.18 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut phase = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                // Pitch rises then dips
                let freq =
                    2800.0 + 1200.0 * (PI * (t / 0.18)).sin() + 80.0 * (2.0 * PI * 35.0 * t).sin();
                phase += 2.0 * PI * freq / SAMPLE_RATE as f32;
                let env = (1.0 - (-80.0 * t).exp()) * (-18.0 * t).exp();
                *sample = phase.sin() * env * 0.35;
            }
            out
        }
        SoundEffect::PlantSeed => {
            // Soft earth rustle + harmonic chime (0.25s)
            let len = (0.25 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut lcg: u32 = 88;
            let mut s = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let white = ((lcg >> 16) as f32 / 32768.0) - 1.0;
                s += 0.08 * (white - s);

                let rustle = s * (-20.0 * t).exp();
                let chime1 = (2.0 * PI * 587.33 * t).sin() * (-14.0 * t).exp();
                let chime2 = (2.0 * PI * 880.00 * t).sin() * (-18.0 * t).exp();
                *sample = (rustle * 0.4 + chime1 * 0.3 + chime2 * 0.2) * 0.4;
            }
            out
        }
        SoundEffect::WaterRipple => {
            // Classic gentle water drop "bloop" (0.16s)
            let len = (0.16 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut phase = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let progress = t / 0.16;
                let freq = 420.0 + 460.0 * progress * progress;
                phase += 2.0 * PI * freq / SAMPLE_RATE as f32;
                let env = (1.0 - (-100.0 * t).exp()) * (-22.0 * t).exp();
                *sample = phase.sin() * env * 0.45;
            }
            out
        }
        SoundEffect::FeedKoi => {
            // Three micro-droplets mimicking scattered pellets (0.22s)
            let len = (0.22 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let drops = [
                (0.00f32, 540.0f32),
                (0.05f32, 680.0f32),
                (0.11f32, 610.0f32),
            ];
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let mut sum = 0.0f32;
                for &(start, f_base) in &drops {
                    if t >= start {
                        let dt = t - start;
                        let freq = f_base + 350.0 * (dt / 0.08).min(1.0);
                        let env = (1.0 - (-120.0 * dt).exp()) * (-30.0 * dt).exp();
                        sum += (2.0 * PI * freq * dt).sin() * env;
                    }
                }
                *sample = sum * 0.35;
            }
            out
        }
        SoundEffect::NebulaPulse => {
            // Resonant low gravitational well sweep (0.50s)
            let len = (0.50 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut phase = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let freq = 160.0 - 110.0 * (t / 0.50).sqrt();
                phase += 2.0 * PI * freq / SAMPLE_RATE as f32;
                let raw = phase.sin() + 0.3 * (phase * 2.0).sin();
                let env = (1.0 - (-60.0 * t).exp()) * (-6.0 * t).exp();
                *sample = (raw * env).tanh() * 0.45;
            }
            out
        }
        SoundEffect::PaletteCycle => {
            // Shimmering celestial 4-chord chime (0.35s)
            let len = (0.35 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let freqs = [523.25, 659.25, 783.99, 1046.50];
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let mut val = 0.0f32;
                for (idx, &f) in freqs.iter().enumerate() {
                    let start = idx as f32 * 0.04;
                    if t >= start {
                        let dt = t - start;
                        let env = (1.0 - (-100.0 * dt).exp()) * (-15.0 * dt).exp();
                        val += (2.0 * PI * f * dt).sin() * env;
                    }
                }
                *sample = val * 0.25;
            }
            out
        }
        SoundEffect::ShuttleLaunch => {
            // Low-rumble thruster burn pulse (0.55s)
            let len = (0.55 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut lcg: u32 = 55555;
            let mut filter = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = ((lcg >> 16) as f32 / 32768.0) - 1.0;
                filter += 0.04 * (noise - filter);

                let freq = 60.0 + 90.0 * (t / 0.55);
                let sine = (2.0 * PI * freq * t).sin();
                let env = (1.0 - (-40.0 * t).exp()) * (-5.0 * t).exp();
                *sample = (sine * 0.5 + filter * 0.5) * env * 0.4;
            }
            out
        }
        SoundEffect::ModuleConstruct => {
            // Solid mechanical docking latch clunk (0.28s)
            let len = (0.28 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let thud = (2.0 * PI * 85.0 * t).sin() * (-35.0 * t).exp();
                let latch = (2.0 * PI * 720.0 * t).sin() * (-18.0 * t).exp();
                let sub = (2.0 * PI * 42.0 * t).sin() * (-20.0 * t).exp();
                *sample = (thud * 0.6 + latch * 0.3 + sub * 0.3) * 0.45;
            }
            out
        }
        SoundEffect::OrbitalScan => {
            // Crystalline subspace radar sonar ping + harmonic tail (0.38s)
            let len = (0.38 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let ping1 = (2.0 * PI * 880.0 * t).sin() * (-14.0 * t).exp();
                let ping2 = if t > 0.07 {
                    let dt = t - 0.07;
                    (2.0 * PI * 1318.5 * dt).sin() * (-12.0 * dt).exp()
                } else {
                    0.0
                };
                let sub = (2.0 * PI * 220.0 * t).sin() * (-10.0 * t).exp();
                *sample = (ping1 * 0.45 + ping2 * 0.40 + sub * 0.25) * 0.38;
            }
            out
        }
        SoundEffect::RainSplash => {
            let len = (0.18 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut lcg: u32 = 13579;
            let mut lp = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let white = ((lcg >> 16) as f32 / 32768.0) - 1.0;
                lp += 0.18 * (white - lp);
                let env = (-24.0 * t).exp();
                let plink =
                    (2.0 * PI * (680.0 + 380.0 * (-28.0 * t).exp()) * t).sin() * (-30.0 * t).exp();
                *sample = ((lp * 0.55 + plink * 0.35) * env * 0.45).clamp(-0.95, 0.95);
            }
            out
        }
        SoundEffect::TrainPass => {
            let len = (0.75 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut lcg: u32 = 97531;
            let mut lp = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let white = ((lcg >> 16) as f32 / 32768.0) - 1.0;
                lp += 0.045 * (white - lp);
                let env = (PI * (t / 0.75)).sin();
                let clack = (2.0 * PI * 9.5 * t).sin().powi(8);
                let rumble = 0.25 * (2.0 * PI * 72.0 * t).sin();
                *sample = ((lp * 0.5 + rumble + lp * clack * 0.35) * env * 0.38).clamp(-0.95, 0.95);
            }
            out
        }
        SoundEffect::ThunderRumble => {
            let len = (1.1 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            let mut lcg: u32 = 54321;
            let mut lp = 0.0f32;
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let white = ((lcg >> 16) as f32 / 32768.0) - 1.0;
                lp += 0.015 * (white - lp);
                let env = (1.0 - (-18.0 * t).exp()) * (-2.6 * t).exp();
                let sub = 0.30 * (2.0 * PI * 42.0 * t).sin();
                *sample = ((lp * 1.4 + sub) * env * 0.42).clamp(-0.95, 0.95);
            }
            out
        }
        SoundEffect::SignToggle => {
            let len = (0.14 * SAMPLE_RATE as f64) as usize;
            let mut out = vec![0.0f32; len];
            for (i, sample) in out.iter_mut().enumerate() {
                let t = i as f32 / SAMPLE_RATE as f32;
                let click = (2.0 * PI * 1100.0 * t).sin() * (-55.0 * t).exp();
                let hum = (2.0 * PI * 120.0 * t).sin() * (-16.0 * t).exp();
                *sample = ((click * 0.35 + hum * 0.25) * 0.4).clamp(-0.95, 0.95);
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_sfx_buffers_are_valid() {
        let all_sfx = [
            SoundEffect::Prune,
            SoundEffect::BirdChirp,
            SoundEffect::PlantSeed,
            SoundEffect::WaterRipple,
            SoundEffect::FeedKoi,
            SoundEffect::NebulaPulse,
            SoundEffect::PaletteCycle,
            SoundEffect::ShuttleLaunch,
            SoundEffect::ModuleConstruct,
            SoundEffect::OrbitalScan,
            SoundEffect::RainSplash,
            SoundEffect::TrainPass,
            SoundEffect::ThunderRumble,
            SoundEffect::SignToggle,
        ];

        for sfx in all_sfx {
            let buffer = synth_sfx(sfx);
            assert!(!buffer.is_empty(), "buffer for {sfx:?} should not be empty");
            for &sample in &buffer {
                assert!(!sample.is_nan(), "sample is NaN for {sfx:?}");
                assert!(!sample.is_infinite(), "sample is infinite for {sfx:?}");
                assert!(
                    (-1.0..=1.0).contains(&sample),
                    "sample {sample} clipped out of [-1.0, 1.0] for {sfx:?}"
                );
            }
        }
    }

    #[test]
    fn ambient_loops_are_seamless_and_valid() {
        let apps = [
            "living-rookery",
            "koi-pond",
            "cosmic-nebula",
            "orbital-haven",
            "shinjuku-rain",
            "unknown-app",
        ];

        for app in apps {
            let loop_buf = synth_ambient_loop(app);
            assert!(
                !loop_buf.is_empty(),
                "ambient loop for {app} should not be empty"
            );
            for &sample in &loop_buf {
                assert!(!sample.is_nan(), "sample is NaN for ambient {app}");
                assert!(
                    (-1.0..=1.0).contains(&sample),
                    "ambient sample out of bounds for {app}"
                );
            }
            // Check boundary smoothness: start and end samples should be close
            let first = loop_buf.first().copied().unwrap_or(0.0);
            let last = loop_buf.last().copied().unwrap_or(0.0);
            assert!(
                (first - last).abs() < 0.05,
                "loop discontinuity for {app}: first={first}, last={last}"
            );
        }
    }

    #[test]
    fn test_device_sink_open() {
        let Some(mut sink) = open_audio_sink() else {
            // Headless CI environment without physical audio output device
            return;
        };
        sink.log_on_drop(false);
        let player = Player::connect_new(sink.mixer());
        player.set_volume(0.5);
        let samples = synth_sfx(SoundEffect::BirdChirp);
        let channels = NonZeroU16::new(1).unwrap();
        let sample_rate = NonZeroU32::new(SAMPLE_RATE).unwrap();
        player.append(SamplesBuffer::new(channels, sample_rate, samples));
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_ambient_loop_playing_in_engine() {
        let settings = AudioSettings {
            enabled: true,
            volume: 0.8,
            ..Default::default()
        };
        let engine = AudioEngine::new(settings, "koi-pond");
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(engine.settings().enabled);
    }

    #[test]
    fn audio_engine_initializes_without_panic() {
        let settings = AudioSettings::default();
        let engine = AudioEngine::new(settings, "living-rookery");
        assert_eq!(engine.settings(), settings);

        // Muted by default, so sfx and app switching are no-ops that never panic
        engine.play_sfx(SoundEffect::BirdChirp);
        engine.switch_app("koi-pond");
        engine.set_interactive(true);
        engine.set_paused(true);

        let mut updated = settings;
        updated.volume = 0.8;
        engine.update_settings(updated);
        assert_eq!(engine.settings().volume, 0.8);
    }
}
