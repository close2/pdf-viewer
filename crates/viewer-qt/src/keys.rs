//! `Qt::Key` numbers, and the one thing this host contributes to its own key bindings.
//!
//! **The mapping is on this side deliberately.** A Qt host could as easily have decided what a key
//! means in C++ and sent a command number; keeping it here is what makes the three hosts
//! comparable, because what a key *means* is this project's reading of §12.5.1 and §12.4.4.2 plus a
//! page of documented choices, and the three hosts should be seen to agree about it in the same
//! kind of code.
//!
//! **And since the six-hundred-and-eighty-seventh session that agreement is a value rather than a
//! resemblance** (ADR 0526). This module used to carry a table of its own — `Qt::Key` to
//! [`viewer_core::Command`] — beside two others that disagreed with it about the arrow keys, about
//! `f` and about Escape. What is left is a *translation*: a `Qt::Key` becomes a
//! [`viewer_host::Key`] and [`viewer_host::meaning`] says what it means.
//!
//! The cost of that choice is the table below: `Qt::Key` constants written out by hand, because a
//! C++ enumerator is not a Rust one and `cxx` carries no enumeration this crate wants to own. Every
//! value is from `/usr/include/qt6/QtCore/qnamespace.h`.

use viewer_host::Key as Stated;

/// `Qt::Key_Escape`.
///
/// Named because `cpp/window.cpp` forwards this one key by number: a `QAction` shortcut consumes
/// it before `keyPressEvent` ever runs, so the find bar's close action is the only place Escape
/// arrives in that window and it hands the key back here rather than deciding for itself.
pub(crate) const ESCAPE: u32 = 0x0100_0000;
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
/// `Qt::Key_Tab`, which §12.5.1 names.
const TAB: u32 = 0x0100_0001;
/// `Qt::Key_Backtab`, which is what Qt reports for Shift and Tab together.
const BACKTAB: u32 = 0x0100_0002;
/// `Qt::Key_Space`.
const SPACE: u32 = 0x20;
/// `Qt::Key_Plus`.
const PLUS: u32 = 0x2b;
/// `Qt::Key_Minus`.
const MINUS: u32 = 0x2d;
/// `Qt::Key_Equal`.
const EQUAL: u32 = 0x3d;
/// `Qt::Key_Slash`.
const SLASH: u32 = 0x2f;
/// `Qt::Key_Question`.
const QUESTION: u32 = 0x3f;
/// `Qt::Key_0`.
const ZERO: u32 = 0x30;
/// `Qt::Key_A`.
const A: u32 = 0x41;
/// `Qt::Key_C`.
const C: u32 = 0x43;
/// `Qt::Key_F`.
const F: u32 = 0x46;
/// `Qt::Key_H`.
const H: u32 = 0x48;
/// `Qt::Key_K`.
const K: u32 = 0x4b;
/// `Qt::Key_L`.
const L: u32 = 0x4c;
/// `Qt::Key_O`.
const O: u32 = 0x4f;
/// `Qt::Key_P`.
const P: u32 = 0x50;
/// `Qt::Key_S`.
const S: u32 = 0x53;
/// `Qt::Key_T`.
const T: u32 = 0x54;
/// `Qt::Key_W`.
const W: u32 = 0x57;
/// `Qt::Key_Y`.
const Y: u32 = 0x59;
/// `Qt::Key_Z`.
const Z: u32 = 0x5a;

/// The key [`viewer_host::keys`] states a meaning for, or nothing for one it does not name.
///
/// Qt reports a letter as its **upper-case** enumerator whatever the Shift key was doing, which is
/// why there is one constant per letter here and two per letter in `viewer-gtk`.
#[must_use]
pub(crate) fn stated(code: u32) -> Option<Stated> {
    Some(match code {
        A => Stated::A,
        C => Stated::C,
        F => Stated::F,
        H => Stated::H,
        K => Stated::K,
        L => Stated::L,
        O => Stated::O,
        P => Stated::P,
        S => Stated::S,
        T => Stated::T,
        W => Stated::W,
        Y => Stated::Y,
        Z => Stated::Z,
        ZERO => Stated::Zero,
        PLUS => Stated::Plus,
        MINUS => Stated::Minus,
        EQUAL => Stated::Equals,
        SLASH => Stated::Slash,
        QUESTION => Stated::Question,
        ESCAPE => Stated::Escape,
        TAB | BACKTAB => Stated::Tab,
        SPACE => Stated::Space,
        HOME => Stated::Home,
        END => Stated::End,
        LEFT => Stated::Left,
        RIGHT => Stated::Right,
        UP => Stated::Up,
        DOWN => Stated::Down,
        PAGE_UP => Stated::PageUp,
        PAGE_DOWN => Stated::PageDown,
        _ => return None,
    })
}

/// Whether Qt's own name for the key already says Shift was held.
///
/// `Qt::Key_Backtab` is what a Qt window reports for Shift and Tab together, so the direction
/// §12.5.1's key moves in can arrive without the modifier state at all — and it has to be folded
/// back in, because [`viewer_host::meaning`] asks the same question of all three hosts.
#[must_use]
pub(crate) const fn shifted_by_name(code: u32) -> bool {
    code == BACKTAB
}

#[cfg(test)]
mod tests {
    use super::{shifted_by_name, stated};

    /// Every key the shared table states has a `Qt::Key` in this host.
    ///
    /// **This is the instrument the level-hosts decision never had** (ADR 0526). The match is
    /// exhaustive over [`viewer_host::Key`], so a binding added to `viewer-host` fails to compile
    /// here until this host says which number produces it, and the assertion then checks that the
    /// *runtime* translation agrees. `viewer-gtk` and `viewer-ui` carry the same test against their
    /// own toolkits.
    ///
    /// It is also the check the numbers themselves want: they are transcribed from
    /// `qnamespace.h` rather than imported, so the one thing that can go wrong here is a digit.
    #[test]
    fn every_key_the_table_states_has_one_in_this_toolkit() {
        use viewer_host::Key as Stated;
        for key in Stated::ALL {
            let code = match key {
                Stated::A => 0x41,
                Stated::C => 0x43,
                Stated::F => 0x46,
                Stated::H => 0x48,
                Stated::K => 0x4b,
                Stated::L => 0x4c,
                Stated::O => 0x4f,
                Stated::P => 0x50,
                Stated::S => 0x53,
                Stated::T => 0x54,
                Stated::W => 0x57,
                Stated::Y => 0x59,
                Stated::Z => 0x5a,
                Stated::Zero => 0x30,
                Stated::Plus => 0x2b,
                Stated::Minus => 0x2d,
                Stated::Equals => 0x3d,
                Stated::Slash => 0x2f,
                Stated::Question => 0x3f,
                Stated::Escape => 0x0100_0000,
                Stated::Tab => 0x0100_0001,
                Stated::Space => 0x20,
                Stated::Home => 0x0100_0010,
                Stated::End => 0x0100_0011,
                Stated::Left => 0x0100_0012,
                Stated::Right => 0x0100_0014,
                Stated::Up => 0x0100_0013,
                Stated::Down => 0x0100_0015,
                Stated::PageUp => 0x0100_0016,
                Stated::PageDown => 0x0100_0017,
            };
            assert_eq!(
                stated(code),
                Some(*key),
                "{key:?} is stated by the table and this host does not produce it"
            );
        }
    }

    /// Qt reports Shift and Tab as a key of its own, and §12.5.1's direction has to survive it.
    #[test]
    fn backtab_is_the_tab_key_with_shift_already_in_it() {
        assert_eq!(stated(0x0100_0002), Some(viewer_host::Key::Tab));
        assert!(shifted_by_name(0x0100_0002));
        assert!(!shifted_by_name(0x0100_0001));
    }

    /// A key this host does not bind produces nothing rather than a default one.
    #[test]
    fn an_unbound_key_is_nothing_rather_than_something() {
        assert_eq!(stated(0x42), None);
    }
}
