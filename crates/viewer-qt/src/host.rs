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
    PageTarget, PointerAction, Query, Rendered, Viewer, Zoom,
};
use viewer_host::ControlFit;
use viewer_host::arrangement::next_layout;
use viewer_host::form::{ControlKind, control_kind};
use viewer_host::panel::{PanelRow, RowAction};
use viewer_host::trace::{Topic, Trace};

use crate::bridge::ffi::{QtControl, QtFrame, QtMeasure, QtQuad, QtRow, QtUpdate};
use crate::keys;
use crate::page;

/// The identity this host gives the one document it opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// §7.6.4.1: how many times a password is asked for before the host gives up.
///
/// The clause states no number — it says a processor shall ask — so this is a host's choice, and
/// it is deliberately the same three `viewer-gtk` makes so that the two hosts differ in their
/// toolkit and in nothing else.
const PASSWORD_ATTEMPTS: u32 = 3;

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

/// Which of the three trees a row belongs to.
///
/// §12.3.3's outline, §8.11.4.3's `/Order` and §7.11.4's embedded files. Three answers of three
/// different types, which `viewer_host::panel` turns into one row shape — so the only thing left
/// to distinguish is which tab a row is in, and it crosses the bridge as this.
const TREES: usize = 3;

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
    /// §12.7.4.2's qualified name and the widget annotation, which together name this control.
    key: (String, pdf_syntax::ObjectId),
    /// What kind of control it is.
    kind: ControlKind,
    /// The appearance state §12.7.5.2.3 makes a check box's on value, where the file names one.
    on_state: Option<String>,
    /// Table 229 bit 15: whether clicking the selected radio button of a set turns it off.
    no_toggle_to_off: bool,
}

/// One document, one viewer, and the loop between them.
pub struct Host {
    /// The state machine every host on this boundary drives.
    viewer: Viewer,
    /// Where the bytes came from, which is what rule 2 makes a host's business and not the core's.
    path: PathBuf,
    /// The directory §12.7.6.4's policy resolves against.
    directory: Option<PathBuf>,
    /// The file, kept because §7.6.4.1's second attempt opens it again with a password.
    bytes: Vec<u8>,
    /// Annex O's fragment, where the host was given one.
    fragment: Option<String>,
    /// Tier 1's worker, called on the thread Qt runs its event loop on.
    rasterizer: CpuRasterizer,
    /// The launch timeline.
    trace: Trace,
    /// §6.3.2.2: who draws §12.7's widgets — this host's controls, or the document's own pictures.
    widget_appearances: WidgetAppearances,
    /// Device pixels per logical pixel, from the screen Qt put the window on.
    scale: f32,
    /// How many times §7.6.4.1's password has been asked for.
    attempts: u32,
    /// Whether the document has been opened yet, which waits for the first resize.
    opened: bool,
    /// Whether anything is unsaved.
    dirty: bool,
    /// What the title bar says about the page.
    caption: String,
    /// The most recent sentence for a person.
    message: String,
    /// The three trees.
    trees: [Vec<Flat>; TREES],
    /// The fields of the page being shown, as `Query::Fields` last answered.
    fields: Vec<FormField>,
    /// One entry per control, in the order the C++ side placed them.
    placed: Vec<Placement>,
    /// What has changed since the C++ side last asked.
    update: QtUpdate,
    /// Whether the first frame has been reported, so that the launch line is printed once.
    presented: bool,
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
    /// Table 29's arrangement, as this window last asked for it — what `l` cycles from.
    layout: pdf_model::viewer_preferences::PageLayout,
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
        trace: Trace,
    ) -> Result<Self, HostError> {
        let bytes = std::fs::read(path).map_err(|error| HostError::Unreadable {
            path: path.to_owned(),
            error: error.to_string(),
        })?;
        trace.say(
            Topic::Launch,
            format_args!("read {} bytes of {}", bytes.len(), path.display()),
        );
        Ok(Self {
            viewer: Viewer::new(1, 1, 1.0),
            path: path.to_owned(),
            directory: path.parent().map(Path::to_owned),
            bytes,
            fragment,
            rasterizer: CpuRasterizer::new(),
            trace,
            widget_appearances,
            scale: 1.0,
            attempts: 0,
            opened: false,
            dirty: false,
            caption: String::new(),
            message: String::new(),
            trees: [Vec::new(), Vec::new(), Vec::new()],
            fields: Vec::new(),
            placed: Vec::new(),
            update: nothing_changed(),
            presented: false,
            needle: String::new(),
            pages_left: 0,
            fit_magnification: None,
            layout: pdf_model::viewer_preferences::PageLayout::SinglePage,
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

    /// A key was pressed, as `Qt::Key`.
    pub(crate) fn key(&mut self, code: u32) {
        // The one key whose meaning is not in `keys::command`'s table, and the reason is that it
        // is not a fact about the key: what `w` sends depends on what this page's controls
        // measured, so the command cannot be built without the host. `viewer-gtk` binds the same
        // letter to the same thing.
        if code == keys::FIT_CONTROLS {
            match self.fit_magnification {
                Some(wanted) => {
                    self.say(&format!("fitting §12.7's controls at {wanted:.3}"));
                    self.dispatch(Command::Zoom {
                        zoom: Zoom::Scale(wanted),
                        at: None,
                    });
                }
                None => self.say("every control on this page already fits its /Rect"),
            }
            return;
        }
        // The second key whose meaning is this host's state rather than the key's: Table 29's
        // six arrangements are cycled, so what to send depends on which one is in force.
        if code == keys::NEXT_LAYOUT {
            self.layout = next_layout(self.layout);
            self.dispatch(Command::Layout(self.layout));
            self.say(&format!("page layout: {:?} (§7.7.2)", self.layout));
            // A new arrangement is a new set of pages on the screen, and therefore a new set of
            // things they could not draw.
            self.restate();
            return;
        }
        if let Some(command) = keys::command(code) {
            self.dispatch(command);
        }
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
        let field = placed.key.0.clone();
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
        let field = placed.key.0.clone();
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
    pub(crate) fn toggle_control(&mut self, index: usize, on: bool) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let field = placed.key.0.clone();
        let value = if on {
            let Some(state) = placed.on_state.clone() else {
                self.say(&format!(
                    "the field {field} states no appearance for an on state (§12.7.5.2.3)"
                ));
                return;
            };
            state
        } else {
            if placed.no_toggle_to_off {
                // Table 229 bit 15: "selecting the currently selected button has no effect".
                return;
            }
            "Off".to_owned()
        };
        self.dispatch(Command::Edit(Edit::SetField {
            field,
            value: Entered::Text(value),
        }));
    }

    /// §12.7.5.2.2's push button was pressed.
    pub(crate) fn activate_control(&mut self, index: usize) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let annotation = placed.key.1;
        self.dispatch(Command::Activate(annotation));
    }

    /// §7.6.4.1: a person typed a password.
    pub(crate) fn supply_password(&mut self, password: &str) {
        self.open_document(Some(password.to_owned()));
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

    /// How many pages Table 29's arrangement is showing pixels for.
    ///
    /// One under `SinglePage`, which is what this host drew for two hundred sessions; more under
    /// a column or a spread, and the window paints each of them where [`Self::frame`] says.
    pub(crate) fn frame_count(&self) -> usize {
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames.len(),
            _ => 0,
        }
    }

    /// Where one page's pixels belong and how big they are.
    pub(crate) fn frame(&self, index: usize) -> QtFrame {
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
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames
                .get(index)
                .map_or(&[][..], |frame| frame.raster.data.as_slice()),
            _ => &[],
        }
    }

    /// One tree's rows, depth first.
    pub(crate) fn rows(&self, tree: u8) -> Vec<QtRow> {
        self.tree(tree)
            .map(|rows| rows.iter().map(|flat| flat.row.clone()).collect())
            .unwrap_or_default()
    }

    /// Every control the page's form wants.
    pub(crate) fn controls(&self) -> Vec<QtControl> {
        let mut controls = Vec::with_capacity(self.placed.len());
        for (placed, (field, widget)) in self.placed.iter().zip(self.widgets()) {
            let (x, y, width, height) = bounds(widget.quad);
            let (kind, max_len, multi, editable) = describe_kind(&placed.kind);
            controls.push(QtControl {
                x,
                y,
                width,
                height,
                kind,
                field: placed.key.0.clone(),
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
    }

    /// One sentence from the C++ side.
    pub(crate) fn note(&self, what: &str) {
        self.trace.say(Topic::Panel, format_args!("{what}"));
    }

    // ---------------------------------------------------------------------------------------
    // The loop, which is the same one every host on this boundary runs.
    // ---------------------------------------------------------------------------------------

    /// §7.6.4.1: opens the document, with a password where one has been supplied.
    fn open_document(&mut self, password: Option<String>) {
        let bytes = self.bytes.clone();
        let fragment = self.fragment.clone();
        // §6.3.2.2's instruction goes first, so that page one is interpreted once. It is a
        // property of *this host* rather than of the document (ADR 0245).
        self.pump(vec![
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
    fn dispatch(&mut self, command: Command) {
        self.pump(vec![command]);
    }

    /// Runs commands until nothing is left, reacting to what each produces, then refreshes.
    fn pump(&mut self, queue: Vec<Command>) {
        let mut queue: std::collections::VecDeque<Command> = queue.into();
        while let Some(command) = queue.pop_front() {
            let described = self
                .trace
                .on(Topic::Events)
                .then(|| format!("{command:?}"))
                .map(|text| text.chars().take(120).collect::<String>());
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
    }

    /// Does what one event asks.
    fn react(&mut self, event: Event, queue: &mut std::collections::VecDeque<Command>) {
        match event {
            Event::Opened { pages, .. } => {
                self.trace
                    .say(Topic::Launch, format_args!("opened, {pages} page(s)"));
                // Table 29's `/PageLayout` is the viewer's to apply, and what this host needs
                // from it is the value `l` cycles from — see `viewer-gtk`, which does the same.
                if let Answer::Opening(opening) = self.viewer.query(Query::Opening) {
                    self.layout = opening.layout;
                    if opening.layout != pdf_model::viewer_preferences::PageLayout::SinglePage {
                        self.say(&format!(
                            "this document opens in the {:?} page layout (§7.7.2)",
                            opening.layout
                        ));
                    }
                }
                self.build_panels();
            }
            Event::OpenFailed { reason, .. } => {
                self.say(&format!("cannot open {}: {reason}", self.path.display()));
            }
            // §7.6.4.1: "the interactive PDF processor shall … prompt the user for a password".
            // The prompt is a window, and a window is a host's — which is the whole reason this
            // event exists rather than a refusal.
            Event::PasswordRequired { .. } => {
                self.attempts = self.attempts.saturating_add(1);
                if self.attempts > PASSWORD_ATTEMPTS {
                    self.say("too many password attempts");
                    return;
                }
                self.update.password = true;
            }
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
            }
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
            // §12.4.4: named rather than played. This host has no presentation clock, so the page
            // is drawn, which is the transition's own end state — and the name is said rather
            // than swallowed, because a person who asked for a slide show is owed the difference.
            Event::Transition { transition, .. } => {
                self.say(&format!("transition: {:?}", transition.style));
            }
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
            Event::Refused { notes, .. } => self.say(&format!(
                "{} — this reader is obeying that; --ignore-restrictions turns it off",
                notes.join("; ")
            )),
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
        let keys: Vec<(String, pdf_syntax::ObjectId)> =
            placed.iter().map(|one| one.key.clone()).collect();
        let was: Vec<(String, pdf_syntax::ObjectId)> =
            self.placed.iter().map(|one| one.key.clone()).collect();
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
    }

    /// Builds the three trees from the three answers.
    fn build_panels(&mut self) {
        let outline = match self.viewer.query(Query::Outline) {
            Answer::Outline(outline) => viewer_host::panel::outline_rows(&outline),
            _ => Vec::new(),
        };
        let layers = match self.viewer.query(Query::Layers) {
            Answer::Layers(layers) => viewer_host::panel::layer_rows(&layers),
            _ => Vec::new(),
        };
        let files = match self.viewer.query(Query::Attachments) {
            Answer::Attachments(attachments) => viewer_host::panel::attachment_rows(&attachments),
            _ => Vec::new(),
        };
        self.trace.say(
            Topic::Panel,
            format_args!(
                "outline {} row(s), layers {} row(s), files {} row(s)",
                outline.len(),
                layers.len(),
                files.len()
            ),
        );
        self.trees = [flatten(&outline), flatten(&layers), flatten(&files)];
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
            key: (field.name.qualified.clone(), widget.annotation),
            kind: kind.clone(),
            on_state: widget.on_state.clone(),
            no_toggle_to_off: matches!(
                kind,
                ControlKind::Radio {
                    no_toggle_to_off: true,
                    ..
                }
            ),
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
            },
            action: row.action.clone(),
        });
        push_rows(&row.children, depth.saturating_add(1), into);
    }
}

/// The axis-aligned bound of a quadrilateral, in the device pixels it arrived in.
///
/// A widget's `/Rect` can arrive rotated — §7.7.3.3's `/Rotate` and Table 192's `/R` both turn it
/// — and a platform control is a rectangle, so this is where a host loses that. Both native hosts
/// lose it in the same place and for the same reason, which is a fact about toolkits rather than
/// about the boundary: `FormWidget::quad` carries the four corners correctly.
fn bounds(quad: [f32; 8]) -> (f32, f32, f32, f32) {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let left = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let top = ys.iter().copied().fold(f32::INFINITY, f32::min);
    let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (left, top, right - left, bottom - top)
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
        title: false,
        status: false,
        password: false,
    }
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
    use super::{Host, describe_kind, flatten};
    use pdf_model::view::WidgetAppearances;
    use pdf_syntax::ObjectId;
    use std::path::{Path, PathBuf};
    use viewer_host::form::ControlKind;
    use viewer_host::panel::{PanelRow, RowAction};
    use viewer_host::trace::Trace;

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
        let mut host = Host::open(
            path,
            None,
            WidgetAppearances::Delegated,
            Trace::off(std::time::Instant::now()),
        )
        .expect("the document is readable");
        host.resized(800, 1000, 1.0);
        host
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

    /// The two trees this document states nothing for answer nothing rather than something empty.
    #[test]
    fn a_document_with_no_layers_and_no_files_has_no_rows_for_them() {
        let host = opened(&committed());
        assert!(host.rows(1).is_empty());
        assert!(host.rows(2).is_empty());
        // And a tree number this host does not have answers nothing rather than the first one.
        assert!(host.rows(9).is_empty());
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
                children: vec![PanelRow {
                    label: "under".to_owned(),
                    detail: Some("said".to_owned()),
                    expanded: false,
                    action: RowAction::Inert,
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
