//! What a key press means, stated once for every host that has a keyboard.
//!
//! # Why the decision is here and not in a host
//!
//! Three windowed hosts, three key tables, and until the six-hundred-and-eighty-seventh session
//! they disagreed: `f` opened the find bar in `viewer-gtk` and armed §12.5.6.6's free-text drag in
//! `viewer-ui`; the Up and Down keys scrolled the view in one host and turned the page in two;
//! Escape cleared the selection in the two native hosts and **quit the program** in the third. The
//! four-consumer rule `doc/todo/30` is written under — a feature lands on the boundary and every
//! host adopts it — is a rule about *features*, and it had no purchase at all on the thing a person
//! actually touches.
//!
//! The argument for putting the table here is [`crate::presentation`]'s and [`crate::clock`]'s
//! verbatim (ADRs 0470, 0473): **which sentence a window is obeying is shared, and
//! `gdk::Key` against `Qt::Key` against `winit::keyboard::Key` is what a toolkit is.** A host
//! translates its toolkit's key into [`Key`] and asks [`meaning`]; what it supplies is the
//! translation and the platform call, and nothing else.
//!
//! It is deliberately **not** in [`viewer_core`], for that crate's rule 5: a key press is chrome,
//! and a crate whose whole subject is a document has no opinion about which key a window binds.
//! Half of this table is not a message at all — see [`WindowAct`].
//!
//! # What the standard says about a key, which is two sentences
//!
//! Almost every row below is a **documented choice** rather than a reading, and saying which is
//! which is the point of this section. The standard names a key exactly twice.
//!
//! §12.5.1, on moving between a page's annotations:
//!
//! > Interactive PDF processors may permit the user to navigate through the annotations on a page
//! > by using the keyboard (in particular, the tab key).
//!
//! — a permission, and the only key in the standard with a job attached. The *order* it moves in is
//! the document's (Table 31's `/Tabs`), which is [`viewer_core::Command::Focused`]'s business.
//!
//! §12.4.4.2, on a navigation request in a presentation, gives one as an EXAMPLE —
//!
//! > Pressing an arrow key.
//!
//! — and then again in the requirement itself: "[i]f the user requests to navigate forward (such as
//! an arrow key press) and there is a current navigation node". Both sentences are inside the
//! presentation subclause, which is why [`Mode`] exists below rather than being folded away.
//!
//! Everything else — a letter for a copy, a letter for a save, which key leaves full screen — is
//! stated nowhere, and this module is where those choices are written down as choices.
//!
//! # The choices, and the three disagreements they settle
//!
//! - **Up and Down move the *view*, and Left, Right, Page Up, Page Down and Space move between
//!   *pages*.** Both readings were live in this tree and neither is derivable: §12.4.4.2's "arrow
//!   key press" is an example of a navigation request and says nothing about which arrow. What
//!   decides it is that a reader has both jobs and only six keys, and that since ADR 0442 every
//!   host scrolls with the wheel — so binding all four arrows to a page turn spent two keys on a
//!   job three other keys already do and left the view with none.
//! - **In a presentation the arrows navigate, all four of them.** That *is* §12.4.4.2's sentence,
//!   and it costs nothing: a presented page fills the screen, so there is no view to move.
//! - **Escape clears the selection, and leaves full screen first.** `viewer-ui` quit the process
//!   here, which is the outlier and is worse than surprising: this program has had
//!   [`viewer_core::Command::Save`] and a `Query::Dirty` since long before it had a full screen, so
//!   a key that exits without asking is a key that can throw away an edit a reader made. Leaving
//!   full screen takes precedence for [`crate::presentation`]'s stated reason — no clause says how
//!   full screen ends, and a reader who cannot get their chrome back has had a restriction imposed
//!   on them by somebody else's file.
//! - **Escape also stops a draw the window has warned about, and only then.** This is the owner's
//!   *"warn the user and allow the user to abort, however don't block"* reaching the three
//!   established windows, and the binding is argued in three parts:
//!
//!   - **Why Escape and not a key of its own.** A key a person has to have learned is no use in
//!     the one situation this exists for, which is somebody sitting in front of a window that is
//!     not answering. Escape is the key every program uses to mean *not that*, this table's own
//!     documentation already says so two rows up, and the fourth window on the confined boundary
//!     has meant exactly this by it since ADR 0713 — so the four agree rather than the three.
//!   - **Why only while the warning is up.** A binding that means two things is a binding a
//!     person has to guess at, unless the window has said which one it means. It has:
//!     [`Waiting::Warned`] is the state a window enters by *saying*
//!     [`crate::status::still_drawing`], which names this key, and leaves when the draw ends — so
//!     the meaning changes when the sentence offering it does rather than when a clock passes.
//!     With no warning up, Escape clears the selection as it always did.
//!   - **Why a presentation still leaves full screen instead.** Table 29's `FullScreen` shows
//!     "no menu bar, window controls, or any other window visible", so the warning is not on the
//!     screen at all while one is running — and a key that did the thing an unseen sentence
//!     offered would be the guess this row exists to avoid. Escape leaves full screen, the
//!     sentence appears, and the next Escape stops the draw. The window stays responsive
//!     throughout either way, because the drawing is not on the toolkit's thread
//!     ([`crate::drawing`]).
//! - **`f` opens the find bar**, which two of the three hosts already meant by it, and §12.5.6.6's
//!   free-text drag moves to `t`, which nothing bound.
//! - **While a presentation is running, the three keys that ask for chrome mean nothing.** §7.7.2's
//!   Table 29 states `FullScreen` as
//!
//!   > Full-screen mode, with no menu bar, window controls, or any other window visible
//!
//!   so a find bar, a panel of trees and a card of notices are exactly what may not appear. This is
//!   the one place in the table where a *clause* takes a binding away, and [`Chrome::HIDDEN`] is
//!   the same sentence applied to the widgets.
//!
//!   [`Chrome::HIDDEN`]: crate::presentation::Chrome::HIDDEN
//!
//! # What a host still owns
//!
//! **Chrome takes a key before the page does, and that ordering is each host's own.** A §12.7.4.3
//! field with the keyboard, an open find bar, a modal card over the page: a key that reaches one of
//! those never reaches this table, and which widget has the focus is not something a shared value
//! can know. What this module states is what a press means **once it has reached the page**.
//!
//! **Modifiers are [`Modifiers`], and there are two of them.** `shift` is §12.5.1's tab key, which needs
//! a direction that winit reports one key for. `ctrl` is the conventional binding for an operation
//! a person already knows the key for, and it is here because of what its absence did: the table
//! never saw a Control at all, so every host discarded it before asking, and Ctrl + P entered
//! §12.4.4's presentation while Ctrl + C copied by coincidence. Both are wrong in the same way —
//! a key that means the unmodified thing is a key that ignores what the person held down.
//!
//! **Control selects a table of its own** ([`ctrl_meaning`]), and a key with no row in it means
//! *nothing*. That is the half worth stating: a modifier this program does not bind is a modifier
//! whose press belongs to something else — a window manager, a toolkit accelerator, a shortcut a
//! desktop added — and answering it with the unmodified binding is a window acting on a keystroke
//! that was not addressed to it. ADR 1192.
//!
//! What the chrome takes first is unchanged and is still each host's: by the time a press reaches
//! the page, the widget that wanted Ctrl + C has already had it.

use pdf_model::view::Markup;
use viewer_core::{Command, Edit, FocusMove, PageTarget, Selection, Zoom};

/// A key this program has a meaning for.
///
/// A closed set rather than a character and a name, and that is what makes the table checkable: a
/// host maps its toolkit's key onto one of these or onto nothing, [`Key::ALL`] is every one of
/// them, and a host's test walks that list to prove its own translation states a key for each.
/// Adding a row below therefore fails to compile in three hosts, which is the mechanism
/// `doc/ui-boundary.md` prefers to a catch-all arm.
///
/// **A letter is the letter, not the keystroke.** Case is the host's to fold: `A` is what a person
/// pressing the `a` key produces with or without Shift, because none of the letters below means a
/// second thing when shifted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// The letter `a` — select everything on the page.
    A,
    /// The letter `c` — §14.8.2.5's copy.
    C,
    /// The letter `f` — the find bar.
    F,
    /// The letter `h` — §12.5.6.10's highlight.
    H,
    /// The letter `k` — §12.5.6.10's strike-out.
    K,
    /// The letter `l` — Table 29's next arrangement.
    L,
    /// The letter `m` — §12.9's measuring.
    M,
    /// The letter `o` — the panel of trees.
    O,
    /// The letter `p` — §12.4.4's presentation.
    P,
    /// The letter `r` — the menu of `CLAUDE.md`'s four restriction levels
    /// ([`crate::restriction`]).
    R,
    /// The letter `s` — §7.5.6's incremental save.
    S,
    /// The letter `t` — §12.5.6.6's free text.
    T,
    /// The letter `w` — the magnification every §12.7 control fits at.
    W,
    /// The letter `y` — redo.
    Y,
    /// The letter `z` — undo.
    Z,
    /// The digit `0` — fit the page.
    Zero,
    /// `+`, on whichever of a layout's keys produces it.
    Plus,
    /// `-`.
    Minus,
    /// `=`, which is `+` without the Shift on most layouts and is bound as its equal.
    Equals,
    /// `/` — the find bar's other key, which every host has bound since it had a find bar.
    Slash,
    /// `?` — the third-party notices this binary is obliged to carry.
    Question,
    /// Escape.
    Escape,
    /// Tab — §12.5.1's own key.
    Tab,
    /// The space bar.
    Space,
    /// Home.
    Home,
    /// End.
    End,
    /// The Left arrow.
    Left,
    /// The Right arrow.
    Right,
    /// The Up arrow.
    Up,
    /// The Down arrow.
    Down,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
}

impl Key {
    /// Every key this program binds, so that a host can be held to translating all of them.
    ///
    /// The list is checked against the enumeration by
    /// `every_key_is_in_the_list_a_host_is_held_to`, which is the one thing a hand-written
    /// array of variants can get wrong.
    pub const ALL: &'static [Self] = &[
        Self::A,
        Self::C,
        Self::F,
        Self::H,
        Self::K,
        Self::L,
        Self::M,
        Self::O,
        Self::P,
        Self::R,
        Self::S,
        Self::T,
        Self::W,
        Self::Y,
        Self::Z,
        Self::Zero,
        Self::Plus,
        Self::Minus,
        Self::Equals,
        Self::Slash,
        Self::Question,
        Self::Escape,
        Self::Tab,
        Self::Space,
        Self::Home,
        Self::End,
        Self::Left,
        Self::Right,
        Self::Up,
        Self::Down,
        Self::PageUp,
        Self::PageDown,
    ];
}

/// Which modifier keys were down when the press arrived.
///
/// **A value rather than two `bool` parameters**, for the reason [`Mode`] and [`Waiting`] give
/// one row below: `meaning(key, true, false, …)` at a call site has said nothing about which of
/// them is which, and the two do entirely different things — `shift` chooses between two rows of
/// the same table, `ctrl` chooses a different table.
///
/// A modifier this program does not name is not here at all: Alt, Meta and the platform key
/// belong to a window manager and to a toolkit's accelerators, and a host that folded one of them
/// into `ctrl` would bind a key it was not given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Shift, which §12.5.1's tab key needs a direction from.
    pub shift: bool,
    /// Control, which selects [`ctrl_meaning`]'s table instead of the unmodified one.
    pub ctrl: bool,
}

impl Modifiers {
    /// Nothing held down, which is what most presses are.
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
    };
    /// Shift alone.
    pub const SHIFT: Self = Self {
        shift: true,
        ctrl: false,
    };
    /// Control alone.
    pub const CTRL: Self = Self {
        shift: false,
        ctrl: true,
    };
}

/// Whether §12.4.4's presentation is running, which two rows of the table depend on.
///
/// Not a `bool`, because a host reading `meaning(key, shift, true)` at a call site has been told
/// nothing about which `true` that is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// A window with its chrome, which is what a document is read in.
    #[default]
    Reading,
    /// §12.4.4's presentation, full screen and with Table 29's "no … other window visible".
    Presenting,
}

/// Something only the window knows how to do, so the table names it and stops.
///
/// The other half of [`Meaning`], and the reason a key table cannot simply be a map onto
/// [`viewer_core::Command`]: a clipboard is a platform's, a panel is chrome, and the magnification
/// every control fits at is a number the page's controls measured. Each of these is a *decision
/// already taken* somewhere in this crate or in a host — what the table adds is that all three
/// hosts take it on the same key.
///
/// Exhaustive by construction and not `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason: a
/// host that grew a catch-all arm here is a host where the next binding goes to be ignored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowAct {
    /// Move the view by this many **logical** pixels, downwards for a positive number.
    ///
    /// Logical rather than device, which is why it is here rather than a
    /// [`viewer_core::Command::Scroll`] the table could have built for itself: that message speaks
    /// device pixels and this value has no display to ask. A host multiplies by its own scale, and
    /// the two native hosts already do exactly that for a wheel notch.
    ScrollBy(f32),
    /// §14.8.2.5's text, onto whatever this platform calls a clipboard ([`crate::copying`]).
    Copy,
    /// Show the find bar, or put the keyboard back in it.
    Find,
    /// Show or hide the panel of trees — §12.3.3's outline, §8.11.4.3's layers, §7.11.4's files.
    Panel,
    /// Show the levels this reader has set for what a document asserts over them
    /// ([`crate::restriction`]).
    ///
    /// **A window act rather than a [`Command`]**, for this half of the table's standing reason:
    /// what a menu *is* differs in every host — a `gtk4::MenuButton`, a `QMenuBar`, a card this
    /// program draws — and what it offers does not. The key exists because one of the three
    /// windows has no chrome to hang a menu off at all, and `CLAUDE.md`'s "it shall always be
    /// possible to turn them off" is not kept by a menu two windows of three can reach (ADR 1145).
    Restrictions,
    /// Show the third-party notices this binary is obliged to carry with it.
    ///
    /// A licence obligation with a surface: the two licences covering the compiled-in standard 14
    /// font programs (§9.6.2.2) both require a *binary* distribution to reproduce their notices,
    /// and a command-line flag is a poor answer for somebody looking at a window.
    Notices,
    /// Start §12.4.4's presentation, or stop the one that is running ([`crate::presentation`]).
    Present,
    /// Ask to print: open this window's print dialogue.
    ///
    /// **A window act rather than a [`Command`], and the split is the sharpest example in this
    /// table of why this half exists.** §7.6.4.2's bit 3 *is* a message —
    /// [`viewer_core::Command::Print`] asks the policy and puts the document into print intent —
    /// but nothing on that boundary can open a dialogue, and `GtkPrintOperation` against a
    /// `QPrinter` against a panel this program draws is what a print system is. So the key names
    /// the job and the window sends the message once it knows what sheet the person chose
    /// (ADR 1180).
    Print,
    /// Leave full screen, which is what Escape means while one is running.
    LeaveFullScreen,
    /// Move on to the next of Table 29's six arrangements ([`crate::arrangement`]).
    NextLayout,
    /// Magnify until every §12.7 control fits the `/Rect` its document states ([`crate::fit`]).
    FitControls,
    /// Arm §12.5.6.6's free-text drag: the next drag on the page draws the annotation's rectangle.
    FreeText,
    /// Start or stop §12.9's measuring: while it is on, presses on the page put down points.
    ///
    /// **A window act rather than a [`Command`]**, and it is the clearest case in this half of
    /// the table: §12.9 states no state at all for a viewer to be in. What it states is the
    /// arithmetic and the formatting, which [`viewer_core::Query::Measure`] answers from the
    /// points a host has collected — so the mode is chrome, the points are the host's, and the
    /// only thing that crosses the boundary is the question. [`crate::measuring`] is the state
    /// and the sentence; a window supplies the presses and somewhere to show the answer.
    Measure,
    /// Stop drawing the page the window has just said is taking a long time
    /// ([`crate::drawing::Drawing::abandon`]).
    ///
    /// **The abort half of the owner's "warn the user and allow the user to abort, however don't
    /// block"**, and it is a [`WindowAct`] rather than a [`Command`] for the reason the whole of
    /// this half of the table exists: a draw is a *host's* thread, `viewer_core` neither knows
    /// that one is running nor is owed an answer for one that is abandoned (trap 20), and the
    /// three windows take the thread back in three different places — `viewer_host::Drawing` in
    /// the two native ones and the composing thread in `viewer-ui`'s.
    AbortDrawing,
    /// Turn §10.8.3's separation simulation on, or off again.
    ///
    /// **A window act rather than a [`Command`] although it ends in one**, for this half of the
    /// table's standing reason: the *state* — which of the two pictures this reader has asked
    /// for — is the host's, because `viewer_core` holds no preference a person can toggle and a
    /// window is what shows them which one is on. What crosses the boundary is
    /// [`viewer_core::Command::Separations`] with the answer. ADR 1228.
    Separations,
}

/// Whether a draw the window has already warned about is still running.
///
/// The fourth thing [`meaning`] needs and the only one that is not about the keyboard: it decides
/// one row, Escape's, and it exists so that **the key never changes meaning without the window
/// having said so first**. [`crate::drawing::Drawing::overlong`] is where a native host gets it;
/// `viewer-ui`'s composing thread answers the same question about a whole frame.
///
/// Not a `bool`, for [`Mode`]'s reason one row over: `meaning(key, shift, mode, true)` at a call
/// site has said nothing about which `true` that is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Waiting {
    /// Nothing is being drawn, or nothing has been drawn for long enough to say so.
    #[default]
    Nothing,
    /// A draw has outlasted [`crate::drawing::WARN`] and the window is showing
    /// [`crate::status::still_drawing`].
    Warned,
}

/// What a key press means, once it has reached the page.
///
/// Two variants because a key table has two kinds of row in it, and collapsing them would cost the
/// thing that makes this checkable: a [`Meaning::Send`] is complete from the key alone and every
/// host dispatches it identically, while a [`Meaning::Window`] names a job whose *doing* differs in
/// every host and whose *meaning* does not.
#[derive(Debug)]
pub enum Meaning {
    /// A message to [`viewer_core`], complete from the key alone.
    Send(Command),
    /// A job the window finishes from its own state.
    Window(WindowAct),
}

/// How far the arrow keys move the view, in **logical** pixels.
///
/// A choice, and a small one deliberately: about a fifteenth of a fitted A4 page, so that holding
/// the key reads as scrolling rather than as jumping. It is the number `viewer-ui` has used since
/// it had arrow keys, kept because nothing argues for another and a changed number is a changed
/// feel with no reason attached.
pub const SCROLL_STEP: f32 = 60.0;

/// §12.5.6.10's highlight colour, which the standard states nowhere.
///
/// Table 166's `/C` "carries what a processor was told" and no clause says what to tell it, so this
/// is a documented choice: a soft yellow, because that is what a highlighter is.
pub const HIGHLIGHT: [f32; 3] = [1.0, 0.9, 0.2];

/// §12.5.6.10's strike-out colour, on the same footing as [`HIGHLIGHT`] and for the same reason.
pub const STRIKE_OUT: [f32; 3] = [0.85, 0.15, 0.15];

/// What a press means, or nothing for a key this program does not bind.
///
/// [`Modifiers::ctrl`] chooses the table: with Control down the answer is [`ctrl_meaning`]'s and a key
/// that has no row there means nothing at all. Without it, [`Modifiers::shift`] answers §12.5.1's tab
/// key and the print job, `mode` answers §12.4.4.2's arrow keys and Table 29's chrome, and
/// `waiting` answers the third of Escape's three rows.
///
/// **A host calls this only for a press that reached the page.** A field being typed into, an open
/// find bar and a modal card each take the keyboard first, and which of them has it is the host's
/// own question.
#[must_use]
pub fn meaning(key: Key, held: Modifiers, mode: Mode, waiting: Waiting) -> Option<Meaning> {
    let presenting = matches!(mode, Mode::Presenting);
    let warned = matches!(waiting, Waiting::Warned);
    if held.ctrl {
        return ctrl_meaning(key, mode);
    }
    let shift = held.shift;
    Some(match key {
        // §12.5.1: "Interactive PDF processors may permit the user to navigate through the
        // annotations on a page by using the keyboard (in particular, the tab key)." The only key
        // in the standard with a job attached, and Shift is the only thing separating the two
        // directions because winit reports one key for both.
        Key::Tab => Meaning::Send(Command::Focused(if shift {
            FocusMove::Previous
        } else {
            FocusMove::Next
        })),
        Key::Right | Key::PageDown | Key::Space => Meaning::Send(Command::GoTo(PageTarget::Next)),
        Key::Left | Key::PageUp => Meaning::Send(Command::GoTo(PageTarget::Previous)),
        // §12.4.4.2's "arrow key press" while a presentation is running, and the view's own
        // movement otherwise. A presented page fills the screen, so nothing is lost either way.
        Key::Down if presenting => Meaning::Send(Command::GoTo(PageTarget::Next)),
        Key::Up if presenting => Meaning::Send(Command::GoTo(PageTarget::Previous)),
        Key::Down => Meaning::Window(WindowAct::ScrollBy(SCROLL_STEP)),
        Key::Up => Meaning::Window(WindowAct::ScrollBy(-SCROLL_STEP)),
        Key::Home => Meaning::Send(Command::GoTo(PageTarget::First)),
        Key::End => Meaning::Send(Command::GoTo(PageTarget::Last)),
        // No anchor on any of the three: a keyboard names no point, so the core magnifies about
        // the viewport's centre.
        Key::Plus | Key::Equals => Meaning::Send(Command::Zoom {
            zoom: Zoom::In,
            at: None,
        }),
        Key::Minus => Meaning::Send(Command::Zoom {
            zoom: Zoom::Out,
            at: None,
        }),
        Key::Zero => Meaning::Send(Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        }),
        Key::W => Meaning::Window(WindowAct::FitControls),
        Key::A => Meaning::Send(Command::Select(Selection::All)),
        // Full screen first, then the draw the window has warned about, then §12.4.2's selection.
        // All three are documented choices and the module documentation has the argument for
        // each — including why the warning's row is *below* full screen rather than above it.
        Key::Escape if presenting => Meaning::Window(WindowAct::LeaveFullScreen),
        Key::Escape if warned => Meaning::Window(WindowAct::AbortDrawing),
        Key::Escape => Meaning::Send(Command::Select(Selection::None)),
        // §10.8.3's simulation, on the letter it is named after and on Shift for `Print`'s
        // reason: unshifted `S` is `Command::Save` and has been since this table existed, and
        // the clause names no key at all.
        Key::S if shift => Meaning::Window(WindowAct::Separations),
        Key::S => Meaning::Send(Command::Save),
        Key::Z => Meaning::Send(Command::Undo),
        Key::Y => Meaning::Send(Command::Redo),
        // §12.5.6.10 over what is selected. Four subtypes and one key apiece would be four
        // bindings a person has to learn; these are the two a person means by "mark this", and a
        // host with a menu can offer the other two.
        Key::H => Meaning::Send(Command::Edit(Edit::Markup {
            kind: Markup::Highlight,
            colour: HIGHLIGHT,
        })),
        Key::K => Meaning::Send(Command::Edit(Edit::Markup {
            kind: Markup::StrikeOut,
            colour: STRIKE_OUT,
        })),
        Key::C => Meaning::Window(WindowAct::Copy),
        // **Shift and P rather than Control and P, and it is a choice this table had already
        // made.** The standard names no key for printing at all, and the module documentation
        // above states why no modifier but Shift reaches here: by the time a press has got past
        // the chrome, the widget that would have wanted Control has had it. `P` unshifted is
        // §12.4.4's presentation and has been since this table existed, so the one key a person
        // expects is the one already spoken for — which leaves Shift, the modifier this table
        // does read, on the letter the job is named after. ADR 1180.
        Key::P if shift => Meaning::Window(WindowAct::Print),
        Key::P => Meaning::Window(WindowAct::Present),
        Key::L => Meaning::Window(WindowAct::NextLayout),
        Key::T => Meaning::Window(WindowAct::FreeText),
        // §12.9's measuring, on the letter it is named after and on no modifier: the clause
        // names no key, and the three letters a measurement could be called after — `m`, `d` for
        // a distance, `u` for units — leave only this one that nothing else binds.
        Key::M => Meaning::Window(WindowAct::Measure),
        // Table 29's `FullScreen` shows "no menu bar, window controls, or any other window
        // visible", so the three keys that ask for chrome ask for nothing while one is running.
        Key::F | Key::Slash | Key::O | Key::Question | Key::R if presenting => return None,
        Key::F | Key::Slash => Meaning::Window(WindowAct::Find),
        Key::O => Meaning::Window(WindowAct::Panel),
        Key::Question => Meaning::Window(WindowAct::Notices),
        // The menu is chrome too, and for `Question`'s reason it asks for nothing while a
        // presentation is running — the row above puts `R` in that list.
        Key::R => Meaning::Window(WindowAct::Restrictions),
    })
}

/// What a press with Control held down means, which is a table of its own.
///
/// # Why these four and no others
///
/// Every row here is an operation this program **already performs**, given the key a person
/// pressing Control expects it on. Nothing was invented to fill the table: there is no Ctrl + O,
/// because no window in this tree opens a second document — each is given a file on its command
/// line — and a binding for a verb that does not exist would be a key that appears to do nothing.
///
/// Two of the four already had an unmodified key and keep it. `s` is still §7.5.6's save and `c`
/// is still §14.8.2.5's copy, because a table three windows agree about is not improved by taking
/// a binding away from the people using it; what Control adds is the key everything else on the
/// desktop uses for the same job.
///
/// **Shift is not read here**, so Ctrl + Shift + P prints. A person holding a third key down has
/// not asked for a fifth meaning, and the shifted rows of the unmodified table are about
/// *direction* — §12.5.1's tab — which none of these four has.
///
/// # And a key with no row means nothing
///
/// This function answers `None` rather than falling through to [`meaning`]'s table, and that is
/// the change ADR 1192 is about. A Control this program does not bind is a press addressed to
/// something else, and a window that answered Ctrl + T by arming §12.5.6.6's drag would be acting
/// on a keystroke aimed past it.
///
/// Table 29's `FullScreen` shows "no menu bar, window controls, or any other window visible", so
/// the find bar's row is taken away while a presentation is running, exactly as its unmodified
/// key is.
#[must_use]
pub fn ctrl_meaning(key: Key, mode: Mode) -> Option<Meaning> {
    Some(match key {
        Key::C => Meaning::Window(WindowAct::Copy),
        Key::S => Meaning::Send(Command::Save),
        // §7.6.4.2's bit 3 is asked by the message this act sends once the window knows what
        // sheet the person chose; the key names the job (ADR 1180).
        Key::P => Meaning::Window(WindowAct::Print),
        Key::F if matches!(mode, Mode::Presenting) => return None,
        Key::F => Meaning::Window(WindowAct::Find),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{Key, Meaning, Mode, Modifiers, Waiting, WindowAct, meaning};
    use viewer_core::{Command, PageTarget, Selection};

    /// [`Key::ALL`] is the list three hosts are held to, so it has to be the whole enumeration.
    ///
    /// The match is what makes this a check rather than a second hand-written list: a variant
    /// added above and forgotten here fails to compile, and one added to `ALL` twice fails below.
    #[test]
    fn every_key_is_in_the_list_a_host_is_held_to() {
        for key in Key::ALL {
            // Exhaustive on purpose. A new variant lands here as a compile error, which is the
            // only way a list like this stays complete.
            match key {
                Key::A
                | Key::C
                | Key::F
                | Key::H
                | Key::K
                | Key::L
                | Key::M
                | Key::O
                | Key::P
                | Key::R
                | Key::S
                | Key::T
                | Key::W
                | Key::Y
                | Key::Z
                | Key::Zero
                | Key::Plus
                | Key::Minus
                | Key::Equals
                | Key::Slash
                | Key::Question
                | Key::Escape
                | Key::Tab
                | Key::Space
                | Key::Home
                | Key::End
                | Key::Left
                | Key::Right
                | Key::Up
                | Key::Down
                | Key::PageUp
                | Key::PageDown => {}
            }
        }
        let mut seen: Vec<Key> = Vec::new();
        for key in Key::ALL {
            assert!(!seen.contains(key), "{key:?} is in the list twice");
            seen.push(*key);
        }
        assert_eq!(
            seen.len(),
            32,
            "the list is the enumeration, and no shorter"
        );
    }

    /// Every key in the list means something while a document is being read.
    ///
    /// The table is what a host is held to, so a key with no meaning at all would be a row three
    /// hosts translate and none can act on.
    #[test]
    fn no_key_this_program_names_is_bound_to_nothing() {
        for key in Key::ALL {
            assert!(
                meaning(*key, Modifiers::NONE, Mode::Reading, Waiting::Nothing).is_some(),
                "{key:?} reaches the page and means nothing"
            );
        }
    }

    /// §12.4.4.2's arrow keys, and the two rows [`Mode`] exists for.
    ///
    /// The clause gives "[p]ressing an arrow key" as its example of a navigation request and does
    /// so inside the presentation subclause, so the arrows navigate there and move the view here.
    #[test]
    fn the_arrows_move_the_view_while_reading_and_navigate_while_presenting() {
        assert!(matches!(
            meaning(Key::Down, Modifiers::NONE, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::ScrollBy(by))) if by > 0.0
        ));
        assert!(matches!(
            meaning(Key::Up, Modifiers::NONE, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::ScrollBy(by))) if by < 0.0
        ));
        assert!(matches!(
            meaning(
                Key::Down,
                Modifiers::NONE,
                Mode::Presenting,
                Waiting::Nothing
            ),
            Some(Meaning::Send(Command::GoTo(PageTarget::Next)))
        ));
        assert!(matches!(
            meaning(Key::Up, Modifiers::NONE, Mode::Presenting, Waiting::Nothing),
            Some(Meaning::Send(Command::GoTo(PageTarget::Previous)))
        ));
        // And the other four move between pages either way, so nothing is unreachable.
        for key in [Key::Right, Key::PageDown, Key::Space] {
            for mode in [Mode::Reading, Mode::Presenting] {
                assert!(matches!(
                    meaning(key, Modifiers::NONE, mode, Waiting::Nothing),
                    Some(Meaning::Send(Command::GoTo(PageTarget::Next)))
                ));
            }
        }
    }

    /// Escape clears the selection, and leaves full screen before it does.
    ///
    /// Both are documented choices. The one worth a test is the second: `viewer-ui` exited the
    /// process on this key, so a reader with an unsaved edit could lose it by pressing the key
    /// every other program uses to mean "not that".
    #[test]
    fn escape_leaves_full_screen_first_and_never_leaves_the_program() {
        assert!(matches!(
            meaning(
                Key::Escape,
                Modifiers::NONE,
                Mode::Presenting,
                Waiting::Nothing
            ),
            Some(Meaning::Window(WindowAct::LeaveFullScreen))
        ));
        assert!(matches!(
            meaning(
                Key::Escape,
                Modifiers::NONE,
                Mode::Reading,
                Waiting::Nothing
            ),
            Some(Meaning::Send(Command::Select(Selection::None)))
        ));
    }

    /// Escape's third row: it stops a draw the window has warned about, and nothing else changes.
    ///
    /// Three things are asserted because three could go wrong separately: the abort is offered
    /// while the warning is up, the selection is still what Escape means when it is not, and a
    /// presentation still leaves full screen — Table 29's `FullScreen` shows "no menu bar, window
    /// controls, or any other window visible", so the sentence offering this key is not on the
    /// screen there and the key may not act on it.
    #[test]
    fn escape_stops_a_draw_only_while_the_window_is_saying_that_it_can() {
        assert!(matches!(
            meaning(Key::Escape, Modifiers::NONE, Mode::Reading, Waiting::Warned),
            Some(Meaning::Window(WindowAct::AbortDrawing))
        ));
        assert!(matches!(
            meaning(
                Key::Escape,
                Modifiers::NONE,
                Mode::Reading,
                Waiting::Nothing
            ),
            Some(Meaning::Send(Command::Select(Selection::None)))
        ));
        assert!(matches!(
            meaning(
                Key::Escape,
                Modifiers::NONE,
                Mode::Presenting,
                Waiting::Warned
            ),
            Some(Meaning::Window(WindowAct::LeaveFullScreen))
        ));
    }

    /// A draw being warned about changes **one** row, which is the claim [`Waiting`] makes.
    ///
    /// The same shape as the Shift test below it and for the same reason: a state that quietly
    /// moved a second binding would be a key meaning two things with nothing on the screen saying
    /// which, which is exactly what the warning exists to prevent.
    #[test]
    fn the_warning_changes_escape_and_no_other_key() {
        for key in Key::ALL {
            if matches!(key, Key::Escape) {
                continue;
            }
            for mode in [Mode::Reading, Mode::Presenting] {
                assert_eq!(
                    format!(
                        "{:?}",
                        meaning(*key, Modifiers::NONE, mode, Waiting::Nothing)
                    ),
                    format!(
                        "{:?}",
                        meaning(*key, Modifiers::NONE, mode, Waiting::Warned)
                    ),
                    "{key:?} means a second thing while a draw is being warned about"
                );
            }
        }
    }

    /// Table 29's "or any other window visible", applied to the keys that ask for chrome.
    #[test]
    fn a_presentation_shows_no_find_bar_no_panel_and_no_card() {
        for key in [Key::F, Key::Slash, Key::O, Key::Question] {
            assert!(
                meaning(key, Modifiers::NONE, Mode::Presenting, Waiting::Nothing).is_none(),
                "{key:?} asks for chrome that Table 29's FullScreen forbids"
            );
            assert!(
                meaning(key, Modifiers::NONE, Mode::Reading, Waiting::Nothing).is_some(),
                "{key:?} still means something in a window that has chrome"
            );
        }
    }

    /// Control binds the four operations this program has, on the keys a desktop uses for them.
    ///
    /// Each of the four is an operation that already existed with no conventional key reaching
    /// it: §7.5.6's save, §14.8.2.5's copy, the print job ADR 1180 built and the find bar. What
    /// makes this a test rather than a list is the pairing with the one below it — these four
    /// mean something and every other key means nothing, which is the whole of ADR 1192's rule.
    #[test]
    fn control_binds_the_operations_this_program_already_performs() {
        assert!(matches!(
            meaning(Key::C, Modifiers::CTRL, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::Copy))
        ));
        assert!(matches!(
            meaning(Key::S, Modifiers::CTRL, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Send(Command::Save))
        ));
        assert!(matches!(
            meaning(Key::P, Modifiers::CTRL, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::Print))
        ));
        assert!(matches!(
            meaning(Key::F, Modifiers::CTRL, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::Find))
        ));
        // Shift is not read with Control, so a third key held down asks for no fifth meaning.
        assert!(matches!(
            meaning(
                Key::P,
                Modifiers {
                    shift: true,
                    ctrl: true
                },
                Mode::Reading,
                Waiting::Nothing
            ),
            Some(Meaning::Window(WindowAct::Print))
        ));
        // Table 29's `FullScreen` shows "no menu bar, window controls, or any other window
        // visible", so the find bar's row goes with its unmodified key.
        assert!(meaning(Key::F, Modifiers::CTRL, Mode::Presenting, Waiting::Nothing).is_none());
    }

    /// A Control this program does not bind means **nothing**, and never the unmodified row.
    ///
    /// The defect this is written against: all three hosts discarded Control before asking, so
    /// Ctrl + P entered §12.4.4's presentation and Ctrl + X armed whatever bare `x` meant. A
    /// press with a modifier this program has no row for is addressed to something else — a
    /// window manager, a toolkit accelerator — and answering it is a window acting on a
    /// keystroke aimed past it. ADR 1192.
    #[test]
    fn a_control_this_table_does_not_bind_falls_through_to_nothing() {
        for key in Key::ALL {
            if matches!(key, Key::C | Key::S | Key::P | Key::F) {
                continue;
            }
            for mode in [Mode::Reading, Mode::Presenting] {
                for waiting in [Waiting::Nothing, Waiting::Warned] {
                    assert!(
                        meaning(*key, Modifiers::CTRL, mode, waiting).is_none(),
                        "{key:?} with Control means the unmodified thing"
                    );
                }
            }
        }
    }

    /// §12.9's measuring is a key of its own, and pressing it changes nothing about the page.
    ///
    /// The mode is a [`WindowAct`] because §12.9 states no state for a viewer to be in: the
    /// clause is arithmetic and formatting, and where the two points come from is a gesture.
    #[test]
    fn the_measuring_key_names_a_mode_and_sends_no_message() {
        assert!(matches!(
            meaning(Key::M, Modifiers::NONE, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::Measure))
        ));
        assert!(
            matches!(
                meaning(Key::M, Modifiers::NONE, Mode::Presenting, Waiting::Nothing),
                Some(Meaning::Window(WindowAct::Measure))
            ),
            "a measurement is not chrome Table 29 forbids: nothing appears over the page"
        );
    }

    /// Shift changes three rows, and they are named here so that a fourth cannot arrive quietly.
    ///
    /// §12.5.1's tab key is the first: the clause gives it a direction and winit reports one key
    /// for both, so the modifier is the only thing separating them. The second is `P`, where the
    /// letter a print job is named after was already §12.4.4's presentation — the argument is at
    /// the binding and in ADR 1180. The third is `S`, where §10.8.3's simulation meets
    /// `Command::Save` on the same letter (ADR 1228). Every other key means one thing, and the
    /// loop below is what keeps that true: a row that started reading Shift without saying so
    /// fails here.
    #[test]
    fn shift_separates_the_tab_key_the_print_job_and_the_separation_simulation() {
        assert!(matches!(
            meaning(Key::Tab, Modifiers::NONE, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Send(Command::Focused(
                viewer_core::FocusMove::Next
            )))
        ));
        assert!(matches!(
            meaning(Key::Tab, Modifiers::SHIFT, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Send(Command::Focused(
                viewer_core::FocusMove::Previous
            )))
        ));
        assert!(matches!(
            meaning(Key::P, Modifiers::NONE, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::Present))
        ));
        assert!(matches!(
            meaning(Key::P, Modifiers::SHIFT, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::Print))
        ));
        assert!(matches!(
            meaning(Key::S, Modifiers::NONE, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Send(Command::Save))
        ));
        assert!(matches!(
            meaning(Key::S, Modifiers::SHIFT, Mode::Reading, Waiting::Nothing),
            Some(Meaning::Window(WindowAct::Separations))
        ));
        for key in Key::ALL {
            if matches!(key, Key::Tab | Key::P | Key::S) {
                continue;
            }
            for mode in [Mode::Reading, Mode::Presenting] {
                assert_eq!(
                    format!(
                        "{:?}",
                        meaning(*key, Modifiers::NONE, mode, Waiting::Nothing)
                    ),
                    format!(
                        "{:?}",
                        meaning(*key, Modifiers::SHIFT, mode, Waiting::Nothing)
                    ),
                    "{key:?} means a second thing when shifted, which this table does not state"
                );
            }
        }
    }
}
