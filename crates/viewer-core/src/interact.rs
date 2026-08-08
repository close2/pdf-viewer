//! What a click does: §12.5.6.5's links, and the §12.6 actions behind them.
//!
//! The interesting half of "the mouse followed a link" is a clause. The pointer is four lines;
//! the rest is Table 176's activation region, §12.5.2's coordinate space, §7.7.3.3's rotation,
//! and the eleven actions of §12.6 this program performs — two of which change what the *current*
//! page draws, so a click may repaint without going anywhere.
//!
//! Everything here is a function of an open document and a point. Nothing here touches a screen,
//! a filesystem or a clock: what the caller must do about the result is an [`Outcome`], and the
//! two things this program cannot do itself — resolve a URI, read a file — leave as requests.

use pdf_model::Pages;
use pdf_model::action::{Action, EmbeddedGoTo, ImportData, Trigger};
use pdf_model::navigation::Transition;
use pdf_model::view::{Pointer, Request};
use pdf_syntax::{Dictionary, Document, ObjectId};

use crate::command::Purpose;
use crate::open::Open;

/// What activating something asks of the caller.
///
/// A struct rather than an enum because one click legitimately does several of these: §12.6.2
/// makes an action's `/Next` a *sequence*, so a link may switch a layer off, import a file and
/// then jump to a page, and dropping any of them would be dropping something the document
/// asked for.
#[derive(Debug, Default)]
pub(crate) struct Outcome {
    /// Sentences to say out loud, including every refusal.
    pub(crate) notes: Vec<String>,
    /// §12.6.4.8: URIs to resolve somewhere this program is not.
    pub(crate) uris: Vec<String>,
    /// §12.7.6.4: a file the document asked for, which only the host can fetch.
    pub(crate) needs_file: Option<(Purpose, String)>,
    /// §12.4.4: transitions to play, for a caller that has one.
    pub(crate) transitions: Vec<Transition>,
    /// The page to show, where a request named one.
    pub(crate) target: Option<usize>,
    /// Table 149's view of that page, where the destination that named it stated one.
    ///
    /// Carried beside the page rather than resolved here because §12.3.2.1's other two items —
    /// "[t]he location of the document window on that page" and "[t]he magnification (zoom)
    /// factor" — need a viewport and a display list, and this function has neither.
    pub(crate) view: Option<pdf_model::destination::View>,
    /// §12.6.4.4: a document from inside this one, which replaces it.
    pub(crate) replacement: Option<Box<Open>>,
    /// Whether what is on the screen has to be drawn again.
    pub(crate) redraw: bool,
}

/// Whether this dictionary is one of §12.5.6.14's popup annotations.
fn is_popup(document: &Document, dict: &Dictionary) -> bool {
    document
        .get_key(dict, "Subtype")
        .as_name()
        .is_some_and(|subtype| subtype.as_bytes() == b"Popup")
}

/// The annotation under a point in default user space, where it is a link.
pub(crate) fn link_at(open: &Open, x: f32, y: f32) -> Option<ObjectId> {
    let links = pdf_model::link::links(&open.document, open.shown_page()?);
    pdf_model::link::at(&links, x, y).and_then(|link| link.id)
}

/// Whether the pointer being here changes what the annotation looks like.
///
/// Asked before the pointer state is changed at all, because changing it invalidates the page's
/// display list: a cursor crossing an annotation whose picture cannot differ would otherwise
/// re-interpret the page — 2 000 M instructions — for nothing.
///
/// **The two ends of a press are two different questions.** A hover shows Table 170's `/R`,
/// which most annotations do not state. A press shows `/D` *or* draws §12.5.6.19's `/H` mark,
/// whose default is `I` — so an annotation stating neither entry still changes under a press.
/// Both are `pdf_model`'s to answer: each is several clauses rather than one lookup, and this
/// crate reading an `/AP` for itself is how the hovering half missed §12.5.3's `ToggleNoView`.
pub(crate) fn has_appearance(open: &Open, annotation: ObjectId, pointer: Pointer) -> bool {
    let view = open.view.annotation(annotation);
    match pointer {
        Pointer::Down => {
            pdf_model::view::press_changes_appearance(&open.document, annotation, view)
        }
        Pointer::Over => {
            pdf_model::view::hover_changes_appearance(&open.document, annotation, view)
        }
    }
}

/// Activates whatever is at a point in default user space.
///
/// Returns an empty outcome for a click on nothing, on a link whose action this program will not
/// perform, or on a link to the page already shown.
pub(crate) fn activate(open: &mut Open, x: f32, y: f32) -> Outcome {
    let Some(page) = open.shown_page() else {
        return Outcome::default();
    };
    let links = pdf_model::link::links(&open.document, page);
    let Some(link) = pdf_model::link::at(&links, x, y) else {
        return Outcome::default();
    };
    let (actions, destination, rect) = (link.actions.clone(), link.destination, link.rect);
    drop(links);
    perform(
        open,
        &actions,
        destination,
        Some(((x, y), rect)),
        "this link",
    )
}

/// Activates an object the *host* named, which is §12.3.3's outline item and nothing else yet.
///
/// The clause: "[c]licking the text of any visible item activates the item, causing the
/// interactive PDF processor to jump to a destination or trigger an action associated with the
/// item." A host has a row in a panel and no way to read `/A`; what it hands over is the item's
/// object, and everything about what activation *means* stays on this side with the document —
/// which is the same division `Command::GoTo(PageTarget::Destination)` makes for the half of the
/// sentence that is a jump.
///
/// Table 151 gives an item `/Dest` **or** `/A` and forbids both, so the two are read in that
/// order and the second is a whole `/Next` chain. An object that is not an outline item states
/// neither and activates nothing, which is the right answer for a host that named the wrong
/// thing.
pub(crate) fn activate_object(open: &mut Open, id: ObjectId) -> Outcome {
    let object = open.document.get(id);
    let Some(dict) = object.as_dict() else {
        return Outcome::default();
    };
    // §12.5.1's other reading of "activate", for a host with no pointer on the object: an
    // annotation carrying one of §12.5.6.14's windows exhibits it. Asked before `/A` and `/Dest`
    // because a markup annotation states neither and would otherwise activate nothing — and a
    // popup is not an outline item, so nothing here can mean both.
    if pdf_model::popup::popup_of(dict).is_some() || is_popup(&open.document, dict) {
        return Outcome {
            redraw: open.toggle_popup(id),
            ..Outcome::default()
        };
    }
    let actions = pdf_model::action::read(
        &open.document,
        dict.get("A").unwrap_or(&pdf_syntax::Object::Null),
    );
    // Table 151's `/Dest` first, then a go-to anywhere in the chain — the same order and the
    // same reason as `pdf_model::link`'s: reading only the outermost `/S` would miss a jump
    // §12.6.2 put after a sound.
    let destination = dict
        .get("Dest")
        .and_then(|dest| pdf_model::destination::Destination::read(&open.document, dest))
        .or_else(|| {
            actions.iter().find_map(|action| match action {
                Action::GoTo(destination) => Some(*destination),
                _ => None,
            })
        });
    // §12.4.3's thread is the third thing a host can hand this, and it states neither `/A` nor
    // `/Dest`: a thread dictionary is Table 162's `/F`, `/I` and nothing that names a place. So
    // what activating one means has to be *composed* — and §12.6.4.7 has already composed it, as
    // the action a file writes to do the same job. Turning the object into that action's own
    // `ThreadTarget::Object` reuses `Request::Thread` whole, down to Table 163's `/R` framing the
    // bead rather than the page it sits on, rather than adding a second route to one place.
    if actions.is_empty() && destination.is_none() && is_thread(&open.document, dict) {
        let jump = pdf_model::action::ThreadJump {
            thread: pdf_model::action::ThreadTarget::Object(id),
            // Table 209 makes `/B` optional and says what its absence means; a thread activated
            // from a list has named no bead either, so it means the same thing.
            bead: None,
        };
        return perform(
            open,
            &[Action::Thread(jump)],
            None,
            None,
            "this article thread",
        );
    }
    // **No position.** §12.6.4.8's `/IsMap` "applies only to actions triggered by the user's
    // clicking an annotation; it shall be ignored for actions associated with outline items" —
    // so the clause itself says what a caller with no cursor position does here.
    perform(open, &actions, destination, None, "this item")
}

/// Whether a dictionary is one of §12.4.3's article threads.
///
/// Table 162 gives a thread dictionary an optional `/Type /Thread` and a **required** `/F`, "[a]n
/// indirect reference to the first bead in the thread", so the entry that must be there is the one
/// to test and the one that may be there is the one to accept. A bead is not mistaken for a thread
/// by either: Table 163 names a bead's own first entry `/T`, pointing the other way.
fn is_thread(document: &Document, dict: &Dictionary) -> bool {
    if document
        .get_key(dict, "Type")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
        == Some(b"Thread")
    {
        return true;
    }
    dict.get("F")
        .and_then(pdf_syntax::Object::as_reference)
        .is_some()
        && document.get_key(dict, "F").as_dict().is_some_and(|bead| {
            // A bead's `/T` refers back to its thread, which is what tells a thread's `/F` from a
            // file specification's `/F` — the other entry of that name a dictionary can carry.
            bead.get("T").is_some() || bead.get("N").is_some()
        })
}

/// Performs §12.6.3's trigger event on one annotation, where the file states actions for it.
///
/// Table 197's ten events belong to *any* annotation dictionary, and `action::for_annotation`
/// applies the one precedence rule the table states — "[f]or backward compatibility, the `A`
/// entry in an annotation dictionary, if present, takes precedence over" `/AA /U`. What is here
/// is the other half, the one this crate could not supply until session 132 gave it a pointer:
/// **something has to raise the event**.
///
/// No position is handed on, and the clause is why: §12.6.4.8's `/IsMap` "applies only to
/// actions triggered by the user's clicking an annotation", and a cursor *entering* a region is
/// not a click. A mouse-up over a link is not routed through here at all — see the caller.
pub(crate) fn trigger(open: &mut Open, annotation: ObjectId, event: Trigger) -> Outcome {
    let object = open.document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return Outcome::default();
    };
    let actions = pdf_model::action::for_annotation(&open.document, dict, event);
    if actions.is_empty() {
        return Outcome::default();
    }
    perform(open, &actions, None, None, "this annotation")
}

/// Table 198's two page-scoped events, which nothing raised until the two-hundred-and-fourth
/// session.
///
/// `/O` "when the page is opened" and `/C` "when the page is closed" — the same page turn
/// §12.6.3's `/PO` and `/PC` are about, one level up. Read from the page's *own* dictionary:
/// `/AA` is not one of §7.7.3.4's inheritable entries, which `action::for_page` states.
pub(crate) fn page_trigger(
    open: &mut Open,
    page: &Dictionary,
    event: pdf_model::action::PageTrigger,
) -> Outcome {
    let actions = pdf_model::action::for_page(&open.document, page, event);
    if actions.is_empty() {
        return Outcome::default();
    }
    perform(open, &actions, None, None, "this page")
}

/// Performs §12.6.2's action sequence and resolves whatever page it names.
///
/// One function for both callers, because §12.6.2 makes an action list an action list wherever
/// it came from. `at` is the click that started it, where there was one: it carries the cursor
/// and the annotation's `/Rect`, which is what §12.6.4.8's `/IsMap` needs and the only thing in
/// the whole sequence that does.
///
/// Where a chain of §12.6.2 actions says to go, and how to look at it there.
///
/// **The first request that names a page wins**, because a chain that jumps twice has shown the
/// second page either way and §12.6.2 states no rule for the pair — and Table 149's view is taken
/// from the *same* destination as the page, always. A jump that took its page from one
/// destination and its magnification from another would show a place no link in the file names.
#[derive(Debug, Default)]
struct Jump {
    /// The page index, zero-based.
    page: Option<usize>,
    /// §12.3.2.1's other two items, where the destination that won stated them.
    view: Option<pdf_model::destination::View>,
}

impl Jump {
    /// Takes this destination's page and view, if nothing has named a page yet.
    fn take(
        &mut self,
        document: &Document,
        pages: &Pages,
        destination: &pdf_model::destination::Destination,
    ) {
        if self.page.is_none()
            && let Some(page) = destination.page_index(document, pages)
        {
            self.page = Some(page);
            self.view = Some(destination.view);
        }
    }
}

/// `subject` is how a refusal names itself to a person — "this link declines", "this item
/// declines" — and nothing else depends on it.
fn perform(
    open: &mut Open,
    actions: &[Action],
    destination: Option<pdf_model::destination::Destination>,
    at: Option<((f32, f32), [f32; 4])>,
    subject: &str,
) -> Outcome {
    let mut outcome = Outcome::default();

    // §12.6.4's actions first, because two of the eleven this program performs change what the
    // *current* page draws — a layer's state (§12.6.4.13) and an annotation's Hidden flag
    // (§12.6.4.11) — and a link may do both and then jump.
    let before = open.view.clone();
    // Trap 5, on the one path where an action can be declined: every type Table 201 lists and
    // this program does not perform arrives as `Action::Refused` carrying its own reason, and
    // dropping it silently would make a click that does nothing indistinguishable from a click
    // on nothing.
    for action in actions {
        if let Action::Refused(why) = action {
            outcome.notes.push(format!("{subject} declines — {why}"));
        }
    }
    let requests = open.view.perform_all(&open.document, actions);

    // The first request that names a page wins, because a chain that jumps twice has shown the
    // second page either way and §12.6.2 states no rule for the pair.
    let pages = Pages::new(&open.document);
    let mut jump = Jump::default();
    if let Some(destination) = destination.as_ref() {
        jump.take(&open.document, &pages, destination);
    }
    let (mut import, mut embedded) = (None, None);
    for request in &requests {
        match request {
            Request::Display(destination) => jump.take(&open.document, &pages, destination),
            Request::Page(named) => {
                jump.page = jump
                    .page
                    .or_else(|| named.page_from(open.page_index, open.page_count));
            }
            // §12.6.4.5: "changes the view to the Start page of a specified DPart" — a page of
            // *this* document, so it resolves here beside the other two that name one.
            Request::DocumentPart(part) => {
                jump.page = jump.page.or_else(|| part.page_in(&open.document, &pages));
            }
            Request::Resolve(uri) => outcome.uris.push(match at {
                Some((point, rect)) => uri.at_position(point, rect),
                None => uri.uri.clone(),
            }),
            // Deferred rather than performed here: both need `&mut open` and `pages` still
            // borrows the document. Nothing is lost by the wait — §12.6.2 makes a chain a
            // sequence, and neither changes a page number.
            Request::Import(request) => import = Some(request.clone()),
            Request::Embedded(request) => embedded = Some(request.clone()),
            Request::Transition(transition) => outcome.transitions.push(transition.clone()),
            Request::Thread(thread) => {
                // §12.4.3's threads are read *here* rather than when the document opens: an
                // article is a list nothing else consults, and principle 2's "nothing eager"
                // applies to the two documents in a thousand that would pay for it at launch.
                let articles = pdf_model::article::Articles::read(&open.document);
                let bead = thread.bead_in(&articles);
                // **Table 163's `/R` is where the window goes**, and until the
                // two-hundred-and-fifty-fifth session this jumped to the bead's *page* and
                // stopped there. The entry is "[a] rectangle specifying the location of this
                // bead on the page in default user space", which is the same kind of statement
                // Table 149's `/FitR` makes — "[d]isplay the page … with its contents magnified
                // just enough to fit the rectangle specified by the coordinates left, bottom,
                // right, and top entirely within the window" — so a thread followed to a bead
                // shows the bead, not the page it happens to sit on. §12.4.3's own sentence is
                // what makes this the point of an article at all: the beads "are connected in
                // sequence" so that a reader "can follow a thread from one bead to the next".
                //
                // The ledger's reason for leaving it was "`viewer-ui` fits whole pages", which
                // stopped being true in the hundred-and-thirty-second session and stopped being
                // true of *destinations* in the two-hundred-and-first (ADR 0162), where
                // `Open::apply_view` learned all eight of Table 149's forms. This composes one
                // of them rather than adding a ninth.
                if let Some(bead) = bead
                    && let Some(rect) = bead.rect
                    && jump.view.is_none()
                {
                    jump.view = Some(pdf_model::destination::View::FitR { rect });
                }
                jump.page = jump
                    .page
                    .or_else(|| bead.and_then(|bead| bead.page_index(&pages)));
            }
        }
    }
    drop(pages);

    outcome.redraw = open.view != before;
    outcome.target = jump.page.filter(|target| *target < open.page_count);
    outcome.view = outcome.target.and(jump.view);
    if let Some(import) = import {
        request_file(open, &import, &mut outcome);
    }
    // §12.6.4.4 last, because it replaces the document every earlier request was about.
    if let Some(embedded) = embedded {
        open_embedded(open, &embedded, &mut outcome);
    }
    outcome
}

/// §12.7.6.4's import-data action, as far as a crate with no filesystem can take it.
///
/// The clause says a processor "shall import data … from a specified file", and specifies
/// nothing about *which* files a document may name — because that is a property of the processor
/// rather than of the document. So the name is handed to the host with the reason it is wanted,
/// and the host's policy decides. What is decided here is the one thing that is about the
/// *format*: ISO 19444-1's XFDF is the same data in XML and would need an XML parser, which is a
/// dependency and a decision rather than a clause.
fn request_file(open: &mut Open, import: &ImportData, outcome: &mut Outcome) {
    if import.format != pdf_model::action::DataFormat::Fdf {
        outcome.notes.push(format!(
            "this link declines — {} is not §12.7.8's FDF, and no other data format is read",
            import.file
        ));
        return;
    }
    open.importing = Some(import.clone());
    outcome.needs_file = Some((Purpose::ImportData, import.file.clone()));
}

/// Applies §12.7.8's form data from bytes the host supplied.
pub(crate) fn import(open: &mut Open, bytes: &[u8]) -> Outcome {
    use pdf_model::forms_data::FormsData;

    let mut outcome = Outcome::default();
    let Some(import) = open.importing.take() else {
        return outcome;
    };
    let opened = match Document::open(bytes.to_vec()) {
        Ok(opened) => opened,
        Err(error) => {
            outcome
                .notes
                .push(format!("import-data: cannot read {}: {error}", import.file));
            return outcome;
        }
    };
    let data = match FormsData::read(&opened) {
        Ok(data) => data,
        Err(error) => {
            outcome
                .notes
                .push(format!("import-data: {}: {error}", import.file));
            return outcome;
        }
    };

    // §14.4's file identifier, where both files state one: an FDF exported from a different
    // document is imported anyway — the clause states no rule against it and a form's fields may
    // legitimately be shared — but a person deserves to be told.
    if data.belongs_to(&open.document) == Some(false) {
        outcome
            .notes
            .push("import-data: this FDF file's /ID names a different document".to_owned());
    }
    // Table 246's `/Status` is "a status string that shall be displayed".
    if let Some(status) = &data.status {
        outcome
            .notes
            .push(format!("import-data: status — {status}"));
    }
    for owed in &data.owed {
        outcome
            .notes
            .push(format!("import-data: not applied — {owed}"));
    }

    let applied = open.view.import(&open.document, &data);
    outcome.notes.push(format!(
        "import-data: {} field(s) from {}, into {} widget(s)",
        data.fields.len(),
        import.file,
        applied.widgets
    ));
    for name in &applied.unmatched {
        outcome.notes.push(format!(
            "import-data: this document has no field named {name}"
        ));
    }
    for refusal in &applied.refused {
        outcome
            .notes
            .push(format!("import-data: declined — {refusal}"));
    }
    if applied.pages > 0 {
        // §12.7.7's template pages become part of the document being shown, so the page count
        // moves — which is the one thing an action in this program has ever changed about how
        // many pages there are.
        open.recount();
        outcome.notes.push(format!(
            "import-data: {} template page(s) added; the document now has {}",
            applied.pages, open.page_count
        ));
    }
    // §12.7.8's values are what §12.7.4.3 lays out, so a successful import changes this page's
    // ink.
    outcome.redraw = true;
    outcome
}

/// §12.6.4.4's embedded go-to, the one action that changes the document.
///
/// The target is *inside the file already open* — §7.11.4's embedded file streams — so this needs
/// no filesystem and no permission of any kind, which is what separates it from §12.6.4.3's
/// remote go-to standing beside it and refused.
///
/// Table 204's `/NewWindow` is a *should* and this is one view, so the target replaces the source
/// and says so.
fn open_embedded(open: &Open, target: &EmbeddedGoTo, outcome: &mut Outcome) {
    let opened = match target.target_in(&open.document) {
        Ok(opened) => opened,
        Err(error) => {
            outcome
                .notes
                .push(format!("this link declines — GoToE: {error}"));
            return;
        }
    };
    let mut replacement = Open::around(opened);
    if replacement.page_count == 0 {
        outcome
            .notes
            .push("this link declines — GoToE: the embedded document has no pages".to_owned());
        return;
    }
    // The destination is read in the *target*, because a named one is looked up in the target's
    // own tables and §12.3.2.2 makes an explicit one's first element a page number there rather
    // than a reference here.
    let pages = Pages::new(&replacement.document);
    let page_index =
        pdf_model::destination::Destination::read(&replacement.document, &target.destination)
            .and_then(|destination| destination.page_index_in_target(&replacement.document, &pages))
            .filter(|index| *index < replacement.page_count)
            .unwrap_or(0);
    drop(pages);
    replacement.page_index = page_index;
    if target.new_window == Some(true) {
        outcome.notes.push(
            "this link asks for a new window; this view has one, so the embedded document \
             replaces what was open"
                .to_owned(),
        );
    }
    outcome.notes.push(format!(
        "opened an embedded document, {} page(s), at page {}",
        replacement.page_count,
        page_index.saturating_add(1)
    ));
    outcome.replacement = Some(Box::new(replacement));
}
