//! One document, one viewer, and everything the Qt side asks it for.
//!
//! The loop is the one `viewer-ui`, `tests/headless.rs` and `viewer-gtk` already run — a command
//! in, the events it produced out, and the ones asking for work answered until nothing is left —
//! because the vocabulary is what is being tested and a host that invented its own scheduling
//! would be testing that instead.
//!
//! # No Qt type appears in this file
//!
//! That is not a style rule; it is what makes the crate's `unsafe` position defensible. Every
//! `unsafe` token in `viewer-qt` is inside one macro expansion in `crate::bridge`, and it stays
//! there because the Rust half of the host never touches a `QObject`. What crosses is values:
//! rows, controls, quadrilaterals, a frame's dimensions and a borrowed slice of its pixels.
//!
//! # Where the state lives, and why it is a plain `Box`
//!
//! C++ owns this struct for the whole life of `QApplication::exec` and calls into it from Qt's
//! signal handlers, one at a time, on the one thread Qt runs its event loop on. There is no path
//! from here back into Qt, so there is no re-entrancy to guard: `viewer-gtk` needs
//! `Rc<RefCell<Host>>` and a `try_borrow_mut` because `gtk4-rs` callbacks are `'static` closures
//! the Rust side installs, and this host needs neither because the callbacks are C++ lambdas.
//! Two ownership models, one vocabulary — ADR 0246.

use std::path::{Path, PathBuf};

use pdf_model::view::WidgetAppearances;
use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Edit, Entered, Event, Extraction, Find, FindDirection, FormField,
    PageTarget, PointerAction, PresentationMode, Query, Viewer, Zoom,
};
use viewer_host::ControlFit;
use viewer_host::arrangement::next_layout;
use viewer_host::form::{ControlKind, control_kind};
use viewer_host::panel::{PanelRow, RowAction, Tab};
use viewer_host::trace::{Topic, Trace};

use crate::bridge::ffi::{
    QtChrome, QtControl, QtFrame, QtMeasure, QtPage, QtPopup, QtQuad, QtRow, QtUpdate,
};
use crate::keys;
use crate::page;

/// The identity this host gives the one document it opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// The window this host asks Qt for, in logical pixels.
///
/// The same 1000×1100 `viewer-gtk` asks for, for the same reason: two hosts whose windows are the
/// same size produce launch numbers that can be read side by side.
const WINDOW: (i32, i32) = (1000, 1100);

/// Why the host could not start.
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    /// The file named on the command line could not be read.
    #[error("cannot read {path}: {error}")]
    Unreadable {
        /// What was named.
        path: PathBuf,
        /// What the operating system said.
        error: String,
    },
}

/// How many panels this window has, which is `viewer_host::Tab`'s own count.
///
/// §12.3.3's outline, §12.3.4's miniatures, §8.11.4.3's `/Order`, §7.11.4's embedded files,
/// §12.4.3's threads and §14.3.3's information. Five of the six are answers of five different
/// types that `viewer_host::panel` turns into one row shape, so the only thing left to distinguish
/// is which panel a row is in and it crosses the bridge as an index into
/// [`viewer_host::Tab::ALL`]. The sixth is §12.3.4's and holds no rows at all — its pictures are
/// asked for a page at a time by [`Host::page_row`] — so its slot here stays empty and the C++
/// side puts a `QListView` where the other five have a `QTreeView`.
const PANELS: usize = Tab::ALL.len();

/// One row of a tree as the C++ side sees it, beside what acting on it does.
///
/// The flattening is Qt's requirement rather than a convenience: `QAbstractItemModel` must answer
/// `rowCount` and `parent` for every node at any moment, so a lazily-built tree is not available
/// to it the way `GtkTreeListModel`'s is. ADR 0246.
#[derive(Debug, Clone)]
struct Flat {
    /// What the row shows and how deep it is.
    row: QtRow,
    /// What acting on it does, which stays on this side of the bridge.
    action: RowAction,
}

/// One control placed over the page, and which field's widget it is for.
#[derive(Debug, Clone)]
struct Placement {
    /// The two names §14.9.3 makes a processor distinguish: one to address an edit to, one to say.
    ///
    /// It was §12.7.4.2's qualified name alone until the seven-hundred-and-thirty-fifth session,
    /// which is why every sentence this host said about a field named it the way a *file* does
    /// rather than the way the clause says a user interface shall.
    name: pdf_model::view::FieldName,
    /// The widget annotation, which with the name above identifies this control.
    annotation: pdf_syntax::ObjectId,
    /// What kind of control it is.
    kind: ControlKind,
    /// The appearance state §12.7.5.2.3 makes a check box's on value, where the file names one.
    on_state: Option<String>,
    /// Table 229 bit 15: whether clicking the selected radio button of a set turns it off.
    no_toggle_to_off: bool,
    /// Table 227 bit 1: "the field shall not be modified by the user".
    ///
    /// The control is disabled, so a person cannot reach it — and an assistive technology's click
    /// does not go through the control, which is why the flag has to be carried rather than left
    /// to the widget's own sensitivity (ADR 0630).
    read_only: bool,
}

impl Placement {
    /// What identifies this control between two frames: the field's name and its widget.
    fn key(&self) -> (String, pdf_syntax::ObjectId) {
        (self.name.qualified.clone(), self.annotation)
    }
}

/// One document, one viewer, and the loop between them.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about one window, each read in one place and none a state"
)]
pub struct Host {
    /// The state machine every host on this boundary drives.
    pub(crate) viewer: Viewer,
    /// Where the bytes came from, which is what rule 2 makes a host's business and not the core's.
    path: PathBuf,
    /// The directory §12.7.6.4's policy resolves against.
    directory: Option<PathBuf>,
    /// The file, kept because §7.6.4.1's second attempt opens it again with a password.
    bytes: pdf_syntax::FileBytes,
    /// Annex O's fragment, where the host was given one.
    fragment: Option<String>,
    /// Tier 1's worker for the pictures this window makes *for itself* — §12.3.4's miniatures and
    /// §12.4.4.1's two transition faces.
    ///
    /// **The page is no longer drawn here**, which is ADR 0668: a display list this program did not
    /// write is drawn on [`Host::drawing`]'s thread, so that a page written to be expensive cannot
    /// take `QApplication::exec` with it. What is left on this thread is bounded by *this host's*
    /// own asking rather than by what a document chose to state.
    rasterizer: CpuRasterizer,
    /// The page rasteriser, on a thread of its own.
    ///
    /// `viewer_host::Drawing` is the whole arrangement, shared with `viewer-gtk`: what is Qt's is
    /// the `QTimer` that asks it whether anything has landed, and the interval for that is
    /// [`Host::drawing_wait`] — pulled rather than pushed, for this crate's own rule.
    drawing: viewer_host::Drawing,
    /// Which page this window has told the person is taking a long time to draw.
    ///
    /// **The state Escape's third row depends on** (`viewer_host::Waiting`), held rather than
    /// asked for twice because it decides two things: what the status bar is saying, and what the
    /// key means while it says it. The same field `viewer-gtk` holds, for the same reason and off
    /// the same poll — `viewer_host::Drawing::overlong` is the question and the wait is
    /// `viewer-host`'s.
    warned: Option<usize>,
    /// The launch timeline.
    pub(crate) trace: Trace,
    /// §6.3.2.2: who draws §12.7's widgets — this host's controls, or the document's own pictures.
    widget_appearances: WidgetAppearances,
    /// How much of what the document asserts over its reader this window obeys.
    ///
    /// The other policy value beside the one above, and the same kind of thing: a fact about what
    /// *this reader* has been asked to do rather than about the file.
    /// [`viewer_core::RestrictionLevel::On`] unless [`viewer_host::IGNORE_RESTRICTIONS`] was on the
    /// command line — which this program refused as an unknown option while telling every person
    /// who hit a refusal to use it (ADR 0604).
    restrictions: viewer_core::RestrictionLevel,
    /// Device pixels per logical pixel, from the screen Qt put the window on.
    scale: f32,
    /// Whether the reader wants the panel of three trees on the screen.
    ///
    /// **A wish, beside Table 29's *permission***. [`Host::chrome`] shows the panel only where the
    /// clause allows one and the person asked for one, so leaving full screen puts back what the
    /// reader had. `o` is the key, in all three hosts since ADR 0526.
    panel_shown: bool,
    /// §7.6.4.1's attempts, counted by [`viewer_host::Asking`] so that three hosts count alike.
    asking: viewer_host::Asking,
    /// What to put above the entry, worded when the prompt is asked for and read once by C++.
    prompt: String,
    /// Whether the document has been opened yet, which waits for the first resize.
    opened: bool,
    /// Whether anything is unsaved.
    dirty: bool,
    /// What the title bar says about the page.
    caption: String,
    /// The most recent sentence for a person.
    message: String,
    /// The three trees.
    trees: [Vec<Flat>; PANELS],
    /// The fields of the page being shown, as `Query::Fields` last answered.
    fields: Vec<FormField>,
    /// One entry per control, in the order the C++ side placed them.
    placed: Vec<Placement>,
    /// §12.5.6.14's open windows, as `Query::Popups` last answered.
    ///
    /// Kept rather than asked for twice: `refresh` compares this against the new answer to decide
    /// whether the C++ side has to rebuild the widgets, and `popups` then hands over what the
    /// comparison was made on. `viewer_core::PopupWindow` is `PartialEq`, which is the whole test.
    popups_shown: Vec<viewer_core::PopupWindow>,
    /// Whether the pointer was last over §12.5.6.5's activation region.
    ///
    /// Kept so that the cursor is changed when the answer changes rather than on every motion
    /// event, which is the same economy `viewer-gtk` makes for the same reason.
    over_link: bool,
    /// What has changed since the C++ side last asked.
    update: QtUpdate,
    /// Table 29's `/PageMode /FullScreen`, §12.2's chrome flags, and the way back out.
    ///
    /// **The window §12.4.4's presentation had never had** (ADR 0470). Which sentence this window
    /// is obeying is `viewer_host::Presenting`, shared with the other two hosts; what is Qt's is
    /// `QWidget::showFullScreen` and which widget each of Table 147's three flags names — and
    /// that half is in `cpp/window.cpp`, because the widgets are.
    presenting: viewer_host::Presenting,
    /// Whether the first frame has been reported, so that the launch line is printed once.
    presented: bool,
    /// §14.7's tree on AT-SPI, brought up after the first frame and never before it (ADR 0623).
    pub(crate) accessibility: Option<viewer_accessibility::Bridge>,
    /// The page and viewport last published to it, so that a tree is not rebuilt per frame.
    pub(crate) spoken: Option<viewer_accessibility::Showing>,
    /// Where the window is on the screen, as Qt last reported it: the frame, then the contents.
    ///
    /// **The half of AT-SPI's geometry `viewer-gtk` has no answer for.** It is kept here rather
    /// than handed straight on because a `moveEvent` arrives before the first paint and the
    /// adapter does not exist until after it.
    pub(crate) window_at: Option<crate::access::WindowPlace>,
    /// Set from `accesskit_unix`'s own thread when a client asks for something.
    pub(crate) access_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// What the find bar is looking for, kept because the page's highlights are asked for on
    /// every repaint while the document-wide search is a plan inside `viewer-core`.
    needle: String,
    /// How many pages a search still has to read. Zero when nothing is being searched for.
    pages_left: usize,
    /// The magnification at which every control on this page would fit its `/Rect`, where they do
    /// not fit now.
    ///
    /// **ADR 0245's third decision, on the far side of a bridge.** `viewer_host::ControlFit`
    /// computes it from what `Query::Fields` answers and what Qt says a control's minimum is, and
    /// the second of those two is the reason this host was the last to have it: a
    /// `minimumSizeHint` is a C++ value, so the numbers have to cross before the arithmetic can
    /// happen. `w` sends the answer as `Zoom::Scale`, as it does in `viewer-gtk`; it is offered
    /// rather than applied, because a viewer that magnified a page by itself because a form is on
    /// it would be answering a question nobody asked (rule 5).
    fit_magnification: Option<f32>,
    /// §12.4.4.1's clock, while a presentation is running.
    ///
    /// **`None` is a window whose `QTimer` is stopped**, rather than one that fires to discover
    /// it has nothing to advance. The decision inside it — how often, what a tick carries, when
    /// Table 164's `/D` has run out — is `viewer_host::Clock`, shared with the other two hosts.
    clock: Option<viewer_host::Clock>,
    /// A transition named by the core, waiting for the page it moves *to* to be rendered.
    arming: Option<pdf_model::navigation::Transition>,
    /// The page on the screen, as the list that drew it and where a whole-viewport draw would
    /// put it — the face a transition would move *from*. Kept only while presenting.
    shown: Option<(
        std::sync::Arc<pdf_render::DisplayList>,
        pdf_render::TargetSpec,
    )>,
    /// The frame of a transition in flight, which is what this window shows instead of a page.
    ///
    /// A whole `Raster` rather than a display list, because what crosses this bridge is pixels:
    /// `frame_pixels` hands the C++ side a borrowed slice and a transition frame has to be
    /// somewhere for that borrow to point at.
    playing: Option<pdf_render::Raster>,
    /// The viewport in device pixels, which is the rectangle a transition's frames are drawn in.
    pub(crate) viewport: (u32, u32),
    /// Table 29's arrangement, as this window last asked for it — what `l` cycles from.
    layout: pdf_model::viewer_preferences::PageLayout,
    /// §14.8.2.5's text between the key that copied it and the C++ side taking it to `QClipboard`.
    ///
    /// Empty at every other moment, because `take_clipboard` clears it: this is a hand-over and
    /// not a second clipboard. `viewer-ui` keeps its string for a different reason — it is what
    /// Ctrl + V pastes into a field — and this host pastes with `QLineEdit`'s own binding, so
    /// there is nothing here to keep.
    clipboard: String,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Host")
            .field("path", &self.path)
            .field("opened", &self.opened)
            .field("dirty", &self.dirty)
            .field("placed", &self.placed.len())
            .finish_non_exhaustive()
    }
}

impl Host {
    /// Reads the document's bytes and builds the host around them.
    ///
    /// The document is **not** opened here: it is opened on the first resize, because the
    /// viewport's size decides the resolution page one is rasterised at and a page drawn at a
    /// guessed size would be drawn twice. That is `viewer-gtk`'s choice too, so the two hosts'
    /// launch timelines measure the same thing.
    ///
    /// # Errors
    ///
    /// [`HostError::Unreadable`] where the file named cannot be read.
    pub fn open(
        path: &Path,
        fragment: Option<String>,
        widget_appearances: WidgetAppearances,
        restrictions: viewer_core::RestrictionLevel,
        trace: Trace,
    ) -> Result<Self, HostError> {
        // Open on disk rather than read whole: the core reads what page one needs through the
        // handle, and the file's size stops being the launch's cost (ADR 0809).
        let bytes =
            pdf_syntax::FileBytes::on_disk(path).map_err(|error| HostError::Unreadable {
                path: path.to_owned(),
                error: error.to_string(),
            })?;
        trace.say(
            Topic::Launch,
            format_args!("opened {} bytes of {} on disk", bytes.len(), path.display()),
        );
        Ok(Self {
            viewer: Viewer::new(1, 1, 1.0),
            path: path.to_owned(),
            directory: path.parent().map(Path::to_owned),
            bytes,
            fragment,
            rasterizer: CpuRasterizer::new(),
            // No thread yet, and none until a page needs one: `CLAUDE.md` section 2's rule that
            // nothing page one does not need happens before page one.
            drawing: viewer_host::Drawing::new(),
            warned: None,
            trace,
            widget_appearances,
            restrictions,
            scale: 1.0,
            // The panel is what this window opens with, and `o` is what takes it away.
            panel_shown: true,
            asking: viewer_host::Asking::new(),
            prompt: String::new(),
            opened: false,
            dirty: false,
            caption: String::new(),
            message: String::new(),
            trees: std::array::from_fn(|_| Vec::new()),
            fields: Vec::new(),
            placed: Vec::new(),
            popups_shown: Vec::new(),
            over_link: false,
            update: nothing_changed(),
            presented: false,
            accessibility: None,
            spoken: None,
            window_at: None,
            access_pending: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            needle: String::new(),
            pages_left: 0,
            fit_magnification: None,
            clock: None,
            arming: None,
            shown: None,
            playing: None,
            viewport: (1, 1),
            // Table 147's and Table 29's own defaults, replaced by what the catalog states the
            // moment the document opens.
            presenting: viewer_host::Presenting::default(),
            layout: pdf_model::viewer_preferences::PageLayout::SinglePage,
            clipboard: String::new(),
        })
    }

    // ---------------------------------------------------------------------------------------
    // What the C++ side calls. Every one of these is a `Command` or a question, and none of them
    // knows what a widget is.
    // ---------------------------------------------------------------------------------------

    /// The viewport changed size, in device pixels.
    pub(crate) fn resized(&mut self, width: u32, height: u32, scale: f32) {
        if width == 0 || height == 0 || !scale.is_finite() || scale <= 0.0 {
            return;
        }
        self.scale = scale;
        // §12.4.4.1's transition is drawn in the viewport rather than in a page's own rectangle,
        // so the size the core is told is also the size a frame is shaped for.
        self.viewport = (width, height);
        self.dispatch(Command::Resize {
            width,
            height,
            scale,
        });
        if !self.opened {
            self.opened = true;
            self.trace.say(
                Topic::Launch,
                format_args!("first resize {width}x{height} device px, scale {scale}"),
            );
            self.open_document(None);
        }
    }

    /// A key was pressed, as `Qt::Key`, with Qt's own modifier state beside it.
    ///
    /// **What it means is [`viewer_host::keys`]'s answer and not this host's** (ADR 0526). This
    /// method used to hold five special cases and a table, and the table disagreed with the other
    /// two hosts about the arrow keys, about `f` and about Escape; what is left is
    /// [`keys::stated`] turning a number into a [`viewer_host::Key`] and [`Host::window_act`]
    /// doing the half of the answer that is a widget's rather than a message.
    pub(crate) fn key(&mut self, code: u32, shift: bool) {
        let Some(stated) = keys::stated(code) else {
            return;
        };
        let mode = if self.presenting.full_screen() {
            viewer_host::Mode::Presenting
        } else {
            viewer_host::Mode::Reading
        };
        let waiting = self.waiting();
        let Some(meaning) =
            viewer_host::meaning(stated, shift || keys::shifted_by_name(code), mode, waiting)
        else {
            return;
        };
        match meaning {
            viewer_host::Meaning::Send(command) => {
                // §12.5.6.10's markups are defined over selected text, so a press with nothing
                // selected asks for an annotation over nothing. The core answers by doing nothing,
                // which is right and silent — and trap 5 is that a person who pressed a key and
                // saw no change has been told nothing at all.
                if matches!(command, Command::Edit(Edit::Markup { .. })) && !self.has_selection() {
                    self.say("select some text first — §12.5.6.10's markups mark up text");
                    return;
                }
                self.dispatch(command);
            }
            viewer_host::Meaning::Window(act) => self.window_act(act),
        }
    }

    /// The half of the key table that is a widget's rather than a message.
    ///
    /// Matched exhaustively and with no catch-all arm, which is `doc/ui-boundary.md`'s rule applied
    /// one layer out: a binding added to [`viewer_host::keys`] fails to compile in all three hosts.
    ///
    /// **Four of these arms are a flag rather than a call**, and the reason is this crate's shape
    /// rather than Qt's: [`crate::bridge`] states that C++ owns the `Host` for the life of
    /// `QApplication::exec` and that Rust never calls a Qt object, which is what keeps the crate to
    /// one hand-written `unsafe` token. So showing a find bar, a panel or a card of notices is a
    /// [`QtUpdate`](crate::bridge::ffi::QtUpdate) field the window reads back — the shape `window`
    /// has had since ADR 0470 and `clipboard` since ADR 0519.
    fn window_act(&mut self, act: viewer_host::WindowAct) {
        match act {
            // Qt places widgets in logical pixels and `Command::Scroll` speaks device ones, which
            // is why the table states a distance rather than building the message itself.
            viewer_host::WindowAct::ScrollBy(by) => self.dispatch(Command::Scroll {
                dx: 0.0,
                dy: by * self.scale,
            }),
            // The only binding whose answer leaves the program: §14.8.2.5's text on the session's
            // clipboard. Not a command, because what a copy *is* belongs to the platform (ADR
            // 0519).
            viewer_host::WindowAct::Copy => self.copy_selection(),
            viewer_host::WindowAct::Find => self.update.find_bar = true,
            // Table 29's "any other window visible" is a *permission* and this is the reader's
            // wish beside it; `Host::chrome` composes the two, so leaving full screen puts back
            // what the reader had rather than what the document last permitted.
            viewer_host::WindowAct::Panel => {
                self.panel_shown = !self.panel_shown;
                self.update.window = true;
            }
            viewer_host::WindowAct::Notices => self.update.notices = true,
            viewer_host::WindowAct::Present | viewer_host::WindowAct::LeaveFullScreen => {
                self.present_or_stop();
            }
            // Table 29's six arrangements are cycled, so what to send depends on which one is in
            // force — a fact about this host's state rather than about the key.
            viewer_host::WindowAct::NextLayout => {
                self.layout = next_layout(self.layout);
                self.dispatch(Command::Layout(self.layout));
                self.say(&format!("page layout: {:?} (§7.7.2)", self.layout));
                // A new arrangement is a new set of pages on the screen, and therefore a new set
                // of things they could not draw.
                self.restate();
            }
            // What `w` sends depends on what this page's controls measured, so the command cannot
            // be built from the key alone. `viewer-gtk` answers the same way.
            viewer_host::WindowAct::FitControls => match self.fit_magnification {
                Some(wanted) => {
                    self.say(&format!("fitting §12.7's controls at {wanted:.3}"));
                    self.dispatch(Command::Zoom {
                        zoom: Zoom::Scale(wanted),
                        at: None,
                    });
                }
                None => self.say("every control on this page already fits its /Rect"),
            },
            // **Refused by name rather than ignored**, which is trap 5 and the honest answer here:
            // §12.5.6.6's annotation is authored by dragging a rectangle and then typing into it,
            // and this host has neither the drag mode nor the editor. `doc/todo/30` carries it as
            // the remaining asymmetry rather than leaving it silent.
            viewer_host::WindowAct::FreeText => self.say(
                "this host cannot draw a §12.5.6.6 free text annotation yet — the drag mode and \
                 its editor are viewer-ui's alone (doc/todo/30)",
            ),
            viewer_host::WindowAct::AbortDrawing => self.stop_the_long_draw(),
        }
    }

    /// Table 29's two display entries, and ISO 32000-2 §12.2's chrome flags with them.
    ///
    /// `/PageLayout` is the viewer's to apply, and what this host needs from it is the value `l`
    /// cycles from — see `viewer-gtk`, which does the same. `/PageMode` is "how the document shall
    /// be displayed when opened", which since ADR 0470 includes a full-screen window for the one
    /// name that used to get a note saying this program had no such thing.
    fn obey_the_catalog(&mut self, queue: &mut std::collections::VecDeque<Command>) {
        let Answer::Opening(opening) = self.viewer.query(Query::Opening) else {
            return;
        };
        self.layout = opening.layout;
        if opening.layout != pdf_model::viewer_preferences::PageLayout::SinglePage {
            self.say(&format!(
                "this document opens in the {:?} page layout (§7.7.2)",
                opening.layout
            ));
        }
        if let Answer::Preferences(preferences) = self.viewer.query(Query::Preferences) {
            self.presenting = viewer_host::Presenting::opening(opening, &preferences);
            // Trap 5: a document asking for something and getting silence. Two of Table 147's
            // three flags name widgets this window has and the third does not — none of the
            // three hosts draws a menu bar.
            if preferences.hide_menubar {
                self.say(
                    "this document asks to hide the menu bar (§12.2's /HideMenubar), which this \
                     window does not have; /HideToolbar and /HideWindowUI are obeyed",
                );
            }
        }
        if self.presenting.full_screen() {
            self.start_the_clock();
            // Queued rather than dispatched: this runs inside `react`, and `dispatch` would start
            // a second `pump` under the first.
            queue.push_back(Command::Present(self.presenting.mode()));
            self.say("presenting full screen (§7.7.2's FullScreen page mode) — Escape comes back");
        }
        self.update.window = true;
    }

    /// Enters or leaves §12.4.4's presentation, which for this program is the full-screen window.
    ///
    /// The same letter and the same act as the other two hosts; what differs is that the *widgets*
    /// are C++'s, so this side flags [`QtUpdate::window`] and the window asks
    /// [`Host::chrome`] what it should look like now.
    fn present_or_stop(&mut self) {
        if self.presenting.toggle() {
            self.start_the_clock();
            self.dispatch(Command::Present(PresentationMode::On));
            self.say(
                "presenting full screen (§7.7.2's FullScreen page mode) — §12.4.4's /Dur advances \
                 the page, its /Trans is drawn, and Escape comes back",
            );
        } else {
            self.stop_the_clock();
            self.dispatch(Command::Present(PresentationMode::Off));
            // §12.2: "[t]he document's page mode, specifying how to display the document on
            // exiting full-screen mode". `None` is Table 147's own condition unmet, and then what
            // goes back is what the reader had — a choice rather than a reading (ADR 0470).
            match self.presenting.on_exit() {
                Some(mode) => {
                    self.say(&format!("§12.2's /NonFullScreenPageMode asks for {mode:?}"));
                }
                None => self.say("presentation stopped"),
            }
        }
        self.update.window = true;
    }

    /// Starts §12.4.4.1's clock, which is what makes this a presentation rather than a big page.
    fn start_the_clock(&mut self) {
        if self.clock.is_none() {
            self.clock = Some(viewer_host::Clock::started(std::time::Instant::now()));
        }
    }

    /// Stops it, and takes the frame in flight with it.
    ///
    /// The `QTimer` on the C++ side stops the next time it asks [`Host::presentation_wait`], which
    /// is immediately: leaving full screen goes through `applyUpdates`. A window that is not
    /// presenting has nothing to advance, and `CLAUDE.md`'s principle 2 makes a wakeup with
    /// nothing behind it a defect rather than a rounding error.
    fn stop_the_clock(&mut self) {
        self.clock = None;
        self.arming = None;
        self.shown = None;
        if self.playing.take().is_some() {
            self.update.frame = true;
        }
    }

    /// How long the window should wait before the next turn of the clock, in milliseconds.
    ///
    /// `-1` where nothing is presenting, which is what stops the timer. Asked after every pump
    /// rather than set once, because the interval a transition wants and the interval a still
    /// page wants are not the same number.
    pub(crate) fn presentation_wait(&self) -> i32 {
        self.clock.as_ref().map_or(-1, |clock| {
            i32::try_from(clock.interval().as_millis()).unwrap_or(i32::MAX)
        })
    }

    /// One turn of §12.4.4.1's clock: tell the core how much time passed, and draw what is due.
    ///
    /// **A tick that produces no events repaints nothing**, which is this host's answer to a
    /// viewer that must idle. A page stating no `/Dur` — "the page shall not advance
    /// automatically" — swallows every tick, so the window wakes ten times a second, adds a
    /// number, and flags no update at all.
    pub(crate) fn presentation_tick(&mut self) {
        let now = std::time::Instant::now();
        let animating = self
            .clock
            .as_ref()
            .is_some_and(viewer_host::Clock::animating);
        let Some(millis) = self.clock.as_mut().and_then(|clock| clock.tick(now)) else {
            // Held: a transition is being drawn, and §12.4.4.1's EXAMPLE puts that before the
            // page is displayed. What is due is a frame.
            if animating {
                self.draw_a_frame(now);
            }
            return;
        };
        let events: Vec<Event> = self.viewer.handle(Command::Tick { millis }).collect();
        if events.is_empty() {
            return;
        }
        let mut queue = std::collections::VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue.into_iter().collect());
    }

    /// The viewport a transition's frames are shaped in, in this window's own device pixels.
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    fn viewport_rect(&self) -> pdf_render::Rect {
        pdf_render::Rect::from_corners(
            pdf_render::Point::new(0.0, 0.0),
            pdf_render::Point::new(self.viewport.0 as f32, self.viewport.1 as f32),
        )
    }

    /// Rasterises the frame of the transition in flight, or puts the page back when it has ended.
    fn draw_a_frame(&mut self, now: std::time::Instant) {
        let viewport = self.viewport_rect();
        let shaped = self.clock.as_mut().map(|clock| clock.frame(viewport, now));
        let list = match shaped {
            Some(Ok(Some(list))) => list,
            Some(Err(problem)) => {
                // Not reachable from a frame — the largest one adds four clips — and said rather
                // than swallowed for the reason every refusal in this tree is.
                self.say(&format!("transition: this frame would not draw: {problem}"));
                self.playing = None;
                self.update.frame = true;
                return;
            }
            None | Some(Ok(None)) => {
                self.playing = None;
                self.update.frame = true;
                return;
            }
        };
        let target = pdf_render::TargetSpec {
            width: self.viewport.0,
            height: self.viewport.1,
            transform: pdf_render::Transform::IDENTITY,
        };
        match self.rasterizer.rasterize(&list, target) {
            Ok(raster) => self.playing = Some(raster),
            Err(error) => {
                self.say(&format!(
                    "transition: this frame would not rasterise: {error}"
                ));
                self.playing = None;
            }
        }
        self.update.frame = true;
    }

    /// Takes `transition` to be drawn when the page it moves *to* arrives, or says why it is not.
    ///
    /// Armed rather than begun: `Viewer::handle` settles after the command that turned the page,
    /// so the events arrive as page change, transition, render request, and the arriving page's
    /// list is in the last of the three. §12.4.4.1's transition is one *to* a page.
    fn arm_transition(&mut self, transition: pdf_model::navigation::Transition) {
        if self.clock.is_none() {
            self.say(&format!(
                "transition: {:?} over {} s — nothing is presenting, so the page is shown at once \
                 (press p)",
                transition.style, transition.duration
            ));
            return;
        }
        if !viewer_host::Clock::shapes(&transition, self.viewport_rect()) {
            // The core has already said *why* through `Event::Reported`.
            return;
        }
        self.arming = Some(transition);
    }

    /// Keeps the page just rendered as a transition's face, and begins one that was armed.
    ///
    /// Two whole-viewport rasterisations happen here and none per frame: the page being left and
    /// the page arriving, each drawn where a frame will place it.
    fn face_arrived(&mut self, request: &viewer_core::RenderRequest) {
        if self.clock.is_none() {
            self.shown = None;
            self.arming = None;
            return;
        }
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        let arriving = (
            std::sync::Arc::clone(&request.list),
            viewer_host::face_target(request.target, origin, self.viewport),
        );
        let Some(transition) = self.arming.take() else {
            self.shown = Some(arriving);
            return;
        };
        let leaving = self.shown.replace(arriving.clone());
        let Some((list, target)) = leaving else {
            self.say(&format!(
                "transition: {:?} was named with no page to move from, so the page is shown at \
                 once",
                transition.style
            ));
            return;
        };
        let began = std::time::Instant::now();
        let (Some(outgoing), Some(incoming)) =
            (self.face(&list, target), self.face(&arriving.0, arriving.1))
        else {
            self.say(&format!(
                "transition: {:?} was named but the pages behind it would not rasterise, so the \
                 page is shown at once",
                transition.style
            ));
            return;
        };
        self.trace.say(
            Topic::Frames,
            format_args!(
                "TRANSITION {:?} over {} s: two {}x{} pages rasterised in {:?}",
                transition.style,
                transition.duration,
                self.viewport.0,
                self.viewport.1,
                began.elapsed()
            ),
        );
        if let Some(clock) = self.clock.as_mut() {
            clock.begin(transition, outgoing, incoming, std::time::Instant::now());
        }
    }

    /// One page of a transition, drawn to the viewport's own pixels and ready to be drawn again.
    fn face(
        &mut self,
        list: &pdf_render::DisplayList,
        target: pdf_render::TargetSpec,
    ) -> Option<pdf_render::Image> {
        let raster = self.rasterizer.rasterize(list, target).ok()?;
        viewer_core::transition::drawable(&raster)
    }

    /// Which pieces of chrome this window may show, and whether it is full screen.
    pub(crate) fn chrome(&self) -> QtChrome {
        let chrome = self.presenting.chrome();
        QtChrome {
            full_screen: self.presenting.full_screen(),
            menu_bar: chrome.menu_bar,
            tool_bar: chrome.tool_bar,
            window_ui: chrome.window_ui,
            // **Two conditions rather than one since ADR 0526**: Table 29's permission, and the
            // reader's own wish, which `o` moves. Composed here so that leaving full screen puts
            // back what the reader had rather than what the document last permitted.
            other_windows: chrome.other_windows && self.panel_shown,
        }
    }

    /// Which of Table 29's six panels the document asks to be showing, as a tab index.
    ///
    /// The page mode in force: `/NonFullScreenPageMode` while full screen is ending and
    /// nothing while it is not, because §12.2 states no page mode for a window that never left.
    /// `-1` where this host has no panel for the name.
    pub(crate) fn panel_wanted(&self) -> i32 {
        let mode = if self.presenting.full_screen() {
            return -1;
        } else if let Some(mode) = self.presenting.on_exit() {
            mode
        } else {
            let Answer::Opening(opening) = self.viewer.query(Query::Opening) else {
                return -1;
            };
            opening.mode
        };
        // Which panel a page mode opens is `viewer_host::Tab`'s, shared with the other two hosts,
        // and **`UseThumbs` is no longer among the names that reach none**: this host reported it
        // as a mode it had no panel for until §12.3.4's list was built. `UseNone` asks for nothing
        // and `FullScreen` is the window rather than a panel, so those two still answer -1.
        Tab::of_page_mode(mode)
            .and_then(|tab| i32::try_from(tab.index()).ok())
            .unwrap_or(-1)
    }

    /// The wheel turned, already in the device pixels the boundary speaks.
    ///
    /// The conversion from Qt's notches is C++'s, beside the event that states them; what is here
    /// is the message, so that the two hosts differ in the toolkit and in nothing else.
    pub(crate) fn scrolled(&mut self, dx: f32, dy: f32) {
        self.dispatch(Command::Scroll { dx, dy });
    }

    /// The pointer moved or a button changed.
    pub(crate) fn pointer(&mut self, x: f32, y: f32, action: u8) {
        let action = match action {
            0 => PointerAction::Moved,
            1 => PointerAction::Pressed,
            2 => PointerAction::Dragged,
            3 => PointerAction::Released,
            // Trap 5: a number this host did not send is a mistake in the C++ half, and a silent
            // `Moved` would hide it behind a working-looking window.
            other => {
                self.say(&format!(
                    "the window sent pointer action {other}, which is none"
                ));
                return;
            }
        };
        self.dispatch(Command::Pointer { at: (x, y), action });
        // §12.5.6.5's activation region, asked at pointer speed — which is what makes
        // `Query::LinkAt` a query rather than a command. The clause states no cursor at all, so
        // what a reader sees over a link is a convention; this is the one all three hosts now
        // keep, and the flag is how it reaches a Qt object without Rust calling one.
        if let Answer::Link(over) = self.viewer.query(Query::LinkAt((x, y)))
            && over != self.over_link
        {
            self.over_link = over;
            self.update.cursor = true;
            self.trace.say(
                Topic::Pointer,
                format_args!(
                    "the pointer is {} §12.5.6.5's activation region",
                    if over { "over" } else { "off" }
                ),
            );
        }
    }

    /// A row of one of the three trees was activated.
    pub(crate) fn activate_row(&mut self, tree: u8, index: usize) {
        let Some(flat) = self.flat(tree, index) else {
            return;
        };
        match flat.action.clone() {
            RowAction::Activate(object) => self.dispatch(Command::Activate(object)),
            RowAction::Extract { name } => self.dispatch(Command::Extract { name }),
            // A switch is moved by `toggle_row` and a heading does nothing: §8.11.4.3's leading
            // text string is "a collection of related groups" under a heading, and the heading is
            // not a layer.
            RowAction::Toggle { .. } | RowAction::Inert => {}
        }
    }

    /// §8.11.4.3's switch on a row was moved.
    pub(crate) fn toggle_row(&mut self, tree: u8, index: usize, on: bool) {
        let Some(flat) = self.flat(tree, index) else {
            return;
        };
        if let RowAction::Toggle { group, locked, .. } = flat.action {
            // Table 99: "[t]he state of a locked group cannot be changed through the user
            // interface of an interactive PDF processor." The model refuses the click as well —
            // this is the second half of the same clause, said where the command is sent.
            if locked {
                self.say("that group is locked (Table 99) and its state cannot be changed");
                return;
            }
            self.dispatch(Command::SetGroup { group, on });
        }
    }

    /// A control's value was typed into.
    pub(crate) fn set_control(&mut self, index: usize, value: &str) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let field = placed.name.qualified.clone();
        self.dispatch(Command::Edit(Edit::SetField {
            field,
            value: Entered::Text(value.to_owned()),
        }));
    }

    /// §12.7.5.4: which of Table 234's `/Opt` entries are selected now.
    ///
    /// An empty set is a field with nothing chosen, which the clause makes a real state — "[t]he
    /// default value of V is null , indicating that no item is currently selected" — and not a
    /// reason to send nothing.
    pub(crate) fn choose_control(&mut self, index: usize, chosen: &[u32]) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let field = placed.name.qualified.clone();
        let chosen: Vec<usize> = chosen
            .iter()
            .filter_map(|index| usize::try_from(*index).ok())
            .collect();
        self.trace.say(
            Topic::Panel,
            format_args!("{field}: option(s) {chosen:?} of Table 234's /Opt selected"),
        );
        self.dispatch(Command::Edit(Edit::SetField {
            field,
            value: Entered::Chosen(chosen),
        }));
    }

    /// §12.7.5.2.3's check box or §12.7.5.2.4's radio button was clicked.
    ///
    /// The *name* the value takes is this side's, because it is a clause and not a widget:
    /// §12.7.5.2.3 makes `/V` select among Table 170's appearance states by name and the names
    /// are the file's own, so a host that sent "on" would be inventing one.
    ///
    /// **The rule is [`viewer_host::toggling`] since the seven-hundred-and-thirty-fifth session**,
    /// shared with the other two windows and with this host's own accessibility click, so a
    /// `QCheckBox`'s `toggled` and an assistive technology's `DoAction` cannot come to different
    /// answers about one clause (ADR 0630). What this method still owns is the *index*, which is
    /// the one thing only a placed control knows.
    pub(crate) fn toggle_control(&mut self, index: usize, on: bool) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let clicked = viewer_host::toggling(
            &placed.name,
            placed.read_only,
            on,
            placed.no_toggle_to_off,
            placed.on_state.as_deref(),
        );
        // `false`: the click did reach the control — it *is* the control's signal — so there is
        // nothing about a page coordinate to report.
        if let Some(said) = clicked.note(false) {
            self.say(&said);
        }
        match clicked {
            viewer_host::Clicked::Toggles { name, value } => {
                self.dispatch(Command::Edit(Edit::SetField {
                    field: name.qualified,
                    value: Entered::Text(value),
                }));
            }
            // A refusal leaves the field alone, and the `QAbstractButton` goes back to whatever
            // `Query::Fields` says on the next `applyUpdates` — which it does, unconditionally,
            // for exactly this reason.
            viewer_host::Clicked::ReadOnly { .. }
            | viewer_host::Clicked::Stays { .. }
            | viewer_host::Clicked::Unnamed { .. }
            | viewer_host::Clicked::Pointed { .. }
            | viewer_host::Clicked::Aimed { .. }
            | viewer_host::Clicked::Page => {}
        }
    }

    /// §12.7.5.2.2's push button was pressed.
    pub(crate) fn activate_control(&mut self, index: usize) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let annotation = placed.annotation;
        self.dispatch(Command::Activate(annotation));
    }

    /// §7.6.4.1: a person typed a password, or closed the prompt with nothing in it.
    ///
    /// Exhaustive over `Supplied` on purpose, which is what holds three hosts level.
    pub(crate) fn supply_password(&mut self, password: &str) {
        match viewer_host::password::supplied(password.to_owned().into()) {
            viewer_host::Supplied::Open(secret) => self.open_document(Some(secret)),
            viewer_host::Supplied::Cancelled => self.say(viewer_host::password::CANCELLED),
        }
    }

    /// §7.6.4.1: what the prompt says, worded by [`viewer_host::password`] for all three hosts.
    pub(crate) fn password_prompt(&self) -> String {
        self.prompt.clone()
    }

    /// A toolbar button.
    pub(crate) fn command(&mut self, what: u8) {
        let command = match what {
            0 => Command::GoTo(PageTarget::Previous),
            1 => Command::GoTo(PageTarget::Next),
            2 => Command::Zoom {
                zoom: Zoom::Out,
                at: None,
            },
            3 => Command::Zoom {
                zoom: Zoom::In,
                at: None,
            },
            4 => Command::Zoom {
                zoom: Zoom::FitPage,
                at: None,
            },
            other => {
                self.say(&format!("the window sent command {other}, which is none"));
                return;
            }
        };
        self.dispatch(command);
    }

    /// Every control the window has just placed, measured against the rectangle it was given.
    ///
    /// The counting used to be in `cpp/window.cpp`, which meant the two native hosts computed the
    /// same finding twice from the same numbers and only one of them could offer a magnification.
    /// It is `viewer_host::ControlFit`'s now — `panel.rs`'s reason, and ADR 0346's: a mapping from
    /// rectangles to a number is not a statement about a document, and two hosts measuring the
    /// same thing must not be able to disagree about it.
    pub(crate) fn measured(&mut self, controls: &[QtMeasure]) {
        let mut fit = ControlFit::default();
        for control in controls {
            fit.record(
                (control.asked_width, control.asked_height),
                (control.minimum_width, control.minimum_height),
            );
        }
        let (placed, wider, taller, widest, tallest) = fit.counts();
        if placed == 0 {
            self.fit_magnification = None;
            return;
        }
        self.fit_magnification = self
            .showing_at()
            .and_then(|showing| fit.magnification(showing));
        self.trace.say(
            Topic::Panel,
            format_args!(
                "{wider} of {placed} control(s) wider than their /Rect (worst +{} on {} px), \
                 {taller} taller (worst +{} on {} px){}",
                widest.0,
                widest.1,
                tallest.0,
                tallest.1,
                match self.fit_magnification {
                    Some(wanted) => format!("; every control fits at {wanted:.3}, which `w` sends"),
                    None => String::from("; every control fits at this magnification"),
                }
            ),
        );
    }

    /// The magnification the page is drawn at now, in the units [`Zoom::Scale`] takes.
    ///
    /// `PageGeometry::scale` is device pixels per user space unit — "the zoom and the display's
    /// scale together" — and `Zoom::Scale` is *logical* pixels per user space unit, so the
    /// display's own factor comes back out. The same arithmetic `viewer-gtk` does, and it has to
    /// be the same: a doubled display would otherwise put one host's answer out by two.
    fn showing_at(&self) -> Option<f32> {
        let Answer::Page { index, .. } = self.viewer.query(Query::CurrentPage) else {
            return None;
        };
        let Answer::Geometry(geometry) = self.viewer.query(Query::PageGeometry(index)) else {
            return None;
        };
        let display = if self.scale.is_finite() && self.scale >= 1.0 {
            self.scale
        } else {
            1.0
        };
        let logical = geometry.scale / display;
        (logical.is_finite() && logical > 0.0).then_some(logical)
    }

    /// What has changed since this was last called, which also clears it.
    pub(crate) fn take_update(&mut self) -> QtUpdate {
        std::mem::replace(&mut self.update, nothing_changed())
    }

    /// Puts what is selected on the page where the C++ side will hand it to `QClipboard`.
    ///
    /// **The platform end of `doc/todo/30`'s first item** (ADR 0519), and it goes *out* through
    /// the update flag rather than by Rust calling Qt, which is the one property `crate::bridge`'s
    /// documentation states about this crate's direction of travel. The two questions and the
    /// choice between §14.8.2.5's orders are this side's; the clipboard is `window.cpp`'s.
    ///
    /// Which order the text is in is [`viewer_host::copied`] and not this host's, because it is
    /// the same decision in all three windowed hosts.
    fn copy_selection(&mut self) {
        // Owned before the second question, because both answers borrow the viewer.
        let page_order = match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.text.into_owned(),
            _ => String::new(),
        };
        let logical = match self.viewer.query(Query::LogicalSelection) {
            Answer::LogicalSelection(text) => Some(text),
            _ => None,
        };
        let Some(copied) = viewer_host::copied(logical, &page_order) else {
            self.say("nothing on the page is selected to copy");
            return;
        };
        self.say(&format!(
            "copied {} characters in {}",
            copied.text.chars().count(),
            copied.order
        ));
        self.clipboard = copied.text;
        self.update.clipboard = true;
    }

    /// The text a copy is putting on the clipboard, which this also clears.
    ///
    /// Taken rather than borrowed, for the reason every other `take_` on this bridge is: a
    /// `cxx::String` is a copy whichever way it crosses, and leaving the characters here
    /// afterwards would be this host holding a second clipboard nobody reads.
    pub(crate) fn take_clipboard(&mut self) -> String {
        std::mem::take(&mut self.clipboard)
    }

    /// Whether anything on the page is selected.
    ///
    /// Asked before §12.5.6.10's markup, which is defined over selected text: the core does
    /// nothing when there is nothing to mark up, and a person who pressed a key and saw no change
    /// has been told nothing at all.
    fn has_selection(&self) -> bool {
        matches!(self.viewer.query(Query::Selection),
            Answer::Selected(selection) if !selection.quads.is_empty())
    }

    /// The third-party notices this binary is obliged to carry, for the window `?` opens.
    ///
    /// **A licence obligation with a surface, and this host had neither half of it until ADR
    /// 0526**: `pdf-font` compiles the standard 14 font programs (§9.6.2.2) into every binary in
    /// this tree, both of their licences require a binary distribution to reproduce their notices,
    /// and `pdf-viewer-qt` reproduced them nowhere at all. The text is `viewer_host::NOTICE`,
    /// shared with the other two hosts.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn notices(&self) -> String {
        viewer_host::NOTICE.to_owned()
    }

    /// How many pages Table 29's arrangement is showing pixels for.
    ///
    /// One under `SinglePage`, which is what this host drew for two hundred sessions; more under
    /// a column or a spread, and the window paints each of them where [`Self::frame`] says.
    pub(crate) fn frame_count(&self) -> usize {
        // §12.4.4.1: while one of Table 164's effects is in flight the window shows the effect and
        // not the page — one picture, because a frame is already two pages placed where they
        // belong. Answered here rather than pushed across the bridge, so the C++ side draws a
        // transition with the code it already draws a page with.
        if self.playing.is_some() {
            return 1;
        }
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames.len(),
            _ => 0,
        }
    }

    /// Where one page's pixels belong and how big they are.
    pub(crate) fn frame(&self, index: usize) -> QtFrame {
        if let Some(raster) = self.playing.as_ref() {
            return match page::describe(raster) {
                // A transition frame is already in the viewport's own pixels, so its origin is
                // the viewport's own.
                Ok((width, height)) => QtFrame {
                    present: index == 0,
                    width,
                    height,
                    origin_x: 0.0,
                    origin_y: 0.0,
                },
                Err(error) => {
                    eprintln!("note: {error}");
                    no_frame()
                }
            };
        }
        let Answer::Frame(frames) = self.viewer.query(Query::Frame) else {
            return no_frame();
        };
        let Some(frame) = frames.get(index) else {
            return no_frame();
        };
        match page::describe(frame.raster) {
            Ok((width, height)) => QtFrame {
                present: true,
                width,
                height,
                origin_x: frame.origin.0,
                origin_y: frame.origin.1,
            },
            Err(error) => {
                // Trap 5: a host that quietly showed the previous page would be telling a
                // person something false about this one.
                eprintln!("note: {error}");
                no_frame()
            }
        }
    }

    /// The pixels themselves, borrowed rather than copied.
    ///
    /// Tier 1 is priced at "one copy per frame" and this is what keeps it to one: the `QImage`
    /// the C++ side builds copies out of this slice, and nothing on this side copies into it.
    pub(crate) fn frame_pixels(&self, index: usize) -> &[u8] {
        if let Some(raster) = self.playing.as_ref() {
            return if index == 0 {
                raster.data.as_slice()
            } else {
                &[]
            };
        }
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames
                .get(index)
                .map_or(&[][..], |frame| frame.raster.data.as_slice()),
            _ => &[],
        }
    }

    /// One panel's rows, depth first.
    pub(crate) fn rows(&self, tree: u8) -> Vec<QtRow> {
        self.tree(tree)
            .map(|rows| rows.iter().map(|flat| flat.row.clone()).collect())
            .unwrap_or_default()
    }

    /// What one of [`viewer_host::Tab`]'s panels is called.
    ///
    /// Asked rather than written into the C++ so that the six words are one set of six words: the
    /// same argument `notices` and `password_prompt` are here for.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn panel_label(&self, tab: u8) -> String {
        Tab::at(usize::from(tab)).map_or_else(String::new, |tab| tab.label().to_owned())
    }

    /// Which of the panels is §12.3.4's, which is the one that is a list of pictures.
    ///
    /// Asked rather than written into the C++ so that the *order* of the panels is stated once:
    /// a panel inserted before this one in `viewer_host::Tab::ALL` moves this number, and a window
    /// that had the position written down would put a `QListView` where a `QTreeView` belongs.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn pages_panel(&self) -> u8 {
        pages_panel_index()
    }

    /// How many rows §12.3.4's panel has, which is how many pages the document has.
    pub(crate) fn page_count(&self) -> usize {
        match self.viewer.query(Query::PageCount) {
            Answer::Count(count) => count,
            _ => 0,
        }
    }

    /// One row of §12.3.4's panel: the page's label, and its miniature where it states one.
    ///
    /// **Asked for one page at a time, when the model is about to draw that row**, which is the
    /// whole of `CLAUDE.md` section 2's rule reaching this panel — a call per page at build time would
    /// have moved the eager decode out of the launch path rather than out of the program.
    ///
    /// The picture is copied out rather than borrowed, unlike a frame's: a miniature is tens of
    /// kilobytes against a frame's megabytes, it is asked for once per row rather than once per
    /// present, and the C++ side keeps a `QPixmap` of it afterwards — so a borrow would buy one
    /// avoided copy at the price of a second cache on this side of the bridge.
    pub(crate) fn page_row(&self, index: usize) -> QtPage {
        let entry = viewer_host::page_entry(&self.viewer, index);
        match entry.thumbnail {
            Some(image) => QtPage {
                label: entry.label,
                width: image.width,
                height: image.height,
                pixels: image.data.to_vec(),
            },
            None => QtPage {
                label: entry.label,
                width: 0,
                height: 0,
                pixels: Vec::new(),
            },
        }
    }

    /// How many of §12.3.4's miniatures the panel may keep, from the one place that decides it.
    ///
    /// The `QPixmap` cache is C++'s, because a `QPixmap` is, and the *bound* is
    /// [`viewer_host::KEPT_MINIATURES`] — asked for across the bridge rather than written down a
    /// second time, which is the same reason the panel labels are asked for.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn kept_miniatures(&self) -> usize {
        viewer_host::KEPT_MINIATURES
    }

    /// §12.3.4: "allowing the user to navigate to a page by clicking its thumbnail image".
    ///
    /// A page index rather than a destination, because a thumbnail *is* the page.
    pub(crate) fn show_page(&mut self, index: usize) {
        self.dispatch(Command::GoTo(PageTarget::Index(index)));
    }

    /// Every control the page's form wants.
    pub(crate) fn controls(&self) -> Vec<QtControl> {
        let mut controls = Vec::with_capacity(self.placed.len());
        for (placed, (field, widget)) in self.placed.iter().zip(self.widgets()) {
            let (x, y, width, height) = viewer_host::bounds(widget.quad);
            let (kind, max_len, multi, editable) = describe_kind(&placed.kind);
            controls.push(QtControl {
                x,
                y,
                width,
                height,
                kind,
                field: placed.name.qualified.clone(),
                annotation: widget.annotation.number,
                value: field
                    .value
                    .as_ref()
                    .map_or_else(String::new, |shown| shown.text.clone()),
                obscured: field.value.as_ref().is_some_and(|shown| shown.obscured),
                read_only: field.read_only,
                on: widget.on || (on_by_value(&placed.kind) && widget.on_state.is_none()),
                max_len,
                multi,
                editable,
                top: top_option(&placed.kind),
                tooltip: tooltip(field),
            });
        }
        controls
    }

    /// Table 234's `/Opt` labels for one control.
    pub(crate) fn control_options(&self, index: usize) -> Vec<String> {
        match self.placed.get(index).map(|placed| &placed.kind) {
            Some(ControlKind::Combo { options, .. } | ControlKind::List { options, .. }) => {
                options.clone()
            }
            _ => Vec::new(),
        }
    }

    /// Which of them are selected.
    pub(crate) fn control_selection(&self, index: usize) -> Vec<u32> {
        let chosen: Vec<usize> = match self.placed.get(index).map(|placed| &placed.kind) {
            Some(ControlKind::Combo { selected, .. }) => selected.iter().copied().collect(),
            Some(ControlKind::List { selected, .. }) => selected.clone(),
            _ => Vec::new(),
        };
        chosen
            .into_iter()
            .filter_map(|index| u32::try_from(index).ok())
            .collect()
    }

    /// What is selected on the page.
    pub(crate) fn selection(&self) -> Vec<QtQuad> {
        match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.quads.iter().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

    /// Every occurrence of the find bar's string on the page being shown.
    ///
    /// Asked on every repaint, because `Query::Find` reads a readback that is already there.
    /// `Command::Find` is the other question and the expensive one — see `find`.
    pub(crate) fn matches(&self) -> Vec<QtQuad> {
        if self.needle.is_empty() {
            return Vec::new();
        }
        match self.viewer.query(Query::Find(&self.needle)) {
            Answer::Found(occurrences) => occurrences.iter().flatten().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

    /// ISO 32000-2 Annex O's `highlight`: the rectangles the URI's fragment named.
    ///
    /// Table Annex O.4: "Open the document with the specified rectangle highlighted … [t]he nature
    /// of the highlighting is implementation-dependent." So the shapes cross and the colour is
    /// Qt's, which is the whole argument for chrome crossing as geometry. Empty for every document
    /// opened without a fragment naming one.
    pub(crate) fn highlights(&self) -> Vec<QtQuad> {
        match self.viewer.query(Query::Highlight) {
            Answer::Highlighted(quads) => quads.iter().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

    /// A step of the search reported. Says what it found, and how much is left to read.
    fn searched(&mut self, found: Option<viewer_core::Found>, remaining: usize, wrapped: bool) {
        self.pages_left = remaining;
        match found {
            Some(found) => self.say(&format!(
                "found {:?} on page {}{}",
                self.needle,
                found.page.saturating_add(1),
                if wrapped { " (wrapped)" } else { "" }
            )),
            None if remaining == 0 => {
                self.say(&format!("{:?} is not in this document", self.needle));
            }
            None => self.say(&format!(
                "searching for {:?}: {remaining} page(s) left",
                self.needle
            )),
        }
    }

    /// The find bar's string changed: highlight this page, and look no further.
    ///
    /// Deliberately not a search per keystroke — see `viewer_gtk`'s `retype`, which makes the same
    /// choice for the same reason and is the agreement between the two hosts that
    /// `doc/ui-boundary.md` is about.
    pub(crate) fn retype(&mut self, needle: &str) {
        needle.clone_into(&mut self.needle);
        self.update.chrome = true;
    }

    /// The next occurrence anywhere in the document, or the previous one.
    pub(crate) fn find(&mut self, backward: bool) {
        if self.needle.is_empty() {
            return;
        }
        let needle = self.needle.clone();
        self.dispatch(Command::Find(Find::Start {
            needle,
            direction: if backward {
                FindDirection::Backward
            } else {
                FindDirection::Forward
            },
        }));
    }

    /// One more page of the search in progress. The C++ calls this from a zero-delay timer.
    pub(crate) fn find_continue(&mut self) {
        self.dispatch(Command::Find(Find::Continue));
    }

    /// The find bar was closed: forget the plan and the highlights.
    pub(crate) fn find_stop(&mut self) {
        self.needle.clear();
        self.pages_left = 0;
        self.dispatch(Command::Find(Find::Stop));
        self.update.chrome = true;
    }

    /// Whether a search still has pages to read, which is what the C++ pumps on.
    pub(crate) fn searching(&self) -> bool {
        self.pages_left > 0
    }

    /// §12.5.1's focus ring: one quadrilateral, or none.
    pub(crate) fn focus(&self) -> Vec<QtQuad> {
        match self.viewer.query(Query::Focus) {
            Answer::Focus { quad: ring, .. } => vec![quad(ring)],
            _ => Vec::new(),
        }
    }

    /// §12.5.6.14's open popup windows, as the furniture Qt places.
    ///
    /// The clause gives a popup "no appearance stream or associated actions of its own", so there
    /// is nothing of it in the page's pixels and a window is the platform's to draw. The three
    /// texts and the box are `viewer_host::popup`'s, so this host and `viewer-gtk` say the same
    /// thing about one clause.
    pub(crate) fn popups(&self) -> Vec<QtPopup> {
        let placed = viewer_host::popup::windows(&self.popups_shown);
        placed
            .iter()
            .map(|window| {
                let (x, y, width, height) = window.place;
                let colour = window
                    .colour
                    .map(|colour| (level(colour.r), level(colour.g), level(colour.b)));
                QtPopup {
                    x,
                    y,
                    width,
                    height,
                    title: window.title.to_owned(),
                    modified: viewer_host::popup::modified(window).unwrap_or_default(),
                    text: window.text.to_owned(),
                    coloured: colour.is_some(),
                    red: colour.map_or(0, |rgb| rgb.0),
                    green: colour.map_or(0, |rgb| rgb.1),
                    blue: colour.map_or(0, |rgb| rgb.2),
                }
            })
            .collect()
    }

    /// Whether the pointer is over §12.5.6.5's activation region.
    pub(crate) fn over_link(&self) -> bool {
        self.over_link
    }

    /// What the title bar should say.
    pub(crate) fn title(&self) -> String {
        let mark = if self.dirty { "• " } else { "" };
        if self.caption.is_empty() {
            return format!("{mark}{}", named(&self.path));
        }
        format!("{mark}{} — {}", named(&self.path), self.caption)
    }

    /// The most recent sentence for a person.
    pub(crate) fn status(&self) -> String {
        self.message.clone()
    }

    /// The window's initial size in logical pixels.
    ///
    /// A method on the host rather than a constant in the C++, because the two hosts asking for
    /// the same window is the thing that makes their launch numbers comparable, and a number
    /// written down twice is a number that stops agreeing.
    #[expect(
        clippy::unused_self,
        reason = "`cxx` carries methods on the opaque type; a free function would need a second \
                  entry in the bridge for a value that belongs to this host"
    )]
    pub(crate) fn window_size(&self) -> Vec<i32> {
        vec![WINDOW.0, WINDOW.1]
    }

    /// What the page area shows where no page lies: `pdf_render::SURROUND`, as three bytes.
    ///
    /// **Not §11.4.7's 𝑊.** The page's own colour is the standard's, is white, and is composited
    /// by the rasteriser inside §14.11.2.1's crop box; this is the ground a window lays the pages
    /// on, which no clause of ISO 32000-2 discusses — `pdf_render::medium` has the search that
    /// establishes the silence and the argument for the value. Read from there rather than
    /// restated here, for the same reason `window_size` is a method: a number written down twice
    /// is a number that stops agreeing, and this one has to agree with `viewer-gtk`, with
    /// `viewer-ui` and with all three rasterisers.
    #[expect(
        clippy::unused_self,
        reason = "`cxx` carries methods on the opaque type; a free function would need a second \
                  entry in the bridge for a value that belongs to this host"
    )]
    pub(crate) fn surround(&self) -> Vec<u8> {
        let colour = pdf_render::SURROUND;
        vec![level(colour.r), level(colour.g), level(colour.b)]
    }

    /// The C++ side finished putting a frame on the screen, with what the copy cost.
    pub(crate) fn painted(&mut self, bytes: usize, nanos: u64) {
        self.trace.say(
            Topic::Frames,
            format_args!(
                "{bytes} bytes into a QImage in {:?}",
                std::time::Duration::from_nanos(nanos)
            ),
        );
        if !self.presented {
            self.presented = true;
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "first frame on the screen at {:?}",
                    self.trace.since_start()
                ),
            );
        }
        // After the first frame rather than before it, and after every one after that: §14.7's
        // tree on AT-SPI. This is the one moment in this host that is on the far side of a paint,
        // which is exactly what `CLAUDE.md`'s startup rule asks of it (ADR 0623).
        self.attend();
    }

    /// What to call the document, which is the file's own name.
    pub(crate) fn named(&self) -> String {
        named(&self.path)
    }

    /// What the title bar says about the page, after the document's name.
    pub(crate) fn caption(&self) -> &str {
        &self.caption
    }

    /// One sentence from the C++ side.
    pub(crate) fn note(&self, what: &str) {
        self.trace.say(Topic::Panel, format_args!("{what}"));
    }

    // ---------------------------------------------------------------------------------------
    // The loop, which is the same one every host on this boundary runs.
    // ---------------------------------------------------------------------------------------

    /// §7.6.4.1: opens the document, with a password where one has been supplied.
    fn open_document(&mut self, password: Option<viewer_core::Secret>) {
        let bytes = self.bytes.clone();
        let fragment = self.fragment.clone();
        // Both policy values go before the document, and for one reason: a policy applied halfway
        // through is not a policy. §6.3.2.2's instruction is a property of *this host* rather than
        // of the document (ADR 0245), and `Restrict` is the reader's answer to what the *file*
        // asserts, which `CLAUDE.md` says is always the reader's to give.
        self.pump(vec![
            Command::Restrict(self.restrictions),
            Command::Delegate(self.widget_appearances),
            Command::Open {
                id: DOCUMENT,
                bytes,
                password,
                fragment,
            },
        ]);
    }

    /// One command, and everything it produces.
    pub(crate) fn dispatch(&mut self, command: Command) {
        self.pump(vec![command]);
    }

    /// Runs commands until nothing is left, reacting to what each produces, then refreshes.
    ///
    /// **The outer loop is the drawing thread's** (ADR 0668). A page that finished while the
    /// commands were being run answers the viewer with a `RenderReady`, which produces more events
    /// — so the queue is drained again rather than the answer waiting for the next turn of the
    /// timer.
    fn pump(&mut self, queue: Vec<Command>) {
        let mut queue: std::collections::VecDeque<Command> = queue.into();
        loop {
            while let Some(command) = queue.pop_front() {
                let described = self
                    .trace
                    .on(Topic::Events)
                    .then(|| format!("{command:?}"))
                    .map(|text| text.chars().take(120).collect::<String>());
                // **A command that changes the document changes what §14.7's tree says**, and
                // `Showing` cannot see it: an edit and a click move neither the page nor the
                // viewport. Which commands those are is one statement for all three windows
                // (`viewer_accessibility::republishes`, ADR 0623).
                if viewer_accessibility::republishes(&command) {
                    self.spoken = None;
                }
                let events: Vec<Event> = self.viewer.handle(command).collect();
                if let Some(described) = described {
                    self.trace.say(
                        Topic::Events,
                        format_args!("{described} -> {} event(s)", events.len()),
                    );
                }
                for event in events {
                    self.react(event, &mut queue);
                }
            }
            self.take_the_thread_back();
            self.take_the_drawn(&mut queue);
            if queue.is_empty() {
                break;
            }
        }
        self.refresh();
    }

    /// ADR 0668's second half: takes the drawing thread back from a page the arrangement has
    /// stopped showing.
    ///
    /// The *rule* is `viewer_host::Drawing`'s and the same in both native windows; what this
    /// supplies is the one fact that crate cannot ask for itself, because it holds no viewer: a
    /// page the arrangement does not show has no place on the screen, so `Query::PageGeometry`
    /// answers `Answer::None` for it.
    fn take_the_thread_back(&mut self) {
        let Some(page) = self.drawing.inside() else {
            return;
        };
        let shown = matches!(
            self.viewer.query(Query::PageGeometry(page)),
            Answer::Geometry(_)
        );
        if self.drawing.superseded(shown) {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} left the arrangement while it was being drawn, so the draw was \
                     abandoned",
                    page.saturating_add(1)
                ),
            );
        }
    }

    /// Takes whatever the drawing thread has finished and answers the viewer for it.
    ///
    /// **A `Finished` whose outcome is `None` is answered to nobody**, which is trap 20 and is why
    /// the type carries an `Option` rather than a `Rendered`: `Rendered::Failed` would record the
    /// page as answered for and stop the scheduler ever asking again, which freezes a page this
    /// window merely chose not to finish drawing.
    ///
    /// **Before the first frame this waits, and after it this only asks** — ADR 0678, and the same
    /// three lines as `viewer-gtk`'s because the rule is `viewer_host::Drawing`'s rather than a
    /// toolkit's. A window with nothing on the screen has no frame to spoil and no input to lose,
    /// and `viewer_host::drawing::SETTLE` bounds the whole of what it may spend. The measurement
    /// that put it there is GTK's — Qt's own launch was not the one that lost 44 ms — and it is
    /// here for levelness and because a `QTimer` is no more dispatchable than a `glib` one while a
    /// toolkit is inside its own first frame.
    fn take_the_drawn(&mut self, queue: &mut std::collections::VecDeque<Command>) {
        let drawn = if self.presented {
            self.drawing.collect()
        } else {
            self.drawing.settle(viewer_host::drawing::SETTLE)
        };
        for finished in drawn {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} {} {}x{} in {:?}, waited {:?}",
                    finished.request.page.saturating_add(1),
                    if finished.outcome.is_some() {
                        "rasterised"
                    } else {
                        "abandoned after"
                    },
                    finished.request.target.width,
                    finished.request.target.height,
                    finished.cost,
                    finished.waited
                ),
            );
            let Some(rendered) = finished.outcome else {
                continue;
            };
            queue.push_back(Command::RenderReady {
                token: finished.request.token,
                rendered,
            });
            // §12.4.4.1: the page a transition moves *to* is the one whose list has just arrived,
            // so this is where an armed one can begin. Only while a presentation is running,
            // because taking the face costs a whole-viewport rasterisation.
            self.face_arrived(&finished.request);
        }
        self.mind_a_long_draw();
    }

    /// Tells the person about a draw that has outlasted `viewer_host::drawing::WARN`, and takes
    /// the sentence back when it ends.
    ///
    /// **The *warn* half of the owner's "warn the user and allow the user to abort, however don't
    /// block"**, on the poll this window already runs while something is being drawn — so nothing
    /// new wakes for it. The decision, the duration and the wording are `viewer-host`'s and the
    /// three lines are `viewer-gtk`'s; what is Qt's is the status bar.
    ///
    /// It says something only when the answer *changes*: the poll runs every millisecond, and a
    /// status bar rewritten a thousand times a second is one nobody can read.
    fn mind_a_long_draw(&mut self) {
        let overlong = self.drawing.overlong();
        if overlong == self.warned {
            return;
        }
        let was = self.warned.take();
        self.warned = overlong;
        match (was, overlong) {
            (_, Some(page)) => self.say(&viewer_host::still_drawing(Some(page))),
            // A page that finished on its own: the sentence offered a key it no longer has a job
            // for, so it is withdrawn rather than left standing.
            (Some(page), None) => self.say(&viewer_host::drew_after_all(Some(page))),
            (None, None) => {}
        }
    }

    /// Whether this window has offered the key that stops a draw — `viewer_host::Waiting`.
    ///
    /// Read off [`Host::warned`] rather than asked of the drawing again: what decides Escape's
    /// meaning is whether the window has *said* something, not whether a clock has passed.
    fn waiting(&self) -> viewer_host::Waiting {
        if self.warned.is_some() {
            viewer_host::Waiting::Warned
        } else {
            viewer_host::Waiting::Nothing
        }
    }

    /// Stops the draw the sentence above offered to stop — `viewer_host::WindowAct::AbortDrawing`.
    ///
    /// Nothing is reported to the viewer for it (trap 20), so the page keeps whatever it was
    /// showing and is drawn again the next time the view changes, which is what the sentence says.
    fn stop_the_long_draw(&mut self) {
        let stopped = self.drawing.abandon();
        self.warned = None;
        if let Some(page) = stopped {
            self.say(&viewer_host::stopped_drawing(Some(page)));
        }
    }

    /// How long the window should wait before asking the drawing thread again, in milliseconds.
    ///
    /// `-1` where nothing is being drawn, which is what stops the timer — so a window showing a
    /// drawn page wakes for this exactly never. [`Host::presentation_wait`]'s shape and the same
    /// argument: the interval is `viewer_host::Drawing`'s decision, shared with the other native
    /// host, and the timer is Qt's.
    pub(crate) fn drawing_wait(&self) -> i32 {
        self.drawing.interval().map_or(-1, |interval| {
            i32::try_from(interval.as_millis()).unwrap_or(i32::MAX)
        })
    }

    /// One look at the drawing thread: answer the viewer for whatever landed, and draw it.
    ///
    /// **A look that finds nothing flags no update at all**, which is [`Host::presentation_tick`]'s
    /// own answer to the same question: a page being drawn is asked about a thousand times a
    /// second, and rebuilding this window's `QImage` at that rate would copy megabytes for a
    /// picture that has not changed.
    pub(crate) fn drawing_pump(&mut self) {
        let mut queue = std::collections::VecDeque::new();
        self.take_the_drawn(&mut queue);
        if queue.is_empty() {
            return;
        }
        self.pump(queue.into());
    }

    /// Does what one event asks.
    fn react(&mut self, event: Event, queue: &mut std::collections::VecDeque<Command>) {
        match event {
            Event::Opened { pages, .. } => {
                self.trace
                    .say(Topic::Launch, format_args!("opened, {pages} page(s)"));
                // Trap 5, and the same sentence the other two hosts say — see `viewer-gtk`.
                if pages == 0 {
                    self.say(&viewer_host::no_pages(&named(&self.path)));
                }
                self.asking.opened();
                self.obey_the_catalog(queue);
                self.build_panels();
            }
            Event::OpenFailed { reason, .. } => {
                self.say(&viewer_host::cannot_open(&named(&self.path), &reason));
            }
            // §7.6.4.1: "the interactive PDF processor should prompt for a password". The prompt
            // is a window, and a window is a host's — which is the whole reason this event exists
            // rather than a refusal. How many times to ask is `viewer_host::password`'s, because
            // the clause states no number and three hosts held three copies of the same three.
            //
            // Exhaustive over `Ask` on purpose: a case added there fails to compile here.
            Event::PasswordRequired { .. } => match self.asking.required() {
                viewer_host::Ask::Prompt { attempt, of } => {
                    let words = viewer_host::password::prompt(&named(&self.path), attempt, of);
                    // One `QLabel` for both, which is this toolkit's arrangement of the same two
                    // sentences `viewer-gtk` puts in two labels and the card draws in two colours.
                    self.prompt = format!("{}\n{}", words.question, words.counted);
                    self.update.password = true;
                }
                viewer_host::Ask::Exhausted => self.say(viewer_host::password::EXHAUSTED),
            },
            // Two events with nothing to do here, for two different reasons. A close drops
            // everything derived from the document and this host opens one document and never
            // closes it; damage is what a tier-1 host repaints from `Query::Frame`, and the
            // refresh at the end of every pump has already been scheduled by the time this
            // arrives.
            Event::Closed(_) | Event::Damage(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => self.turned(index, label.as_deref(), of, section.as_deref()),
            // **Handed to the drawing thread rather than drawn here** (ADR 0668). Until the
            // seven-hundred-and-fifty-fourth session this arm called `rasterize` inside
            // `QApplication::exec`, so a page written to draw for 27.6 s took the window with it —
            // no repaint, no key, and no thread from which `pdf_render::Interrupt` could be raised.
            // What comes back arrives in `take_the_drawn`.
            Event::NeedsRender(request) => self.drawing.ask(request),
            // §12.6.4.8: handed over rather than opened. The string is one the *document*
            // controls, and giving it to a browser is a decision about this machine that this
            // host has not been given — the same answer `viewer-ui` and `viewer-gtk` give.
            Event::OpenUri { uri, .. } => self.say(&format!("link: {uri}")),
            Event::NeedsFile { purpose, name, .. } => {
                let bytes = match viewer_host::policy::read_import(self.directory.as_deref(), &name)
                {
                    Ok(bytes) => Some(bytes),
                    Err(refusal) => {
                        self.say(&format!("import-data: declined — {refusal}"));
                        None
                    }
                };
                queue.push_back(Command::Supply { purpose, bytes });
            }
            // §12.4.4.1: played since this host was given a clock, and named where it is not.
            //
            // A transition outside a presentation is not drawn at all — there is no clock to draw
            // it on — and a style `viewer_core::transition` does not shape is refused before two
            // pages are rasterised for it. Both are said rather than swallowed.
            Event::Transition { transition, .. } => self.arm_transition(transition),
            Event::Extracted {
                asked, name, bytes, ..
            } => self.write_extracted(asked, &name, &bytes),
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            Event::Dirty { dirty, .. } => {
                self.dirty = dirty;
                self.update.title = true;
            }
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => self.searched(found, remaining, wrapped),
            Event::Reported { page, notes, .. } => self.reported(page, &notes),
            // `CLAUDE.md`: a document's restrictions are the reader's to set, and it shall always
            // be possible to turn them off.
            // `CLAUDE.md`: a document's restrictions are the reader's to set, and it shall always
            // be possible to turn them off. The sentence is `viewer_host::refused` rather than
            // this host's own because it names the word the argument parser takes, and this host
            // wrote its own copy of it for sessions while taking no such word (ADR 0604).
            Event::Refused { notes, .. } => self.say(&viewer_host::refused(&notes)),
            // The other two of `CLAUDE.md`'s four levels, since the eight-hundred-and-eighty-fifth
            // session (ADR 0814). *Warn* is a sentence after an edit that went ahead. *Ask* is a
            // question this window has no dialogue for yet — the gestures follow the owner's
            // mockups (`doc/todo/38`) — so it answers no, out loud, rather than letting the level
            // behave like *on* in silence; `viewer_host::unanswerable` is the sentence.
            Event::Warned { notes, .. } => self.say(&viewer_host::warned(&notes)),
            Event::Asking {
                document, notes, ..
            } => {
                self.say(&viewer_host::unanswerable(&notes));
                queue.push_back(Command::Answer {
                    document,
                    proceed: false,
                });
            }
            // §7.11.4's list moved under the files tab: rebuilt from the same answer it was
            // built from, which is the only thing a window may do here this round — display
            // the list it already shows.
            Event::AttachmentsChanged { .. } => self.build_panels(),
        }
    }

    /// Recomputes what the window shows, and says what of it changed.
    ///
    /// Idempotent and called after every pump, which is what keeps the window a function of the
    /// viewer's state rather than of the order events arrived in. ADR 0244 finding 5 records why
    /// both native hosts make that choice: `Event::Damage` covers a moved page mapping in
    /// practice but is documented as "a bound on what changed", so a host that placed controls
    /// from it would be relying on a coincidence.
    fn refresh(&mut self) {
        self.update.frame = true;
        self.update.chrome = true;
        let fields = match self.viewer.query(Query::Fields) {
            Answer::Fields(fields) => fields,
            _ => Vec::new(),
        };
        let placed: Vec<Placement> = fields
            .iter()
            .flat_map(|field| {
                let kind = control_kind(&field.control);
                field
                    .widgets
                    .iter()
                    .filter_map(move |widget| placement(field, widget, &kind))
            })
            .collect();
        let keys: Vec<(String, pdf_syntax::ObjectId)> = placed.iter().map(Placement::key).collect();
        let was: Vec<(String, pdf_syntax::ObjectId)> =
            self.placed.iter().map(Placement::key).collect();
        if keys != was {
            self.update.controls = true;
            self.trace.say(
                Topic::Panel,
                format_args!(
                    "{} field(s) on the page, {} control(s) placed",
                    fields.len(),
                    placed.len()
                ),
            );
        }
        self.fields = fields;
        self.placed = placed;
        // §12.5.6.14's windows, which are widgets and not pixels: rebuilt when the set changes or
        // one of them moves, and left alone otherwise. Reported on what the *answer* held rather
        // than on what could be placed, so that a window with no area is a line rather than a
        // silence.
        let popups = match self.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows,
            _ => Vec::new(),
        };
        if popups != self.popups_shown {
            self.update.popups = true;
            if !popups.is_empty() || !self.popups_shown.is_empty() {
                self.trace.say(
                    Topic::Panel,
                    format_args!(
                        "{} of {} §12.5.6.14 popup window(s) placed",
                        viewer_host::popup::windows(&popups).len(),
                        popups.len()
                    ),
                );
            }
            self.popups_shown = popups;
        }
    }

    /// The rows of one panel, or none at all for the one panel that has no rows.
    ///
    /// **The match is exhaustive over [`viewer_host::Tab`] on purpose**, and it is this host's half
    /// of `doc/todo/30`'s "all three hosts stay level": a panel added to `viewer_host::Tab` fails
    /// to compile here, in `viewer-gtk` and in `viewer-ui` until each supplies a widget for it. It
    /// is `viewer_host::Key`'s mechanism applied to the other thing a window shows (ADR 0526).
    fn panel_of(&self, tab: Tab) -> Vec<PanelRow> {
        use viewer_host::panel::{
            article_rows, attachment_rows, collection_rows, layer_rows, outline_rows, property_rows,
        };
        match tab {
            Tab::Contents => match self.viewer.query(Query::Outline) {
                Answer::Outline(outline) if !outline.items.is_empty() => outline_rows(&outline),
                _ => vec![PanelRow::saying("This document states no outline.")],
            },
            // §12.3.4 is a page count and a picture per row rather than a tree; the C++ side asks
            // `page_count` and `page_row` for it, one row at a time.
            Tab::Pages => Vec::new(),
            Tab::Layers => match self.viewer.query(Query::Layers) {
                Answer::Layers(layers) if !layers.is_empty() => layer_rows(&layers),
                _ => vec![PanelRow::saying(
                    "This document states no optional content.",
                )],
            },
            // §12.3.5: "[i]f this dictionary is present in a PDF document, the interactive PDF
            // processor shall present the document as a portable collection." The collection is
            // asked for first because it decides how this same list is shown — §12.3.5.2's folder
            // tree and Table 155's columns where a document states one, the `/EmbeddedFiles`
            // tree's own order where it does not. Both mappings are `viewer_host::panel`'s, so
            // this window and `viewer-gtk` present a collection identically.
            Tab::Files => {
                let files = match self.viewer.query(Query::Attachments) {
                    Answer::Attachments(files) => files,
                    _ => Vec::new(),
                };
                match self.viewer.query(Query::Collection) {
                    Answer::Collection {
                        collection,
                        initial,
                    } => collection_rows(&collection, &initial, &files),
                    _ if files.is_empty() => {
                        vec![PanelRow::saying("This document embeds no files.")]
                    }
                    _ => attachment_rows(&files),
                }
            }
            Tab::Articles => match self.viewer.query(Query::Articles) {
                Answer::Articles(threads) => article_rows(&threads),
                _ => article_rows(&[]),
            },
            Tab::Document => match self.viewer.query(Query::Properties) {
                Answer::Properties {
                    information,
                    metadata,
                } => property_rows(&information, metadata.as_ref()),
                _ => property_rows(&pdf_model::metadata::Information::default(), None),
            },
        }
    }

    /// Builds every panel that has rows from its own answer.
    fn build_panels(&mut self) {
        self.trees = std::array::from_fn(|index| {
            Tab::at(index)
                .map(|tab| flatten(&self.panel_of(tab)))
                .unwrap_or_default()
        });
        for (tab, rows) in Tab::ALL.iter().zip(&self.trees) {
            self.trace.say(
                Topic::Panel,
                format_args!("{}: {} row(s)", tab.label(), rows.len()),
            );
        }
        self.update.panels = true;
    }

    /// §7.5.6's update, written beside the document rather than over it.
    fn write_saved(&mut self, bytes: &[u8]) {
        let path = self.path.with_extension("edited.pdf");
        match std::fs::write(&path, bytes) {
            Ok(()) => self.say(&format!(
                "saved {} bytes to {}",
                bytes.len(),
                path.display()
            )),
            Err(error) => self.say(&format!("cannot write {}: {error}", path.display())),
        }
    }

    /// §7.11.4's file, written beside the document.
    ///
    /// The name is the document's own words and not a path — §7.11.4's note is that `/F` is "a
    /// platform-dependent encoding" — so only its last component is used and only beside the
    /// document, which is the same policy §12.7.6.4's import takes in the other direction.
    fn write_extracted(&mut self, asked: Extraction, name: &str, bytes: &[u8]) {
        // §O.2.1's own sentence, decided once for all three hosts in `viewer_host::policy`: a URI
        // that named a file is not a person who asked for one.
        if let Err(refusal) = viewer_host::may_write_extracted(asked) {
            self.say(&refusal);
            return;
        }
        let Some(directory) = self.directory.clone() else {
            self.say("cannot write the attachment: the document is not in a known directory");
            return;
        };
        let single = Path::new(name).file_name().unwrap_or(name.as_ref());
        let path = directory.join(single);
        match std::fs::write(&path, bytes) {
            Ok(()) => self.say(&format!(
                "wrote {} bytes to {}",
                bytes.len(),
                path.display()
            )),
            Err(error) => self.say(&format!("cannot write {}: {error}", path.display())),
        }
    }

    /// One sentence for a person, in the status bar and on standard output.
    ///
    /// Both, because the status bar is what a person sees and the terminal is what a test reads —
    /// and the `Xvfb` runs this project checks a window with read the terminal.
    fn say(&mut self, what: &str) {
        if what.is_empty() {
            return;
        }
        println!("note: {what}");
        what.clone_into(&mut self.message);
        self.update.status = true;
    }

    /// A page turn: the title bar, and what the pages now on the screen could not draw.
    ///
    /// §12.4.2: "[p]age labels and page indices need not coincide". Both are shown, because a
    /// label alone cannot say "of 320".
    fn turned(&mut self, index: usize, label: Option<&str>, of: usize, section: Option<&str>) {
        let page = match label {
            Some(label) => format!("{label} — page {} of {of}", index.saturating_add(1)),
            None => format!("page {} of {of}", index.saturating_add(1)),
        };
        self.caption = match section {
            Some(section) if !section.is_empty() => format!("{page} — {section}"),
            _ => page,
        };
        self.update.title = true;
        self.restate();
    }

    /// One page's refusals, in the status bar, with the page named.
    ///
    /// **The page is named because under a column it has to be**: `Event::Reported` has always
    /// carried which page it is about, and a host that dropped it was attributing one page's
    /// refusals to whichever page a person happened to be looking at. `None` is what the
    /// *document* says about itself before any page is drawn, and belongs to no page.
    fn reported(&mut self, page: Option<usize>, notes: &[String]) {
        let notes = notes.join("; ");
        match page {
            Some(page) => self.say(&format!("page {}: {notes}", page.saturating_add(1))),
            None => self.say(&notes),
        }
    }

    /// Says again what the pages now on the screen could not draw.
    ///
    /// **`Query::Reports` is where a host that cleared its status bar finds it again**, and under
    /// Table 29's continuous arrangements it is the only way to be right: the events came page by
    /// page as each was interpreted, and a scroll that brings a page back does not interpret it
    /// again. The wording is `viewer_host::status`'s, so that three hosts say one sentence.
    fn restate(&mut self) {
        let said = match self.viewer.query(Query::Reports) {
            Answer::Reports(pages) => viewer_host::status::on_screen(&pages),
            _ => return,
        };
        self.say(&said);
    }

    /// One tree's rows, where the number names one.
    fn tree(&self, tree: u8) -> Option<&Vec<Flat>> {
        self.trees.get(usize::from(tree))
    }

    /// One row of one tree, where both indices name one.
    fn flat(&self, tree: u8, index: usize) -> Option<&Flat> {
        self.tree(tree).and_then(|rows| rows.get(index))
    }

    /// Every widget a control was placed over, in the order the controls are in.
    fn widgets(&self) -> impl Iterator<Item = (&FormField, &viewer_core::FormWidget)> {
        self.fields.iter().flat_map(|field| {
            let kind = control_kind(&field.control);
            field
                .widgets
                .iter()
                .filter(move |widget| placement(field, widget, &kind).is_some())
                .map(move |widget| (field, widget))
        })
    }
}

/// The control one widget of one field becomes, or nothing where the clause gives it none.
///
/// §12.7.5.5's signature has no control to build and Table 226's absent `/FT` names none, so this
/// host places nothing and the page's own appearance stands. Inventing a control for either would
/// be a statement about the document that the document did not make.
fn placement(
    field: &FormField,
    widget: &viewer_core::FormWidget,
    kind: &ControlKind,
) -> Option<Placement> {
    match kind {
        ControlKind::Signature | ControlKind::Unstated => None,
        _ => Some(Placement {
            name: field.name.clone(),
            annotation: widget.annotation,
            kind: kind.clone(),
            on_state: widget.on_state.clone(),
            no_toggle_to_off: matches!(
                kind,
                ControlKind::Radio {
                    no_toggle_to_off: true,
                    ..
                }
            ),
            read_only: field.read_only,
        }),
    }
}

/// Which number the bridge carries for a control, and the three flags that go with it.
fn describe_kind(kind: &ControlKind) -> (u8, i32, bool, bool) {
    match kind {
        ControlKind::Entry {
            multiline,
            password,
            max_len,
        } => {
            // Table 231 bit 14 outranks bit 13: a field that is both is a password field, and a
            // password field is never multiline.
            let which = match (*password, *multiline) {
                (true, _) => 2,
                (false, true) => 1,
                (false, false) => 0,
            };
            let cap = max_len
                .and_then(|len| i32::try_from(len).ok())
                .unwrap_or(-1);
            (which, cap, false, false)
        }
        ControlKind::Check { .. } => (3, -1, false, false),
        ControlKind::Radio { .. } => (4, -1, false, false),
        ControlKind::Push => (5, -1, false, false),
        ControlKind::Combo { editable, .. } => (6, -1, false, *editable),
        ControlKind::List { multi, .. } => (7, -1, *multi, false),
        // Unreachable: `placement` returns `None` for both, so no control of either kind is ever
        // in `placed`. Answered rather than asserted away, because a refusal that cannot happen
        // still has to say something if it does.
        ControlKind::Signature | ControlKind::Unstated => (255, -1, false, false),
    }
}

/// Table 234's `/TI` for a list box, and 0 for every other control.
///
/// The clause states it of the list rather than of the value — "the index in the Opt array of the
/// first option visible in the list" — so it decides where a scrollable list *starts*, which is a
/// question about the control and not about what is selected in it. `pdf-model` has read the entry
/// since the three-hundred-and-ninety-eighth session and the page's own appearance obeys it
/// (ADR 0407); a host that started every list at row 0 showed a different first option than the
/// picture underneath it.
fn top_option(kind: &ControlKind) -> u32 {
    match kind {
        ControlKind::List { top, .. } => u32::try_from(*top).unwrap_or(0),
        _ => 0,
    }
}

/// Whether a control's on-ness comes from the field's value rather than the widget's state.
fn on_by_value(kind: &ControlKind) -> bool {
    match kind {
        ControlKind::Check { on } | ControlKind::Radio { on, .. } => *on,
        _ => false,
    }
}

/// Table 226's `/TU`, as the two hosts show it.
fn tooltip(field: &FormField) -> String {
    let shown = field.name.shown();
    if shown.is_empty() {
        return String::new();
    }
    if field.required {
        return format!("{shown} (required)");
    }
    shown.to_owned()
}

/// A tree of rows, depth first, with each row's depth beside it.
fn flatten(rows: &[PanelRow]) -> Vec<Flat> {
    let mut flat = Vec::new();
    push_rows(rows, 0, &mut flat);
    flat
}

/// Which place in [`Tab::ALL`] §12.3.4's panel is, which is the one the C++ builds a list at.
///
/// A function rather than a literal in `window.cpp` so that the order of the panels is stated once:
/// a panel inserted before §12.3.4's moves this number, and a window with it written down would put
/// a `QListView` where a `QTreeView` belongs.
fn pages_panel_index() -> u8 {
    u8::try_from(Tab::Pages.index()).unwrap_or(u8::MAX)
}

/// One level of the tree and everything under it.
fn push_rows(rows: &[PanelRow], depth: u32, into: &mut Vec<Flat>) {
    for row in rows {
        let (action, on, locked) = match &row.action {
            RowAction::Inert => (0, false, false),
            RowAction::Activate(_) => (1, false, false),
            RowAction::Toggle { on, locked, .. } => (2, *on, *locked),
            RowAction::Extract { .. } => (3, false, false),
        };
        into.push(Flat {
            row: QtRow {
                label: row.label.clone(),
                detail: row.detail.clone().unwrap_or_default(),
                depth,
                expanded: row.expanded,
                action,
                on,
                locked,
                note: row.note,
                emphasis: row.emphasis,
            },
            action: row.action.clone(),
        });
        push_rows(&row.children, depth.saturating_add(1), into);
    }
}

/// One `[x0, y0, … x3, y3]` as the bridge carries it.
fn quad(corners: [f32; 8]) -> QtQuad {
    QtQuad {
        x0: corners[0],
        y0: corners[1],
        x1: corners[2],
        y1: corners[3],
        x2: corners[4],
        y2: corners[5],
        x3: corners[6],
        y3: corners[7],
    }
}

/// No frame at all, which is what a viewer with nothing open answers.
fn no_frame() -> QtFrame {
    QtFrame {
        present: false,
        width: 0,
        height: 0,
        origin_x: 0.0,
        origin_y: 0.0,
    }
}

/// Every flag clear, which is what `take_update` leaves behind.
fn nothing_changed() -> QtUpdate {
    QtUpdate {
        frame: false,
        panels: false,
        controls: false,
        chrome: false,
        popups: false,
        cursor: false,
        title: false,
        status: false,
        password: false,
        window: false,
        clipboard: false,
        find_bar: false,
        notices: false,
    }
}

/// A colour component in `0.0..=1.0` as the 0..=255 level a `QColor` takes.
///
/// Shared by `pdf_render::SURROUND` and Table 166's `/C`, because two roundings of one convention
/// is exactly the kind of thing that comes to differ by a level and be argued about.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a colour component in 0..=1 scaled by 255"
)]
fn level(component: f32) -> u8 {
    (component.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// What to call the document in a title bar: its file name, or the whole path where it has none.
fn named(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::{Host, PANELS, describe_kind, flatten, pages_panel_index};
    use pdf_model::view::WidgetAppearances;
    use pdf_syntax::ObjectId;
    use std::path::{Path, PathBuf};
    use viewer_host::Tab;
    use viewer_host::form::ControlKind;
    use viewer_host::panel::{PanelRow, RowAction};
    use viewer_host::trace::Trace;

    /// Every panel the shared list states has a widget in this toolkit, and one is not a tree.
    ///
    /// **The instrument `doc/todo/30`'s "all three hosts stay level" never had for a *panel***, and
    /// the same shape as the key test in `crate::keys` (ADR 0526). The match is exhaustive over
    /// [`viewer_host::Tab`], so a panel added there fails to compile here — and in
    /// [`Host::panel_of`], which is where its rows have to come from. The runtime half is the
    /// *place*: `window.cpp` builds a `QListView` at whatever [`pages_panel_index`] answers and a
    /// `QTreeView` everywhere else, so a panel inserted before §12.3.4's in the list must move
    /// both, and this is what says they moved together.
    ///
    /// It needs no display: nothing here calls into Qt.
    #[test]
    fn every_panel_the_list_states_has_a_widget_in_this_toolkit() {
        let mut lists = 0;
        for (place, tab) in Tab::ALL.iter().enumerate() {
            // Exhaustive on purpose. A new panel lands here as a compile error.
            let tree = match tab {
                Tab::Contents | Tab::Layers | Tab::Files | Tab::Articles | Tab::Document => true,
                Tab::Pages => false,
            };
            if !tree {
                assert_eq!(
                    place,
                    usize::from(pages_panel_index()),
                    "the C++ builds its list of miniatures somewhere else"
                );
                lists += 1;
            }
        }
        assert_eq!(lists, 1, "exactly one panel is a list of pictures");
        assert_eq!(PANELS, Tab::ALL.len());
    }

    /// A document committed in `doc/`, which every checkout has once the archive is unpacked.
    fn committed() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf")
    }

    /// A corpus document, or `None` when the submodule is not checked out.
    fn corpus(name: &str) -> Option<PathBuf> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/pdf.js/test/pdfs")
            .join(name);
        path.exists().then_some(path)
    }

    /// A host with the document open at a viewport of a stated size, and no window anywhere.
    ///
    /// **This is the whole of what a workspace test suite can see of a Qt host**, and it is more
    /// than it could see of the GTK one: `Host` here is a plain struct C++ happens to own, so a
    /// test can build one, drive it and read its answers with no display and no `QApplication`.
    /// That is the ownership inversion paying a dividend rather than costing one (ADR 0246).
    fn opened(path: &Path) -> Host {
        opened_under(path, viewer_core::RestrictionLevel::On)
    }

    /// The same, with the reader's answer to what the document asserts stated explicitly.
    ///
    /// `CLAUDE.md`'s rule is that turning a document's restrictions off is always possible, and
    /// this is where a test can see whether the value the command line supplies reaches the core
    /// at all — no display, no `QApplication`, and a real file asserting a real `/P`.
    fn opened_under(path: &Path, restrictions: viewer_core::RestrictionLevel) -> Host {
        let mut host = Host::open(
            path,
            None,
            WidgetAppearances::Delegated,
            restrictions,
            Trace::off(std::time::Instant::now()),
        )
        .expect("the document is readable");
        host.resized(800, 1000, 1.0);
        drawn(&mut host);
        host
    }

    /// Waits for the page to come back from the drawing thread.
    ///
    /// **A test is a host with no event loop**, so it supplies what `MainWindow::pumpDrawing`
    /// supplies in a running window: the page is rasterised on a thread of its own since ADR 0668,
    /// and a test asking about a frame the moment `resized` returns would be asking before it had
    /// been drawn. Bounded rather than looping for ever, because a thread that never answers is a
    /// defect and not something to hang on.
    fn drawn(host: &mut Host) {
        let began = std::time::Instant::now();
        while host.drawing_wait() >= 0 {
            assert!(
                began.elapsed() < std::time::Duration::from_mins(2),
                "the drawing thread never answered"
            );
            std::thread::sleep(viewer_host::drawing::POLL);
            host.drawing_pump();
        }
    }

    /// §12.3.3's outline reaches the bridge flattened, in the order a tree shows it.
    #[test]
    fn the_outline_crosses_depth_first_with_a_depth_on_every_row() {
        let host = opened(&committed());
        let rows = host.rows(0);
        assert_eq!(rows.len(), 14, "the document's outline is 14 items");
        assert_eq!(rows[0].depth, 0, "the first row is top level");
        // Depth-first order means a row is never more than one deeper than the row above it,
        // which is exactly what lets the C++ side rebuild the parentage from a stack.
        for pair in rows.windows(2) {
            assert!(
                pair[1].depth <= pair[0].depth.saturating_add(1),
                "{:?} follows {:?}",
                pair[1],
                pair[0]
            );
        }
        // §12.3.3: "[i]f the outline item is open, Count is the sum of the number of visible
        // descendent outline items." So the flag belongs to an item that *has* descendants, and
        // every one of this document's does ask to be open — which is what makes all 14 rows
        // visible. A leaf carries `false` and is shown anyway, which is the distinction ADR
        // 0244's "every one of which asks to be open" ran together.
        for (above, below) in rows.iter().zip(rows.iter().skip(1)) {
            if below.depth > above.depth {
                assert!(above.expanded, "a parent this file opened: {above:?}");
            }
        }
        let open = rows.iter().filter(|row| row.expanded).count();
        let parents = rows
            .iter()
            .zip(rows.iter().skip(1))
            .filter(|(above, below)| below.depth > above.depth)
            .count();
        assert_eq!(
            open, parents,
            "every item with children is one this file opened"
        );
        // Every row activates an object, because an outline item is a destination or an action
        // and which of the two it is belongs to the document (ADR 0144).
        assert!(rows.iter().all(|row| row.action == 1));
    }

    /// The panels this document states nothing for say so rather than showing an empty list.
    ///
    /// Trap 5's shape for a panel, and **the assertion changed direction in the
    /// seven-hundred-and-fourth session**: these two used to answer no rows at all, and an empty
    /// list is indistinguishable from a list this program failed to fill. The sentence is
    /// `viewer_host::panel`'s, so all three hosts say the same one.
    #[test]
    fn a_document_with_no_layers_and_no_files_says_so_rather_than_showing_nothing() {
        let host = opened(&committed());
        for tab in [Tab::Layers, Tab::Files] {
            let rows = host.rows(index(tab));
            assert_eq!(rows.len(), 1, "{tab:?}");
            assert!(rows[0].note, "{tab:?} says nothing about being empty");
        }
        // §12.3.4's panel is the one with no rows at all: its pictures are `page_row`'s.
        assert!(host.rows(index(Tab::Pages)).is_empty());
        // And a panel number this host does not have answers nothing rather than the first one.
        assert!(host.rows(99).is_empty());
    }

    /// A panel's place in the shared list, as the C++ side spells one.
    fn index(tab: Tab) -> u8 {
        u8::try_from(tab.index()).expect("six panels fit in a byte")
    }

    /// §12.7's form becomes controls, and the count is the one both native hosts place.
    #[test]
    fn a_real_form_becomes_the_same_controls_the_other_host_places() {
        let Some(path) = corpus("160F-2019.pdf") else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        let host = opened(&path);
        let controls = host.controls();
        assert_eq!(
            controls.len(),
            76,
            "the same 76 controls `viewer-gtk` places over this form's 67 fields"
        );
        // Every one of them names the field an edit is addressed by and the widget that
        // distinguishes it from the field's other widgets.
        assert!(controls.iter().all(|control| !control.field.is_empty()));
        // And every kind is one the C++ side has a widget for: 8 kinds, 0 to 7.
        assert!(controls.iter().all(|control| control.kind < 8));
    }

    /// `CLAUDE.md`: a document's restrictions are the reader's, and turning them off must work.
    ///
    /// **The one test in this tree that drives the whole chain a person walks**: a real file
    /// asserting a real §7.6.4.2 `/P`, a real key press, the sentence the window puts on the
    /// screen, and the same run again with the command line's answer changed. Everything else
    /// about this is checked one link at a time — `viewer-core`'s headless harness has the
    /// refusal, `viewer-host` has the sentence, each binary has its parser — and the defect ADR
    /// 0604 found lived exactly between two of those links, where nothing looked.
    ///
    /// `issue17215.pdf` is one of the corpus's seven witnesses (ADR 0212): it opens with the empty
    /// user password and withholds both filling in a form and annotating.
    #[test]
    fn a_restricted_document_refuses_and_the_reader_can_turn_that_off() {
        let Some(path) = corpus("issue17215.pdf") else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        // `a` selects the page and `h` is §12.5.6.10's highlight, which is an annotation and is
        // what this document's `/P` withholds.
        let mut host = opened_under(&path, viewer_core::RestrictionLevel::On);
        host.key(0x41, false);
        host.key(0x48, false);
        let said = host.status();
        assert!(
            said.contains(viewer_host::IGNORE_RESTRICTIONS),
            "a refusal names the way out: {said}"
        );

        let mut host = opened_under(&path, viewer_core::RestrictionLevel::Off);
        host.key(0x41, false);
        host.key(0x48, false);
        let said = host.status();
        assert!(
            !said.contains(viewer_host::IGNORE_RESTRICTIONS),
            "the reader said not to obey it, so nothing is refused: {said}"
        );
        assert!(host.dirty, "and the annotation is in the edit log: {said}");
    }

    /// §12.7.5.4: what a `QListWidget` in `ExtendedSelection` has to say, and where it goes.
    ///
    /// The C++ collects the selected rows ascending and hands them over as a slice; this is the
    /// Rust half of that, driven without Qt because ADR 0246's rule is that no Qt type appears in
    /// this file. Table 233 bit 22 is what makes the mode `ExtendedSelection` at all (ADR 0248).
    #[test]
    fn several_items_of_a_list_box_are_chosen_and_read_back() {
        let Some(path) = corpus("issue17492.pdf") else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        let mut host = opened(&path);
        let Some(index) = host
            .controls()
            .iter()
            .position(|control| control.field == "databases")
        else {
            panic!("Table 233 bit 22's list box has a control")
        };
        // 7 is the list box, and `multi` is the flag the C++ turns into a selection mode.
        assert_eq!(host.controls()[index].kind, 7);
        assert!(host.controls()[index].multi, "Table 233 bit 22");
        assert_eq!(host.control_options(index).len(), 4);

        host.choose_control(index, &[0, 2]);
        assert_eq!(
            host.control_selection(index),
            vec![0_u32, 2],
            "both, which is what a single value could not have said"
        );
        assert!(host.dirty, "and the document knows it was edited");
    }

    /// A frame the viewer is holding crosses as its own dimensions and a borrowed slice.
    #[test]
    fn the_frame_and_its_pixels_agree_about_how_many_bytes_there_are() {
        let host = opened(&committed());
        let frame = host.frame(0);
        assert!(frame.present, "a page has been rasterised");
        let need = usize::try_from(frame.width).unwrap_or(0)
            * usize::try_from(frame.height).unwrap_or(0)
            * 4;
        assert!(
            host.frame_pixels(0).len() >= need,
            "{} bytes for {}x{}",
            host.frame_pixels(0).len(),
            frame.width,
            frame.height
        );
    }

    /// The update flags clear when they are taken, so nothing is rebuilt twice.
    #[test]
    fn taking_the_update_leaves_nothing_behind() {
        let mut host = opened(&committed());
        let first = host.take_update();
        assert!(first.frame && first.panels, "opening changes both");
        let second = host.take_update();
        assert!(!second.frame && !second.panels && !second.controls);
    }

    /// The flattening is depth first and carries §8.11.4.3's switch state with it.
    #[test]
    fn a_tree_of_rows_flattens_to_the_order_a_tree_shows() {
        let rows = vec![
            PanelRow {
                label: "top".to_owned(),
                detail: None,
                expanded: true,
                action: RowAction::Toggle {
                    group: ObjectId {
                        number: 7,
                        generation: 0,
                    },
                    on: true,
                    locked: true,
                },
                note: false,
                emphasis: false,
                children: vec![PanelRow {
                    label: "under".to_owned(),
                    detail: Some("said".to_owned()),
                    expanded: false,
                    action: RowAction::Inert,
                    note: false,
                    emphasis: false,
                    children: Vec::new(),
                }],
            },
            PanelRow {
                label: "beside".to_owned(),
                detail: None,
                expanded: false,
                action: RowAction::Extract {
                    name: "a.txt".to_owned(),
                },
                note: false,
                // §12.3.5.1's `/D` crosses the bridge like every other row flag, and this one
                // carries it so that the bold row is the one the *document* named.
                emphasis: true,
                children: Vec::new(),
            },
        ];
        let flat = flatten(&rows);
        let shape: Vec<(&str, u32, u8)> = flat
            .iter()
            .map(|one| (one.row.label.as_str(), one.row.depth, one.row.action))
            .collect();
        assert_eq!(
            shape,
            vec![("top", 0, 2), ("under", 1, 0), ("beside", 0, 3)]
        );
        // Table 99's `/Locked` crosses beside the state rather than instead of it: the switch is
        // shown as the document set it and is not a person's to move.
        assert!(flat[0].row.on && flat[0].row.locked);
        assert_eq!(flat[1].row.detail, "said");
        // §12.3.5.1's mark travels with the row it is on and with no other.
        assert_eq!(
            flat.iter().map(|one| one.row.emphasis).collect::<Vec<_>>(),
            vec![false, false, true]
        );
    }

    /// Every control kind has a number, and the two the clause gives no control to have none.
    #[test]
    fn every_control_kind_crosses_as_a_number_the_window_knows() {
        assert_eq!(
            describe_kind(&ControlKind::Entry {
                multiline: false,
                password: true,
                max_len: Some(8),
            }),
            (2, 8, false, false)
        );
        assert_eq!(
            describe_kind(&ControlKind::Entry {
                multiline: true,
                password: false,
                max_len: None,
            }),
            (1, -1, false, false)
        );
        assert_eq!(describe_kind(&ControlKind::Push), (5, -1, false, false));
        // §12.7.5.5's signature and Table 226's absent `/FT` are never placed, so their number is
        // one the window refuses by name rather than one it draws.
        assert_eq!(describe_kind(&ControlKind::Signature).0, 255);
        assert_eq!(describe_kind(&ControlKind::Unstated).0, 255);
    }
}
