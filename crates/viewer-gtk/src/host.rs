//! The window, and everything that crosses [`viewer_core`]'s boundary to fill it.
//!
//! One `Viewer`, one `render-cpu` rasteriser, and GTK4 widgets on the other side. The loop is the
//! one `viewer-ui` and `tests/headless.rs` already run — a command in, the events it produced out,
//! and the ones asking for work answered until nothing is left — because the vocabulary is what is
//! being tested and a host that invented its own scheduling would be testing that instead.
//!
//! # Where the pixels go
//!
//! A [`gtk4::Overlay`] whose main child is a [`gtk4::Fixed`]. The page is a [`gtk4::Picture`]
//! inside the `Fixed`, moved to the origin [`viewer_core::Query::Frame`] reports; §12.7's
//! controls are `Fixed` children at their widgets' own rectangles. On top of both is a
//! [`gtk4::DrawingArea`] that cannot be targeted by input and draws nothing but chrome —
//! the selection, the focus ring — in the platform's own colour.
//!
//! That third layer is also where the viewport's *size* comes from: `GtkDrawingArea::resize` is
//! the only size signal GTK4 gives application code without subclassing a widget, and subclassing
//! is a larger thing to take on than one signal is worth.
//!
//! **This sentence used to end "and `#![forbid(unsafe_code)]` is what makes subclassing the wrong
//! answer here", and that half was false** — checked in the seven-hundred-and-thirty-first session
//! by writing the subclass rather than by reading the attribute (ADR 0623). `#[glib::object_subclass]`
//! expands to `unsafe impl` and `unsafe` blocks, but the `unsafe_code` lint does not fire on a
//! proc-macro's expansion, so a `GObject` implementing `gtk4::Accessible` compiles in this crate
//! today with the `forbid` untouched. The reason not to subclass is a judgement about cost, which
//! is a different kind of claim from a compiler-enforced impossibility — and stating the second
//! where only the first is true is how a floor gets written down that nobody re-checks (trap 17,
//! and ADR 0508's rule paying a third time).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use gtk4::glib;
use gtk4::prelude::*;
use pdf_model::view::WidgetAppearances;
use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Edit, Entered, Event, Extraction, Find, FindDirection, FormField,
    PageTarget, PointerAction, PresentationMode, Query, Rendered, Viewer, Zoom,
};

use crate::controls::{FieldChange, Placed};
use viewer_host::ControlFit;
use viewer_host::arrangement::next_layout;
use viewer_host::panel::{self, Miniatures, RowAction, Tab};
use viewer_host::trace::{Topic, Trace};

use crate::{controls, page, pages, tree};

/// The identity this host gives the one document it opens.
const DOCUMENT: DocumentId = DocumentId(1);

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

/// What the chrome layer draws, kept apart from the [`Host`] on purpose.
///
/// GTK draws from its own main loop, so a draw function that reached into the host would borrow
/// it at a moment nothing else chose. This is the one piece of state both sides touch, it is
/// written only by [`Host::refresh`] and read only by the draw function, and keeping it separate
/// is what makes that provable rather than argued.
#[derive(Debug, Default)]
struct Chrome {
    /// §12.5.6.10's target: what is selected, in device pixels of the viewport.
    selection: Vec<[f32; 8]>,
    /// Every occurrence of the find bar's string on this page, in the same pixels.
    ///
    /// Beside the selection rather than folded into it, because the two say different things and
    /// a person needs both at once: these are *where else* the word is and the selection is
    /// *which one you are on*. Drawn first and fainter, so the current occurrence reads as the
    /// one the platform's own selection colour is over.
    matches: Vec<[f32; 8]>,
    /// Annex O's `highlight`: the rectangles the URI's fragment named, in the same pixels.
    ///
    /// Table Annex O.4 leaves what one looks like to a processor — "[t]he nature of the
    /// highlighting is implementation-dependent" — and this platform's honest answer is the same
    /// one it gives a match: the theme's own colour at an alpha of its own, because GTK exposes no
    /// second colour to ask for.
    highlights: Vec<[f32; 8]>,
    /// §12.5.1's ring, in the same pixels.
    focus: Option<[f32; 8]>,
    /// Device pixels per logical pixel, because GTK draws in logical ones.
    scale: f64,
}

/// What one of [`Tab`]'s panels turned out to hold.
///
/// Two shapes rather than one, because §12.3.4's is genuinely not a tree: five of the six panels
/// are a [`panel::PanelRow`] apiece and the sixth is a page count whose pictures are fetched a row
/// at a time. Collapsing them would mean either a row type carrying a picture nobody else has or a
/// list of miniatures decoded before anybody looked at one, and `CLAUDE.md` section 2 forbids the second.
#[derive(Debug)]
enum Panel {
    /// A tree of rows.
    Rows(Vec<panel::PanelRow>),
    /// §12.3.4: how many pages there are. The miniatures come from [`Host::page_sink`].
    Pages(usize),
}

/// The widgets, held so that the loop can put things in them.
#[derive(Debug)]
struct Ui {
    /// The toplevel.
    window: gtk4::ApplicationWindow,
    /// The page and the form controls.
    fixed: gtk4::Fixed,
    /// §12.5.6.14's popup windows, in a layer of their own over the page.
    ///
    /// Not in [`Ui::fixed`], and the reason is in `build_window`: a `GtkFixed` measures its
    /// children and a window the document put beside the page would then decide how wide the
    /// window is.
    popups: gtk4::Fixed,
    /// The pixels of every page Table 29's arrangement is showing, one widget apiece.
    ///
    /// Grown when a layout puts more pages on the screen and never shrunk — a `GtkPicture` with
    /// no paintable and `set_visible(false)` costs a hidden widget, and destroying and rebuilding
    /// them on every scroll would churn the widget tree at scroll speed.
    pictures: Vec<gtk4::Picture>,
    /// The layer that measures the viewport and draws the interactive chrome.
    chrome: gtk4::DrawingArea,
    /// Where each of [`viewer_host::Tab`]'s panels goes, in [`viewer_host::Tab::ALL`]'s order.
    ///
    /// A slot apiece rather than six named fields, because the list of panels is
    /// `viewer_host::Tab` and a second list here would be a second thing to keep level with the
    /// other two hosts. [`Tab::index`] is what addresses one.
    slots: Vec<gtk4::Box>,
    /// The panels' notebook — Table 29's "any other window", and what the page mode opens.
    tabs: gtk4::Notebook,
    /// The splitter it sits in, because full screen takes the notebook *out* rather than hiding
    /// it: a `GtkPaned` keeps a position that was set, and a hidden child would leave the space.
    split: gtk4::Paned,
    /// The header bar's own buttons, which are this host's tool bar: §12.2's `/HideToolbar`.
    ///
    /// The *buttons* rather than the bar, because GTK4 puts them in the same widget as the window
    /// controls and a document asking for no tool bar has not asked for no close button. Full
    /// screen takes the whole titlebar, which is Table 29's separate sentence.
    tool_buttons: Vec<gtk4::Button>,
    /// The separator and the status label, which are §12.2's `/HideWindowUI`.
    status_bar: gtk4::Box,
    /// Trap 5's channel reaching a person: what the page could not draw, and what was refused.
    status: gtk4::Label,
    /// The find bar, which is a real [`gtk4::SearchBar`] and not a rectangle this host draws.
    find: gtk4::SearchBar,
    /// The string in it.
    find_entry: gtk4::SearchEntry,
}

/// One document, one window, and the loop between them.
///
/// `struct_excessive_bools` is asking for a state machine, and there is no state here to make one
/// of: each flag is an independent fact about a *different* clause's window — whether the document
/// is open, whether it is unsaved, whether the first frame has been reported, whether the reader
/// asked for Table 29's panel. Packing four unrelated sentences into one enumeration would make a
/// state out of their product, which is fifteen states nobody wrote down. The same argument
/// `viewer_host::Chrome` is written under.
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
    bytes: Vec<u8>,
    /// Annex O's fragment, where the host was given one.
    fragment: Option<String>,
    /// Tier 1's worker, called on this thread. See ADR 0244 for what that costs and why.
    rasterizer: CpuRasterizer,
    /// The launch timeline.
    pub(crate) trace: Trace,
    /// The widgets.
    ui: Ui,
    /// What the chrome layer draws.
    chrome: Rc<RefCell<Chrome>>,
    /// Set while this host is writing values into its own controls, so that the write is not
    /// mistaken for a person typing.
    suppress: Rc<Cell<bool>>,
    /// This host, so that a dialogue's callback can reach it after the borrow that opened it ends.
    me: Weak<RefCell<Self>>,
    /// Device pixels per logical pixel, from the display GTK put the window on.
    scale: i32,
    /// §7.6.4.1's attempts, counted by [`viewer_host::Asking`] so that three hosts count alike.
    asking: viewer_host::Asking,
    /// §12.3.4's miniatures, decoded when a row is drawn and bounded by [`viewer_host::Miniatures`].
    ///
    /// Beside the host's own fields rather than among them, because the closure a `GtkListView`
    /// binds a row with reaches it while the host may itself be borrowed — the same reason
    /// [`Host::chrome`] is an [`Rc<RefCell<Chrome>>`].
    miniatures: Rc<RefCell<Miniatures<gtk4::gdk::MemoryTexture>>>,
    /// Whether the document has been opened yet, which waits for the first allocation.
    opened: bool,
    /// Whether anything is unsaved.
    dirty: bool,
    /// What the title bar says about the page.
    caption: String,
    /// The controls over the page, and which fields they are for.
    placed: Vec<Placed>,
    /// §12.5.6.14's open popup windows, as the widgets placed for them.
    popups: Vec<gtk4::Frame>,
    /// The answer those widgets were built from, so that a repaint that changes nothing rebuilds
    /// nothing. `viewer_core::PopupWindow` is `PartialEq`, which is what makes the comparison the
    /// whole test.
    popups_shown: Vec<viewer_core::PopupWindow>,
    /// Whether the pointer was last over §12.5.6.5's activation region.
    ///
    /// Kept so that `Query::LinkAt` changes the cursor when the answer changes rather than on
    /// every motion event: `gdk_surface_set_cursor` is a round trip to the display server and a
    /// pointer moved across a page produces hundreds of them a second.
    over_link: bool,
    /// Whether the first frame has been reported, so that the launch line is printed once.
    presented: bool,
    /// §14.7's tree on AT-SPI, brought up after the first frame and never before it.
    ///
    /// **`None` until [`Host::attend`] has run once**, which is `CLAUDE.md`'s startup rule: the
    /// bridge spawns a thread that connects to the session bus, and page one may not wait behind
    /// a D-Bus round trip for a screen reader that is probably not there (ADR 0623).
    pub(crate) accessibility: Option<viewer_accessibility::Bridge>,
    /// The page and viewport last published to it, so that a tree is not rebuilt per frame.
    pub(crate) spoken: Option<viewer_accessibility::Showing>,
    /// Set from `accesskit_unix`'s own thread when a client asks for something.
    ///
    /// The only value in this host that crosses a thread boundary, and it carries no payload on
    /// purpose: the request itself is read back on the main thread through
    /// `Bridge::requested`, against the tree the client actually walked.
    pub(crate) access_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// How often the drain below is waking, so that the source is re-armed only when it changes.
    pub(crate) access_interval: Option<i32>,
    /// The source draining that flag, at the interval the bridge asks for.
    pub(crate) access_draining: Option<glib::SourceId>,
    /// What the find bar is looking for, kept because the *page's* highlights are asked for on
    /// every repaint while the document-wide search is a plan inside `viewer-core`.
    needle: String,
    /// How many pages a search still has to read, which is what puts a `Find::Continue` on the
    /// idle queue after each repaint. Zero when nothing is being searched for.
    ///
    /// A count rather than a flag because it is also what the status line says: a person watching
    /// a thousand-page document wants to see it come down.
    pages_left: usize,
    /// §6.3.2.2: who draws §12.7's widgets — this host's controls, or the document's own pictures.
    ///
    /// [`WidgetAppearances::Delegated`] unless the command line asked otherwise, because this host
    /// *does* place a real control over every widget and leaving the appearance underneath is the
    /// duplication ADR 0244 photographed. `--draw-widget-appearances` restores §6.3.2.2's default,
    /// which is what taking the two pictures side by side needs.
    widget_appearances: WidgetAppearances,
    /// How much of what the document asserts over its reader this window obeys.
    ///
    /// The other policy value beside the one above, and the same kind of thing: a fact about what
    /// *this reader* has been asked to do rather than about the file.
    /// [`viewer_core::RestrictionLevel::On`] unless [`viewer_host::IGNORE_RESTRICTIONS`] was on the
    /// command line — which this program refused as an unknown option while telling every person
    /// who hit a refusal to use it (ADR 0604).
    restrictions: viewer_core::RestrictionLevel,
    /// The magnification at which every control on this page would fit its `/Rect`, where they do
    /// not fit now.
    ///
    /// **ADR 0245's third decision, and it needed no message.** `viewer_host::ControlFit` computes
    /// it from what `Query::Fields` already answers and what GTK already says a control's minimum
    /// is; `w` sends it as `Zoom::Scale`. It is offered rather than applied, because a viewer that
    /// magnified a page by itself because a form is on it would be answering a question nobody
    /// asked — which gesture asks for it is chrome, and chrome is a host's (rule 5).
    fit_magnification: Option<f32>,
    /// Whether the reader wants the panel of three trees on the screen.
    ///
    /// **A wish, beside Table 29's *permission***. `Presenting::chrome`'s `other_windows` says
    /// whether the clause allows a panel at all and this says whether the person asked for one;
    /// `apply_chrome` shows it only where both are true, so leaving full screen puts back what the
    /// reader had rather than what the document last permitted. `o` is the key, in all three hosts
    /// since ADR 0526.
    panel_wanted: bool,
    /// Table 29's `/PageMode /FullScreen`, §12.2's chrome flags, and the way back out.
    ///
    /// **The window §12.4.4's presentation had never had** (ADR 0470). Which sentence this window
    /// is obeying is `viewer_host::Presenting`, shared with the other two hosts; what is GTK's is
    /// `GtkWindow::fullscreen` and which widget each of Table 147's three flags names.
    presenting: viewer_host::Presenting,
    /// §12.4.4.1's clock, while a presentation is running.
    ///
    /// **`None` is a window with no timer armed at all**, rather than a timer that wakes to find
    /// nothing to do: `pump_presentation` re-arms a one-shot only while this is `Some`, so a
    /// reader who is not presenting pays nothing. The decision inside it — how often, what a tick
    /// carries, when Table 164's `/D` has run out — is `viewer_host::Clock`, shared with the other
    /// two hosts.
    clock: Option<viewer_host::Clock>,
    /// The one-shot waiting to turn the clock, so that it can be moved when the interval changes.
    ///
    /// One source at a time, removed and re-armed rather than repeating: a transition wants a
    /// frame every sixtieth of a second and a still page wants a tick every tenth, and a repeating
    /// source cannot change its mind.
    armed: Option<glib::SourceId>,
    /// A transition named by the core, waiting for the page it moves *to* to be rendered.
    ///
    /// §12.4.4.1's transition is one *to* a page, and the core settles after the command that
    /// turned it: the events arrive as page change, transition, render request. Beginning the
    /// effect here would animate the page being left against itself.
    arming: Option<pdf_model::navigation::Transition>,
    /// The page on the screen, as the list that drew it and where a whole-viewport draw would put
    /// it — the face a transition would move *from*.
    ///
    /// Kept only while a presentation is running, because it costs a `Query::PageGeometry` per
    /// page render and nothing outside §12.4.4 asks for it. The list is shared rather than
    /// copied: `RenderRequest::list` is an `Arc` precisely so that a host may keep one.
    shown: Option<(
        std::sync::Arc<pdf_render::DisplayList>,
        pdf_render::TargetSpec,
    )>,
    /// The viewport in device pixels, which is the rectangle a transition's frames are drawn in.
    pub(crate) viewport: (u32, u32),
    /// Table 29's arrangement, as this window last asked for it.
    ///
    /// Kept because `l` *cycles*: the value in force is the viewer's, and a host that wanted to
    /// know it without remembering would have to ask a question this vocabulary does not have —
    /// which is the right answer, because `Query::Opening` says what the *document* asked for and
    /// that is a different sentence.
    layout: pdf_model::viewer_preferences::PageLayout,
}

/// The next of Table 29's six arrangements, in the order that table states them.
///
/// A host's choice and not a clause's: the standard states the six and says nothing about moving
/// between them, because moving between them is a user interface.
/// How wide the panel is, in logical pixels, when it is showing.
///
/// Named because Table 29's `FullScreen` needs to put the divider back where it was: hiding the
/// notebook is not enough on its own, since a `GtkPaned` keeps a position that was set.
///
/// **Three hundred until `viewer_host::Tab` made it six panels**, and the screen is what raised it:
/// six tab labels and a tree do not share three hundred logical pixels. The tabs went down the
/// side at the same time, which is where the rest of the answer is.
const PANEL_WIDTH: i32 = 380;

/// How far one notch of the wheel moves the page, in logical pixels.
///
/// A choice, and written down as one: the standard says nothing about a wheel. Three lines of a
/// document's body text is what every reader the project owner uses has converged on, and this is
/// about that.
const SCROLL_STEP: f64 = 48.0;

/// How far §12.5.6.14's text sits from the edge of its window, in logical pixels.
///
/// A choice, and written down as one: the clause states a rectangle and not one word about what a
/// window looks like inside it. The other two hosts make the same choice in their own units.
const POPUP_PADDING: i32 = 5;

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
    /// Builds the window and everything hanging off it, and shows it.
    ///
    /// The document is **not** opened here: it is opened on the first allocation, because the
    /// viewport's size decides the resolution page one is rasterised at and a page drawn at a
    /// guessed size would be drawn twice. What that costs the launch path is one GTK allocation,
    /// and `--trace=launch` prints it.
    ///
    /// # Errors
    ///
    /// [`HostError::Unreadable`] where the file named cannot be read.
    pub fn open(
        app: &gtk4::Application,
        path: &Path,
        fragment: Option<String>,
        widget_appearances: WidgetAppearances,
        restrictions: viewer_core::RestrictionLevel,
        trace: Trace,
    ) -> Result<Rc<RefCell<Self>>, HostError> {
        let bytes = std::fs::read(path).map_err(|error| HostError::Unreadable {
            path: path.to_owned(),
            error: error.to_string(),
        })?;
        trace.say(
            Topic::Launch,
            format_args!("read {} bytes of {}", bytes.len(), path.display()),
        );
        Ok(Rc::new_cyclic(|me| {
            let chrome = Rc::new(RefCell::new(Chrome {
                scale: 1.0,
                ..Chrome::default()
            }));
            let ui = build_window(app, path, me, &chrome);
            trace.say(Topic::Launch, format_args!("window built"));
            RefCell::new(Self {
                viewer: Viewer::new(1, 1, 1.0),
                path: path.to_owned(),
                directory: path.parent().map(Path::to_owned),
                bytes,
                fragment,
                rasterizer: CpuRasterizer::new(),
                trace,
                ui,
                chrome,
                suppress: Rc::new(Cell::new(false)),
                me: me.clone(),
                scale: 1,
                asking: viewer_host::Asking::new(),
                miniatures: Rc::new(RefCell::new(Miniatures::new())),
                opened: false,
                dirty: false,
                caption: String::new(),
                placed: Vec::new(),
                popups: Vec::new(),
                popups_shown: Vec::new(),
                over_link: false,
                presented: false,
                accessibility: None,
                spoken: None,
                access_pending: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                access_interval: None,
                access_draining: None,
                needle: String::new(),
                pages_left: 0,
                widget_appearances,
                restrictions,
                fit_magnification: None,
                // The panel is what this window opens with, and `o` is what takes it away.
                panel_wanted: true,
                // Table 147's and Table 29's own defaults, replaced by what the catalog states
                // the moment the document opens.
                presenting: viewer_host::Presenting::default(),
                clock: None,
                armed: None,
                arming: None,
                shown: None,
                viewport: (1, 1),
                layout: pdf_model::viewer_preferences::PageLayout::SinglePage,
            })
        }))
    }

    /// The viewport changed size, in logical pixels.
    ///
    /// The first one opens the document, which is what makes page one's raster the size of the
    /// window it will be shown in rather than of a guess.
    fn resized(&mut self, width: i32, height: i32) {
        self.scale = self.ui.chrome.scale_factor().max(1);
        let scale = f64::from(self.scale);
        self.chrome.borrow_mut().scale = scale;
        let (Ok(width), Ok(height)) = (
            u32::try_from(width.saturating_mul(self.scale)),
            u32::try_from(height.saturating_mul(self.scale)),
        ) else {
            return;
        };
        if width == 0 || height == 0 {
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small integer, so f64 to f32 here loses \
                      nothing a viewport can express"
        )]
        let scale = scale as f32;
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
                format_args!("first allocation {width}x{height} device px, scale {scale}"),
            );
            self.open_document(None);
        }
    }

    /// §7.6.4.1: opens the document, with a password where one has been supplied.
    fn open_document(&mut self, password: Option<viewer_core::Secret>) {
        let bytes = self.bytes.clone();
        let fragment = self.fragment.clone();
        // Both policy values go before the document, and for one reason: a policy applied halfway
        // through is not a policy. §6.3.2.2's instruction is a property of *this host* rather than
        // of the document, which is why it is not part of `Open` — a host that changes its mind
        // sends it again and the page is rebuilt — and `Restrict` is the reader's answer to what
        // the *file* asserts, which `CLAUDE.md` says is always the reader's to give.
        self.pump(VecDeque::from([
            Command::Restrict(self.restrictions),
            Command::Delegate(self.widget_appearances),
            Command::Open {
                id: DOCUMENT,
                bytes,
                password,
                fragment,
            },
        ]));
    }

    /// One command, and everything it produces.
    pub(crate) fn dispatch(&mut self, command: Command) {
        self.pump(VecDeque::from([command]));
    }

    /// Runs commands until nothing is left, reacting to what each produces, then repaints.
    fn pump(&mut self, mut queue: VecDeque<Command>) {
        while let Some(command) = queue.pop_front() {
            let described = self
                .trace
                .on(Topic::Events)
                .then(|| format!("{command:?}"))
                .map(|text| text.chars().take(120).collect::<String>());
            // **A command that changes the document changes what §14.7's tree says**, and
            // `Showing` cannot see it: an edit and a click move neither the page nor the viewport.
            // Which commands those are is one statement for all three windows
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
        self.refresh();
        self.pump_search();
        self.pump_presentation();
    }

    /// Does what one event asks.
    fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            Event::Opened { pages, .. } => {
                self.trace
                    .say(Topic::Launch, format_args!("opened, {pages} page(s)"));
                // Trap 5: a page tree with no leaves is a *correctly read* document with nothing
                // to show, and a blank window is what a broken file looks like too. Said in all
                // three hosts since the seven-hundred-and-fourth session; it was said in none.
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
                viewer_host::Ask::Prompt { attempt, of } => self.ask_for_a_password(attempt, of),
                viewer_host::Ask::Exhausted => self.say(viewer_host::password::EXHAUSTED),
            },
            // Two events with nothing to do here, for two different reasons. A close drops
            // everything derived from the document and this host opens one document and never
            // closes it; damage is what a tier-1 host repaints from `Query::Frame`, and the
            // repaint at the end of every pump has already been scheduled by the time this
            // arrives.
            Event::Closed(_) | Event::Damage(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => self.turned(index, label.as_deref(), of, section.as_deref()),
            Event::NeedsRender(request) => {
                let began = std::time::Instant::now();
                let rendered = match self.rasterizer.rasterize(&request.list, request.target) {
                    Ok(raster) => Rendered::Raster(raster),
                    // Trap 5: a host that quietly kept the previous page would be telling a person
                    // something false about this one.
                    Err(error) => Rendered::Failed(error.to_string()),
                };
                self.trace.say(
                    Topic::Frames,
                    format_args!(
                        "page {} rasterised {}x{} in {:?}",
                        request.page.saturating_add(1),
                        request.target.width,
                        request.target.height,
                        began.elapsed()
                    ),
                );
                queue.push_back(Command::RenderReady {
                    token: request.token,
                    rendered,
                });
                // §12.4.4.1: the page a transition moves *to* is the one whose list has just
                // arrived, so this is where an armed one can begin. Only while a presentation is
                // running, because taking the face costs a whole-viewport rasterisation and no
                // other clause wants one.
                self.face_arrived(&request);
            }
            // §12.6.4.8: handed over rather than opened. The string is one the *document*
            // controls, and giving it to a browser is a decision about this machine that this
            // host has not been given — the same answer `viewer-ui` gives.
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
            // pages are rasterised for it. Both are said rather than swallowed, because a person
            // who asked for a slide show is owed the difference.
            Event::Transition { transition, .. } => self.arm_transition(transition),
            Event::Extracted {
                asked, name, bytes, ..
            } => self.write_extracted(asked, &name, &bytes),
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            Event::Dirty { dirty, .. } => {
                self.dirty = dirty;
                self.retitle();
            }
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => self.searched(found, remaining, wrapped),
            Event::Reported { page, notes, .. } => self.reported(page, &notes),
            // `CLAUDE.md`: a document's restrictions are the reader's to set, and it shall always
            // be possible to turn them off. The sentence is `viewer_host::refused` rather than
            // this host's own because it names the word the argument parser takes, and this host
            // wrote its own copy of it for sessions while taking no such word (ADR 0604).
            Event::Refused { notes, .. } => self.say(&viewer_host::refused(&notes)),
        }
    }

    /// Puts the frame on the screen, the controls over it, and the chrome over both.
    ///
    /// Idempotent and called after every pump, which is what keeps the window a function of the
    /// viewer's state rather than of the order events arrived in.
    fn refresh(&mut self) {
        let began = std::time::Instant::now();
        // **One texture per page of Table 29's arrangement**, since `/PageLayout` was obeyed.
        // Under `SinglePage` this is the one placement it has always been.
        // §12.4.4.1: while one of Table 164's effects is in flight the window shows the effect
        // and not the page. One texture at the viewport's own origin, because a frame is already
        // a picture of two pages placed where they belong.
        let placements: Vec<(gtk4::gdk::MemoryTexture, (f32, f32), usize)> =
            if let Some(frame) = self.transition_placement() {
                vec![frame]
            } else {
                match self.viewer.query(Query::Frame) {
                    Answer::Frame(frames) => frames
                        .into_iter()
                        .filter_map(|frame| match page::texture(frame.raster) {
                            Ok(texture) => Some((texture, frame.origin, frame.raster.data.len())),
                            Err(error) => {
                                eprintln!("note: {error}");
                                None
                            }
                        })
                        .collect(),
                    _ => Vec::new(),
                }
            };
        if !placements.is_empty() {
            // Tier 1's whole cost, in the one place it is paid: `doc/ui-boundary.md` prices the
            // tier at "one copy per frame" and this is that copy, timed rather than assumed
            // (ADR 0244).
            let bytes: usize = placements.iter().map(|(_, _, bytes)| bytes).sum();
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "{bytes} bytes into {} texture(s) in {:?}",
                    placements.len(),
                    began.elapsed()
                ),
            );
            let scale = f64::from(self.scale);
            while self.ui.pictures.len() < placements.len() {
                let picture = gtk4::Picture::new();
                picture.set_content_fit(gtk4::ContentFit::Fill);
                picture.set_can_shrink(false);
                self.ui.fixed.put(&picture, 0.0, 0.0);
                self.ui.pictures.push(picture);
            }
            for (slot, (texture, origin, _)) in self.ui.pictures.iter().zip(&placements) {
                slot.set_paintable(Some(texture));
                slot.set_size_request(
                    logical(f64::from(texture.width()), scale),
                    logical(f64::from(texture.height()), scale),
                );
                slot.set_visible(true);
                self.ui.fixed.move_(
                    slot,
                    f64::from(origin.0) / scale,
                    f64::from(origin.1) / scale,
                );
            }
            // The widgets a smaller arrangement no longer needs: hidden rather than destroyed,
            // because the next scroll will want them back.
            for spare in self.ui.pictures.iter().skip(placements.len()) {
                spare.set_paintable(gtk4::gdk::Paintable::NONE);
                spare.set_visible(false);
            }
            self.ui.fixed.set_visible(true);
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
        }
        let fields = match self.viewer.query(Query::Fields) {
            Answer::Fields(fields) => fields,
            _ => Vec::new(),
        };
        self.place_fields(&fields);
        // §12.5.6.14's windows, which have no appearance stream and are therefore nowhere in the
        // pixels above: they are furniture, and this host places real GTK widgets for them.
        let popups = match self.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows,
            _ => Vec::new(),
        };
        self.place_popups(&popups);
        self.refresh_chrome();
        // Last, and **only once a frame is actually on the screen**: §14.7's tree on AT-SPI. The
        // guard is the rule rather than an optimisation, and it was put here by a measurement —
        // without it `--trace` printed `accessibility bridge up` at 1.656 s and
        // `first frame on the screen` at 1.671, because `refresh` runs once on the first
        // allocation before the document is even open. `Bridge::new` spawns a thread and connects
        // to the session bus, which is exactly what `CLAUDE.md`'s startup section forbids in front
        // of page one (ADR 0623, and ADR 0214 for the rule).
        if self.presented {
            self.attend();
        }
    }

    /// This host, weakly, for a callback that has to reach it after the borrow that armed it ends.
    pub(crate) fn me(&self) -> Weak<RefCell<Self>> {
        self.me.clone()
    }

    /// What to call the document, which is the file's own name.
    pub(crate) fn named(&self) -> String {
        named(&self.path)
    }

    /// What the title bar says about the page, which is what the window is called after the name.
    pub(crate) fn caption(&self) -> &str {
        &self.caption
    }

    /// The shapes the chrome layer draws, gathered from the four questions that answer them.
    ///
    /// Its own method rather than the tail of [`Host::refresh`] because all four are asked on
    /// every repaint and none of them touches a widget: what comes back is written into the one
    /// piece of state the draw function reads, and the layer is asked to redraw itself.
    fn refresh_chrome(&mut self) {
        let selection = match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.quads.clone(),
            _ => Vec::new(),
        };
        let focus = match self.viewer.query(Query::Focus) {
            Answer::Focus { quad, .. } => Some(quad),
            _ => None,
        };
        // Every occurrence on the page being shown. Asked on every repaint because it is a query
        // over a readback that already exists — `Query::Find` is this page and `Command::Find` is
        // the document, and only the second one costs anything.
        let matches = if self.needle.is_empty() {
            Vec::new()
        } else {
            match self.viewer.query(Query::Find(&self.needle)) {
                Answer::Found(occurrences) => occurrences.into_iter().flatten().collect(),
                _ => Vec::new(),
            }
        };
        // Annex O's highlighted rectangle, if the URI this document was opened from named one.
        // Answered from a list the fragment filled when the document opened, so it costs a filter
        // over that list and nothing else.
        let highlights = match self.viewer.query(Query::Highlight) {
            Answer::Highlighted(quads) => quads,
            _ => Vec::new(),
        };
        {
            let mut chrome = self.chrome.borrow_mut();
            chrome.selection = selection;
            chrome.matches = matches;
            chrome.highlights = highlights;
            chrome.focus = focus;
            chrome.scale = f64::from(self.scale);
        }
        self.ui.chrome.queue_draw();
    }

    /// §12.7's controls, built once and moved thereafter.
    ///
    /// Rebuilt only when the set of fields and widgets on the page changes, because rebuilding
    /// takes the keyboard away from whatever a person is typing into and the page moves under
    /// them on every scroll.
    fn place_fields(&mut self, fields: &[FormField]) {
        let scale = f64::from(self.scale);
        if controls::signature(fields)
            != self
                .placed
                .iter()
                .map(|p| p.key.clone())
                .collect::<Vec<_>>()
        {
            for placed in self.placed.drain(..) {
                self.ui.fixed.remove(&placed.widget);
            }
            let change = self.field_sink();
            for field in fields {
                for widget in &field.widgets {
                    if let Some(placed) = controls::build(field, widget, &self.suppress, &change) {
                        self.ui.fixed.put(&placed.widget, 0.0, 0.0);
                        self.placed.push(placed);
                    }
                }
            }
            self.trace.say(
                Topic::Panel,
                format_args!(
                    "{} field(s) on the page, {} control(s) placed",
                    fields.len(),
                    self.placed.len()
                ),
            );
        }
        // The quadrilaterals are in device pixels of the viewport, the same form
        // `Selected::quads` and `Answer::Focus` take — one arithmetic in one place (ADR 0118).
        self.suppress.set(true);
        let mut fit = ControlFit::default();
        for field in fields {
            for widget in &field.widgets {
                let key = (field.name.qualified.clone(), widget.annotation);
                let Some(placed) = self.placed.iter().find(|placed| placed.key == key) else {
                    continue;
                };
                let (x, y, width, height) = viewer_host::bounds(widget.quad);
                self.ui
                    .fixed
                    .move_(&placed.widget, f64::from(x) / scale, f64::from(y) / scale);
                let (asked_width, asked_height) = (
                    logical(f64::from(width), scale),
                    logical(f64::from(height), scale),
                );
                placed.widget.set_size_request(asked_width, asked_height);
                // `measure` with `-1` in both directions is GTK's "for no particular size", which
                // is the minimum itself; asking for a height at a width below the control's
                // minimum is the case GTK warns about on the console.
                let (minimum_width, ..) = placed.widget.measure(gtk4::Orientation::Horizontal, -1);
                let (minimum_height, ..) = placed.widget.measure(gtk4::Orientation::Vertical, -1);
                fit.record((asked_width, asked_height), (minimum_width, minimum_height));
                write_back(placed, field, widget);
            }
        }
        self.suppress.set(false);
        self.report_fit(&fit);
    }

    /// §12.5.6.14's open windows, as real widgets over the page.
    ///
    /// **Rebuilt rather than moved, which is the opposite of [`Host::place_fields`]' rule and is
    /// right for the opposite reason.** A control holds the keyboard and a person's half-typed
    /// value, so rebuilding one costs them their place; a popup holds neither — the clause gives
    /// it "no appearance stream or associated actions of its own", and this host makes it untargetable
    /// — so there is nothing in one to lose. A page states as many windows as it has open comments,
    /// which is few, and the comparison below means a scroll that moves nothing rebuilds nothing.
    fn place_popups(&mut self, popups: &[viewer_core::PopupWindow]) {
        if self.popups_shown == popups {
            return;
        }
        for widget in self.popups.drain(..) {
            self.ui.popups.remove(&widget);
        }
        let scale = f64::from(self.scale);
        let placed = viewer_host::popup::windows(popups);
        for window in &placed {
            let widget = popup_window(window);
            let (x, y, width, height) = window.place;
            widget.set_size_request(
                logical(f64::from(width), scale),
                logical(f64::from(height), scale),
            );
            self.ui
                .popups
                .put(&widget, f64::from(x) / scale, f64::from(y) / scale);
            self.popups.push(widget);
        }
        // On what the *answer* held rather than on what was placed, so that a window the page
        // states and this host could not put anywhere is a line rather than a silence — which is
        // trap 5 applied to the one report that would otherwise say nothing about a refusal.
        if !popups.is_empty() || !self.popups_shown.is_empty() {
            self.trace.say(
                Topic::Panel,
                format_args!(
                    "{} of {} §12.5.6.14 popup window(s) placed",
                    placed.len(),
                    popups.len()
                ),
            );
        }
        self.popups_shown = popups.to_vec();
    }

    /// What the controls' minimum sizes say about the magnification, said once per placement.
    ///
    /// **ADR 0245 left this as a third decision and it needed no message.** The count is what ADR
    /// 0244 and ADR 0246 measured by hand on the two toolkits; the magnification beside it is
    /// `viewer_host::ControlFit`'s arithmetic over the same numbers, and pressing `w` sends it as
    /// `Zoom::Scale`, which the vocabulary has had since the hundred-and-thirty-first session.
    fn report_fit(&mut self, fit: &ControlFit) {
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
    /// display's own factor comes back out. Getting that wrong on a doubled display would put the
    /// answer out by two, which is exactly the class of arithmetic ADR 0118 keeps in one place.
    fn showing_at(&self) -> Option<f32> {
        let Answer::Page { index, .. } = self.viewer.query(Query::CurrentPage) else {
            return None;
        };
        let Answer::Geometry(geometry) = self.viewer.query(Query::PageGeometry(index)) else {
            return None;
        };
        let display = self.scale.max(1);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a display scale is 1, 2 or 3; the lint is about the general case"
        )]
        let logical = geometry.scale / display as f32;
        (logical.is_finite() && logical > 0.0).then_some(logical)
    }

    /// What a control does when a person changes it.
    fn field_sink(&self) -> Rc<dyn Fn(FieldChange)> {
        let me = self.me.clone();
        Rc::new(move |change| {
            with(&me, |host| match change {
                FieldChange::Set { field, value } => {
                    if let Entered::Chosen(chosen) = &value {
                        host.trace.say(
                            Topic::Panel,
                            format_args!(
                                "{field}: option(s) {chosen:?} of Table 234's /Opt selected"
                            ),
                        );
                    }
                    host.dispatch(Command::Edit(Edit::SetField { field, value }));
                }
                FieldChange::Activate(annotation) => {
                    host.dispatch(Command::Activate(annotation));
                }
            });
        })
    }

    /// What one of [`Tab`]'s panels holds, once its answer has been asked for.
    ///
    /// **The match below is exhaustive over [`Tab`] on purpose**, and it is this host's half of
    /// `doc/todo/30`'s "all three hosts stay level": a panel added to `viewer_host::Tab` fails to
    /// compile here, in `viewer-qt` and in `viewer-ui` until each supplies a widget for it. It is
    /// [`viewer_host::Key`]'s mechanism applied to the other thing a window shows (ADR 0526).
    ///
    /// [`viewer_host::Key`]: viewer_host::Key
    fn panel_of(&self, tab: Tab) -> Panel {
        match tab {
            Tab::Contents => Panel::Rows(match self.viewer.query(Query::Outline) {
                Answer::Outline(outline) if !outline.items.is_empty() => {
                    panel::outline_rows(&outline)
                }
                _ => vec![panel::PanelRow::saying("This document states no outline.")],
            }),
            // §12.3.4 is a page count and a picture per row rather than a tree — see `crate::pages`
            // for why the count is all that crosses here.
            Tab::Pages => Panel::Pages(match self.viewer.query(Query::PageCount) {
                Answer::Count(count) => count,
                _ => 0,
            }),
            Tab::Layers => Panel::Rows(match self.viewer.query(Query::Layers) {
                Answer::Layers(layers) if !layers.is_empty() => panel::layer_rows(&layers),
                _ => vec![panel::PanelRow::saying(
                    "This document states no optional content.",
                )],
            }),
            Tab::Files => Panel::Rows(match self.viewer.query(Query::Attachments) {
                Answer::Attachments(files) if !files.is_empty() => panel::attachment_rows(&files),
                _ => vec![panel::PanelRow::saying("This document embeds no files.")],
            }),
            Tab::Articles => Panel::Rows(match self.viewer.query(Query::Articles) {
                Answer::Articles(threads) => panel::article_rows(&threads),
                _ => panel::article_rows(&[]),
            }),
            Tab::Document => Panel::Rows(match self.viewer.query(Query::Properties) {
                Answer::Properties {
                    information,
                    metadata,
                } => panel::property_rows(&information, metadata.as_ref()),
                _ => panel::property_rows(&pdf_model::metadata::Information::default(), None),
            }),
        }
    }

    /// Builds every panel from its own answer.
    fn build_panels(&mut self) {
        let act = self.row_sink();
        let row = self.page_sink();
        let show = self.show_page_sink();
        self.miniatures.borrow_mut().clear();
        for tab in Tab::ALL {
            let filled = self.panel_of(*tab);
            let Some(slot) = self.ui.slots.get(tab.index()) else {
                continue;
            };
            while let Some(child) = slot.first_child() {
                slot.remove(&child);
            }
            match &filled {
                Panel::Rows(rows) => {
                    self.trace.say(
                        Topic::Panel,
                        format_args!("{}: {} row(s)", tab.label(), rows.len()),
                    );
                    slot.append(&tree::tree(rows, &act));
                }
                Panel::Pages(count) => {
                    self.trace.say(
                        Topic::Panel,
                        format_args!("{}: {count} page(s), miniatures on demand", tab.label()),
                    );
                    // **On the idle queue, not here**, and the screen is what said so.
                    // `GtkListView` binds a row from GTK's layout, and putting the list into a
                    // realised window starts one *synchronously* — inside this method, which runs
                    // with the host's `RefCell` borrowed. The first run printed "the host was busy,
                    // so page 1's row was drawn without its /Thumb" and drew a page list with no
                    // pictures in it. Deferring lets the command that got here unwind first; it is
                    // the same move `find.connect_search_mode_enabled_notify` makes below, and
                    // `viewer-qt` makes with `Busy`.
                    let (slot, count, row, show) =
                        (slot.clone(), *count, row.clone(), show.clone());
                    glib::idle_add_local_once(move || {
                        slot.append(&pages::page_list(count, &row, &show));
                    });
                }
            }
        }
    }

    /// §12.3.4: one page's label and miniature, asked for when the list is about to draw that row.
    ///
    /// The decode is here rather than in `build_panels` because that is the whole of `CLAUDE.md`
    /// §2's rule reaching this panel — a loop over the page count would have moved the eager work
    /// out of the launch path rather than out of the program. [`viewer_host::Miniatures`] bounds
    /// what is kept afterwards.
    ///
    /// The cache is beside the host rather than in it so that a `GtkListView` binding a row while
    /// the host is borrowed still gets its picture: `bind` fires from GTK's layout, which a
    /// `set_start_child` or a `set_current_page` inside a command can start.
    fn page_sink(&self) -> Rc<dyn Fn(usize) -> Option<pages::Row>> {
        let me = self.me.clone();
        let held = Rc::clone(&self.miniatures);
        Rc::new(move |index| {
            let host = me.upgrade()?;
            // Trap 5 rather than a blank row: GTK binds from its own layout, which is not inside
            // any call into this host — but a toolkit's scheduling is a claim, and a claim that
            // failed silently would be a page miniature nobody could account for.
            let host = host
                .try_borrow()
                .inspect_err(|_| {
                    eprintln!(
                        "note: the host was busy, so page {}'s row was drawn without its /Thumb",
                        index.saturating_add(1)
                    );
                })
                .ok()?;
            Some(held.borrow_mut().row(index, || {
                let entry = viewer_host::page_entry(&host.viewer, index);
                viewer_host::Held {
                    label: entry.label,
                    picture: entry.thumbnail.as_ref().and_then(|image| {
                        page::thumbnail(image)
                            .inspect_err(|error| {
                                eprintln!("note: cannot show page {index}'s /Thumb: {error}");
                            })
                            .ok()
                    }),
                }
            }))
        })
    }

    /// §12.3.4: "allowing the user to navigate to a page by clicking its thumbnail image".
    ///
    /// A page index rather than a destination, because a thumbnail *is* the page and there is
    /// nothing to resolve.
    fn show_page_sink(&self) -> Rc<dyn Fn(usize)> {
        let me = self.me.clone();
        Rc::new(move |index| {
            with(&me, |host| {
                host.dispatch(Command::GoTo(PageTarget::Index(index)));
            });
        })
    }

    /// What a tree row does when a person acts on it.
    fn row_sink(&self) -> Rc<dyn Fn(&RowAction)> {
        let me = self.me.clone();
        Rc::new(move |action| {
            let action = action.clone();
            with(&me, |host| match action {
                RowAction::Activate(object) => host.dispatch(Command::Activate(object)),
                RowAction::Toggle { group, on, .. } => {
                    host.dispatch(Command::SetGroup { group, on });
                }
                RowAction::Extract { name } => host.dispatch(Command::Extract { name }),
                RowAction::Inert => {}
            });
        })
    }

    /// §7.6.4.1's prompt, in a window of the platform's own.
    ///
    /// The words are [`viewer_host::password`]'s so that three hosts ask the same question; the
    /// `gtk4::PasswordEntry` is this toolkit's answer to *what a password entry is* and is the
    /// whole of what this method adds.
    fn ask_for_a_password(&mut self, attempt: u32, of: u32) {
        let dialog = gtk4::Window::new();
        dialog.set_title(Some("Password"));
        dialog.set_modal(true);
        dialog.set_transient_for(Some(&self.ui.window));
        dialog.set_default_size(360, -1);
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        column.set_margin_top(12);
        column.set_margin_bottom(12);
        column.set_margin_start(12);
        column.set_margin_end(12);
        let words = viewer_host::password::prompt(&named(&self.path), attempt, of);
        let label = gtk4::Label::new(Some(&words.question));
        label.set_xalign(0.0);
        label.set_wrap(true);
        column.append(&label);
        let entry = gtk4::PasswordEntry::new();
        entry.set_show_peek_icon(true);
        entry.set_activates_default(true);
        column.append(&entry);
        let count = gtk4::Label::new(Some(&words.counted));
        count.set_xalign(0.0);
        count.add_css_class("dim-label");
        column.append(&count);
        let open = gtk4::Button::with_label("Open");
        open.add_css_class("suggested-action");
        column.append(&open);
        dialog.set_child(Some(&column));

        // **Whether the prompt was answered lives beside the dialogue rather than on the host**,
        // and that is not a style choice: `GtkWindow::close` fires `close-request`
        // *synchronously*, so a flag set inside the handler that runs after it is still false when
        // the close handler reads it — which is what happened here first time, and every password
        // supplied was reported as a decline.
        let answered = Rc::new(Cell::new(false));

        let me = self.me.clone();
        let dialogue = dialog.clone();
        let typed = entry.clone();
        let done = Rc::clone(&answered);
        open.connect_clicked(move |_| {
            let password = taken_from(&typed);
            done.set(true);
            dialogue.close();
            with(&me, |host| host.supply_password(password));
        });
        let me = self.me.clone();
        let dialogue = dialog.clone();
        let done = Rc::clone(&answered);
        entry.connect_activate(move |typed| {
            let password = taken_from(typed);
            done.set(true);
            dialogue.close();
            with(&me, |host| host.supply_password(password));
        });
        // Escape is the platform's own way out of a modal window, and closing it without typing has
        // to be a *decline* rather than silence — trap 5, in a window a person walked away from.
        // `close-request` fires for the button and for the key alike, so it answers only where
        // nothing was supplied through the two handlers above.
        let me = self.me.clone();
        dialog.connect_close_request(move |_| {
            if !answered.get() {
                with(&me, |host| host.say(viewer_host::password::CANCELLED));
            }
            glib::Propagation::Proceed
        });
        // **Escape closes it, and this is the toolkit's own gap rather than an extra.** A
        // `GtkDialog` binds that key and is deprecated in the release this crate targets; a plain
        // `GtkWindow` binds nothing, so without this controller the only way out of the prompt is
        // the window manager's close button — and a reader with a keyboard could not decline at
        // all. Driven under `Xvfb`, where there is no window manager to have hidden it: the other
        // two hosts declined on Escape and this one did nothing.
        let dialogue = dialog.clone();
        let keys = gtk4::EventControllerKey::new();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                dialogue.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        dialog.add_controller(keys);
        dialog.present();
    }

    /// §7.6.4.1: a person typed a password, or dismissed the prompt with nothing in it.
    ///
    /// Exhaustive over `Supplied` on purpose, which is what holds three hosts level.
    fn supply_password(&mut self, password: viewer_core::Secret) {
        match viewer_host::password::supplied(password) {
            viewer_host::Supplied::Open(secret) => self.open_document(Some(secret)),
            viewer_host::Supplied::Cancelled => self.say(viewer_host::password::CANCELLED),
        }
    }

    /// §7.5.6's update, written beside the document rather than over it.
    ///
    /// Rule 2 in one method: the core produced the bytes and the host owns the filesystem.
    /// Overwriting somebody's document is a decision this program has not been given.
    fn write_saved(&self, bytes: &[u8]) {
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
    fn write_extracted(&self, asked: Extraction, name: &str, bytes: &[u8]) {
        // §O.2.1's own sentence, decided once for all three hosts in `viewer_host::policy`: a URI
        // that named a file is not a person who asked for one.
        if let Err(refusal) = viewer_host::may_write_extracted(asked) {
            self.say(&refusal);
            return;
        }
        let Some(directory) = self.directory.as_ref() else {
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
    fn say(&self, what: &str) {
        if what.is_empty() {
            return;
        }
        println!("note: {what}");
        self.ui.status.set_text(what);
        // The label ellipsizes at the end, so a long sentence loses its tail — and the longest
        // sentence this window says is the one that names the way out of a refusal, whose tail is
        // the way out. Found by photographing the window this round's own fix was made for, which
        // is trap 1 one layer out from a page. A tooltip is GTK's own idiom for an elided label
        // and costs nothing when the text fits.
        self.ui.status.set_tooltip_text(Some(what));
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

    /// Puts what is selected on the page onto the session's clipboard.
    ///
    /// **The platform end of `doc/todo/30`'s first item, and it is one call** (ADR 0519). This
    /// host draws a selection in the platform's own colour and could not give it to another
    /// application; `gdk::Clipboard` is what a GTK program has for that, reached from any widget
    /// through the display it is on, so there is nothing to keep and nothing to bring up — which
    /// is also why `CLAUDE.md`'s second principle's launch path is untouched by this.
    ///
    /// Which of §14.8.2.5's two orders the text is in is *not* this host's decision and is
    /// [`viewer_host::copied`]: the same question in all three windowed hosts, and the third copy
    /// of it is where two of them would stop agreeing. What is this host's is the two questions
    /// and the toolkit.
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
        self.ui.chrome.clipboard().set_text(&copied.text);
        self.say(&format!(
            "copied {} characters in {}",
            copied.text.chars().count(),
            copied.order
        ));
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
        self.retitle();
        self.restate();
    }

    /// One page's refusals, in the status bar, with the page named.
    ///
    /// **The page is named because under a column it has to be**: `Event::Reported` has always
    /// carried which page it is about, and a host that dropped it was attributing one page's
    /// refusals to whichever page a person happened to be looking at. `None` is what the
    /// *document* says about itself before any page is drawn, and belongs to no page.
    fn reported(&self, page: Option<usize>, notes: &[String]) {
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
    fn restate(&self) {
        let Answer::Reports(pages) = self.viewer.query(Query::Reports) else {
            return;
        };
        self.say(&viewer_host::status::on_screen(&pages));
    }

    /// Puts the caption in the title bar.
    fn retitle(&self) {
        let mark = if self.dirty { "• " } else { "" };
        self.ui.window.set_title(Some(&format!(
            "{mark}{} — {}",
            named(&self.path),
            self.caption
        )));
    }

    /// Table 29's two display entries, and §12.2's chrome flags with them.
    ///
    /// `/PageLayout` is the *viewer's* to apply — it read the catalog when the document opened —
    /// and what this host needs from it is the value `l` cycles from, so that the first press
    /// moves off what the document asked for rather than back onto it. `/PageMode` is "how the
    /// document shall be displayed when opened", which is a panel for three of its six names,
    /// nothing for `UseNone`, and since ADR 0470 a full-screen window for `FullScreen`.
    fn obey_the_catalog(&mut self, queue: &mut VecDeque<Command>) {
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
        // Queued rather than dispatched: this runs inside `react`, and `dispatch` would start a
        // second `pump` under the first. `queue` is what that parameter is for.
        if self.presenting.full_screen() {
            self.start_the_clock();
            queue.push_back(Command::Present(self.presenting.mode()));
        }
        self.show_page_mode(opening.mode);
        self.apply_chrome();
    }

    /// Shows the document the way one of Table 29's six page modes asks for.
    ///
    /// Two callers and one mapping, which is the point: the document opening, where §7.7.2 states
    /// "how the document shall be displayed when opened", and full screen ending, where §12.2's
    /// `/NonFullScreenPageMode` states "how to display the document on exiting full-screen mode".
    ///
    /// Which panel each of Table 29's names opens is [`Tab::of_page_mode`]'s, shared with the
    /// other two hosts, and **`UseThumbs` is no longer among the names that reach none** — this
    /// host reported it as a mode it had no panel for until §12.3.4's list was built.
    /// `FullScreen` cannot arrive from the second caller, because `pdf_model` refuses that name for
    /// `/NonFullScreenPageMode`.
    fn show_page_mode(&mut self, mode: pdf_model::viewer_preferences::PageMode) {
        use pdf_model::viewer_preferences::PageMode;
        if let Some(tab) = Tab::of_page_mode(mode) {
            self.ui.tabs.set_current_page(Some(notebook_page(tab)));
            return;
        }
        if matches!(mode, PageMode::FullScreen) {
            self.say(
                "presenting full screen (§7.7.2's FullScreen page mode) — Escape comes \
                      back",
            );
        }
    }

    /// Puts the window in the state [`viewer_host::Presenting`] says the document asked for.
    ///
    /// Four sentences, four widgets. §12.2's `/HideToolbar` is the header bar's own buttons and
    /// the find bar; `/HideWindowUI` — "scroll bars and navigation controls" — is the status line;
    /// Table 29's "any other window visible" is the notebook of three trees; and full screen is
    /// the window itself, with the titlebar gone because that sentence names the window controls
    /// too. `/HideMenubar` has nothing to name here and is reported when a document asks for it.
    fn apply_chrome(&mut self) {
        let chrome = self.presenting.chrome();
        for button in &self.ui.tool_buttons {
            button.set_visible(chrome.tool_bar);
        }
        if !chrome.tool_bar {
            self.ui.find.set_search_mode(false);
        }
        self.ui.find.set_visible(chrome.tool_bar);
        self.ui.status_bar.set_visible(chrome.window_ui);
        // **Taken out of the splitter rather than hidden**, because a `GtkPaned` carries a
        // `position` that was *set* and a hidden start child leaves that property standing —
        // which is a three-hundred-pixel hole where the panel was, and Table 29 says "no … other
        // window visible" rather than "no other window drawn". A removed child is unambiguous:
        // the end child gets the whole allocation. `Ui::tabs` holds the reference across the
        // unparenting, so the three trees inside it are still there when it goes back.
        //
        // **Two conditions rather than one since ADR 0526**: the clause's permission, and the
        // reader's own wish. `o` moves the second, and composing them here is what makes leaving
        // full screen put back what the *reader* had.
        if chrome.other_windows && self.panel_wanted {
            if self.ui.split.start_child().is_none() {
                self.ui.split.set_start_child(Some(&self.ui.tabs));
                self.ui.split.set_position(PANEL_WIDTH);
            }
        } else if self.ui.split.start_child().is_some() {
            self.ui.split.set_start_child(None::<&gtk4::Widget>);
        }
        if self.presenting.full_screen() {
            self.ui.window.set_decorated(false);
            self.ui.window.fullscreen();
        } else {
            self.ui.window.unfullscreen();
            self.ui.window.set_decorated(true);
        }
    }

    /// Enters or leaves §12.4.4's presentation, which for this program is the full-screen window.
    ///
    /// `p`, the same letter `viewer-ui` and `viewer-qt` bind. That the window and §12.4.4.2's mode
    /// are one act is a documented choice, argued in `viewer_host::presentation`; what crosses the
    /// boundary is `Command::Present`, which has existed since ADR 0316 and needed no change.
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
            if let Some(mode) = self.presenting.on_exit() {
                self.say(&format!("§12.2's /NonFullScreenPageMode asks for {mode:?}"));
                self.show_page_mode(mode);
            } else {
                self.say("presentation stopped");
            }
        }
        self.apply_chrome();
    }

    /// Starts §12.4.4.1's clock, which is what makes this a presentation rather than a big page.
    fn start_the_clock(&mut self) {
        if self.clock.is_none() {
            self.clock = Some(viewer_host::Clock::started(std::time::Instant::now()));
        }
    }

    /// Takes down the pending timer, if there still is one.
    ///
    /// **`SourceId::remove` panics on a source that is already gone** — it unwraps a `Result` — so
    /// the id is looked up before it is used. A one-shot destroys itself when it fires, and the
    /// callback that fires it clears [`Host::armed`]; this is the case where it could not, because
    /// the host was borrowed by a nested main loop (§7.6.4.1's password dialogue runs one).
    fn disarm(&mut self) {
        let Some(armed) = self.armed.take() else {
            return;
        };
        if let Some(source) = glib::MainContext::default().find_source_by_id(&armed) {
            source.destroy();
        }
    }

    /// Stops it, and takes the timer with it.
    ///
    /// **The source is removed rather than left to fire on a `None` clock.** A window that is not
    /// presenting has nothing to advance, and `CLAUDE.md`'s principle 2 makes a wakeup with
    /// nothing behind it a defect rather than a rounding error.
    fn stop_the_clock(&mut self) {
        self.clock = None;
        self.arming = None;
        self.shown = None;
        self.disarm();
    }

    /// Arms the next turn of the clock on GTK's own main loop.
    ///
    /// A one-shot that is re-armed after every turn, which is the shape [`Host::pump_search`]
    /// already uses and which answers what a repeating source cannot: the interval changes when a
    /// transition starts and stops, and a presentation that ends leaves nothing armed at all.
    fn pump_presentation(&mut self) {
        self.disarm();
        let Some(interval) = self.clock.as_ref().map(viewer_host::Clock::interval) else {
            return;
        };
        let me = self.me.clone();
        self.armed = Some(glib::timeout_add_local_once(interval, move || {
            with(&me, |host| {
                host.armed = None;
                host.turn_the_clock();
            });
        }));
    }

    /// One turn of §12.4.4.1's clock: tell the core how much time passed, and draw what is due.
    ///
    /// **A tick that produces no events repaints nothing**, and that is the whole of this host's
    /// answer to a viewer that must idle. A page stating no `/Dur` — "the page shall not advance
    /// automatically" — swallows every tick, so the window wakes ten times a second, adds a
    /// number, and goes back to sleep without touching a pixel. Repainting a still page at that
    /// rate would copy a page's worth of samples into a fresh texture for a picture that has not
    /// changed.
    fn turn_the_clock(&mut self) {
        let now = std::time::Instant::now();
        let animating = self
            .clock
            .as_ref()
            .is_some_and(viewer_host::Clock::animating);
        let Some(millis) = self.clock.as_mut().and_then(|clock| clock.tick(now)) else {
            // Held: a transition is being drawn, and §12.4.4.1's EXAMPLE puts that before the
            // page is displayed. What is due is a frame.
            if animating {
                self.refresh();
            }
            self.pump_presentation();
            return;
        };
        let events: Vec<Event> = self.viewer.handle(Command::Tick { millis }).collect();
        if events.is_empty() {
            self.pump_presentation();
            return;
        }
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue);
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

    /// Takes `transition` to be drawn when the page it moves *to* arrives, or says why it is not.
    ///
    /// Armed rather than begun: `Viewer::handle` settles after the command that turned the page,
    /// so the events arrive as page change, transition, render request, and the arriving page's
    /// list is in the last of the three. §12.4.4.1's transition is one *to* a page, so waiting for
    /// that page's own request is the clause's order as well as this host's.
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
            // The core has already said *why* through `Event::Reported`; a second sentence here
            // would say it twice.
            return;
        }
        self.arming = Some(transition);
    }

    /// Keeps the page just rendered as a transition's face, and begins one that was armed.
    ///
    /// Two whole-viewport rasterisations happen here and none per frame: the page being left and
    /// the page arriving, each drawn where a frame will place it. A transition that re-rasterised
    /// per frame would pay a page's interpretation sixty times a second for the length of it.
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

    /// The frame of a transition in flight, as one texture filling the viewport.
    ///
    /// `None` where there is nothing being drawn, and then the window shows the page — which is
    /// the transition's own end state, so the two answers meet without a seam.
    fn transition_placement(&mut self) -> Option<(gtk4::gdk::MemoryTexture, (f32, f32), usize)> {
        let viewport = self.viewport_rect();
        let now = std::time::Instant::now();
        let shaped = self.clock.as_mut().map(|clock| clock.frame(viewport, now));
        let list = match shaped {
            Some(Ok(Some(list))) => list,
            Some(Err(problem)) => {
                // Not reachable from a frame — the largest one adds four clips — and said rather
                // than swallowed for the reason every refusal in this tree is.
                self.say(&format!("transition: this frame would not draw: {problem}"));
                return None;
            }
            None | Some(Ok(None)) => return None,
        };
        let target = pdf_render::TargetSpec {
            width: self.viewport.0,
            height: self.viewport.1,
            transform: pdf_render::Transform::IDENTITY,
        };
        let raster = match self.rasterizer.rasterize(&list, target) {
            Ok(raster) => raster,
            Err(error) => {
                self.say(&format!(
                    "transition: this frame would not rasterise: {error}"
                ));
                return None;
            }
        };
        match page::texture(&raster) {
            Ok(texture) => Some((texture, (0.0, 0.0), raster.data.len())),
            Err(error) => {
                eprintln!("note: {error}");
                None
            }
        }
    }

    /// What a key press means, which is [`viewer_host::keys`]'s answer and not this host's.
    ///
    /// **Only the translation is GTK's** (ADR 0526). This host used to carry a table of its own
    /// and it disagreed with the other two about the arrow keys, about `f` and about Escape; what
    /// is left here is [`key_pressed`] turning a `gdk::Key` into a [`viewer_host::Key`] and
    /// [`Host::window_act`] doing the half of the table that is a widget's rather than a message.
    fn key(&mut self, key: gtk4::gdk::Key, shift: bool) {
        let Some(stated) = key_pressed(key) else {
            return;
        };
        let mode = if self.presenting.full_screen() {
            viewer_host::Mode::Presenting
        } else {
            viewer_host::Mode::Reading
        };
        let Some(meaning) = viewer_host::meaning(stated, shift, mode) else {
            return;
        };
        match meaning {
            viewer_host::Meaning::Send(command) => {
                // §12.5.6.10's markups are defined over selected text, so a press with nothing
                // selected asks for an annotation over nothing. The core answers by doing
                // nothing, which is right and silent — and trap 5 is that a person who pressed a
                // key and saw no change has been told nothing at all.
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
    /// Matched exhaustively and with no catch-all arm, which is `doc/ui-boundary.md`'s rule
    /// applied one layer out: a binding added to [`viewer_host::keys`] fails to compile in all
    /// three hosts, so "the hosts stay level" is checked rather than agreed to.
    fn window_act(&mut self, act: viewer_host::WindowAct) {
        match act {
            // GTK draws in logical pixels and `Command::Scroll` speaks device ones, which is why
            // the table states a distance and not the message: the same conversion `scrolled`
            // does for a wheel notch.
            viewer_host::WindowAct::ScrollBy(by) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a step in logical pixels times a display scale, which is tens"
                )]
                let dy = (f64::from(by) * f64::from(self.scale)) as f32;
                self.dispatch(Command::Scroll { dx: 0.0, dy });
            }
            // §14.8.2.5 leaving the program, which needed no message at all (ADR 0519). A `c`
            // reaching here has already passed the find bar, and a `c` typed into a §12.7 control
            // never reaches a window-level controller because the widget has the focus.
            viewer_host::WindowAct::Copy => self.copy_selection(),
            // The find bar is revealed by a key this host binds rather than by
            // `gtk_search_bar_set_key_capture_widget`, which forwards *every* letter to the entry
            // and would take `a`, `s`, `z` and `y` away from the rest of the table.
            viewer_host::WindowAct::Find => self.ui.find.set_search_mode(true),
            // Table 29's "any other window visible" is a *permission*, and this is the reader's
            // wish beside it: the panel is on the screen when the clause allows one and the
            // person asked for one. `apply_chrome` composes the two.
            viewer_host::WindowAct::Panel => {
                self.panel_wanted = !self.panel_wanted;
                self.apply_chrome();
            }
            viewer_host::WindowAct::Notices => self.show_notices(),
            viewer_host::WindowAct::Present | viewer_host::WindowAct::LeaveFullScreen => {
                self.present_or_stop();
            }
            // Table 29's six arrangements, in the order that table states them. A key rather
            // than a menu because this host has no menu bar; what matters for `doc/todo/30` is
            // that the *message* is exercised by a person driving a real window.
            viewer_host::WindowAct::NextLayout => {
                self.layout = next_layout(self.layout);
                self.dispatch(Command::Layout(self.layout));
                self.say(&format!("page layout: {:?} (§7.7.2)", self.layout));
                // A new arrangement is a new set of pages on the screen, and therefore a new set
                // of things they could not draw.
                self.restate();
            }
            // ADR 0245's third decision, made with the messages that already exist: magnify until
            // every platform control fits the `/Rect` the document states for it.
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
            // **Refused by name rather than ignored**, which is trap 5 and is the honest answer
            // here: §12.5.6.6's annotation is authored by dragging a rectangle and then typing
            // into it, and this host has neither the drag mode nor the editor. `doc/todo/30`
            // carries it as the remaining asymmetry rather than leaving it silent.
            viewer_host::WindowAct::FreeText => self.say(
                "this host cannot draw a §12.5.6.6 free text annotation yet — the drag mode and \
                 its editor are viewer-ui's alone (doc/todo/30)",
            ),
        }
    }

    /// The third-party notices this binary is obliged to carry, in a window of their own.
    ///
    /// **A licence obligation with a surface, and this host had neither half of it until the
    /// six-hundred-and-eighty-seventh session**: `pdf-font` compiles the standard 14 font programs
    /// into every binary in this tree, both of their licences require a binary distribution to
    /// reproduce their notices, and `pdf-viewer-gtk` reproduced them nowhere at all. The text is
    /// [`viewer_host::NOTICE`], shared with the other two hosts because a notice that differs
    /// between two binaries of one program is two claims about one obligation.
    ///
    /// Set in a monospace font and **not re-wrapped**: a BSD licence's paragraphs and a font
    /// list's columns are laid out by the file's own line breaks, and re-flowing text this program
    /// is obliged to reproduce would be editing it.
    fn show_notices(&self) {
        let text = gtk4::TextView::new();
        text.set_editable(false);
        text.set_cursor_visible(false);
        text.set_monospace(true);
        text.set_left_margin(12);
        text.set_right_margin(12);
        text.set_top_margin(12);
        text.set_bottom_margin(12);
        text.buffer().set_text(viewer_host::NOTICE);
        let scroller = gtk4::ScrolledWindow::new();
        scroller.set_child(Some(&text));
        let window = gtk4::Window::builder()
            .title("Third-party notices")
            .transient_for(&self.ui.window)
            .modal(true)
            .default_width(760)
            .default_height(620)
            .child(&scroller)
            .build();
        window.present();
    }

    /// One notch of the wheel, in whichever direction it turned.
    ///
    /// The deltas GTK reports for a discrete device are notches rather than pixels, so the
    /// distance is this host's choice and `SCROLL_STEP` is where it is written down. Device
    /// pixels out, because that is what `Command::Scroll` speaks.
    fn scrolled(&mut self, dx: f64, dy: f64) {
        let step = f64::from(self.scale) * SCROLL_STEP;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a notch count times a step in pixels, which is tens"
        )]
        let (dx, dy) = ((dx * step) as f32, (dy * step) as f32);
        self.dispatch(Command::Scroll { dx, dy });
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

    /// The find bar's string changed: highlight what is on this page, and look no further.
    ///
    /// Deliberately **not** a `Command::Find` per keystroke. Typing changes what is highlighted on
    /// the page — which is a query over a readback that already exists — and a search across the
    /// document is what pressing Enter asks for. A host that started one on every character would
    /// be interpreting pages while a person was still deciding what to look for.
    fn retype(&mut self, needle: String) {
        self.needle = needle;
        self.refresh();
    }

    /// Enter, Ctrl+G and Ctrl+Shift+G: the next occurrence anywhere in the document.
    fn find(&mut self, backward: bool) {
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

    /// The bar was revealed or closed. Closing it forgets the plan and the highlights.
    fn find_bar_shown(&mut self, shown: bool) {
        if shown {
            self.ui.find_entry.grab_focus();
            return;
        }
        self.needle.clear();
        self.pages_left = 0;
        self.dispatch(Command::Find(Find::Stop));
    }

    /// One more page of the search, scheduled on GTK's idle queue.
    ///
    /// **The whole reason a step is one page.** `viewer-core` has no thread to read a thousand
    /// pages on (rule 4) and this host must not block the main loop for the 5.84 s that would
    /// cost, so each step is posted back through `glib::idle_add_local_once`: the window keeps
    /// repainting, the status line counts down, and the search finishes when it finishes.
    fn pump_search(&mut self) {
        if self.pages_left == 0 {
            return;
        }
        let me = self.me.clone();
        glib::idle_add_local_once(move || {
            with(&me, |host| host.dispatch(Command::Find(Find::Continue)));
        });
    }

    /// The pointer, in logical pixels of the overlay.
    fn pointer(&mut self, x: f64, y: f64, action: PointerAction) {
        let scale = f64::from(self.scale);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a pointer position inside a window is far inside f32's exact integer range"
        )]
        let at = ((x * scale) as f32, (y * scale) as f32);
        self.dispatch(Command::Pointer { at, action });
        self.show_whether_it_is_a_link(at);
    }

    /// §12.5.6.5's activation region under the pointer, as this platform's cursor.
    ///
    /// The clause makes a link annotation "either a hypertext link to a destination elsewhere in
    /// the document or an action to be performed", and states nothing at all about a pointer — so
    /// what a reader sees over one is a **convention**, and this is GTK's name for it. It is
    /// nevertheless the difference between a page whose links can be found and one where they can
    /// only be discovered by clicking, which is why `viewer-ui` has had it since ADR 0166 and why
    /// two windows of three having it was a debt rather than a platform difference.
    ///
    /// Asked at pointer speed, which is what makes `Query::LinkAt` a query rather than a command —
    /// and set only when the answer *changes*, because `gdk_surface_set_cursor` reaches the
    /// display server and a pointer sweeping a page of links would reach it on every motion event.
    fn show_whether_it_is_a_link(&mut self, at: (f32, f32)) {
        let Answer::Link(over) = self.viewer.query(Query::LinkAt(at)) else {
            return;
        };
        if over == self.over_link {
            return;
        }
        self.over_link = over;
        self.trace.say(
            Topic::Pointer,
            format_args!(
                "the pointer is {} §12.5.6.5's activation region",
                if over { "over" } else { "off" }
            ),
        );
        // The layer the pages are in rather than the toplevel: GTK takes the cursor from the
        // widget the pointer is *picked* on, so setting it on the window would override the text
        // cursor a `GtkEntry` over a §12.7 widget sets for itself, and setting it on the chrome
        // layer would set it on a widget `set_can_target(false)` means is never picked.
        self.ui
            .fixed
            .set_cursor_from_name(over.then_some("pointer"));
    }
}

/// What to call the document in a title bar: its file name, or the whole path where it has none.
fn named(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// §7.6.4.1's password out of the entry it was typed into, and out of the entry.
///
/// The widget is emptied on the way past. GTK's own buffer is not this program's to clear — a
/// `GtkPasswordEntry` holds a `GtkEntryBuffer` whose storage is glib's — so what this buys is that
/// the *live* widget stops holding it, which is the part a host can reach; the rest is
/// [`viewer_core::Secret`]'s and is documented there as best effort.
fn taken_from(entry: &gtk4::PasswordEntry) -> viewer_core::Secret {
    let password = viewer_core::Secret::from(entry.text().to_string());
    entry.set_text("");
    password
}

/// Runs `what` against the host, or says why it could not.
///
/// A callback that arrives while the host is already borrowed is a callback GTK raised from
/// inside one of this host's own writes — dropping it is right, and saying so is trap 5.
pub(crate) fn with(me: &Weak<RefCell<Host>>, what: impl FnOnce(&mut Host)) {
    let Some(host) = me.upgrade() else {
        return;
    };
    match host.try_borrow_mut() {
        Ok(mut host) => what(&mut host),
        Err(_) => eprintln!("note: the host was busy and an action was dropped"),
    }
}

/// The value a field now holds, written back into its control.
///
/// ADR 0201: a host keeps the *point* it clicked and never the text, because §12.7.5.3's
/// truncation means the field can take less than was typed. So the control shows what the field
/// took — except where the answer says the string is not the field's characters, which is Table
/// 231 bit 14's password field: writing that back would replace what a person typed with a row of
/// dots and send those as the next value.
///
/// **The refusal is on the answer rather than on the control**, since ADR 0247. It used to be the
/// `password: false` in the pattern below, which is this host inferring from the control's kind
/// what the answer now states; `ShownValue::obscured` is the statement, and a control kind that
/// stopped agreeing with it would have been a silent bug in exactly the place this one was found.
///
/// **§12.7.5.2's two toggling kinds arrived here in the seven-hundred-and-thirty-fifth session**
/// (ADR 0630), and their absence was a defect this host had shipped since ADR 0244 and `viewer-qt`
/// never had: a `GtkCheckButton`'s state was set when the control was *built* and never again. So
/// a value the field acquired any other way — an undo, an imported §12.7.8 data set, a click an
/// assistive technology asked for, or the other button of a radio set going on — left the picture
/// saying one thing and `/V` another. §12.7.5.2.4 is what makes the last of those a clause rather
/// than an untidiness: with `RadiosInUnison` clear, "at most one radio button in a field shall be
/// set at a time", and two of this host's buttons showed a tick together.
fn write_back(placed: &Placed, field: &FormField, widget: &viewer_core::FormWidget) {
    if let Some(button) = placed.widget.downcast_ref::<gtk4::CheckButton>() {
        // The same expression `controls::toggle` builds the button with, re-derived from what
        // `Query::Fields` says *now*: a widget is on when the field's value names its appearance
        // state, and a widget whose `/AP` names no on state at all falls back to the field's own.
        let on = widget.on
            || (matches!(
                viewer_host::control_kind(&field.control),
                viewer_host::form::ControlKind::Check { on: true }
                    | viewer_host::form::ControlKind::Radio { on: true, .. }
            ) && widget.on_state.is_none());
        if button.is_active() != on {
            button.set_active(on);
        }
        return;
    }
    let Some(shown) = field.value.as_ref() else {
        return;
    };
    if shown.obscured {
        return;
    }
    let value = shown.text.as_str();
    match &placed.kind {
        viewer_host::form::ControlKind::Entry {
            multiline: false, ..
        } => {
            if let Some(entry) = placed.widget.downcast_ref::<gtk4::Entry>()
                && entry.text() != value
            {
                entry.set_text(value);
            }
        }
        viewer_host::form::ControlKind::Entry {
            multiline: true, ..
        } => {
            if let Some(scroller) = placed.widget.downcast_ref::<gtk4::ScrolledWindow>()
                && let Some(view) = scroller.child().and_downcast::<gtk4::TextView>()
            {
                let buffer = view.buffer();
                let (start, end) = buffer.bounds();
                if buffer.text(&start, &end, false) != value {
                    buffer.set_text(value);
                }
            }
        }
        // Table 233 bit 19 set: the text box half of `controls::editable_combo`, which is where a
        // row picked out of the drop-down arrives — the list sends a position and the field
        // resolves it to Table 234's name string, so the entry shows what the *field* holds rather
        // than what this host guessed it would.
        viewer_host::form::ControlKind::Combo { editable: true, .. } => {
            if let Some(entry) = placed
                .widget
                .downcast_ref::<gtk4::Box>()
                .and_then(WidgetExt::first_child)
                .and_downcast::<gtk4::Entry>()
                && entry.text() != value
            {
                entry.set_text(value);
            }
        }
        _ => {}
    }
}

/// One of §12.5.6.14's popup windows, as the widgets GTK draws it with.
///
/// The clause gives a popup "no appearance stream", so there is nothing on the page to show and a
/// window is furniture — which is why `viewer-core` answers a rectangle and three strings and why
/// this is a real [`gtk4::Frame`] rather than a rectangle painted on the chrome layer. What is
/// this host's is only the *look*: the border, the fonts and the two style classes below.
/// The three texts and the box are `viewer_host::popup`'s, shared with the other two hosts.
fn popup_window(window: &viewer_host::Window<'_>) -> gtk4::Frame {
    let frame = gtk4::Frame::new(None);
    // §12.5.6.14: a popup has "no appearance stream or associated actions of its own", so there is
    // nothing on it to activate — and a widget over the page that swallowed a press would take the
    // selection, the link and the form control underneath it away from the reader.
    frame.set_can_target(false);
    frame.set_can_focus(false);
    // A window is the rectangle the *document* states, so text that does not fit stops at its edge
    // rather than growing it. The drawn host stops at the same place for the same reason.
    frame.set_overflow(gtk4::Overflow::Hidden);

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let bar = gtk4::Box::new(gtk4::Orientation::Horizontal, POPUP_PADDING);
    bar.set_margin_start(POPUP_PADDING);
    bar.set_margin_end(POPUP_PADDING);
    // §12.5.6.2's `/T`: "[t]he text label that shall be displayed in the title bar of the
    // annotation's popup window when open and active."
    let title = gtk4::Label::new(Some(window.title));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title.add_css_class("heading");
    bar.append(&title);
    // Table 166's `/M`, in the one format `viewer_host::stamp` gives every date this program
    // shows, so that a popup and §14.3.3's panel do not spell one clause's answer two ways.
    if let Some(when) = viewer_host::popup::modified(window) {
        let stamp = gtk4::Label::new(Some(&when));
        stamp.add_css_class("dim-label");
        bar.append(&stamp);
    }
    // Table 166's `/C` is "[t]he title bar of the annotation's popup window", so the colour is
    // painted *behind* the bar's two labels rather than applied to their text. A drawing area
    // under an overlay, because GTK4 has no per-widget background short of a style sheet and a
    // provider per window would be a document's colour reaching into this program's CSS.
    let painted = gtk4::Overlay::new();
    if let Some(colour) = window.colour {
        let ground = gtk4::DrawingArea::new();
        ground.set_draw_func(move |_, cr, _, _| {
            cr.set_source_rgb(
                f64::from(colour.r),
                f64::from(colour.g),
                f64::from(colour.b),
            );
            if let Err(error) = cr.paint() {
                eprintln!("note: cannot draw a popup window's title bar: {error}");
            }
        });
        painted.set_child(Some(&ground));
    }
    painted.add_overlay(&bar);
    // **A `GtkOverlay` measures its main child and not its overlays**, and the main child here is
    // a drawing area with no natural size at all — so without this the title bar was allocated no
    // height and the note's first line was drawn through the author's name. Photographed, not
    // reasoned: the first screenshot of this window had the two on top of each other.
    painted.set_measure_overlay(&bar, true);
    column.append(&painted);

    // Table 166's `/Contents`: the text in the window. Wrapped by Pango, which is the whole reason
    // a native host places a label here instead of breaking lines for itself.
    let note = gtk4::Label::new(Some(window.text));
    note.set_xalign(0.0);
    note.set_yalign(0.0);
    note.set_wrap(true);
    note.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    note.set_vexpand(true);
    note.set_margin_start(POPUP_PADDING);
    note.set_margin_end(POPUP_PADDING);
    note.set_margin_top(POPUP_PADDING);
    column.append(&note);

    frame.set_child(Some(&column));
    frame
}

/// Device pixels as the logical ones GTK lays out in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a widget extent inside a window is far inside i32's range, and this is the size \
              request GTK itself takes as an i32"
)]
fn logical(device: f64, scale: f64) -> i32 {
    (device / scale).round().max(1.0) as i32
}

/// Paints [`pdf_render::SURROUND`] behind the pages, so that a reader can see a page's edge.
///
/// **This is not §11.4.7's 𝑊 and must never be confused with it.** The page's own colour is the
/// standard's, is white, and is imposed by `render-cpu` inside the page's boundary; what lies
/// *outside* every page is not any clause's subject and is this program's documented choice —
/// `pdf_render::medium` has both readings. The colour is taken from there rather than restated
/// here so that all three hosts and all three rasterisers say one thing.
///
/// **Why a native host does not simply take the toolkit's window background, which is what this
/// crate did until the six-hundred-and-eleventh session.** It sounds like the native answer and
/// it is not one: GTK's default background under Adwaita is within a few levels of white, so a
/// continuous column drew white paper on almost-white ground and the gap between two pages was
/// as good as invisible — measured on the screen, not assumed. The toolkit has no notion of
/// "the surface a document is laid on", so there is no platform value to inherit; picking one is
/// the application's job either way, and picking the same one in all three hosts is
/// `doc/todo/30`'s standing decision that the hosts stay level.
///
/// A failure to load the rule is reported and nothing else: a window whose ground is the
/// toolkit's default still shows every page, so refusing to open would be a worse answer than
/// the faint boundary this exists to sharpen.
fn surround(widget: &impl IsA<gtk4::Widget>) {
    /// The class the rule below names, and the only widget that carries it.
    const CLASS: &str = "pdf-page-surround";
    let level = |component: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a colour component in 0..=1 scaled by 255"
        )]
        {
            (component.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
        }
    };
    let colour = pdf_render::SURROUND;
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(&format!(
        ".{CLASS} {{ background-color: rgb({}, {}, {}); }}",
        level(colour.r),
        level(colour.g),
        level(colour.b)
    ));
    let Some(display) = gtk4::gdk::Display::default() else {
        eprintln!("note: no display to style the page's surround on; it stays the toolkit's");
        return;
    };
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    widget.as_ref().add_css_class(CLASS);
}

/// Everything the window is made of, and the callbacks that reach the host from it.
fn build_window(
    app: &gtk4::Application,
    path: &Path,
    me: &Weak<RefCell<Host>>,
    chrome_state: &Rc<RefCell<Chrome>>,
) -> Ui {
    let window = gtk4::ApplicationWindow::new(app);
    window.set_default_size(1000, 1100);
    window.set_title(Some(&named(path)));

    let (fixed, popups, chrome, overlay) = page_area(chrome_state);

    // One notebook page per `viewer_host::Tab`, in that list's own order and with that list's own
    // wording — six panels rather than the three this host drew until `doc/todo/30`'s item 4, and
    // a seventh added there appears here without a line changing.
    let tabs = gtk4::Notebook::new();
    // **Down the side rather than across the top**, and the screen is what decided it: six labels
    // do not fit across a sidebar, and a `GtkNotebook` that cannot fit its tabs puts the rest
    // behind scroll arrows — so four of `viewer_host::Tab`'s six panels were reachable only by
    // pressing an arrow nobody would look for. A vertical strip holds all six at any width this
    // panel would sensibly have. `set_scrollable` stays on regardless, because a theme with a
    // taller font is nobody's to predict.
    tabs.set_tab_pos(gtk4::PositionType::Left);
    tabs.set_scrollable(true);
    let slots: Vec<gtk4::Box> = Tab::ALL
        .iter()
        .map(|tab| {
            let slot = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            tabs.append_page(&slot, Some(&gtk4::Label::new(Some(tab.label()))));
            slot
        })
        .collect();
    tabs.set_size_request(PANEL_WIDTH - 20, -1);

    let split = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    split.set_start_child(Some(&tabs));
    split.set_end_child(Some(&overlay));
    split.set_position(PANEL_WIDTH);
    split.set_resize_start_child(false);

    let status = gtk4::Label::new(None);
    status.set_xalign(0.0);
    status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    status.set_margin_start(6);
    status.set_margin_end(6);
    status.set_margin_top(3);
    status.set_margin_bottom(3);

    let (find, find_entry) = find_bar();

    // The separator and the label in one box, because §12.2's `/HideWindowUI` names "scroll bars
    // and navigation controls" as one thing to hide and a rule that left a hairline behind would
    // be obeying half of it.
    let status_bar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    status_bar.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    status_bar.append(&status);

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    column.append(&find);
    column.append(&split);
    column.append(&status_bar);
    split.set_vexpand(true);
    window.set_child(Some(&column));
    let (bar, tool_buttons) = header(me);
    window.set_titlebar(Some(&bar));

    // Three signals, and each is a different question a find bar answers. Typing changes what is
    // highlighted *on this page*, which is a query and costs nothing; activating asks the core for
    // the next occurrence anywhere; and `GtkSearchEntry`'s own next/previous — Ctrl+G and
    // Ctrl+Shift+G — are the same command in each direction.
    let listener = me.clone();
    find_entry.connect_search_changed(move |entry| {
        let typed = entry.text().to_string();
        with(&listener, |host| host.retype(typed));
    });
    let listener = me.clone();
    find_entry.connect_activate(move |_| {
        with(&listener, |host| host.find(false));
    });
    let listener = me.clone();
    find_entry.connect_next_match(move |_| {
        with(&listener, |host| host.find(false));
    });
    let listener = me.clone();
    find_entry.connect_previous_match(move |_| {
        with(&listener, |host| host.find(true));
    });
    let listener = me.clone();
    find.connect_search_mode_enabled_notify(move |bar| {
        // **On the idle queue, not here.** This signal fires *synchronously* from
        // `set_search_mode`, which `Host::key` calls while it is holding the host's `RefCell` —
        // so calling in would find it borrowed and `with` would print "the host was busy and an
        // action was dropped", which is what the first run under `Xvfb` printed. Deferring lets
        // the key handler unwind first, and is the same move `viewer-qt` makes with
        // `QTimer::singleShot` for `QDialog::exec`.
        let shown = bar.is_search_mode();
        let listener = listener.clone();
        glib::idle_add_local_once(move || {
            with(&listener, |host| host.find_bar_shown(shown));
        });
    });

    listen(&window, &overlay, &chrome, me);

    window.present();

    Ui {
        window,
        fixed,
        popups,
        pictures: Vec::new(),
        chrome,
        slots,
        tabs,
        split,
        tool_buttons,
        status_bar,
        status,
        find,
        find_entry,
    }
}

/// The four widgets the page is drawn in, and the order they are stacked in.
///
/// Its own function because it is one decision — *what is over what* — and stating it beside the
/// notebook, the header bar and the status bar buried it. Bottom to top: the page's own pictures
/// and §12.7's controls, §12.5.6.14's windows, and the layer the selection and §12.5.1's ring are
/// drawn on.
fn page_area(
    chrome_state: &Rc<RefCell<Chrome>>,
) -> (gtk4::Fixed, gtk4::Fixed, gtk4::DrawingArea, gtk4::Overlay) {
    let fixed = gtk4::Fixed::new();
    fixed.set_overflow(gtk4::Overflow::Hidden);
    surround(&fixed);

    let chrome = gtk4::DrawingArea::new();
    // The chrome layer is over the page and over the controls, so it must not take a click that
    // belongs to either: this is what makes a `GtkEntry` under it still receive the keyboard.
    chrome.set_can_target(false);
    chrome.set_hexpand(true);
    chrome.set_vexpand(true);
    let state = Rc::clone(chrome_state);
    chrome.set_draw_func(move |area, cr, _, _| {
        if let Err(error) = draw_chrome(area, cr, &state.borrow()) {
            eprintln!("note: cannot draw the chrome: {error}");
        }
    });

    // §12.5.6.14's windows go in a layer of their own, and **not** in the `GtkFixed` the page and
    // the form controls are in. A `GtkFixed` measures the union of its children, so a popup whose
    // `/Rect` the document put *beside* the page — which is where every one of `issue14438.pdf`'s
    // six is — asks the `GtkPaned` for more room, which widens the viewport, which moves the
    // window further out, which asks for more room again: measured, the page area walked from 509
    // to 1229 device pixels in nine frames before the geometric series ran out. An overlay child
    // is not measured by `GtkOverlay` unless `set_measure_overlay` says so, and it is allocated
    // the overlay's own size, so a window outside the viewport is clipped instead of moving it.
    let popups = gtk4::Fixed::new();
    popups.set_can_target(false);
    popups.set_overflow(gtk4::Overflow::Hidden);

    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&fixed));
    // Under the chrome layer: a selection, a match and §12.5.1's ring are marks *on the page*, and
    // a window that hid them would be furniture eating the document. The window is opaque either
    // way, so what this decides is only what happens where the two meet.
    overlay.add_overlay(&popups);
    overlay.add_overlay(&chrome);
    overlay.set_overflow(gtk4::Overflow::Hidden);
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);

    (fixed, popups, chrome, overlay)
}

/// Everything a person does to the window, in the vocabulary the viewer takes.
///
/// Separated from building the widgets because the two are different kinds of decision: which
/// widgets exist is a layout, and what a key or a drag *means* is this project's reading of
/// §12.5.5's pointer and §12.5.1's focus.
fn listen(
    window: &gtk4::ApplicationWindow,
    overlay: &gtk4::Overlay,
    chrome: &gtk4::DrawingArea,
    me: &Weak<RefCell<Host>>,
) {
    let listener = me.clone();
    chrome.connect_resize(move |_, width, height| {
        with(&listener, |host| host.resized(width, height));
    });

    let keys = gtk4::EventControllerKey::new();
    let listener = me.clone();
    // The modifier state is read because §12.5.1's tab key needs a direction and Shift is the
    // only thing that separates the two; no other row of `viewer_host::keys` looks at it.
    keys.connect_key_pressed(move |_, key, _, held| {
        let shift = held.contains(gtk4::gdk::ModifierType::SHIFT_MASK);
        with(&listener, |host| host.key(key, shift));
        glib::Propagation::Proceed
    });
    window.add_controller(keys);

    // **The wheel, which this host had no binding for until Table 29's `/PageLayout` was
    // obeyed.** Under `SinglePage` at `Zoom::FitPage` there is nothing to scroll, so the gap was
    // invisible; a continuous arrangement is a thing a person moves through and a viewer that
    // could not would be offering an arrangement it cannot use. GTK reports a discrete wheel in
    // notches, so a notch is a distance this host chooses — see `SCROLL_STEP`.
    let scrolling = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::BOTH_AXES | gtk4::EventControllerScrollFlags::DISCRETE,
    );
    let listener = me.clone();
    scrolling.connect_scroll(move |_, dx, dy| {
        with(&listener, |host| host.scrolled(dx, dy));
        glib::Propagation::Stop
    });
    overlay.add_controller(scrolling);

    // §12.5.5's three appearances follow the pointer, and §12.5.6.19's `/H` with them — which is
    // why a move with no button down is a command and not only a cursor question.
    let motion = gtk4::EventControllerMotion::new();
    let listener = me.clone();
    motion.connect_motion(move |_, x, y| {
        with(&listener, |host| host.pointer(x, y, PointerAction::Moved));
    });
    overlay.add_controller(motion);

    let drag = gtk4::GestureDrag::new();
    let listener = me.clone();
    drag.connect_drag_begin(move |_, x, y| {
        with(&listener, |host| host.pointer(x, y, PointerAction::Pressed));
    });
    let listener = me.clone();
    drag.connect_drag_update(move |gesture, dx, dy| {
        let Some((x, y)) = gesture.start_point() else {
            return;
        };
        with(&listener, |host| {
            host.pointer(x + dx, y + dy, PointerAction::Dragged);
        });
    });
    let listener = me.clone();
    drag.connect_drag_end(move |gesture, dx, dy| {
        let Some((x, y)) = gesture.start_point() else {
            return;
        };
        with(&listener, |host| {
            host.pointer(x + dx, y + dy, PointerAction::Released);
        });
    });
    overlay.add_controller(drag);
}

/// The find bar, and it is somebody else's widget: a `GtkSearchBar` with a `GtkSearchEntry` in it,
/// so Ctrl+F, Escape, the clear icon and Ctrl+G all behave the way they do in every other GTK
/// application.
///
/// Nothing about it is drawn by this program — which is `doc/ui-boundary.md`'s whole argument,
/// applied to a find bar: what crosses from the core is the geometry of the matches and the
/// vocabulary of the search, and the *bar* is the platform's.
fn find_bar() -> (gtk4::SearchBar, gtk4::SearchEntry) {
    let entry = gtk4::SearchEntry::new();
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some("Find in document"));
    let bar = gtk4::SearchBar::new();
    bar.set_child(Some(&entry));
    bar.connect_entry(&entry);
    bar.set_show_close_button(true);
    (bar, entry)
}

/// The title bar's own buttons, and the buttons apart from it.
///
/// The pair rather than the bar alone, because §12.2's `/HideToolbar` and Table 29's "window
/// controls" name different things and GTK4 puts both in one widget: the buttons are this host's
/// tool bar and the bar itself carries the close button, so a document asking for no tool bar
/// loses the four buttons and keeps its way out of the window.
fn header(me: &Weak<RefCell<Host>>) -> (gtk4::HeaderBar, Vec<gtk4::Button>) {
    let bar = gtk4::HeaderBar::new();
    let mut buttons = Vec::new();
    for (label, target) in [("‹", PageTarget::Previous), ("›", PageTarget::Next)] {
        let button = gtk4::Button::with_label(label);
        let listener = me.clone();
        button.connect_clicked(move |_| {
            with(&listener, |host| host.dispatch(Command::GoTo(target)));
        });
        bar.pack_start(&button);
        buttons.push(button);
    }
    for (label, zoom) in [("−", Zoom::Out), ("+", Zoom::In)] {
        let button = gtk4::Button::with_label(label);
        let listener = me.clone();
        button.connect_clicked(move |_| {
            with(&listener, |host| {
                host.dispatch(Command::Zoom { zoom, at: None });
            });
        });
        bar.pack_end(&button);
        buttons.push(button);
    }
    (bar, buttons)
}

/// The interactive chrome, in the platform's own colour.
///
/// `doc/ui-boundary.md`: "[e]mitting them as quads and points lets a native host draw selection in
/// **macOS's selection colour, KDE's accent, the Windows highlight brush**". The colour here is
/// [`gtk4::prelude::WidgetExt::color`] — the theme's own foreground at this widget, which follows
/// a light or dark theme without this program knowing which is on. It is deliberately *not* the
/// desktop's accent colour, because GTK 4.22 exposes none to application code: there is no
/// `gtk_widget_get_accent_color`, and `@accent_bg_color` is a CSS name libadwaita defines, which
/// is a dependency this crate did not take. ADR 0244.
fn draw_chrome(
    area: &gtk4::DrawingArea,
    cr: &gtk4::cairo::Context,
    chrome: &Chrome,
) -> Result<(), gtk4::cairo::Error> {
    let colour = area.color();
    let scale = if chrome.scale > 0.0 {
        chrome.scale
    } else {
        1.0
    };
    if !chrome.highlights.is_empty() {
        // Annex O's rectangle, under both of the others: it says how the document was *opened*,
        // where a match and a selection say what is happening now. Fainter still, for the same
        // reason the matches are fainter than the selection.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.18,
        );
        for quad in &chrome.highlights {
            trace_quad(cr, *quad, scale);
        }
        cr.fill()?;
    }
    if !chrome.matches.is_empty() {
        // Fainter than the selection and in the same colour, which is this platform's answer to
        // "what does a match look like": GTK exposes no accent colour to application code, so
        // there is nothing else honest to draw it in. The alpha is what distinguishes *a* match
        // from *the* match a person is on.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.12,
        );
        for quad in &chrome.matches {
            trace_quad(cr, *quad, scale);
        }
        cr.fill()?;
    }
    if !chrome.selection.is_empty() {
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.25,
        );
        for quad in &chrome.selection {
            trace_quad(cr, *quad, scale);
        }
        cr.fill()?;
    }
    if let Some(quad) = chrome.focus {
        // §12.5.1: an annotation with the input focus. What a focus ring looks like is the
        // platform's, which is why this crosses as one quadrilateral and not as pixels.
        cr.set_source_rgba(
            f64::from(colour.red()),
            f64::from(colour.green()),
            f64::from(colour.blue()),
            0.9,
        );
        cr.set_line_width(2.0);
        trace_quad(cr, quad, scale);
        cr.stroke()?;
    }
    Ok(())
}

/// One `[x0, y0, … x3, y3]` in device pixels, as a closed path in logical ones.
fn trace_quad(cr: &gtk4::cairo::Context, quad: [f32; 8], scale: f64) {
    cr.move_to(f64::from(quad[0]) / scale, f64::from(quad[1]) / scale);
    cr.line_to(f64::from(quad[2]) / scale, f64::from(quad[3]) / scale);
    cr.line_to(f64::from(quad[4]) / scale, f64::from(quad[5]) / scale);
    cr.line_to(f64::from(quad[6]) / scale, f64::from(quad[7]) / scale);
    cr.close_path();
}

/// GDK's key as the one [`viewer_host::keys`] states a meaning for, or nothing.
///
/// **This is the whole of what this host contributes to its key bindings** (ADR 0526).
/// `gdk::Key` against `Qt::Key` against `winit::keyboard::Key` is what a toolkit is, and what a
/// press *means* is this project's reading of §12.5.1 and §12.4.4.2 plus a page of choices — which
/// is why the table is in `viewer-host` and this function is here.
///
/// A letter arrives in both cases because GDK reports the shifted one and none of the letters the
/// table binds means a second thing when shifted.
fn key_pressed(key: gtk4::gdk::Key) -> Option<viewer_host::Key> {
    use gtk4::gdk::Key as Gdk;
    use viewer_host::Key as Stated;
    Some(match key {
        Gdk::a | Gdk::A => Stated::A,
        Gdk::c | Gdk::C => Stated::C,
        Gdk::f | Gdk::F => Stated::F,
        Gdk::h | Gdk::H => Stated::H,
        Gdk::k | Gdk::K => Stated::K,
        Gdk::l | Gdk::L => Stated::L,
        Gdk::o | Gdk::O => Stated::O,
        Gdk::p | Gdk::P => Stated::P,
        Gdk::s | Gdk::S => Stated::S,
        Gdk::t | Gdk::T => Stated::T,
        Gdk::w | Gdk::W => Stated::W,
        Gdk::y | Gdk::Y => Stated::Y,
        Gdk::z | Gdk::Z => Stated::Z,
        Gdk::_0 => Stated::Zero,
        Gdk::plus => Stated::Plus,
        Gdk::minus => Stated::Minus,
        Gdk::equal => Stated::Equals,
        Gdk::slash => Stated::Slash,
        Gdk::question => Stated::Question,
        Gdk::Escape => Stated::Escape,
        Gdk::Tab | Gdk::ISO_Left_Tab => Stated::Tab,
        Gdk::space => Stated::Space,
        Gdk::Home => Stated::Home,
        Gdk::End => Stated::End,
        Gdk::Left => Stated::Left,
        Gdk::Right => Stated::Right,
        Gdk::Up => Stated::Up,
        Gdk::Down => Stated::Down,
        Gdk::Page_Up => Stated::PageUp,
        Gdk::Page_Down => Stated::PageDown,
        _ => return None,
    })
}

/// Which of this notebook's pages one of [`Tab`]'s panels is.
///
/// **Exhaustive over [`Tab`] on purpose, and it is a second statement rather than a wrapper around
/// [`Tab::index`].** The notebook's pages are appended in [`Tab::ALL`]'s order and a panel is
/// addressed by its index, so the two ways of naming a panel are two things that can disagree —
/// which is exactly what `every_panel_the_list_states_has_a_page_of_this_notebook` checks. A panel
/// added to `viewer_host::Tab` fails to compile here until this host says where it goes.
const fn notebook_page(tab: Tab) -> u32 {
    match tab {
        Tab::Contents => 0,
        Tab::Pages => 1,
        Tab::Layers => 2,
        Tab::Files => 3,
        Tab::Articles => 4,
        Tab::Document => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::key_pressed;
    use gtk4::gdk::Key as Gdk;
    use viewer_host::Tab;

    /// Every key the shared table states has a `gdk::Key` in this host.
    ///
    /// **This is the instrument the level-hosts decision never had** (ADR 0526). The match is
    /// exhaustive over [`viewer_host::Key`], so a binding added to `viewer-host` fails to compile
    /// here until this host says which key produces it, and the assertion then checks that the
    /// *runtime* translation agrees — a key named here and forgotten in [`key_pressed`] fails
    /// rather than drifting. `viewer-ui` and `viewer-qt` carry the same test against their own
    /// toolkits.
    ///
    /// It needs no display: a `gdk::Key` is a wrapped keyval and nothing here calls into GTK.
    #[test]
    fn every_key_the_table_states_has_one_in_this_toolkit() {
        use viewer_host::Key as Stated;
        for stated in Stated::ALL {
            let key = match stated {
                Stated::A => Gdk::a,
                Stated::C => Gdk::c,
                Stated::F => Gdk::f,
                Stated::H => Gdk::h,
                Stated::K => Gdk::k,
                Stated::L => Gdk::l,
                Stated::O => Gdk::o,
                Stated::P => Gdk::p,
                Stated::S => Gdk::s,
                Stated::T => Gdk::t,
                Stated::W => Gdk::w,
                Stated::Y => Gdk::y,
                Stated::Z => Gdk::z,
                Stated::Zero => Gdk::_0,
                Stated::Plus => Gdk::plus,
                Stated::Minus => Gdk::minus,
                Stated::Equals => Gdk::equal,
                Stated::Slash => Gdk::slash,
                Stated::Question => Gdk::question,
                Stated::Escape => Gdk::Escape,
                Stated::Tab => Gdk::Tab,
                Stated::Space => Gdk::space,
                Stated::Home => Gdk::Home,
                Stated::End => Gdk::End,
                Stated::Left => Gdk::Left,
                Stated::Right => Gdk::Right,
                Stated::Up => Gdk::Up,
                Stated::Down => Gdk::Down,
                Stated::PageUp => Gdk::Page_Up,
                Stated::PageDown => Gdk::Page_Down,
            };
            assert_eq!(
                key_pressed(key),
                Some(*stated),
                "{stated:?} is stated by the table and this host does not produce it"
            );
        }
    }

    /// Every panel the shared list states has a page of this notebook, at the same place.
    ///
    /// **The instrument `doc/todo/30`'s "all three hosts stay level" never had for a *panel***, and
    /// the same shape as the key test above (ADR 0526). Two halves: [`super::notebook_page`] is
    /// exhaustive over [`viewer_host::Tab`], so a panel added there fails to compile in this host
    /// — and in `Host::panel_of`, which is where its answer has to come from — and the assertion
    /// then checks that this host's page numbers are the shared list's own order, which is what
    /// `Ui::slots` is indexed by. `viewer-ui` and `viewer-qt` carry the same test.
    ///
    /// It needs no display: nothing here calls into GTK.
    #[test]
    fn every_panel_the_list_states_has_a_page_of_this_notebook() {
        for (place, tab) in Tab::ALL.iter().enumerate() {
            assert_eq!(
                u64::from(super::notebook_page(*tab)),
                place as u64,
                "{tab:?} is the {place}th panel and this host puts it somewhere else"
            );
        }
    }

    /// A capital letter is the same key, because GDK reports the shifted keyval.
    #[test]
    fn a_capital_letter_is_the_same_key_as_its_lower_case() {
        assert_eq!(key_pressed(Gdk::A), key_pressed(Gdk::a));
        assert_eq!(key_pressed(Gdk::Z), key_pressed(Gdk::z));
    }

    /// A key this host does not bind produces nothing rather than a default.
    #[test]
    fn an_unbound_key_is_nothing_rather_than_something() {
        assert_eq!(key_pressed(Gdk::F1), None);
        assert_eq!(key_pressed(Gdk::b), None);
    }
}
