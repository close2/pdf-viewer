//! What a key press means, crossed to C: `viewer_host::keys`' table under numbers a header can
//! state (RFC 0010 step N1, ADR 2072).
//!
//! # Why the table crosses rather than being copied
//!
//! Three windowed hosts with three key tables disagreed about the arrows, about `f` and about
//! Escape until ADR 0526 put the table in `viewer-host` and had each host supply only the
//! translation of its toolkit's key. A C caller is the fourth host, and a .NET window re-writing
//! [`viewer_host::meaning`] in C# would be the copy `doc/todo/30` warns of. So the caller translates
//! its key into a `QUORRA_KEY_*` and asks; what it is handed back is the press already acted on
//! where acting needs nothing of the window, and a `QUORRA_ACT_*` where it does.
//!
//! # The numbers are this ABI's, written out
//!
//! [`KeyKind`] and [`ActKind`] number `viewer_host::Key` and `viewer_host::WindowAct` explicitly
//! rather than by position in [`viewer_host::Key::ALL`]: that list is `viewer-host`'s to order, and
//! a key inserted in its middle would renumber every key after it under a caller already compiled.
//! A new number is appended. Both conversions from `viewer-host`'s values are exhaustive matches
//! with no catch-all arm, so a key or an act added there fails to compile here — the mechanism
//! `doc/ui-boundary.md` prefers, in the one crate where the C side cannot have it.
//!
//! # The modifier is named by its role
//!
//! `command` is the platform's command modifier: Control on Windows and Linux, Command on macOS
//! (RFC 0010 section 5). [`viewer_host::Modifiers::ctrl`] is that role under the name of the key
//! that plays it on two of three platforms, and `quorra_keys` already names it `modifier` for
//! Adobe's `event.modifier`.

use viewer_core::{Answer, Command, Edit, Query, Viewer};
use viewer_host::{Key, Meaning, Mode, Modifiers, Waiting, WindowAct};

/// What every window says when §12.5.6.10's markup key is pressed with nothing selected.
///
/// The core answers a markup over nothing by doing nothing, which is right and silent, and a
/// person who pressed a key and saw no change has been told nothing at all (trap 5). The three
/// windows say this sentence in their own key handlers; the library says it for a C caller.
pub(crate) const SELECT_FIRST: &str = "select some text first — §12.5.6.10's markups mark up text";

/// A key this program binds, as a C caller names it: `QUORRA_KEY_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum KeyKind {
    /// The letter `a`.
    A = 0,
    /// The letter `c`.
    C = 1,
    /// The letter `f`.
    F = 2,
    /// The letter `h`.
    H = 3,
    /// The letter `k`.
    K = 4,
    /// The letter `l`.
    L = 5,
    /// The letter `m`.
    M = 6,
    /// The letter `o`.
    O = 7,
    /// The letter `p`.
    P = 8,
    /// The letter `r`.
    R = 9,
    /// The letter `s`.
    S = 10,
    /// The letter `t`.
    T = 11,
    /// The letter `w`.
    W = 12,
    /// The letter `y`.
    Y = 13,
    /// The letter `z`.
    Z = 14,
    /// The digit `0`.
    Zero = 15,
    /// `+`.
    Plus = 16,
    /// `-`.
    Minus = 17,
    /// `=`.
    Equals = 18,
    /// `/`.
    Slash = 19,
    /// `?`.
    Question = 20,
    /// Escape.
    Escape = 21,
    /// Tab.
    Tab = 22,
    /// The space bar.
    Space = 23,
    /// Enter, or Return.
    Enter = 24,
    /// Home.
    Home = 25,
    /// End.
    End = 26,
    /// The Left arrow.
    Left = 27,
    /// The Right arrow.
    Right = 28,
    /// The Up arrow.
    Up = 29,
    /// The Down arrow.
    Down = 30,
    /// Page Up.
    PageUp = 31,
    /// Page Down.
    PageDown = 32,
}

impl KeyKind {
    /// How many keys this build binds: `QUORRA_KEY_COUNT`.
    ///
    /// Not part of `quorra_abi_check`, for `QUORRA_CONTROL_KIND_COUNT`'s reason: a key is a number
    /// the caller passes, and one this build does not define is refused by name. What the count is
    /// for is the other direction — a caller held to translating every key the library binds, as
    /// each window is held to [`Key::ALL`].
    pub const COUNT: u32 = 33;

    /// The number `viewer-host`'s key has here. Exhaustive with no catch-all arm.
    #[must_use]
    pub const fn of(key: Key) -> Self {
        match key {
            Key::A => Self::A,
            Key::C => Self::C,
            Key::F => Self::F,
            Key::H => Self::H,
            Key::K => Self::K,
            Key::L => Self::L,
            Key::M => Self::M,
            Key::O => Self::O,
            Key::P => Self::P,
            Key::R => Self::R,
            Key::S => Self::S,
            Key::T => Self::T,
            Key::W => Self::W,
            Key::Y => Self::Y,
            Key::Z => Self::Z,
            Key::Zero => Self::Zero,
            Key::Plus => Self::Plus,
            Key::Minus => Self::Minus,
            Key::Equals => Self::Equals,
            Key::Slash => Self::Slash,
            Key::Question => Self::Question,
            Key::Escape => Self::Escape,
            Key::Tab => Self::Tab,
            Key::Space => Self::Space,
            Key::Enter => Self::Enter,
            Key::Home => Self::Home,
            Key::End => Self::End,
            Key::Left => Self::Left,
            Key::Right => Self::Right,
            Key::Up => Self::Up,
            Key::Down => Self::Down,
            Key::PageUp => Self::PageUp,
            Key::PageDown => Self::PageDown,
        }
    }

    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::A,
            1 => Self::C,
            2 => Self::F,
            3 => Self::H,
            4 => Self::K,
            5 => Self::L,
            6 => Self::M,
            7 => Self::O,
            8 => Self::P,
            9 => Self::R,
            10 => Self::S,
            11 => Self::T,
            12 => Self::W,
            13 => Self::Y,
            14 => Self::Z,
            15 => Self::Zero,
            16 => Self::Plus,
            17 => Self::Minus,
            18 => Self::Equals,
            19 => Self::Slash,
            20 => Self::Question,
            21 => Self::Escape,
            22 => Self::Tab,
            23 => Self::Space,
            24 => Self::Enter,
            25 => Self::Home,
            26 => Self::End,
            27 => Self::Left,
            28 => Self::Right,
            29 => Self::Up,
            30 => Self::Down,
            31 => Self::PageUp,
            32 => Self::PageDown,
            _ => return None,
        })
    }

    /// What `viewer-host` calls it.
    #[must_use]
    pub const fn key(self) -> Key {
        match self {
            Self::A => Key::A,
            Self::C => Key::C,
            Self::F => Key::F,
            Self::H => Key::H,
            Self::K => Key::K,
            Self::L => Key::L,
            Self::M => Key::M,
            Self::O => Key::O,
            Self::P => Key::P,
            Self::R => Key::R,
            Self::S => Key::S,
            Self::T => Key::T,
            Self::W => Key::W,
            Self::Y => Key::Y,
            Self::Z => Key::Z,
            Self::Zero => Key::Zero,
            Self::Plus => Key::Plus,
            Self::Minus => Key::Minus,
            Self::Equals => Key::Equals,
            Self::Slash => Key::Slash,
            Self::Question => Key::Question,
            Self::Escape => Key::Escape,
            Self::Tab => Key::Tab,
            Self::Space => Key::Space,
            Self::Enter => Key::Enter,
            Self::Home => Key::Home,
            Self::End => Key::End,
            Self::Left => Key::Left,
            Self::Right => Key::Right,
            Self::Up => Key::Up,
            Self::Down => Key::Down,
            Self::PageUp => Key::PageUp,
            Self::PageDown => Key::PageDown,
        }
    }
}

/// What is left for the window to do once the library has acted on a press: `QUORRA_ACT_*`.
///
/// **Answered, so it is named and counted**, as a control kind is. Two values are not
/// `viewer_host::WindowAct`s: [`Self::Nothing`], for a press that needed nothing of the window —
/// a key with no meaning, or one whose command the library has already sent — and [`Self::Say`],
/// for a press the window owes the person a sentence about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum ActKind {
    /// Nothing for the window to do: the events are the whole answer.
    Nothing = 0,
    /// Say `quorra_key_said`'s sentence where this window says things.
    Say = 1,
    /// Move the view by `by` logical pixels, downwards for a positive number.
    ScrollBy = 2,
    /// Copy what is selected: `quorra_copy`, then the platform's clipboard.
    Copy = 3,
    /// Show the find bar, or put the keyboard back in it.
    Find = 4,
    /// Show or hide the panel of trees.
    Panel = 5,
    /// Show the levels this reader has set for what a document asserts over them.
    Restrictions = 6,
    /// Show the third-party notices this binary carries.
    Notices = 7,
    /// Start §12.4.4's presentation, or stop the one that is running.
    Present = 8,
    /// Open this window's print dialogue.
    Print = 9,
    /// Convert to ISO 19005 at the target and profile the window holds.
    Archive = 10,
    /// Put the archive chooser up again.
    ChooseArchive = 11,
    /// Leave full screen.
    LeaveFullScreen = 12,
    /// Move on to the next of Table 29's six arrangements.
    NextLayout = 13,
    /// Magnify until every §12.7 control fits the `/Rect` its document states.
    FitControls = 14,
    /// Arm §12.5.6.6's free-text drag.
    FreeText = 15,
    /// Start or stop §12.9's measuring.
    Measure = 16,
    /// Stop the draw the window has warned about.
    AbortDrawing = 17,
    /// Show the next of the documents this window has open.
    NextDocument = 18,
    /// Close the document in front, or the window where it is the only one.
    CloseDocument = 19,
    /// Ask the person for a document and open it beside the one showing.
    OpenDocument = 20,
    /// Turn §10.8.3's separation simulation on, or off again.
    Separations = 21,
}

impl ActKind {
    /// How many acts this build answers with: `QUORRA_ACT_COUNT`. Not part of `quorra_abi_check`,
    /// for [`KeyKind::COUNT`]'s reason; `quorra_act_name` names one a caller has no arm for.
    pub const COUNT: u32 = 22;

    /// The act a window act is, and the distance [`WindowAct::ScrollBy`] carries (zero for every
    /// other). Exhaustive with no catch-all arm.
    #[must_use]
    pub const fn of(act: WindowAct) -> (Self, f32) {
        let kind = match act {
            WindowAct::ScrollBy(by) => return (Self::ScrollBy, by),
            WindowAct::Copy => Self::Copy,
            WindowAct::Find => Self::Find,
            WindowAct::Panel => Self::Panel,
            WindowAct::Restrictions => Self::Restrictions,
            WindowAct::Notices => Self::Notices,
            WindowAct::Present => Self::Present,
            WindowAct::Print => Self::Print,
            WindowAct::Archive => Self::Archive,
            WindowAct::ChooseArchive => Self::ChooseArchive,
            WindowAct::LeaveFullScreen => Self::LeaveFullScreen,
            WindowAct::NextLayout => Self::NextLayout,
            WindowAct::FitControls => Self::FitControls,
            WindowAct::FreeText => Self::FreeText,
            WindowAct::Measure => Self::Measure,
            WindowAct::AbortDrawing => Self::AbortDrawing,
            WindowAct::NextDocument => Self::NextDocument,
            WindowAct::CloseDocument => Self::CloseDocument,
            WindowAct::OpenDocument => Self::OpenDocument,
            WindowAct::Separations => Self::Separations,
        };
        (kind, 0.0)
    }

    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Nothing,
            1 => Self::Say,
            2 => Self::ScrollBy,
            3 => Self::Copy,
            4 => Self::Find,
            5 => Self::Panel,
            6 => Self::Restrictions,
            7 => Self::Notices,
            8 => Self::Present,
            9 => Self::Print,
            10 => Self::Archive,
            11 => Self::ChooseArchive,
            12 => Self::LeaveFullScreen,
            13 => Self::NextLayout,
            14 => Self::FitControls,
            15 => Self::FreeText,
            16 => Self::Measure,
            17 => Self::AbortDrawing,
            18 => Self::NextDocument,
            19 => Self::CloseDocument,
            20 => Self::OpenDocument,
            21 => Self::Separations,
            _ => return None,
        })
    }

    /// The name, NUL-terminated for `const char *`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nothing => "Nothing\0",
            Self::Say => "Say\0",
            Self::ScrollBy => "ScrollBy\0",
            Self::Copy => "Copy\0",
            Self::Find => "Find\0",
            Self::Panel => "Panel\0",
            Self::Restrictions => "Restrictions\0",
            Self::Notices => "Notices\0",
            Self::Present => "Present\0",
            Self::Print => "Print\0",
            Self::Archive => "Archive\0",
            Self::ChooseArchive => "ChooseArchive\0",
            Self::LeaveFullScreen => "LeaveFullScreen\0",
            Self::NextLayout => "NextLayout\0",
            Self::FitControls => "FitControls\0",
            Self::FreeText => "FreeText\0",
            Self::Measure => "Measure\0",
            Self::AbortDrawing => "AbortDrawing\0",
            Self::NextDocument => "NextDocument\0",
            Self::CloseDocument => "CloseDocument\0",
            Self::OpenDocument => "OpenDocument\0",
            Self::Separations => "Separations\0",
        }
    }

    /// The number, as C sees it.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// What a press that reached the page comes to, decided the way every window decides it.
#[derive(Debug)]
pub(crate) enum Decided {
    /// A message the library sends for the caller.
    Send(Command),
    /// A job only the window can do.
    Window(WindowAct),
    /// A sentence the window owes the person instead of an effect.
    Say(String),
    /// Nothing: the key has no meaning here.
    Nothing,
}

/// A press, in the order the three windows ask about one (ADR 0526, ADR 1357).
///
/// First whether it presses the push-button §12.5.1's focus is on ([`viewer_host::pressed`]), then
/// what the table means by it ([`viewer_host::meaning`]), and last the one check every window makes
/// before sending a §12.5.6.10 markup: that something is selected for it to mark up.
pub(crate) fn decide(
    viewer: &Viewer,
    key: Key,
    held: Modifiers,
    mode: Mode,
    waiting: Waiting,
) -> Decided {
    if let Some(pressed) = viewer_host::pressed(viewer, key, held) {
        return match &pressed {
            viewer_host::Pressed::Activates { annotation, .. } => {
                Decided::Send(Command::Activate(*annotation))
            }
            // Table 227's bit 1, a field the document locked: the windows' sentence, worded by
            // `Pressed::note` as a click on it is.
            viewer_host::Pressed::ReadOnly { .. } => {
                pressed.note().map_or(Decided::Nothing, Decided::Say)
            }
        };
    }
    match viewer_host::meaning(key, held, mode, waiting) {
        None => Decided::Nothing,
        Some(Meaning::Window(act)) => Decided::Window(act),
        Some(Meaning::Send(Command::Edit(Edit::Markup { .. }))) if !selected(viewer) => {
            Decided::Say(SELECT_FIRST.to_owned())
        }
        Some(Meaning::Send(command)) => Decided::Send(command),
    }
}

/// Whether anything on the page is selected, as `NativeWindow::has_selection` asks it.
fn selected(viewer: &Viewer) -> bool {
    matches!(viewer.query(Query::Selection),
        Answer::Selected(selection) if !selection.quads.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{ActKind, Decided, KeyKind, SELECT_FIRST, decide};
    use pdf_model::view::Markup;
    use viewer_core::{Command, Edit, Viewer};
    use viewer_host::{Key, Mode, Modifiers, Waiting, WindowAct};

    /// `viewer_host::keys`' two markup rows, §12.5.6.10's, which the check in [`decide`] is for.
    const MARKUP_KEYS: [(Key, Markup); 2] =
        [(Key::H, Markup::Highlight), (Key::K, Markup::StrikeOut)];

    /// Every key `viewer-host` binds has exactly one number here, and the number goes back to it.
    #[test]
    fn every_key_the_table_binds_has_one_number_and_it_goes_back() {
        assert_eq!(Key::ALL.len(), KeyKind::COUNT as usize);
        let mut seen = std::collections::BTreeSet::new();
        for key in Key::ALL {
            let kind = KeyKind::of(*key);
            assert!(seen.insert(kind), "{key:?} shares a number");
            assert_eq!(KeyKind::from_code(kind as u32), Some(kind));
            assert_eq!(kind.key(), *key);
        }
        assert_eq!(KeyKind::from_code(KeyKind::COUNT), None);
    }

    /// Every act has a number, a name and a way back, and the first number past the count is none.
    #[test]
    fn every_act_has_a_number_and_a_name() {
        for code in 0..ActKind::COUNT {
            let kind = ActKind::from_code(code).expect("every number below the count is an act");
            assert_eq!(kind.code(), code);
            assert!(kind.name().ends_with('\0'));
        }
        assert_eq!(ActKind::from_code(ActKind::COUNT), None);
        assert_eq!(
            ActKind::of(WindowAct::ScrollBy(-60.0)),
            (ActKind::ScrollBy, -60.0)
        );
        assert_eq!(ActKind::of(WindowAct::Find), (ActKind::Find, 0.0));
    }

    /// The table's answers arrive as the windows' do: Down scrolls, Control + F finds, a Control
    /// the table does not bind means nothing (ADR 1192), and Escape stops a warned draw.
    #[test]
    fn a_press_is_decided_as_every_window_decides_it() {
        let viewer = Viewer::new(800, 1000, 1.0);
        let read = |key, held, waiting| decide(&viewer, key, held, Mode::Reading, waiting);
        assert!(matches!(
            read(Key::Down, Modifiers::NONE, Waiting::Nothing),
            Decided::Window(WindowAct::ScrollBy(by)) if by > 0.0
        ));
        assert!(matches!(
            read(Key::F, Modifiers::CTRL, Waiting::Nothing),
            Decided::Window(WindowAct::Find)
        ));
        assert!(matches!(
            read(Key::T, Modifiers::CTRL, Waiting::Nothing),
            Decided::Nothing
        ));
        assert!(matches!(
            read(Key::Escape, Modifiers::NONE, Waiting::Warned),
            Decided::Window(WindowAct::AbortDrawing)
        ));
        assert!(matches!(
            decide(
                &viewer,
                Key::F,
                Modifiers::NONE,
                Mode::Presenting,
                Waiting::Nothing
            ),
            Decided::Nothing
        ));
        assert!(matches!(
            read(Key::Z, Modifiers::NONE, Waiting::Nothing),
            Decided::Send(Command::Undo)
        ));
    }

    /// A markup key with nothing selected is the windows' sentence, never a markup over nothing.
    #[test]
    fn a_markup_with_nothing_selected_is_said_rather_than_sent() {
        let viewer = Viewer::new(800, 1000, 1.0);
        for (key, _) in MARKUP_KEYS {
            match decide(
                &viewer,
                key,
                Modifiers::NONE,
                Mode::Reading,
                Waiting::Nothing,
            ) {
                Decided::Say(said) => assert_eq!(said, SELECT_FIRST),
                other => panic!("{key:?} with nothing selected came to {other:?}"),
            }
        }
        // And the table does mean a markup by both, which is what makes the check above reachable.
        for (key, kind) in MARKUP_KEYS {
            assert!(matches!(
                viewer_host::meaning(key, Modifiers::NONE, Mode::Reading, Waiting::Nothing),
                Some(viewer_host::Meaning::Send(Command::Edit(Edit::Markup { kind: sent, .. })))
                    if sent == kind
            ));
        }
    }
}
