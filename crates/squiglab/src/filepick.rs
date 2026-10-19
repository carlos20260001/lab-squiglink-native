//! Asking the user for a file.
//!
//! The desktop has a dialog of its own, the browser has the file input the
//! web has always had, and the phone has the system picker. They are all
//! behind this one door, so that the panels that need a file do not have to
//! know where they are running.
//!
//! Where there is a filesystem the file is named by its path; where there is
//! not, as in the browser, the bytes come back instead.

use std::path::PathBuf;

#[cfg(target_os = "ios")]
#[path = "filepick_ios.rs"]
mod ios;

/// A file the user has chosen.
#[derive(Clone, Debug)]
pub struct Chosen {
    /// The file's name.
    pub name: String,
    /// Where the file is, where there is a filesystem to name.
    pub path: Option<PathBuf>,
    /// What the file holds, where there is no filesystem to read it from.
    pub bytes: Option<Vec<u8>>,
}

impl Chosen {
    /// The file's text, whether it came from a path or from bytes.
    pub fn text(&self) -> Option<String> {
        match (&self.path, &self.bytes) {
            (Some(path), _) => std::fs::read_to_string(path).ok(),
            (None, Some(bytes)) => String::from_utf8(bytes.clone()).ok(),
            (None, None) => None,
        }
    }

    /// The file's bytes, whether they came from a path or from the picker.
    pub fn contents(&self) -> Option<Vec<u8>> {
        match (&self.path, &self.bytes) {
            (Some(path), _) => std::fs::read(path).ok(),
            (None, Some(bytes)) => Some(bytes.clone()),
            (None, None) => None,
        }
    }
}

/// The picker, waiting for the user to choose a file.
///
/// The dialog is the system's own and it is opened on its own thread, so the
/// window keeps drawing while it is open. The result is collected with
/// [`Picker::take`], which is called once a frame.
pub struct Picker {
    inner: imp::Inner,
}

impl Picker {
    /// Ask for a file, with the given name and extensions.
    pub fn start(title: &str, extensions: &[&str]) -> Self {
        Self {
            inner: imp::Inner::start(title, extensions),
        }
    }

    /// The file, if the user has chosen one.
    pub fn take(&mut self) -> Option<Chosen> {
        self.inner.take()
    }

    /// Whether the dialog is still open.
    pub fn waiting(&self) -> bool {
        self.inner.waiting()
    }
}

/// The desktop and the browser: the dialog both of them have.
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows",
    target_arch = "wasm32"
))]
mod imp {
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::task::Poll;

    use super::Chosen;

    /// The dialog, and the future it comes back on.
    pub struct Inner {
        future: Option<Pin<Box<dyn std::future::Future<Output = Option<Chosen>>>>>,
        chosen: Option<Chosen>,
    }

    impl Inner {
        pub fn start(title: &str, extensions: &[&str]) -> Self {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(title);
            if !extensions.is_empty() {
                let names: Vec<String> = extensions.iter().map(|one| one.to_string()).collect();
                dialog = dialog.add_filter(title, &names);
            }
            let future = Box::pin(async move {
                let handle = dialog.pick_file().await?;
                Some(chosen_of(handle).await)
            });
            Self {
                future: Some(future),
                chosen: None,
            }
        }

        pub fn take(&mut self) -> Option<Chosen> {
            if self.chosen.is_none() {
                if let Some(future) = &mut self.future {
                    if let Some(chosen) = ready(future.as_mut()) {
                        self.chosen = chosen;
                        self.future = None;
                    }
                }
            }
            self.chosen.take()
        }

        pub fn waiting(&self) -> bool {
            self.chosen.is_none() && self.future.is_some()
        }
    }

    /// Whether the dialog has an answer yet, without waiting for one.
    ///
    /// The dialog is asked once a frame, so the window keeps drawing while it
    /// is open and the answer is taken the moment it arrives.
    fn ready(future: Pin<&mut dyn Future<Output = Option<Chosen>>>) -> Option<Option<Chosen>> {
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        match future.poll(&mut context) {
            Poll::Ready(chosen) => Some(chosen),
            Poll::Pending => None,
        }
    }

    /// What the user chose.
    #[cfg(not(target_arch = "wasm32"))]
    async fn chosen_of(handle: rfd::FileHandle) -> Chosen {
        let name = handle.file_name();
        let path = Some(PathBuf::from(handle.path()));
        Chosen {
            name,
            path,
            bytes: None,
        }
    }

    /// What the user chose. In a browser there is no path, only the bytes.
    #[cfg(target_arch = "wasm32")]
    async fn chosen_of(handle: rfd::FileHandle) -> Chosen {
        let name = handle.file_name();
        let bytes = handle.read().await;
        Chosen {
            name,
            path: None::<PathBuf>,
            bytes: Some(bytes),
        }
    }
}

/// The phone: the system document picker, written against the system's own
/// libraries rather than taken from a third party.
#[cfg(target_os = "ios")]
mod imp {
    use std::path::PathBuf;
    use std::sync::mpsc::{channel, Receiver, TryRecvError};

    use super::Chosen;

    /// The picker, and the thread it comes back on.
    pub struct Inner {
        receiver: Option<Receiver<Chosen>>,
        chosen: Option<Chosen>,
    }

    impl Inner {
        pub fn start(title: &str, extensions: &[&str]) -> Self {
            let (sender, receiver) = channel();
            crate::filepick::ios::ask(title, extensions, move |chosen| {
                let _ = sender.send(chosen);
            });
            Self {
                receiver: Some(receiver),
                chosen: None,
            }
        }

        pub fn take(&mut self) -> Option<Chosen> {
            if self.chosen.is_none() {
                if let Some(receiver) = &self.receiver {
                    if let Ok(chosen) = receiver.try_recv() {
                        self.chosen = Some(chosen);
                    }
                }
            }
            self.chosen.take()
        }

        pub fn waiting(&self) -> bool {
            self.chosen.is_none() && self.receiver.is_some()
        }
    }

    /// The file the picker gives back is named by its path.
    pub fn chosen_of(name: String, path: PathBuf) -> Chosen {
        Chosen {
            name,
            path: Some(path),
            bytes: None,
        }
    }
}

/// Anywhere else there is no dialog, and the path is typed as it always was.
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows",
    target_os = "ios",
    target_arch = "wasm32"
)))]
mod imp {
    use super::Chosen;

    /// There is nothing to wait for.
    pub struct Inner;

    impl Inner {
        pub fn start(_title: &str, _extensions: &[&str]) -> Self {
            Self
        }

        pub fn take(&mut self) -> Option<Chosen> {
            None
        }

        pub fn waiting(&self) -> bool {
            false
        }
    }
}
