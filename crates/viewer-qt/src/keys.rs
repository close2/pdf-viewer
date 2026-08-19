//! What a key press means, in `Qt::Key` numbers.
//!
//! **The mapping is on this side deliberately.** `viewer-gtk` reads a `gdk::Key` in Rust and
//! decides there what the key means; a Qt host could as easily have decided it in C++ and sent a
//! command number. Keeping it here is what makes the two hosts comparable — what a key *means* is
//! this project's reading of §12.5.1 and §12.3.3 rather than a fact about a toolkit, and the two
//! hosts should be seen to agree about it in the same kind of code.
//!
//! The cost of that choice is this module: a table of `Qt::Key` constants written out by hand,
//! because a C++ enumerator is not a Rust one and `cxx` carries no enumeration this crate wants
//! to own. Every value below is from `/usr/include/qt6/QtCore/qnamespace.h`.

use viewer_core::{Command, PageTarget, Selection, Zoom};

/// `Qt::Key_Escape`.
const ESCAPE: u32 = 0x0100_0000;
/// `Qt::Key_Home`.
const HOME: u32 = 0x0100_0010;
/// `Qt::Key_End`.
const END: u32 = 0x0100_0011;
/// `Qt::Key_Left`.
const LEFT: u32 = 0x0100_0012;
/// `Qt::Key_Up`.
const UP: u32 = 0x0100_0013;
/// `Qt::Key_Right`.
const RIGHT: u32 = 0x0100_0014;
/// `Qt::Key_Down`.
const DOWN: u32 = 0x0100_0015;
/// `Qt::Key_PageUp`.
const PAGE_UP: u32 = 0x0100_0016;
/// `Qt::Key_PageDown`.
const PAGE_DOWN: u32 = 0x0100_0017;
/// `Qt::Key_Plus`.
const PLUS: u32 = 0x2b;
/// `Qt::Key_Minus`.
const MINUS: u32 = 0x2d;
/// `Qt::Key_Equal`.
const EQUAL: u32 = 0x3d;
/// `Qt::Key_0`.
const ZERO: u32 = 0x30;
/// `Qt::Key_A`.
const A: u32 = 0x41;
/// `Qt::Key_S`.
const S: u32 = 0x53;
/// `Qt::Key_W`, and the one key this module names without giving it a command.
///
/// What `w` means is *"magnify until every §12.7 control fits the `/Rect` the document states for
/// it"*, and the number that makes that true is what the page's controls measured — so the
/// command cannot be built from the key alone. `crate::host` binds it; the constant is here so
/// that the table of `Qt::Key` numbers stays in one file.
pub(crate) const FIT_CONTROLS: u32 = 0x57;

/// `Qt::Key_Y`.
const Y: u32 = 0x59;
/// `Qt::Key_Z`.
const Z: u32 = 0x5a;

/// The command a key press is, or nothing for a key this host does not use.
///
/// The same set `viewer-gtk` binds, in the same order, so that the one difference between the two
/// programs is which toolkit drew the window.
#[must_use]
pub(crate) fn command(code: u32) -> Option<Command> {
    match code {
        RIGHT | DOWN | PAGE_DOWN => Some(Command::GoTo(PageTarget::Next)),
        LEFT | UP | PAGE_UP => Some(Command::GoTo(PageTarget::Previous)),
        HOME => Some(Command::GoTo(PageTarget::First)),
        END => Some(Command::GoTo(PageTarget::Last)),
        PLUS | EQUAL => Some(Command::Zoom {
            zoom: Zoom::In,
            at: None,
        }),
        MINUS => Some(Command::Zoom {
            zoom: Zoom::Out,
            at: None,
        }),
        ZERO => Some(Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        }),
        A => Some(Command::Select(Selection::All)),
        ESCAPE => Some(Command::Select(Selection::None)),
        S => Some(Command::Save),
        Z => Some(Command::Undo),
        Y => Some(Command::Redo),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::command;
    use viewer_core::{Command, PageTarget};

    /// The four keys a page is turned with, checked against the numbers Qt states for them.
    ///
    /// Worth a test because the constants are transcribed rather than imported: a Qt enumerator
    /// cannot reach Rust without a bridge, so the one thing that could go wrong here is a digit.
    #[test]
    fn the_page_turning_keys_are_the_numbers_qt_states() {
        assert!(matches!(
            command(0x0100_0014),
            Some(Command::GoTo(PageTarget::Next))
        ));
        assert!(matches!(
            command(0x0100_0012),
            Some(Command::GoTo(PageTarget::Previous))
        ));
        assert!(matches!(
            command(0x0100_0010),
            Some(Command::GoTo(PageTarget::First))
        ));
        assert!(matches!(
            command(0x0100_0011),
            Some(Command::GoTo(PageTarget::Last))
        ));
    }

    /// A key this host does not bind produces no command rather than a default one.
    #[test]
    fn an_unbound_key_is_nothing_rather_than_something() {
        assert!(command(0x42).is_none());
    }

    /// And `w` is bound without being in this table, which is the one case worth stating.
    ///
    /// A later session adding `Zoom::Scale` here would be inventing the magnification, because
    /// the number belongs to the controls the page measured rather than to the key.
    #[test]
    fn the_fit_key_is_the_hosts_rather_than_this_tables() {
        assert!(command(super::FIT_CONTROLS).is_none());
    }
}
