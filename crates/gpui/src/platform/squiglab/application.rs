//! GPUI Platform implementation for squiglab.
//!
//! Implements GPUI's internal `Platform` trait, delegating windowing and
//! input to squiglab-platform's Winit-based abstraction.

use std::any::Any;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, DummyKeyboardMapper,
    ForegroundExecutor, Keymap, Menu, MenuItem, NoopTextSystem, PathPromptOptions, Platform,
    PlatformDispatcher, PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper,
    PlatformTextSystem, PlatformWindow, PromptButton, ScreenCaptureSource, Task, WindowAppearance,
    WindowParams,
};
use parking_lot::Mutex;

pub use crate::platform::squiglab::window::SquiglabWindow;

thread_local! {
    /// Queue of runnables to execute on the main thread.
    static MAIN_THREAD_QUEUE: RefCell<std::collections::VecDeque<async_task::Runnable>> =
        RefCell::new(std::collections::VecDeque::new());
}

/// Dispatcher that runs tasks on the main thread.
pub struct SquiglabDispatcher;

impl SquiglabDispatcher {
    pub fn new() -> Arc<dyn PlatformDispatcher> {
        Arc::new(Self)
    }
}

impl PlatformDispatcher for SquiglabDispatcher {
    fn is_main_thread(&self) -> bool {
        true
    }

    fn dispatch(&self, runnable: async_task::Runnable, _label: Option<gpui::TaskLabel>) {
        MAIN_THREAD_QUEUE.with(|q| q.borrow_mut().push_back(runnable));
    }

    fn dispatch_on_main_thread(&self, runnable: async_task::Runnable) {
        MAIN_THREAD_QUEUE.with(|q| q.borrow_mut().push_back(runnable));
    }

    fn dispatch_after(&self, _duration: Duration, runnable: async_task::Runnable) {
        MAIN_THREAD_QUEUE.with(|q| q.borrow_mut().push_back(runnable));
    }
}

/// Drain all queued runnables. Called from the event loop.
pub fn drain_main_thread_queue() {
    MAIN_THREAD_QUEUE.with(|q| {
        while let Some(runnable) = q.borrow_mut().pop_front() {
            runnable.run();
        }
    });
}

/// GPUI Platform implementation backed by squiglab-platform.
pub struct SquiglabPlatform {
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    text_system: Arc<dyn PlatformTextSystem>,
    active_display: Rc<dyn PlatformDisplay>,
    active_window: RefCell<Option<AnyWindowHandle>>,
    current_clipboard_item: Mutex<Option<ClipboardItem>>,
}

impl SquiglabPlatform {
    pub fn new(_headless: bool) -> Rc<Self> {
        let dispatcher = SquiglabDispatcher::new();
        let background_executor = BackgroundExecutor::new(dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(dispatcher);

        Rc::new_cyclic(|_weak| SquiglabPlatform {
            background_executor,
            foreground_executor,
            text_system: Arc::new(NoopTextSystem),
            active_display: Rc::new(SquiglabDisplay::new()),
            active_window: RefCell::new(None),
            current_clipboard_item: Mutex::new(None),
        })
    }
}

impl Platform for SquiglabPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.background_executor.clone()
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.text_system.clone()
    }

    fn run(&self, _on_finish_launching: Box<dyn FnOnce()>) {
        // The Winit event loop is started by the application crate.
    }

    fn quit(&self) {
        std::process::exit(0);
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {}

    fn activate(&self, _ignoring_other_apps: bool) {}

    fn hide(&self) {}

    fn hide_other_apps(&self) {}

    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![self.active_display.clone()]
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.active_display.clone())
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        self.active_window.borrow().clone()
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        params: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        let window = SquiglabWindow::new(handle, params)?;
        *self.active_window.borrow_mut() = Some(handle);
        Ok(Box::new(window))
    }

    fn window_appearance(&self) -> WindowAppearance {
        WindowAppearance::Light
    }

    fn open_url(&self, url: &str) {
        let _ = url;
    }

    fn on_open_urls(&self, _callback: Box<dyn FnMut(Vec<String>)>) {}

    fn register_url_scheme(&self, _url: &str) -> Task<anyhow::Result<()>> {
        self.background_executor().spawn(async { Ok(()) })
    }

    fn prompt_for_paths(
        &self,
        _options: PathPromptOptions,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<Option<Vec<PathBuf>>>> {
        let (tx, rx) = futures::channel::oneshot::channel();
        let _ = tx.send(Ok(None));
        rx
    }

    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        _suggested_name: Option<&str>,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<Option<PathBuf>>> {
        let (tx, rx) = futures::channel::oneshot::channel();
        let _ = tx.send(Ok(None));
        rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    fn reveal_path(&self, _path: &Path) {}

    fn open_with_system(&self, _path: &Path) {}

    fn on_quit(&self, _callback: Box<dyn FnMut()>) {}

    fn on_reopen(&self, _callback: Box<dyn FnMut()>) {}

    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}

    fn on_app_menu_action(&self, _callback: Box<dyn FnMut(&dyn Action)>) {}

    fn on_will_open_app_menu(&self, _callback: Box<dyn FnMut()>) {}

    fn on_validate_app_menu_command(&self, _callback: Box<dyn FnMut(&dyn Action) -> bool>) {}

    fn app_path(&self) -> anyhow::Result<PathBuf> {
        Ok(std::env::current_dir().unwrap_or_default())
    }

    fn path_for_auxiliary_executable(&self, name: &str) -> anyhow::Result<PathBuf> {
        Ok(std::env::current_dir()
            .unwrap_or_default()
            .join(name))
    }

    fn set_cursor_style(&self, _style: CursorStyle) {}

    fn should_auto_hide_scrollbars(&self) -> bool {
        false
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        *self.current_clipboard_item.lock() = Some(item);
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        self.current_clipboard_item.lock().clone()
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn write_to_primary(&self, _item: ClipboardItem) {}

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn read_from_primary(&self) -> Option<ClipboardItem> {
        None
    }

    fn write_credentials(
        &self,
        _url: &str,
        _username: &str,
        _password: &[u8],
    ) -> Task<anyhow::Result<()>> {
        self.background_executor().spawn(async { Ok(()) })
    }

    fn read_credentials(
        &self,
        _url: &str,
    ) -> Task<anyhow::Result<Option<(String, Vec<u8>)>>> {
        self.background_executor().spawn(async { Ok(None) })
    }

    fn delete_credentials(&self, _url: &str) -> Task<anyhow::Result<()>> {
        self.background_executor().spawn(async { Ok(()) })
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(SquiglabKeyboardLayout)
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(DummyKeyboardMapper)
    }

    fn on_keyboard_layout_change(&self, _callback: Box<dyn FnMut()>) {}
}

/// Minimal keyboard layout.
struct SquiglabKeyboardLayout;

impl PlatformKeyboardLayout for SquiglabKeyboardLayout {
    fn id(&self) -> &str {
        "us"
    }

    fn name(&self) -> &str {
        "US"
    }
}

/// Minimal display implementation.
#[derive(Debug)]
struct SquiglabDisplay {
    bounds: gpui::Bounds<gpui::Pixels>,
}

impl SquiglabDisplay {
    fn new() -> Self {
        Self {
            bounds: gpui::Bounds {
                origin: gpui::Point { x: gpui::px(0.0), y: gpui::px(0.0) },
                size: gpui::Size { width: gpui::px(1440.0), height: gpui::px(900.0) },
            },
        }
    }
}

impl PlatformDisplay for SquiglabDisplay {
    fn id(&self) -> gpui::DisplayId {
        gpui::DisplayId(0)
    }

    fn uuid(&self) -> anyhow::Result<uuid::Uuid> {
        Ok(uuid::Uuid::nil())
    }

    fn bounds(&self) -> gpui::Bounds<gpui::Pixels> {
        self.bounds
    }
}
