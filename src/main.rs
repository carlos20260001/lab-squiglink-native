mod app;

use app::CrinGraphApp;

fn main() -> eframe::Result {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let shared_state = arguments
        .iter()
        .find(|argument| argument.starts_with("cringraph://state/"))
        .cloned();
    let debug_hooks = arguments.iter().any(|argument| argument == "--debug")
        || std::env::var_os("CRINGRAPH_DEBUG").is_some();
    if debug_hooks {
        eprintln!("CrinGraph debug hooks enabled (egui widget hover diagnostics).");
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("CrinGraph")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CrinGraph",
        options,
        Box::new(move |_creation_context| {
            Ok(Box::new(CrinGraphApp::with_debug_hooks(
                shared_state,
                debug_hooks,
            )))
        }),
    )
}
