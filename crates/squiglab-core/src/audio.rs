//! Sound tools: a continuous tone generator, pink noise, and white noise.
//!
//! The reference tool drives a Web Audio `OscillatorNode`: one continuous
//! oscillator whose pitch is dragged along a log-mapped frequency slider, not
//! an automated sweep, with a pink-noise source beside it. That is what
//! [`Generator`] renders — the generator is plain sample arithmetic,
//! independent of where the samples go.
//!
//! Three backends carry those samples to a speaker:
//!
//! - **Linux** speaks to the PipeWire daemon directly through `libpipewire`
//!   ([`pipewire`]), because that is the audio server Linux actually uses and
//!   the one asked for; it is not ALSA, and there is no helper process in
//!   between.
//! - **macOS and Windows** go through cpal ([`cpal_sink`]), which reaches
//!   CoreAudio and WASAPI respectively.
//! - Everywhere else, [`Player::start`] reports that sound is not wired up
//!   rather than failing silently.
//!
//! Everything in the generator is deterministic and testable without an audio
//! device: the tests below check the pitch, the glide, and the noise's
//! spectrum, not that a sound came out. The backends are tested against a real
//! daemon where one is present — see `tests/pipewire_playback.rs`.

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod cpal_sink;
#[cfg(target_os = "linux")]
pub mod pipewire;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use std::sync::mpsc::{Receiver, Sender};

/// The sample rate the generators run at. It matches the equalizer DSP's rate,
/// so a tone and the curve it is auditioned against sit on the same grid.
pub const SAMPLE_RATE: u32 = 48_000;

/// Frames rendered per pass to the player. At 48 kHz this is about 11 ms,
/// short enough that a drag of the frequency slider reaches the speaker
/// promptly.
pub const BUFFER_FRAMES: usize = 512;

/// How long a tone takes to reach full volume when it starts or stops, in
/// seconds. Without it the output clicks on every play and stop.
const FADE_SECONDS: f32 = 0.01;

/// How long the pitch takes to follow a dragged frequency, in seconds. The
/// reference glides rather than jumping; this is the time constant of that
/// glide.
const GLIDE_SECONDS: f64 = 0.05;

/// What the generator is playing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Source {
    /// A continuous sine at a frequency, the reference's tone generator.
    Tone {
        /// The frequency being aimed at, in Hz. The generator glides toward it.
        freq: f64,
    },
    /// Pink noise: equal energy per octave, the reference's noise source.
    PinkNoise,
    /// White noise: equal energy per hertz. Not in the reference's sound
    /// tools, but the generator that pink noise is filtered from, and the
    /// honest way to hear what the pink filter is doing.
    WhiteNoise,
}

impl Source {
    /// Name for the control that selects this source.
    pub fn name(self) -> &'static str {
        match self {
            Self::Tone { .. } => "Tone generator",
            Self::PinkNoise => "Pink noise",
            Self::WhiteNoise => "White noise",
        }
    }

    /// Whether this source has a frequency to drag.
    pub fn is_tone(self) -> bool {
        matches!(self, Self::Tone { .. })
    }
}

/// One half of the Compare A | B pair: a source and how loud it plays. The
/// reference's Compare toggle applies to the whole sound-tools section rather
/// than to one control, so it switches between two of these.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Settings {
    pub source: Source,
    /// Playback volume, 0 to 1.
    pub volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source: Source::Tone { freq: 1000.0 },
            volume: 0.2,
        }
    }
}

/// The filters a source is played through.
///
/// The reference plays its sound through the parametric equalizer, so the
/// bands can be heard doing what they say, and a switch chooses between the
/// curve as it is and the curve as the bands shape it. The shared filters
/// shape both channels; the left and right banks shape one side each.
#[derive(Clone, Debug, Default)]
pub struct Chain {
    /// Applied to every channel.
    pub shared: Vec<crate::equalizer::Biquad>,
    /// Applied to the left channel only.
    pub left: Vec<crate::equalizer::Biquad>,
    /// Applied to the right channel only.
    pub right: Vec<crate::equalizer::Biquad>,
}

impl Chain {
    /// Whether anything runs through.
    pub fn is_empty(&self) -> bool {
        self.shared.is_empty() && self.left.is_empty() && self.right.is_empty()
    }
}

/// Renders the generators into interleaved stereo sample buffers.
///
/// One generator runs for the life of a playback session: its oscillator
/// phase, its glide, and its noise state all carry across buffers, which is
/// what makes a tone continuous rather than a series of beeps.
pub struct Generator {
    sample_rate: f64,
    /// Oscillator position, in turns.
    phase: f64,
    /// The pitch currently sounding, as the natural log of Hz, smoothed toward
    /// the target so a dragged slider glides.
    pitch: f64,
    /// The gain envelope, smoothed so starting and stopping do not click.
    gain: f32,
    pink: PinkNoise,
    /// The white noise the pink filter is fed from, kept so the two sources
    /// share one sequence rather than sounding like different noises.
    white: White,
    /// Per-sample smoothing coefficients, derived from the time constants.
    glide_coefficient: f64,
    fade_coefficient: f32,
    /// The filters the source is played through, and their state.
    chain: Chain,
}

impl Generator {
    /// A generator with a fresh oscillator and noise state.
    pub fn new(sample_rate: u32) -> Self {
        Self::seeded(sample_rate, 0x9e37_79b9)
    }

    /// A generator with a chosen noise seed, so a test can assert on the
    /// spectrum it produces.
    pub fn seeded(sample_rate: u32, seed: u32) -> Self {
        let sample_rate = f64::from(sample_rate);
        Self {
            sample_rate,
            phase: 0.0,
            pitch: (1000.0_f64).ln(),
            gain: 0.0,
            pink: PinkNoise::new(seed),
            white: White::new(seed),
            glide_coefficient: smoothing(sample_rate, GLIDE_SECONDS),
            fade_coefficient: smoothing(sample_rate, f64::from(FADE_SECONDS)) as f32,
            chain: Chain::default(),
        }
    }

    /// Fill `buffer` with interleaved stereo frames, advancing the generator by
    /// its length. The source is mono, so both channels start with the same
    /// sample, and the filters then shape each side as the reference's do.
    pub fn render(&mut self, buffer: &mut [f32], settings: &Settings) {
        for frame in buffer.as_chunks_mut::<2>().0 {
            let sample = self.next_sample(settings);
            let mut left = f64::from(sample);
            let mut right = f64::from(sample);
            let chain = &mut self.chain;
            for filter in &mut chain.shared {
                left = filter.process(left);
                right = filter.process(right);
            }
            for filter in &mut chain.left {
                left = filter.process(left);
            }
            for filter in &mut chain.right {
                right = filter.process(right);
            }
            frame[0] = left as f32;
            frame[1] = right as f32;
        }
    }

    /// Change the filters the source is played through.
    pub fn set_chain(&mut self, chain: Chain) {
        self.chain = chain;
    }

    /// The next sample.
    fn next_sample(&mut self, settings: &Settings) -> f32 {
        let target_gain = if settings.volume > 0.0 { 1.0 } else { 0.0 };
        self.gain += (target_gain - self.gain) * self.fade_coefficient;
        let volume = settings.volume.clamp(0.0, 1.0) * self.gain;
        // The oscillator's state advances whether or not anything is audible,
        // so resuming plays at the pitch it was aimed at rather than gliding up
        // from where it stopped.
        self.advance_pitch(settings);
        if volume <= 0.0 {
            return 0.0;
        }
        let sample = match settings.source {
            Source::Tone { .. } => (std::f64::consts::TAU * self.phase).sin() as f32,
            Source::PinkNoise => self.pink.next_sample(),
            Source::WhiteNoise => self.white.next_sample(),
        };
        (sample * volume).clamp(-1.0, 1.0)
    }

    /// Move the oscillator's pitch toward the target and step its position.
    fn advance_pitch(&mut self, settings: &Settings) {
        let target = match settings.source {
            Source::Tone { freq } => freq.clamp(20.0, 20_000.0).ln(),
            // Noise has no pitch, but the state is kept moving so a switch back
            // to a tone resumes rather than gliding.
            Source::PinkNoise | Source::WhiteNoise => self.pitch,
        };
        self.pitch += (target - self.pitch) * self.glide_coefficient;
        let increment = self.pitch.exp() / self.sample_rate;
        self.phase = (self.phase + increment).fract();
    }
}

/// Per-sample smoothing coefficient for a time constant in seconds: the
/// fraction of the remaining distance closed each sample.
fn smoothing(sample_rate: f64, seconds: f64) -> f64 {
    1.0 - (-1.0 / (seconds * sample_rate)).exp()
}

/// Pink noise by Paul Kellet's economy method: six decaying one-pole filters
/// summed, which spreads white noise evenly across the spectrum at a fraction
/// of the cost of an FFT. This is the filter the Web Audio pink-noise examples
/// use.
pub struct PinkNoise {
    b0: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
    b6: f32,
    /// Running mean of the output, subtracted so the noise has no DC offset.
    /// Kellet's filter leaves a small constant, which is inaudible in playback
    /// but would make a measured mean drift.
    mean: f32,
    white: White,
}

impl PinkNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            b3: 0.0,
            b4: 0.0,
            b5: 0.0,
            b6: 0.0,
            mean: 0.0,
            white: White(seed),
        }
    }

    /// The next sample, in roughly the same range as the white input.
    pub fn next_sample(&mut self) -> f32 {
        let w = self.white.next_sample();
        self.b0 = 0.99886 * self.b0 + w * 0.0555179_f32;
        self.b1 = 0.99332 * self.b1 + w * 0.0750759;
        self.b2 = 0.96900 * self.b2 + w * 0.153852_f32;
        self.b3 = 0.86650 * self.b3 + w * 0.3104856;
        self.b4 = 0.55000 * self.b4 + w * 0.5329522;
        self.b5 = -0.7616 * self.b5 - w * 0.0168980_f32;
        let pink =
            self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + w * 0.5362_f32;
        self.b6 = w * 0.115926_f32;
        // The sum runs well over one, so scale it back to the white input's
        // range and leave the caller to apply the volume.
        let mut sample = pink * 0.11_f32;
        // Subtract a slow running mean to remove the filter's DC offset, then
        // let the mean follow the corrected signal so it stays near zero.
        self.mean += sample * DC_BLOCK_COEFFICIENT;
        sample -= self.mean;
        sample
    }
}

/// A white-noise source: a 32-bit xorshift, which is cheap, has no periodicity
/// in practice, and is deterministic from its seed so a test can assert on the
/// noise's spectrum.
pub struct White(u32);

impl White {
    pub fn new(seed: u32) -> Self {
        // Zero is a fixed point of xorshift, so a zero seed would emit silence
        // forever.
        Self(if seed == 0 { 0x9e37_79b9 } else { seed })
    }

    /// The next sample, spread across -1 to 1.
    pub fn next_sample(&mut self) -> f32 {
        let mut state = self.0;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.0 = state;
        // Map the full 32-bit range onto -1 to 1, so the full 32-bit pattern
        // space is used rather than a truncated one.
        (f64::from(state) / f64::from(u32::MAX) * 2.0 - 1.0) as f32
    }
}

/// How fast the pink-noise DC blocker follows its input, per sample. A slow
/// follower passes the audible band untouched and removes only the constant.
const DC_BLOCK_COEFFICIENT: f32 = 1.0 / 4_000.0;

/// A single-producer, single-consumer ring of interleaved stereo samples,
/// shared between the generator thread that fills it and the audio callback
/// that drains it.
///
/// The two ends are different threads, and the consumer is a real-time one that
/// must not block or allocate. So the ring is a fixed allocation, both ends
/// advance their own index with atomics, and there is no lock anywhere.
///
/// Only one thread ever writes `write` and one ever writes `read`, so the
/// indices do not need to be a pair of atomics guarding each other; the
/// `Acquire`/`Release` pair on each is what makes the samples behind an index
/// visible to the reader.
#[cfg(any(target_os = "macos", target_os = "windows"))]
struct SampleRing {
    buffer: std::sync::Arc<Vec<f32>>,
    /// Index the producer writes next. Only the producer advances this.
    write: std::sync::atomic::AtomicUsize,
    /// Index the consumer reads next. Only the consumer advances this.
    read: std::sync::atomic::AtomicUsize,
    capacity: usize,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl SampleRing {
    /// A ring holding `frames` stereo frames.
    fn new(frames: usize) -> Self {
        assert!(frames > 0, "a ring needs room for at least one frame");
        Self {
            buffer: std::sync::Arc::new(vec![0.0; frames * 2]),
            write: std::sync::atomic::AtomicUsize::new(0),
            read: std::sync::atomic::AtomicUsize::new(0),
            capacity: frames,
        }
    }

    /// How many frames are waiting to be read.
    fn pending(&self) -> usize {
        self.write
            .load(std::sync::atomic::Ordering::Acquire)
            .wrapping_sub(self.read.load(std::sync::atomic::Ordering::Acquire))
    }

    /// The producing end: hand over a rendered buffer.
    ///
    /// When the ring is full this sleeps until the consumer has caught up,
    /// which is what paces the generator to the device's rate. Dropping samples
    /// instead would click, and spinning on `yield_now` would burn a core for
    /// as long as the tone plays.
    fn push(&self, samples: &[f32]) {
        use std::sync::atomic::Ordering;
        let frames = samples.len() / 2;
        assert!(
            frames <= self.capacity,
            "a push larger than the ring would overwrite itself"
        );
        while self.pending() + frames > self.capacity {
            std::thread::sleep(std::time::Duration::from_micros(200));
        }
        let write = self.write.load(Ordering::Relaxed);
        for frame in 0..frames {
            let index = (write + frame) % self.capacity * 2;
            // These are the only writes to the ring's storage. The consumer
            // only reads a slot after seeing the write index pass it, which
            // the release below orders, so a plain store is enough for the
            // values themselves.
            unsafe {
                *self.buffer.as_ptr().cast_mut().add(index) = samples[frame * 2];
                *self.buffer.as_ptr().cast_mut().add(index + 1) = samples[frame * 2 + 1];
            }
        }
        self.write
            .store(write.wrapping_add(frames), Ordering::Release);
    }

    /// The consuming end: copy up to `out.len() / 2` pending frames into
    /// `out`, oldest first, and return how many frames were copied. Anything
    /// the ring does not have is left as silence.
    ///
    /// Safe to call from a real-time thread: it only reads two atomics and
    /// copies, and never waits.
    fn drain(&self, out: &mut [f32]) -> usize {
        use std::sync::atomic::Ordering;
        let write = self.write.load(Ordering::Acquire);
        let read = self.read.load(Ordering::Relaxed);
        let frames = write
            .wrapping_sub(read)
            .min(out.len() / 2)
            .min(self.capacity);
        for frame in 0..frames {
            let index = (read + frame) % self.capacity * 2;
            out[frame * 2] = self.buffer[index];
            out[frame * 2 + 1] = self.buffer[index + 1];
        }
        self.read
            .store(read.wrapping_add(frames), Ordering::Release);
        frames
    }
}

/// What the player thread is told to do.
enum Command {
    Settings(Settings),
    Chain(Chain),
    Stop,
}

/// A playing generator, streamed to the audio device on a background thread.
/// Dropping it stops the sound.
pub struct Player {
    commands: Option<Sender<Command>>,
    reports: Receiver<String>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Signalled when the backend thread has finished, so `stop` can tell a
    /// prompt exit from one that had to be forced.
    done: Option<Receiver<()>>,
    /// Breaks a backend whose stream has stopped calling back — a device that
    /// disappeared, say — where the ordinary stop would otherwise wait for a
    /// callback that will never come.
    force_quit: Option<Box<dyn Fn() + Send + Sync>>,
    /// The cpal stream, kept on the thread that opened it. cpal's streams are
    /// not `Send`: WASAPI and CoreAudio drive them from the thread that created
    /// them, so the sink cannot be moved onto the generator thread. It is
    /// dropped once the player stops, which is what silences the device.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    sink: Option<cpal::Stream>,
}

impl Player {
    /// Start playing, and report why not if the audio path is unavailable.
    #[cfg(target_os = "linux")]
    pub fn start(settings: Settings) -> Result<Self, String> {
        crate::audio::pipewire::start(settings)
    }

    /// Start playing on macOS or Windows, through CoreAudio or WASAPI.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn start(settings: Settings) -> Result<Self, String> {
        crate::audio::cpal_sink::start(settings)
    }

    /// Playback is not implemented on this platform: the generators render, but
    /// only Linux, macOS, and Windows have a device backend.
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    pub fn start(_settings: Settings) -> Result<Self, String> {
        Err("Sound is not wired up on this platform yet.".to_owned())
    }

    /// Change what is playing. A new frequency is glided to rather than
    /// jumped to, which is what the reference's dragged slider does.
    pub fn set_settings(&self, settings: Settings) {
        if let Some(commands) = &self.commands {
            let _ = commands.send(Command::Settings(settings));
        }
    }

    /// Change the filters the source is played through.
    ///
    /// The chain carries the state of the filters, so it is sent whole: the
    /// filters start again from silence, which is a click the reference's
    /// live equalizer also has when the bands are changed under it.
    pub fn set_chain(&self, chain: Chain) {
        if let Some(commands) = &self.commands {
            let _ = commands.send(Command::Chain(chain));
        }
    }

    /// A message from the player, if it has reported one. Called once a frame.
    pub fn take_message(&mut self) -> Option<String> {
        self.reports.try_recv().ok()
    }

    /// Stop playing and wait for the stream to close.
    ///
    /// A backend that has already gone quiet is nudged rather than waited on:
    /// if the thread has not finished shortly after the stop request, the
    /// backend's force-quit hook is called, which is what a lost device would
    /// otherwise turn into a frozen interface.
    pub fn stop(&mut self) {
        if let Some(commands) = self.commands.take() {
            let _ = commands.send(Command::Stop);
        }
        if let Some(thread) = self.thread.take() {
            let finished = match self.done.take() {
                Some(done) => done
                    .recv_timeout(std::time::Duration::from_millis(500))
                    .is_ok(),
                // No completion signal means the backend is expected to stop on
                // its own, as cpal does.
                None => true,
            };
            if !finished {
                if let Some(force_quit) = self.force_quit.take() {
                    force_quit();
                }
            }
            let _ = thread.join();
        }
        self.force_quit = None;
        // The sink goes last: the device has to keep draining the ring until the
        // generator thread has returned, or a `push` waiting for room would
        // never be released.
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self.sink = None;
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(generator: &mut Generator, settings: &Settings, frames: usize) -> Vec<f32> {
        let mut buffer = vec![0.0; frames * 2];
        generator.render(&mut buffer, settings);
        buffer
    }

    /// The frequency of a rendered tone, counted by upward zero crossings,
    /// which is how the ear tracks a sine.
    fn measure_frequency(samples: &[f32]) -> f64 {
        let mut crossings = 0usize;
        let mut previous = samples[0];
        for &sample in &samples[1..] {
            if previous <= 0.0 && sample > 0.0 {
                crossings += 1;
            }
            previous = sample;
        }
        crossings as f64 * f64::from(SAMPLE_RATE) / samples.len() as f64
    }

    /// The ratio of high-frequency energy to low-frequency energy in a signal,
    /// split by a one-pole low-pass filter with the given corner frequency.
    ///
    /// This measures the tilt of a signal without needing an absolute
    /// calibration of the generator, which is what the noise's defining
    /// property is: a flat signal and a bass-heavy one have the same total
    /// energy and differ only in how it is spread.
    fn high_to_low_energy(samples: &[f32], corner_hz: f64) -> f64 {
        let coefficient = (-std::f64::consts::TAU * corner_hz / f64::from(SAMPLE_RATE)).exp();
        let (mut low_pass, mut high_energy, mut low_energy) = (0.0, 0.0, 0.0);
        for &sample in samples {
            let value = f64::from(sample);
            low_pass = coefficient * low_pass + (1.0 - coefficient) * value;
            high_energy += (value - low_pass) * (value - low_pass);
            low_energy += low_pass * low_pass;
        }
        high_energy / low_energy
    }

    /// Render one full second at full volume, discarding the first quarter
    /// second so the fade-in and glide have settled, then return the rest. A
    /// full second is what makes a zero-crossing count resolve a pitch to
    /// within a hertz.
    fn settled(generator: &mut Generator, settings: &Settings) -> Vec<f32> {
        let loud = Settings {
            source: settings.source,
            volume: 1.0,
        };
        render(generator, &loud, SAMPLE_RATE as usize / 4);
        render(generator, settings, SAMPLE_RATE as usize * 3 / 4)
    }

    #[test]
    fn a_tone_plays_the_frequency_it_is_aimed_at() {
        for frequency in [440.0, 1000.0, 5000.0] {
            let settings = Settings {
                source: Source::Tone { freq: frequency },
                volume: 1.0,
            };
            let mut generator = Generator::new(SAMPLE_RATE);
            let buffer = settled(&mut generator, &settings);
            let left: Vec<f32> = buffer.iter().step_by(2).copied().collect();
            let measured = measure_frequency(&left);
            // A zero-crossing count over the window resolves to about a hertz,
            // and the glide settles to a fraction of a percent.
            assert!(
                (measured - frequency * 1.0).abs() < frequency * 0.01,
                "aimed at {frequency} Hz, measured {measured} Hz"
            );
        }
    }

    #[test]
    fn the_pitch_glides_rather_than_jumping() {
        let mut generator = Generator::new(SAMPLE_RATE);
        let low = Settings {
            source: Source::Tone { freq: 200.0 },
            volume: 1.0,
        };
        let high = Settings {
            source: Source::Tone { freq: 4000.0 },
            volume: 1.0,
        };
        settled(&mut generator, &low);
        let start = generator.pitch.exp();
        // Drag the slider to twenty times the frequency and follow the pitch
        // the generator glides through, sampling it often enough to see the
        // ramp. The pitch is what the oscillator plays, so watching it is
        // watching the glide.
        let mut pitches = Vec::new();
        for _ in 0..200 {
            render(&mut generator, &high, 128);
            pitches.push(generator.pitch.exp());
        }
        let end = generator.pitch.exp();
        // The pitch rose, and is on its way without having arrived: a jump
        // would have landed at 4000 on the first sample.
        assert!(
            end > start * 10.0,
            "the pitch did not rise: {start} -> {end}"
        );
        assert!(end < 4000.0, "the pitch jumped straight to the target");
        // It only ever rises, and each step is a fraction of the way across the
        // remaining distance: the ramp is exponential in log space, so a window
        // closer to the start of a 20x glide gains more than one nearer the
        // end, but no step more than doubles, which is what a jump looks like.
        assert!(
            pitches
                .windows(2)
                .all(|pair| pair[1] >= pair[0] && pair[1] < pair[0] * 2.0),
            "the pitch was not a smooth glide: {:?}",
            &pitches[..20.min(pitches.len())]
        );
    }

    #[test]
    fn starting_and_stopping_do_not_click() {
        let settings = Settings {
            source: Source::Tone { freq: 1000.0 },
            volume: 1.0,
        };
        // The first sample must not be a step from silence.
        let mut generator = Generator::new(SAMPLE_RATE);
        let start = render(&mut generator, &settings, 64);
        assert!(start[0].abs() < 0.05, "the tone started at {}", start[0]);
        settled(&mut generator, &settings);
        // And the same when the volume is taken away. The fade is 10 ms, so the
        // window has to be long enough for it to finish before the tone
        // resumes.
        let mut quiet = Settings {
            source: Source::Tone { freq: 1000.0 },
            volume: 0.0,
        };
        let stop = render(&mut generator, &quiet, SAMPLE_RATE as usize / 10);
        assert!(stop[0].abs() < 0.01, "the tone stopped at {}", stop[0]);
        quiet.volume = 1.0;
        let restart = render(&mut generator, &quiet, 64);
        assert!(
            restart[0].abs() < 0.05,
            "the tone resumed at {}",
            restart[0]
        );
    }

    #[test]
    fn pink_noise_tilts_toward_the_bass_rather_than_being_white() {
        // Pink noise has equal energy per octave, so across the 20 Hz to 20 kHz
        // band it carries far more energy in the bottom than a flat signal does.
        // Comparing it against the white generator it is built from says the
        // tilt is real and the right way round, without needing to know the
        // filter's absolute gain.
        let noise = {
            let settings = Settings {
                source: Source::PinkNoise,
                volume: 1.0,
            };
            let mut generator = Generator::seeded(SAMPLE_RATE, 20_240_101);
            let buffer = settled(&mut generator, &settings);
            buffer.iter().step_by(2).copied().collect::<Vec<f32>>()
        };
        let mut white = White::new(20_240_101);
        let white: Vec<f32> = (0..noise.len()).map(|_| white.next_sample()).collect();
        let corner = 1000.0;
        let pink_tilt = high_to_low_energy(&noise, corner);
        let white_tilt = high_to_low_energy(&white, corner);
        assert!(
            pink_tilt * 5.0 < white_tilt,
            "pink noise is not tilted toward the bass: it falls {} high-to-low against white's {}",
            pink_tilt,
            white_tilt
        );
        // The white source really is flat, so its tilt is only set by where the
        // corner sits in the band: well under a hundred to one.
        assert!(
            white_tilt > 4.0 && white_tilt < 40.0,
            "the white reference did not behave like white noise: {white_tilt}"
        );
    }

    #[test]
    fn the_noise_is_roughly_centred_on_silence() {
        let mut noise = PinkNoise::new(1);
        // The one-pole filters charge up over their first few thousand samples;
        // measure the mean once they have settled.
        for _ in 0..5_000 {
            noise.next_sample();
        }
        let samples: Vec<f32> = (0..20_000).map(|_| noise.next_sample()).collect();
        let mean =
            samples.iter().map(|value| f64::from(*value)).sum::<f64>() / samples.len() as f64;
        // The one-pole filters leave a small DC residue, around a percent of
        // the peak: inaudible, but not zero.
        assert!(mean.abs() < 0.02, "the noise drifted to {mean}");
        assert!(
            samples.iter().all(|value| value.abs() < 2.0),
            "the noise left the sample range"
        );
    }

    #[test]
    fn the_white_source_is_reproducible_from_its_seed() {
        let mut first = White::new(42);
        let mut second = White::new(42);
        let mut other = White::new(43);
        let a: Vec<f32> = (0..1000).map(|_| first.next_sample()).collect();
        let b: Vec<f32> = (0..1000).map(|_| second.next_sample()).collect();
        let c: Vec<f32> = (0..1000).map(|_| other.next_sample()).collect();
        assert_eq!(a, b);
        assert_ne!(a, c);
        // A zero seed must not be a fixed point, which would emit silence.
        assert!((0..1000).any(|_| White::new(0).next_sample() != 0.0));
    }

    #[test]
    fn white_noise_is_flat_across_the_spectrum() {
        // The counterpart to the pink test: white has the same energy at every
        // frequency, so its high-to-low split matches the reference it is
        // generated against. This is what makes it useful as the reference for
        // hearing what the pink filter does.
        let settings = Settings {
            source: Source::WhiteNoise,
            volume: 1.0,
        };
        let mut generator = Generator::seeded(SAMPLE_RATE, 20_240_102);
        let buffer = settled(&mut generator, &settings);
        let left: Vec<f32> = buffer.iter().step_by(2).copied().collect();
        let mut white = White::new(20_240_102);
        let white: Vec<f32> = (0..left.len()).map(|_| white.next_sample()).collect();
        let corner = 1000.0;
        let generated = high_to_low_energy(&left, corner);
        let reference = high_to_low_energy(&white, corner);
        // The generated white is the same signal the reference describes, so
        // the two splits agree closely; pink, by contrast, is far lower.
        assert!(
            (generated / reference - 1.0).abs() < 0.35,
            "white noise tilted away from flat: {generated} against {reference}"
        );
    }

    #[test]
    fn both_channels_carry_the_same_mono_source() {
        let settings = Settings {
            source: Source::Tone { freq: 1000.0 },
            volume: 0.5,
        };
        let mut generator = Generator::new(SAMPLE_RATE);
        let buffer = settled(&mut generator, &settings);
        assert!(buffer
            .as_chunks::<2>()
            .0
            .iter()
            .all(|frame| frame[0] == frame[1]));
    }
}
