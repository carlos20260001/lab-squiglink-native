//! Sound tools: a continuous tone generator and pink noise, played through
//! PipeWire on Linux.
//!
//! The reference tool drives a Web Audio `OscillatorNode`: one continuous
//! oscillator whose pitch is dragged along a log-mapped frequency slider, not
//! an automated sweep, with a pink-noise source beside it. That is what
//! [`Generator`] renders — the generator is plain sample arithmetic,
//! independent of where the samples go.
//!
//! Output goes to the PipeWire daemon through `pw-cat`, the PipeWire command
//! line player, which reads raw samples from its standard input. Talking to
//! the daemon that way keeps the application free of a build-time dependency
//! on `libpipewire` and off ALSA, and it is the same daemon a native client
//! would connect to. A backend built on `libpipewire` can replace it later;
//! [`Generator`] is the whole of what such a backend would need.
//!
//! Everything in the generator is deterministic and testable without an audio
//! device: the tests below check the pitch, the glide, and the noise's
//! spectrum, not that a sound came out.

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

/// The sample rate the generators run at. It matches the equalizer DSP's rate,
/// so a tone and the curve it is auditioned against sit on the same grid.
pub const SAMPLE_RATE: u32 = 48_000;

/// Frames rendered per pass to the player. At 48 kHz this is about 11 ms,
/// short enough that a drag of the frequency slider reaches the speaker
/// promptly.
pub const BUFFER_FRAMES: usize = 512;

/// The PipeWire command line player this module streams to.
const PW_CAT: &str = "pw-cat";

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
}

impl Source {
    /// Name for the control that selects this source.
    pub fn name(self) -> &'static str {
        match self {
            Self::Tone { .. } => "Tone generator",
            Self::PinkNoise => "Pink noise",
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
    /// Per-sample smoothing coefficients, derived from the time constants.
    glide_coefficient: f64,
    fade_coefficient: f32,
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
            glide_coefficient: smoothing(sample_rate, GLIDE_SECONDS),
            fade_coefficient: smoothing(sample_rate, f64::from(FADE_SECONDS)) as f32,
        }
    }

    /// Fill `buffer` with interleaved stereo frames, advancing the generator by
    /// its length. The source is mono: both channels carry the same sample.
    pub fn render(&mut self, buffer: &mut [f32], settings: &Settings) {
        for frame in buffer.as_chunks_mut::<2>().0 {
            let sample = self.next_sample(settings);
            frame[0] = sample;
            frame[1] = sample;
        }
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
        };
        (sample * volume).clamp(-1.0, 1.0)
    }

    /// Move the oscillator's pitch toward the target and step its position.
    fn advance_pitch(&mut self, settings: &Settings) {
        let target = match settings.source {
            Source::Tone { freq } => freq.clamp(20.0, 20_000.0).ln(),
            // Noise has no pitch, but the state is kept moving so a switch back
            // to a tone resumes rather than gliding.
            Source::PinkNoise => self.pitch,
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

/// What the player thread is told to do.
enum Command {
    Settings(Settings),
    Stop,
}

/// A playing generator, streamed to the PipeWire daemon on a background
/// thread. Dropping it stops the sound.
pub struct Player {
    commands: Option<Sender<Command>>,
    reports: Receiver<String>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Player {
    /// Start playing, and report why not if the audio path is unavailable.
    #[cfg(target_os = "linux")]
    pub fn start(settings: Settings) -> Result<Self, String> {
        let mut child = std::process::Command::new(PW_CAT)
            .args([
                "--playback",
                "--raw",
                "--format",
                "f32",
                "--rate",
                &SAMPLE_RATE.to_string(),
                "--channels",
                "2",
                "-",
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("Unable to start {PW_CAT}, the PipeWire player: {error}"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| format!("{PW_CAT} did not open an input stream"))?;
        let (commands, receiver) = mpsc::channel();
        let (reports_out, reports) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut generator = Generator::new(SAMPLE_RATE);
            let mut settings = settings;
            let mut buffer = vec![0.0_f32; BUFFER_FRAMES * 2];
            let mut bytes = vec![0_u8; BUFFER_FRAMES * 2 * size_of::<f32>()];
            'play: loop {
                loop {
                    match receiver.try_recv() {
                        Ok(Command::Settings(next)) => settings = next,
                        Ok(Command::Stop) | Err(TryRecvError::Disconnected) => break 'play,
                        Err(TryRecvError::Empty) => break,
                    }
                }
                generator.render(&mut buffer, &settings);
                for (chunk, sample) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(&buffer) {
                    chunk.copy_from_slice(&sample.to_le_bytes());
                }
                if let Err(error) = stdin.write_all(&bytes).and_then(|()| stdin.flush()) {
                    // A closed pipe means the daemon went away, which is the
                    // usual reason a stream stops: no device, or the session
                    // ended.
                    let _ = reports_out.send(format!("PipeWire stopped the stream: {error}"));
                    break;
                }
            }
            drop(stdin);
            let _ = child.wait();
        });
        Ok(Self {
            commands: Some(commands),
            reports,
            thread: Some(thread),
        })
    }

    /// Playback is not implemented on this platform yet: the tone generator and
    /// pink noise render, but only the Linux PipeWire path is wired up.
    #[cfg(not(target_os = "linux"))]
    pub fn start(_settings: Settings) -> Result<Self, String> {
        Err("Sound is only wired up on Linux, through PipeWire.".to_owned())
    }

    /// Change what is playing. A new frequency is glided to rather than
    /// jumped to, which is what the reference's dragged slider does.
    pub fn set_settings(&self, settings: Settings) {
        if let Some(commands) = &self.commands {
            let _ = commands.send(Command::Settings(settings));
        }
    }

    /// A message from the player, if it has reported one. Called once a frame.
    pub fn take_message(&mut self) -> Option<String> {
        self.reports.try_recv().ok()
    }

    /// Stop playing and wait for the stream to close.
    pub fn stop(&mut self) {
        if let Some(commands) = self.commands.take() {
            let _ = commands.send(Command::Stop);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
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
