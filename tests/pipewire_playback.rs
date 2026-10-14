//! The PipeWire backend, exercised against a real daemon.
//!
//! `cargo test` on a Linux machine with PipeWire running starts the actual
//! stream the sound tools use, waits for the daemon to hand the real-time
//! callback a buffer — the message it sends is the proof, since a stream that
//! merely connected produces none — walks the settings through every source,
//! and stops. It asserts on connectivity and callbacks, not on sound, because
//! nothing in a test can hear a speaker.
//!
//! On a machine with no PipeWire daemon this reports the connection failure
//! and passes, because "no audio here" is not a broken backend. Set
//! `SQUIGLAB_REQUIRE_AUDIO=1` to make a missing daemon a failure, which is
//! what a developer with audio wants.

#![cfg(target_os = "linux")]

use squiglab::audio::{Player, Settings, Source, SAMPLE_RATE};
use std::time::{Duration, Instant};

/// Whether a missing daemon should fail the test rather than pass.
fn require_audio() -> bool {
    std::env::var("SQUIGLAB_REQUIRE_AUDIO").is_ok_and(|value| value == "1")
}

#[test]
fn the_pipewire_backend_plays_every_source_through_the_daemon() {
    let settings = Settings {
        source: Source::Tone { freq: 440.0 },
        volume: 0.3,
    };
    let mut player = match Player::start(settings) {
        Ok(player) => player,
        Err(error) => {
            if require_audio() {
                panic!("the PipeWire backend could not start: {error}");
            }
            eprintln!("passing without a check: no PipeWire output here ({error})");
            return;
        }
    };

    // Wait for the callback to actually run. The backend reports the first
    // buffer it was handed, so this is the daemon really playing, not a stream
    // that connected and sat there.
    let mut playing = false;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if player
            .take_message()
            .is_some_and(|message| message.contains("PipeWire"))
        {
            playing = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if !playing {
        // No callback means no audio. With a daemon present that is a failure;
        // without one the start above already returned an error.
        panic!("the stream connected but the daemon never called the audio callback");
    }

    // Walk every source, so each render path runs through the real callback
    // and the real negotiated format. This is where a wrong buffer size or a
    // missing format read shows up as a crash rather than a silent failure.
    for source in [
        Source::Tone { freq: 880.0 },
        Source::Tone { freq: 200.0 },
        Source::PinkNoise,
        Source::WhiteNoise,
        Source::Tone { freq: 1000.0 },
    ] {
        player.set_settings(Settings {
            source,
            volume: 0.2,
        });
        std::thread::sleep(Duration::from_millis(120));
    }

    // Stopping must return promptly: a stream that will not answer would
    // otherwise freeze the interface, which is what the force-quit path exists
    // for.
    let start = Instant::now();
    player.stop();
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "stopping took {:?}, so the stream was not answering",
        start.elapsed()
    );

    // And the rate the generators run at is one PipeWire accepts.
    assert!((20_000..=192_000).contains(&SAMPLE_RATE));
}
