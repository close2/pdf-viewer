//! A file or a URL handed to the program hosting this window, which opens it with the platform's
//! own opener (ADR 3217).
//!
//! The GTK and Qt windows hand a clip to the desktop portal, and a launched document and
//! §12.6.4.8's link to `xdg-open` (ADRs 3202, 2171, 1155). A `quorra_window` is hosted by a program that is the platform's
//! own — the `WinUI` window, through `Windows.System.Launcher` — and it has the opener this library
//! does not, so every hand-off becomes a record the host carries out: [`super::Owed::OpenFile`] or
//! [`super::Owed::OpenUri`]. What is decided before the hand-off — the level, the question, the
//! refusals at every level, the private directory a clip is written into — is the same code every
//! window runs, and only the last step moves across the boundary. The opener answers later, so a
//! failure comes back through `quorra_window_open_failed` and is said as a portal's failure after
//! its call is (ADR 3219).

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;

use viewer_host::native_window::NativeWindow as _;
use viewer_host::policy::Opener;

use super::{Owed, Window};

/// How the host's opener is named in what the window says.
pub(super) const HOST_OPENER: &str =
    "the program hosting this window, for the platform's own opener";

/// The [`Opener`] that collects what is to be opened, for the window to owe once
/// `viewer_host::policy::hand_media` has decided and written it.
#[derive(Debug, Default)]
struct ForTheHost {
    opened: RefCell<Vec<Owed>>,
}

/// A path as the C ABI's text carries it, or why it cannot be: a path that is not UTF-8 would
/// name another file once converted, so it is refused rather than converted lossily.
fn path_text(path: &Path) -> Result<String, String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        format!(
            "its path {} is not UTF-8, which is what this window can tell its host",
            path.display()
        )
    })
}

impl Opener for ForTheHost {
    /// The descriptor is closed here: the host opens the file by its path, as the platform's opener
    /// takes it.
    fn open_file(&self, file: File, path: &Path) -> Result<(), String> {
        drop(file);
        let text = path_text(path)?;
        self.opened.borrow_mut().push(Owed::OpenFile(text));
        Ok(())
    }

    fn open_uri(&self, uri: &str) -> Result<(), String> {
        self.opened.borrow_mut().push(Owed::OpenUri(uri.to_owned()));
        Ok(())
    }

    fn name(&self) -> String {
        HOST_OPENER.to_owned()
    }
}

impl Window {
    /// [`viewer_host::native_window::NativeWindow::hand_media_over`] for a window whose host opens
    /// files: the clip decided and written as every window's is, then owed to the host to open.
    pub(super) fn media_for_the_host(
        &mut self,
        document: &str,
        handed: &viewer_host::Handed,
        note: Option<&str>,
    ) {
        let host = ForTheHost::default();
        let said = viewer_host::policy::hand_over_media_to(document, handed, note, &host);
        for owed in host.opened.into_inner() {
            self.owing.owe(owed);
        }
        self.say(&said);
    }

    /// [`viewer_host::native_window::NativeWindow::hand_launched_over`] for a window whose host
    /// opens files.
    pub(super) fn launched_for_the_host(&mut self, name: &str, path: &Path) {
        match path_text(path) {
            Ok(text) => {
                self.owing.owe(Owed::OpenFile(text));
                self.say(&viewer_host::policy::handed_to(name, HOST_OPENER));
            }
            Err(why) => self.say(&format!("Launch: {name} could not be handed over — {why}")),
        }
    }

    /// [`viewer_host::native_window::NativeWindow::hand_link_over`] for a window whose host opens
    /// URIs: §12.6.4.8's link, which the level or the person let through, owed as the record a
    /// media clip's URL is (ADR 3280). The schemes handed over are every window's, checked here
    /// before the host is given anything.
    pub(super) fn link_for_the_host(&mut self, uri: &str) -> Result<(), String> {
        viewer_host::handable_uri(uri)?;
        self.owing.owe(Owed::OpenUri(uri.to_owned()));
        Ok(())
    }

    /// The host's opener answered that it did not open `handed`, an `OpenFile` record's path or
    /// an `OpenUri` record's URL, for `why`: said in this window, as a portal's failure after its
    /// call is (ADR 3219).
    pub fn open_failed(&mut self, handed: &str, why: &str) {
        self.say(&viewer_host::policy::not_opened(HOST_OPENER, handed, why));
    }
}
