//! Playback through CoreAudio on macOS and WASAPI on Windows, via cpal.
//!
//! cpal is used only on these two platforms. Linux goes to PipeWire instead
//! (see [`super::pipewire`]), because PipeWire was asked for there
//! specifically and cpal reaches Linux only through ALSA.
//!
//! The shape matches the PipeWire backend: the generators run on an ordinary
//! thread and push into a lock-free ring, and cpal's data callback — which also
//! must not block or allocate — drains it into the device buffer. Silence when
//! the ring is short is the right response in a real-time callback, not
//! waiting.
//!
//! **Not compiled or run on the machine this was written on**, which is Linux.
//! Everything here is gated to macOS and Windows, so treat the first run on
//! either as the test it has not had. The logic it shares with the PipeWire
//! backend — the ring, the generators, the command loop — is tested.

use super::{Command, Generator, Player, SampleRing, Settings, TryRecvError, BUFFER_FRAMES};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::sync::Arc;

/// Frames of slack between the generator and the device. The generator waits
/// here when the device is behind, which is what paces it to real time.
const RING_FRAMES: usize = BUFFER_FRAMES * 16;

/// Start playback on the host's default output, and return the [`Player`] that
/// owns it.
pub fn start(settings: Settings) -> Result<Player, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No audio output device is available.".to_owned())?;
    let config = device
        .default_output_config()
        .map_err(|error| format!("Unable to read the output configuration: {error}"))?;
    let channels = config.channels().max(1) as usize;
    let sample_rate = config.sample_rate().0;

    let ring = Arc::new(SampleRing::new(RING_FRAMES));
    let callback_ring = Arc::clone(&ring);
    let (reports_out, reports) = mpsc::channel::<String>();

    // The callback's scratch buffer, grown once on the first call and reused
    // after: allocating inside a real-time callback is how a stream glitches.
    let mut scratch: Vec<f32> = Vec::new();
    let stream = device
        .build_output_stream(
            &config,
            move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if scratch.len() < output.len() {
                    scratch.resize(output.len(), 0.0);
                }
                let drained = callback_ring.drain(&mut scratch[..output.len()]);
                for (index, sample) in output.iter_mut().enumerate() {
                    // The generators are mono, so one sample is spread across
                    // every channel.
                    *sample = scratch[index / channels];
                }
                let _ = drained;
            },
            move |error| {
                let _ = reports_out.send(format!("Audio output error: {error}"));
            },
            None,
        )
        .map_err(|error| format!("Unable to open an output stream: {error}"))?;
    stream
        .play()
        .map_err(|error| format!("Unable to start the output stream: {error}"))?;

    let (commands_out, receiver) = mpsc::channel::<Command>();
    let handle = std::thread::spawn(move || {
        // The generators run at the device's own rate, so a 44.1 kHz device
        // gets a 44.1 kHz tone rather than one resampled after the fact.
        let mut generator = Generator::new(sample_rate);
        let mut settings = settings;
        let mut buffer = vec![0.0_f32; BUFFER_FRAMES * 2];
        'play: loop {
            loop {
                match receiver.try_recv() {
                    Ok(Command::Settings(next)) => settings = next,
                    Ok(Command::Stop) | Err(TryRecvError::Disconnected) => break 'play,
                    Err(TryRecvError::Empty) => break,
                }
            }
            generator.render(&mut buffer, &settings);
            ring.push(&buffer);
        }
        // Pausing here, on the generator thread, is what stops the device: the
        // callback stops being called and the stream is dropped with the thread.
        let _ = stream.pause();
    });

    Ok(Player {
        commands: Some(commands_out),
        reports,
        thread: Some(handle),
    })
}
