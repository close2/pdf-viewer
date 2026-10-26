//! Page one drawn from the document's first bytes before the rest is read, and the handles that
//! carry that work to another thread.

use std::sync::Arc;

use pdf_render::{DisplayList, TargetSpec};

use super::{DocumentId, Viewer};
#[cfg(doc)]
use crate::RenderRequest;
use crate::command::Command;
use crate::event::Event;
use crate::open::Open;

impl Viewer {
    /// What the focused document still has to read that page one does not need, for a host to
    /// run on a thread of its own.
    ///
    /// **The launch path's third overlap** (ADR 1543). [`Command::Open`] no longer reads
    /// §12.3.3's outline or places §7.7.3's whole page tree — together 100.6 M of the 185.1 M
    /// instructions ISO 32000-2's open executed — so the opening [`Event::PageChanged`] names no
    /// section unless they are already read. A host takes this once it has asked for page one,
    /// calls [`crate::Preparation::run`] on any thread, and hands the result to
    /// [`Self::prepared`]; the core spawns nothing and blocks on nothing (rule 4). `None` where
    /// there is nothing to read: no document, or both already read.
    #[must_use]
    pub fn preparation(&self) -> Option<crate::Preparation> {
        let id = self.focused?;
        self.documents.get(&id)?.preparation(id)
    }

    /// Takes in what a [`crate::Preparation`] read, and says the page again with its section.
    ///
    /// An answer for a document no longer open, or for another file opened since under the same
    /// id, is dropped: it names its file as well as its id (trap 104). Where the document is the
    /// one in front and its page has a section, [`Event::PageChanged`] is raised again with it —
    /// the caption the opening announcement could not complete.
    pub fn prepared(&mut self, prepared: crate::Prepared) -> impl Iterator<Item = Event> + use<> {
        let mut events = Vec::new();
        let id = prepared.id;
        let taken = self
            .documents
            .get_mut(&id)
            .is_some_and(|open| open.prepared(prepared));
        if taken
            && self.focused == Some(id)
            && let Some(open) = self.documents.get(&id)
            && open.section_at(open.page_index, false).is_some()
        {
            self.announce_page_reading(false, &mut events);
        }
        events.into_iter()
    }

    /// Interprets the page the focused document stands at, before the host has a viewport.
    ///
    /// **The launch path's second overlap** (ADR 1531). A host that opens its document on a
    /// thread of its own while the graphics device comes up on another (ADR 0182) calls this on
    /// the document's thread, after [`Command::Open`] and any page it was asked to open at; the
    /// first [`Command::Resize`] then finds the page interpreted and asks for its render at once.
    /// Measured on the launch gate's five documents, the interpretation is 0.6 to 18.6 ms that
    /// stood after the device was up and is now inside the time the device takes to come up.
    ///
    /// Nothing here is decided that the first `settle` does not ask again. The page is interpreted
    /// under the view as it stands, with no magnification yet, and `settle` treats it as it treats
    /// any interpretation: dropped by `Open::reinterpret` where the magnification it then learns is
    /// one the page notices (§12.5.3's `NoZoom`), by `Open::stale` where the ink moves, and by the
    /// arrangement where the page is not on the screen at all. So the frame the host draws is the
    /// frame an interpretation at the first resize would have drawn.
    pub fn anticipate(&mut self) -> impl Iterator<Item = Event> + use<> {
        let mut events = Vec::new();
        let delegated = self.delegated;
        if let Some(id) = self.focused
            && let Some(open) = self.documents.get_mut(&id)
        {
            // §6.3.2.2's "unless otherwise instructed" is the host's word, and `settle` carries
            // it into the view before it interprets; carried here too, or a host that delegates
            // its widgets has the page interpreted ahead thrown away by the first `settle` and
            // interpreted again behind the device — what the native windows measured (ADR 1539).
            if open.view.set_widget_appearances(delegated) {
                open.stale();
            }
            let page = open.page_index;
            if open.on(page).is_none()
                && let Some(object) = open.page(page)
            {
                open.on_screen.push(crate::open::OnScreen {
                    page,
                    object,
                    origin: (0.0, 0.0),
                    raster: (0, 0),
                    interpreted: None,
                    replaceable: None,
                    revision: 0,
                    shown: None,
                    frame: None,
                    pending: None,
                    refused: None,
                });
                open.on_screen.sort_by_key(|on_screen| on_screen.page);
            }
            Self::interpret_arranged(open, id, None, &mut events);
        }
        events.into_iter()
    }

    /// [`Self::anticipate`] taken apart, so that a host can draw page one on a thread of its own
    /// while the page is read on another (ADR 1855): the page is set up exactly as `anticipate`
    /// sets it up, and what comes back is the read to run and the target its first frame will be
    /// asked at, both borrowing this viewer. `None` where there is nothing to read — no document,
    /// or a page already interpreted — and then [`Self::anticipate`] is all there is to do.
    ///
    /// **And the same for the page a turn brings on** (ADR 1871): after [`Self::handle_unread`]
    /// the arrangement holds the page unread, and this hands out its read exactly as it hands out
    /// page one's.
    ///
    /// **The core still spawns nothing** (rule 4): [`Reader::read`] runs on whatever thread
    /// the host runs it on, and [`Self::anticipated`] takes what it read in, after which the
    /// viewer is exactly the viewer `anticipate` would have left.
    pub fn anticipating(&mut self) -> Option<Anticipating<'_>> {
        let delegated = self.delegated;
        let budget = self.raster_budget();
        let id = self.focused?;
        let open = self.documents.get_mut(&id)?;
        if open.view.set_widget_appearances(delegated) {
            open.stale();
        }
        let page = open.page_index;
        if open.on(page).is_none() {
            let object = open.page(page)?;
            open.on_screen.push(crate::open::OnScreen {
                page,
                object,
                origin: (0.0, 0.0),
                raster: (0, 0),
                interpreted: None,
                replaceable: None,
                revision: 0,
                shown: None,
                frame: None,
                pending: None,
                refused: None,
            });
            open.on_screen.sort_by_key(|on_screen| on_screen.page);
        }
        let on_screen = open.on(page)?;
        if on_screen.interpreted.is_some()
            || on_screen.replaceable.is_some()
            || open
                .on_screen
                .iter()
                .any(|on_screen| on_screen.page != page && on_screen.interpreted.is_none())
        {
            return None;
        }
        let object = on_screen.object.clone();
        Some(Anticipating {
            open,
            reader: Reader {
                id,
                page,
                object,
                document: Arc::clone(&open.document),
                view: &open.view,
                fonts: &open.fonts,
                references: &open.references,
            },
            budget,
        })
    }

    /// [`Self::anticipating`] for a read the host runs on a thread of its own while this viewer
    /// goes on answering on the host's (ADR 2344): the page a [`Self::handle_unread`] brought on,
    /// handed out as an owned [`ReadOut`] with the target its first frame is foretold at, for a
    /// viewport of `viewport` device pixels at `scale`.
    ///
    /// **Until the read comes back, no `settle` reads the page**, so a command handled in the
    /// meantime — a frame answered, a zoom, a tick — neither reads it a second time on the host's
    /// thread nor asks for it to be drawn; [`Self::anticipated`] takes it in, and
    /// [`Self::read_the_rest`] then asks for its frame. The read is of the view as it stands now,
    /// and a read that comes back to a view changed since is dropped, after which `settle` reads
    /// the page as it always did. `None` where [`Self::anticipating`] answers `None`.
    ///
    /// **The core still spawns nothing** (rule 4): the thread is the host's.
    pub fn read_out(&mut self, viewport: (u32, u32), scale: f32) -> Option<ReadOut> {
        let serial = self.next_out;
        let read_out = {
            let anticipating = self.anticipating()?;
            let foreseen = anticipating.foresee(viewport, scale);
            let Anticipating { open, reader, .. } = anticipating;
            ReadOut {
                id: reader.id,
                page: reader.page,
                object: reader.object,
                document: reader.document,
                view: open.view.clone(),
                fonts: Arc::clone(&open.fonts),
                references: Arc::clone(&open.references),
                serial,
                foreseen,
            }
        };
        self.next_out = serial.wrapping_add(1);
        self.out = Some(Out {
            document: read_out.id,
            page: read_out.page,
            serial,
        });
        Some(read_out)
    }

    /// Takes in the read [`Reader::read`] or [`ReadOut::read`] made, as [`Self::anticipate`]
    /// would have, and names the list to `parts` so that a drawer handed it later knows it for
    /// the one it drew.
    ///
    /// A read out ([`Self::read_out`]) is taken in only where the page still waits for it under
    /// the view it was read under; whether taken or dropped, the page is then `settle`'s again.
    pub fn anticipated(
        &mut self,
        read: Anticipated,
        parts: Option<&pdf_model::progress::Parts>,
    ) -> impl Iterator<Item = Event> + use<> {
        let mut events = Vec::new();
        if let Some((serial, _)) = &read.out
            && self.out.is_some_and(|out| out.serial == *serial)
        {
            self.out = None;
        }
        if let Some(open) = self.documents.get_mut(&read.id)
            && Arc::ptr_eq(&open.document, &read.document)
            && let Some(index) = open
                .on_screen
                .iter()
                .position(|on_screen| on_screen.page == read.page)
            && read.out.as_ref().is_none_or(|(_, view)| {
                *view == open.view
                    && open.on_screen[index].interpreted.is_none()
                    && open.on_screen[index].replaceable.is_none()
            })
            && let Some(done) = read.read
            && let Some(list) = Self::install(open, read.id, index, done, &mut events)
            && let Some(parts) = parts
        {
            parts.keep(&list);
        }
        events.into_iter()
    }

    /// [`Self::handle`], with the read of the pages the command brings onto the screen left to the
    /// host, so that a page turn can be drawn while it is read as page one is (ADR 1871).
    ///
    /// Everything the command does is done, in the order `handle` does it, except that the
    /// arrangement's unread pages stay unread and nothing is asked to be drawn of them. The host
    /// then reads the one page through [`Self::anticipating`] and takes it in with
    /// [`Self::anticipated`] — or calls [`Self::anticipate`] where `anticipating` answers `None` —
    /// and ends with [`Self::read_the_rest`], which a host must call: until it does, the pages
    /// the command brought on are neither read nor asked for.
    ///
    /// **The core still spawns nothing** (rule 4), and the read is the one `handle` would have
    /// made: [`Reader::read`] makes the first read `crate::open::interpret` makes of a page with
    /// no replacement, and `anticipating` hands out none for a page that holds one.
    pub fn handle_unread(&mut self, command: Command) -> impl Iterator<Item = Event> + use<> {
        self.unread = true;
        let events: Vec<Event> = self.handle(command).collect();
        self.unread = false;
        events.into_iter()
    }

    /// Reads whatever of the arrangement is still unread and asks for what is to be drawn — the
    /// end of a [`Self::handle_unread`].
    pub fn read_the_rest(&mut self) -> impl Iterator<Item = Event> + use<> {
        let mut events = Vec::new();
        self.settle(&mut events);
        events.into_iter()
    }
}

/// Page one's read and its first frame's target, borrowed from the viewer between
/// [`Viewer::anticipating`] and [`Viewer::anticipated`] (ADR 1855).
///
/// Both halves read the document's state and change none of it, so a host may run
/// [`Reader::read`] on one thread while [`Self::foresee`] is asked on another.
#[derive(Debug)]
pub struct Anticipating<'a> {
    open: &'a Open,
    reader: Reader<'a>,
    budget: u64,
}

/// The read half of [`Anticipating`]: what reading the page needs, and nothing of the document's
/// state that it does not.
#[derive(Debug)]
pub struct Reader<'a> {
    id: DocumentId,
    page: usize,
    object: pdf_model::Page,
    document: Arc<pdf_syntax::Document>,
    view: &'a pdf_model::view::ViewState,
    fonts: &'a pdf_model::FontCache,
    references: &'a pdf_model::reference::Supply,
}

/// What [`Reader::read`] read, for [`Viewer::anticipated`].
pub struct Anticipated {
    id: DocumentId,
    page: usize,
    /// The file it was read from, so that an answer for another file opened under the same id
    /// is told apart (trap 104).
    document: Arc<pdf_syntax::Document>,
    read: Option<crate::open::Read>,
    /// For a [`ReadOut`]'s read, its serial and the view it was read under, which the viewer may
    /// have moved past while it ran.
    out: Option<(u64, pdf_model::view::ViewState)>,
}

/// A page turn's read, owned, for a thread of the host's choosing while the viewer goes on
/// answering on the host's own (ADR 2344) — [`Viewer::read_out`]'s answer.
///
/// [`Reader`] borrows the viewer for the length of the read, so the host's thread can do nothing
/// else until it ends; this holds its own handles on the same document, font cache and target
/// files, and a copy of the view the page is read under.
#[derive(Debug)]
pub struct ReadOut {
    id: DocumentId,
    page: usize,
    object: pdf_model::Page,
    document: Arc<pdf_syntax::Document>,
    view: pdf_model::view::ViewState,
    fonts: Arc<pdf_model::FontCache>,
    references: Arc<pdf_model::reference::Supply>,
    serial: u64,
    foreseen: Option<Foreseen>,
}

/// The page a [`ReadOut`] is reading, as the viewer remembers it until the read comes back.
#[derive(Debug, Clone, Copy)]
pub(super) struct Out {
    pub(super) document: DocumentId,
    pub(super) page: usize,
    serial: u64,
}

impl ReadOut {
    /// Reads the page, handing its list over to `parts` as it is built — the read
    /// [`Reader::read`] makes.
    #[must_use]
    pub fn read(&self, parts: &pdf_model::progress::Parts) -> Anticipated {
        Anticipated {
            id: self.id,
            page: self.page,
            document: Arc::clone(&self.document),
            read: Some(parts.during(|| {
                crate::open::first_read(
                    &self.document,
                    self.object.clone(),
                    &self.view,
                    &self.fonts,
                    &self.references,
                )
            })),
            out: Some((self.serial, self.view.clone())),
        }
    }

    /// Where the page's first frame will be drawn, foretold when the read was handed out
    /// ([`Anticipating::foresee`]).
    #[must_use]
    pub const fn foreseen(&self) -> Option<Foreseen> {
        self.foreseen
    }

    /// Which page is being read, counted from zero.
    #[must_use]
    pub const fn page(&self) -> usize {
        self.page
    }
}

/// Where page one's first frame will be drawn, asked before the page has been read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Foreseen {
    /// The target the core will ask the page to be drawn at — [`RenderRequest::target`].
    pub target: TargetSpec,
    /// Where that raster sits in the viewport, in device pixels —
    /// [`crate::Answer::Geometry`]'s origin.
    pub origin: (f32, f32),
    /// Which page it is, counted from zero — the page the document opens at.
    pub page: usize,
}

/// Written by hand because a read is a whole interpretation, which nobody wants printed.
impl std::fmt::Debug for Anticipated {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Anticipated")
            .field("id", &self.id)
            .field("page", &self.page)
            .field("read", &self.read.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a> Anticipating<'a> {
    /// The read, for a thread of the host's choosing.
    #[must_use]
    pub fn reader(&self) -> &Reader<'a> {
        &self.reader
    }

    /// The target and placement the first frame of the page will be asked at, for a viewport of
    /// `viewport` device pixels at `scale` — by the same arithmetic the core's own request takes
    /// once it is told the viewport (`Open::magnification`, `TargetSpec::for_page`,
    /// `layout::origin_of`), so there is no second opinion about either (ADR 0118).
    ///
    /// **A forecast and not a promise**: what happens between now and the first resize can move
    /// it — a page that notices its magnification is read again, a resize lands first — so a host
    /// compares it with the request when the request comes, and draws the page whole where the two
    /// differ.
    #[must_use]
    pub fn foresee(&self, viewport: (u32, u32), scale: f32) -> Option<Foreseen> {
        let magnification = self.open.magnification(viewport, scale)?;
        let page = self.reader.page;
        let size = self.open.page_size(page)?;
        let target =
            TargetSpec::for_page(&DisplayList::new(size), magnification, self.budget).ok()?;
        let origin = crate::layout::origin_of(self.open, page, viewport, scale, magnification)?;
        Some(Foreseen {
            target,
            origin,
            page,
        })
    }
}

impl Reader<'_> {
    /// Reads the page, handing its list over to `parts` as it is built.
    #[must_use]
    pub fn read(&self, parts: &pdf_model::progress::Parts) -> Anticipated {
        Anticipated {
            id: self.id,
            page: self.page,
            document: Arc::clone(&self.document),
            read: Some(parts.during(|| {
                crate::open::first_read(
                    &self.document,
                    self.object.clone(),
                    self.view,
                    self.fonts,
                    self.references,
                )
            })),
            out: None,
        }
    }
}
