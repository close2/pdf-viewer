//! The viewer's lifetime and the commands that move it: open, close, resize, zoom, scroll, find, activate.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::owned_text;
use crate::events::Events;
use crate::kinds::{PageTargetKind, ZoomKind};
use crate::session::Drawn;
use crate::session::Session;
use crate::status::Status;
use core::ffi::{c_char, c_int};
use viewer_core::RenderRequest;

// ---------------------------------------------------------------------------------------------
// The viewer.
// ---------------------------------------------------------------------------------------------

/// A viewer for a viewport of this size, or null if it could not be made.
///
/// `scale` is device pixels per logical pixel: 1.0 on an ordinary display, 2.0 on a doubled one.
/// Release it with [`quorra_viewer_free`].
#[unsafe(no_mangle)]
pub extern "C" fn quorra_viewer_new(width: u32, height: u32, scale: f32) -> *mut Session {
    Box::into_raw(Box::new(Session::new(width, height, scale)))
}

/// Releases a viewer and everything it holds.
///
/// A null pointer is ignored, which is what `free(NULL)` does and what a caller freeing in an
/// error path expects.
///
/// # Safety
///
/// See the module documentation. `viewer` came from [`quorra_viewer_new`] and is not used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_viewer_free(viewer: *mut Session) {
    if !viewer.is_null() {
        drop(Box::from_raw(viewer));
    }
}

// ---------------------------------------------------------------------------------------------
// Commands. One function each, because a union's size is part of an ABI and a symbol is not.
// ---------------------------------------------------------------------------------------------

/// §7.6.4.1 and Annex O: opens a document from bytes the caller owns.
///
/// The bytes are copied, so the caller may free them as soon as this returns. `password` and
/// `fragment` may be null, which is a caller with neither.
///
/// # Safety
///
/// See the module documentation. `bytes` is readable for `len`; `password` and `fragment` are
/// null or NUL-terminated UTF-8; `events` receives an owning handle to be freed with
/// [`quorra_events_free`](super::quorra_events_free).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_open(
    viewer: *mut Session,
    document: u64,
    bytes: *const u8,
    len: usize,
    password: *const c_char,
    fragment: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    if bytes.is_null() {
        return Status::NullArgument.code();
    }
    let (Ok(password), Ok(fragment)) = (owned_text(password), owned_text(fragment)) else {
        return Status::NotUtf8.code();
    };
    let file = core::slice::from_raw_parts(bytes, len).to_vec();
    let password = password.map(viewer_core::Secret::from);
    *events = Box::into_raw(Box::new(viewer.open(document, file, password, fragment)));
    Status::Ok.code()
}

/// Closes a document and forgets everything derived from it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_close(
    viewer: *mut Session,
    document: u64,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.close(document)));
    Status::Ok.code()
}

/// Makes an already-open document the one commands apply to.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_focus(
    viewer: *mut Session,
    document: u64,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.focus(document)));
    Status::Ok.code()
}

/// The viewport changed size, or moved to a display with a different scale.
///
/// Width and height are **device** pixels: a page is rasterised at the resolution it will be
/// shown at, because rendering at the logical size and letting a compositor scale it up is the
/// blur this project exists to avoid.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_resize(
    viewer: *mut Session,
    width: u32,
    height: u32,
    scale: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.resize(width, height, scale)));
    Status::Ok.code()
}

/// Shows another page.
///
/// `target` is one of `QUORRA_PAGE_*`; `argument` is the index for `QUORRA_PAGE_INDEX`, the signed
/// number of pages for `QUORRA_PAGE_RELATIVE`, and ignored for the other four.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_go_to_page(
    viewer: *mut Session,
    target: u32,
    argument: i64,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = PageTargetKind::from_code(target) else {
        return Status::WrongKind.code();
    };
    let Some(target) = kind.target(argument) else {
        return Status::OutOfRange64.code();
    };
    *events = Box::into_raw(Box::new(viewer.go_to(target)));
    Status::Ok.code()
}

/// Changes the magnification, holding the viewport's centre still.
///
/// `zoom` is one of `QUORRA_ZOOM_*`; `scale` is logical pixels per user space unit for
/// `QUORRA_ZOOM_SCALE`, where 1.0 is 72 dpi, and is ignored for the other five.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_zoom(
    viewer: *mut Session,
    zoom: u32,
    scale: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = ZoomKind::from_code(zoom) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.zoom(kind.zoom(scale))));
    Status::Ok.code()
}

/// Moves the page under the viewport by a device-pixel delta.
///
/// Positive `dy` moves the content up, which is what a wheel scrolling down does.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_scroll(
    viewer: *mut Session,
    dx: f32,
    dy: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.scroll(dx, dy)));
    Status::Ok.code()
}

/// Annex O's `search`: begins a document-wide search and takes its first step.
///
/// `needle` is NUL-terminated UTF-8. `backward` non-zero searches up the document. The search
/// starts from what is selected, so calling this again with the same string is *next*.
///
/// **A step reads one page**, and the caller pumps [`quorra_find_continue`] until
/// [`quorra_event_searched`](super::quorra_event_searched) reports `remaining` of zero. That is not a courtesy: a sweep of ISO
/// 32000-2's own 1023 pages is 5.84 s, and this ABI does not block a caller's event loop for it.
///
/// # Safety
///
/// See the module documentation. `needle` must point at a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_find_start(
    viewer: *mut Session,
    needle: *const c_char,
    backward: c_int,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false) = (viewer.as_mut(), events.as_mut(), needle.is_null())
    else {
        return Status::NullArgument.code();
    };
    let Ok(Some(needle)) = owned_text(needle) else {
        return Status::NotUtf8.code();
    };
    *events = Box::into_raw(Box::new(viewer.find_start(needle, backward != 0)));
    Status::Ok.code()
}

/// Reads one more page of the search in progress.
///
/// Nothing at all when there is none, which is the right answer for a caller that pumped once too
/// often.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_find_continue(
    viewer: *mut Session,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.find_continue()));
    Status::Ok.code()
}

/// Forgets the search in progress. What closing a find bar sends.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_find_stop(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.find_stop()));
    Status::Ok.code()
}

/// §12.3.3: activates an object the caller is showing outside the page — an outline row.
///
/// The two numbers are §7.3.10's indirect reference, which `quorra_outline_object` answered with.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_activate(
    viewer: *mut Session,
    number: u32,
    generation: u16,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.activate(number, generation)));
    Status::Ok.code()
}

/// Hands back the pixels a request asked for.
///
/// **Both handles are consumed** and must not be used again: the request carries the token the
/// viewer matches, and the raster's pixels move into the viewer. A stale token is dropped, which
/// is what stops a page turned mid-render from being overwritten by the previous page's frame.
///
/// # Safety
///
/// See the module documentation. `request` came from [`quorra_event_render_request`](super::quorra_event_render_request) and `raster`
/// from [`quorra_render_request_rasterise`](super::quorra_render_request_rasterise); neither is used or freed after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_ready_raster(
    viewer: *mut Session,
    request: *mut RenderRequest,
    raster: *mut Drawn,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    if request.is_null() || raster.is_null() {
        return Status::NullArgument.code();
    }
    let request = Box::from_raw(request);
    let raster = Box::from_raw(raster);
    *events = Box::into_raw(Box::new(viewer.render_ready_raster(&request, *raster)));
    Status::Ok.code()
}

/// Says that a request could not be drawn, and why.
///
/// The request is consumed. `why` may be null, which is a caller with nothing to add beyond the
/// status it got. Reported rather than swallowed: a viewer that silently shows the previous page
/// when a render fails is telling a person something false about the document.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_ready_failed(
    viewer: *mut Session,
    request: *mut RenderRequest,
    why: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    if request.is_null() {
        return Status::NullArgument.code();
    }
    let Ok(why) = owned_text(why) else {
        return Status::NotUtf8.code();
    };
    let request = Box::from_raw(request);
    let said = why.unwrap_or_else(|| "the host's rasteriser refused the request".to_owned());
    *events = Box::into_raw(Box::new(viewer.render_ready_failed(&request, said)));
    Status::Ok.code()
}
