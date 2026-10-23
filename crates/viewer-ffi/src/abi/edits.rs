//! The four edits and the log they go into, and the bytes written out.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::owned_text;
use crate::events::Events;
use crate::kinds::{AttachKind, MarkupKind, PurposeKind};
use crate::session::Session;
use crate::status::Status;
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// The four edits, and the log they go into.
//
// `viewer_core::Edit` is one enum with four variants and its `SetField` value is a third enum;
// here it is six functions, for the reason a command is a function at all — a union's size is part
// of an ABI. It also means the shape change of ADR 0248, which broke every Rust consumer's build,
// would have cost a C caller one new symbol and left the others answering what they always did.
// ---------------------------------------------------------------------------------------------

/// §12.7.4: puts characters into a field, by the name §12.7.4.2 gives it.
///
/// A name rather than a widget, because §12.7.4.1 lets one field own several widgets and a field's
/// value is the field's: typing into one of them changes all of them. This is also how
/// §12.7.5.2's two toggling buttons are checked — their value is the appearance-state name
/// [`quorra_field_widget_text`](super::quorra_field_widget_text) hands over.
///
/// # Safety
///
/// See the module documentation. `field` and `text` are NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_set_field_text(
    viewer: *mut Session,
    field: *const c_char,
    text: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false, false) = (
        viewer.as_mut(),
        events.as_mut(),
        field.is_null(),
        text.is_null(),
    ) else {
        return Status::NullArgument.code();
    };
    let (Ok(Some(field)), Ok(Some(text))) = (owned_text(field), owned_text(text)) else {
        return Status::NotUtf8.code();
    };
    *events = Box::into_raw(Box::new(
        viewer.set_field(field, pdf_model::view::Entered::Text(text)),
    ));
    Status::Ok.code()
}

/// §12.7.5.4: chooses which of Table 234's options are selected, by index into `/Opt`.
///
/// **The variant ADR 0248 changed, arriving in C as its own symbol.** Table 233 bit 22 lets a list
/// box hold several items at once and one string could not say which; `options` is an array of
/// `count` indices, and an empty one selects nothing — which §12.7.5.4 makes the same state as a
/// clear: "\[t]he default value of V is null, indicating that no item is currently selected."
///
/// An index past the end of `/Opt` names nothing and is dropped, and more than one index on a field
/// whose `MultiSelect` flag is clear is cut to the first, both by `viewer-core`.
///
/// # Safety
///
/// See the module documentation. `options` is readable for `count` elements, or null when `count`
/// is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_set_field_options(
    viewer: *mut Session,
    field: *const c_char,
    options: *const usize,
    count: usize,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false) = (viewer.as_mut(), events.as_mut(), field.is_null())
    else {
        return Status::NullArgument.code();
    };
    let Ok(Some(field)) = owned_text(field) else {
        return Status::NotUtf8.code();
    };
    let chosen = if count == 0 {
        Vec::new()
    } else if options.is_null() {
        return Status::NullArgument.code();
    } else {
        core::slice::from_raw_parts(options, count).to_vec()
    };
    *events = Box::into_raw(Box::new(
        viewer.set_field(field, pdf_model::view::Entered::Chosen(chosen)),
    ));
    Status::Ok.code()
}

/// Empties a field: §12.7.6.3's "its V entry shall be removed".
///
/// A different state from never having touched the field, which shows Table 226's `/V`.
///
/// # Safety
///
/// See the module documentation. `field` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_clear_field(
    viewer: *mut Session,
    field: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false) = (viewer.as_mut(), events.as_mut(), field.is_null())
    else {
        return Status::NullArgument.code();
    };
    let Ok(Some(field)) = owned_text(field) else {
        return Status::NotUtf8.code();
    };
    *events = Box::into_raw(Box::new(
        viewer.set_field(field, pdf_model::view::Entered::Cleared),
    ));
    Status::Ok.code()
}

/// §12.5.6.10: marks up **what is selected**, in one of the clause's four ways.
///
/// The selection is the target and is resolved when the command arrives: the clause defines these
/// four over *text* — "text markup annotations shall appear as highlights, underlines, strikeouts
/// … in the text of a document" — so nothing happens where nothing is selected. The colour is
/// Table 166's `/C` in `DeviceRGB`, components in 0..=1.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_markup(
    viewer: *mut Session,
    kind: u32,
    red: f32,
    green: f32,
    blue: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = MarkupKind::from_code(kind) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.markup(kind.markup(), [red, green, blue])));
    Status::Ok.code()
}

/// §12.5.6.6: puts an empty free text annotation over a rectangle a person **drew**.
///
/// A drag rather than a selection, which is the whole difference from [`quorra_markup`] and follows
/// from the clause: that subtype "displays text directly on the page", so there is nothing on the
/// page for it to be over. The two corners are in device pixels of the viewport, in either order;
/// a rectangle with no area is a press that never moved and adds nothing.
///
/// [`quorra_free_text_at`](super::quorra_free_text_at) is how the caller learns which annotation the drag made.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_free_text(
    viewer: *mut Session,
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    red: f32,
    green: f32,
    blue: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.free_text(
        (from_x, from_y),
        (to_x, to_y),
        [red, green, blue],
    )));
    Status::Ok.code()
}

/// §12.5.6.6: says what a free text annotation this session added says — Table 166's `/Contents`.
///
/// # Safety
///
/// See the module documentation. `text` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_set_free_text(
    viewer: *mut Session,
    number: u32,
    generation: u16,
    text: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false) = (viewer.as_mut(), events.as_mut(), text.is_null())
    else {
        return Status::NullArgument.code();
    };
    let Ok(Some(text)) = owned_text(text) else {
        return Status::NotUtf8.code();
    };
    *events = Box::into_raw(Box::new(viewer.set_free_text(number, generation, text)));
    Status::Ok.code()
}

/// §12.5.6.4: says what a text note says, as a person typing into its popup window does — Table
/// 166's `/Contents`, the icon appearance kept (ADR 1726). The note is the one
/// [`quorra_popup_note`](super::quorra_popup_note) names; any other annotation is refused and said among the events.
///
/// # Safety
///
/// See the module documentation. `text` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_set_note_text(
    viewer: *mut Session,
    number: u32,
    generation: u16,
    text: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false) = (viewer.as_mut(), events.as_mut(), text.is_null())
    else {
        return Status::NullArgument.code();
    };
    let Ok(Some(text)) = owned_text(text) else {
        return Status::NotUtf8.code();
    };
    *events = Box::into_raw(Box::new(viewer.set_note_text(number, generation, text)));
    Status::Ok.code()
}

/// Undoes the last edit.
///
/// The surviving prefix of the log is *replayed* rather than inverted, which is why an edit can be
/// undone without the viewer having remembered what it replaced.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_undo(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.undo()));
    Status::Ok.code()
}

/// Redoes the last undone edit. A new edit after an undo discards what was undone.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_redo(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.redo()));
    Status::Ok.code()
}

/// Whether anything has been edited since the document opened.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_dirty(viewer: *const Session, dirty: *mut bool) -> c_int {
    let (Some(viewer), Some(dirty)) = (viewer.as_ref(), dirty.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.dirty() {
        Ok(found) => {
            *dirty = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many milliseconds of `quorra_tick` may pass before a script's timer is owed: `held` false
/// where no document holds one (ADR 1702).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_timer_due(
    viewer: *const Session,
    held: *mut bool,
    millis: *mut u32,
) -> c_int {
    let (Some(viewer), Some(held), Some(millis)) =
        (viewer.as_ref(), held.as_mut(), millis.as_mut())
    else {
        return Status::NullArgument.code();
    };
    let due = viewer.timer_due();
    *held = due.is_some();
    *millis = due.unwrap_or(0);
    Status::Ok.code()
}

// ---------------------------------------------------------------------------------------------
// Bytes out: §7.5.6's incremental update and §7.11.4's embedded files.
// ---------------------------------------------------------------------------------------------

/// Writes §7.5.6's incremental update for everything the log holds.
///
/// The bytes arrive on a `QUORRA_EVENT_SAVED` and are read with [`quorra_event_bytes`](super::quorra_event_bytes); the *caller*
/// writes them somewhere, because this library has no filesystem and where a file lands is a
/// policy rather than a rendering decision. The whole file comes back rather than the update
/// alone, because §7.5.6's update is only meaningful after the bytes it chains to.
///
/// A field carrying Table 231 bit 14 has neither its value nor its appearance written, and the
/// withholding is *reported* rather than silent — that table's NOTE makes it "imperative that PDF
/// processors never store the value of the text field in the PDF file if this flag is set".
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_save(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.save()));
    Status::Ok.code()
}

/// Asks the focused document what it says about *itself*, and is told in words.
///
/// The sentences arrive on a `QUORRA_EVENT_REPORTED` with no page on it — §12.11's requirements,
/// §12.8's signatures, §7.11.4's embedded files, §14.13.2's associated files that are not
/// embedded, §7.5's rebuilt cross-reference table, Annex I's version, §14.8.6.2's namespaces and
/// §14.8.6.3's unenclosed `MathML`.
///
/// **Call it once the reader has their page.** §12.8.1's byte range digest is read and hashed
/// over the signed part of the file, so this is exactly the work `CLAUDE.md` principle 2 keeps
/// off a launch; a second call answers the same sentences and does the work no second time.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_document_report(
    viewer: *mut Session,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.report()));
    Status::Ok.code()
}

/// Says that the focused document's first frame is on the screen, so its open sequence runs —
/// RFC 0008 section 6.5 step 1: Table 32's `/JavaScript` name tree, the `/OpenAction` script, page
/// one's `/O` and its annotations' `/PO`.
///
/// **Call it when `quorra_document_report` is called**, once the reader has their page: *when
/// opened* is not *before the first frame*, so the sequence costs page one nothing (ADR 1602). It
/// runs once a document; what it says arrives on a `QUORRA_EVENT_REPORTED` with no page on it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_presented(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.presented()));
    Status::Ok.code()
}

/// §7.11.4: takes an embedded file's bytes out of the document.
///
/// The name is the key the `/EmbeddedFiles` tree filed the file under, which is what
/// [`quorra_panel_name`](super::quorra_panel_name) answered with. The bytes arrive on a `QUORRA_EVENT_EXTRACTED`, decoded:
/// §7.4's filters are undone here, because a caller that had to decode them would be a second
/// reader of the document.
///
/// # Safety
///
/// See the module documentation. `name` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_extract(
    viewer: *mut Session,
    name: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false) = (viewer.as_mut(), events.as_mut(), name.is_null())
    else {
        return Status::NullArgument.code();
    };
    let Ok(Some(name)) = owned_text(name) else {
        return Status::NotUtf8.code();
    };
    *events = Box::into_raw(Box::new(viewer.extract(name)));
    Status::Ok.code()
}

/// §7.11.4: puts a file into the document, in one of §7.11.4.1's two homes — `QUORRA_ATTACH_*`.
///
/// The file is what `bytes` reads for `len`; the name is what it is filed under, one namespace
/// for both homes, and a name already embedded is a `QUORRA_EVENT_REPORTED` rather than a second
/// entry. `description` and `mime` may be null. Under `QUORRA_ATTACH_PAGE`, `x` and `y` are the
/// point of the viewport the file goes under, in device pixels; under `QUORRA_ATTACH_DOCUMENT`
/// they are ignored. What comes back is the ordinary edit's events — a `QUORRA_EVENT_DIRTY`, a
/// `QUORRA_EVENT_ATTACHMENTS_CHANGED` — or the reader's policy speaking: refused, asked or warned.
/// Nothing reaches the file until `quorra_save`.
///
/// # Safety
///
/// See the module documentation. `bytes` is readable for `len`; `name`, `description` and
/// `mime` are NUL-terminated UTF-8, the last two or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_attach(
    viewer: *mut Session,
    bytes: *const u8,
    len: usize,
    name: *const c_char,
    description: *const c_char,
    mime: *const c_char,
    home: u32,
    x: f32,
    y: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false, false) = (
        viewer.as_mut(),
        events.as_mut(),
        name.is_null(),
        bytes.is_null(),
    ) else {
        return Status::NullArgument.code();
    };
    let Some(home) = AttachKind::from_code(home) else {
        return Status::WrongKind.code();
    };
    let (Ok(Some(name)), Ok(description), Ok(mime)) =
        (owned_text(name), owned_text(description), owned_text(mime))
    else {
        return Status::NotUtf8.code();
    };
    let file = core::slice::from_raw_parts(bytes, len).to_vec();
    let home = match home {
        AttachKind::Document => viewer_core::AttachHome::Document,
        AttachKind::Page => viewer_core::AttachHome::Page { at: (x, y) },
    };
    *events = Box::into_raw(Box::new(viewer.attach(file, name, description, mime, home)));
    Status::Ok.code()
}

/// §7.11.4: takes an embedded file out of the document, by the `/EmbeddedFiles` key
/// [`quorra_panel_name`](super::quorra_panel_name) answered with.
///
/// A file this session attached is forgotten; one the document's own tree names leaves the
/// list now and the tree at the next `quorra_save`, with what it alone reached marked free. A name
/// nothing files is a `QUORRA_EVENT_REPORTED`.
///
/// # Safety
///
/// See the module documentation. `name` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_detach(
    viewer: *mut Session,
    name: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false) = (viewer.as_mut(), events.as_mut(), name.is_null())
    else {
        return Status::NullArgument.code();
    };
    let Ok(Some(name)) = owned_text(name) else {
        return Status::NotUtf8.code();
    };
    *events = Box::into_raw(Box::new(viewer.detach(name)));
    Status::Ok.code()
}

/// The answer to a `QUORRA_EVENT_ASKING`: whether the edit the document restricts goes ahead.
///
/// `document` is the one the event named ([`quorra_event_document`](super::quorra_event_document)). `true` performs the held
/// edit exactly as `QUORRA_RESTRICT_OFF` would have; `false` forgets it and says nothing. An answer
/// to a document with nothing outstanding does nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_answer(
    viewer: *mut Session,
    document: u64,
    proceed: bool,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.answer(document, proceed)));
    Status::Ok.code()
}

/// The bytes a `QUORRA_EVENT_NEEDS_FILE` asked for, or a refusal.
///
/// A null `bytes` is a caller that will not or cannot supply them, which is a **legitimate answer
/// and not an error**: the policy about which files a document may name belongs to whoever owns
/// the filesystem, and that is never this library. The refusal is said out loud rather than
/// swallowed.
///
/// # Safety
///
/// See the module documentation. `bytes` is readable for `len`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_supply(
    viewer: *mut Session,
    purpose: u32,
    bytes: *const u8,
    len: usize,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(purpose) = PurposeKind::from_code(purpose) else {
        return Status::WrongKind.code();
    };
    let file = if bytes.is_null() {
        None
    } else {
        Some(core::slice::from_raw_parts(bytes, len).to_vec())
    };
    *events = Box::into_raw(Box::new(viewer.supply(purpose.purpose(), file)));
    Status::Ok.code()
}
