//! Fetches a preview from the Apple Music catalogue and decodes it.
//!
//! The music player does this in the background while the window is open;
//! this does it on the command line, so the chain can be checked without a
//! window:
//!
//!     cargo run --release --example apple_preview -- "portishead"

fn main() {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "portishead".to_owned());
    let tracks = match squiglab::music::search_apple(&query) {
        Ok(tracks) => tracks,
        Err(problem) => {
            eprintln!("{problem}");
            std::process::exit(1);
        }
    };
    println!("{} results for {query:?}", tracks.len());
    for track in tracks.iter().take(3) {
        println!("  {}", track.label());
        let bytes = match squiglab::music::fetch_preview(&track.preview) {
            Ok(bytes) => bytes,
            Err(problem) => {
                println!("    the preview could not be fetched: {problem}");
                continue;
            }
        };
        match squiglab_core::audio::decode::decode_bytes(&bytes) {
            Ok(decoded) => println!(
                "    {} bytes, {} channels at {} Hz, {:.0} seconds",
                bytes.len(),
                decoded.channels,
                decoded.rate,
                decoded.seconds()
            ),
            Err(problem) => println!("    the preview could not be decoded: {problem}"),
        }
    }
}
