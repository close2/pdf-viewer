//! A pointer over, on and off the page: §12.5.5's appearances, §12.6.3's triggers and what a
//! release activates.

use std::sync::Arc;

use pdf_model::action::Trigger;
use pdf_model::view::Pointer;
use pdf_syntax::ObjectId;

use super::editing::commit_typed;
use super::editing::field_of_widget;
use super::layout::damage;
use super::media::hand_over;
use super::{DocumentId, Viewer};
#[cfg(doc)]
use crate::Query;
use crate::command::PointerAction;
use crate::event::{Event, Extraction};
use crate::interact;
use crate::open::{Chosen, Open};

impl Viewer {
    /// §12.5.5's three appearances, and what a release activates.
    ///
    /// The pointer state is only changed for an annotation that *states* the appearance in
    /// question, because changing it invalidates the page's display list: a cursor crossing a
    /// link whose only appearance stream is `/N` would otherwise re-interpret the page — 2 000 M
    /// instructions — for a picture that cannot differ.
    ///
    /// **Which annotation the pointer is on is `annotation_at`'s answer and not `link_at`'s**
    /// (ADR 0177). §12.5.5 is written about an annotation — "\[a]n annotation may define as many
    /// as three separate appearances" — and §12.5.6.19's `/H` is an entry of a *widget*; taking
    /// the region from the link one would mean that neither reached anything but a link, where
    /// `pdf_model` implements both for every annotation (ADR 0123). `over` is also the region
    /// §12.6.3's events already use, so this is one question asked once rather than two answers
    /// that could disagree.
    ///
    /// It is `annotation_at` for a second reason, which is a clause rather than a tidy-up: that
    /// function filters by `annotation::interacts`, and §12.5.3's `ReadOnly` says an annotation
    /// "should not respond to mouse clicks or change its appearance in response to mouse
    /// motions". Reading the region through it is what makes that sentence true here.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per pointer action, and each arm is a different clause: §12.6.3's four \
                  triggers, §12.5.1's activation, §12.5.5's appearance and the selection. \
                  Splitting it would separate the state they all read from the state they set"
    )]
    pub(super) fn pointer(
        &mut self,
        at: (f32, f32),
        action: PointerAction,
        events: &mut Vec<Event>,
    ) {
        // Whether this message is a move with the button down, read before this message changes
        // the button's state.
        let held = action == PointerAction::Moved && self.button_down;
        match action {
            PointerAction::Pressed => self.button_down = true,
            PointerAction::Released => self.button_down = false,
            PointerAction::Moved | PointerAction::Dragged => {}
        }
        let Some(id) = self.focused else { return };
        let viewport = self.viewport;
        // **Which page of Table 29's arrangement the pointer is over**, and where on it. Under
        // `SinglePage` that is always the page showing and every answer below is what it was;
        // under a column or a spread it is whichever page the point landed on, so that a link on
        // the page beside or below the current one is still a link.
        let point = self.focused().and_then(|open| self.user_space(open, at));
        let on_page = self.focused().and_then(|open| self.page_point(open, at));
        // **Where in the text this pointer message lands, decided against the display list the
        // person is looking at rather than against whatever survives this function.** §12.5.5's
        // appearance state is changed below, and changing it calls `Open::stale`, which throws
        // the interpretation away — so a press on text that happens to lie over an annotation
        // with a down appearance used to ask an interpretation that had just been dropped and
        // get no anchor at all, and the whole drag from it selected nothing. Found by
        // `viewer-core/tests/selection_census.rs` on its first run, on 44 corpus documents; ADR
        // 0424. Asked once here for both arms below, which is also what a drag over a widget
        // costs least.
        let position = on_page.and_then(|(page, point)| self.focused()?.position_at(page, point));
        let Some(open) = self.focused_mut() else {
            return;
        };
        // What a *click* activates is the link one, and only that: §12.5.6.5's activation region
        // is a link's, and `open.pressed` below decides whether a release follows one.
        let under = point.and_then(|(page, (x, y))| interact::link_at(open, page, x, y));
        // §12.6.3's events belong to *any* annotation, and so does §12.5.5's appearance (ADR
        // 0177). Asked once per pointer message, which is
        // what a `/Rect` test over a page's annotation array costs: the same shape
        // `Query::FieldAt` already pays at pointer speed.
        // **`point` is already in default user space**, which is what §12.5.2 states an
        // annotation's `/Rect` in, so it is handed straight to `annotation_at`. This arm used to
        // put it through `user_space_at` a *second* time — `Self::user_space` had already applied
        // it — so on every page whose crop box does not start at the origin, and on every page
        // §7.7.3.3 turns, §12.5.5's appearance, §12.6.3's four pointer triggers, the focus a press
        // gives a widget and §12.5.1's popup all hit-tested somewhere the pointer was not. No gate
        // clicks and `Query::LinkAt` next door was right, which is why it survived; trap 12a is
        // the shape of it and this is the third instance.
        let over = point.and_then(|(page, (x, y))| {
            let object = open.placed_page(page)?;
            pdf_model::view::annotation_at(&open.document, object, &open.view, x, y)
        });

        // **A move between a press and its release is a move with the button held**, whatever the
        // host called it: GTK's motion controller reports the pointer during a press as well as
        // between presses, so a `Moved` arrives right after every `Pressed`. §12.5.5 is stated
        // about the button, not about the message — "[t]he down appearance shall be used when the
        // mouse button is pressed or held down within the annotation's active area" — so while
        // the annotation the press went down on is still under the pointer, its down appearance
        // stays. ADR 1370.
        //
        // **And over anything else it shows nothing**, including after a press on no annotation
        // at all: the rollover "shall be used when the user moves the cursor into the
        // annotation's active area without pressing the mouse button", and the button is pressed.
        //
        // **A drag is the same move under another name**: winit reports the pointer with the
        // button down as `Dragged`, and the clause's "within the annotation's active area" holds
        // for it exactly as for a held `Moved`, so a drag off the pressed annotation ends its down
        // appearance and a drag onto another shows nothing of it.
        let wanted = match action {
            PointerAction::Moved if held => over
                .filter(|annotation| Some(*annotation) == open.pressed_on)
                .map(|annotation| (annotation, Pointer::Down)),
            PointerAction::Dragged => over
                .filter(|annotation| Some(*annotation) == open.pressed_on)
                .map(|annotation| (annotation, Pointer::Down)),
            PointerAction::Moved => over.map(|annotation| (annotation, Pointer::Over)),
            PointerAction::Pressed => over.map(|annotation| (annotation, Pointer::Down)),
            // Back to hovering: the button is up and the cursor is still where it was.
            PointerAction::Released => over.map(|annotation| (annotation, Pointer::Over)),
        };
        let wanted = wanted
            .filter(|(annotation, pointer)| interact::has_appearance(open, *annotation, *pointer));
        if open.pointer != wanted {
            open.pointer = wanted;
            open.view.set_pointer(wanted);
            open.stale();
        }

        // Table 197's `/E` and `/X`, in the clause's own order: the cursor leaves one region
        // before it enters the next, and a document may act on both. §12.6.3's fourth constraint
        // needs nothing here because `over` is the topmost annotation rather than a set — "[i]n
        // the case of overlapping or nested annotations, entering a second annotation's active
        // area causes an X event to occur for the first annotation", and a cursor crossing into
        // an annotation nested inside the one it is in changes `over`, so both are raised.
        //
        // **The button decides whether an entry may happen at all.** §12.6.3 states four
        // constraints on the mouse events beside the table that defines them, and the first two
        // are about this pair: "[a]n E (enter) event may occur only when the mouse button is up"
        // and "[a]n X (exit) event may not occur without a preceding E event". A press puts the
        // button down and a drag keeps it there, so neither may enter — and the region the cursor
        // reached that way is therefore *not entered*, which is what keeps the second constraint
        // true when the cursor later leaves it. The entry is postponed rather than dropped: this
        // field stays empty until a message finds the button up over the same annotation, which
        // is the earliest moment the first constraint allows one.
        let button_down = matches!(action, PointerAction::Pressed | PointerAction::Dragged) || held;
        let mut raised: Vec<(ObjectId, Trigger)> = Vec::new();
        if open.inside != over {
            raised.extend(open.inside.map(|left| (left, Trigger::Exit)));
            if button_down {
                open.inside = None;
            } else {
                raised.extend(over.map(|entered| (entered, Trigger::Enter)));
                open.inside = over;
            }
        }

        match action {
            PointerAction::Moved => {}
            PointerAction::Pressed => {
                // `/D`: "a mouse button is pressed inside the annotation's active area".
                raised.extend(over.map(|annotation| (annotation, Trigger::Down)));
                if let Some((page, at)) = point {
                    interact::say_unreachable(open, page, at);
                }
                open.pressed = under;
                open.pressed_on = over;
                // Table 197's `/Bl` and `/Fo`, in the clause's own order, which is `/X` and
                // `/E`'s: one thing loses the focus before the next receives it.
                //
                // **How focus is acquired is not in the standard**, which says only what
                // happens when an annotation "receives the input focus" — so a press inside a
                // widget's active area giving it the focus is a choice, and it is the one every
                // pointing interface makes. Widgets only, because Table 197 says so of both
                // entries; a press on a link or a stamp therefore *blurs* whatever held it.
                let wants_focus = over.filter(|annotation| is_widget(&open.document, *annotation));
                if open.focus != wants_focus {
                    raised.extend(open.focus.map(|left| (left, Trigger::Blur)));
                    raised.extend(wants_focus.map(|got| (got, Trigger::Focus)));
                    open.focus = wants_focus;
                }
                // A press starts an empty selection where it landed, so that the first drag
                // has an anchor. An empty selection highlights nothing and is not a selection
                // a person can see.
                open.selection = position
                    .zip(on_page)
                    .map(|(position, (page, _))| Chosen::in_page(page, (position, position)));
                events.push(damage(viewport));
            }
            PointerAction::Dragged => {
                // A drag moves the selection's far end to wherever the pointer is, **on whatever
                // page of Table 29's arrangement that is**. It used to refuse to leave the page
                // the press landed on, because a range was into one page's readback and an offset
                // meant nothing anywhere else; both ends name their own page now, so a sweep down
                // a column selects the first page's tail, the pages between whole, and the last
                // page's head. §12.4.2 still gives no document-wide offset and this still does not
                // invent one — it carries the page beside the offset instead.
                //
                // A pointer over no page at all — the gap between two rows, the margin beside
                // them — leaves the selection where it was rather than dropping its far end
                // somewhere arbitrary, which is what `on_page` answering `None` already meant.
                //
                // **A drag still inside the annotation whose down appearance the press put up is
                // that press, not a selection.** The press anchored the selection in the readback
                // of the page as it was drawn *before* §12.5.5's down appearance replaced the
                // normal one, and the down appearance may carry text of its own — so an offset
                // taken now is into a different readback, and the two ends of the "selection"
                // would name characters of two different pages. GTK reports a drag at the press's
                // own point with every press, and that range was enough to turn a click on a push
                // button into a selection and withhold its `/A`. ADR 1370.
                //
                // A push button only: §12.7.5.2.2 makes it a control that "responds immediately to
                // user input", so a press on it is its activation, while a text field or an
                // annotation lying over the page's text is somewhere a person may start a
                // selection (ADR 0424).
                let pressing = open.pressed_on.is_some_and(|annotation| {
                    over == Some(annotation)
                        && open.pointer == Some((annotation, Pointer::Down))
                        && is_push_button(&open.document, annotation)
                });
                if !pressing
                    && let (Some(chosen), Some(position), Some((page, _))) =
                        (open.selection, position, on_page)
                {
                    open.selection = Some(Chosen {
                        from: chosen.from,
                        to: crate::open::Spot {
                            page,
                            offset: position,
                        },
                    });
                    events.push(damage(viewport));
                }
            }
            PointerAction::Released => {
                let pressed = open.pressed.take();
                let pressed_on = open.pressed_on.take();
                // A press dragged off the annotation before release is a press the person
                // changed their mind about; see `PointerAction::Released`. And a press that
                // selected something was a drag rather than a click, so it does not follow the
                // link it happened to start on — which is what every viewer does and what a
                // person dragging across a paragraph of links expects.
                let selecting = open.selection.is_some_and(|chosen| !chosen.empty());
                let clicked = !selecting;
                // Table 197's `/U` — "an action that shall be performed when the mouse button is
                // released inside the annotation's active area" — for anything that is not the
                // link about to be activated. **The exclusion is the precedence rule, not a
                // shortcut**: the table says an annotation's `/A` "takes precedence over" its
                // `/AA /U`, `interact::activate` performs a link's `/A`, and
                // `action::for_annotation` would return the same list again — so routing a link
                // through both would perform its actions twice.
                //
                // **Raised before the link question is asked** (ADR 0191), and that is a clause
                // rather than a tidy-up: returning early whenever the release did not activate a
                // link would mean a click on a stamp, a widget or a markup annotation raised
                // nothing at all.
                //
                // **Table 197's sentence is not the whole condition**, and §12.6.3's third
                // constraint is the rest of it: "[a] U (up) event may not occur without preceding
                // E and D events". So a release inside an annotation's area raises `/U` only
                // where that same annotation has both — `inside` is the one an `/E` was raised
                // for and not withdrawn by an `/X`, and `pressed_on` is the one the `/D` went
                // down on. A press on the page that drags into an annotation and lets go there
                // has neither, and used to raise `/U` all the same.
                let entered = open.inside;
                if clicked
                    && let Some(annotation) = over
                        .filter(|over| Some(*over) != under)
                        .filter(|over| entered == Some(*over) && pressed_on == Some(*over))
                {
                    raised.push((annotation, Trigger::Up));
                }
                // §12.5.1's other half: "[w]hen the user activates the annotation by clicking it,
                // it exhibits its associated object, such as by opening a popup window displaying
                // a text note". A press and a release on one annotation, and the window §12.5.6.14
                // gives it opens — or closes, which the clause does not state and every reader of
                // a sticky note expects (`Open::toggle_popup`).
                let toggled = clicked
                    && pressed_on
                        .filter(|annotation| Some(*annotation) == over)
                        .is_some_and(|annotation| exhibit(id, open, annotation, events));
                if toggled {
                    events.push(damage(viewport));
                }
                if selecting || pressed.is_none() || pressed != under {
                    self.raise(id, raised, point.map(|(_, at)| at), events);
                    return;
                }
                let Some((page, (x, y))) = point else {
                    self.raise(id, raised, point.map(|(_, at)| at), events);
                    return;
                };
                self.raise(id, raised, point.map(|(_, at)| at), events);
                let Some(open) = self.focused_mut() else {
                    return;
                };
                let outcome = interact::activate(open, page, x, y);
                self.apply(id, outcome, events);
                return;
            }
        }
        self.raise(id, raised, point.map(|(_, at)| at), events);
    }

    /// Performs §12.6.3's events the pointer just raised, in the order they were raised.
    ///
    /// `at` is where the pointer was, in the page's default user space, for the events a
    /// pointer raised — and `None` for the ones it did not, which are Table 198's page open and
    /// close and a focus change. Table 240 bit 5 is the one thing that reads it: "the
    /// coordinates of the mouse click that caused the submit-form action", and a submit button
    /// is a widget, so this is the path those coordinates arrive by.
    ///
    /// **Each event's scripts run here too** (ADR 1752): a pointer's and a focus's through
    /// `run_annotation_scripts`, under the runner the reader's `Scripts` level supplied, before the
    /// same chain's other actions — ADR 1602 keeps no order between the two. Table 197's four
    /// page events are not run here: their scripts are [`Self::page_events`]'s, page and
    /// annotations together, and the open sequence's at the first present (RFC 0008 section 6.5).
    pub(super) fn raise(
        &mut self,
        id: DocumentId,
        raised: Vec<(ObjectId, Trigger)>,
        at: Option<(f32, f32)>,
        events: &mut Vec<Event>,
    ) {
        let runner = !matches!(self.scripting, crate::Scripting::Off);
        for (annotation, event) in raised {
            let Some(open) = self.focused_mut() else {
                return;
            };
            // Table 197's `/Bl` is "when the annotation loses the input focus", and a value typed
            // into the widget's field is committed then, before the widget's own action — the
            // order Adobe's "Form event processing" gives, the commit's validate and calculate
            // ahead of the blur (ADR 1592).
            if event == Trigger::Blur
                && let Some(field) = field_of_widget(&open.document, annotation)
            {
                commit_typed(id, open, &field, events);
            }
            let scripted = match event {
                Trigger::Enter
                | Trigger::Exit
                | Trigger::Down
                | Trigger::Up
                | Trigger::Focus
                | Trigger::Blur => {
                    let ran = open
                        .view
                        .run_annotation_scripts(&open.document, annotation, event);
                    // Interpreted again only where the scripts can have changed what the page
                    // draws: a `/E` that logs leaves the page as it was (ADR 1771).
                    if ran.changed {
                        open.stale();
                    }
                    ran.answered()
                }
                // `page_events` hands these four to the runner with the page's own, whenever
                // there is one, and the open sequence hands them over for the page an open shows.
                Trigger::PageOpen
                | Trigger::PageClose
                | Trigger::PageVisible
                | Trigger::PageInvisible => runner,
            };
            let outcome = interact::trigger(open, annotation, event, at, scripted);
            self.apply(id, outcome, events);
        }
    }

    /// §12.3.3: activates an object a host is showing outside the page.
    pub(super) fn activate(&mut self, object: ObjectId, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        let outcome = interact::activate_object(open, object);
        self.apply(id, outcome, events);
    }
}

/// Whether an annotation is a push button: a widget whose field states Table 229 bit 17 with a
/// `/FT` of `Btn`, both inheritable through `/Parent` (Table 226). ADR 1370.
fn is_push_button(document: &pdf_syntax::Document, annotation: ObjectId) -> bool {
    /// Table 229 bit 17, `Pushbutton`.
    const PUSH_BUTTON: i64 = 1 << 16;
    /// How far up `/Parent` the inheritable entries are looked for, as `pdf_model::form` bounds it.
    const DEPTH: usize = 32;
    let mut field_type: Option<bool> = None;
    let mut flags: Option<i64> = None;
    let mut node = document.get(annotation);
    for _ in 0..DEPTH {
        let Some(dict) = node.as_dict() else {
            break;
        };
        if field_type.is_none() {
            field_type = document
                .get_key(dict, "FT")
                .as_name()
                .map(|name| name.as_bytes() == b"Btn");
        }
        if flags.is_none() {
            flags = document.get_key(dict, "Ff").as_integer();
        }
        if field_type.is_some() && flags.is_some() {
            break;
        }
        let parent = document.get_key(dict, "Parent");
        let Some(next) = parent.as_reference() else {
            break;
        };
        node = document.get(next);
    }
    field_type == Some(true) && flags.unwrap_or(0) & PUSH_BUTTON != 0
}

/// Whether an annotation is a widget, which is what §12.6.3's Table 197 makes `/Fo` and `/Bl`
/// about — both entries are
///
/// > (Optional; PDF 1.2; widget annotations only)
///
/// and nothing else in the table carries that restriction.
pub(super) fn is_widget(document: &pdf_syntax::Document, annotation: ObjectId) -> bool {
    let object = document.get(annotation);
    object.as_dict().is_some_and(|dict| {
        document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Widget")
    })
}

/// §12.5.1: what an annotation a person just clicked "exhibits", and whether the page changed.
///
/// > When the user activates the annotation by clicking it, it exhibits its associated object,
/// > such as by opening a popup window displaying a text note ("Figure 77 -Open annotation") or
/// > by playing a sound or a movie.
///
/// Two associated objects, and the clause's "such as" is why they are one function rather than
/// two conditions in the pointer arm. §12.5.6.14's window opens or closes (`Open::toggle_popup`,
/// ADR 0191). §12.5.6.15's *file* comes out: "\[a] file attachment annotation contains a
/// reference to a file, which typically shall be embedded in the PDF file", and the clause says
/// what activating one does — "activating the annotation extracts the embedded file and gives
/// the user an opportunity to view it or store it in the file system". The bytes cross as
/// [`Event::Extracted`], which is where they cross for [`crate::Command::Extract`] too, and
/// where they land is the host's policy (rule 2).
///
/// **The file is reached here rather than from a document-wide list, and that is a
/// measurement**: a panel listing every page's attachments has to walk every page, which costs
/// 78 to 123 ms cold over three runs on ISO 32000-2's 1023 pages and 13 to 15 ms warm
/// (`pdf-model --example file_attachment_census`) — and every host asks [`Query::Attachments`]
/// when a document opens, which is exactly where `CLAUDE.md` forbids a full page-tree walk. A
/// click has already found its page. ADR 0295.
fn exhibit(id: DocumentId, open: &mut Open, annotation: ObjectId, events: &mut Vec<Event>) -> bool {
    let toggled = open.toggle_popup(annotation);
    for file in attached_files(open, annotation) {
        hand_over(id, Extraction::Asked, &open.document, &file, None, events);
    }
    toggled
}

/// The files the annotation under the click carries: §12.5.6.15's, and §14.13.9's.
///
/// Empty for nearly every click, and the question is asked of one annotation the pointer already
/// found, so it costs two dictionary lookups rather than a walk.
///
/// **§12.5.6.15's `/FS` first**, because its clause is the one that states the extraction outright
/// — "activating the annotation extracts the embedded file and gives the user an opportunity to
/// view it or store it in the file system" — and it belongs to that subtype alone.
///
/// **Then §14.13.9's `/AF`, for every subtype**, which is that clause read through the one above
/// it: "\[t]o associate files with annotations, the annotation dictionary shall contain an AF entry
/// which represents the associated files for that annotation", and §12.5.1 says what activating an
/// annotation does with what belongs to it — "it exhibits its associated object, such as by
/// opening a popup window displaying a text note … or by playing a sound or a movie". The *such
/// as* is examples rather than a closed list, and an associated file is an associated object in
/// the plainest sense the standard has.
///
/// This is the entry's caller (ADR 1065). Without one it is in the shape of a reading that
/// exists, a row that calls it implemented, and a payload no host can extract.
///
/// One payload named both ways is one file, deduplicated by the stream's identity exactly as
/// `attachment::attachments` does it — a producer that writes a `/FS` and repeats it in `/AF` has
/// attached one file.
pub(super) fn attached_files(
    open: &Open,
    annotation: ObjectId,
) -> Vec<pdf_model::attachment::Attachment> {
    let object = open.document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if open
        .document
        .get_key(dict, "Subtype")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
        == Some(b"FileAttachment")
        && let Some(file) = pdf_model::attachment::of_annotation(&open.document, dict)
    {
        out.push(file);
    }
    for file in pdf_model::attachment::associated(&open.document, dict) {
        if out
            .iter()
            .any(|seen| Arc::ptr_eq(&seen.stream, &file.stream))
        {
            continue;
        }
        out.push(file);
    }
    out
}
