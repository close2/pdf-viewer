//! What an interaction asked of the host and what the host answered: files, passwords and the work
//! held behind them.

use pdf_syntax::{FileBytes, SyntaxError};

use super::{DocumentId, Hold, Turn, Viewer};
#[cfg(doc)]
use crate::Command;
use crate::command::Purpose;
use crate::event::Event;
use crate::interact;
use crate::open::Open;

impl Viewer {
    /// Takes the bytes a host was asked for, or says that it declined.
    pub(super) fn supply(
        &mut self,
        purpose: Purpose,
        bytes: Option<&[u8]>,
        events: &mut Vec<Event>,
    ) {
        let Some(id) = self.focused else { return };
        // A decline that finds a file held for §7.6.4.1's password is about that file: its prompt
        // was cancelled or ran out of attempts, and the hold ends here rather than waiting for
        // the next open of any name (ADR 1345).
        if bytes.is_none()
            && let Some(hold) = self
                .locked
                .take_if(|hold| hold.source == id && hold.locked.purpose() == purpose)
        {
            self.withdraw(&hold, events);
            return;
        }
        let Some(open) = self.focused_mut() else {
            return;
        };
        let outcome = match (purpose, bytes) {
            (Purpose::ImportData, Some(bytes)) => {
                interact::import(open, bytes, interact::Arrival::Action)
            }
            // §12.6.4.4's suspended walk, resumed against the root that arrived.
            (Purpose::TargetRoot, Some(bytes)) => interact::resume_root(open, bytes),
            // §12.6.4.3's jump, made against the file Table 203's `/F` named.
            // §12.6.4.6's launch runs the same route, and `resume_remote` reads the bytes'
            // header before it opens anything (ADR 1358).
            (Purpose::RemoteDocument | Purpose::LaunchDocument, Some(bytes)) => {
                interact::resume_remote(open, bytes)
            }
            // Trap 5 on the one path where a *host* declines: a click that silently does
            // nothing is indistinguishable from a click on nothing.
            (Purpose::ImportData, None) => {
                let named = open
                    .importing
                    .take()
                    .map_or_else(String::new, |import| format!(" {}", import.file));
                let mut outcome = interact::Outcome::default();
                outcome
                    .notes
                    .push(format!("import-data: declined —{named} was not supplied"));
                outcome
            }
            (Purpose::TargetRoot, None) => interact::decline_root(open),
            (Purpose::RemoteDocument | Purpose::LaunchDocument, None) => {
                interact::decline_remote(open)
            }
            // §12.7.8's named page in a second file, and §12.6.4.7's thread in one (ADR 1239).
            (Purpose::NamedPage, Some(bytes)) => interact::supply_named_page(open, bytes),
            (Purpose::NamedPage, None) => interact::decline_named_page(open),
            (Purpose::ThreadDocument, Some(bytes)) => interact::resume_threaded(open, bytes),
            (Purpose::ThreadDocument, None) => interact::decline_threaded(open),
        };
        self.apply(id, outcome, events);
    }

    /// Applies §12.7.8's form data a server answered a submission or a fetched import with —
    /// [`Command::Respond`].
    ///
    /// The document in front takes the whole of [`Self::apply`], exactly as an import-data action's
    /// file does. One behind it takes the values and the sentences, and is drawn again when it next
    /// comes to the front; what it cannot take is Table 253's second file, because
    /// [`Command::Supply`] answers the document in front, so that is declined out loud rather than
    /// asked for on behalf of a document nobody is looking at.
    pub(super) fn respond(
        &mut self,
        document: DocumentId,
        answers: crate::Answered,
        source: String,
        format: pdf_model::action::DataFormat,
        bytes: &[u8],
        events: &mut Vec<Event>,
    ) {
        let in_front = self.focused == Some(document);
        let Some(open) = self.documents.get_mut(&document) else {
            return;
        };
        open.importing = Some(pdf_model::action::ImportData {
            file: source,
            format,
        });
        // A fetched import reads as the import it is, whichever route asked for it, so that the
        // same parameter says the same sentence whether its file was beside the document or on a
        // server (ADR 1527).
        let arrival = match answers {
            crate::Answered::Submission => interact::Arrival::Answer,
            crate::Answered::Import => interact::Arrival::Action,
        };
        let outcome = interact::import(open, bytes, arrival);
        if in_front {
            self.apply(document, outcome, events);
            return;
        }
        Self::apply_behind(document, open, outcome, events);
    }

    /// What an outcome about a document behind the one in front can still do: its values, its
    /// sentences, and a redraw when it next comes to the front.
    ///
    /// Table 253's second file is declined out loud rather than asked for, because
    /// [`Command::Supply`] answers the document in front and a question on behalf of a document
    /// nobody is looking at is one nobody can place.
    fn apply_behind(
        document: DocumentId,
        open: &mut Open,
        mut outcome: interact::Outcome,
        events: &mut Vec<Event>,
    ) {
        if outcome.redraw {
            open.stale();
        }
        if !outcome.needs_file.is_empty() {
            outcome.needs_file.clear();
            outcome
                .notes
                .extend(interact::decline_named_page(open).notes);
        }
        if !outcome.notes.is_empty() {
            events.push(Event::Reported {
                document,
                page: Some(open.page_index),
                notes: outcome.notes,
            });
        }
    }

    /// §12.7.8's named page whose file asked for §7.6.4.1's password, retried with one.
    ///
    /// The file is read under the bounds of the document that named it and drawn into that
    /// document, which is the only one it was ever for. Nothing opens under the name the prompt was
    /// raised under, so where that name is not the source's own it is closed, which is how a host
    /// holding it for the file learns the file has been read (ADR 1335).
    pub(super) fn supply_held(
        &mut self,
        hold: Hold,
        bytes: FileBytes,
        password: Option<&crate::Secret>,
        events: &mut Vec<Event>,
    ) {
        let interact::Held::NamedPage(file) = &hold.locked.held else {
            return;
        };
        let file = file.clone();
        let read = Open::read(bytes, password, hold.locked.limits);
        if matches!(read, Err(SyntaxError::PasswordRequired)) {
            let asked = hold.asked;
            self.locked = Some(hold);
            events.push(Event::PasswordRequired { document: asked });
            return;
        }
        if hold.asked != hold.source {
            events.push(Event::Closed(hold.asked));
        }
        let in_front = self.focused == Some(hold.source);
        // The document that named the file, where it is still waiting for this one: a file whose
        // references were declined meanwhile has nothing left to give.
        let Some(open) = self
            .documents
            .get_mut(&hold.source)
            .filter(|open| open.view.file_awaited() == Some(file.as_str()))
        else {
            return;
        };
        let outcome = interact::named_pages_from(open, &file, read);
        if in_front {
            self.apply(hold.source, outcome, events);
        } else {
            Self::apply_behind(hold.source, open, outcome, events);
        }
    }

    /// A file held for §7.6.4.1's password, declined by the host that was asking about it.
    ///
    /// The name the prompt was raised under is given back where it is not the source's own, as
    /// after a named page's password was right, so a host holding it learns the file is settled
    /// either way. What the decline says and leaves is [`interact::withdraw`]'s (ADR 1345).
    fn withdraw(&mut self, hold: &Hold, events: &mut Vec<Event>) {
        if hold.asked != hold.source {
            events.push(Event::Closed(hold.asked));
        }
        let Some(open) = self.documents.get_mut(&hold.source) else {
            return;
        };
        let outcome = interact::withdraw(open, &hold.locked);
        self.apply(hold.source, outcome, events);
    }

    /// Turns what a click asked for into events, and does the parts that are this crate's.
    pub(super) fn apply(
        &mut self,
        id: DocumentId,
        mut outcome: interact::Outcome,
        events: &mut Vec<Event>,
    ) {
        let page = self.focused().map(|open| open.page_index);
        // Table 203's and Table 204's `/NewWindow true`, decided here because this is where the
        // two halves meet: `interact` read the entry, and whether this program has a second place
        // to put a document is the name `Command::Beside` left in reserve. Both outcomes are said
        // out loud — a person who asked for a second window and got one view is owed the
        // difference, and so is one who got a second tab they did not ask this program for
        // (trap 5, ADR 1263).
        let into = if (outcome.replacement.is_some() || outcome.locked.is_some()) && outcome.beside
        {
            let reserved = self.beside.take();
            outcome.notes.push(match reserved {
                Some(_) => "this link asks for a new window: the destination opens beside the \
                            document it was reached from"
                    .to_owned(),
                None => "this link asks for a new window; this view has one, so the destination \
                         replaces what was open"
                    .to_owned(),
            });
            reserved
        } else {
            None
        };
        if !outcome.notes.is_empty() {
            events.push(Event::Reported {
                document: id,
                page,
                notes: outcome.notes,
            });
        }
        for uri in outcome.uris {
            events.push(Event::OpenUri { document: id, uri });
        }
        for submission in outcome.submissions {
            events.push(Event::Submit {
                document: id,
                submission: Box::new(submission),
            });
        }
        self.ask_for_files(id, outcome.needs_file, events);
        for clip in outcome.media {
            self.media(id, clip, events);
        }
        if let Some(locked) = outcome.locked {
            self.hold(id, into, locked, events);
        }
        for transition in outcome.transitions {
            if let Some(note) = crate::transition::note(&transition) {
                events.push(Event::Reported {
                    document: id,
                    page,
                    notes: vec![note],
                });
            }
            events.push(Event::Transition {
                document: id,
                transition,
            });
        }

        // §12.6.4.4 replaces the document every other request was about, so a page named by one
        // of them is a page of a document that is no longer open — unless the action asked for a
        // window of its own and this host reserved a name for one, in which case the document
        // that was open is still open and still focused.
        if let Some(mut replacement) = outcome.replacement {
            let pages = replacement.page_count;
            let target = into.unwrap_or(id);
            // Every host-supplied value applies to "every open document and to every one opened
            // afterwards", which a document reached through an action is: it is shown in this
            // window, by this reader, under the answers they gave (ADR 1263).
            self.adopt(&mut replacement);
            self.documents.insert(target, *replacement);
            // A document that opens beside the one showing is *focused*, which is
            // `Command::Open`'s own rule and not a second decision: a person who followed a link
            // asking for a new window asked to be reading the destination. A host that wants the
            // source back in front says so with `Command::Focus`, and it knows which document
            // arrived from the name on the event below.
            self.focused = Some(target);
            events.push(Event::Opened {
                document: target,
                pages,
            });
            // What the replacement says about itself waits for `Command::Report`, exactly as the
            // document it replaced did: an embedded go-to is an open, and an open draws a page
            // before it digests a signature (ADR 1044).
            self.announce_page(events);
            self.page_events(target, None, events);
            return;
        }

        let Some(open) = self.focused_mut() else {
            return;
        };
        if outcome.redraw {
            open.stale();
        }
        // Even a destination naming the page already showing states where to look at it, which
        // is what an outline item pointing at a heading half way down a page is for.
        if let Some(view) = outcome.view {
            open.pending_views = vec![view];
        }
        if let Some(target) = outcome.target
            && target != open.page_index
        {
            let left = open.page_index;
            open.page_index = target;
            open.scroll = (0.0, 0.0);
            // §12.4.4.1's clock is the page's, and an action that jumped has changed the page.
            open.shown_for = 0.0;
            self.announce_page(events);
            self.page_events(id, Some(left), events);
            // §12.4.4.2 names this one: "the navigation request was for random access (such as by
            // clicking on a link)", which the clause treats as forward.
            self.arrive(id, true, Turn::Requested, events);
        }
    }

    /// Every file an outcome asked a host for, as `Event::NeedsFile`.
    ///
    /// Table 203's and Table 204's `/NewWindow true` travels on the question, so that a host
    /// asking a person about the file can say where it would open (ADR 1335).
    fn ask_for_files(
        &self,
        id: DocumentId,
        files: Vec<(Purpose, String)>,
        events: &mut Vec<Event>,
    ) {
        let (remote_beside, root_beside) = self.documents.get(&id).map_or((false, false), |open| {
            (
                open.opening
                    .as_ref()
                    .is_some_and(|remote| remote.new_window == Some(true)),
                open.resuming
                    .as_ref()
                    .is_some_and(|target| target.new_window == Some(true)),
            )
        });
        for (purpose, name) in files {
            events.push(Event::NeedsFile {
                document: id,
                purpose,
                name,
                beside: match purpose {
                    Purpose::RemoteDocument | Purpose::LaunchDocument => remote_beside,
                    Purpose::TargetRoot => root_beside,
                    Purpose::ImportData | Purpose::NamedPage | Purpose::ThreadDocument => false,
                },
            });
        }
    }

    /// Holds a file that asked for §7.6.4.1's password, and raises the prompt about it.
    ///
    /// The prompt is under the name the file would open under: for a jump, `into` — the reserve,
    /// where the action asked for a window of its own and the host offered one — and the document
    /// it was reached from otherwise, which is what `/NewWindow false` replaces (ADR 1332).
    /// §12.7.8's named page opens under no name, so it is asked about under the reserve where the
    /// host offered one — a name no tab has, which the retry then closes — and under the source
    /// otherwise (ADR 1335).
    fn hold(
        &mut self,
        id: DocumentId,
        into: Option<DocumentId>,
        locked: Box<interact::Locked>,
        events: &mut Vec<Event>,
    ) {
        let asked = if locked.opens_a_document() {
            into.unwrap_or(id)
        } else {
            self.beside.take().unwrap_or(id)
        };
        self.locked = Some(Hold {
            asked,
            source: id,
            locked,
        });
        events.push(Event::PasswordRequired { document: asked });
    }
}
