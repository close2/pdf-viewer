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

use crate::answers::{Collection, Matches, Miniature, Popups, Structure};
use crate::events::Events;
use crate::form::Form;
use crate::kinds::{
    BoxKind, ColumnTextKind, ControlKind, DelegateKind, ElementKind, EventKind, FocusKind,
    FolderTextKind, LayoutKind, MarkupKind, NoteKind, OrderKind, PageModeKind, PageTargetKind,
    PixelFormat, PointerKind, PreferenceKey, PresentKind, PurposeKind, RestrictKind, RowKind,
    SelectKind, ShortfallKind, TextKind, ZoomKind,
};
use crate::panels::{Outline, Panel};
use crate::session::{self, FrameInfo, Session};
use crate::shapes::Quads;
use crate::status::Status;

/// The revision of everything in this header that a caller compiles against.
///
/// **What it protects is the structs passed by value**, and nothing else needs it: a function
/// added later is a symbol an old caller never looks up, and a status or a kind added later is a
/// number an old caller has a `default:` arm for — but a field added to [`PdfvGeometry`],
/// [`PdfvFrame`] or [`PdfvViewing`] changes a size the caller has already compiled, and no
/// diagnostic anywhere would catch it. This number moves whenever one of those does.
///
/// **A struct *added* does not move it**, which is the same argument as the one for a function: a
/// caller compiled before [`PdfvViewing`] existed passes nothing of that shape and looks up neither
/// of the two symbols that take one.
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

/// Where the reader is looking: the page, the magnification and the scroll.
///
/// Passed by value, which is why [`PDFV_ABI_VERSION`] exists. A caller reads one from
/// `pdfv_view`, keeps it, and hands it back to `pdfv_set_view` — the two directions this value is
/// meant to travel in, and a C caller composing one from numbers of its own is guessing where the
/// viewer's clamp would have left the reader.
///
/// **Named `pdfv_viewing` in the header and not `pdfv_view`, for [`PdfvFrame`]'s reason**: C puts
/// a struct tag and a function in one namespace, so a `typedef struct pdfv_view` beside an
/// `int32_t pdfv_view(…)` is a redeclaration error. That note was written about
/// `pdfv_frame_info`; this is the second time the same fact has decided a name here.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvViewing {
    /// Which page, zero-based.
    pub page: usize,
    /// How large the page is drawn: one of `PDFV_ZOOM_*`.
    pub zoom: u32,
    /// Logical pixels per user space unit for `PDFV_ZOOM_SCALE`, and zero for the others.
    pub scale: f32,
    /// How far the page is scrolled under the viewport, in device pixels; positive moves the
    /// content up and left, which is the sense `pdfv_scroll`'s delta has.
    pub scroll_x: f32,
    /// The same, vertically.
    pub scroll_y: f32,
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

/// Where the reader is looking: the page, the magnification and the scroll.
///
/// [`Status::NoAnswer`] when no document is focused. What it is for is `pdfv_set_view`: the
/// commands that make a view are relative and clamped, so a caller that issued every one of them
/// still cannot say where the reader ended up.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_view(viewer: *const Session, view: *mut PdfvViewing) -> c_int {
    let (Some(viewer), Some(view)) = (viewer.as_ref(), view.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.view() {
        Ok(found) => {
            let (kind, scale) = ZoomKind::of(found.zoom);
            *view = PdfvViewing {
                page: found.page,
                zoom: kind as u32,
                scale,
                scroll_x: found.scroll.0,
                scroll_y: found.scroll.1,
            };
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Puts the reader back at a view `pdfv_view` answered with.
///
/// [`Status::WrongKind`] for a magnification code this build does not define, which is what every
/// other entry point taking a kind answers.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_set_view(
    viewer: *mut Session,
    view: PdfvViewing,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = ZoomKind::from_code(view.zoom) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.set_view(viewer_core::Viewing {
        page: view.page,
        zoom: kind.zoom(view.scale),
        scroll: (view.scroll_x, view.scroll_y),
    })));
    Status::Ok.code()
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

/// How many frames the viewer is holding — Table 29's arrangement, counted.
///
/// **The one entry point `/PageLayout` cost this ABI**, and it exists because a C consumer cannot
/// fail to compile: `pdfv_frame_info` and `pdfv_frame_copy` gained an index, and a caller has to
/// be able to learn how many there are. Zero where the viewer holds none.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_frame_count(viewer: *const Session) -> usize {
    viewer.as_ref().map_or(0, Session::frame_count)
}

/// What the viewer is holding for one page of the arrangement, without the pixels.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_frame_info(
    viewer: *const Session,
    frame: usize,
    info: *mut PdfvFrame,
) -> c_int {
    let (Some(viewer), Some(info)) = (viewer.as_ref(), info.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.frame_info(frame) {
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
    frame: usize,
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
    match viewer.frame_copy(frame, room) {
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
// The pointer, the selection and §12.5.1's focus.
//
// Everything a person *does* to a page, which the four-hundred-and-eleventh session left out
// because a C host had not asked for it yet. Each is a symbol, and a symbol added later costs a
// compiled caller nothing — which is the property the shape was chosen for.
// ---------------------------------------------------------------------------------------------

/// §12.5.5: the pointer moved, or a button went down or up, at a point in the viewport.
///
/// Device pixels from the viewport's top-left corner. `action` is one of `PDFV_POINTER_*`; a
/// number outside them answers [`Status::WrongKind`] rather than guessing at a fifth situation the
/// clause does not describe.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_pointer(
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
/// A drag is [`pdfv_pointer`]'s business; this is what a menu item or a keystroke asks for.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_select(
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
pub unsafe extern "C" fn pdfv_focused(
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
pub unsafe extern "C" fn pdfv_link_at(
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

/// The selected text, in the two-call idiom [`pdfv_events_describe`] uses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_selection_text(
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

/// The text a copy should carry, in the two-call idiom, with the order it came back in.
///
/// **What a caller needs in order to put a selection on its own clipboard** (ADR 0519). The three
/// windowed hosts in this tree call `gdk::Clipboard`, `QClipboard` and `arboard`; a C caller's
/// platform is its own business and this ABI has no business guessing it — so what crosses is the
/// characters and the one thing a caller cannot work out for itself, which of ISO 32000-2
/// §14.8.2.5's two content orders they are in.
///
/// `order` receives `PDFV_ORDER_LOGICAL` or `PDFV_ORDER_PAGE_CONTENT`, and may be null for a
/// caller that does not care. It is written only on `PDFV_OK`.
///
/// [`pdfv_selection_text`] is still there and still answers in page content order: that is the
/// order the quadrilaterals are in, so it is the right answer for anything being *drawn*.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null; `order` is writable
/// for one `uint32_t`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_selection_copy_text(
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
            // `PDFV_BUFFER_TOO_SMALL` on the sizing call of the two-call idiom, and a caller that
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
/// Release it with [`pdfv_quads_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_selection_quads(
    viewer: *const Session,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(viewer), Some(quads)) = (viewer.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.selection_quads() {
        Ok(found) => {
            *quads = Box::into_raw(Box::new(found));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// ISO 32000-2 Annex O's highlighted rectangles on the page being shown, or nothing.
///
/// Table Annex O.4's `highlight` parameter of the fragment the document was opened with — "[o]pen
/// the document with the specified rectangle highlighted … [t]he nature of the highlighting is
/// implementation-dependent", which is why this hands over shapes and not a picture. A caller that
/// passed no fragment, or one naming no rectangle for this page, gets an empty list. Release it
/// with [`pdfv_quads_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_highlight_quads(
    viewer: *const Session,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(viewer), Some(quads)) = (viewer.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.highlight_quads() {
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
pub unsafe extern "C" fn pdfv_quads_free(quads: *mut Quads) {
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
pub unsafe extern "C" fn pdfv_quads_len(quads: *const Quads) -> usize {
    quads.as_ref().map_or(0, Quads::len)
}

/// One shape's eight numbers, `[x0, y0, … x3, y3]` in device pixels of the viewport.
///
/// # Safety
///
/// See the module documentation. `into` is writable for eight `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_quads_get(
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
pub unsafe extern "C" fn pdfv_focused_annotation(
    viewer: *const Session,
    number: *mut u32,
    generation: *mut u16,
    quad: *mut f32,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.focused_annotation() {
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

// ---------------------------------------------------------------------------------------------
// §12.7's form: the sixth chrome population, and the last one to reach this ABI.
// ---------------------------------------------------------------------------------------------

/// §12.7's fields with a widget on the page being shown, as an owning handle.
///
/// Release it with [`pdfv_fields_free`]. **Not at pointer speed**: this walks §12.7.4.1's field
/// tree, so a caller asks it when a page appears and after an edit, exactly as `viewer-gtk` and
/// `viewer-qt` do. A click asks [`pdfv_field_at`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_fields_read(viewer: *const Session, fields: *mut *mut Form) -> c_int {
    let (Some(viewer), Some(fields)) = (viewer.as_ref(), fields.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.fields() {
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
pub unsafe extern "C" fn pdfv_fields_free(fields: *mut Form) {
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
pub unsafe extern "C" fn pdfv_fields_len(fields: *const Form) -> usize {
    fields.as_ref().map_or(0, Form::len)
}

/// One of the three names a field carries — `PDFV_TEXT_QUALIFIED`, `_SHOWN` or `_PARTIAL`.
///
/// §14.9.3 is why there is more than one: the shown name "shall be used in place of the actual
/// field name when an interactive PDF processor identifies the field in a user-interface", while
/// the qualified one is what [`pdfv_set_field_text`] addresses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_name(
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

/// Which control the field is — one of `PDFV_CONTROL_*` — and every flag that decides how to
/// build it.
///
/// The flags are `PDFV_FIELD_*`, one bit per boolean Tables 227, 229, 231 and 233 state. A bit
/// this build does not define is zero, and a bit added later is one an old caller does not read.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_control(
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
pub unsafe extern "C" fn pdfv_field_limits(
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
/// A password field answers Table 231 bit 14's bullets and sets `PDFV_FIELD_OBSCURED`. A caller
/// obeying the read-back rule must consult that bit: writing the bullets back would send them as
/// the next value, which is the bug ADR 0247 found in this project's own first host.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_value(
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

/// How many of Table 234's `/Opt` entries the field states.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_option_count(
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

/// One option's label (`PDFV_TEXT_LABEL`) or export value (`PDFV_TEXT_EXPORT`).
///
/// In the array's own order, which Table 233 bit 20 requires: "PDF readers shall display the
/// options in the order in which they occur in the Opt array."
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_option(
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
pub unsafe extern "C" fn pdfv_field_option_selected(
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
pub unsafe extern "C" fn pdfv_field_widget_count(
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
/// [`pdfv_selection_quads`] and [`pdfv_focused_annotation`] take, because a caller places a control the same way
/// it draws a highlight.
///
/// # Safety
///
/// See the module documentation. `quad` is writable for eight `float`s, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_widget(
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

/// A widget's `/AP /N` on-state name (`PDFV_TEXT_LABEL`) or Table 230's `/Opt` entry for it
/// (`PDFV_TEXT_EXPORT`).
///
/// **The two say different things and the clause is why.** §12.7.5.2.3: "the names used to
/// represent the on state in the AP dictionary of each annotation may use numerical position …
/// encoded as a name object (for example: /0, /1)", so `/AP` may say `0` while `/Opt` says `Rot` —
/// and only the first selects an appearance while only the second is worth showing a person. The
/// first is what [`pdfv_set_field_text`] sends to check the box.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_widget_text(
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

/// What the field at a viewport point is called — `PDFV_TEXT_QUALIFIED` or `PDFV_TEXT_SHOWN`.
///
/// What a caller asks on a click, before it can send an edit. [`Status::NoAnswer`] where no field
/// is there.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_at(
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
pub unsafe extern "C" fn pdfv_caret(
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

/// How far into a field's value a point inside it is, in bytes — [`pdfv_caret`]'s inverse.
///
/// `x` and `y` name the field, as [`pdfv_field_at`]'s do; `point_x` and `point_y` are the place to
/// measure, which is the same point on a click and a different one on every move of a drag. The
/// answer is the *nearest* boundary between two glyphs and never a refusal for a point in the wrong
/// place: a caller that has decided a press belongs to a field has to put the cursor somewhere.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_offset(
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
/// A third question rather than [`pdfv_caret`] twice, and §12.7.5.3's Table 231 bit 13 is what
/// settles it: a multiline field's value is broken into lines by the layout, so a caller holding
/// both ends of a selection cannot name the lines *between* them. Release the handle with
/// [`pdfv_quads_free`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_field_selection(
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
/// How a caller aims a keyboard at one: the object goes back to [`pdfv_set_free_text`]. An
/// annotation the *file* states answers [`Status::NoAnswer`] deliberately — nothing in this
/// vocabulary can change one, and offering it would be an interface pretending to work.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_free_text_at(
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
/// [`pdfv_field_widget_text`] hands over.
///
/// # Safety
///
/// See the module documentation. `field` and `text` are NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_set_field_text(
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
/// clear: "[t]he default value of V is null, indicating that no item is currently selected."
///
/// An index past the end of `/Opt` names nothing and is dropped, and more than one index on a field
/// whose `MultiSelect` flag is clear is cut to the first, both by `viewer-core`.
///
/// # Safety
///
/// See the module documentation. `options` is readable for `count` elements, or null when `count`
/// is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_set_field_options(
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
pub unsafe extern "C" fn pdfv_clear_field(
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
pub unsafe extern "C" fn pdfv_markup(
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
/// A drag rather than a selection, which is the whole difference from [`pdfv_markup`] and follows
/// from the clause: that subtype "displays text directly on the page", so there is nothing on the
/// page for it to be over. The two corners are in device pixels of the viewport, in either order;
/// a rectangle with no area is a press that never moved and adds nothing.
///
/// [`pdfv_free_text_at`] is how the caller learns which annotation the drag made.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_free_text(
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
pub unsafe extern "C" fn pdfv_set_free_text(
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

/// Undoes the last edit.
///
/// The surviving prefix of the log is *replayed* rather than inverted, which is why an edit can be
/// undone without the viewer having remembered what it replaced.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_undo(viewer: *mut Session, events: *mut *mut Events) -> c_int {
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
pub unsafe extern "C" fn pdfv_redo(viewer: *mut Session, events: *mut *mut Events) -> c_int {
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
pub unsafe extern "C" fn pdfv_dirty(viewer: *const Session, dirty: *mut bool) -> c_int {
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

// ---------------------------------------------------------------------------------------------
// Bytes out: §7.5.6's incremental update and §7.11.4's embedded files.
// ---------------------------------------------------------------------------------------------

/// Writes §7.5.6's incremental update for everything the log holds.
///
/// The bytes arrive on a `PDFV_EVENT_SAVED` and are read with [`pdfv_event_bytes`]; the *caller*
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
pub unsafe extern "C" fn pdfv_save(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.save()));
    Status::Ok.code()
}

/// §7.11.4: takes an embedded file's bytes out of the document.
///
/// The name is the key the `/EmbeddedFiles` tree filed the file under, which is what
/// [`pdfv_panel_name`] answered with. The bytes arrive on a `PDFV_EVENT_EXTRACTED`, decoded:
/// §7.4's filters are undone here, because a caller that had to decode them would be a second
/// reader of the document.
///
/// # Safety
///
/// See the module documentation. `name` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_extract(
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

/// The bytes a `PDFV_EVENT_NEEDS_FILE` asked for, or a refusal.
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
pub unsafe extern "C" fn pdfv_supply(
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

// ---------------------------------------------------------------------------------------------
// §8.11.4.3's layers and §7.11.4's files: the other two panels, flattened as the outline is.
// ---------------------------------------------------------------------------------------------

/// §8.11.4.3's `/Order`, depth first, as an owning handle.
///
/// Release it with [`pdfv_panel_free`]. Table 99's `/Locked` comes back on
/// [`pdfv_panel_action`]: "[t]he state of a locked group cannot be changed through the user
/// interface of an interactive PDF processor", so a caller builds that row's switch insensitive
/// rather than sending [`pdfv_set_group`] for it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_layers_read(viewer: *const Session, panel: *mut *mut Panel) -> c_int {
    let (Some(viewer), Some(panel)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.layers() {
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
pub unsafe extern "C" fn pdfv_attachments_read(
    viewer: *const Session,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(viewer), Some(panel)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.attachments() {
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
pub unsafe extern "C" fn pdfv_panel_free(panel: *mut Panel) {
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
pub unsafe extern "C" fn pdfv_panel_len(panel: *const Panel) -> usize {
    panel.as_ref().map_or(0, Panel::len)
}

/// A row's label, or the second line beside it — `detail` non-zero for the second.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_panel_text(
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
pub unsafe extern "C" fn pdfv_panel_depth(
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

/// Which of `PDFV_ROW_*` acting on the row is, and everything the action carries.
///
/// `number` and `generation` are §7.3.10's two numbers for `PDFV_ROW_ACTIVATE` and
/// `PDFV_ROW_TOGGLE`, and zero otherwise — §7.5.4 reserves object number zero for the head of the
/// free list, so it is never an object a document states. `on` and `locked` mean something for
/// `PDFV_ROW_TOGGLE` alone; the `/EmbeddedFiles` key of a `PDFV_ROW_EXTRACT` row is
/// [`pdfv_panel_name`].
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_panel_action(
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

/// The `/EmbeddedFiles` key [`pdfv_extract`] takes for a `PDFV_ROW_EXTRACT` row, `""` for others.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_panel_name(
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
/// hold; [`pdfv_layers_read`] is where a caller gets the identities.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_set_group(
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

// ---------------------------------------------------------------------------------------------
// §12.4.4's clock, and the three policy values only a host can supply.
// ---------------------------------------------------------------------------------------------

/// §12.4.4.1: time has passed, in milliseconds.
///
/// **This library has no clock**, so a presentation that advances by itself can only know a second
/// went by by being told. A page stating no `/Dur` swallows every tick — "[i]f no Dur entry is
/// specified in the page object, the page shall not advance automatically."
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_tick(
    viewer: *mut Session,
    millis: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.tick(millis)));
    Status::Ok.code()
}

/// §12.4.4: the caller has entered or left presentation mode.
///
/// A statement only a host can make, and §12.4.4.2 is why it exists at all: that clause conditions
/// a *state machine* — which navigation node is current — on being in presentation mode, and a
/// person stepping through a slide show by hand drives no clock, so it cannot be deduced from
/// [`pdfv_tick`] (ADR 0316). Entering saves §8.11's group states and leaving restores them, which
/// is NOTE 2's own instruction.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_present(
    viewer: *mut Session,
    mode: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = PresentKind::from_code(mode) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.present(kind.mode())));
    Status::Ok.code()
}

/// Table 29's `/PageLayout`: how the pages are arranged in the window.
///
/// The document's own value is what a session opens in, and this is how a *reader* changes it —
/// which is the whole reason the message exists: Table 29 states the layout "shall be used when
/// the document is opened", an initial state and not a permanent one.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_layout(
    viewer: *mut Session,
    layout: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = LayoutKind::from_code(layout) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.layout(kind.layout())));
    Status::Ok.code()
}

/// How much of what a document asserts about its reader this viewer obeys.
///
/// **`CLAUDE.md` states it: "it shall always be possible to turn them off."** A document's
/// restrictions — §7.6.4.2's Table 22, §12.8.2.2's `/DocMDP` — are the *reader's* to set, and a
/// state machine over the file cannot know how much of somebody else's file a person's own program
/// should obey. `PDFV_RESTRICT_ON` is the default; what is refused arrives as a
/// `PDFV_EVENT_REFUSED`, which is deliberately not a `PDFV_EVENT_REPORTED`: one says what the
/// *document* could not do and the other what the reader's own policy did.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_restrict(
    viewer: *mut Session,
    level: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = RestrictKind::from_code(level) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.restrict(kind.level())));
    Status::Ok.code()
}

/// §6.3.2.2's "unless otherwise instructed": who draws §12.7's widget appearances.
///
/// `PDFV_DELEGATE_DELEGATED` removes from the page **exactly the widgets [`pdfv_fields_read`]
/// answered for**, so a caller that has placed real controls over them draws each part of the page
/// once. A widget §12.7.4.2 leaves "simply a Widget annotation" keeps its appearance, because no
/// control replaced it. Changing this re-interprets the page, because §12.5.5's appearance streams
/// are drawing commands rather than pixels.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_delegate(
    viewer: *mut Session,
    appearances: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(kind) = DelegateKind::from_code(appearances) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.delegate(kind.appearances())));
    Status::Ok.code()
}

// ---------------------------------------------------------------------------------------------
// The event accessors the four-hundred-and-eleventh session left out.
// ---------------------------------------------------------------------------------------------

/// Which document the event at `index` is about.
///
/// [`Status::WrongKind`] for a `PDFV_EVENT_DAMAGE`, which is about the *viewport* and names no
/// document.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_document(
    events: *const Events,
    index: usize,
    document: *mut u64,
) -> c_int {
    let (Some(events), Some(document)) = (events.as_ref(), document.as_mut()) else {
        return Status::NullArgument.code();
    };
    match events.document(index) {
        Ok(found) => {
            *document = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The bytes of a `PDFV_EVENT_SAVED` or a `PDFV_EVENT_EXTRACTED`.
///
/// **A byte buffer and not a string**, in the same two-call idiom: both carry a *file*, and a file
/// is not text — §7.5.6's update is a PDF and an embedded file may be anything at all, so the
/// NUL-terminated idiom would truncate either at its first zero byte. `needed` is the exact length
/// and counts no terminator; nothing is written unless the whole file fits.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_bytes(
    events: *const Events,
    index: usize,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    let bytes = match events.bytes(index) {
        Ok(bytes) => bytes,
        Err(status) => return status.code(),
    };
    if let Some(needed) = needed.as_mut() {
        *needed = bytes.len();
    }
    if out.is_null() || cap < bytes.len() {
        return Status::BufferTooSmall.code();
    }
    core::slice::from_raw_parts_mut(out, bytes.len()).copy_from_slice(bytes);
    Status::Ok.code()
}

/// A `PDFV_EVENT_EXTRACTED`'s file name, and whether a person asked for it.
///
/// `asked` is §O.2.1's distinction rather than decoration: a URI's `ef` parameter extracts a file
/// nobody pressed anything for, and the annex says a processor "may choose to prompt the user or
/// even prevent opening of the file" for that case alone. The name is the document's own words and
/// is **not** a path.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_extracted(
    events: *const Events,
    index: usize,
    asked: *mut bool,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.extracted(index) {
        Ok((name, by_a_person)) => {
            if let Some(asked) = asked.as_mut() {
                *asked = by_a_person;
            }
            copy_out(name, out, cap, needed)
        }
        Err(status) => status.code(),
    }
}

/// A `PDFV_EVENT_OPEN_URI`'s resolved URI.
///
/// Handed over rather than opened, and that is not squeamishness: the string is one the *document*
/// controls, and handing it to a browser is a decision about this machine.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_open_uri(
    events: *const Events,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.open_uri(index) {
        Ok(uri) => copy_out(uri, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// A `PDFV_EVENT_NEEDS_FILE`'s purpose and the document's own words for the file.
///
/// Answer it with [`pdfv_supply`], including with a null buffer, which is a caller declining.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_needs_file(
    events: *const Events,
    index: usize,
    purpose: *mut u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.needs_file(index) {
        Ok((wanted, name)) => {
            if let Some(purpose) = purpose.as_mut() {
                *purpose = wanted.code();
            }
            copy_out(name, out, cap, needed)
        }
        Err(status) => status.code(),
    }
}

/// A `PDFV_EVENT_DAMAGE`'s rectangle, `[x0, y0, x1, y1]` in device pixels.
///
/// A bound on what changed rather than a promise that everything inside it did.
///
/// # Safety
///
/// See the module documentation. `into` is writable for four `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_damage(
    events: *const Events,
    index: usize,
    into: *mut f32,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match events.damage(index) {
        Ok(rect) => {
            core::slice::from_raw_parts_mut(into, rect.len()).copy_from_slice(&rect);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A `PDFV_EVENT_DIRTY`'s answer: whether the document now differs from the file it came from.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_dirty(
    events: *const Events,
    index: usize,
    dirty: *mut bool,
) -> c_int {
    let (Some(events), Some(dirty)) = (events.as_ref(), dirty.as_mut()) else {
        return Status::NullArgument.code();
    };
    match events.dirty(index) {
        Ok(found) => {
            *dirty = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A `PDFV_EVENT_TRANSITION`'s Table 164 numbers, without the style.
///
/// `seconds` is `/D`, and zero for `R`, whose row says "the D entry shall be ignored".
/// `dimension` is `PDFV_DIMENSION_*` and `motion` is `PDFV_MOTION_*`. `directed` says whether `/Di`
/// states an angle at all, as against the name `None`; `degrees` is that angle counterclockwise
/// from a left-to-right direction, which the table warns "differs from the page object's Rotate
/// entry, which is measured clockwise from the top".
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_transition(
    events: *const Events,
    index: usize,
    seconds: *mut f32,
    dimension: *mut u32,
    motion: *mut u32,
    directed: *mut bool,
    degrees: *mut f32,
    scale: *mut f32,
    opaque: *mut bool,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.transition(index) {
        Ok(numbers) => {
            if let Some(seconds) = seconds.as_mut() {
                *seconds = numbers.seconds;
            }
            if let Some(dimension) = dimension.as_mut() {
                *dimension = u32::from(numbers.vertical);
            }
            if let Some(motion) = motion.as_mut() {
                *motion = u32::from(numbers.outward);
            }
            if let Some(directed) = directed.as_mut() {
                *directed = numbers.directed;
            }
            if let Some(degrees) = degrees.as_mut() {
                *degrees = numbers.degrees;
            }
            if let Some(scale) = scale.as_mut() {
                *scale = numbers.scale;
            }
            if let Some(opaque) = opaque.as_mut() {
                *opaque = numbers.opaque;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A `PDFV_EVENT_TRANSITION`'s Table 164 `/S`, as the table spells it.
///
/// **A name rather than a number this ABI invented**, and it is the only enumeration here that
/// crosses as text. `/S` *is* a name in the file, and the table's thirteenth case is a name it does
/// not define, "kept as the file wrote it" — a number would have had to lose that one, and a caller
/// that cannot animate an unknown style falls back to the table's own default `R` and can say which
/// style it could not play (ADR 0230).
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_event_transition_style(
    events: *const Events,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.transition_style(index) {
        Ok(style) => copy_out(&style, out, cap, needed),
        Err(status) => status.code(),
    }
}

// ---------------------------------------------------------------------------------------------
// What the page could not draw, and the names of the two answered enumerations.
// ---------------------------------------------------------------------------------------------

/// How many pages on the screen this viewer has anything to say about.
///
/// **Table 29's arrangement, counted a second time**, and it is here for the reason
/// [`pdfv_frame_count`] is: a C consumer cannot fail to compile, so a `/PageLayout` putting four
/// pages on the screen has to be something a caller *asks* about rather than something it is
/// silently given one quarter of. Zero where no document is focused or no page has been read;
/// one under `SinglePage`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_reported_pages(viewer: *const Session) -> usize {
    viewer.as_ref().map_or(0, Session::reported_pages)
}

/// Which page one of those entries is about, zero-based.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_reported_page(
    viewer: *const Session,
    entry: usize,
    page: *mut usize,
) -> c_int {
    let (Some(viewer), Some(page)) = (viewer.as_ref(), page.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reported_page(entry) {
        Ok(index) => {
            *page = index;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many sentences one of those pages has about what it could not draw.
///
/// The same sentences a `PDFV_EVENT_REPORTED` carried, kept so that a caller which cleared its
/// status bar can ask again rather than remembering. Trap 5's channel: every layer of this program
/// says what it could not handle rather than falling back to something plausible.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_reports_len(
    viewer: *const Session,
    entry: usize,
    count: *mut usize,
) -> c_int {
    let (Some(viewer), Some(count)) = (viewer.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reports(entry) {
        Ok(notes) => {
            *count = notes.len();
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of those sentences.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_report(
    viewer: *const Session,
    entry: usize,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    let notes = match viewer.reports(entry) {
        Ok(notes) => notes,
        Err(status) => return status.code(),
    };
    match notes.get(index) {
        Some(note) => copy_out(note, out, cap, needed),
        None => Status::OutOfRange.code(),
    }
}

/// How many control kinds this library defines, and the name of one.
///
/// The pair `pdfv_event_kind_count` and `pdfv_event_kind_name` make for an event, and for the same
/// reason: a number a caller has no arm for should still be printable. **Not part of
/// [`pdfv_abi_check`]**, deliberately — an event *arrives* whether or not the caller asked, so its
/// count has to be checked before the first one turns up; a control kind is the answer to a call
/// the caller wrote.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_control_kind_count() -> u32 {
    ControlKind::COUNT
}

/// The name of a control kind, NUL-terminated and never freed. `"unknown"` for one this build does
/// not define.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_control_kind_name(kind: u32) -> *const c_char {
    let name = ControlKind::from_code(kind).map_or("unknown\0", ControlKind::name);
    name.as_ptr().cast::<c_char>()
}

/// How many panel row actions this library defines.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_row_kind_count() -> u32 {
    RowKind::COUNT
}

/// The name of a panel row action, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_row_kind_name(kind: u32) -> *const c_char {
    let name = RowKind::from_code(kind).map_or("unknown\0", RowKind::name);
    name.as_ptr().cast::<c_char>()
}

// ---------------------------------------------------------------------------------------------
// The other half of the queries — `doc/todo/30` item 5, ADR 0576.
//
// Eleven of `viewer_core::Query`'s variants reached no symbol at all, and nothing counted them
// until `tools/state.sh hosts` did. Every one of them is below, and
// `tests/every_query_reaches_the_abi.rs` is what keeps the list closed: it matches exhaustively
// over `Query`, so a variant added to `viewer-core` fails to compile there rather than arriving
// with no symbol and no signal — which is exactly how eleven accumulated.
//
// No entry point below takes or returns a struct by value. `PDFV_ABI_VERSION` therefore does not
// move, which is the whole reason the shapes are handles and out-parameters.
// ---------------------------------------------------------------------------------------------

/// Every occurrence of a string on the page being shown, as shapes to draw over it.
///
/// `pdfv_find_start` searches the *document* one page per step; this answers for the page in front
/// of the reader, out of a readback that already exists, so a find bar may ask it on every repaint.
/// A caller had the first and not the second until ADR 0576.
///
/// # Safety
///
/// See the module documentation. `needle` is NUL-terminated and UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_find_matches(
    viewer: *const Session,
    needle: *const c_char,
    matches: *mut *mut Matches,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), matches.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Ok(Some(needle)) = owned_text(needle) else {
        return Status::NotUtf8.code();
    };
    match viewer.find(&needle) {
        Ok(found) => {
            *out = Box::into_raw(Box::new(found));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `pdfv_find_matches` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_matches_free(matches: *mut Matches) {
    if !matches.is_null() {
        drop(Box::from_raw(matches));
    }
}

/// How many occurrences there are. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_matches_len(matches: *const Matches) -> usize {
    matches.as_ref().map_or(0, Matches::len)
}

/// The shapes covering one occurrence, as a `pdfv_quads *` the caller frees.
///
/// **One occurrence is several quadrilaterals**, because a term wrapped across a line is merged
/// per run of a line — so *next match* is this index and never the next shape.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_matches_quads(
    matches: *const Matches,
    index: usize,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(matches), Some(out)) = (matches.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match matches.quads(index) {
        Ok(shapes) => {
            *out = Box::into_raw(Box::new(Quads::new(shapes)));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Table 29's `/PageMode` and `/PageLayout`: what the catalogue asks of the window opening it.
///
/// `pdfv_layout` sets the arrangement a *reader* chose; this is the one the *document* opens in,
/// and a caller had no way to ask for it. Both native hosts obey the catalogue on open.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_opening(
    viewer: *const Session,
    mode: *mut u32,
    layout: *mut u32,
) -> c_int {
    let (Some(viewer), Some(mode), Some(layout)) =
        (viewer.as_ref(), mode.as_mut(), layout.as_mut())
    else {
        return Status::NullArgument.code();
    };
    match viewer.opening() {
        Ok((page_mode, arrangement)) => {
            *mode = page_mode.code();
            *layout = arrangement as u32;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many of Table 29's page modes this library defines, and the name of one.
///
/// The pair `pdfv_control_kind_count` and `pdfv_control_kind_name` make, for the same reason and
/// deliberately not in `pdfv_abi_check`: this is the answer to a call the caller wrote. Table 29
/// gained `/UseOC` in PDF 1.5 and `/UseAttachments` in PDF 1.6, which is why it is counted at all.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_page_mode_count() -> u32 {
    PageModeKind::COUNT
}

/// The name of a page mode, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_page_mode_name(mode: u32) -> *const c_char {
    let name = PageModeKind::from_code(mode).map_or("unknown\0", PageModeKind::name);
    name.as_ptr().cast::<c_char>()
}

/// One entry of §12.2's Table 147, as a number.
///
/// **One keyed accessor rather than nineteen symbols or a struct**, and the argument is this
/// module's own transposed from a command to a table: a struct passed by value would put Table
/// 147's size in the ABI, and a symbol apiece would be nineteen exports for one table. An entry
/// added by a later part of ISO 32000 is a new `PDFV_PREF_…` constant beside a function every
/// compiled caller already links.
///
/// A boolean answers zero or one, an enumerated name answers its own `PDFV_…` number, and a count
/// answers itself. `PDFV_NO_ANSWER` is the three entries Table 147 leaves genuinely open —
/// `/Duplex`, `/PickTrayByPDFSize` and `/NumCopies` — where the document states none, because
/// "the document says nothing" and "the document says the default" are different facts.
/// `PDFV_WRONG_KIND` is `PDFV_PREF_PRINT_PAGE_RANGE`, which is a list: see
/// `pdfv_preference_ranges`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_preference(
    viewer: *const Session,
    key: u32,
    value: *mut i64,
) -> c_int {
    let (Some(viewer), Some(value)) = (viewer.as_ref(), value.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(key) = PreferenceKey::from_code(key) else {
        return Status::OutOfRange.code();
    };
    match viewer.preference(key) {
        Ok(number) => {
            *value = number;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many sub-ranges Table 147's `/PrintPageRange` states.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_preference_ranges(
    viewer: *const Session,
    count: *mut usize,
) -> c_int {
    let (Some(viewer), Some(count)) = (viewer.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.preference_ranges() {
        Ok(ranges) => {
            *count = ranges.len();
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One sub-range: "[t]he first and last pages in a sub-range", one-based as the entry states them.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_preference_range(
    viewer: *const Session,
    index: usize,
    first: *mut i64,
    last: *mut i64,
) -> c_int {
    let (Some(viewer), Some(first), Some(last)) = (viewer.as_ref(), first.as_mut(), last.as_mut())
    else {
        return Status::NullArgument.code();
    };
    let ranges = match viewer.preference_ranges() {
        Ok(ranges) => ranges,
        Err(status) => return status.code(),
    };
    match ranges.get(index) {
        Some(&(from, to)) => {
            *first = from;
            *last = to;
            Status::Ok.code()
        }
        None => Status::OutOfRange.code(),
    }
}

/// How many entries of Table 147 this library answers for.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_preference_key_count() -> u32 {
    PreferenceKey::COUNT
}

/// The Table 147 key a number names, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_preference_key_name(key: u32) -> *const c_char {
    let name = PreferenceKey::from_code(key).map_or("unknown\0", PreferenceKey::name);
    name.as_ptr().cast::<c_char>()
}

/// §14.3.3's Table 349 and §14.3.2's metadata stream, as a `pdfv_panel *` the caller frees.
///
/// Both tables, shown rather than merged: §14.3.4 leaves a disagreement between them "at the
/// discretion of the PDF processor", and a panel that merged them would hide one rather than
/// resolve it. Read with the `pdfv_panel_…` accessors and released with `pdfv_panel_free`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_properties_read(
    viewer: *const Session,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.properties() {
        Ok(rows) => {
            *out = Box::into_raw(Box::new(rows));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §12.4.3's article threads, as a `pdfv_panel *` the caller frees.
///
/// Every row is a `PDFV_ROW_ACTIVATE`, which is the same message an outline row sends: the
/// *document* decides what activating a thread means, and following one lands on Table 163's `/R`
/// rather than on the page its first bead sits on.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_articles_read(
    viewer: *const Session,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.articles() {
        Ok(rows) => {
            *out = Box::into_raw(Box::new(rows));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §12.4.2's label for one page, where the document states one.
///
/// `PDFV_NO_ANSWER` for a page that states none, which is most pages of most documents: §12.4.2
/// makes the integer index what identifies a page and the label an addition, so a caller falls
/// back to the number rather than to nothing.
///
/// **Separate from `pdfv_thumbnail_read` on purpose.** A caller drawing a page list needs a name
/// per row and a picture only for the rows it is showing; one call answering both would make
/// listing a thousand pages decode a thousand images.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_page_label(
    viewer: *const Session,
    page: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.page_label(page) {
        Ok(label) => copy_out(&label, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// §12.3.4's thumbnail for one page, decoded, as a handle the caller frees.
///
/// **One page at a time and no list-valued form of this call exists**, which is `CLAUDE.md`
/// section 2 obeyed by construction rather than by advice: §12.3.4's NOTE says thumbnails "are not
/// required, and can be included for some pages and not for others", a thousand-page document
/// stating one for every page would decode a thousand images to draw eight, and Table 29's
/// `/PageMode /UseThumbs` opens that panel *as the document opens*. A caller asks for the rows it
/// is about to draw.
///
/// `PDFV_NO_ANSWER` for a page with no `/Thumb`, and for one this reader could not decode.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_thumbnail_read(
    viewer: *const Session,
    page: usize,
    thumbnail: *mut *mut Miniature,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), thumbnail.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.thumbnail(page) {
        Ok(miniature) => {
            *out = Box::into_raw(Box::new(miniature));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `pdfv_thumbnail_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_thumbnail_free(thumbnail: *mut Miniature) {
    if !thumbnail.is_null() {
        drop(Box::from_raw(thumbnail));
    }
}

/// The miniature's size, how many bytes `pdfv_thumbnail_copy` writes, and §12.3.4's two
/// producer-side constraints.
///
/// The format is always `PDFV_FORMAT_RGBA8`, like every other picture this boundary hands over.
/// `permitted_colour_space` and `permitted_subtype` are the clause's constraints **carried rather
/// than enforced**: a file breaking either is wrong and its picture is still what the file says,
/// so the image is decoded either way and a caller with somewhere to put a note can say so.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_thumbnail_info(
    thumbnail: *const Miniature,
    width: *mut u32,
    height: *mut u32,
    format: *mut u32,
    bytes: *mut usize,
    permitted: *mut u32,
) -> c_int {
    let (Some(thumbnail), Some(width), Some(height), Some(bytes)) = (
        thumbnail.as_ref(),
        width.as_mut(),
        height.as_mut(),
        bytes.as_mut(),
    ) else {
        return Status::NullArgument.code();
    };
    let (across, down, len, colour_space, subtype) = thumbnail.info();
    *width = across;
    *height = down;
    *bytes = len;
    if let Some(format) = format.as_mut() {
        *format = PixelFormat::Rgba8.code();
    }
    if let Some(permitted) = permitted.as_mut() {
        // Two bits rather than two booleans, because they are two answers to one question — "is
        // this file's thumbnail dictionary the one §12.3.4 describes" — and a caller that cares
        // about neither tests the word against zero.
        *permitted = u32::from(!colour_space) | (u32::from(!subtype) << 1_u32);
    }
    Status::Ok.code()
}

/// Copies the miniature's samples into the caller's buffer, RGBA8 and row-major with no padding.
///
/// # Safety
///
/// See the module documentation. `into` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_thumbnail_copy(
    thumbnail: *const Miniature,
    into: *mut u8,
    cap: usize,
    written: *mut usize,
) -> c_int {
    let Some(thumbnail) = thumbnail.as_ref() else {
        return Status::NullArgument.code();
    };
    let (_, _, bytes, _, _) = thumbnail.info();
    if let Some(written) = written.as_mut() {
        *written = bytes;
    }
    if into.is_null() || cap < bytes {
        return Status::BufferTooSmall.code();
    }
    let room = core::slice::from_raw_parts_mut(into, cap);
    match thumbnail.copy(room) {
        Ok(_) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// How many pages on the screen have §9.10.2's counts to report. Zero for a null pointer.
///
/// `pdfv_reported_pages`'s counterpart, and one entry per page for the same reason: a column shows
/// several pages, and a caller given one page's counts for four would be silent about three.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_readback_pages(viewer: *const Session) -> usize {
    viewer.as_ref().map_or(0, Session::readback_pages)
}

/// Which page one of those entries is about, zero-based.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_readback_page(
    viewer: *const Session,
    entry: usize,
    page: *mut usize,
) -> c_int {
    let (Some(viewer), Some(page)) = (viewer.as_ref(), page.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.readback_page(entry) {
        Ok(index) => {
            *page = index;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of §9.10.2's counts for one page on the screen.
///
/// **Deliberately not a report.** §9.10.2's own closing sentence is "there is no way to determine
/// what the character code represents", so a code that route ends at is an answer the standard
/// states rather than something this program failed to do — and folding these into
/// `pdfv_report` would say the opposite. What a caller does with them is what a person needs: say
/// that a search found nothing on a page whose text cannot be read, or that a copied selection is
/// short.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_readback_count(
    viewer: *const Session,
    entry: usize,
    which: u32,
    count: *mut usize,
) -> c_int {
    let (Some(viewer), Some(count)) = (viewer.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(which) = ShortfallKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match viewer.readback_count(entry, which) {
        Ok(number) => {
            *count = number;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many of §9.10.2's counts this library distinguishes.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_shortfall_kind_count() -> u32 {
    ShortfallKind::COUNT
}

/// The name of one of those counts, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn pdfv_shortfall_kind_name(which: u32) -> *const c_char {
    let name = ShortfallKind::from_code(which).map_or("unknown\0", ShortfallKind::name);
    name.as_ptr().cast::<c_char>()
}

/// §12.5.6.14's open popup windows on the page being shown, as a handle the caller frees.
///
/// The clause makes a popup "a window … for entry and editing" with "no appearance stream", so it
/// is the one annotation subtype whose picture is *not* the page's: a caller draws it as chrome,
/// in its platform's own window furniture. Only the open ones — Table 186's `/Open` says which
/// start that way and `pdfv_activate` on the parent annotation changes it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_popups_read(
    viewer: *const Session,
    popups: *mut *mut Popups,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), popups.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.popups() {
        Ok(windows) => {
            *out = Box::into_raw(Box::new(windows));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `pdfv_popups_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_popups_free(popups: *mut Popups) {
    if !popups.is_null() {
        drop(Box::from_raw(popups));
    }
}

/// How many windows are open. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_popups_len(popups: *const Popups) -> usize {
    popups.as_ref().map_or(0, Popups::len)
}

/// The popup annotation, and Table 186's `/Parent` where it names one.
///
/// Two objects because they answer two questions: the first is what `pdfv_activate` closes the
/// window with, and the second is the markup annotation the note belongs to — which is what a
/// caller highlights when the pointer is over the window. `has_parent` is false for a popup the
/// file left unattached, which Table 186 permits.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_popup_object(
    popups: *const Popups,
    index: usize,
    number: *mut u32,
    generation: *mut u16,
    has_parent: *mut bool,
    parent_number: *mut u32,
    parent_generation: *mut u16,
) -> c_int {
    let (Some(popups), Some(number), Some(generation)) =
        (popups.as_ref(), number.as_mut(), generation.as_mut())
    else {
        return Status::NullArgument.code();
    };
    let (annotation, parent) = match popups.objects(index) {
        Ok(objects) => objects,
        Err(status) => return status.code(),
    };
    *number = annotation.0;
    *generation = annotation.1;
    if let Some(has_parent) = has_parent.as_mut() {
        *has_parent = parent.is_some();
    }
    let (parent_id, parent_generation_number) = parent.unwrap_or((0, 0));
    if let Some(parent_number) = parent_number.as_mut() {
        *parent_number = parent_id;
    }
    if let Some(parent_generation) = parent_generation.as_mut() {
        *parent_generation = parent_generation_number;
    }
    Status::Ok.code()
}

/// The window's rectangle on the screen: `[x0, y0, … x3, y3]`, y downwards, eight floats.
///
/// The same form `pdfv_quads_get`, `pdfv_focused_annotation` and `pdfv_field_widget` take, in
/// device pixels of the viewport — one arithmetic in one place.
///
/// # Safety
///
/// See the module documentation. `into` is writable for eight floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_popup_quad(
    popups: *const Popups,
    index: usize,
    into: *mut f32,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match popups.quad(index) {
        Ok(quad) => {
            core::slice::from_raw_parts_mut(into, quad.len()).copy_from_slice(&quad);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of the window's three strings: `PDFV_NOTE_TITLE`, `PDFV_NOTE_CONTENTS`, `PDFV_NOTE_MODIFIED`.
///
/// An empty string for one the annotation does not state, because Table 166 makes none of the
/// three required and a note with no title is a note a caller draws with no title.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_popup_text(
    popups: *const Popups,
    index: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = NoteKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match popups.text(index, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Table 166's `/C`, "[t]he title bar of the annotation's popup window", as three `DeviceRGB`
/// components.
///
/// `PDFV_NO_ANSWER` where the annotation states no colour, which is a different thing from black.
///
/// # Safety
///
/// See the module documentation. `into` is writable for three floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_popup_colour(
    popups: *const Popups,
    index: usize,
    into: *mut f32,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match popups.colour(index) {
        Ok(colour) => {
            core::slice::from_raw_parts_mut(into, colour.len()).copy_from_slice(&colour);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §14.7's logical structure for every page the arrangement is showing, as a handle the caller
/// frees.
///
/// **Two indices, and the standard is why.** §14.7.5.2's marked-content identifier "uniquely
/// identifies the marked-content sequence within its content stream" and §14.7.5.4 keys the route
/// in from that page's `/StructParents`, so two pages' trees share no numbering and there is no
/// order between them to renumber by. A caller walks pages with `pdfv_structure_page`, then nodes;
/// every index a node carries is into **that page's** list.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_read(
    viewer: *const Session,
    structure: *mut *mut Structure,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), structure.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.structure() {
        Ok(tree) => {
            *out = Box::into_raw(Box::new(tree));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `pdfv_structure_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_free(structure: *mut Structure) {
    if !structure.is_null() {
        drop(Box::from_raw(structure));
    }
}

/// How many pages the arrangement is showing. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_pages(structure: *const Structure) -> usize {
    structure.as_ref().map_or(0, Structure::len)
}

/// Which page an entry is about, and how many nodes its tree has.
///
/// Zero nodes is an answer rather than a silence: §14.7 leaves a producer free to state no
/// structure, and a reader that invented a reading order for an untagged page would be presenting
/// a guess where a person is entitled to the author's answer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_page(
    structure: *const Structure,
    entry: usize,
    page: *mut usize,
    nodes: *mut usize,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let (index, count) = match structure.page(entry) {
        Ok(answer) => answer,
        Err(status) => return status.code(),
    };
    if let Some(page) = page.as_mut() {
        *page = index;
    }
    if let Some(nodes) = nodes.as_mut() {
        *nodes = count;
    }
    Status::Ok.code()
}

/// A node's parent, §14.9.3's substitution, and Table 384's `/Scope` for a header cell.
///
/// `has_parent` is false for a root. `substituted` is the one a client acts on rather than
/// displays: §14.9.3 makes `/Alt` "a complete (or whole) word or phrase substitution for the
/// current element" and §14.9.5 says the same of `/E`, so an element stating one has said what to
/// speak *instead of* its content and a client stops there rather than descending — descending
/// anyway reads the element twice. `has_scope` is false for every element that is not a `TH` and
/// for a `TH` this reader could place in no grid, which is a caller being told we do not know.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_node(
    structure: *const Structure,
    entry: usize,
    node: usize,
    parent: *mut usize,
    has_parent: *mut bool,
    substituted: *mut bool,
    scope: *mut u32,
    has_scope: *mut bool,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let (encloser, substitution, header_scope) = match structure.shape(entry, node) {
        Ok(facts) => facts,
        Err(status) => return status.code(),
    };
    if let Some(parent) = parent.as_mut() {
        *parent = encloser.unwrap_or_default();
    }
    if let Some(has_parent) = has_parent.as_mut() {
        *has_parent = encloser.is_some();
    }
    if let Some(substituted) = substituted.as_mut() {
        *substituted = substitution;
    }
    if let Some(scope) = scope.as_mut() {
        *scope = header_scope.map_or(0, |scope| scope as u32);
    }
    if let Some(has_scope) = has_scope.as_mut() {
        *has_scope = header_scope.is_some();
    }
    Status::Ok.code()
}

/// One of a node's three strings: `PDFV_ELEMENT_ROLE`, `PDFV_ELEMENT_NAME`,
/// `PDFV_ELEMENT_LANGUAGE`.
///
/// The role is §14.7.4's `/S` **after §14.7.3's role map**, which is the file's own statement
/// about its own names and a `shall`: "[a] structure type shall always be mapped to its
/// corresponding name in the role map, if there is one, even if the original name is one of the
/// standard types." Mapping it onto a *platform's* vocabulary is the caller's, and is a different
/// mapping.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_text(
    structure: *const Structure,
    entry: usize,
    node: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = ElementKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match structure.text(entry, node, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Where the element's own text was drawn, as a `pdfv_quads *` the caller frees.
///
/// Empty for an element whose content drew no text — a figure, a table cell holding an image —
/// which is a statement about this program's text layer rather than about the element, and is why
/// `pdfv_structure_box` exists beside it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_quads(
    structure: *const Structure,
    entry: usize,
    node: usize,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(structure), Some(out)) = (structure.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.quads(entry, node) {
        Ok(shapes) => {
            *out = Box::into_raw(Box::new(Quads::new(shapes)));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of a node's two rectangles: `[x0, y0, x1, y1]` in device pixels of the viewport.
///
/// `PDFV_BOX_STATED` is what the **document** says the element's extent is — Table 379's `/BBox`,
/// which §14.8.5.4.3 makes "the rectangle that completely encloses its visible content" — and
/// `PDFV_BOX_DRAWN` is where **this program** drew its text. Two kinds of statement, carried side
/// by side rather than merged, because an element whose content is a picture has the first and not
/// the second. `PDFV_NO_ANSWER` where the node has no rectangle of that kind.
///
/// # Safety
///
/// See the module documentation. `into` is writable for four floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_box(
    structure: *const Structure,
    entry: usize,
    node: usize,
    which: u32,
    into: *mut f32,
) -> c_int {
    let Some(structure) = structure.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = BoxKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match structure.rectangle(entry, node, which) {
        Ok(rectangle) => {
            core::slice::from_raw_parts_mut(into, rectangle.len()).copy_from_slice(&rectangle);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many header cells §14.8.4.8.3 associates with this element.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_headers(
    structure: *const Structure,
    entry: usize,
    node: usize,
    count: *mut usize,
) -> c_int {
    let (Some(structure), Some(count)) = (structure.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.headers(entry, node) {
        Ok(number) => {
            *count = number;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of those header cells, as an index into **this page's** node list.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_header(
    structure: *const Structure,
    entry: usize,
    node: usize,
    header: usize,
    cell: *mut usize,
) -> c_int {
    let (Some(structure), Some(cell)) = (structure.as_ref(), cell.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.header(entry, node, header) {
        Ok(index) => {
            *cell = index;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many lines of text this element's own content items drew.
///
/// **What AT-SPI's `Text` interface is built on.** `PDFV_ELEMENT_NAME` is what the element is
/// *called* and these are what it *says*: a name is one string for a whole paragraph, so a client
/// can read the paragraph or not read it, and moving a caret through it by character, by word or by
/// line needs to know where each character begins and which characters share a line. That is what
/// `org.a11y.atspi.Text`'s `GetCharacterExtents`, `GetOffsetAtPoint` and `GetTextAtOffset` ask for
/// and what no string can answer.
///
/// **Not §14.9's substitutions**, deliberately, and the difference is the point: `PDFV_ELEMENT_NAME`
/// applies §14.9.3's `/Alt` and §14.9.5's `/E`, and this does not — a caret moves over what is on
/// the page, and a phrase that substitutes for the content has no glyphs to report positions for.
/// Zero lines for an element that states one, which is also what `pdfv_structure_node`'s
/// `substituted` says, and zero for an element whose content drew no text.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_lines(
    structure: *const Structure,
    entry: usize,
    node: usize,
    count: *mut usize,
) -> c_int {
    let (Some(structure), Some(count)) = (structure.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.lines(entry, node) {
        Ok(number) => {
            *count = number;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One line's text, with the number of character codes that produced it.
///
/// The text follows this ABI's string convention — `out` and `cap`, with `needed` receiving the
/// length including the terminator — and `characters` receives the count in the same call, because
/// the invariant between them is what a text interface rests on: the character byte counts sum to
/// the text's length, so an offset into the string and an index into the characters convert into
/// each other without either side guessing.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_line(
    structure: *const Structure,
    entry: usize,
    node: usize,
    line: usize,
    characters: *mut usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let (Some(structure), Some(characters)) = (structure.as_ref(), characters.as_mut()) else {
        return Status::NullArgument.code();
    };
    match structure.line(entry, node, line) {
        Ok((text, codes)) => {
            *characters = codes;
            copy_out(text, out, cap, needed)
        }
        Err(status) => status.code(),
    }
}

/// One character code's share of a line: its byte count, and where its glyph is.
///
/// `bytes` is how much of the line's text this code produced — **the unit is the code and not the
/// character**, because a code mapped through `/ToUnicode` to a several-character string drew one
/// glyph in one place and splitting its box would invent positions the file does not state.
/// `into` receives `[x0, y0, x1, y1]` in the device pixels of the viewport, the same space every
/// other shape in this ABI is in.
///
/// # Safety
///
/// See the module documentation. `into` is writable for four floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_structure_character(
    structure: *const Structure,
    entry: usize,
    node: usize,
    line: usize,
    character: usize,
    bytes: *mut usize,
    into: *mut f32,
) -> c_int {
    let (Some(structure), Some(bytes)) = (structure.as_ref(), bytes.as_mut()) else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match structure.character(entry, node, line, character) {
        Ok((produced, rectangle)) => {
            *bytes = produced;
            core::slice::from_raw_parts_mut(into, rectangle.len()).copy_from_slice(&rectangle);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §12.3.5's portable collection, as a handle the caller frees.
///
/// The clause is a `shall` on a viewer — "[i]f this dictionary is present in a PDF document, the
/// interactive PDF processor shall present the document as a portable collection" — so what
/// crosses is everything needed to *arrange* the files `pdfv_attachments_read` already lists:
/// Table 153's `/View`, §12.3.5.1's resolved initial document, Table 155's columns in `/O` order,
/// and §12.3.5.2's folder tree flattened depth first. `pdfv_collection_folder_of` is the fifth
/// piece and the one a caller could not compute for itself.
///
/// `PDFV_NO_ANSWER` where the catalogue states no collection, which is every document in this
/// project's corpora.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_read(
    viewer: *const Session,
    collection: *mut *mut Collection,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), collection.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.collection() {
        Ok(read) => {
            *out = Box::into_raw(Box::new(read));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `pdfv_collection_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_free(collection: *mut Collection) {
    if !collection.is_null() {
        drop(Box::from_raw(collection));
    }
}

/// Table 153's `/View`: how the collection is first presented.
///
/// `PDFV_COLLECTION_HIDDEN` is the one value that is load-bearing rather than a preference:
/// §7.6.7's unencrypted wrapper document requires it, because the wrapper's own page says the
/// payload is encrypted and showing a file browser over it would hide that.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_view(
    collection: *const Collection,
    view: *mut u32,
) -> c_int {
    let (Some(collection), Some(view)) = (collection.as_ref(), view.as_mut()) else {
        return Status::NullArgument.code();
    };
    *view = collection.view() as u32;
    Status::Ok.code()
}

/// §12.3.5.1's outcome for `/D`, resolved, and the `/EmbeddedFiles` key where it names a file.
///
/// A *resolved* answer rather than the entry: Table 153's `/D` "identifies an entry in the
/// `EmbeddedFiles` name tree, determining the document that shall be initially presented in the
/// user interface", the tree is the document's, and turning a byte string into one of four
/// outcomes is therefore not a caller's to do. The key is empty for the three outcomes that name
/// no file, and is what `pdfv_extract` takes for the one that does.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_initial(
    collection: *const Collection,
    kind: *mut u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let (outcome, name) = collection.initial();
    if let Some(kind) = kind.as_mut() {
        *kind = outcome as u32;
    }
    copy_out(name, out, cap, needed)
}

/// How many columns Table 153's `/Schema` states.
///
/// Zero is a permission rather than a gap: the table says an absent schema lets a processor
/// "choose useful defaults that are known to exist in a file specification dictionary, such as the
/// file name, file size, and modified date", so an empty list is the document declining to choose.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_columns(
    collection: *const Collection,
    count: *mut usize,
) -> c_int {
    let (Some(collection), Some(count)) = (collection.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    *count = collection.columns();
    Status::Ok.code()
}

/// One column's subtype, where its value lives, Table 155's `/O`, `/V` and `/E`.
///
/// The columns are already in `/O` order — "[t]he relative order of the field name in the user
/// interface" — with a field stating none after every field that states one, which is the only
/// order left when the file says nothing. `in_the_item` is the clause's own division stated rather
/// than left to be derived: the first three subtypes "identify the types of fields in the
/// collection item … dictionary" and the rest "identify the types of file-related fields", so it
/// says whether a caller reads §7.11.6's `/CI` or the file specification it already has.
/// `has_order` is false where the field states no `/O`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_column(
    collection: *const Collection,
    index: usize,
    kind: *mut u32,
    in_the_item: *mut bool,
    order: *mut i64,
    has_order: *mut bool,
    flags: *mut u32,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let (subtype, item, stated_order, visible, editable) = match collection.column(index) {
        Ok(facts) => facts,
        Err(status) => return status.code(),
    };
    if let Some(kind) = kind.as_mut() {
        *kind = subtype as u32;
    }
    if let Some(in_the_item) = in_the_item.as_mut() {
        *in_the_item = item;
    }
    if let Some(order) = order.as_mut() {
        *order = stated_order.unwrap_or_default();
    }
    if let Some(has_order) = has_order.as_mut() {
        *has_order = stated_order.is_some();
    }
    if let Some(flags) = flags.as_mut() {
        *flags = u32::from(visible) | (u32::from(editable) << 1_u32);
    }
    Status::Ok.code()
}

/// One of a column's three strings: `PDFV_COLUMN_NAME`, `PDFV_COLUMN_KEY`, `PDFV_COLUMN_SUBTYPE`.
///
/// The third is the whole of what `PDFV_COLLECTION_FIELD_OTHER` leaves to say: a subtype this
/// standard does not define is still a name the file wrote, and a number a caller cannot resolve
/// beside a name it cannot read would be a silent fallback in a header.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_column_text(
    collection: *const Collection,
    index: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = ColumnTextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match collection.column_text(index, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// How many folders §12.3.5.2's tree holds, counting every level.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_folders(
    collection: *const Collection,
    count: *mut usize,
) -> c_int {
    let (Some(collection), Some(count)) = (collection.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    *count = collection.folders();
    Status::Ok.code()
}

/// One folder's `/ID`, its depth in the tree, and whether it states a `/Thumb`.
///
/// Depth first with a depth on each row, exactly as §12.3.3's outline crosses and for the same
/// reason: a tree is the one shape a C ABI cannot hand over as itself. The `/ID` is "a
/// non-negative integer value representing the unique folder identification number", and it is
/// what `pdfv_collection_folder_of` answers with.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_folder(
    collection: *const Collection,
    index: usize,
    id: *mut u32,
    depth: *mut u32,
    has_thumbnail: *mut bool,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let (number, level, thumbnail) = match collection.folder(index) {
        Ok(facts) => facts,
        Err(status) => return status.code(),
    };
    if let Some(id) = id.as_mut() {
        *id = number;
    }
    if let Some(depth) = depth.as_mut() {
        *depth = level;
    }
    if let Some(has_thumbnail) = has_thumbnail.as_mut() {
        *has_thumbnail = thumbnail;
    }
    Status::Ok.code()
}

/// One of a folder's two strings: `PDFV_FOLDER_NAME`, `PDFV_FOLDER_DESCRIPTION`.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_folder_text(
    collection: *const Collection,
    index: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(collection) = collection.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = FolderTextKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match collection.folder_text(index, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Which folder an `/EmbeddedFiles` key names, and the file name inside it.
///
/// **The one piece of §12.3.5 a caller could not compute**, and the reason this is a function
/// rather than a note in the header. §12.3.5.2 gives the key of a file in a folder as the folder's
/// identification number in angle brackets followed by the file name; a caller holding a folder
/// tree and a file list has no way to put one inside the other without that grammar. A key with no
/// such prefix is a file at the root, which answers `PDFV_NO_ANSWER` and copies the key unchanged.
///
/// It takes no viewer, because it is a fact about a string rather than about a document.
///
/// # Safety
///
/// See the module documentation. `key` is NUL-terminated and UTF-8; `out` is writable for `cap`
/// bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfv_collection_folder_of(
    key: *const c_char,
    id: *mut u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    if key.is_null() {
        return Status::NullArgument.code();
    }
    let Ok(key) = core::ffi::CStr::from_ptr(key).to_str() else {
        return Status::NotUtf8.code();
    };
    let Some((folder, name)) = pdf_model::collection::folder_of(key) else {
        // The key names no folder, so the file is at the root. The name is still copied — it is
        // the key itself — so that a caller may write one loop rather than two.
        let written = copy_out(key, out, cap, needed);
        return if written == Status::Ok.code() {
            Status::NoAnswer.code()
        } else {
            written
        };
    };
    if let Some(id) = id.as_mut() {
        *id = folder;
    }
    copy_out(name, out, cap, needed)
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
