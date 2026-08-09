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
//! the only size signal GTK4 gives application code without subclassing a widget, and
//! `#![forbid(unsafe_code)]` is what makes subclassing the wrong answer here.

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
    Answer, Command, DocumentId, Edit, Entered, Event, FormField, PageTarget, PointerAction, Query,
    Rendered, Selection, Viewer, Zoom,
};

use crate::controls::{FieldChange, Placed};
use viewer_host::panel::{self, RowAction};
use viewer_host::trace::{Topic, Trace};

use crate::{controls, page, tree};

/// The identity this host gives the one document it opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// §7.6.4.1: how many times a password is asked for before the host gives up.
///
/// The clause states no number — it says a processor shall ask — so this is a host's choice and
/// it is the one every login prompt makes. A host that asked forever would be a host a person
/// cannot leave.
const PASSWORD_ATTEMPTS: u32 = 3;

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
    /// §12.5.1's ring, in the same pixels.
    focus: Option<[f32; 8]>,
    /// Device pixels per logical pixel, because GTK draws in logical ones.
    scale: f64,
}

/// The widgets, held so that the loop can put things in them.
#[derive(Debug)]
struct Ui {
    /// The toplevel.
    window: gtk4::ApplicationWindow,
    /// The page and the form controls.
    fixed: gtk4::Fixed,
    /// The page's pixels.
    picture: gtk4::Picture,
    /// The layer that measures the viewport and draws the interactive chrome.
    chrome: gtk4::DrawingArea,
    /// Where §12.3.3's tree goes.
    outline_slot: gtk4::Box,
    /// Where §8.11.4.3's tree goes.
    layers_slot: gtk4::Box,
    /// Where §7.11.4's tree goes.
    files_slot: gtk4::Box,
    /// Trap 5's channel reaching a person: what the page could not draw, and what was refused.
    status: gtk4::Label,
}

/// One document, one window, and the loop between them.
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
    /// Tier 1's worker, called on this thread. See ADR 0244 for what that costs and why.
    rasterizer: CpuRasterizer,
    /// The launch timeline.
    trace: Trace,
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
    /// How many times §7.6.4.1's password has been asked for.
    attempts: u32,
    /// Whether the document has been opened yet, which waits for the first allocation.
    opened: bool,
    /// Whether anything is unsaved.
    dirty: bool,
    /// What the title bar says about the page.
    caption: String,
    /// The controls over the page, and which fields they are for.
    placed: Vec<Placed>,
    /// Whether the first frame has been reported, so that the launch line is printed once.
    presented: bool,
    /// §6.3.2.2: who draws §12.7's widgets — this host's controls, or the document's own pictures.
    ///
    /// [`WidgetAppearances::Delegated`] unless the command line asked otherwise, because this host
    /// *does* place a real control over every widget and leaving the appearance underneath is the
    /// duplication ADR 0244 photographed. `--draw-widget-appearances` restores §6.3.2.2's default,
    /// which is what taking the two pictures side by side needs.
    widget_appearances: WidgetAppearances,
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
                attempts: 0,
                opened: false,
                dirty: false,
                caption: String::new(),
                placed: Vec::new(),
                presented: false,
                widget_appearances,
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
    fn open_document(&mut self, password: Option<String>) {
        let bytes = self.bytes.clone();
        let fragment = self.fragment.clone();
        // §6.3.2.2's instruction goes first, so that page one is interpreted once. It is a
        // property of *this host* rather than of the document, which is why it is not part of
        // `Open`: a host that changes its mind sends it again and the page is rebuilt.
        self.pump(VecDeque::from([
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
    fn dispatch(&mut self, command: Command) {
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
    fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            Event::Opened { pages, .. } => {
                self.trace
                    .say(Topic::Launch, format_args!("opened, {pages} page(s)"));
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
                self.ask_for_a_password();
            }
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
            } => {
                // §12.4.2: "[p]age labels and page indices need not coincide". Both, because a
                // label alone cannot say "of 320".
                let page = match label {
                    Some(label) => format!("{label} — page {} of {of}", index.saturating_add(1)),
                    None => format!("page {} of {of}", index.saturating_add(1)),
                };
                self.caption = match section {
                    Some(section) if !section.is_empty() => format!("{page} — {section}"),
                    _ => page,
                };
                self.retitle();
            }
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
            // §12.4.4: named rather than played. This host has no presentation clock, so the page
            // is drawn, which is the transition's own end state — and the name is said rather
            // than swallowed, because a person who asked for a slide show is owed the difference.
            Event::Transition { transition, .. } => {
                self.say(&format!("transition: {:?}", transition.style));
            }
            Event::Extracted { name, bytes, .. } => self.write_extracted(&name, &bytes),
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            Event::Dirty { dirty, .. } => {
                self.dirty = dirty;
                self.retitle();
            }
            Event::Reported { notes, .. } => self.say(&notes.join("; ")),
            // `CLAUDE.md`: a document's restrictions are the reader's to set, and it shall always
            // be possible to turn them off. This host says which one applied and how to turn it
            // off; the four levels have no user interface yet and none is to be built now.
            Event::Refused { notes, .. } => self.say(&format!(
                "{} — this reader is obeying that; --ignore-restrictions turns it off",
                notes.join("; ")
            )),
        }
    }

    /// Puts the frame on the screen, the controls over it, and the chrome over both.
    ///
    /// Idempotent and called after every pump, which is what keeps the window a function of the
    /// viewer's state rather than of the order events arrived in.
    fn refresh(&mut self) {
        let began = std::time::Instant::now();
        let placement = match self.viewer.query(Query::Frame) {
            Answer::Frame(frame) => match page::texture(frame.raster) {
                Ok(texture) => Some((texture, frame.origin, frame.raster.data.len())),
                Err(error) => {
                    eprintln!("note: {error}");
                    None
                }
            },
            _ => None,
        };
        if let Some((texture, origin, bytes)) = placement {
            // Tier 1's whole cost, in the one place it is paid: `doc/ui-boundary.md` prices the
            // tier at "one copy per frame" and this is that copy, timed rather than assumed
            // (ADR 0244).
            self.trace.say(
                Topic::Frames,
                format_args!("{bytes} bytes into a texture in {:?}", began.elapsed()),
            );
            let scale = f64::from(self.scale);
            let (width, height) = (texture.width(), texture.height());
            self.ui.picture.set_paintable(Some(&texture));
            self.ui.picture.set_size_request(
                logical(f64::from(width), scale),
                logical(f64::from(height), scale),
            );
            self.ui.fixed.set_visible(true);
            self.ui.fixed.move_(
                &self.ui.picture,
                f64::from(origin.0) / scale,
                f64::from(origin.1) / scale,
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
        let fields = match self.viewer.query(Query::Fields) {
            Answer::Fields(fields) => fields,
            _ => Vec::new(),
        };
        self.place_fields(&fields);
        let selection = match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.quads.clone(),
            _ => Vec::new(),
        };
        let focus = match self.viewer.query(Query::Focus) {
            Answer::Focus { quad, .. } => Some(quad),
            _ => None,
        };
        {
            let mut chrome = self.chrome.borrow_mut();
            chrome.selection = selection;
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
                let (x, y, width, height) = bounds(widget.quad);
                self.ui
                    .fixed
                    .move_(&placed.widget, f64::from(x) / scale, f64::from(y) / scale);
                let (asked_width, asked_height) = (
                    logical(f64::from(width), scale),
                    logical(f64::from(height), scale),
                );
                placed.widget.set_size_request(asked_width, asked_height);
                fit.record(&placed.widget, asked_width, asked_height);
                write_back(placed, field.value.as_ref());
            }
        }
        self.suppress.set(false);
        fit.say(&self.trace);
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

    /// Builds the three trees from the three answers.
    fn build_panels(&mut self) {
        let outline = match self.viewer.query(Query::Outline) {
            Answer::Outline(outline) => panel::outline_rows(&outline),
            _ => Vec::new(),
        };
        let layers = match self.viewer.query(Query::Layers) {
            Answer::Layers(layers) => panel::layer_rows(&layers),
            _ => Vec::new(),
        };
        let files = match self.viewer.query(Query::Attachments) {
            Answer::Attachments(attachments) => panel::attachment_rows(&attachments),
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
        let act = self.row_sink();
        fill(
            &self.ui.outline_slot,
            &outline,
            &act,
            "no outline (§12.3.3)",
        );
        fill(&self.ui.layers_slot, &layers, &act, "no layers (§8.11.4.3)");
        fill(&self.ui.files_slot, &files, &act, "no files (§7.11.4)");
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
    fn ask_for_a_password(&mut self) {
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
        let label = gtk4::Label::new(Some(&format!(
            "{} is encrypted (§7.6.4.1).",
            named(&self.path)
        )));
        label.set_xalign(0.0);
        column.append(&label);
        let entry = gtk4::PasswordEntry::new();
        entry.set_show_peek_icon(true);
        entry.set_activates_default(true);
        column.append(&entry);
        let open = gtk4::Button::with_label("Open");
        open.add_css_class("suggested-action");
        column.append(&open);
        dialog.set_child(Some(&column));

        let me = self.me.clone();
        let dialogue = dialog.clone();
        let typed = entry.clone();
        open.connect_clicked(move |_| {
            let password = typed.text().to_string();
            dialogue.close();
            with(&me, |host| host.open_document(Some(password)));
        });
        let me = self.me.clone();
        let dialogue = dialog.clone();
        entry.connect_activate(move |typed| {
            let password = typed.text().to_string();
            dialogue.close();
            with(&me, |host| host.open_document(Some(password)));
        });
        dialog.present();
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
    fn write_extracted(&self, name: &str, bytes: &[u8]) {
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

    /// What a key press means.
    fn key(&mut self, key: gtk4::gdk::Key) {
        match key {
            gtk4::gdk::Key::Right | gtk4::gdk::Key::Down | gtk4::gdk::Key::Page_Down => {
                self.dispatch(Command::GoTo(PageTarget::Next));
            }
            gtk4::gdk::Key::Left | gtk4::gdk::Key::Up | gtk4::gdk::Key::Page_Up => {
                self.dispatch(Command::GoTo(PageTarget::Previous));
            }
            gtk4::gdk::Key::Home => self.dispatch(Command::GoTo(PageTarget::First)),
            gtk4::gdk::Key::End => self.dispatch(Command::GoTo(PageTarget::Last)),
            gtk4::gdk::Key::plus | gtk4::gdk::Key::equal => self.dispatch(Command::Zoom {
                zoom: Zoom::In,
                at: None,
            }),
            gtk4::gdk::Key::minus => self.dispatch(Command::Zoom {
                zoom: Zoom::Out,
                at: None,
            }),
            gtk4::gdk::Key::_0 => self.dispatch(Command::Zoom {
                zoom: Zoom::FitPage,
                at: None,
            }),
            gtk4::gdk::Key::a => self.dispatch(Command::Select(Selection::All)),
            gtk4::gdk::Key::Escape => self.dispatch(Command::Select(Selection::None)),
            gtk4::gdk::Key::s => self.dispatch(Command::Save),
            gtk4::gdk::Key::z => self.dispatch(Command::Undo),
            gtk4::gdk::Key::y => self.dispatch(Command::Redo),
            _ => {}
        }
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
    }
}

/// What to call the document in a title bar: its file name, or the whole path where it has none.
fn named(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Runs `what` against the host, or says why it could not.
///
/// A callback that arrives while the host is already borrowed is a callback GTK raised from
/// inside one of this host's own writes — dropping it is right, and saying so is trap 5.
fn with(me: &Weak<RefCell<Host>>, what: impl FnOnce(&mut Host)) {
    let Some(host) = me.upgrade() else {
        return;
    };
    match host.try_borrow_mut() {
        Ok(mut host) => what(&mut host),
        Err(_) => eprintln!("note: the host was busy and an action was dropped"),
    }
}

/// A tree in a slot, or the sentence saying the document states none.
fn fill(slot: &gtk4::Box, rows: &[panel::PanelRow], act: &Rc<dyn Fn(&RowAction)>, empty: &str) {
    while let Some(child) = slot.first_child() {
        slot.remove(&child);
    }
    if rows.is_empty() {
        let label = gtk4::Label::new(Some(empty));
        label.add_css_class("dim-label");
        label.set_vexpand(true);
        slot.append(&label);
        return;
    }
    slot.append(&tree::tree(rows, act));
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
fn write_back(placed: &Placed, value: Option<&pdf_model::view::ShownValue>) {
    let Some(shown) = value else {
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
        _ => {}
    }
}

/// The axis-aligned bound of a quadrilateral, in the device pixels it arrived in.
///
/// A widget's `/Rect` can arrive rotated — §7.7.3.3's `/Rotate` and Table 192's `/R` both turn it
/// — and a platform control is a rectangle, so this is where a host loses that. Said rather than
/// hidden: a rotated widget gets an upright control, and now that ADR 0245 takes the appearance out
/// from under it there is nothing rotated left on the page to disagree with. (This cited Table 189,
/// which is the *movie* annotation's, until the four-hundred-and-ninth session read it.)
fn bounds(quad: [f32; 8]) -> (f32, f32, f32, f32) {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let left = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let top = ys.iter().copied().fold(f32::INFINITY, f32::min);
    let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (left, top, right - left, bottom - top)
}

/// How badly the platform's controls fit the rectangles the document states for them.
///
/// **ADR 0244's second finding, measured now that it can be seen alone.** A `/Rect` is whatever
/// the document says; a GTK control has a *minimum* size its theme decides, and
/// `gtk_widget_set_size_request` is a floor rather than a size — so a control whose minimum
/// exceeds its rectangle covers the page around it. Taking the widget's own `measure` is what
/// turns "a `GtkEntry` is about 34 logical pixels tall" into a count and a worst case, per page
/// and per run.
///
/// Not a defect in `viewer-core` and not this host's to fix by shrinking a control: a platform
/// control that a theme sizes is the whole point of using one. What it says is that a native form
/// host also wants to choose the *scale* the page is drawn at, which is a third thing and neither
/// this crate's nor `pdf-model`'s.
#[derive(Debug, Default)]
struct ControlFit {
    /// How many controls were placed.
    placed: usize,
    /// How many are wider than their rectangle, and how many are taller.
    over: (usize, usize),
    /// The largest excess width, and the rectangle's own width beside it, in logical pixels.
    widest: (i32, i32),
    /// The largest excess height, and the rectangle's own height beside it.
    tallest: (i32, i32),
}

impl ControlFit {
    /// Measures one control against the rectangle it was asked to occupy.
    ///
    /// `measure` is asked with `-1` in both directions — GTK's "for no particular size" — because
    /// asking for a height at a width smaller than the control's minimum is the case GTK warns
    /// about on the console, and what is wanted here is the minimum itself.
    fn record(&mut self, widget: &gtk4::Widget, width: i32, height: i32) {
        self.placed = self.placed.saturating_add(1);
        let (minimum_width, ..) = widget.measure(gtk4::Orientation::Horizontal, -1);
        let (minimum_height, ..) = widget.measure(gtk4::Orientation::Vertical, -1);
        if minimum_width > width {
            self.over.0 = self.over.0.saturating_add(1);
            if minimum_width.saturating_sub(width) > self.widest.0 {
                self.widest = (minimum_width.saturating_sub(width), width);
            }
        }
        if minimum_height > height {
            self.over.1 = self.over.1.saturating_add(1);
            if minimum_height.saturating_sub(height) > self.tallest.0 {
                self.tallest = (minimum_height.saturating_sub(height), height);
            }
        }
    }

    /// Says what was measured, where anything was placed.
    fn say(&self, trace: &Trace) {
        if self.placed == 0 {
            return;
        }
        trace.say(
            Topic::Panel,
            format_args!(
                "{} of {} control(s) wider than their /Rect (worst +{} on {} px), {} taller \
                 (worst +{} on {} px)",
                self.over.0,
                self.placed,
                self.widest.0,
                self.widest.1,
                self.over.1,
                self.tallest.0,
                self.tallest.1
            ),
        );
    }
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

    let fixed = gtk4::Fixed::new();
    fixed.set_overflow(gtk4::Overflow::Hidden);
    let picture = gtk4::Picture::new();
    picture.set_content_fit(gtk4::ContentFit::Fill);
    picture.set_can_shrink(false);
    fixed.put(&picture, 0.0, 0.0);

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

    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&fixed));
    overlay.add_overlay(&chrome);
    overlay.set_overflow(gtk4::Overflow::Hidden);
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);

    let outline_slot = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let layers_slot = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let files_slot = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let tabs = gtk4::Notebook::new();
    tabs.append_page(&outline_slot, Some(&gtk4::Label::new(Some("Outline"))));
    tabs.append_page(&layers_slot, Some(&gtk4::Label::new(Some("Layers"))));
    tabs.append_page(&files_slot, Some(&gtk4::Label::new(Some("Files"))));
    tabs.set_size_request(280, -1);

    let split = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    split.set_start_child(Some(&tabs));
    split.set_end_child(Some(&overlay));
    split.set_position(300);
    split.set_resize_start_child(false);

    let status = gtk4::Label::new(None);
    status.set_xalign(0.0);
    status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    status.set_margin_start(6);
    status.set_margin_end(6);
    status.set_margin_top(3);
    status.set_margin_bottom(3);

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    column.append(&split);
    column.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    column.append(&status);
    split.set_vexpand(true);
    window.set_child(Some(&column));
    window.set_titlebar(Some(&header(me)));

    listen(&window, &overlay, &chrome, me);

    window.present();

    Ui {
        window,
        fixed,
        picture,
        chrome,
        outline_slot,
        layers_slot,
        files_slot,
        status,
    }
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
    keys.connect_key_pressed(move |_, key, _, _| {
        with(&listener, |host| host.key(key));
        glib::Propagation::Proceed
    });
    window.add_controller(keys);

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

/// The title bar's own buttons.
fn header(me: &Weak<RefCell<Host>>) -> gtk4::HeaderBar {
    let bar = gtk4::HeaderBar::new();
    for (label, target) in [("‹", PageTarget::Previous), ("›", PageTarget::Next)] {
        let button = gtk4::Button::with_label(label);
        let listener = me.clone();
        button.connect_clicked(move |_| {
            with(&listener, |host| host.dispatch(Command::GoTo(target)));
        });
        bar.pack_start(&button);
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
    }
    bar
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
