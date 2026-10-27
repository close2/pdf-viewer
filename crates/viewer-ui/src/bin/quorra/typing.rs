//! The keyboard inside a value: §12.7.4.3's field, §12.5.6.6's annotation, and the clipboard.
//!
//! Two things on a page take characters and the standard sends both to the same subclause, so
//! they are one module: what differs is the question that reads the text back and the edit that
//! puts it there, and everything between those two — the caret, the selection, what a key does
//! to a value — is shared. The clipboard is here because the same three keys serve a field's
//! value and the page's own selection, and because what a copy *is* belongs to a host.
//!
//! **The `impl App` is continued in five modules by what a key press does**, the way `surface` is
//! (ADR 2424): `clipboard` (a copy off the page, and the field's own copy, cut and paste),
//! `aiming` (which control a press or a focus addresses), `choices` (§12.7.5.4's open list),
//! `annotations` (the free text being drawn and the note being typed into) and `keys` (one press
//! while a field has the keyboard). The types, the caret arithmetic they share and its tests stay here.

use pdf_syntax::ObjectId;
use viewer_host::form::ControlKind;

#[path = "typing/aiming.rs"]
mod aiming;
#[path = "typing/annotations.rs"]
mod annotations;
#[path = "typing/choices.rs"]
mod choices;
#[path = "typing/clipboard.rs"]
mod clipboard;
#[path = "typing/keys.rs"]
mod keys;

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
        // Between the two halves of a carriage return and line feed is no place either: the pair
        // is one paragraph break (ADR 1798), so the caret stands before it.
        let inside_a_break = value.get(..offset).is_some_and(|head| head.ends_with('\r'))
            && value
                .get(offset..)
                .is_some_and(|tail| tail.starts_with('\n'));
        return if inside_a_break {
            offset.saturating_sub(1)
        } else {
            offset
        };
    }
    value
        .char_indices()
        .map(|(at, _)| at)
        .take_while(|at| *at < offset)
        .last()
        .unwrap_or(value.len())
}

/// The offset one character before `caret`, or the start of the value — a carriage return and
/// line feed together counting as one, since the pair is one paragraph break wherever this host
/// draws text (ADR 1798) and an arrow or a Backspace that took half of it would change nothing
/// on the screen.
fn before(value: &str, caret: usize) -> usize {
    let Some(prefix) = value.get(..caret) else {
        return 0;
    };
    if prefix.ends_with("\r\n") {
        return caret.saturating_sub(2);
    }
    prefix.char_indices().next_back().map_or(0, |(at, _)| at)
}

/// The offset one character after `caret`, or the end of the value — [`before`]'s pair counted
/// as one from the other side.
fn after(value: &str, caret: usize) -> usize {
    let Some(rest) = value.get(caret..) else {
        return value.len();
    };
    if rest.starts_with("\r\n") {
        return caret.saturating_add(2);
    }
    rest.chars().next().map_or(value.len(), |character| {
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
    /// The option an arrow scrolled the list to start at, where one has: until then a list box
    /// starts at Table 234's `/TI` and a drop-down at its value (ADR 1786).
    pub(crate) first: Option<usize>,
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
/// its own annotation to that same subclause — "\[s]ubclause 12.7.4.3, 'Variable text', describes
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
    /// §12.5.6.4's text note, typed into in its popup window rather than on the page — named by
    /// its object for the free text annotation's reason (ADR 1726).
    Note(ObjectId),
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

    /// The note whose window has the keyboard, where one has.
    pub(crate) fn note(self) -> Option<ObjectId> {
        match self.target {
            Target::Note(note) => Some(note),
            Target::Field | Target::FreeText(_) => None,
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

    /// A carriage return and line feed is one paragraph break where the note's window draws it
    /// (ADR 1798), so the caret steps over the pair in one press and Backspace takes both halves:
    /// half a break would leave the window drawing the same text and the person seeing nothing
    /// happen. A lone carriage return or line feed is still one character.
    #[test]
    fn a_carriage_return_and_line_feed_are_one_step_for_the_caret() {
        let value = "ab\r\ncd";
        assert_eq!(
            after(value, 2),
            4,
            "Right from the end of a paragraph crosses the pair"
        );
        assert_eq!(
            before(value, 4),
            2,
            "and Left from the start of the next crosses it back"
        );
        assert_eq!(spliced(value, before(value, 4), 4, ""), "abcd");
        assert_eq!(
            caret_boundary(value, 3),
            2,
            "between the halves stands before the pair"
        );
        assert_eq!(after("ab\rcd", 2), 3);
        assert_eq!(before("ab\ncd", 3), 2);
        assert_eq!(
            before("a\n\rb", 3),
            2,
            "a line feed then a carriage return is two breaks"
        );
    }
}
