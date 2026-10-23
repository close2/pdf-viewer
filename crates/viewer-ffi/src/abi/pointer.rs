//! The pointer, the selection and the focus.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::copy_out;
use crate::events::Events;
use crate::kinds::{FocusKind, OrderKind, PointerKind, SelectKind};
use crate::session::Session;
use crate::shapes::Quads;
use crate::status::Status;
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// The pointer, the selection and §12.5.1's focus.
//
// Everything a person *does* to a page (ADR 0346). Each is a symbol, and a symbol added later costs
// a compiled caller nothing — which is the property the shape was chosen for.
// ---------------------------------------------------------------------------------------------

/// §12.5.5: the pointer moved, or a button went down or up, at a point in the viewport.
///
/// Device pixels from the viewport's top-left corner. `action` is one of `QUORRA_POINTER_*`; a
/// number outside them answers [`Status::WrongKind`] rather than guessing at a fifth situation the
/// clause does not describe.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_pointer(
    viewer: *mut Session,
    x: f32,
    y: f32,
    action: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = PointerKind::from_code(action) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.pointer((x, y), kind.action())));
    Status::Ok.code()
}

/// Selects everything the page reads back as, or nothing.
///
/// A drag is [`quorra_pointer`]'s business; this is what a menu item or a keystroke asks for.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_select(
    viewer: *mut Session,
    what: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = SelectKind::from_code(what) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.select(kind.selection())));
    Status::Ok.code()
}

/// §12.5.1: moves the input focus to the next or previous annotation on the page.
///
/// The *order* is the document's — Table 31's `/Tabs` — and the *key* is the caller's, because the
/// clause names a key and this library has no keyboard.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_focused(
    viewer: *mut Session,
    direction: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = FocusKind::from_code(direction) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.focused(kind.moved())));
    Status::Ok.code()
}

/// Whether activating at this viewport point would follow a §12.5.6.5 link.
///
/// What a caller needs to choose a cursor, which it does on every pointer move — so it is a
/// question rather than a command, and it produces no events.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_link_at(
    viewer: *const Session,
    x: f32,
    y: f32,
    link: *mut bool,
) -> c_int {
    let (Some(viewer), Some(link)) = (viewer.as_ref(), link.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.link_at((x, y)) {
        Ok(found) => {
            *link = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The selected text, in the two-call idiom [`quorra_events_describe`](super::quorra_events_describe) uses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_selection_text(
    viewer: *const Session,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.selection_text() {
        Ok(text) => copy_out(&text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// The text the last [`quorra_copy`](super::quorra_copy) released, in the two-call idiom, with the order it is in.
///
/// **What a caller needs in order to put a selection on its own clipboard** (ADR 0519). The three
/// windowed hosts in this tree call `gdk::Clipboard`, `QClipboard` and `arboard`; a C caller's
/// platform is its own business and this ABI has no business guessing it — so what crosses is the
/// characters and the one thing a caller cannot work out for itself, which of ISO 32000-2
/// §14.8.2.5's two content orders they are in.
///
/// **The copy's own text, read back.** ISO 32000-2 Table 22's bit 5 restricts copying, so the text
/// arrives only through `quorra_copy`, at the reader's level; this answers with what that grant
/// released — the same string `quorra_event_copied` reads off its `QUORRA_EVENT_COPIED` — and with
/// `QUORRA_NO_ANSWER` where the last copy released nothing: none was asked, it was refused, its
/// question is unanswered or was declined, or nothing was selected (ADR 2087).
///
/// `order` receives `QUORRA_ORDER_LOGICAL` or `QUORRA_ORDER_PAGE_CONTENT`, and may be null for a
/// caller that does not care. It is written only on `QUORRA_OK`.
///
/// [`quorra_selection_text`] is still there and still answers in page content order: that is the
/// order the quadrilaterals are in, so it is the right answer for anything being *drawn*.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null; `order` is writable
/// for one `uint32_t`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_selection_copy_text(
    viewer: *const Session,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
    order: *mut u32,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.copy_text() {
        Ok((text, content_order)) => {
            let status = copy_out(&text, out, cap, needed);
            // Written only where the text actually arrived: `copy_out` answers
            // `QUORRA_BUFFER_TOO_SMALL` on the sizing call of the two-call idiom, and a caller that
            // read the order out of that call would be reading a value for a string it does not
            // have yet.
            if status == Status::Ok.code()
                && let Some(order) = order.as_mut()
            {
                *order = OrderKind::of(content_order).code();
            }
            status
        }
        Err(status) => status.code(),
    }
}

/// The shapes covering what is selected, as an owning handle.
///
/// **Geometry rather than pixels**, which is `doc/ui-boundary.md`'s rule and the whole reason a
/// selection is not baked into the frame: a native host draws it in macOS's selection colour,
/// KDE's accent or the Windows highlight brush, and a highlight that arrived as pixels could not.
/// Release it with [`quorra_quads_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_selection_quads(
    viewer: *const Session,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(viewer), Some(quads)) = (viewer.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().selection_quads() {
        Ok(found) => {
            *quads = Box::into_raw(Box::new(found));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// ISO 32000-2 Annex O's highlighted rectangles on the page being shown, or nothing.
///
/// Table Annex O.4's `highlight` parameter of the fragment the document was opened with — "\[o]pen
/// the document with the specified rectangle highlighted … \[t]he nature of the highlighting is
/// implementation-dependent", which is why this hands over shapes and not a picture. A caller that
/// passed no fragment, or one naming no rectangle for this page, gets an empty list. Release it
/// with [`quorra_quads_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_highlight_quads(
    viewer: *const Session,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(viewer), Some(quads)) = (viewer.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().highlight_quads() {
        Ok(found) => {
            *quads = Box::into_raw(Box::new(found));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases a list of shapes. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_quads_free(quads: *mut Quads) {
    if !quads.is_null() {
        drop(Box::from_raw(quads));
    }
}

/// How many shapes there are. Zero for a null pointer, which is also the empty answer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_quads_len(quads: *const Quads) -> usize {
    quads.as_ref().map_or(0, Quads::len)
}

/// One shape's eight numbers, `[x0, y0, … x3, y3]` in device pixels of the viewport.
///
/// # Safety
///
/// See the module documentation. `into` is writable for eight `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_quads_get(
    quads: *const Quads,
    index: usize,
    into: *mut f32,
) -> c_int {
    let Some(quads) = quads.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match quads.get(index) {
        Ok(shape) => {
            core::slice::from_raw_parts_mut(into, shape.len()).copy_from_slice(&shape);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Which annotation holds §12.5.1's focus, and the quadrilateral covering it.
///
/// The *ring* is the caller's, drawn in its platform's focus colour; this is the geometry it needs.
///
/// # Safety
///
/// See the module documentation. `quad` is writable for eight `float`s, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_focused_annotation(
    viewer: *const Session,
    number: *mut u32,
    generation: *mut u16,
    quad: *mut f32,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.reads().focused_annotation() {
        Ok(((found, made), shape)) => {
            if let Some(number) = number.as_mut() {
                *number = found;
            }
            if let Some(generation) = generation.as_mut() {
                *generation = made;
            }
            if !quad.is_null() {
                core::slice::from_raw_parts_mut(quad, shape.len()).copy_from_slice(&shape);
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}
