//! Rendering: the opaque request, the pixels it becomes, the oversized-page window and the preparation read on a thread the caller chooses.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use crate::events::Events;
use crate::kinds::PixelFormat;
use crate::session;
use crate::session::Drawn;
use crate::session::Session;
use crate::status::Status;
use core::ffi::c_int;
use viewer_core::RenderRequest;

// ---------------------------------------------------------------------------------------------
// Rendering. The display list stays opaque; what crosses is "draw this" and the pixels.
// ---------------------------------------------------------------------------------------------

/// Releases a render request that will not be handed back. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_request_free(request: *mut RenderRequest) {
    if !request.is_null() {
        drop(Box::from_raw(request));
    }
}

/// Which page a request is for, zero-based, and the extent it asks for.
///
/// Enough for a caller to decide whether to draw it at all — a request for a page it has turned
/// away from is one it may hand back with [`quorra_render_ready_failed`](super::quorra_render_ready_failed) rather than spend a
/// rasterisation on. Any out-parameter may be null.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_request_page(
    request: *const RenderRequest,
    page: *mut usize,
    width: *mut u32,
    height: *mut u32,
) -> c_int {
    let Some(request) = request.as_ref() else {
        return Status::NullArgument.code();
    };
    if let Some(page) = page.as_mut() {
        *page = request.page;
    }
    if let Some(width) = width.as_mut() {
        *width = request.target.width;
    }
    if let Some(height) = height.as_mut() {
        *height = request.target.height;
    }
    Status::Ok.code()
}

/// Draws a request with the processor rasteriser.
///
/// The one thing this crate does that is not a translation, and it is here because a display list
/// is clauses 8 and 9 in a data structure rather than something to put in a header. The raster is
/// an owning handle: hand it back with [`quorra_render_ready_raster`](super::quorra_render_ready_raster), or release it with
/// [`quorra_raster_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_request_rasterise(
    request: *const RenderRequest,
    raster: *mut *mut Drawn,
) -> c_int {
    let (Some(request), Some(raster)) = (request.as_ref(), raster.as_mut()) else {
        return Status::NullArgument.code();
    };
    match session::rasterise(request) {
        Ok(drawn) => {
            *raster = Box::into_raw(Box::new(drawn));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases a raster that will not be handed back. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_raster_free(raster: *mut Drawn) {
    if !raster.is_null() {
        drop(Box::from_raw(raster));
    }
}

// ---------------------------------------------------------------------------------------------
// `QUORRA_OVERSIZE_LISTED`'s half of the round trip: the request stays the list a window is
// drawn from (ADR 2060).
// ---------------------------------------------------------------------------------------------

/// Whether a request is for a page drawn a window at a time in a viewport this size — every
/// toolkit window's rule, `viewer_host::oversize::oversized` (ADRs 1892, 1930, 1994).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_request_oversized(
    request: *const RenderRequest,
    viewport_width: u32,
    viewport_height: u32,
    oversized: *mut bool,
) -> c_int {
    let (Some(request), Some(oversized)) = (request.as_ref(), oversized.as_mut()) else {
        return Status::NullArgument.code();
    };
    *oversized = viewer_host::oversize::oversized(request, (viewport_width, viewport_height));
    Status::Ok.code()
}

/// The window of a request's target to draw for a page whose top-left corner sits at `origin` in
/// a viewport this size: what it shows, grown by the margin every toolkit window draws with.
/// [`Status::NoAnswer`] where none of the page is in the viewport.
///
/// # Safety
///
/// See the module documentation. The four out-parameters are writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_request_window(
    request: *const RenderRequest,
    origin_x: f32,
    origin_y: f32,
    viewport_width: u32,
    viewport_height: u32,
    x: *mut u32,
    y: *mut u32,
    width: *mut u32,
    height: *mut u32,
) -> c_int {
    let (Some(request), Some(x), Some(y), Some(width), Some(height)) = (
        request.as_ref(),
        x.as_mut(),
        y.as_mut(),
        width.as_mut(),
        height.as_mut(),
    ) else {
        return Status::NullArgument.code();
    };
    let Some(window) = session::window_for(
        request,
        (origin_x, origin_y),
        (viewport_width, viewport_height),
    ) else {
        return Status::NoAnswer.code();
    };
    (*x, *y, *width, *height) = (window.x, window.y, window.width, window.height);
    Status::Ok.code()
}

/// Draws one window of a request's target with the processor rasteriser. May be called on a
/// thread of the caller's own. The raster is an owning handle, read with [`quorra_raster_info`]
/// and [`quorra_raster_copy`] and released with [`quorra_raster_free`]: the viewer holds no pixels
/// for a page it was told the caller took.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_request_rasterise_window(
    request: *const RenderRequest,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    raster: *mut *mut Drawn,
) -> c_int {
    let (Some(request), Some(raster)) = (request.as_ref(), raster.as_mut()) else {
        return Status::NullArgument.code();
    };
    let window = viewer_host::oversize::Window {
        x,
        y,
        width,
        height,
    };
    match session::rasterise_window(request, window) {
        Ok(drawn) => {
            *raster = Box::into_raw(Box::new(drawn));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Tells the viewer what drawing this window of a request's page declined — the marks the draw
/// left to the library's converter — which it says once a condition for the page's reading, as
/// `QUORRA_EVENT_REPORTED` (ADRs 2290, 2316). Neither handle is consumed: a window is the
/// caller's to show and free, so this is the call that carries what a window's draw declined,
/// which [`quorra_render_ready_raster`](super::quorra_render_ready_raster) carries for a page.
/// The events are empty where the draw declined nothing.
///
/// # Safety
///
/// See the module documentation. `raster` came from [`quorra_render_request_rasterise_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_raster_declined(
    viewer: *mut Session,
    request: *const RenderRequest,
    raster: *const Drawn,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(request), Some(raster), Some(events)) = (
        viewer.as_mut(),
        request.as_ref(),
        raster.as_ref(),
        events.as_mut(),
    ) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.window_declined(request, raster)));
    Status::Ok.code()
}

/// Tells the viewer the caller took this request's list and holds the page itself —
/// `Rendered::Listed`. The request is **not** consumed: it is the list the caller draws windows
/// from, released with [`quorra_render_request_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_render_ready_listed(
    viewer: *mut Session,
    request: *const RenderRequest,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(request), Some(events)) =
        (viewer.as_mut(), request.as_ref(), events.as_mut())
    else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.render_ready_listed(request)));
    Status::Ok.code()
}

/// A raster's size, its `QUORRA_FORMAT_*` and how many bytes [`quorra_raster_copy`] writes.
///
/// # Safety
///
/// See the module documentation. Every out-parameter is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_raster_info(
    raster: *const Drawn,
    width: *mut u32,
    height: *mut u32,
    format: *mut u32,
    bytes: *mut usize,
) -> c_int {
    let Some(raster) = raster.as_ref() else {
        return Status::NullArgument.code();
    };
    if let Some(width) = width.as_mut() {
        *width = raster.raster.width;
    }
    if let Some(height) = height.as_mut() {
        *height = raster.raster.height;
    }
    if let Some(format) = format.as_mut() {
        *format = PixelFormat::of(raster.raster.format).code();
    }
    if let Some(bytes) = bytes.as_mut() {
        *bytes = raster.raster.data.len();
    }
    Status::Ok.code()
}

/// Copies a raster's pixels into a buffer the caller owns: one copy, as for a frame.
///
/// # Safety
///
/// See the module documentation. `into` is writable for `cap` bytes; `written` is writable, or
/// null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_raster_copy(
    raster: *const Drawn,
    into: *mut u8,
    cap: usize,
    written: *mut usize,
) -> c_int {
    let (Some(raster), false) = (raster.as_ref(), into.is_null()) else {
        return Status::NullArgument.code();
    };
    let pixels = &raster.raster.data;
    if cap < pixels.len() {
        return Status::BufferTooSmall.code();
    }
    core::slice::from_raw_parts_mut(into, pixels.len()).copy_from_slice(pixels);
    if let Some(written) = written.as_mut() {
        *written = pixels.len();
    }
    Status::Ok.code()
}

// ---------------------------------------------------------------------------------------------
// What page one does not need, read on a thread the caller chooses (ADRs 1543, 1553).
// ---------------------------------------------------------------------------------------------

/// What the focused document still reads after page one — §12.3.3's outline and §7.7.3's page
/// tree, placed — as an owning handle the caller may move to another thread.
///
/// Writes null where there is nothing left to read, which is not a failure. Run it with
/// [`quorra_preparation_run`] and hand the answer back with [`quorra_prepared_hand_back`]; a
/// caller that never asks loses only the timing, since the first use reads both on the thread that
/// asked.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_preparation_take(
    viewer: *const Session,
    preparation: *mut *mut viewer_core::Preparation,
) -> c_int {
    let (Some(viewer), Some(preparation)) = (viewer.as_ref(), preparation.as_mut()) else {
        return Status::NullArgument.code();
    };
    *preparation = viewer.preparation().map_or(core::ptr::null_mut(), |found| {
        Box::into_raw(Box::new(found))
    });
    Status::Ok.code()
}

/// Reads what a preparation names, on whichever thread calls it, and **consumes** the preparation.
///
/// It touches only the immutable file, never the viewer, so it may run while the viewer is being
/// used on another thread. The answer is an owning handle: hand it back with
/// [`quorra_prepared_hand_back`], or release it with [`quorra_prepared_free`].
///
/// # Safety
///
/// See the module documentation. `preparation` is not used again after this returns `QUORRA_OK`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_preparation_run(
    preparation: *mut viewer_core::Preparation,
    prepared: *mut *mut viewer_core::Prepared,
) -> c_int {
    let Some(prepared) = prepared.as_mut() else {
        return Status::NullArgument.code();
    };
    if preparation.is_null() {
        return Status::NullArgument.code();
    }
    let read = Box::from_raw(preparation).run();
    *prepared = Box::into_raw(Box::new(read));
    Status::Ok.code()
}

/// Releases a preparation that will not be run. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_preparation_free(preparation: *mut viewer_core::Preparation) {
    if !preparation.is_null() {
        drop(Box::from_raw(preparation));
    }
}

/// Hands back what a preparation read, and **consumes** it.
///
/// Where the document is the one in front and its page falls under a section of the outline, the
/// events hold `QUORRA_EVENT_PAGE_CHANGED` again, whose section [`quorra_event_page_section`](super::quorra_event_page_section)
/// reads. An answer for a document closed since is dropped, and the events are then empty.
///
/// # Safety
///
/// See the module documentation. `prepared` is not used again after this returns `QUORRA_OK`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_prepared_hand_back(
    viewer: *mut Session,
    prepared: *mut viewer_core::Prepared,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    if prepared.is_null() {
        return Status::NullArgument.code();
    }
    *events = Box::into_raw(Box::new(viewer.prepared(*Box::from_raw(prepared))));
    Status::Ok.code()
}

/// Releases a preparation's answer that will not be handed back. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_prepared_free(prepared: *mut viewer_core::Prepared) {
    if !prepared.is_null() {
        drop(Box::from_raw(prepared));
    }
}
