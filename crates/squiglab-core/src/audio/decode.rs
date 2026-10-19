//! Decoding the audio files the music player plays.
//!
//! The reference plays a music file, and offers the short previews the Apple
//! Music catalogue serves, which are AAC in an MP4 container. A local file may
//! be MP3 or anything else the decoder knows.

use std::io::Read;

/// The samples of a decoded file.
#[derive(Clone, Debug)]
pub struct Decoded {
    /// The samples, interleaved by channel.
    pub samples: Vec<f32>,
    /// How many channels the file has.
    pub channels: u32,
    /// The rate the file was recorded at.
    pub rate: f64,
}

impl Decoded {
    /// The length of the file, in seconds.
    pub fn seconds(&self) -> f64 {
        if self.channels == 0 || self.rate == 0.0 {
            return 0.0;
        }
        self.samples.len() as f64 / self.channels as f64 / self.rate
    }
}

/// Decode an audio file into its samples.
pub fn decode_file(path: &std::path::Path) -> Result<Decoded, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("The file was not read: {error}"))?;
    decode_bytes(&bytes)
}

/// Decode audio bytes into their samples.
pub fn decode_bytes(bytes: &[u8]) -> Result<Decoded, String> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::codecs::CodecParameters;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let mut hint = Hint::new();
    // The first bytes tell an MP4 container from an MP3: the former starts
    // with a box called "ftyp".
    hint.with_extension(match bytes.get(4..8) {
        Some(b"ftyp") => "m4a",
        _ => "mp3",
    });

    let source = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(describe)?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| "the file has no audio in it".to_owned())?;
    let track_id = track.id;
    let Some(CodecParameters::Audio(params)) = track.codec_params.as_ref() else {
        return Err("the file's track is not audio".to_owned());
    };
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(describe)?;

    let mut samples: Vec<f32> = Vec::new();
    let mut channels = 1_u32;
    let mut rate = 0.0_f64;
    while let Some(packet) = format.next_packet().map_err(describe)? {
        if packet.track_id != track_id {
            continue;
        }
        let decoded = decoder.decode(&packet).map_err(describe)?;
        let spec = decoded.spec();
        channels = spec.channels().count() as u32;
        rate = spec.rate() as f64;
        // The buffer's samples are copied out interleaved, which is the shape
        // the generator plays.
        let start = samples.len();
        let count = decoded.samples_interleaved();
        samples.resize(start + count, 0.0);
        decoded.copy_to_slice_interleaved(&mut samples[start..]);
    }
    if samples.is_empty() {
        return Err("the file holds no audio".to_owned());
    }
    Ok(Decoded {
        samples,
        channels,
        rate,
    })
}

/// Read a whole stream into memory, for a file that is not on disk.
pub fn read_all(mut reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| format!("The file was not read: {error}"))?;
    Ok(bytes)
}

/// The decoder's own words for what went wrong.
fn describe(error: symphonia::core::errors::Error) -> String {
    format!("the file could not be decoded: {error}")
}
