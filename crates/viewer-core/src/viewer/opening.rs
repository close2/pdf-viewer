//! Opening a document: the host's bytes taken into an [`Open`], page one interpreted, and a
//! finished render installed.

use std::sync::Arc;

use pdf_render::DisplayList;
use pdf_syntax::{FileBytes, SyntaxError};

use super::layout::damage;
use super::{DocumentId, RenderToken, Standing, Viewer};
#[cfg(doc)]
use crate::Command;
use crate::command::{Purpose, Rendered};
use crate::event::{Event, Extraction};
use crate::open::{Interpreted, Open};

impl Viewer {
    /// §8.10.4's target documents, parsed once here rather than once per open document.
    ///
    /// The files are the *reader's* policy and not any document's, so one supply is shared by every
    /// tab and a document opened later gets the same one — [`Command::Restrict`]'s rule for every
    /// host-supplied value. The refusals are kept rather than printed: what a host does with them
    /// is the host's, exactly as `viewer_host::trust_anchors` leaves an anchor's to its caller.
    pub(super) fn supply_references(&mut self, files: &crate::ReferenceFiles) {
        let (supply, refused) = pdf_model::reference::Supply::read(
            files
                .files
                .iter()
                .map(|(name, bytes)| (name.clone(), bytes.clone())),
            files.source.clone(),
        );
        let supply = Arc::new(supply);
        self.reference_refusals = refused;
        for open in self.documents.values_mut() {
            open.references = Arc::clone(&supply);
            // Every list this document has is a function of what could be imported, so the ones
            // already built are no longer the ones this policy would produce.
            open.stale();
        }
        self.references = supply;
    }

    /// The files [`Command::References`] offered that this reader will not import from.
    ///
    /// Named one by one rather than counted, for the reason `pdf_signature::trust::Supply` gives
    /// one crate over: a person who pointed at a directory of six files and got four target
    /// documents would otherwise be reading pages imported under a supply they did not supply.
    ///
    /// A query rather than an [`Event`], because a refusal here is a fact about the *host's*
    /// input and about no document — so the party that sent the command is the party that asks,
    /// and it may ask whenever it likes. Empty until [`Command::References`] has been sent.
    #[must_use]
    pub fn reference_refusals(&self) -> &[pdf_model::reference::Refusal] {
        &self.reference_refusals
    }

    /// Opens a document, or says why it could not be.
    pub(super) fn open(
        &mut self,
        id: DocumentId,
        bytes: FileBytes,
        password: Option<&crate::Secret>,
        fragment: Option<&str>,
        events: &mut Vec<Event>,
    ) {
        // A file held for a password is this open's only where the name is the one it was asked
        // under; any other open ends it, for `locked`'s reason.
        let hold = match self.locked.take().filter(|hold| hold.asked == id) {
            // §12.7.8's named page is drawn into the document that named it rather than shown, so
            // its retry reads the file and opens nothing (ADR 1335).
            Some(hold) if !hold.locked.opens_a_document() => {
                self.supply_held(hold, bytes, password, events);
                return;
            }
            hold => hold,
        };
        let limits = hold
            .as_ref()
            .map_or(pdf_syntax::Limits::DEFAULT, |hold| hold.locked.limits);
        match Open::new(bytes, password, limits) {
            Ok(mut open) => {
                let mut placed = None;
                if let Some(hold) = &hold {
                    match hold.locked.place(&mut open) {
                        Ok(note) => placed = Some(note),
                        Err(reason) => {
                            events.push(Event::OpenFailed {
                                document: id,
                                reason,
                            });
                            return;
                        }
                    }
                }
                // Every answer this reader has given, before anything reads the document — the
                // same list a document reached through an action is handed, so that a host naming
                // a file and a link naming one cannot come to hold two different documents.
                self.adopt(&mut open);
                // §12.11.6, and this is the moment the clause names: "Document requirements shall
                // be evaluated before execution of any document ECMAScripts. If requirements
                // cannot be met … then the processing of the document shall not continue."
                // Nothing below has run, so *not continuing* is a document that was read and put
                // down again rather than one half processed. The window's levels rather than the
                // document's, because a departure belongs to a document that is open and this one
                // is not (ADR 1145, ADR 1167).
                let level = self
                    .restrictions
                    .level(pdf_model::restriction::Operation::Process)
                    .level();
                let warned = match Self::verdict_of(
                    id,
                    level,
                    &open.document,
                    pdf_model::restriction::Operation::Process,
                    None,
                    None,
                ) {
                    Standing::Refuse(refused) => {
                        events.push(refused);
                        return;
                    }
                    Standing::Ask(notes) => {
                        open.asking = Some(crate::open::Held::Process {
                            fragment: fragment.map(str::to_owned),
                        });
                        // Held in the map so that `Command::Answer` can name it, and deliberately
                        // not focused: nothing has been processed, no `Event::Opened` has been
                        // raised, and a question about a document is not that document arriving.
                        self.documents.insert(id, open);
                        events.push(Event::Asking {
                            document: id,
                            operation: pdf_model::restriction::Operation::Process,
                            notes,
                        });
                        return;
                    }
                    Standing::Warn(notes) => Some(notes),
                    Standing::Proceed => None,
                };
                self.documents.insert(id, open);
                self.process(id, fragment, events);
                if let Some(note) = placed {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![note],
                    });
                }
                if let Some(notes) = warned {
                    events.push(Event::Warned {
                        document: id,
                        operation: pdf_model::restriction::Operation::Process,
                        notes,
                    });
                }
            }
            // §7.6.4.1's prompt, and the reason this is not an `OpenFailed`: a document that
            // wants a password is not a document this program cannot read.
            Err(SyntaxError::PasswordRequired) => {
                // Held again for the next attempt, which a wrong password is owed.
                self.locked = hold;
                events.push(Event::PasswordRequired { document: id });
            }
            Err(error) => events.push(Event::OpenFailed {
                document: id,
                reason: error.to_string(),
            }),
        }
    }

    /// Hands a document this window has just acquired every answer its reader has already given.
    ///
    /// The one list, asked by [`Self::open`] for a document a host named — the first file, a
    /// second on the command line, a file a person chose — and where an action's replacement is
    /// taken in, for a document §12.6.4.3's or §12.6.4.4's action reached. Every one of these
    /// values is documented as applying to "every open document and to every one opened
    /// afterwards", and a document shown in this window by this reader is one of those whoever
    /// named it. ADR 1263.
    pub(super) fn adopt(&self, open: &mut Open) {
        // §8.10.4's target documents are the reader's policy rather than any document's, so a
        // document opened after `Command::References` gets the same supply as one opened before
        // it — `Command::Restrict`'s rule for every host-supplied value.
        open.references = Arc::clone(&self.references);
        // §8.11.4.4's two categories about this reader, on the same rule: a host that said who is
        // reading said it about every document it will show. Free where nothing was said, which
        // is every host by default.
        open.view.set_audience(self.audience.clone());
        // Table 166's `/M`, on the same rule: a host with a clock has one for every document it
        // will show. No entry where nothing was said, which is every host by default.
        open.view.set_modification_time(self.clock);
        // §10.8.3's simulation, on the same rule: a reader who asked for it asked about every
        // document this window will show. Off where nothing was said, which is every host by
        // default.
        open.view.set_separation_simulation(self.separations);
        // RFC 0008 section 6.3's level, on the same rule: a reader who let scripts run, or asked
        // to be asked, said it about every document this window will show (ADR 1616).
        let runner = crate::scripting::runner_for(&self.scripting, &mut open.consent);
        open.view.run_scripts_with(runner);
        // The keys, on the same rule: a key held is held over every document (ADR 1771).
        open.view.set_keys(self.keys);
        // A document opened *during* a presentation arrives in the mode the host is in: §12.4.4.2's
        // node is a property of the page being shown and NOTE 2's saved groups of the document, so
        // both are taken here rather than only on `Command::Present`.
        if self.presenting == crate::PresentationMode::On {
            crate::presentation::enter(open);
        }
    }

    /// Everything opening a document does once §12.11.6 has let it go on.
    ///
    /// Split out of [`Self::open`] because the *ask* level puts a person between the two: a
    /// document held at [`crate::Event::Asking`] has been read and nothing more, and what a `yes`
    /// releases is this. ADR 1167.
    pub(super) fn process(
        &mut self,
        id: DocumentId,
        fragment: Option<&str>,
        events: &mut Vec<Event>,
    ) {
        let Some((pages, mut notes)) = self.documents.get_mut(&id).map(|open| {
            // What opening the document has already discovered about §7.5.7's storage — the
            // catalogue and the page tree are read by now, and either may live in an object
            // stream. The rest arrives per page, below.
            //
            // **What the document says about *itself* is not here, and that is principle 2.**
            // `notes::about` answers clauses about the file, and §12.8's answer reads and
            // digests the signed byte ranges — on a signed document that is the whole file, and
            // none of it draws a page. `Command::Report` is what asks for it, and `Open::about`
            // is what makes asking twice cost once. ADR 1044.
            (open.page_count, crate::notes::losses(open))
        }) else {
            return;
        };
        // Annex O's open parameters, and this is where the annex puts them: §O.2.2 says they
        // "should be processed immediately after any other document-specified open parameters
        // have been processed", and `Open::around` has just processed Table 29's `/OpenAction`.
        // So the document states where it opens and the URI overrules it, in that order, which is
        // what a fragment identifier is for.
        if let Some(fragment) = fragment
            && let Some(open) = self.documents.get_mut(&id)
        {
            notes.extend(open.apply_fragment(&pdf_model::fragment::Fragment::parse(fragment)));
        }
        self.focused = Some(id);
        events.push(Event::Opened {
            document: id,
            pages,
        });
        if !notes.is_empty() {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes,
            });
        }
        // Annex O's `ef`, if the fragment named one. Before the first page's events
        // because it is the *document* that was asked for a file rather than the view:
        // Table Annex O.3 files this parameter under object identifiers, and the file is
        // out of the bytes already read rather than out of a page nobody has drawn. The
        // same channel `Command::Extract` uses, so a host needed no new message and every
        // one of the six already handles it (ADR 0310). What travels with the bytes since
        // ADR 0431 is the rest of the fragment: "[a]ny remaining parameters after this
        // parameter apply to the selected embedded file", so they go where the file goes.
        if let Some(file) = self
            .documents
            .get_mut(&id)
            .and_then(|open| open.opening_file.take())
        {
            self.extract(&file.name, Extraction::Fragment, file.fragment, events);
        }
        self.announce_page_reading(false, events);
        // §12.6.3 puts `/PO` "after … the OpenAction entry in the document Catalog",
        // and `Open::around` has already applied that entry's destination — the page it
        // names is `open.page_index` and its view is waiting in `pending_views` — so the
        // first page's events are raised here, in the clause's order. An `/OpenAction`
        // that is an action rather than a destination is still not *performed*; that is
        // §12.6.4's row and not this one's, and it changes nothing about this ordering.
        self.page_events(id, None, events);
        // Annex O's `search`, if the fragment asked for one. The plan is made and no page
        // has been read: this event is what tells a host there is something to pump, and
        // it is the same division `Event::NeedsRender` makes — a unit of work handed over
        // rather than done on the launch path.
        if let Some(remaining) = self
            .documents
            .get(&id)
            .and_then(|open| open.searching.as_ref())
            .map(|searching| searching.remaining)
        {
            events.push(Event::Searched {
                document: id,
                found: None,
                remaining,
                wrapped: false,
            });
        }
        // Annex O's `fdf`, if the fragment named one — "[o]pen the document and then
        // import the data from the specified FDF or XFDF file". **Last of the four
        // things a fragment can start**, which is the annex's own order: the `fdf`
        // parameter "is recommended to be the last parameter so that the document can
        // open directly to the appropriate view", and the view is what everything above
        // has just settled. The name crosses as the document's own words, exactly as
        // §12.7.6.4's does, and a host is what resolves or refuses it (rule 2).
        if let Some(name) = self
            .documents
            .get(&id)
            .and_then(|open| open.importing.as_ref())
            .map(|import| import.file.clone())
        {
            events.push(Event::NeedsFile {
                document: id,
                purpose: Purpose::ImportData,
                name,
                beside: false,
            });
        }
    }

    /// Tells the page what a draw of it declined, each condition once while its reading stands
    /// (`Command::Declined`, ADR 2290).
    pub(super) fn declined(
        &mut self,
        document: DocumentId,
        page: usize,
        declined: Vec<crate::Declined>,
        events: &mut Vec<Event>,
    ) {
        let Some(interpreted) = self
            .documents
            .get_mut(&document)
            .and_then(|open| open.on_screen.iter_mut().find(|shown| shown.page == page))
            .and_then(|shown| shown.interpreted.as_mut())
        else {
            return;
        };
        let mut notes = Vec::new();
        for decline in declined {
            if interpreted.declined.contains(&decline.condition) {
                continue;
            }
            interpreted.declined.push(decline.condition);
            interpreted.reports.push(decline.sentence.clone());
            notes.push(decline.sentence);
        }
        if !notes.is_empty() {
            events.push(Event::Reported {
                document,
                page: Some(page),
                notes,
            });
        }
    }

    /// Records what a worker did with a request, or drops it for being about the past.
    pub(super) fn rendered(
        &mut self,
        token: RenderToken,
        rendered: Rendered,
        events: &mut Vec<Event>,
    ) {
        // A token that is not one of those outstanding answers a question that has been asked
        // again since. Dropping it is the whole reason the token exists — and an arrangement has
        // one outstanding request per page on the screen rather than one, so the token is what
        // says *which* page an answer is about.
        //
        // **Of which document, too**: tokens are issued from one counter for every document this
        // viewer holds, and a document opened beside the one showing is asked for its first page
        // and may be given back the front before the answer lands. Looked up in the focused
        // document alone, that answer was dropped, the request stayed outstanding and the tab never
        // drew (ADR 1275). The damage is the window's only where the page is the front's.
        let damage_of = |id: DocumentId, focused: Option<DocumentId>, viewport| {
            (focused == Some(id)).then(|| damage(viewport))
        };
        let (viewport, focused) = (self.viewport, self.focused);
        let Some((id, open, index)) = self.documents.iter_mut().find_map(|(id, open)| {
            open.on_screen
                .iter()
                .position(|on_screen| {
                    on_screen
                        .pending
                        .as_ref()
                        .is_some_and(|pending| pending.token == token)
                })
                .map(|index| (*id, open, index))
        }) else {
            return;
        };
        let Some(pending) = open.on_screen[index].pending.take() else {
            return;
        };
        let on_screen = &mut open.on_screen[index];
        match rendered {
            Rendered::Raster(raster) => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = Some(raster);
                events.extend(damage_of(id, focused, viewport));
            }
            // Tier 2: the host drew it onto its own surface, so there is nothing here to hold
            // and nothing to repaint from — but it *is* on the screen, and saying so is what
            // stops the scheduler asking for it again.
            Rendered::Presented => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = None;
                // Said once and remembered: this host draws its own frames at its own size, so
                // nothing here will hold a whole-page raster for it and `MAX_PIXELS` has
                // nothing to bound.
                self.holds_rasters = false;
            }
            // **The same two lines as above and deliberately not the third.** The host took this
            // page's display list, so there are no pixels here either — but that is a fact about
            // one page and not about the host, and a host that mixes the two arms will hand the
            // next page back as a raster. Concluding `holds_rasters = false` from it would take
            // `MAX_PIXELS` off every request the mixed host makes and silence `Query::Frame`
            // about the pages it *is* holding pixels for. See `Rendered::Listed`.
            //
            // Damaged like a raster rather than silent like a presentation: the host has what it
            // needs to draw the page and has not said it drew it.
            Rendered::Listed => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = None;
                events.extend(damage_of(id, focused, viewport));
            }
            // **A refusal is recorded as an answer**, and it has to be: the scheduler's question
            // is "is what is on the screen what should be", and a host that cannot draw this page
            // at this resolution will say so again the next time it is asked. Without this the
            // two of them spin — ask, refuse, ask — for as long as the page is shown. What
            // changes the answer is the question changing: another page, another zoom, another
            // interpretation, all of which move the tuple below.
            Rendered::Failed(reason) => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = None;
                events.push(Event::Reported {
                    document: id,
                    page: Some(pending.page),
                    notes: vec![reason],
                });
            }
        }
    }

    /// Takes one page's read into the arrangement's entry at `index`, and answers the list it
    /// holds — [`Self::interpret_arranged`]'s step, and [`Self::anticipated`]'s.
    pub(super) fn install(
        open: &mut Open,
        id: DocumentId,
        index: usize,
        read: crate::open::Read,
        events: &mut Vec<Event>,
    ) -> Option<Arc<DisplayList>> {
        let crate::open::Read {
            interpretation,
            mut reports,
            object,
            replaceable,
        } = read;
        let page = open.on_screen.get(index)?.page;
        {
            // §7.5.7's losses become known when a page reaches into an object stream, which is
            // here and never at open. The sentence is the document's rather than the page's and
            // is said once — `crate::notes::losses` — so a page carries it only the first time.
            reports.extend(crate::notes::losses(open));
            if !reports.is_empty() {
                events.push(Event::Reported {
                    document: id,
                    page: Some(page),
                    notes: reports.clone(),
                });
            }
            open.revision = open.revision.saturating_add(1);
            // The page a person is looking at was just interpreted, so a search that starts here
            // — which every find bar's does — need not interpret it again. `doc/todo/49`'s fourth
            // item is exactly this: the same page read for a search and then again to draw it was
            // two interpretations. One 2.6 KB copy per page turn against 5.4 ms of the thing it
            // saves.
            let order = crate::select::Order::of(&interpretation.text, &interpretation.text_layer);
            open.readbacks.put(
                page,
                &Arc::from(interpretation.text.as_str()),
                &Arc::new(order),
            );
            let revision = open.revision;
            let on_screen = &mut open.on_screen[index];
            on_screen.object = object;
            on_screen.revision = revision;
            // A `None` is a read that *used* the replacement this page already holds rather than
            // one that made a new one, so it says nothing about whether to keep it; see
            // `open::Read::replaceable`.
            if replaceable.is_some() {
                on_screen.replaceable = replaceable;
            }
            on_screen.interpreted = Some(Interpreted {
                shortfall: interpretation.shortfall(),
                list: Arc::new(interpretation.display_list),
                reports,
                text: interpretation.text,
                placed: interpretation.text_layer,
                marked: interpretation.marked,
                artifacts: interpretation.artifacts,
                objects: interpretation.objects,
                described: interpretation.described,
                language: interpretation.language,
                view_dependent: interpretation.view_dependent,
                declined: Vec::new(),
            });
        }
        open.on_screen
            .get(index)
            .and_then(|on_screen| on_screen.interpreted.as_ref())
            .map(|interpreted| Arc::clone(&interpreted.list))
    }
}
