//! The window.

mod app;
mod eq_input;
mod gpui_bridge;
mod graph;
mod icons;
mod theme;

use gpui::{
    px, size, App as GpuiApp, AppContext, Application, Bounds, TitlebarOptions, WindowBounds,
    WindowOptions,
};
use squiglab_core::product;

fn main() {
    Application::new()
        .with_assets(icons::IconAssets {})
        .run(|cx: &mut GpuiApp| {
            // The faces must be registered before the first frame, or the first
            // layout is measured against the system's own.
            theme::install(cx);

            let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some(product::APP_NAME.into()),
                        ..Default::default()
                    }),
                    app_id: Some(product::APP_ID.to_owned()),
                    window_min_size: Some(size(px(430.0), px(520.0))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| app::App::new()),
            )
            .expect("the window opens");
            cx.activate(true);
        });
}
