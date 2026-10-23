//! The panels flattened for a C caller: the outline, the layers and the embedded files.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::copy_out;
use crate::events::Events;
use crate::panels::{Outline, Panel};
use crate::session::Session;
use crate::status::Status;
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// §12.3.3's outline, flattened. A tree is the one shape a C ABI cannot hand over as itself.
// ---------------------------------------------------------------------------------------------

/// Reads §12.3.3's outline, depth first, as an owning handle.
///
/// Release it with [`quorra_outline_free`]. It is a snapshot and does not change under the caller:
/// an outline is a property of an immutable document, which is why both native hosts take one
/// when the document opens.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_outline_read(
    viewer: *const Session,
    outline: *mut *mut Outline,
) -> c_int {
    let (Some(viewer), Some(outline)) = (viewer.as_ref(), outline.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().outline() {
        Ok(read) => {
            *outline = Box::into_raw(Box::new(read));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases an outline. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_outline_free(outline: *mut Outline) {
    if !outline.is_null() {
        drop(Box::from_raw(outline));
    }
}

/// How many rows there are, counting every level. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_outline_len(outline: *const Outline) -> usize {
    outline.as_ref().map_or(0, Outline::len)
}

/// Table 151's `/Title` for one row, in the two-call idiom [`quorra_events_describe`](super::quorra_events_describe) uses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_outline_title(
    outline: *const Outline,
    row: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(outline) = outline.as_ref() else {
        return Status::NullArgument.code();
    };
    match outline.title(row) {
        Ok(title) => copy_out(title, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// How far in a row is, zero at the top level, and whether §12.3.3's `/Count` asks it to be open.
///
/// Either out-parameter may be null.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_outline_depth(
    outline: *const Outline,
    row: usize,
    depth: *mut u32,
    expanded: *mut bool,
) -> c_int {
    let Some(outline) = outline.as_ref() else {
        return Status::NullArgument.code();
    };
    match (outline.depth(row), outline.expanded(row)) {
        (Ok(found), Ok(open)) => {
            if let Some(depth) = depth.as_mut() {
                *depth = found;
            }
            if let Some(expanded) = expanded.as_mut() {
                *expanded = open;
            }
            Status::Ok.code()
        }
        (Err(status), _) | (_, Err(status)) => status.code(),
    }
}

/// §7.3.10's two numbers for a row, which [`quorra_activate`](super::quorra_activate) takes.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_outline_object(
    outline: *const Outline,
    row: usize,
    number: *mut u32,
    generation: *mut u16,
) -> c_int {
    let Some(outline) = outline.as_ref() else {
        return Status::NullArgument.code();
    };
    match outline.object(row) {
        Ok((found, made)) => {
            if let Some(number) = number.as_mut() {
                *number = found;
            }
            if let Some(generation) = generation.as_mut() {
                *generation = made;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

// ---------------------------------------------------------------------------------------------
// §8.11.4.3's layers and §7.11.4's files: the other two panels, flattened as the outline is.
// ---------------------------------------------------------------------------------------------

/// §8.11.4.3's `/Order`, depth first, as an owning handle.
///
/// Release it with [`quorra_panel_free`]. Table 99's `/Locked` comes back on
/// [`quorra_panel_action`]: "\[t]he state of a locked group cannot be changed through the user
/// interface of an interactive PDF processor", so a caller builds that row's switch insensitive
/// rather than sending [`quorra_set_group`] for it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_layers_read(
    viewer: *const Session,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(viewer), Some(panel)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().layers() {
        Ok(read) => {
            *panel = Box::into_raw(Box::new(read));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §7.11.4's embedded files, as an owning handle.
///
/// Flat, because the `/EmbeddedFiles` name tree is a mapping rather than a hierarchy. **A file
/// hung on §12.5.6.15's annotation is not here**: it has no key in any tree, and activating the
/// annotation is what extracts it (ADR 0295).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_attachments_read(
    viewer: *const Session,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(viewer), Some(panel)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().attachments() {
        Ok(read) => {
            *panel = Box::into_raw(Box::new(read));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases a panel. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_panel_free(panel: *mut Panel) {
    if !panel.is_null() {
        drop(Box::from_raw(panel));
    }
}

/// How many rows there are, counting every level. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_panel_len(panel: *const Panel) -> usize {
    panel.as_ref().map_or(0, Panel::len)
}

/// A row's label, or the second line beside it — `detail` non-zero for the second.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_panel_text(
    panel: *const Panel,
    row: usize,
    detail: c_int,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(panel) = panel.as_ref() else {
        return Status::NullArgument.code();
    };
    match panel.text(row, detail != 0) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// How far in a row is, zero at the top level, and whether the document asked it to start open.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_panel_depth(
    panel: *const Panel,
    row: usize,
    depth: *mut u32,
    expanded: *mut bool,
) -> c_int {
    let Some(panel) = panel.as_ref() else {
        return Status::NullArgument.code();
    };
    match panel.depth(row) {
        Ok((found, open)) => {
            if let Some(depth) = depth.as_mut() {
                *depth = found;
            }
            if let Some(expanded) = expanded.as_mut() {
                *expanded = open;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Which of `QUORRA_ROW_*` acting on the row is, and everything the action carries.
///
/// `number` and `generation` are §7.3.10's two numbers for `QUORRA_ROW_ACTIVATE` and
/// `QUORRA_ROW_TOGGLE`, and zero otherwise — §7.5.4 reserves object number zero for the head of the
/// free list, so it is never an object a document states. `on` and `locked` mean something for
/// `QUORRA_ROW_TOGGLE` alone; the `/EmbeddedFiles` key of a `QUORRA_ROW_EXTRACT` row is
/// [`quorra_panel_name`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_panel_action(
    panel: *const Panel,
    row: usize,
    kind: *mut u32,
    number: *mut u32,
    generation: *mut u16,
    on: *mut bool,
    locked: *mut bool,
) -> c_int {
    let Some(panel) = panel.as_ref() else {
        return Status::NullArgument.code();
    };
    match panel.action(row) {
        Ok((found, (object, made), is_on, is_locked)) => {
            if let Some(kind) = kind.as_mut() {
                *kind = found.code();
            }
            if let Some(number) = number.as_mut() {
                *number = object;
            }
            if let Some(generation) = generation.as_mut() {
                *generation = made;
            }
            if let Some(on) = on.as_mut() {
                *on = is_on;
            }
            if let Some(locked) = locked.as_mut() {
                *locked = is_locked;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The `/EmbeddedFiles` key [`quorra_extract`](super::quorra_extract) takes for a `QUORRA_ROW_EXTRACT` row, `""` for others.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_panel_name(
    panel: *const Panel,
    row: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(panel) = panel.as_ref() else {
        return Status::NullArgument.code();
    };
    match panel.name(row) {
        Ok(name) => copy_out(name, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// §8.11: switches an optional content group on or off.
///
/// The group is named by object because that is what §8.11.2.2's `/OCGs` and Table 99's `/Order`
/// hold; [`quorra_layers_read`] is where a caller gets the identities.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_set_group(
    viewer: *mut Session,
    number: u32,
    generation: u16,
    on: bool,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.set_group(number, generation, on)));
    Status::Ok.code()
}
