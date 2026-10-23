//! ISO 32000-2 section 12.7's form population.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::copy_out;
use crate::form::Form;
use crate::kinds::TextKind;
use crate::session::Session;
use crate::shapes::Quads;
use crate::status::Status;
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// §12.7's form: the sixth chrome population, and the last one to reach this ABI.
// ---------------------------------------------------------------------------------------------

/// §12.7's fields with a widget on the page being shown, as an owning handle.
///
/// Release it with [`quorra_fields_free`]. **Not at pointer speed**: this walks §12.7.4.1's field
/// tree, so a caller asks it when a page appears and after an edit, exactly as `viewer-gtk` and
/// `viewer-qt` do. A click asks [`quorra_field_at`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_fields_read(
    viewer: *const Session,
    fields: *mut *mut Form,
) -> c_int {
    let (Some(viewer), Some(fields)) = (viewer.as_ref(), fields.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().fields() {
        Ok(read) => {
            *fields = Box::into_raw(Box::new(read));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases a form. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_fields_free(fields: *mut Form) {
    if !fields.is_null() {
        drop(Box::from_raw(fields));
    }
}

/// How many fields have a widget on the page. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_fields_len(fields: *const Form) -> usize {
    fields.as_ref().map_or(0, Form::len)
}

/// One of the three names a field carries — `QUORRA_TEXT_QUALIFIED`, `_SHOWN` or `_PARTIAL`.
///
/// §14.9.3 is why there is more than one: the shown name "shall be used in place of the actual
/// field name when an interactive PDF processor identifies the field in a user-interface", while
/// the qualified one is what [`quorra_set_field_text`](super::quorra_set_field_text) addresses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_name(
    fields: *const Form,
    field: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = TextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match fields.name(field, which) {
        Ok(name) => copy_out(name, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Which control the field is — one of `QUORRA_CONTROL_*` — and every flag that decides how to
/// build it.
///
/// The flags are `QUORRA_FIELD_*`, one bit per boolean Tables 227, 229, 231 and 233 state. A bit
/// this build does not define is zero, and a bit added later is one an old caller does not read.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_control(
    fields: *const Form,
    field: usize,
    kind: *mut u32,
    flags: *mut u32,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    match fields.control(field) {
        Ok((found, bits)) => {
            if let Some(kind) = kind.as_mut() {
                *kind = found.code();
            }
            if let Some(flags) = flags.as_mut() {
                *flags = bits;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Table 232's `/MaxLen` and Table 231 bit 25's cell count, zero where the field states none.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_limits(
    fields: *const Form,
    field: usize,
    max_len: *mut u32,
    comb_cells: *mut u32,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    match fields.limits(field) {
        Ok((length, cells)) => {
            if let Some(max_len) = max_len.as_mut() {
                *max_len = length;
            }
            if let Some(comb_cells) = comb_cells.as_mut() {
                *comb_cells = cells;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// What the field says now, as §12.7.4.3 would lay it out.
///
/// [`Status::NoAnswer`] for a field with no text value at all — a button selects an appearance and
/// a signature holds a dictionary — which is a **different answer** from the empty string. A
/// caller deciding where to send the keyboard needs exactly that distinction.
///
/// A password field answers Table 231 bit 14's bullets and sets `QUORRA_FIELD_OBSCURED`. A caller
/// obeying the read-back rule must consult that bit: writing the bullets back would send them as
/// the next value, which is the bug ADR 0247 found in this project's own first host.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_value(
    fields: *const Form,
    field: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    match fields.value(field) {
        Ok(value) => copy_out(value, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// What the field displays when nobody is editing it: its value through Table 199's `/F` — `$12.50`
/// where [`quorra_field_value`] answers `12.5`.
///
/// A control that holds the keyboard shows [`quorra_field_value`], so that typing starts from the
/// characters the format was made of; one that does not shows this, which is what the page shows
/// under it everywhere else (ADR 1604). [`Status::NoAnswer`] exactly where [`quorra_field_value`]
/// answers it, and a password field answers with the same echo.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_displayed(
    fields: *const Form,
    field: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    match fields.displayed(field) {
        Ok(value) => copy_out(value, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// How many of Table 234's `/Opt` entries the field states.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_option_count(
    fields: *const Form,
    field: usize,
    count: *mut usize,
) -> c_int {
    let (Some(fields), Some(count)) = (fields.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match fields.option_count(field) {
        Ok(found) => {
            *count = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One option's label (`QUORRA_TEXT_LABEL`) or export value (`QUORRA_TEXT_EXPORT`).
///
/// In the array's own order, which Table 233 bit 20 requires: "PDF readers shall display the
/// options in the order in which they occur in the Opt array."
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_option(
    fields: *const Form,
    field: usize,
    option: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = TextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match fields.option(field, option, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Whether §12.7.5.4's value selects the option.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_option_selected(
    fields: *const Form,
    field: usize,
    option: usize,
    selected: *mut bool,
) -> c_int {
    let (Some(fields), Some(selected)) = (fields.as_ref(), selected.as_mut()) else {
        return Status::NullArgument.code();
    };
    match fields.option_selected(field, option) {
        Ok(found) => {
            *selected = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many widgets of the field are on the page being shown.
///
/// More than one is §12.7.4.1's ordinary case rather than an oddity: "a field's value" belongs to
/// the field, so typing into one widget changes all of them, and §12.7.5.2.4's radio set *is* a
/// field with several.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_widget_count(
    fields: *const Form,
    field: usize,
    count: *mut usize,
) -> c_int {
    let (Some(fields), Some(count)) = (fields.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match fields.widget_count(field) {
        Ok(found) => {
            *count = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One widget's object, its `/Rect` on the screen and whether it is on.
///
/// `quad` is `[x0, y0, … x3, y3]` in device pixels of the viewport, y downwards — the same form
/// [`quorra_selection_quads`](super::quorra_selection_quads) and [`quorra_focused_annotation`](super::quorra_focused_annotation) take, because a caller places a control the same way
/// it draws a highlight.
///
/// # Safety
///
/// See the module documentation. `quad` is writable for eight `float`s, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_widget(
    fields: *const Form,
    field: usize,
    widget: usize,
    number: *mut u32,
    generation: *mut u16,
    quad: *mut f32,
    on: *mut bool,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    match fields.widget(field, widget) {
        Ok(placed) => {
            if let Some(number) = number.as_mut() {
                *number = placed.object.0;
            }
            if let Some(generation) = generation.as_mut() {
                *generation = placed.object.1;
            }
            if !quad.is_null() {
                core::slice::from_raw_parts_mut(quad, placed.quad.len())
                    .copy_from_slice(&placed.quad);
            }
            if let Some(on) = on.as_mut() {
                *on = placed.on;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A widget's `/AP /N` on-state name (`QUORRA_TEXT_LABEL`) or Table 230's `/Opt` entry for it
/// (`QUORRA_TEXT_EXPORT`).
///
/// **The two say different things and the clause is why.** §12.7.5.2.3: "the names used to
/// represent the on state in the AP dictionary of each annotation may use numerical position …
/// encoded as a name object (for example: /0, /1)", so `/AP` may say `0` while `/Opt` says `Rot` —
/// and only the first selects an appearance while only the second is worth showing a person. The
/// first is what [`quorra_set_field_text`](super::quorra_set_field_text) sends to check the box.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_widget_text(
    fields: *const Form,
    field: usize,
    widget: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(fields) = fields.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = TextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match fields.widget_text(field, widget, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// What the field at a viewport point is called — `QUORRA_TEXT_QUALIFIED` or `QUORRA_TEXT_SHOWN`.
///
/// What a caller asks on a click, before it can send an edit. [`Status::NoAnswer`] where no field
/// is there.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_at(
    viewer: *const Session,
    x: f32,
    y: f32,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = TextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    let (qualified, shown) = match viewer.field_at((x, y)) {
        Ok(names) => names,
        Err(status) => return status.code(),
    };
    match which {
        TextKind::Qualified => copy_out(&qualified, out, cap, needed),
        TextKind::Shown => copy_out(&shown, out, cap, needed),
        TextKind::Partial | TextKind::Label | TextKind::Export => Status::WrongKind.code(),
    }
}

/// Where the caret sits in the field at a point, given how far into the value it is.
///
/// **Two points and not a rectangle, because a caret has no width**: how thick a text cursor is
/// drawn is a platform's convention, and a widget's `/R` or its `/DA`'s `Tm` can turn it off the
/// axes anyway. `from` is the end on the descent side of the baseline. The standard states no
/// caret at all — this is derived from where §12.7.4.3 puts the next glyph (ADR 0211), and
/// §12.5.6.11's caret *annotation* is a different object entirely.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_caret(
    viewer: *const Session,
    x: f32,
    y: f32,
    offset: usize,
    from_x: *mut f32,
    from_y: *mut f32,
    to_x: *mut f32,
    to_y: *mut f32,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.caret((x, y), offset) {
        Ok([from_horizontal, from_vertical, to_horizontal, to_vertical]) => {
            if let Some(at) = from_x.as_mut() {
                *at = from_horizontal;
            }
            if let Some(at) = from_y.as_mut() {
                *at = from_vertical;
            }
            if let Some(at) = to_x.as_mut() {
                *at = to_horizontal;
            }
            if let Some(at) = to_y.as_mut() {
                *at = to_vertical;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How far into a field's value a point inside it is, in bytes — [`quorra_caret`]'s inverse.
///
/// `x` and `y` name the field, as [`quorra_field_at`]'s do; `point_x` and `point_y` are the place to
/// measure, which is the same point on a click and a different one on every move of a drag. The
/// answer is the *nearest* boundary between two glyphs and never a refusal for a point in the wrong
/// place: a caller that has decided a press belongs to a field has to put the cursor somewhere.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_offset(
    viewer: *const Session,
    x: f32,
    y: f32,
    point_x: f32,
    point_y: f32,
    offset: *mut usize,
) -> c_int {
    let (Some(viewer), Some(offset)) = (viewer.as_ref(), offset.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.offset((x, y), (point_x, point_y)) {
        Ok(found) => {
            *offset = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The shapes covering a byte range of a field's value, one per line it touches.
///
/// A third question rather than [`quorra_caret`] twice, and §12.7.5.3's Table 231 bit 13 is what
/// settles it: a multiline field's value is broken into lines by the layout, so a caller holding
/// both ends of a selection cannot name the lines *between* them. Release the handle with
/// [`quorra_quads_free`](super::quorra_quads_free).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_field_selection(
    viewer: *const Session,
    x: f32,
    y: f32,
    from: usize,
    to: usize,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(viewer), Some(quads)) = (viewer.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.field_selection((x, y), from, to) {
        Ok(found) => {
            *quads = Box::into_raw(Box::new(found));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §12.5.6.6: the free text annotation **this session added** at a point, and what it says now.
///
/// How a caller aims a keyboard at one: the object goes back to [`quorra_set_free_text`](super::quorra_set_free_text). An
/// annotation the *file* states answers [`Status::NoAnswer`] deliberately — nothing in this
/// vocabulary can change one, and offering it would be an interface pretending to work.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_free_text_at(
    viewer: *const Session,
    x: f32,
    y: f32,
    number: *mut u32,
    generation: *mut u16,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.free_text_at((x, y)) {
        Ok(((found, made), text)) => {
            if let Some(number) = number.as_mut() {
                *number = found;
            }
            if let Some(generation) = generation.as_mut() {
                *generation = made;
            }
            copy_out(&text, out, cap, needed)
        }
        Err(status) => status.code(),
    }
}
