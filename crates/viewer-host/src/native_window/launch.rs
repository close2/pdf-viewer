//! The launch, once for the three windows: the document a native window opens with, opening — and
//! its first page interpreted — on a thread of its own while the toolkit comes up, and the
//! viewport's sizes, the first of which joins it (ADRs 1531, 1539, 1580, 1811, 1855; written once
//! by ADR 2172).
//!
//! `viewer-gtk`, `viewer-qt` and `viewer-ffi`'s `quorra_window` each wrote this sequence out, line
//! for line: the reader's commands on the opening thread, the named directories' refusals said
//! before the document, the opening commands, page one anticipated; then, at the first size, the
//! wait for the thread and the join. The third copy is where this crate takes a rule (habit 131).
//!
//! What stays a toolkit's is stated as a parameter rather than left as a step in the host (habit
//! 139): where the refusals are said ([`Refusals`]), what the toolkit calls its first size
//! ([`Sizing`]), and the timer a toolkit arms for a
//! document still opening past the wait, which [`Joining::StillOpening`] asks of the one toolkit
//! that has no timer already looking.

use std::path::Path;

use pdf_model::view::WidgetAppearances;
use viewer_core::{Command, Event, Viewer};

use super::{NativeWindow, WindowState, opening_commands, standing};
use crate::trace::{Topic, Trace};

/// Where the sentences the opening thread says before the document go: the toolkit's part of
/// [`Launch::start`].
///
/// One sentence per file in a directory the reader named that is not an anchor or not a target
/// document, said rather than dropped so that a person reading the verdicts that follow knows
/// which files did not take (trap 5).
#[derive(Debug)]
pub enum Refusals {
    /// On the terminal, `note: ` before each, where the toolkit windows say everything a launch
    /// could not do: their window does not exist yet.
    Terminal,
    /// To the window, which says each where it says everything else at its next look — the C
    /// window's `SAID` record. A window freed before it looks has nobody to say them to, and the
    /// thread stops sending.
    Sent(std::sync::mpsc::Sender<String>),
}

impl Refusals {
    /// Says `refusal`, answering whether anybody is still there to say the next one to.
    fn say(&self, refusal: String) -> bool {
        match self {
            Self::Terminal => {
                println!("note: {refusal}");
                true
            }
            Self::Sent(window) => window.send(refusal).is_ok(),
        }
    }
}

/// The document a native window opens with, opening on a thread of its own until the viewport's
/// first size joins it.
///
/// **The launch path's overlap.** What a toolkit needs before its first size — the display, the
/// theme, the window — needs no document, and the open and page one's interpretation need none of
/// it; so the open starts first, on a `Viewer` with no viewport, through the same commands
/// [`NativeWindow::open_document`] sends, and [`viewer_core::Viewer::anticipate`] interprets the
/// page it stands at (ADRs 1531, 1539). The core is still single-threaded: it is made on that
/// thread and moved back, which is `viewer-core`'s rule 4 as `quorra` keeps it (ADR 0182).
#[derive(Debug)]
pub struct Launch {
    /// The open file, kept for a password's second open.
    bytes: pdf_syntax::FileBytes,
    /// Annex O's fragment, as the command line carried it.
    fragment: Option<String>,
    /// §6.3.2.2's instruction, which every later open sends before its document.
    widget_appearances: WidgetAppearances,
    /// The levels and policies the window holds.
    settings: crate::Settings,
    /// Page one drawn while it is read on the thread (ADRs 1855, 1867): told the window's size at
    /// each size, and handed to the drawing at the join.
    ahead: crate::Ahead,
    /// The thread, as a [`crate::Loading`] so that page one publishes how far it has got (ADR 1811).
    anticipated: crate::Loading<(Viewer, Vec<Event>)>,
    /// The conversion `--archive` chose, kept for the window's archive key (ADR 1954).
    archive: Option<viewer_core::Archiving>,
}

impl Launch {
    /// Starts the thread that opens `bytes` under what the window was asked to hold.
    ///
    /// `reader` is the reader's policy words, which become their commands on that thread and
    /// before the document: reading an anchor or a target directory is not needed to show page one
    /// (`CLAUDE.md` principle 2), and a policy applied halfway through is not a policy (ADR 1580).
    /// What each named directory refused goes where `refusals` says, before the document opens.
    ///
    /// The file is the caller's to open — on disk rather than read whole, so that the core reads
    /// what page one needs through the handle (ADR 0809) — because what an unreadable file is
    /// called is the toolkit's own error.
    ///
    /// # Panics
    ///
    /// Where the operating system refuses a thread, as [`crate::Loading::start`] does.
    #[must_use]
    pub fn start(
        bytes: pdf_syntax::FileBytes,
        fragment: Option<String>,
        widget_appearances: WidgetAppearances,
        settings: crate::Settings,
        reader: crate::ReaderWords,
        refusals: Refusals,
    ) -> Self {
        let commands = opening_commands(
            standing(settings).window(),
            widget_appearances,
            settings.separations,
            bytes.clone(),
            None,
            fragment.clone(),
        );
        let ahead = crate::Ahead::page_sized();
        let drawing_ahead = ahead.clone();
        let archive = reader.archive.clone();
        let anticipated = crate::Loading::start(move || {
            // No viewport: the window does not exist yet, and the core renders nothing into one
            // with no extent.
            let mut viewer = Viewer::new(0, 0, 1.0);
            let mut events = Vec::new();
            let supply = reader.commands();
            for command in supply.commands {
                events.extend(viewer.handle(command));
            }
            for refusal in supply
                .refused
                .into_iter()
                .chain(viewer.reference_refusals().iter().map(ToString::to_string))
            {
                if !refusals.say(refusal) {
                    break;
                }
            }
            for command in commands {
                events.extend(viewer.handle(command));
            }
            // Page one read here as `Viewer::anticipate` reads it, and drawn beside the read on a
            // thread of its own once the read has run `GLIMPSE_AFTER` (ADR 1855): the read keeps
            // the thread the launch placed it on (trap 154).
            events.extend(drawing_ahead.anticipate(&mut viewer));
            (viewer, events)
        });
        Self {
            bytes,
            fragment,
            widget_appearances,
            settings,
            ahead,
            anticipated,
            archive,
        }
    }

    /// The open file, for a launch trace's count of its bytes.
    #[must_use]
    pub const fn bytes(&self) -> &pdf_syntax::FileBytes {
        &self.bytes
    }

    /// The window's state for the file at `path`, with this document opening in it.
    #[must_use]
    pub fn state(self, path: &Path, trace: Trace) -> WindowState {
        WindowState::new(
            path,
            self.bytes,
            self.fragment,
            trace,
            self.ahead,
            self.widget_appearances,
            self.settings,
        )
        .archiving(self.archive)
        .opening(self.anticipated)
    }
}

/// What a toolkit calls the viewport's first size, for the launch trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstSize {
    /// GTK's first size allocation.
    Allocation,
    /// The first resize event Qt and a C caller send.
    Resize,
}

/// How a toolkit takes the viewport's sizes: the toolkit's part of [`NativeWindow::sized`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sizing {
    /// What the toolkit calls the first size.
    pub first: FirstSize,
}

/// What a size left for the toolkit to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Joining {
    /// Nothing: the size was told, or the document it joined is the window's.
    Done,
    /// The first size found the document still opening past the wait, the bar is up, and a look at
    /// [`crate::loading::TICK`] is owed that calls [`NativeWindow::mind_the_opening`] until it
    /// answers `false` — a timer the toolkit arms where none of its own is already looking.
    StillOpening,
}

/// [`NativeWindow::sized`]'s body.
pub(super) fn sized<W: NativeWindow + ?Sized>(
    window: &mut W,
    width: u32,
    height: u32,
    scale: f32,
    sizing: Sizing,
) -> Joining {
    // §12.4.4.1's transition is drawn in the viewport rather than in a page's own rectangle, so
    // the size the core is told is also the size a frame is shaped for.
    window.state_mut().viewport = (width, height);
    let resize = Command::Resize {
        width,
        height,
        scale,
    };
    // Page one's drawing ahead draws into the target the first frame will be asked at, which is
    // this viewport's; a size that changes while the page is read is told again, and a frame asked
    // at another target than the one drawn is drawn whole (ADR 1855).
    let extent = crate::loading::Window {
        width,
        height,
        inset: 0,
        scale,
    };
    let state = window.state_mut();
    state.extent = Some(extent);
    if let Some(ahead) = state.ahead.as_ref() {
        ahead.window_is(extent);
    }
    if !std::mem::replace(&mut state.sized, true) {
        let first = match sizing.first {
            FirstSize::Allocation => "allocation",
            FirstSize::Resize => "resize",
        };
        state.trace.say(
            Topic::Launch,
            format_args!("first {first} {width}x{height} device px, scale {scale}"),
        );
        return join_the_opening(window, resize);
    }
    // A document still opening past `WARN` is told the window's size when it is taken in.
    if state.awaiting.is_some() {
        state.awaiting = Some(resize);
        return Joining::Done;
    }
    window.dispatch(resize);
    // Every window measures its page and fits itself, saying when it has
    // ([`crate::Presenting::placed`]): a size taken while the fit is still owed asks it again,
    // because a frame is not always drawn for a viewport the page already fits the width of (ADRs
    // 1429, 2308).
    if window.state().presenting.owes_placing() {
        window.fit_the_window();
    }
    Joining::Done
}

/// The first size: the document the opening thread opened is taken in, and told its viewport.
///
/// The viewport goes in before any of the open's events is answered, so that each is answered
/// against the viewer a first open with a viewport would have left (ADR 1539). **It waits no
/// longer than `crate::drawing::WARN`** — or `GLIMPSE_AFTER`, where page one is drawn while it is
/// read and its looks are what the window shows (ADRs 1811, 1855) — and a document still opening
/// then is left to its thread, joined by [`NativeWindow::mind_the_opening`] on the tick that finds
/// it ready, with the window's size as it is then.
fn join_the_opening<W: NativeWindow + ?Sized>(window: &mut W, resize: Command) -> Joining {
    let state = window.state_mut();
    let drawing_ahead = state.ahead.is_some();
    if state.anticipated.as_ref().is_some_and(|opening| {
        !if drawing_ahead {
            opening.wait_within(crate::drawing::GLIMPSE_AFTER)
        } else {
            opening.wait()
        }
    }) {
        state
            .trace
            .say(Topic::Launch, format_args!("document still opening"));
        state.awaiting = Some(resize);
        window.show_the_opening();
        return Joining::StillOpening;
    }
    window.take_the_opening(resize);
    Joining::Done
}
