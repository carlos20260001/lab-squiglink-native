//! Playback through the PipeWire daemon, on Linux.
//!
//! This speaks to the daemon directly through `libpipewire` rather than
//! handing samples to a helper process, so there is nothing between the
//! generators and the graph.
//!
//! The PipeWire main loop runs on the player's thread and owns the stream, its
//! real-time `process` callback, and the generator that callback pulls from.
//! The generators are cheap and lock-free, so running them on the real-time
//! thread is safe here; the alternative, a separate thread and a hand-off ring,
//! buys nothing but a copy. The only cross-thread state is the current
//! [`Settings`], which the UI thread swaps under a short lock the callback takes
//! once per buffer.
//!
//! Stopping sends a `Stop` command; the callback notices on its next pass and
//! quits the main loop through PipeWire's C entry point, whose wrapper is not
//! `Send` but whose function is thread-safe. That unwinds `run` and lets the
//! player thread finish.

use super::{Command, Generator, Player, Settings, SAMPLE_RATE};
use pipewire as pw;
use pw::spa;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// PipeWire's channel count. The generators are mono and write the same sample
/// to both channels; two channels is what a desktop sink expects.
const CHANNELS: usize = 2;

/// A scratch big enough for one callback buffer. PipeWire's default period is a
/// few hundred frames; a longer buffer is filled with silence past this.
const SCRATCH_FRAMES: usize = 4096;

/// Start a PipeWire playback stream and return the [`Player`] that owns it. The
/// main loop connects the stream before returning, so a failure to reach an
/// output is reported here rather than silently producing silence.
pub fn start(settings: Settings) -> Result<Player, String> {
    let (commands_out, command_receiver) = mpsc::channel::<Command>();
    let (reports_out, reports) = mpsc::channel::<String>();
    let (done_out, done) = mpsc::channel::<()>();
    let shared = Arc::new(Mutex::new(settings));
    // The filters the source is played through. The generator lives in the
    // real-time callback, so the chain is shared the way the settings are.
    let chain = Arc::new(Mutex::new(None::<crate::audio::Chain>));
    // The music file being played, if a file is the source.
    let file = Arc::new(Mutex::new(None::<crate::audio::FilePlayback>));
    let played = Arc::new(Played::new(reports_out.clone()));
    let quit_handle = Arc::new(QuitHandle {
        // Filled in by the player thread once the loop exists.
        pointer: std::sync::atomic::AtomicUsize::new(0),
    });
    let reports_for_thread = reports_out.clone();
    let handle_for_thread = Arc::clone(&quit_handle);
    let thread = std::thread::spawn(move || {
        let result = run(
            shared,
            chain,
            file,
            command_receiver,
            &reports_for_thread,
            played,
            &handle_for_thread,
        );
        if let Err(error) = result {
            let _ = reports_for_thread.send(error);
        }
        // Tells the Player this thread is finished, so `stop` can tell a
        // prompt exit from one that needed forcing.
        let _ = done_out.send(());
    });
    Ok(Player {
        commands: Some(commands_out),
        reports,
        thread: Some(thread),
        done: Some(done),
        force_quit: Some(Box::new(move || quit_handle.quit())),
    })
}

/// The audio format the daemon negotiated for the stream, published by
/// `param_changed` and read by the real-time callback. Zero until negotiated,
/// which is how the callback knows not to write yet.
#[derive(Default)]
struct Negotiated {
    state: Mutex<(usize, usize)>,
}

impl Negotiated {
    fn store(&self, sample_bytes: usize, channels: usize) {
        *self.state.lock().expect("format lock") = (sample_bytes, channels);
    }

    fn get(&self) -> (usize, usize) {
        *self.state.lock().expect("format lock")
    }
}

/// What the real-time callback has actually done, so a test can tell a working
/// stream from one that merely connected.
struct Played {
    buffers: std::sync::atomic::AtomicUsize,
    format: Mutex<Option<(usize, usize)>>,
    reports: mpsc::Sender<String>,
}

impl Played {
    fn new(reports: mpsc::Sender<String>) -> Self {
        Self {
            buffers: std::sync::atomic::AtomicUsize::new(0),
            format: Mutex::new(None),
            reports,
        }
    }

    fn played(&self) {
        let count = self
            .buffers
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        if count == 1 {
            let _ = self.reports.send("Playing through PipeWire.".to_owned());
        }
    }

    fn negotiated(&self, sample_bytes: usize, channels: usize) {
        *self.format.lock().expect("format lock") = Some((sample_bytes, channels));
    }
}

/// Write one generator sample, in whatever size the stream negotiated, from
/// the generator's -1..1 float.
fn write_sample(destination: &mut [u8], value: f32, sample_bytes: usize) {
    let value = f64::from(value);
    match sample_bytes {
        2 => {
            let scaled = (value * f64::from(i16::MAX))
                .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
            destination.copy_from_slice(&scaled.to_le_bytes());
        }
        4 => {
            let scaled = (value * f64::from(i32::MAX))
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
            destination.copy_from_slice(&scaled.to_le_bytes());
        }
        _ => {}
    }
}

/// A handle that stops the PipeWire main loop from another thread.
///
/// PipeWire's quit is safe to call from any thread, but the safe wrapper
/// borrows the loop, which is why this keeps the raw pointer and calls the C
/// entry point. The pointer is published by the player thread once the loop
/// exists, and stays zero until then, so an early stop is a no-op rather than
/// a call through a null pointer.
struct QuitHandle {
    pointer: std::sync::atomic::AtomicUsize,
}

impl QuitHandle {
    fn publish(&self, mainloop: &pw::main_loop::MainLoopRc) {
        // MainLoopRc derefs to MainLoop, whose as_raw_ptr is the
        // *mut pw_main_loop that pw_main_loop_quit expects. (The inner
        // loop_'s pointer is a *mut pw_loop and passing that to quit is a
        // type-confused crash.)
        self.pointer.store(
            mainloop.as_raw_ptr() as usize,
            std::sync::atomic::Ordering::Release,
        );
    }

    fn quit(&self) {
        let pointer = self.pointer.load(std::sync::atomic::Ordering::Acquire);
        if pointer != 0 {
            unsafe { pw::sys::pw_main_loop_quit(pointer as *mut _) };
        }
    }
}

fn run(
    shared: Arc<Mutex<Settings>>,
    chain: Arc<Mutex<Option<crate::audio::Chain>>>,
    file: Arc<Mutex<Option<crate::audio::FilePlayback>>>,
    command_receiver: mpsc::Receiver<Command>,
    reports: &mpsc::Sender<String>,
    played: Arc<Played>,
    quit_handle: &Arc<QuitHandle>,
) -> Result<(), String> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|error| format!("Unable to create the PipeWire main loop: {error}"))?;
    quit_handle.publish(&mainloop);
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|error| format!("Unable to create a PipeWire context: {error}"))?;
    let core = context
        .connect_rc(None)
        .map_err(|error| format!("Unable to reach the PipeWire daemon: {error}"))?;

    let stream = pw::stream::StreamBox::new(
        &core,
        "SquigLab sound tools",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::APP_NAME => "SquigLab",
        },
    )
    .map_err(|error| format!("Unable to create a PipeWire stream: {error}"))?;

    // What the daemon negotiated: bytes per sample and channels. The process
    // callback writes at this size rather than the one that was asked for,
    // because the daemon is free to hand back a different format and a wrong
    // element size would scribble past the end of every buffer.
    let negotiated = Arc::new(Negotiated::default());
    let negotiated_param = Arc::clone(&negotiated);
    let negotiated_process = Arc::clone(&negotiated);

    let callback_shared = Arc::clone(&shared);
    let callback_chain = Arc::clone(&chain);
    let callback_file = Arc::clone(&file);
    let callback_quit = Arc::clone(quit_handle);
    let mut scratch = vec![0.0_f32; SCRATCH_FRAMES * 2];
    // Counts the buffers the daemon actually handed over, and reports the
    // negotiated format once, so a test can prove the real-time callback ran
    // rather than the stream merely existing.
    let for_param_report = Arc::clone(&played);
    let for_process_report = Arc::clone(&played);

    let _listener = stream
        .add_local_listener_with_user_data(Generator::new(SAMPLE_RATE))
        .param_changed(move |_, _, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            let mut info = spa::param::audio::AudioInfoRaw::new();
            if info.parse(param).is_err() {
                return;
            }
            // Bytes per sample, from the format the daemon settled on. Anything
            // this code does not know how to write is left at zero, and the
            // process callback then leaves the buffer alone.
            let sample_bytes = match info.format() {
                spa::param::audio::AudioFormat::S16LE => 2,
                spa::param::audio::AudioFormat::S32LE
                | spa::param::audio::AudioFormat::U32LE
                | spa::param::audio::AudioFormat::S24LE
                | spa::param::audio::AudioFormat::S24BE => 4,
                _ => 0,
            };
            negotiated_param.store(sample_bytes, info.channels() as usize);
            for_param_report.negotiated(sample_bytes, info.channels() as usize);
        })
        .process(move |stream, generator| {
            // Drain commands: the last Settings wins, and a Stop quits the loop.
            let mut quit_now = false;
            while let Ok(command) = command_receiver.try_recv() {
                match command {
                    Command::Settings(next) => {
                        *callback_shared.lock().expect("settings lock") = next;
                    }
                    Command::Chain(next) => {
                        *callback_chain.lock().expect("chain lock") = Some(next);
                    }
                    Command::File(next) => {
                        *callback_file.lock().expect("file lock") = Some(next);
                    }
                    Command::Stop => quit_now = true,
                }
            }
            if quit_now {
                callback_quit.quit();
                return;
            }
            let (sample_bytes, channels) = negotiated_process.get();
            for_process_report.played();
            if sample_bytes == 0 || channels == 0 {
                // The format has not been negotiated yet, or is one this code
                // cannot write. Leave the buffer as the daemon set it.
                return;
            }
            // A chain that has changed is handed to the generator, which
            // starts its filters again from silence.
            if let Some(next) = callback_chain.lock().expect("chain lock").take() {
                generator.set_chain(next);
            }
            if let Some(next) = callback_file.lock().expect("file lock").take() {
                generator.set_file(next);
            }
            let settings = *callback_shared.lock().expect("settings lock");
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            if let Some(data) = datas.first_mut() {
                if let Some(slice) = data.data() {
                    let stride = channels * sample_bytes;
                    let frames = (slice.len() / stride).min(SCRATCH_FRAMES);
                    generator.render(&mut scratch[..frames * 2], &settings);
                    for frame_index in 0..frames {
                        for channel in 0..channels {
                            let value = scratch[frame_index * 2];
                            let start = frame_index * stride + channel * sample_bytes;
                            write_sample(
                                &mut slice[start..start + sample_bytes],
                                value,
                                sample_bytes,
                            );
                        }
                    }
                    let chunk = data.chunk_mut();
                    *chunk.offset_mut() = 0;
                    *chunk.stride_mut() = stride as i32;
                    *chunk.size_mut() = (frames * stride) as u32;
                }
            }
            // Dropping the buffer enqueues it back to the stream.
            drop(buffer);
        })
        .register()
        .map_err(|error| format!("Unable to register the PipeWire callback: {error}"))?;

    // Ask for interleaved signed 32-bit stereo. PipeWire has no interleaved
    // float format constant here (F32P is planar), and S32LE is what a raw
    // client negotiates that a sink resamples cleanly. The daemon is free to
    // hand back a different format, so the process callback reads the
    // negotiated one from param_changed and writes at that element size.
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::S32LE);
    audio_info.set_rate(SAMPLE_RATE);
    audio_info.set_channels(CHANNELS as u32);
    let mut position = [0; spa::param::audio::MAX_CHANNELS];
    position[0] = libspa_sys::SPA_AUDIO_CHANNEL_FL;
    position[1] = libspa_sys::SPA_AUDIO_CHANNEL_FR;
    audio_info.set_position(position);
    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(spa::pod::Object {
            type_: libspa_sys::SPA_TYPE_OBJECT_Format,
            id: libspa_sys::SPA_PARAM_EnumFormat,
            properties: audio_info.into(),
        }),
    )
    .map_err(|error| format!("Unable to describe the audio format to PipeWire: {error}"))?
    .0
    .into_inner();
    let mut params = [spa::pod::Pod::from_bytes(&values)
        .ok_or_else(|| "Unable to build the audio format parameter.".to_owned())?];

    stream
        .connect(
            spa::utils::Direction::Output,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )
        .map_err(|error| format!("Unable to connect the stream to an output: {error}"))?;

    // Block in the main loop. The callback quits it on a Stop command. If the
    // device disappears first, the callback stops being called, so the Player
    // can also force the loop down through the handle it was handed.
    mainloop.run();
    let _ = reports.send("Stopped.".to_owned());
    // The callback may have returned before the loop did, so make sure the
    // loop is not left running if `run` is unwinding for another reason.
    quit_handle.quit();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_sample;

    #[test]
    fn a_sample_is_written_at_the_negotiated_width() {
        // 16-bit: the generator's -1..1 float, scaled to i16.
        let mut two = [0_u8; 2];
        write_sample(&mut two, 1.0, 2);
        assert_eq!(i16::from_le_bytes(two), i16::MAX);
        // The scale is by the positive full-scale value, so -1.0 lands on
        // -i16::MAX, the symmetric end rather than the one-sample-further
        // minimum.
        write_sample(&mut two, -1.0, 2);
        assert_eq!(i16::from_le_bytes(two), -i16::MAX);
        write_sample(&mut two, 0.0, 2);
        assert_eq!(i16::from_le_bytes(two), 0);

        // 32-bit: the same, at i32, which is what the stream negotiates.
        let mut four = [0_u8; 4];
        write_sample(&mut four, 1.0, 4);
        assert_eq!(i32::from_le_bytes(four), i32::MAX);
        write_sample(&mut four, -1.0, 4);
        assert_eq!(i32::from_le_bytes(four), -i32::MAX);
    }

    #[test]
    fn a_value_beyond_the_range_is_clamped_not_wrapped() {
        // A generator sample can never leave -1..1, but a clamp rather than a
        // cast is what stops a stray value wrapping into full-scale noise.
        let mut four = [0_u8; 4];
        write_sample(&mut four, 4.0, 4);
        assert_eq!(i32::from_le_bytes(four), i32::MAX);
        write_sample(&mut four, -4.0, 4);
        assert_eq!(i32::from_le_bytes(four), i32::MIN);
    }

    #[test]
    fn an_unknown_width_is_left_alone() {
        // A width this code does not write is not scribbled over: the callback
        // only calls this with a size it negotiated.
        let mut eight = [7_u8; 8];
        write_sample(&mut eight[..], 1.0, 8);
        assert_eq!(eight, [7_u8; 8], "an unsupported width must not be written");
    }
}
