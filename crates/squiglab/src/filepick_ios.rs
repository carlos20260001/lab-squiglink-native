//! The phone's own document picker.
//!
//! This is the system's picker, written against the system's own libraries:
//! there is no third party between the program and the phone. The picker
//! hands back the file it opened, and the file is read where it lies, which
//! on a phone is inside the program's own storage.
//!
//! The picker is opened on the thread the window is drawn on, which is the
//! thread the system's own windows live on, and the answer is collected by
//! the frame after that.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use objc2::rc::{Allocated, Retained};
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send};
use objc2_foundation::{NSArray, NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{
    UIApplication, UIDocumentPickerDelegate, UIDocumentPickerViewController, UTType,
};

use super::Chosen;

/// The delegate, which the picker holds weakly and so has to be kept alive.
thread_local! {
    static DELEGATE: std::cell::RefCell<Option<Retained<Delegate>>> =
        const { std::cell::RefCell::new(None) };
}

/// Ask the phone for a file.
///
/// The picker is the system's own; the answer is handed to `on_chosen` when
/// the user has chosen, and the file is read then.
pub fn ask(title: &str, extensions: &[&str], on_chosen: impl Fn(Chosen) + 'static) {
    let (sender, receiver) = std::sync::mpsc::channel::<Chosen>();
    let delegate = Retained::new(Delegate {
        sender: sender.clone(),
    });
    DELEGATE.with(|cell| *cell.borrow_mut() = Some(delegate.clone()));

    let Ok(main) = objc2::MainThreadMarker::new() else {
        return;
    };
    let application = UIApplication::sharedApplication(main);
    let Some(window) = (unsafe { application.keyWindow() }) else {
        return;
    };
    let Some(root) = window.rootViewController() else {
        return;
    };
    let types = content_types(extensions);
    let picker = unsafe {
        Allocated::<UIDocumentPickerViewController>::new().initForOpeningContentTypes(&types)
    };
    unsafe {
        let _: () = msg_send![&*picker, setDelegate: &*delegate];
    }
    unsafe {
        root.presentViewController_animated_completion(picker, true, None);
    }
    // The answer comes on the delegate, which reads the file and hands it
    // over; this thread waits for it and then lets it go.
    std::thread::spawn(move || {
        if let Ok(chosen) = receiver.recv() {
            on_chosen(chosen);
        }
    });
    let _ = title;
}

/// The kinds of file the picker will offer.
fn content_types(extensions: &[String]) -> Retained<NSArray<UTType>> {
    let types: Vec<Retained<UTType>> = if extensions.is_empty() {
        vec![UTType::data()]
    } else {
        extensions
            .iter()
            .filter_map(|extension| UTType::from_extension(extension))
            .collect()
    };
    NSArray::from_retained_slice(types)
}

/// The delegate: the picker tells it what was chosen.
struct Delegate {
    sender: Sender<Chosen>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "SquigLabDocumentPickerDelegate"]
    #[ivars = Sender<Chosen>]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl UIDocumentPickerDelegate for Delegate {
        unsafe fn documentPicker_didPickDocumentsAtURLs(
            &self,
            _controller: &UIDocumentPickerViewController,
            urls: &NSArray<NSURL>,
        ) {
            let Some(url) = urls.first() else {
                return;
            };
            if let Some(chosen) = chosen_of(url) {
                let _ = self.ivars().sender.send(chosen);
            }
        }

        unsafe fn documentPickerWasCancelled(&self, _controller: &UIDocumentPickerViewController) {
            let _ = self.ivars().sender.send(Chosen {
                name: String::new(),
                path: None,
                bytes: None,
            });
        }
    }
);

/// The file the picker gives back, read where it lies.
fn chosen_of(url: &NSURL) -> Option<Chosen> {
    let path = url.path().map(PathBuf::from);
    let name = url
        .lastPathComponent()
        .map(|name| name.to_string())
        .unwrap_or_default();
    // The phone hands the file over inside the program's own storage, and the
    // system asks for permission to read it.
    let mut allowed = false;
    if let Some(url) = url.as_ref() {
        allowed = unsafe { msg_send![url, startAccessingSecurityScopedResource] };
    }
    let contents = path.as_ref().and_then(|path| std::fs::read(path).ok());
    if allowed {
        if let Some(url) = url.as_ref() {
            let _: () = unsafe { msg_send![url, stopAccessingSecurityScopedResource] };
        }
    }
    Some(Chosen {
        name,
        path,
        bytes: contents,
    })
}
