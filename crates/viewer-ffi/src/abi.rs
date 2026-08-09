//! The C entry points, and the only `unsafe` in this crate.
//!
//! Every function here does the same three things and nothing else: turn raw arguments into safe
//! Rust values, call one safe function in a module beside this one, and write the result through
//! an out-parameter. **No decision is taken in this file** — not about a clause, not about a
//! command's meaning, not about an error's wording — so that the audit a reviewer owes is
//! "does this handle pointers correctly" and never "is this the right answer".
//!
//! # The pointer contract, stated once
//!
//! It is the same for every function and is not repeated in each `# Safety` section beyond a
//! pointer back here:
//!
//! - every pointer is either null or valid for the type it names, aligned, and pointing at a
//!   live object this library produced (`pdfv_viewer_new`, `pdfv_open`, …);
//! - **null is always checked** and answers [`Status::NullArgument`]. It is the one bad pointer
//!   this side can detect, and detecting it is worth doing precisely because it is the one a C
//!   caller produces by accident rather than by arithmetic;
//! - an owning handle is freed exactly once, with its own `_free`, and is not used afterwards;
//! - a buffer given for output is writable for the number of bytes stated beside it;
//! - a `const char *` argument is NUL-terminated and is UTF-8. A password and a fragment are the
//!   host's own strings, and one that is not UTF-8 is refused rather than repaired: an invented
//!   replacement character in a password is a password that does not open the file, said quietly;
//! - **no handle may be used from two threads at once.** A `pdfv_render_request *` may be *moved*
//!   to another thread and rasterised there, which is what the round trip is for; a
//!   `pdfv_viewer *` may not be shared, exactly as `viewer-core` is not `Sync`.
//!
//! # Why `unsafe fn` and why `unsafe_op_in_unsafe_fn` is lifted here
//!
//! Every entry point is `pub unsafe extern "C" fn`. C does not see the word — the symbol and the
//! calling convention are identical — and Rust does: a function with preconditions a compiler
//! cannot check says so in its signature, which is the whole of what `unsafe fn` means. A safe
//! `extern "C" fn` that dereferenced its arguments would be an unsound API for any Rust caller,
//! and this crate is an `rlib` as well as a `cdylib`.
//!
//! `unsafe_op_in_unsafe_fn` is lifted for this module because these bodies are *entirely* the
//! unsafe operation: a block around three lines of a four-line shim marks nothing out, and the
//! lint's value — "here, inside this function, is the part that is delicate" — is not available
//! where the answer is "all of it". The position that replaces it is the one
//! `tests/unsafe_position.rs` enforces: **one `unsafe` token per entry point, in the signature,
//! and none anywhere else in the crate.**
//!
//! # Panics
//!
//! None of these panics. An `extern "C"` function that unwound would abort the process, which is
//! Rust's own defined behaviour for it and is not something this crate relies on: every fallible
//! step returns a [`Status`] and every index is checked before it is used.

// See the module documentation. The lift is deliberate, narrow to this file, and the position it
// gives up is replaced by a test that reads the sources back.
#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_char, c_int};

use pdf_render::Raster;
use viewer_core::RenderRequest;

use crate::events::Events;
use crate::kinds::{EventKind, PageTargetKind, PixelFormat, ZoomKind};
use crate::panels::Outline;
use crate::session::{self, FrameInfo, Session};
use crate::status::Status;

/// The revision of everything in this header that a caller compiles against.
///
/// **What it protects is the three structs passed by value**, and nothing else needs it: a
/// function added later is a symbol an old caller never looks up, and a status or a kind added
/// later is a number an old caller has a `default:` arm for — but a field added to
/// [`PdfvGeometry`] or [`PdfvFrame`] changes a size the caller has already compiled, and no
/// diagnostic anywhere would catch it. This number moves whenever one of those does.
pub const PDFV_ABI_VERSION: u32 = 1;

/// Where a page sits on the screen and how large it is drawn.
///
/// Passed by value, which is why [`PDFV_ABI_VERSION`] exists.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvGeometry {
    /// The page's extent in PDF user space units after §7.7.3.3's `/Rotate`.
    pub page_width: f32,
    /// The page's height in the same units.
    pub page_height: f32,
    /// Device pixels per user space unit: the zoom and the display's scale together.
    pub scale: f32,
    /// The rasterised page's width in device pixels.
    pub width: u32,
    /// Its height.
    pub height: u32,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin_x: f32,
    /// The same, vertically.
    pub origin_y: f32,
}

/// What the viewer is holding, without the pixels.
///
/// The first half of C's two-call idiom: ask this, size a buffer, then `pdfv_frame_copy`.
///
/// **Named `pdfv_frame` and not `pdfv_frame_info`, because C has one namespace for both.** A
/// `typedef struct pdfv_frame_info` beside an `int32_t pdfv_frame_info(…)` is a redeclaration
/// error, which is the first thing `c/open_a_page.c` found and is a class of mistake no amount of
/// reading the Rust would have shown.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvFrame {
    /// Which page these pixels are of, zero-based.
    pub page: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The pixel layout: `PDFV_FORMAT_RGBA8`, and nothing else in this build.
    pub format: u32,
    /// How many bytes `pdfv_frame_copy` writes.
    pub bytes: usize,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin_x: f32,
    /// The same, vertically.
    pub origin_y: f32,
}

// ---------------------------------------------------------------------------------------------
// The identity of the ABI. None of these takes a pointer, so none of them is `unsafe`.
// ---------------------------------------------------------------------------------------------

/// The revision this library was built at.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_abi_version() -> u32 {
    PDFV_ABI_VERSION
}

/// How many event kinds this library defines.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_event_kind_count() -> u32 {
    EventKind::COUNT
}

/// Whether a caller's header agrees with this library.
///
/// **The answer to "a variant added later", and the only one C admits.** A caller passes its own
/// `PDFV_ABI_VERSION` and `PDFV_EVENT_KIND_COUNT`; a mismatch answers [`Status::OutOfRange64`],
/// which is this boundary saying "a number moved" in the one channel it has. A caller that runs
/// this in `main` has turned "fails to compile in every consumer" into "fails to start, once,
/// naming what changed" — weaker than the Rust rule, and the strongest thing available.
///
/// **A larger count in the library is not an error and is deliberately not treated as one.** New
/// kinds only ever appear at the end, so an old caller meets one exactly when it receives one,
/// and `pdfv_events_describe` is what it does about it then. What this catches is the caller
/// built against a *newer* header than the library it found.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_abi_check(version: u32, event_kinds: u32) -> c_int {
    if version != PDFV_ABI_VERSION || event_kinds > EventKind::COUNT {
        return Status::OutOfRange64.code();
    }
    Status::Ok.code()
}

/// One sentence about a status, NUL-terminated and never freed.
///
/// A code this build does not define answers `"a status this build does not define"` rather than
/// null, because a caller printing the message of a status it got back should not have to check
/// for null to do it.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_status_message(status: c_int) -> *const c_char {
    let message = match status {
        0 => Status::Ok.message(),
        1 => Status::NullArgument.message(),
        2 => Status::OutOfRange.message(),
        3 => Status::WrongKind.message(),
        4 => Status::BufferTooSmall.message(),
        5 => Status::NoAnswer.message(),
        6 => Status::NotUtf8.message(),
        7 => Status::RenderRefused.message(),
        8 => Status::OutOfRange64.message(),
        _ => "a status this build does not define\0",
    };
    message.as_ptr().cast::<c_char>()
}

/// The name of an event kind, NUL-terminated and never freed.
///
/// `"unknown"` for a kind this build does not define, which is what an *old* library answers a
/// *new* caller. The other direction — a new library and an old caller — is the one that matters
/// and is handled by `pdfv_events_describe`.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_event_kind_name(kind: u32) -> *const c_char {
    let name = EventKind::from_code(kind).map_or("unknown\0", EventKind::name);
    name.as_ptr().cast::<c_char>()
}

// ---------------------------------------------------------------------------------------------
// The viewer.
// ---------------------------------------------------------------------------------------------

/// A viewer for a viewport of this size, or null if it could not be made.
///
/// `scale` is device pixels per logical pixel: 1.0 on an ordinary display, 2.0 on a doubled one.
/// Release it with [`pdfv_viewer_free`].
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_viewer_new(width: u32, height: u32, scale: f32) -> *mut Session {
    Box::into_raw(Box::new(Session::new(width, height, scale)))
}

/// Releases a viewer and everything it holds.
///
/// A null pointer is ignored, which is what `free(NULL)` does and what a caller freeing in an
/// error path expects.
///
/// # Safety
///
/// See the module documentation. `viewer` came from [`pdfv_viewer_new`] and is not used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_viewer_free(viewer: *mut Session) {
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
/// [`pdfv_events_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_open(
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
    *events = Box::into_raw(Box::new(viewer.open(document, file, password, fragment)));
    Status::Ok.code()
}

/// Closes a document and forgets everything derived from it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_close(
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
pub unsafe extern "C" fn pdfv_focus(
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
pub unsafe extern "C" fn pdfv_resize(
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
/// `target` is one of `PDFV_PAGE_*`; `argument` is the index for `PDFV_PAGE_INDEX`, the signed
/// number of pages for `PDFV_PAGE_RELATIVE`, and ignored for the other four.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_go_to_page(
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
/// `zoom` is one of `PDFV_ZOOM_*`; `scale` is logical pixels per user space unit for
/// `PDFV_ZOOM_SCALE`, where 1.0 is 72 dpi, and is ignored for the other five.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_zoom(
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
pub unsafe extern "C" fn pdfv_scroll(
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
/// **A step reads one page**, and the caller pumps [`pdfv_find_continue`] until
/// [`pdfv_event_searched`] reports `remaining` of zero. That is not a courtesy: a sweep of ISO
/// 32000-2's own 1023 pages is 5.84 s, and this ABI does not block a caller's event loop for it.
///
/// # Safety
///
/// See the module documentation. `needle` must point at a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_find_start(
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
pub unsafe extern "C" fn pdfv_find_continue(
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
pub unsafe extern "C" fn pdfv_find_stop(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.find_stop()));
    Status::Ok.code()
}

/// §12.3.3: activates an object the caller is showing outside the page — an outline row.
///
/// The two numbers are §7.3.10's indirect reference, which `pdfv_outline_object` answered with.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_activate(
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
/// See the module documentation. `request` came from [`pdfv_event_render_request`] and `raster`
/// from [`pdfv_render_request_rasterise`]; neither is used or freed after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_render_ready_raster(
    viewer: *mut Session,
    request: *mut RenderRequest,
    raster: *mut Raster,
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
pub unsafe extern "C" fn pdfv_render_ready_failed(
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

// ---------------------------------------------------------------------------------------------
// Events. Owned, so that the viewer's borrow ends before the caller sees anything.
// ---------------------------------------------------------------------------------------------

/// Releases a batch of events. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_events_free(events: *mut Events) {
    if !events.is_null() {
        drop(Box::from_raw(events));
    }
}

/// How many events the batch holds. Zero for a null pointer, which is also the empty answer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_events_len(events: *const Events) -> usize {
    events.as_ref().map_or(0, Events::len)
}

/// Which kind the event at `index` is, written through `kind`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_events_kind(
    events: *const Events,
    index: usize,
    kind: *mut u32,
) -> c_int {
    let (Some(events), Some(kind)) = (events.as_ref(), kind.as_mut()) else {
        return Status::NullArgument.code();
    };
    match events.kind(index) {
        Ok(found) => {
            *kind = found.code();
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One sentence about the event at `index`, whatever kind it is.
///
/// **What a caller does with an event added after it was compiled.** Two-call idiom: pass a null
/// buffer or too small a one to learn the size through `needed`, which counts the terminating
/// NUL, then call again. Nothing is written unless the whole sentence fits.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_events_describe(
    events: *const Events,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.describe(index) {
        Ok(said) => copy_out(&said, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// [`viewer_core::Event::Opened`]: the document's identity and how many pages it has.
///
/// Either out-parameter may be null, which is a caller that wants only the other.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_opened(
    events: *const Events,
    index: usize,
    document: *mut u64,
    pages: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.opened(index) {
        Ok((found, count)) => {
            if let Some(document) = document.as_mut() {
                *document = found;
            }
            if let Some(pages) = pages.as_mut() {
                *pages = count;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// [`viewer_core::Event::PageChanged`]: the zero-based index and how many pages there are.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_page_changed(
    events: *const Events,
    index: usize,
    page: *mut usize,
    of: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.page_changed(index) {
        Ok((found, count)) => {
            if let Some(page) = page.as_mut() {
                *page = found;
            }
            if let Some(of) = of.as_mut() {
                *of = count;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// [`viewer_core::Event::Searched`]: what a step of a document-wide search found.
///
/// Every out-parameter may be null. `found` is what says whether `page`, `from` and `to` mean
/// anything; `remaining` is what says whether to call [`pdfv_find_continue`] again.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_searched(
    events: *const Events,
    index: usize,
    found: *mut c_int,
    page: *mut usize,
    from: *mut usize,
    to: *mut usize,
    remaining: *mut usize,
    wrapped: *mut c_int,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.searched(index) {
        Ok(searched) => {
            if let Some(found) = found.as_mut() {
                *found = c_int::from(searched.found);
            }
            if let Some(page) = page.as_mut() {
                *page = searched.page;
            }
            if let Some(from) = from.as_mut() {
                *from = searched.from;
            }
            if let Some(to) = to.as_mut() {
                *to = searched.to;
            }
            if let Some(remaining) = remaining.as_mut() {
                *remaining = searched.remaining;
            }
            if let Some(wrapped) = wrapped.as_mut() {
                *wrapped = c_int::from(searched.wrapped);
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// [`viewer_core::Event::NeedsRender`]: an owning handle to the request.
///
/// The caller may move it to another thread, rasterise it there with
/// [`pdfv_render_request_rasterise`], and hand it back with [`pdfv_render_ready_raster`]. It must
/// be released — by handing it back, or with [`pdfv_render_request_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_render_request(
    events: *const Events,
    index: usize,
    request: *mut *mut RenderRequest,
) -> c_int {
    let (Some(events), Some(request)) = (events.as_ref(), request.as_mut()) else {
        return Status::NullArgument.code();
    };
    match events.render_request(index) {
        Ok(found) => {
            *request = Box::into_raw(Box::new(found));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

// ---------------------------------------------------------------------------------------------
// Rendering. The display list stays opaque; what crosses is "draw this" and the pixels.
// ---------------------------------------------------------------------------------------------

/// Releases a render request that will not be handed back. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_render_request_free(request: *mut RenderRequest) {
    if !request.is_null() {
        drop(Box::from_raw(request));
    }
}

/// Which page a request is for, zero-based, and the extent it asks for.
///
/// Enough for a caller to decide whether to draw it at all — a request for a page it has turned
/// away from is one it may hand back with [`pdfv_render_ready_failed`] rather than spend a
/// rasterisation on. Any out-parameter may be null.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_render_request_page(
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
/// an owning handle: hand it back with [`pdfv_render_ready_raster`], or release it with
/// [`pdfv_raster_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_render_request_rasterise(
    request: *const RenderRequest,
    raster: *mut *mut Raster,
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
pub unsafe extern "C" fn pdfv_raster_free(raster: *mut Raster) {
    if !raster.is_null() {
        drop(Box::from_raw(raster));
    }
}

// ---------------------------------------------------------------------------------------------
// Queries. Synchronous, producing no events, exactly as `Viewer::query` is.
// ---------------------------------------------------------------------------------------------

/// How many pages the focused document has.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_page_count(viewer: *const Session, pages: *mut usize) -> c_int {
    let (Some(viewer), Some(pages)) = (viewer.as_ref(), pages.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.page_count() {
        Ok(count) => {
            *pages = count;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Which page is showing, and how many there are. Either out-parameter may be null.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_current_page(
    viewer: *const Session,
    page: *mut usize,
    of: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.current_page() {
        Ok((index, count)) => {
            if let Some(page) = page.as_mut() {
                *page = index;
            }
            if let Some(of) = of.as_mut() {
                *of = count;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Where a page sits on the screen and how large it is drawn.
///
/// [`Status::NoAnswer`] for a page that is not the one showing: the geometry is a property of the
/// view rather than of the page.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_page_geometry(
    viewer: *const Session,
    page: usize,
    geometry: *mut PdfvGeometry,
) -> c_int {
    let (Some(viewer), Some(geometry)) = (viewer.as_ref(), geometry.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.page_geometry(page) {
        Ok(found) => {
            *geometry = PdfvGeometry {
                page_width: found.page.width,
                page_height: found.page.height,
                scale: found.scale,
                width: found.width,
                height: found.height,
                origin_x: found.origin.0,
                origin_y: found.origin.1,
            };
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// What the viewer is holding, without the pixels.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_frame_info(viewer: *const Session, info: *mut PdfvFrame) -> c_int {
    let (Some(viewer), Some(info)) = (viewer.as_ref(), info.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.frame_info() {
        Ok(FrameInfo {
            page,
            width,
            height,
            format,
            bytes,
            origin,
        }) => {
            *info = PdfvFrame {
                page,
                width,
                height,
                format: PixelFormat::of(format).code(),
                bytes,
                origin_x: origin.0,
                origin_y: origin.1,
            };
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Copies the frame into a buffer the caller owns.
///
/// One copy, which is what tier 1 costs everywhere in this project. Size the buffer from
/// [`pdfv_frame_info`]'s `bytes`; anything shorter answers [`Status::BufferTooSmall`] and writes
/// nothing, rather than leaving a partial page in it.
///
/// # Safety
///
/// See the module documentation. `into` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_frame_copy(
    viewer: *const Session,
    into: *mut u8,
    cap: usize,
    written: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    let room = core::slice::from_raw_parts_mut(into, cap);
    match viewer.frame_copy(room) {
        Ok(bytes) => {
            if let Some(written) = written.as_mut() {
                *written = bytes;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

// ---------------------------------------------------------------------------------------------
// §12.3.3's outline, flattened. A tree is the one shape a C ABI cannot hand over as itself.
// ---------------------------------------------------------------------------------------------

/// Reads §12.3.3's outline, depth first, as an owning handle.
///
/// Release it with [`pdfv_outline_free`]. It is a snapshot and does not change under the caller:
/// an outline is a property of an immutable document, which is why both native hosts take one
/// when the document opens.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_outline_read(
    viewer: *const Session,
    outline: *mut *mut Outline,
) -> c_int {
    let (Some(viewer), Some(outline)) = (viewer.as_ref(), outline.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.outline() {
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
pub unsafe extern "C" fn pdfv_outline_free(outline: *mut Outline) {
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
pub unsafe extern "C" fn pdfv_outline_len(outline: *const Outline) -> usize {
    outline.as_ref().map_or(0, Outline::len)
}

/// Table 151's `/Title` for one row, in the two-call idiom [`pdfv_events_describe`] uses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_outline_title(
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
pub unsafe extern "C" fn pdfv_outline_depth(
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

/// §7.3.10's two numbers for a row, which [`pdfv_activate`] takes.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_outline_object(
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
// The two helpers every entry point above shares. Both are `unsafe fn` and neither is exported.
// ---------------------------------------------------------------------------------------------

/// A NUL-terminated argument as an owned `String`, or `None` for a null pointer.
///
/// Owned rather than borrowed because what it becomes is a [`viewer_core::Command`] field, which
/// the viewer keeps: a borrow would tie the command's lifetime to a buffer the caller is free to
/// reuse the moment the call returns.
///
/// # Errors
///
/// The bytes were not UTF-8, which is refused rather than repaired — see the module
/// documentation on why an invented replacement character in a password is worse than a refusal.
///
/// # Safety
///
/// `text` is null or points at a NUL-terminated sequence of bytes.
unsafe fn owned_text(text: *const c_char) -> Result<Option<String>, ()> {
    if text.is_null() {
        return Ok(None);
    }
    core::ffi::CStr::from_ptr(text)
        .to_str()
        .map(|text| Some(text.to_owned()))
        .map_err(|_| ())
}

/// Writes a string and its terminating NUL into a caller's buffer.
///
/// The second half of C's two-call idiom, in one place so that every string-valued entry point
/// spells it the same way. `needed` counts the NUL, so a caller that allocates exactly that many
/// bytes succeeds on the second call. **Nothing is written unless the whole string fits**, which
/// is what keeps a truncated title from looking like a short one.
///
/// # Safety
///
/// `out` is null or writable for `cap` bytes; `needed` is null or writable.
unsafe fn copy_out(text: &str, out: *mut c_char, cap: usize, needed: *mut usize) -> c_int {
    let wanted = text.len().saturating_add(1);
    if let Some(needed) = needed.as_mut() {
        *needed = wanted;
    }
    if out.is_null() || cap < wanted {
        return Status::BufferTooSmall.code();
    }
    let room = core::slice::from_raw_parts_mut(out.cast::<u8>(), wanted);
    let Some(body) = room.get_mut(..text.len()) else {
        // Unreachable: `wanted` is one more than `text.len()`. Written as a refusal rather than
        // an index so that the slice above is the only place a length is trusted.
        return Status::BufferTooSmall.code();
    };
    body.copy_from_slice(text.as_bytes());
    if let Some(last) = room.last_mut() {
        *last = 0;
    }
    Status::Ok.code()
}
