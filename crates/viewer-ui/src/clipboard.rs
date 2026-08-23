//! The platform's clipboard, which is the one thing a selection needs in order to leave.
//!
//! **Why this host has a module and the other two have a line.** `viewer-gtk` asks its widget for
//! a `gdk::Clipboard` and `viewer-qt` asks `QGuiApplication` for a `QClipboard`; both already link
//! a toolkit whose business this is. This host draws its own chrome on a `winit` window, and
//! `winit` offers no clipboard **deliberately** — on X11 a clipboard is not a property of a window
//! at all but a *selection owner*, a service that has to answer `SelectionRequest` for as long as
//! the program lives — so it is the one consumer in this tree that has to name a dependency for
//! it. The workspace manifest prices `arboard` and says which of its optional halves are off; ADR
//! 0519 has the decision.
//!
//! **Nothing here runs on the launch path**, which is `CLAUDE.md`'s second principle: the
//! connection is made the first time somebody copies and never before, because a clipboard is not
//! needed to show page one. [`Clipboard::connected`] is what makes that checkable rather than
//! promised.
//!
//! **A refusal is loud** (trap 5). Without a display connection — a Wayland session with no
//! `XWayland` is the case that actually happens — there is no clipboard to put anything on, and
//! the caller is told so by name instead of watching a copy do nothing.

use thiserror::Error;

/// Why a copy could not reach the platform.
#[derive(Debug, Error)]
pub enum ClipboardError {
    /// There is no clipboard to connect to.
    ///
    /// The case that happens is a Wayland session with no `XWayland`: `arboard`'s X11 backend is
    /// the one this program builds, and it has nothing to talk to. The message is the platform's,
    /// because a guess about which of several causes it was would be worse than quoting it.
    #[error("this session offers no clipboard: {0}")]
    Unavailable(String),
    /// The connection exists and the platform refused the text.
    #[error("the platform refused the copy: {0}")]
    Refused(String),
}

/// This program's end of the platform's clipboard, connected on first use.
///
/// Held across copies rather than made per copy, and that is not an optimisation: on X11 the
/// program that owns a selection is the program that has to hand it over when another application
/// asks, so dropping the connection after a copy would put text on a clipboard that stops
/// answering. `arboard` keeps that service on a thread of its own behind this handle.
#[derive(Default)]
pub struct Clipboard {
    /// `None` until the first copy, which is what keeps this off the launch path.
    platform: Option<arboard::Clipboard>,
}

impl core::fmt::Debug for Clipboard {
    /// By hand because `arboard::Clipboard` is not [`Debug`], and what is worth printing about
    /// this is the one thing a launch measurement asks: whether it has connected yet.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Clipboard")
            .field("connected", &self.connected())
            .finish()
    }
}

impl Clipboard {
    /// A clipboard this program has not connected to, which costs nothing at all.
    #[must_use]
    pub const fn new() -> Self {
        Self { platform: None }
    }

    /// Whether the platform connection has been made yet.
    ///
    /// Public so that a launch measurement can assert it is still `false`: the "nothing eager"
    /// rule of `CLAUDE.md`'s second principle is one a test can check rather than a habit.
    #[must_use]
    pub const fn connected(&self) -> bool {
        self.platform.is_some()
    }

    /// Puts `text` where another application can take it, connecting if this is the first copy.
    ///
    /// # Errors
    ///
    /// [`ClipboardError::Unavailable`] where the session offers no clipboard at all, and
    /// [`ClipboardError::Refused`] where it has one and would not take this text.
    pub fn put(&mut self, text: &str) -> Result<(), ClipboardError> {
        if self.platform.is_none() {
            let opened = arboard::Clipboard::new()
                .map_err(|error| ClipboardError::Unavailable(error.to_string()))?;
            self.platform = Some(opened);
        }
        let Some(platform) = self.platform.as_mut() else {
            // Unreachable: the branch above either filled it or returned. Written as a `let else`
            // rather than an `expect` because this crate has no `unwrap` outside tests, and the
            // refusal is the same typed one a caller already handles.
            return Err(ClipboardError::Unavailable(
                "the connection made above is gone".to_owned(),
            ));
        };
        platform
            .set_text(text)
            .map_err(|error| ClipboardError::Refused(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::Clipboard;

    /// Making one connects to nothing, which is the launch-path rule stated as a test.
    ///
    /// `CLAUDE.md`'s second principle forbids anything on the launch path that page one does not
    /// need, and a clipboard is the clearest instance there is: nobody has pressed copy yet. A
    /// round that moved the connection into a constructor would break this and nothing else.
    #[test]
    fn a_clipboard_connects_to_nothing_until_somebody_copies() {
        let clipboard = Clipboard::new();
        assert!(
            !clipboard.connected(),
            "a clipboard made at startup has talked to no platform"
        );
        assert!(
            format!("{clipboard:?}").contains("connected: false"),
            "and says so, because that is the one thing worth printing about it"
        );
    }
}
