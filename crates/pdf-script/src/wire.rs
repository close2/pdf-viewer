//! A [`Request`] and an [`Outcome`] as bytes, for the process boundary RFC 0008 section 6.2 puts
//! the engine behind.
//!
//! `confined-transport` carries a kind byte and a length and knows nothing of what is inside; this
//! is what is inside: a request, an outcome, and a script's question and the person's answer. Every integer is little-endian, every string is a `u32` byte length and that
//! many bytes of UTF-8, every boolean is one byte that is 0 or 1, and every enumeration is a tag
//! byte followed by its fields. Both encodings begin with a version byte, so that a worker and a
//! host built from different trees refuse each other rather than misread each other. Decoding
//! reads a length only against the bytes that remain, so a hostile length asks for nothing.

use std::time::Duration;

use pdf_model::action::{PageTrigger, Trigger as AnnotationTrigger};
use pdf_model::aform::Trigger;
use pdf_model::view::{
    Alignment, AnnotationChange, AnnotationReach, AnnotationState, BorderStyle, Colour, Display,
    DocumentState, DocumentTrigger, Face, FieldState, FieldType, Glyph, InfoEntry, Layer, Property,
    ScriptEdit, ScriptSite, Sound, TextFlag, WidgetState,
};

use crate::{
    Answer, Button, Buttons, Ending, Event, Exceeded, Icon, Outcome, Question, Refusal,
    RefusalKind, Request,
};

/// The first byte of every encoding this module writes.
///
/// Moved whenever what crosses changes shape: 4 carries a commit's key, a full field's two
/// changes, the unsaved mark, the document's information dictionary and groups, a push-button's
/// captions, a layer's switch, the depth budget, a run's notes, and a question and its answer
/// (ADRs 1626, 1627); 5 a script's page turn (ADR 1640); 6 a field's widgets each with its own
/// state, and a property set on one of them (ADR 1664); 7 the widget a focus request names (ADR
/// 1688), and the on state of a toggling widget (ADR 1689); 8 a timer's site, a timer set and
/// cleared, and a sound asked for (ADR 1702), and the document's annotations and a script's change
/// to one (ADR 1700); 9 what an annotation's Table 167 flags let it reach — paper, a screen, a
/// pointer — beside its two bits (ADR 1721).
pub const VERSION: u8 = 9;

/// Most fields one request may tell a realm of, and most edits one outcome may carry.
///
/// A count is read before the items it counts, and every item is at least one byte, so a hostile
/// count is already bounded by the bytes that remain; this bounds what an honest encoder of a
/// pathological form would allocate, at four times the largest form the census population holds.
const MAX_ITEMS: u32 = 1 << 16;

/// Why bytes are not an encoding this module wrote.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    /// The bytes end inside a field.
    #[error("the message ends inside a field")]
    Truncated,
    /// The version byte is not [`VERSION`].
    #[error("the message is version {0}, and this reads version {VERSION}")]
    Version(u8),
    /// A tag or a boolean has a value no encoding uses.
    #[error("the message's {0} has a value no encoding uses")]
    Invalid(&'static str),
    /// A string is not UTF-8.
    #[error("a string in the message is not UTF-8")]
    Utf8,
    /// Bytes remain after the message.
    #[error("{0} bytes follow the message")]
    Trailing(usize),
}

/// Encodes a request.
#[must_use]
pub fn encode_request(request: &Request) -> Vec<u8> {
    let mut out = vec![VERSION];
    put_site(&mut out, request.site);
    put_str(&mut out, &request.field);
    put_str(&mut out, &request.label);
    put_str(&mut out, &request.script);
    put_str(&mut out, &request.event.value);
    put_str(&mut out, &request.event.change);
    put_u32(&mut out, request.event.selection_start);
    put_u32(&mut out, request.event.selection_end);
    put_bool(&mut out, request.event.will_commit);
    put_u8(&mut out, request.event.commit_key);
    put_bool(&mut out, request.event.field_full);
    put_str(&mut out, &request.event.change_ex);
    put_str(&mut out, &request.event.source);
    put_len(&mut out, request.fields.len());
    for field in &request.fields {
        put_field(&mut out, field);
    }
    put_u32(&mut out, request.page);
    put_u32(&mut out, request.pages);
    put_bool(&mut out, request.dirty);
    match &request.document {
        None => put_u8(&mut out, 0),
        Some(document) => {
            put_u8(&mut out, 1);
            put_document(&mut out, document);
        }
    }
    put_u64(&mut out, request.moment);
    out.extend_from_slice(&request.utc_offset_seconds.to_le_bytes());
    out
}

/// Decodes a request.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one request of this version.
pub fn decode_request(bytes: &[u8]) -> Result<Request, WireError> {
    let mut reader = Reader::new(bytes)?;
    let site = reader.site()?;
    let field = reader.string()?;
    let label = reader.string()?;
    let script = reader.string()?;
    let event = Event {
        value: reader.string()?,
        change: reader.string()?,
        selection_start: reader.u32()?,
        selection_end: reader.u32()?,
        will_commit: reader.boolean()?,
        commit_key: match reader.u8()? {
            key @ 0..=3 => key,
            _ => return Err(WireError::Invalid("commit key")),
        },
        field_full: reader.boolean()?,
        change_ex: reader.string()?,
        source: reader.string()?,
    };
    let count = reader.count()?;
    let mut fields = Vec::new();
    for _ in 0..count {
        fields.push(reader.field()?);
    }
    let request = Request {
        site,
        field,
        label,
        script,
        event,
        fields,
        page: reader.u32()?,
        pages: reader.u32()?,
        dirty: reader.boolean()?,
        document: match reader.u8()? {
            0 => None,
            1 => Some(reader.document()?),
            _ => return Err(WireError::Invalid("document")),
        },
        moment: reader.u64()?,
        utc_offset_seconds: i32::from_le_bytes(reader.array()?),
    };
    reader.finish()?;
    Ok(request)
}

/// Encodes an outcome.
#[must_use]
pub fn encode_outcome(outcome: &Outcome) -> Vec<u8> {
    let mut out = vec![VERSION];
    put_bool(&mut out, outcome.rc);
    put_optional(&mut out, outcome.value.as_deref());
    put_optional(&mut out, outcome.change.as_deref());
    put_len(&mut out, outcome.edits.len());
    for edit in &outcome.edits {
        put_edit(&mut out, edit);
    }
    match &outcome.ending {
        Ending::Finished => put_u8(&mut out, 0),
        Ending::Exceeded(exceeded) => {
            put_u8(&mut out, 1);
            put_exceeded(&mut out, *exceeded);
        }
        Ending::Threw(text) => {
            put_u8(&mut out, 2);
            put_str(&mut out, text);
        }
        Ending::Unparsed(text) => {
            put_u8(&mut out, 3);
            put_str(&mut out, text);
        }
        Ending::Declined(text) => {
            put_u8(&mut out, 4);
            put_str(&mut out, text);
        }
    }
    put_len(&mut out, outcome.refusals.len());
    for refusal in &outcome.refusals {
        put_str(&mut out, &refusal.member);
        match &refusal.kind {
            RefusalKind::Excluded(reason) => {
                put_u8(&mut out, 0);
                put_str(&mut out, reason);
            }
            RefusalKind::NotBridged => put_u8(&mut out, 1),
            RefusalKind::Unreachable(why) => {
                put_u8(&mut out, 2);
                put_str(&mut out, why);
            }
            RefusalKind::Library(why) => {
                put_u8(&mut out, 3);
                put_str(&mut out, why);
            }
        }
    }
    put_len(&mut out, outcome.log.len());
    for line in &outcome.log {
        put_str(&mut out, line);
    }
    put_len(&mut out, outcome.notes.len());
    for note in &outcome.notes {
        put_str(&mut out, note);
    }
    out
}

/// Encodes a question.
#[must_use]
pub fn encode_question(question: &Question) -> Vec<u8> {
    let mut out = vec![VERSION];
    match question {
        Question::Alert {
            message,
            icon,
            buttons,
            title,
        } => {
            put_u8(&mut out, 0);
            put_str(&mut out, message);
            put_u8(&mut out, tag_of(&Icon::ALL, icon));
            put_u8(&mut out, tag_of(&Buttons::ALL, buttons));
            put_optional(&mut out, title.as_deref());
        }
        Question::Response {
            question,
            title,
            default,
            label,
            password,
        } => {
            put_u8(&mut out, 1);
            put_str(&mut out, question);
            put_optional(&mut out, title.as_deref());
            put_str(&mut out, default);
            put_optional(&mut out, label.as_deref());
            put_bool(&mut out, *password);
        }
    }
    out
}

/// Decodes a question.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one question of this version.
pub fn decode_question(bytes: &[u8]) -> Result<Question, WireError> {
    let mut reader = Reader::new(bytes)?;
    let question = match reader.u8()? {
        0 => Question::Alert {
            message: reader.string()?,
            icon: reader.tagged(&Icon::ALL, "icon")?,
            buttons: reader.tagged(&Buttons::ALL, "buttons")?,
            title: reader.optional()?,
        },
        1 => Question::Response {
            question: reader.string()?,
            title: reader.optional()?,
            default: reader.string()?,
            label: reader.optional()?,
            password: reader.boolean()?,
        },
        _ => return Err(WireError::Invalid("question")),
    };
    reader.finish()?;
    Ok(question)
}

/// Encodes an answer.
#[must_use]
pub fn encode_answer(answer: &Answer) -> Vec<u8> {
    let mut out = vec![VERSION];
    match answer {
        Answer::Pressed(button) => {
            put_u8(&mut out, 0);
            put_u8(&mut out, tag_of(&Button::ALL, button));
        }
        Answer::Typed(text) => {
            put_u8(&mut out, 1);
            put_optional(&mut out, text.as_deref());
        }
        Answer::Unanswerable => put_u8(&mut out, 2),
    }
    out
}

/// Decodes an answer.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one answer of this version.
pub fn decode_answer(bytes: &[u8]) -> Result<Answer, WireError> {
    let mut reader = Reader::new(bytes)?;
    let answer = match reader.u8()? {
        0 => Answer::Pressed(reader.tagged(&Button::ALL, "button")?),
        1 => Answer::Typed(reader.optional()?),
        2 => Answer::Unanswerable,
        _ => return Err(WireError::Invalid("answer")),
    };
    reader.finish()?;
    Ok(answer)
}

/// Decodes an outcome.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one outcome of this version.
pub fn decode_outcome(bytes: &[u8]) -> Result<Outcome, WireError> {
    let mut reader = Reader::new(bytes)?;
    let rc = reader.boolean()?;
    let value = reader.optional()?;
    let change = reader.optional()?;
    let count = reader.count()?;
    let mut edits = Vec::new();
    for _ in 0..count {
        edits.push(reader.edit()?);
    }
    let ending = match reader.u8()? {
        0 => Ending::Finished,
        1 => Ending::Exceeded(reader.exceeded()?),
        2 => Ending::Threw(reader.string()?),
        3 => Ending::Unparsed(reader.string()?),
        4 => Ending::Declined(reader.string()?),
        _ => return Err(WireError::Invalid("ending")),
    };
    let count = reader.count()?;
    let mut refusals = Vec::new();
    for _ in 0..count {
        let member = reader.string()?;
        let kind = match reader.u8()? {
            0 => RefusalKind::Excluded(reader.string()?),
            1 => RefusalKind::NotBridged,
            2 => RefusalKind::Unreachable(reader.string()?),
            3 => RefusalKind::Library(reader.string()?),
            _ => return Err(WireError::Invalid("refusal")),
        };
        refusals.push(Refusal { member, kind });
    }
    let count = reader.count()?;
    let mut log = Vec::new();
    for _ in 0..count {
        log.push(reader.string()?);
    }
    let count = reader.count()?;
    let mut notes = Vec::new();
    for _ in 0..count {
        notes.push(reader.string()?);
    }
    reader.finish()?;
    Ok(Outcome {
        rc,
        value,
        change,
        edits,
        ending,
        refusals,
        log,
        notes,
    })
}

/// Table 199's four triggers, in tag order.
const FIELD_TRIGGERS: [Trigger; 4] = [
    Trigger::Keystroke,
    Trigger::Format,
    Trigger::Validate,
    Trigger::Calculate,
];

/// Table 197's ten events, in tag order.
const ANNOTATION_TRIGGERS: [AnnotationTrigger; 10] = [
    AnnotationTrigger::Enter,
    AnnotationTrigger::Exit,
    AnnotationTrigger::Down,
    AnnotationTrigger::Up,
    AnnotationTrigger::Focus,
    AnnotationTrigger::Blur,
    AnnotationTrigger::PageOpen,
    AnnotationTrigger::PageClose,
    AnnotationTrigger::PageVisible,
    AnnotationTrigger::PageInvisible,
];

/// Table 200's five events, in tag order.
const DOCUMENT_TRIGGERS: [DocumentTrigger; 5] = DocumentTrigger::ALL;

/// Table 231's text field flags, in tag order.
const TEXT_FLAGS: [TextFlag; 4] = TextFlag::ALL;

/// A push-button's captions, in tag order.
const FACES: [Face; 3] = Face::ALL;

/// The field types, in tag order.
const FIELD_TYPES: [FieldType; 7] = [
    FieldType::Text,
    FieldType::PushButton,
    FieldType::CheckBox,
    FieldType::RadioButton,
    FieldType::ComboBox,
    FieldType::ListBox,
    FieldType::Signature,
];

/// The display constants, in tag order.
const DISPLAYS: [Display; 4] = [
    Display::Visible,
    Display::Hidden,
    Display::NoPrint,
    Display::NoView,
];

/// The border styles, in tag order.
const BORDER_STYLES: [BorderStyle; 5] = [
    BorderStyle::Solid,
    BorderStyle::Dashed,
    BorderStyle::Beveled,
    BorderStyle::Inset,
    BorderStyle::Underline,
];

/// The alignments, in tag order.
const ALIGNMENTS: [Alignment; 3] = [Alignment::Left, Alignment::Center, Alignment::Right];

/// Where `item` is in `list`, as a tag byte.
fn tag_of<T: PartialEq>(list: &[T], item: &T) -> u8 {
    list.iter()
        .position(|held| held == item)
        .and_then(|index| u8::try_from(index).ok())
        .unwrap_or(u8::MAX)
}

/// Writes where a script runs.
fn put_site(out: &mut Vec<u8>, site: ScriptSite) {
    match site {
        ScriptSite::Field(trigger) => {
            put_u8(out, 0);
            put_u8(out, tag_of(&FIELD_TRIGGERS, &trigger));
        }
        ScriptSite::Annotation(trigger) => {
            put_u8(out, 1);
            put_u8(out, tag_of(&ANNOTATION_TRIGGERS, &trigger));
        }
        ScriptSite::Page(trigger) => {
            put_u8(out, 2);
            put_bool(out, trigger == PageTrigger::Close);
        }
        ScriptSite::OpenAction => put_u8(out, 3),
        ScriptSite::Library => put_u8(out, 4),
        ScriptSite::Document(trigger) => {
            put_u8(out, 5);
            put_u8(out, tag_of(&DOCUMENT_TRIGGERS, &trigger));
        }
        ScriptSite::Timer => put_u8(out, 6),
    }
}

/// Writes one field's state.
fn put_field(out: &mut Vec<u8>, field: &FieldState) {
    put_str(out, &field.name);
    put_u8(out, tag_of(&FIELD_TYPES, &field.kind));
    put_str(out, &field.value);
    put_u32(out, field.flags);
    for number in [field.char_limit, field.page] {
        put_optional_u32(out, number);
    }
    put_len(out, field.widgets.len());
    for widget in &field.widgets {
        put_widget(out, widget);
    }
}

/// Writes one widget's state.
fn put_widget(out: &mut Vec<u8>, widget: &WidgetState) {
    put_u8(out, tag_of(&DISPLAYS, &widget.display));
    for colour in [widget.text_color, widget.fill_color, widget.stroke_color] {
        match colour {
            None => put_u8(out, 0),
            Some(colour) => {
                put_u8(out, 1);
                put_colour(out, colour);
            }
        }
    }
    put_u8(out, tag_of(&BORDER_STYLES, &widget.border_style));
    put_u8(out, tag_of(&ALIGNMENTS, &widget.alignment));
    for coordinate in widget.rect {
        put_f64(out, coordinate);
    }
    for caption in &widget.captions {
        put_str(out, caption);
    }
    put_optional(out, widget.on_state.as_deref());
}

/// Writes a number that may be absent: 0, or 1 and the number.
fn put_optional_u32(out: &mut Vec<u8>, number: Option<u32>) {
    match number {
        None => put_u8(out, 0),
        Some(number) => {
            put_u8(out, 1);
            put_u32(out, number);
        }
    }
}

/// Writes the document as a whole: its information dictionary's entries, then its groups.
fn put_document(out: &mut Vec<u8>, document: &DocumentState) {
    put_len(out, document.info.len());
    for entry in &document.info {
        put_str(out, &entry.key);
        put_str(out, &entry.text);
        match entry.moment {
            None => put_u8(out, 0),
            Some(moment) => {
                put_u8(out, 1);
                out.extend_from_slice(&moment.to_le_bytes());
            }
        }
    }
    put_len(out, document.layers.len());
    for layer in &document.layers {
        put_u32(out, layer.number);
        out.extend_from_slice(&layer.generation.to_le_bytes());
        put_str(out, &layer.name);
        put_bool(out, layer.on);
        put_bool(out, layer.initially_on);
        put_bool(out, layer.locked);
    }
    put_len(out, document.annotations.len());
    for annotation in &document.annotations {
        put_annotation(out, annotation);
    }
}

/// Writes one annotation as a realm holds it (ADR 1700).
fn put_annotation(out: &mut Vec<u8>, annotation: &AnnotationState) {
    put_u32(out, annotation.number);
    out.extend_from_slice(&annotation.generation.to_le_bytes());
    put_u32(out, annotation.page);
    put_str(out, &annotation.kind);
    for corner in annotation.rect {
        put_f64(out, corner);
    }
    put_optional(out, annotation.name.as_deref());
    put_str(out, &annotation.contents);
    put_optional(out, annotation.author.as_deref());
    match annotation.modified {
        None => put_u8(out, 0),
        Some(moment) => {
            put_u8(out, 1);
            out.extend_from_slice(&moment.to_le_bytes());
        }
    }
    put_bool(out, annotation.hidden);
    put_bool(out, annotation.read_only);
    put_bool(out, annotation.reach.printed);
    put_bool(out, annotation.reach.viewed);
    put_bool(out, annotation.reach.interactive);
    match annotation.popup_open {
        None => put_u8(out, 0),
        Some(false) => put_u8(out, 1),
        Some(true) => put_u8(out, 2),
    }
}

/// Writes a colour: a tag, then its components.
fn put_colour(out: &mut Vec<u8>, colour: Colour) {
    let components: &[f64] = match &colour {
        Colour::Transparent => &[],
        Colour::Gray(gray) => std::slice::from_ref(gray),
        Colour::Rgb(rgb) => rgb,
        Colour::Cmyk(cmyk) => cmyk,
    };
    put_u8(out, u8::try_from(components.len()).unwrap_or(u8::MAX));
    for component in components {
        put_f64(out, *component);
    }
}

/// Writes one edit.
fn put_edit(out: &mut Vec<u8>, edit: &ScriptEdit) {
    match edit {
        ScriptEdit::Value { field, value } => {
            put_u8(out, 0);
            put_str(out, field);
            put_str(out, value);
        }
        ScriptEdit::Property {
            field,
            widget,
            property,
        } => {
            put_u8(out, 1);
            put_str(out, field);
            put_optional_u32(out, *widget);
            put_property(out, property);
        }
        ScriptEdit::Reset { fields } => {
            put_u8(out, 2);
            put_len(out, fields.len());
            for field in fields {
                put_str(out, field);
            }
        }
        ScriptEdit::Calculate => put_u8(out, 3),
        ScriptEdit::Focus { field, widget } => {
            put_u8(out, 4);
            put_str(out, field);
            put_optional_u32(out, *widget);
        }
        ScriptEdit::Layer {
            number,
            generation,
            on,
        } => {
            put_u8(out, 5);
            put_u32(out, *number);
            out.extend_from_slice(&generation.to_le_bytes());
            put_bool(out, *on);
        }
        ScriptEdit::GoTo { page } => {
            put_u8(out, 6);
            put_u32(out, *page);
        }
        ScriptEdit::Timer { .. } | ScriptEdit::ClearTimer { .. } | ScriptEdit::Beep { .. } => {
            put_host_edit(out, edit);
        }
        ScriptEdit::Annotation {
            number,
            generation,
            change,
        } => {
            put_u8(out, 10);
            put_u32(out, *number);
            out.extend_from_slice(&generation.to_le_bytes());
            put_annotation_change(out, change);
        }
    }
}

/// Writes one of the three edits a host's clock or speaker carries out: a timer set, a timer
/// cleared, a sound asked for (ADR 1702).
fn put_host_edit(out: &mut Vec<u8>, edit: &ScriptEdit) {
    match edit {
        ScriptEdit::Timer {
            id,
            script,
            period,
            repeat,
        } => {
            put_u8(out, 7);
            put_u32(out, *id);
            put_str(out, script);
            put_u32(out, *period);
            put_bool(out, *repeat);
        }
        ScriptEdit::ClearTimer { id } => {
            put_u8(out, 8);
            put_u32(out, *id);
        }
        ScriptEdit::Beep { sound } => {
            put_u8(out, 9);
            put_u8(out, tag_of(&Sound::ALL, sound));
        }
        // Every other edit is `put_edit`'s, which hands only these three here.
        _ => {}
    }
}

/// Writes one property a script set: a tag, then its value.
fn put_property(out: &mut Vec<u8>, property: &Property) {
    match property {
        Property::Display(display) => {
            put_u8(out, 0);
            put_u8(out, tag_of(&DISPLAYS, display));
        }
        Property::ReadOnly(flag) => {
            put_u8(out, 1);
            put_bool(out, *flag);
        }
        Property::Required(flag) => {
            put_u8(out, 2);
            put_bool(out, *flag);
        }
        Property::TextColor(colour) => {
            put_u8(out, 3);
            put_colour(out, *colour);
        }
        Property::FillColor(colour) => {
            put_u8(out, 4);
            put_colour(out, *colour);
        }
        Property::StrokeColor(colour) => {
            put_u8(out, 5);
            put_colour(out, *colour);
        }
        Property::BorderStyle(style) => {
            put_u8(out, 6);
            put_u8(out, tag_of(&BORDER_STYLES, style));
        }
        Property::Alignment(alignment) => {
            put_u8(out, 7);
            put_u8(out, tag_of(&ALIGNMENTS, alignment));
        }
        Property::CharLimit(limit) => {
            put_u8(out, 8);
            put_u32(out, *limit);
        }
        Property::TextFlag(flag, on) => {
            put_u8(out, 9);
            put_u8(out, tag_of(&TEXT_FLAGS, flag));
            put_bool(out, *on);
        }
        Property::Caption(face, caption) => {
            put_u8(out, 10);
            put_u8(out, tag_of(&FACES, face));
            put_str(out, caption);
        }
        Property::Style(glyph) => {
            put_u8(out, 11);
            put_u8(out, tag_of(&Glyph::ALL, glyph));
        }
    }
}

/// Writes what a script set on an annotation: a tag, then the value (ADR 1700).
fn put_annotation_change(out: &mut Vec<u8>, change: &AnnotationChange) {
    match change {
        AnnotationChange::Hidden(hidden) => {
            put_u8(out, 0);
            put_bool(out, *hidden);
        }
        AnnotationChange::PopupOpen(open) => {
            put_u8(out, 1);
            put_bool(out, *open);
        }
        AnnotationChange::Contents(text) => {
            put_u8(out, 2);
            put_str(out, text);
        }
    }
}

/// Writes which budget was exceeded.
fn put_exceeded(out: &mut Vec<u8>, exceeded: Exceeded) {
    match exceeded {
        Exceeded::Wall(limit) => {
            put_u8(out, 0);
            put_u64(out, u64::try_from(limit.as_micros()).unwrap_or(u64::MAX));
        }
        Exceeded::Steps(limit) => {
            put_u8(out, 1);
            put_u64(out, limit);
        }
        Exceeded::LoopIterations(limit) => {
            put_u8(out, 2);
            put_u64(out, limit);
        }
        Exceeded::Recursion(limit) => {
            put_u8(out, 3);
            put_u32(out, limit);
        }
        Exceeded::Stack(limit) => {
            put_u8(out, 4);
            put_u32(out, limit);
        }
        Exceeded::Nesting(limit) => {
            put_u8(out, 7);
            put_u32(out, limit);
        }
        Exceeded::Depth { estimated, ceiling } => {
            put_u8(out, 8);
            put_u64(out, estimated);
            put_u64(out, ceiling);
        }
        Exceeded::Elements { asked, ceiling } => {
            put_u8(out, 5);
            put_u64(out, asked);
            put_u64(out, ceiling);
        }
        Exceeded::StringUnits { asked, ceiling } => {
            put_u8(out, 6);
            put_u64(out, asked);
            put_u64(out, ceiling);
        }
    }
}

/// Writes one byte.
fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

/// Writes a `u32`.
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Writes a `u64`.
fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Writes an `f64`.
fn put_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Writes a boolean.
fn put_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

/// Writes a count, which no message this crate builds takes past `u32`.
fn put_len(out: &mut Vec<u8>, length: usize) {
    put_u32(out, u32::try_from(length).unwrap_or(u32::MAX));
}

/// Writes a string. A string longer than `u32` bytes is not one a run produces — a script's
/// strings are bounded by [`crate::Budget::string_units`] — and is cut at the last character
/// boundary that fits rather than written with a length that lies.
fn put_str(out: &mut Vec<u8>, text: &str) {
    let mut end = text.len().min(u32::MAX as usize);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let kept = text.get(..end).unwrap_or_default();
    put_len(out, kept.len());
    out.extend_from_slice(kept.as_bytes());
}

/// Writes a string that may be absent.
fn put_optional(out: &mut Vec<u8>, text: Option<&str>) {
    match text {
        None => put_u8(out, 0),
        Some(text) => {
            put_u8(out, 1);
            put_str(out, text);
        }
    }
}

/// Reads an encoding from the front.
struct Reader<'a> {
    /// What has not been read.
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader past the version byte, which it checks.
    fn new(bytes: &'a [u8]) -> Result<Self, WireError> {
        let mut reader = Self { rest: bytes };
        match reader.u8()? {
            VERSION => Ok(reader),
            other => Err(WireError::Version(other)),
        }
    }

    /// The next `count` bytes.
    fn take(&mut self, count: usize) -> Result<&'a [u8], WireError> {
        if self.rest.len() < count {
            return Err(WireError::Truncated);
        }
        let (taken, rest) = self.rest.split_at(count);
        self.rest = rest;
        Ok(taken)
    }

    /// The next `N` bytes, as an array.
    fn array<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let mut array = [0_u8; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    /// One byte.
    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.array::<1>()?[0])
    }

    /// A `u32`.
    fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// A `u64`.
    fn u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// An `f64`.
    fn f64(&mut self) -> Result<f64, WireError> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    /// A count of items, no more than [`MAX_ITEMS`].
    fn count(&mut self) -> Result<u32, WireError> {
        let count = self.u32()?;
        if count > MAX_ITEMS {
            return Err(WireError::Invalid("count"));
        }
        Ok(count)
    }

    /// An item of `list` named by a tag byte.
    fn tagged<T: Copy>(&mut self, list: &[T], what: &'static str) -> Result<T, WireError> {
        list.get(usize::from(self.u8()?))
            .copied()
            .ok_or(WireError::Invalid(what))
    }

    /// Where a script runs.
    fn site(&mut self) -> Result<ScriptSite, WireError> {
        Ok(match self.u8()? {
            0 => ScriptSite::Field(self.tagged(&FIELD_TRIGGERS, "trigger")?),
            1 => ScriptSite::Annotation(self.tagged(&ANNOTATION_TRIGGERS, "trigger")?),
            2 => ScriptSite::Page(if self.boolean()? {
                PageTrigger::Close
            } else {
                PageTrigger::Open
            }),
            3 => ScriptSite::OpenAction,
            4 => ScriptSite::Library,
            5 => ScriptSite::Document(self.tagged(&DOCUMENT_TRIGGERS, "trigger")?),
            6 => ScriptSite::Timer,
            _ => return Err(WireError::Invalid("site")),
        })
    }

    /// A number that may be absent.
    fn optional_u32(&mut self) -> Result<Option<u32>, WireError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u32()?)),
            _ => Err(WireError::Invalid("optional number")),
        }
    }

    /// A colour.
    fn colour(&mut self) -> Result<Colour, WireError> {
        let count = self.u8()?;
        let mut components = [0.0_f64; 4];
        for slot in components.iter_mut().take(usize::from(count)) {
            *slot = self.f64()?;
        }
        Ok(match count {
            0 => Colour::Transparent,
            1 => Colour::Gray(components[0]),
            3 => Colour::Rgb([components[0], components[1], components[2]]),
            4 => Colour::Cmyk(components),
            _ => return Err(WireError::Invalid("colour")),
        })
    }

    /// A colour that may be absent.
    fn optional_colour(&mut self) -> Result<Option<Colour>, WireError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.colour()?)),
            _ => Err(WireError::Invalid("optional colour")),
        }
    }

    /// One field's state.
    fn field(&mut self) -> Result<FieldState, WireError> {
        let name = self.string()?;
        let kind = self.tagged(&FIELD_TYPES, "field type")?;
        let value = self.string()?;
        let flags = self.u32()?;
        let char_limit = self.optional_u32()?;
        let page = self.optional_u32()?;
        let count = self.count()?;
        let mut widgets = Vec::new();
        for _ in 0..count {
            widgets.push(self.widget()?);
        }
        Ok(FieldState {
            name,
            kind,
            value,
            flags,
            char_limit,
            page,
            widgets,
        })
    }

    /// One widget's state.
    fn widget(&mut self) -> Result<WidgetState, WireError> {
        Ok(WidgetState {
            display: self.tagged(&DISPLAYS, "display")?,
            text_color: self.optional_colour()?,
            fill_color: self.optional_colour()?,
            stroke_color: self.optional_colour()?,
            border_style: self.tagged(&BORDER_STYLES, "border style")?,
            alignment: self.tagged(&ALIGNMENTS, "alignment")?,
            rect: [self.f64()?, self.f64()?, self.f64()?, self.f64()?],
            captions: [self.string()?, self.string()?, self.string()?],
            on_state: self.optional()?,
        })
    }

    /// The document as a whole.
    fn document(&mut self) -> Result<DocumentState, WireError> {
        let count = self.count()?;
        let mut info = Vec::new();
        for _ in 0..count {
            info.push(InfoEntry {
                key: self.string()?,
                text: self.string()?,
                moment: match self.u8()? {
                    0 => None,
                    1 => Some(i64::from_le_bytes(self.array()?)),
                    _ => return Err(WireError::Invalid("moment")),
                },
            });
        }
        let count = self.count()?;
        let mut layers = Vec::new();
        for _ in 0..count {
            layers.push(Layer {
                number: self.u32()?,
                generation: u16::from_le_bytes(self.array()?),
                name: self.string()?,
                on: self.boolean()?,
                initially_on: self.boolean()?,
                locked: self.boolean()?,
            });
        }
        let count = self.count()?;
        let mut annotations = Vec::new();
        for _ in 0..count {
            annotations.push(self.annotation()?);
        }
        Ok(DocumentState {
            info,
            layers,
            annotations,
        })
    }

    /// One annotation as a realm holds it.
    fn annotation(&mut self) -> Result<AnnotationState, WireError> {
        Ok(AnnotationState {
            number: self.u32()?,
            generation: u16::from_le_bytes(self.array()?),
            page: self.u32()?,
            kind: self.string()?,
            rect: [self.f64()?, self.f64()?, self.f64()?, self.f64()?],
            name: self.optional()?,
            contents: self.string()?,
            author: self.optional()?,
            modified: match self.u8()? {
                0 => None,
                1 => Some(i64::from_le_bytes(self.array()?)),
                _ => return Err(WireError::Invalid("moment")),
            },
            hidden: self.boolean()?,
            read_only: self.boolean()?,
            reach: AnnotationReach {
                printed: self.boolean()?,
                viewed: self.boolean()?,
                interactive: self.boolean()?,
            },
            popup_open: match self.u8()? {
                0 => None,
                1 => Some(false),
                2 => Some(true),
                _ => return Err(WireError::Invalid("popup state")),
            },
        })
    }

    /// One edit.
    fn edit(&mut self) -> Result<ScriptEdit, WireError> {
        Ok(match self.u8()? {
            0 => ScriptEdit::Value {
                field: self.string()?,
                value: self.string()?,
            },
            1 => {
                let field = self.string()?;
                let widget = self.optional_u32()?;
                let property = match self.u8()? {
                    0 => Property::Display(self.tagged(&DISPLAYS, "display")?),
                    1 => Property::ReadOnly(self.boolean()?),
                    2 => Property::Required(self.boolean()?),
                    3 => Property::TextColor(self.colour()?),
                    4 => Property::FillColor(self.colour()?),
                    5 => Property::StrokeColor(self.colour()?),
                    6 => Property::BorderStyle(self.tagged(&BORDER_STYLES, "border style")?),
                    7 => Property::Alignment(self.tagged(&ALIGNMENTS, "alignment")?),
                    8 => Property::CharLimit(self.u32()?),
                    9 => {
                        Property::TextFlag(self.tagged(&TEXT_FLAGS, "text flag")?, self.boolean()?)
                    }
                    10 => Property::Caption(self.tagged(&FACES, "face")?, self.string()?),
                    11 => Property::Style(self.tagged(&Glyph::ALL, "style")?),
                    _ => return Err(WireError::Invalid("property")),
                };
                ScriptEdit::Property {
                    field,
                    widget,
                    property,
                }
            }
            2 => {
                let count = self.count()?;
                let mut fields = Vec::new();
                for _ in 0..count {
                    fields.push(self.string()?);
                }
                ScriptEdit::Reset { fields }
            }
            3 => ScriptEdit::Calculate,
            4 => ScriptEdit::Focus {
                field: self.string()?,
                widget: self.optional_u32()?,
            },
            5 => ScriptEdit::Layer {
                number: self.u32()?,
                generation: u16::from_le_bytes(self.array()?),
                on: self.boolean()?,
            },
            6 => ScriptEdit::GoTo { page: self.u32()? },
            7 => ScriptEdit::Timer {
                id: self.u32()?,
                script: self.string()?,
                period: self.u32()?,
                repeat: self.boolean()?,
            },
            8 => ScriptEdit::ClearTimer { id: self.u32()? },
            9 => ScriptEdit::Beep {
                sound: self.tagged(&Sound::ALL, "sound")?,
            },
            10 => ScriptEdit::Annotation {
                number: self.u32()?,
                generation: u16::from_le_bytes(self.array()?),
                change: match self.u8()? {
                    0 => AnnotationChange::Hidden(self.boolean()?),
                    1 => AnnotationChange::PopupOpen(self.boolean()?),
                    2 => AnnotationChange::Contents(self.string()?),
                    _ => return Err(WireError::Invalid("annotation change")),
                },
            },
            _ => return Err(WireError::Invalid("edit")),
        })
    }

    /// A boolean.
    fn boolean(&mut self) -> Result<bool, WireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError::Invalid("boolean")),
        }
    }

    /// A string.
    fn string(&mut self) -> Result<String, WireError> {
        let length = usize::try_from(self.u32()?).map_err(|_| WireError::Truncated)?;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| WireError::Utf8)
    }

    /// A string that may be absent.
    fn optional(&mut self) -> Result<Option<String>, WireError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.string()?)),
            _ => Err(WireError::Invalid("optional string")),
        }
    }

    /// Which budget was exceeded.
    fn exceeded(&mut self) -> Result<Exceeded, WireError> {
        Ok(match self.u8()? {
            0 => Exceeded::Wall(Duration::from_micros(self.u64()?)),
            1 => Exceeded::Steps(self.u64()?),
            2 => Exceeded::LoopIterations(self.u64()?),
            3 => Exceeded::Recursion(self.u32()?),
            4 => Exceeded::Stack(self.u32()?),
            7 => Exceeded::Nesting(self.u32()?),
            8 => Exceeded::Depth {
                estimated: self.u64()?,
                ceiling: self.u64()?,
            },
            5 => Exceeded::Elements {
                asked: self.u64()?,
                ceiling: self.u64()?,
            },
            6 => Exceeded::StringUnits {
                asked: self.u64()?,
                ceiling: self.u64()?,
            },
            _ => return Err(WireError::Invalid("budget")),
        })
    }

    /// Succeeds where nothing remains.
    fn finish(self) -> Result<(), WireError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(WireError::Trailing(self.rest.len()))
        }
    }
}
