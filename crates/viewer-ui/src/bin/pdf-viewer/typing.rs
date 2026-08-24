//! The keyboard inside a value: §12.7.4.3's field, §12.5.6.6's annotation, and the clipboard.
//!
//! Two things on a page take characters and the standard sends both to the same subclause, so
//! they are one module: what differs is the question that reads the text back and the edit that
//! puts it there, and everything between those two — the caret, the selection, what a key does
//! to a value — is shared. The clipboard is here because the same three keys serve a field's
//! value and the page's own selection, and because what a copy *is* belongs to a host.

use pdf_model::form::Control;
use pdf_syntax::ObjectId;
use viewer_core::{Answer, Command, Edit, Entered, Query};
use viewer_host::form::{ControlKind, control_kind};
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

use crate::app::App;

/// The colour §12.5.6.6's text is written in when this host creates one, as `DeviceRGB`.
///
/// **A choice**, and the same one the highlight's yellow is: Table 177's `/DA` carries whatever a
/// processor was told and no clause states what to tell it. A dark red is what a note written on
/// somebody else's page is, and it is what this host offers a person with one key.
const FREE_TEXT_INK: [f32; 3] = [0.7, 0.1, 0.1];

/// The nearest character boundary at or before `offset`, clamped to the value's length.
///
/// A caret is a place *between* characters, and the value it indexes changes under it: a field
/// that truncated what was typed (§12.7.5.3) is shorter than what was sent, and a value read back
/// from the document may be shorter still. Every use of the offset goes through this, so a
/// multi-byte character can never be cut in half by an index.
fn caret_boundary(value: &str, offset: usize) -> usize {
    // The clamp comes first, and the test below is why: `is_char_boundary` answers *false* for an
    // offset past the end, so a search for the boundary before it would land on the last
    // character's rather than on the end of the value — one character short after every
    // truncation, which is the case this function exists for.
    let offset = offset.min(value.len());
    if value.is_char_boundary(offset) {
        return offset;
    }
    value
        .char_indices()
        .map(|(at, _)| at)
        .take_while(|at| *at < offset)
        .last()
        .unwrap_or(value.len())
}

/// The offset one character before `caret`, or the start of the value.
fn before(value: &str, caret: usize) -> usize {
    value
        .get(..caret)
        .and_then(|prefix| prefix.char_indices().next_back().map(|(at, _)| at))
        .unwrap_or(0)
}

/// The offset one character after `caret`, or the end of the value.
fn after(value: &str, caret: usize) -> usize {
    value
        .get(caret..)
        .and_then(|rest| rest.chars().next())
        .map_or(value.len(), |character| {
            caret.saturating_add(character.len_utf8())
        })
}

/// The value with `from..to` replaced by `insert`.
///
/// The whole of what a keystroke does to a field, and it is a whole *value* rather than an edit
/// because that is what `Edit::SetField` carries: the core is told what the field says now, and
/// §12.7.5.3 decides how much of it the widget accepts.
fn spliced(value: &str, from: usize, to: usize, insert: &str) -> String {
    let mut out = String::with_capacity(value.len().saturating_add(insert.len()));
    out.push_str(value.get(..from).unwrap_or_default());
    out.push_str(insert);
    out.push_str(value.get(to..).unwrap_or_default());
    out
}

/// Where the two ends of a selection go when an arrow key moves the caret.
///
/// Three cases, and only the first is anybody's convention but this host's: **shift** holds the
/// anchor and moves the caret by one character, so the selection grows or shrinks; a move with
/// something selected and no shift lands on the *edge* of it, which is what a person means by
/// pressing Left with a word swept; and a move with nothing selected steps one character and takes
/// the anchor with it. The standard states none of this — it describes no cursor at all — so it is
/// recorded as the choice it is (ADR 0225).
fn stepped(current: &str, ends: (usize, usize), shift: bool, forward: bool) -> (usize, usize) {
    let (caret, anchor) = ends;
    let (low, high) = (caret.min(anchor), caret.max(anchor));
    let step = if forward {
        after(current, caret)
    } else {
        before(current, caret)
    };
    if shift {
        (step, anchor)
    } else if low < high {
        let edge = if forward { high } else { low };
        (edge, edge)
    } else {
        (step, step)
    }
}

/// Whether a quadrilateral this crate was handed covers a point in the viewport.
///
/// The **bounding box** of the four corners, and that is exact rather than an approximation for
/// the shape this is asked about: `viewer_core` builds a widget's quadrilateral out of Table 166's
/// `/Rect`, which "shall be two opposite corners" of an upright rectangle, through §7.7.3.3's
/// `/Rotate` — and that entry's value "shall be a multiple of 90", so the rectangle is still
/// upright on the screen. A quadrilateral this test would get wrong is one no page can state.
fn covers(quad: [f32; 8], (x, y): (f32, f32)) -> bool {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let bound = |values: [f32; 4]| {
        values.iter().copied().fold(f32::INFINITY, f32::min)
            ..=values.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    };
    bound(xs).contains(&x) && bound(ys).contains(&y)
}

/// The §12.7.5 control a press landed on, as the three questions about it answer together.
///
/// A value rather than a tuple because the three are asked for different reasons and a caller that
/// wanted the second of four would have to count: which field to address an edit to, which of its
/// widgets was pressed, what §12.7.5's flags make the control, and whether Table 227 bit 1 lets a
/// person change it at all.
struct Pressed {
    /// §12.7.4.2's fully qualified name.
    field: String,
    /// The widget annotation under the point.
    annotation: ObjectId,
    /// What the flags make it.
    kind: ControlKind,
    /// Table 227 bit 1: "the field shall not be modified by the user".
    read_only: bool,
}

/// A choice field whose options are on the screen: §12.7.5.4's list, opened.
///
/// **The field is named and the geometry is not**, which is the same decision [`Typing`] makes one
/// paragraph down and for a sharper reason: the page scrolls, zooms and turns under an open list,
/// and a rectangle kept here would be where the widget *was*. The name and the annotation are what
/// the document says; where they are on the screen is `Query::Fields`' answer, asked again for
/// every frame the list is drawn in and for the press that picks a row — so the row drawn and the
/// row acted on are one layout by construction (`viewer_ui::chrome::ChoiceList`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Choosing {
    /// §12.7.4.2's fully qualified name, which is what an edit is addressed by.
    pub(crate) field: String,
    /// Which of the field's widgets was pressed, since §12.7.4.1's `/Kids` lets a terminal field
    /// refer to "one or more separate widget annotations".
    pub(crate) annotation: ObjectId,
}

/// A person typing into a form field: which field, and where in its value.
///
/// Three numbers and no text, which is ADR 0201's decision with ADR 0211's caret and ADR 0225's
/// selection added to it. The point names the field because a field does not move; the two offsets
/// say where the next character goes and how much of the value a person has swept, and they are
/// the one thing about typing that the core cannot know — nothing in a document says where a
/// person's cursor is.
///
/// **A caret is a collapsed selection here, and two questions in the core.** This host holds one
/// pair of offsets and calls them equal when nothing is selected; `viewer-core` answers
/// `Query::Caret` with a segment and `Query::FieldSelection` with boxes, because those are two
/// shapes and not one (ADR 0225).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Typing {
    /// Which of the two kinds of thing on a page takes characters.
    pub(crate) target: Target,
    /// The point on the page that named the field, in the page viewport's device pixels.
    pub(crate) at: (f32, f32),
    /// How far into the field's value the caret is, in bytes.
    pub(crate) caret: usize,
    /// The other end of the selection, in bytes — equal to [`Self::caret`] when nothing is
    /// selected, which is the ordinary state.
    pub(crate) anchor: usize,
}

/// How one keystroke's edit is addressed, resolved when the key is pressed.
///
/// [`Target`] says which *kind* of thing has the keyboard and is cheap enough to keep in
/// [`Typing`]; this carries the name, which is a `String` for one of the two and would cost
/// `Typing` its `Copy`.
enum Aim {
    /// §12.7.4.2's fully qualified name.
    Field(String),
    /// The annotation's object.
    FreeText(ObjectId),
}

/// A free text annotation being drawn: armed, or with its first corner down.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Drawing {
    /// `f` was pressed and the next press on the page starts the rectangle.
    Armed,
    /// The pointer went down here, in the page viewport's device pixels.
    From((f32, f32)),
}

/// What a person is typing into.
///
/// **Two, and the standard is why**: §12.7.4.3 lays out the value of a field, and §12.5.6.6 sends
/// its own annotation to that same subclause — "[s]ubclause 12.7.4.3, 'Variable text', describes
/// the process of using these entries to generate the appearance of the text in these
/// annotations". The caret, the selection and every key below are the *same* for both, because the
/// core answers `Query::Caret`, `Query::Offset` and `Query::FieldSelection` for both; what differs
/// is the one question that reads the text back and the one edit that puts it there.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Target {
    /// §12.7's field at [`Typing::at`], named by §12.7.4.2's qualified name when an edit is sent.
    Field,
    /// §12.5.6.6's annotation, named by its object because it has no other name.
    FreeText(ObjectId),
}

impl Typing {
    /// Nothing selected, at one offset.
    fn at_offset(target: Target, at: (f32, f32), offset: usize) -> Self {
        Self {
            target,
            at,
            caret: offset,
            anchor: offset,
        }
    }

    /// The selected range, low end first — empty where the two offsets are the same.
    pub(crate) fn range(self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }
}

impl App {
    /// Puts what is selected on the page into this program's clipboard.
    ///
    /// **The first consumer [`Query::LogicalSelection`] has ever had.** `doc/todo/01`'s fifth
    /// sweep — "the model implements this, who calls it?" — found in the four-hundred-and-
    /// thirteenth session that the query had answered since the three-hundred-and-eighty-eighth
    /// and that no host of the four asked it, while §14.8.2.5's ledger row read `implemented` on
    /// the strength of the query alone. A clause reaches a person through a program or it does
    /// not reach one.
    ///
    /// ISO 32000-2 §14.8.2.5 defines the two orders and this host chooses between them:
    ///
    /// > Page content order shall be defined by the sequencing of graphics objects within a
    /// > page's content stream.
    ///
    /// > Logical content order -the ordering for semantic purposes -shall be defined by a
    /// > depth-first traversal of the document's logical structure
    ///
    /// A selection is taken in page content order, because that is the order the shapes are in;
    /// a *copy* off a page whose producer wrote its columns out of order wants the other one.
    /// The core answers [`Answer::None`] where the document states no structure tree or where
    /// the tree does not reach every byte of the selection, and [`viewer_host::copied`] is what
    /// that means here: the order it could give, named out loud, rather than a silent
    /// rearrangement.
    ///
    /// **The choice between the two orders left this file in the six-hundred-and-eighty-third
    /// session** (ADR 0519). It is the same decision in all three windowed hosts and the third
    /// copy is where two of them stop agreeing, so it is `viewer_host::copying` now — and what
    /// stays here is the two questions and the platform, which is what a host actually owns.
    ///
    /// **And the text now leaves the program.** `viewer_ui::clipboard` is this host's end of the
    /// session's clipboard, connected on this line and no earlier; the in-process string is kept
    /// beside it because it is what Ctrl + V pastes into a field, and a paste that asked the
    /// platform would put another program's response time inside a keystroke. A platform that
    /// refuses is reported rather than swallowed, and the copy still happened *inside* the
    /// program, which is the honest thing to say about it.
    pub(crate) fn copy_selection(&mut self) {
        // Owned before the second question, because both answers borrow the viewer.
        let page_order = match self.viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.text.into_owned(),
            _ => String::new(),
        };
        let logical = match self.viewer.query(Query::LogicalSelection) {
            Answer::LogicalSelection(text) => Some(text),
            _ => None,
        };
        let Some(copied) = viewer_host::copied(logical, &page_order) else {
            println!("note: nothing on the page is selected to copy");
            return;
        };
        println!(
            "note: copied {} characters in {}",
            copied.text.chars().count(),
            copied.order
        );
        if let Err(error) = self.platform_clipboard.put(&copied.text) {
            println!("note: {error}, so the copy stayed inside this program");
        }
        self.clipboard = copied.text;
    }

    /// Starts or stops typing, at a point of the page's own viewport.
    ///
    /// **The point is the caller's since the five-hundred-and-ninetieth session**, because a
    /// pointer is no longer the only thing that clicks: an assistive technology asking
    /// `org.a11y.atspi.Action` for a click names a *node*, and [`App::click_page`] turns that into
    /// the same three steps a mouse takes. Reading `self.cursor` here would have aimed the
    /// keyboard wherever the mouse happened to be resting.
    ///
    /// A press inside a field somebody can type into aims the keyboard at it; a press anywhere
    /// else puts the keyboard back on the page. §12.7.5.1's four field types are not equal here —
    /// a button has no text and a signature field's value is a dictionary — and the *core* is what
    /// draws that line: `Answer::Field`'s value is `None` for a field whose value is not text and
    /// `Some("")` for an empty one, which is the same distinction §12.7.4.3 makes when it decides
    /// what to lay out.
    pub(crate) fn aim_at_field(&mut self, at: (f32, f32)) {
        let was = self.typing.is_some();
        // §12.5.6.6 first, because the core hit-tests it first: an annotation a person added is
        // drawn after the page's own `/Annots` and the thing on top is the thing under the
        // pointer. Asking in the other order would put the keyboard in a field underneath a note.
        if let Some(typing) = self.aim_at_free_text(at) {
            self.typing = Some(typing);
            return;
        }
        self.typing = match self.viewer.query(Query::FieldAt(at)) {
            // Table 231 bit 14, refused rather than mishandled — and it was mishandled until the
            // four-hundred-and-eleventh session, which is what ADR 0247's third amendment made
            // visible. This host reads a field's value back after every keystroke (ADR 0201) and a
            // password field answers with bullets, so what it sent as the next value was those
            // bullets with a character appended. Refusing is trap 5: `viewer-gtk` and `viewer-qt`
            // type into a `GtkPasswordEntry` and a `QLineEdit` in `Password` echo mode, which are
            // the platform's own secure controls, and this host draws its own page and has none.
            Answer::Field {
                name,
                value: Some(shown),
            } if shown.obscured => {
                println!(
                    "note: {} is a Table 231 bit 14 password field, which this host does not type \
                     into: its value answers as bullets and cannot be read back (ADR 0247)",
                    name.shown()
                );
                None
            }
            // **Table 233 bit 19's other half, which this host disobeyed for its whole life.**
            // `Answer::Field` answers a combo box with characters whether or not the flag is set —
            // correctly, because the value *is* text and §12.7.4.3 lays it out — and this host read
            // "has a text value" as "takes typed characters". So a person could type *Purple* into
            // a drop-down whose options are Red and Blue, and the file took it. The bit binds in
            // both directions and `ControlKind::takes_typed_characters` is where that sentence now
            // lives for all three hosts. ADR 0596.
            Answer::Field {
                name,
                value: Some(_),
            } if !self.takes_characters_at(at) => {
                println!(
                    "note: {} states Table 233 bit 19 clear, so it shall include only a drop-down \
                     list — pick one of its options rather than typing a value",
                    name.shown()
                );
                None
            }
            Answer::Field {
                name,
                value: Some(value),
            } => {
                println!("note: typing into the field {}", name.shown());
                // **The caret goes where the click went**, which is `Query::Offset` — the inverse
                // of `Query::Caret`, and the piece `doc/todo/33` said was missing until the
                // three-hundred-and-eighty-eighth session. The point names the field and is also
                // the point measured, because a press is one place; a drag then asks the same
                // question with the pointer's place instead. The end of the value is what a field
                // whose layout could not answer falls back to, which is where the caret used to
                // start every time.
                let caret = match self.viewer.query(Query::Offset { at, point: at }) {
                    Answer::Offset(offset) => offset,
                    _ => value.text.len(),
                };
                Some(Typing::at_offset(Target::Field, at, caret))
            }
            _ => None,
        };
        if was && self.typing.is_none() {
            println!("note: the keyboard is back on the page");
        }
    }

    /// The control a point is on: §12.7.4.2's name, the widget pressed, and what kind it is.
    ///
    /// **Two questions and not one**, because they answer about different things: `Query::FieldAt`
    /// says which *field* a point is in and whether its value is text, and `Query::Fields` says
    /// what §12.7.5's flags make the *control*. Table 233 bit 19 is a fact about the control, so
    /// nothing in the first answer could carry it. `App::toggle_button` has asked this pair since
    /// ADR 0235 for §12.7.5.2's own flag; this is that walk named once instead of twice.
    ///
    /// The *last* widget covering the point, because §12.5.2 draws them in `/Annots` order and the
    /// one on top is the one under the pointer.
    fn control_at(&self, at: (f32, f32)) -> Option<Pressed> {
        let Answer::Field { name, .. } = self.viewer.query(Query::FieldAt(at)) else {
            return None;
        };
        let qualified = name.qualified;
        let Answer::Fields(fields) = self.viewer.query(Query::Fields) else {
            return None;
        };
        let field = fields
            .iter()
            .find(|field| field.name.qualified == qualified)?;
        let widget = field
            .widgets
            .iter()
            .rev()
            .find(|widget| covers(widget.quad, at))?;
        Some(Pressed {
            field: qualified,
            annotation: widget.annotation,
            kind: control_kind(&field.control),
            read_only: field.read_only,
        })
    }

    /// Whether the control at a point takes characters from a keyboard at all.
    ///
    /// A point on no control at all answers **true**, which is deliberate: this is a *refusal*
    /// and it may only refuse something it has identified. §12.5.6.6's free text has already been
    /// answered before this is asked, and a field the form walk cannot find is one the existing
    /// value question has already decided about.
    fn takes_characters_at(&self, at: (f32, f32)) -> bool {
        self.control_at(at)
            .is_none_or(|pressed| pressed.kind.takes_typed_characters())
    }

    /// Opens §12.7.5.4's options where the press was on a choice field.
    ///
    /// **This is the control, not a decoration.** A tier-1 host places a `GtkDropDown` or a
    /// `QComboBox` and a person picks a row out of it; this host draws the page's own appearance,
    /// so until now the *only* value it could give a choice field was characters — which Table 233
    /// bit 19 admits for one combo box in two and no list box at all. Both of §12.7.5.4's forms
    /// list their options here, because the clause's two controls differ in how the options are
    /// shown and not in whether a person may choose one.
    ///
    /// Answers whether a list is now open, so that a press on a choice field does not also start a
    /// drag on the page underneath it.
    pub(crate) fn open_choices(&mut self, at: (f32, f32)) -> bool {
        let Some(pressed) = self.control_at(at) else {
            return false;
        };
        if !matches!(
            pressed.kind,
            ControlKind::Combo { .. } | ControlKind::List { .. }
        ) {
            return false;
        }
        // Table 227 bit 1: "the field shall not be modified by the user". Checked here as well as
        // in the core for `App::toggle_button`'s reason — the core refuses the edit, and a program
        // that opened a list nothing could be picked out of looks broken rather than obedient.
        if pressed.read_only {
            println!("note: the field {} is read-only (Table 227)", pressed.field);
            return false;
        }
        self.choosing = Some(Choosing {
            field: pressed.field,
            annotation: pressed.annotation,
        });
        self.redraw();
        true
    }

    /// §12.7.5.4's open list: where its rows are, and which control they belong to.
    ///
    /// Re-derived per frame and per press rather than kept, which is [`Choosing`]'s own reason:
    /// the page moves under an open list, and one derivation used for both the drawing and the
    /// hit test is what keeps the row shown and the row picked the same row.
    pub(crate) fn choices(&self) -> Option<(viewer_ui::chrome::ChoiceList, ControlKind)> {
        let choosing = self.choosing.as_ref()?;
        let chrome = self.chrome.as_ref()?;
        let window = self.window()?;
        let Answer::Fields(fields) = self.viewer.query(Query::Fields) else {
            return None;
        };
        let field = fields
            .iter()
            .find(|field| field.name.qualified == choosing.field)?;
        let widget = field
            .widgets
            .iter()
            .find(|widget| widget.annotation == choosing.annotation)?;
        let kind = control_kind(&field.control);
        // Where the list starts. Table 234's `/TI` is the clause's own answer for a scrollable
        // list box — "the index in the Opt array of the first option visible in the list" — and it
        // says nothing about a drop-down, so a combo box starts where its value is: a list that
        // opened at row 0 with the selection forty rows below would be showing the document's data
        // and hiding its answer.
        let (options, selected, first) = match &kind {
            ControlKind::Combo {
                options, selected, ..
            } => (
                options.clone(),
                selected.map(|index| vec![index]).unwrap_or_default(),
                selected.unwrap_or_default(),
            ),
            ControlKind::List {
                options,
                selected,
                top,
                ..
            } => (options.clone(), selected.clone(), *top),
            _ => return None,
        };
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same one
        // addition every other overlay makes.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let mut quad = widget.quad;
        for x in quad.iter_mut().step_by(2) {
            *x += edge;
        }
        let list =
            viewer_ui::chrome::ChoiceList::of(chrome, quad, &options, &selected, first, window)?;
        Some((list, kind))
    }

    /// Takes §12.7.5.4's options off the screen.
    pub(crate) fn close_choices(&mut self) -> bool {
        if self.choosing.take().is_none() {
            return false;
        }
        self.redraw();
        true
    }

    /// A press while §12.7.5.4's options are on the screen. Answers whether the list took it.
    ///
    /// A press on a row is the selection; a press anywhere else dismisses the list and goes no
    /// further, because a list that closed *and* acted on what was behind it would act on
    /// something the person pressing could not see.
    pub(crate) fn press_on_choices(&mut self, at: (f32, f32)) -> bool {
        let Some((list, kind)) = self.choices() else {
            return false;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let point = (at.0 + self.inset() as f32, at.1);
        let Some(option) = list.option_at(point) else {
            self.close_choices();
            return true;
        };
        let Some(choosing) = self.choosing.clone() else {
            return true;
        };
        let (value, stays) = match &kind {
            // Table 233 bit 22 set: "more than one of the field's option items may be selected
            // simultaneously". A press adds or removes one and the list stays up, because a person
            // choosing several has not finished after the first.
            ControlKind::List {
                selected,
                multi: true,
                ..
            } => {
                let mut chosen: Vec<usize> = selected.clone();
                if let Some(place) = chosen.iter().position(|held| *held == option) {
                    chosen.remove(place);
                } else {
                    chosen.push(option);
                    // Table 234's `/I` wants them "sorted in ascending order", and this is the
                    // list that becomes it.
                    chosen.sort_unstable();
                }
                (Entered::Chosen(chosen), true)
            }
            // "at most one item shall be selected", and a drop-down shows one value.
            _ => (Entered::Chosen(vec![option]), false),
        };
        self.dispatch(Command::Edit(Edit::SetField {
            field: choosing.field,
            value,
        }));
        if !stays {
            self.close_choices();
        }
        self.redraw();
        true
    }

    /// Aims the keyboard at §12.5.6.6's annotation under the point, whoever wrote it.
    ///
    /// The same two questions a field takes, in the same order and for the same reasons:
    /// `Query::FreeTextAt` names the annotation and hands back Table 166's `/Contents` as it reads
    /// now, and `Query::Offset` says where inside that text the press landed. Neither is a second
    /// implementation of anything — the core answers both from §12.7.4.3's own layout, which is
    /// the subclause §12.5.6.6 sends this subtype to.
    ///
    /// **This host needed not one line to reach the file's own annotations** (ADR 0304): the core
    /// answered `Query::FreeTextAt` for an annotation a person had added and now answers it for
    /// the producer's too, and `Edit::SetFreeText` already named its target by object. A capability
    /// that arrives through an existing message is what `doc/ui-boundary.md` is arguing for.
    fn aim_at_free_text(&mut self, at: (f32, f32)) -> Option<Typing> {
        let Answer::FreeText { annotation, text } = self.viewer.query(Query::FreeTextAt { at })
        else {
            return None;
        };
        println!(
            "note: typing into the free text annotation {} {}",
            annotation.number, annotation.generation
        );
        let caret = match self.viewer.query(Query::Offset { at, point: at }) {
            Answer::Offset(offset) => offset,
            _ => text.len(),
        };
        Some(Typing::at_offset(Target::FreeText(annotation), at, caret))
    }

    /// One end of the drag that draws §12.5.6.6's rectangle.
    ///
    /// The press records a corner and the release sends `Edit::FreeText` with both, then aims the
    /// keyboard at what it made — which needs no event, because `Query::FreeTextAt` at a point
    /// inside the rectangle answers with the annotation the core just added. Asking rather than
    /// being told is the rule this vocabulary has grown by: a message is added for a question a
    /// host cannot answer for itself, and this is not one.
    ///
    /// **The mode ends with the release, whether or not anything was made.** A drag that never
    /// moved has drawn no box and the core answers it by doing nothing (`add_free_text` refuses a
    /// rectangle with no area); leaving the mode armed after that would be a program that seemed
    /// stuck.
    pub(crate) fn draw_free_text(&mut self, drawing: Drawing, element: ElementState) {
        let at = self.on_page(self.cursor);
        match (drawing, element) {
            (Drawing::Armed, ElementState::Pressed) => self.drawing = Some(Drawing::From(at)),
            (Drawing::From(from), ElementState::Released) => {
                self.drawing = None;
                // The colour of the *text*, which Table 177's `/DA` carries — Table 166's `/C` is
                // an icon's background, a popup's title bar and a link's border, none of which
                // this subtype has. A dark red is this host's choice and no clause's, the same
                // footing the highlight's yellow stands on.
                self.dispatch(Command::Edit(Edit::FreeText {
                    from,
                    to: at,
                    colour: FREE_TEXT_INK,
                }));
                let middle = ((from.0 + at.0) * 0.5, (from.1 + at.1) * 0.5);
                self.typing = self.aim_at_free_text(middle);
                if self.typing.is_none() {
                    println!("note: that rectangle has no area, so there is nothing to type in");
                }
                self.redraw();
            }
            (Drawing::Armed, ElementState::Released)
            | (Drawing::From(_), ElementState::Pressed) => {}
        }
    }

    /// §12.7.5.2's two toggling kinds, clicked at a point of the page's own viewport.
    ///
    /// The point is the caller's, for the reason [`App::aim_at_field`] gives.
    ///
    /// **What `Query::Fields` made possible and nothing else could.** `Edit::SetField` takes a
    /// string, and for a check box or a radio button the only strings that mean anything are the
    /// names Table 170's appearance dictionary is keyed by — §12.7.5.2.3 makes `/V` "a name object
    /// representing the check box's appearance state, which shall be used to select the
    /// appropriate appearance from the appearance dictionary" — and those names are the file's own
    /// invention. This host had no way to learn one, so it could type into a form and not tick a
    /// box in it. `FormWidget::on_state` is the name to send. ADR 0235.
    ///
    /// Which widget: `Query::FieldAt` is the hit test, because it is the model's own and because a
    /// second one here could disagree with it; the quadrilaterals then say *which of the field's
    /// widgets* the point was in, which is the question §12.7.5.2.4's set makes real — "at most
    /// one button in a set may be on at any given time", and the one that goes on is the one under
    /// the pointer.
    ///
    /// What to send: §12.7.5.2.4 states the rule for turning one off, and it is a flag rather than
    /// a convention — "[i]f set, exactly one radio button shall be selected at all times; selecting
    /// the currently selected button has no effect. If clear, clicking the selected button
    /// deselects it, leaving no button selected."
    ///
    /// Table 227 bit 1 is checked here as well as in the core, and neither is redundant: the core
    /// refuses the edit, and a host that sent it anyway would be a program that looks broken
    /// rather than one that obeys the document.
    pub(crate) fn toggle_button(&mut self, at: (f32, f32)) {
        let Answer::Field {
            name, value: None, ..
        } = self.viewer.query(Query::FieldAt(at))
        else {
            // A field whose value is text is one to type into, which `aim_at_field` has already
            // decided about; no field at all is a press on the page.
            return;
        };
        let qualified = name.qualified.clone();
        let Answer::Fields(fields) = self.viewer.query(Query::Fields) else {
            return;
        };
        let Some(field) = fields
            .iter()
            .find(|field| field.name.qualified == qualified)
        else {
            return;
        };
        let no_toggle_to_off = match field.control {
            Control::CheckBox { .. } => false,
            Control::RadioButton {
                no_toggle_to_off, ..
            } => no_toggle_to_off,
            // §12.7.5.2.2's push-button responds to input "without retaining a permanent value",
            // a signature field's is a dictionary, and neither is a control a click gives a
            // value to. (This line quoted "retains no permanent value" until the
            // four-hundred-and-nineteenth session; Errata Collection 3 strikes the sentence that
            // phrase is from — Issue #386 — and the definition above is what survives.)
            _ => return,
        };
        if field.read_only {
            println!("note: the field {} is read-only (Table 227)", name.shown());
            return;
        }
        // The *last* widget covering the point, because §12.5.2 draws them in `/Annots` order and
        // the one on top is the one under the pointer — the same rule `pdf_model::view::field_at`
        // applies one level down.
        let Some(widget) = field
            .widgets
            .iter()
            .rev()
            .find(|widget| covers(widget.quad, at))
        else {
            return;
        };
        let value = if widget.on {
            if no_toggle_to_off {
                return;
            }
            // §12.7.5.2.3 names the off state; §12.7.5.2.4 gives it as the default value.
            "Off".to_owned()
        } else {
            let Some(state) = widget.on_state.clone() else {
                println!(
                    "note: the field {} states no appearance for an on state (§12.7.5.2.3)",
                    name.shown()
                );
                return;
            };
            state
        };
        println!("note: setting the field {} to {value}", name.shown());
        self.dispatch(Command::Edit(Edit::SetField {
            field: qualified,
            value: Entered::Text(value),
        }));
    }

    /// Aims the keyboard at whatever §12.5.1's tab walk just landed on, where it takes text.
    ///
    /// **The decision `doc/todo/33` left open, and it needed no new message.** The worry it
    /// recorded was that a focus ring on a *button* means something else — a press activates it,
    /// it does not take characters — and the answer is the one this host already uses for a
    /// click: `Answer::Field`'s value is `Some` only for a field §12.7.4.3 lays text out for, so
    /// the same question decides both. What was missing was only the *point*, and `Query::Focus`
    /// answers with the annotation's quadrilateral in the same device pixels `Query::FieldAt`
    /// takes, so the centre of the ring is the point to ask about.
    ///
    /// A walk onto anything else takes the keyboard back to the page, which is what makes Tab out
    /// of a field stop typing without a second binding for it.
    pub(crate) fn aim_at_focus(&mut self) {
        let Answer::Focus { quad, .. } = self.viewer.query(Query::Focus) else {
            self.typing = None;
            return;
        };
        // The centre of the ring, which is inside the widget's `/Rect` by construction: §12.5.5
        // places the appearance *on* that rectangle and `Query::Focus` answers with it.
        let at = ((quad[0] + quad[4]) * 0.5, (quad[1] + quad[5]) * 0.5);
        self.typing = match self.viewer.query(Query::FieldAt(at)) {
            // The same refusal a click makes, for the same reason — see `aim_at_field`.
            Answer::Field {
                value: Some(shown), ..
            } if shown.obscured => None,
            // Table 233 bit 19, refused exactly as the click above refuses it: §12.5.1's walk is a
            // second way into a field and a rule that held for one of them would be a rule with a
            // hole in it.
            Answer::Field {
                name,
                value: Some(_),
            } if !self.takes_characters_at(at) => {
                println!(
                    "note: {} states Table 233 bit 19 clear, so it shall include only a drop-down \
                     list — pick one of its options rather than typing a value",
                    name.shown()
                );
                None
            }
            Answer::Field {
                name,
                value: Some(value),
            } => {
                println!("note: typing into the field {}", name.shown());
                // The end of the value, and here that is not a fallback: a tab press names no
                // point inside the value, so there is nothing for `Query::Offset` to measure and
                // the end is the place ADR 0211 chose for a walk that arrives without one.
                Some(Typing::at_offset(Target::Field, at, value.text.len()))
            }
            _ => None,
        };
        self.redraw();
    }

    /// Ctrl + C, X and V inside a field's value: what the press does to the value, if anything.
    ///
    /// **The finding this round records: copying, cutting and pasting inside a field needed no
    /// message.** The two offsets are into the value `Query::FieldAt` just answered with, so the
    /// characters between them are a slice this host already holds; cutting and pasting are that
    /// slice spliced out or in and sent back as exactly the `Edit::SetField` every keystroke
    /// sends. Nothing crosses the boundary that did not cross it before, which is why ADR 0225
    /// added two questions and no verbs.
    ///
    /// `None` for a character this does not answer, which the caller consumes rather than passing
    /// on. A copy answers `None` for a different reason and says so where it happens: it changes
    /// no value, so there is nothing for the caller to send.
    fn clipped(
        &mut self,
        text: &str,
        current: &str,
        range: (usize, usize),
    ) -> Option<(Option<String>, usize, usize)> {
        let (low, high) = range;
        match text {
            "c" | "x" => {
                current
                    .get(low..high)
                    .unwrap_or_default()
                    .clone_into(&mut self.clipboard);
                println!(
                    "note: {} {} bytes out of the field",
                    if text == "c" { "copied" } else { "cut" },
                    self.clipboard.len()
                );
                // **The same platform end a copy off the page takes** (ADR 0519): text a person
                // took out of §12.7.4.3's value is text they meant to paste somewhere, and a
                // viewer whose form fields copy only into themselves is the half-feature this
                // round exists to close. Table 231 bit 14's password field never reaches here —
                // `aim_at_field` refuses the keyboard to one — so nothing this puts on the
                // session's clipboard is a value the document said to obscure.
                if let Err(error) = self.platform_clipboard.put(&self.clipboard) {
                    println!("note: {error}, so the copy stayed inside this program");
                }
                if text == "c" {
                    self.redraw();
                    return Some((None, low, high));
                }
                Some((Some(spliced(current, low, high, "")), low, low))
            }
            "v" => {
                let to = low.saturating_add(self.clipboard.len());
                Some((Some(spliced(current, low, high, &self.clipboard)), to, to))
            }
            _ => None,
        }
    }

    /// One key press, while a field has the keyboard. Answers whether it was consumed.
    ///
    /// **Nothing is buffered here.** Every press re-asks the core what the field says and sends
    /// back that value with one character added or one removed, so §12.7.5.3's `DoNotScroll`
    /// truncating a value is a thing the host *reads* rather than a thing it has to predict
    /// (ADR 0197). It costs a query per keystroke, which is a walk of one page's annotations.
    pub(crate) fn typed(&mut self, key: &Key<&str>) -> bool {
        let Some(typing) = self.typing else {
            return false;
        };
        let Some((current, aim)) = self.aimed(typing) else {
            // The field or the annotation went away — a page turned under the pointer — so the
            // keyboard goes back.
            self.typing = None;
            return false;
        };
        // The caret is clamped to the value *this* press starts from, because the last one may
        // have been truncated by §12.7.5.3's `DoNotScroll` — the same reason nothing is buffered.
        let caret = caret_boundary(&current, typing.caret);
        let anchor = caret_boundary(&current, typing.anchor);
        let (low, high) = (caret.min(anchor), caret.max(anchor));
        // **Shift holds the anchor still and moves only the caret**, which is this host's
        // convention and no clause's — the standard states neither a cursor nor a selection inside
        // a value (ADR 0225). Without it a move collapses the selection, which is why every arm
        // below says where *both* ends go.
        let held = |to: usize| if self.shift { (to, anchor) } else { (to, to) };
        let (next, moved, anchored) = match *key {
            Key::Named(NamedKey::Escape) => {
                self.typing = None;
                println!("note: the keyboard is back on the page");
                // The caret goes with the keyboard, and the window is what has to be told: this
                // press changes nothing about the *document*, so no command is sent and nothing
                // else would ask for the frame that takes the caret off the screen.
                self.redraw();
                return true;
            }
            // Ctrl + C, X and V, which needed no message at all — see `clipped`.
            Key::Character(text) if self.control => match self.clipped(text, &current, (low, high))
            {
                Some(outcome) => outcome,
                // Every other key with Control held is consumed rather than sent to the page: a
                // field has the keyboard, and a magnification while somebody is typing is the
                // surprise this whole state exists to prevent.
                None => return true,
            },
            // Moving the caret changes nothing about the document, so these send no edit at all
            // and only ask for the frame that redraws the caret. A move that is not extending the
            // selection lands on the *edge* of it rather than one character further, which is what
            // a person means by pressing Left with something selected.
            Key::Named(NamedKey::ArrowLeft) => {
                let (to, anchor) = stepped(&current, (caret, anchor), self.shift, false);
                (None, to, anchor)
            }
            Key::Named(NamedKey::ArrowRight) => {
                let (to, anchor) = stepped(&current, (caret, anchor), self.shift, true);
                (None, to, anchor)
            }
            Key::Named(NamedKey::Home) => {
                let (to, anchor) = held(0);
                (None, to, anchor)
            }
            Key::Named(NamedKey::End) => {
                let (to, anchor) = held(current.len());
                (None, to, anchor)
            }
            // With something selected, Backspace and Delete take out what is selected and nothing
            // more — the same statement typing a character makes, and the reason both are one
            // splice rather than two behaviours.
            Key::Named(NamedKey::Backspace) => {
                let from = if low < high {
                    low
                } else {
                    before(&current, caret)
                };
                (Some(spliced(&current, from, high, "")), from, from)
            }
            Key::Named(NamedKey::Delete) => {
                let to = if low < high {
                    high
                } else {
                    after(&current, caret)
                };
                (Some(spliced(&current, low, to, "")), low, low)
            }
            Key::Named(NamedKey::Enter) => {
                // §12.7.5.3's Multiline decides whether a return is a character or the end of
                // typing, and the core is what knows: a value with a newline in it lays out on two
                // lines only where Table 231 bit 13 is set, and `variable_text::wrap` is where
                // that is read. So the host offers the newline and the field decides what to keep.
                let to = low.saturating_add(1);
                (Some(spliced(&current, low, high, "\n")), to, to)
            }
            Key::Character(text) if !text.is_empty() => {
                let to = low.saturating_add(text.len());
                (Some(spliced(&current, low, high, text)), to, to)
            }
            Key::Named(NamedKey::Space) => {
                let to = low.saturating_add(1);
                (Some(spliced(&current, low, high, " ")), to, to)
            }
            _ => return false,
        };
        self.typing = Some(Typing {
            caret: moved,
            anchor: anchored,
            ..typing
        });
        // A caret that moved is chrome and not a page: `Query::Caret` answers from state this host
        // holds, so nothing has to be interpreted again and the window only repaints. A keystroke
        // that leaves the value as it was is the same case, and there are two of them — Backspace
        // at the start and Delete at the end — where sending the edit anyway would put an entry in
        // the log, mark the document unsaved and re-interpret the page for a picture that cannot
        // differ.
        let Some(next) = next.filter(|next| *next != current) else {
            self.redraw();
            return true;
        };
        // Through `dispatch`, not through `Viewer::handle` directly: the events an edit raises
        // are what asks for the next frame, and a host that counted them instead of pumping them
        // would type into a page that never redraws. (It did, for one run.)
        self.dispatch(Command::Edit(match aim {
            Aim::Field(field) => Edit::SetField {
                field,
                value: Entered::Text(next),
            },
            Aim::FreeText(annotation) => Edit::SetFreeText {
                annotation,
                text: next,
            },
        }));
        // And the field decides how much of that it took, so where the caret ended up is read
        // back rather than assumed — a value §12.7.5.3 truncated is shorter than what was sent.
        if let Some((taken, _)) = self.aimed(typing) {
            self.typing = Some(Typing {
                caret: caret_boundary(&taken, moved),
                anchor: caret_boundary(&taken, anchored),
                ..typing
            });
        }
        true
    }

    /// What the thing being typed into says now, and how an edit to it is addressed.
    ///
    /// One question per keystroke, asked twice — once before the key is applied and once after,
    /// because what the *document* took is what the caret has to be clamped to (ADR 0197). Which
    /// question it is depends on the target and nothing else does: a field is addressed by
    /// §12.7.4.2's qualified name and §12.5.6.6's annotation by its object, because an annotation
    /// has no name for anything to address it by.
    ///
    /// `None` where the thing is no longer under the point — a page turned under the pointer —
    /// which is what takes the keyboard back to the page.
    fn aimed(&self, typing: Typing) -> Option<(String, Aim)> {
        match typing.target {
            Target::Field => match self.viewer.query(Query::FieldAt(typing.at)) {
                // A value that is Table 231 bit 14's echo is not a value to append to, which is
                // why `aim_at_field` never puts the keyboard here; this is the same refusal one
                // step later, so that the two cannot come apart. ADR 0247.
                Answer::Field {
                    value: Some(shown), ..
                } if shown.obscured => None,
                Answer::Field { name, value } => Some((
                    value.map_or_else(String::new, |shown| shown.text),
                    Aim::Field(name.qualified),
                )),
                _ => None,
            },
            Target::FreeText(annotation) => {
                match self.viewer.query(Query::FreeTextAt { at: typing.at }) {
                    Answer::FreeText { text, .. } => Some((text, Aim::FreeText(annotation))),
                    _ => None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{after, before, caret_boundary, spliced};

    /// What a keystroke does to a value, on the one input that breaks a naive index.
    ///
    /// A caret is a byte offset into a value the *core* owns: §12.7.5.3's `DoNotScroll` shortens
    /// it under the host, a page turn can take the field away, and a character is one to four
    /// bytes. So every use of the offset goes through `caret_boundary`, and the splice is written
    /// with `get` rather than with indexing — a panic here would be a program that quits because
    /// somebody typed an accent.
    #[test]
    fn a_caret_never_falls_inside_a_character() {
        let value = "café";
        assert_eq!(value.len(), 5, "é is two bytes");
        // The offset between the two bytes of `é` is not a place a caret can be.
        assert_eq!(caret_boundary(value, 4), 3);
        // And one past the end of a value the field truncated is its end.
        assert_eq!(caret_boundary(value, 99), value.len());
        assert_eq!(
            before(value, 5),
            3,
            "one character back from the end is before é"
        );
        assert_eq!(after(value, 3), 5, "and one forward from there is past it");
        assert_eq!(after(value, 5), 5, "the end of the value stays put");
        assert_eq!(before(value, 0), 0, "so does the start");
        // Backspace at the end, and an insertion in the middle.
        assert_eq!(spliced(value, before(value, 5), 5, ""), "caf");
        assert_eq!(spliced(value, 1, 1, "X"), "cXafé");
    }
}
