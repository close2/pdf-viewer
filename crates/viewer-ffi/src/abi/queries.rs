//! The synchronous queries that produce no events, and the first reads of the document's state.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::layouts::{PdfvFrame, PdfvGeometry, PdfvViewing};
use super::shared::{copy_out, owned_text};
use crate::answers::Matches;
use crate::events::Events;
use crate::kinds::{ControlKind, PageModeKind, PixelFormat, PreferenceKey, RowKind, ZoomKind};
use crate::panels::Panel;
use crate::session::{FrameInfo, Session};
use crate::shapes::Quads;
use crate::status::Status;
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// Queries. Synchronous, producing no events, exactly as `Viewer::query` is.
// ---------------------------------------------------------------------------------------------

/// How many pages the focused document has.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_page_count(viewer: *const Session, pages: *mut usize) -> c_int {
    let (Some(viewer), Some(pages)) = (viewer.as_ref(), pages.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().page_count() {
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
pub unsafe extern "C" fn quorra_current_page(
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
/// [`Status::NoAnswer`] when no document is focused. What it is for is `quorra_set_view`: the
/// commands that make a view are relative and clamped, so a caller that issued every one of them
/// still cannot say where the reader ended up.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_view(viewer: *const Session, view: *mut PdfvViewing) -> c_int {
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

/// Puts the reader back at a view `quorra_view` answered with.
///
/// [`Status::WrongKind`] for a magnification code this build does not define, which is what every
/// other entry point taking a kind answers.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_set_view(
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
pub unsafe extern "C" fn quorra_page_geometry(
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
/// fail to compile: `quorra_frame_info` and `quorra_frame_copy` gained an index, and a caller has to
/// be able to learn how many there are. Zero where the viewer holds none.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_frame_count(viewer: *const Session) -> usize {
    viewer.as_ref().map_or(0, Session::frame_count)
}

/// What the viewer is holding for one page of the arrangement, without the pixels.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_frame_info(
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
/// [`quorra_frame_info`]'s `bytes`; anything shorter answers [`Status::BufferTooSmall`] and writes
/// nothing, rather than leaving a partial page in it.
///
/// # Safety
///
/// See the module documentation. `into` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_frame_copy(
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

/// Draws one page of the print operation `quorra_print` began, and says how large it is.
///
/// **The first half of C's two-call idiom, and the drawing is on this side of it**: the answer to
/// `viewer_core::Query::PrintPage` is a display list, which is not a thing to put in a header, so
/// the library rasterises it with the processor backend — the same backend that is this project's
/// correctness oracle, which is the whole argument for rendering the printed page here rather than
/// handing the file to a spooler. The pixels wait until `quorra_print_page_copy` takes them, and
/// one page waits at a time.
///
/// `*width`, `*height` and `*bytes` are filled where they are not null. The layout is
/// `QUORRA_FORMAT_RGBA8`, as every raster in this ABI is.
///
/// `QUORRA_NO_ANSWER` where no print operation is running — `quorra_print` is where §7.6.4.2's
/// bit 3 is asked, and nothing prints without it — or where the document has no such page.
/// `QUORRA_RENDER_REFUSED` where the page's marks will not go onto a raster at the job's
/// resolution; `quorra_printed_report` says why in both of those cases where anything was said.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_print_page(
    viewer: *mut Session,
    page: usize,
    width: *mut u32,
    height: *mut u32,
    bytes: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_mut() else {
        return Status::NullArgument.code();
    };
    match viewer.print_page(page) {
        Ok((drawn_width, drawn_height, drawn_bytes)) => {
            if let Some(width) = width.as_mut() {
                *width = drawn_width;
            }
            if let Some(height) = height.as_mut() {
                *height = drawn_height;
            }
            if let Some(bytes) = bytes.as_mut() {
                *bytes = drawn_bytes;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Copies the page `quorra_print_page` drew into a buffer the caller owns.
///
/// One copy, which is what tier 1 costs everywhere in this project. Size the buffer from that
/// call's `bytes`; anything shorter answers `QUORRA_BUFFER_TOO_SMALL` and writes nothing.
/// `QUORRA_NO_ANSWER` where no page has been drawn, or where the page drawn is a different one.
///
/// # Safety
///
/// See the module documentation. `into` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_print_page_copy(
    viewer: *const Session,
    page: usize,
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
    match viewer.print_page_copy(page, room) {
        Ok(bytes) => {
            if let Some(written) = written.as_mut() {
                *written = bytes;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many things could not be drawn on the page `quorra_print_page` drew.
///
/// Trap 5's channel for a printed page: a page that goes to the printer with something missing
/// says so, rather than coming out short in silence. Zero where nothing was owed, and zero where
/// no page has been drawn.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_printed_reports(
    viewer: *const Session,
    count: *mut usize,
) -> c_int {
    let (Some(viewer), Some(count)) = (viewer.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    *count = viewer.printed_reports().len();
    Status::Ok.code()
}

/// One of those sentences, by index, as UTF-8 with a trailing NUL.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_printed_report(
    viewer: *const Session,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(report) = viewer.printed_reports().get(index) else {
        return Status::OutOfRange.code();
    };
    copy_out(report, out, cap, needed)
}

// ---------------------------------------------------------------------------------------------
// What the page could not draw, and the names of the two answered enumerations.
// ---------------------------------------------------------------------------------------------

/// How many pages on the screen this viewer has anything to say about.
///
/// **Table 29's arrangement, counted a second time**, and it is here for the reason
/// [`quorra_frame_count`] is: a C consumer cannot fail to compile, so a `/PageLayout` putting four
/// pages on the screen has to be something a caller *asks* about rather than something it is
/// silently given one quarter of. Zero where no document is focused or no page has been read;
/// one under `SinglePage`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_reported_pages(viewer: *const Session) -> usize {
    viewer.as_ref().map_or(0, Session::reported_pages)
}

/// Which page one of those entries is about, zero-based.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_reported_page(
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
/// The same sentences a `QUORRA_EVENT_REPORTED` carried, kept so that a caller which cleared its
/// status bar can ask again rather than remembering. Trap 5's channel: every layer of this program
/// says what it could not handle rather than falling back to something plausible.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_reports_len(
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
pub unsafe extern "C" fn quorra_report(
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
/// The pair `quorra_event_kind_count` and `quorra_event_kind_name` make for an event, and for the same
/// reason: a number a caller has no arm for should still be printable. **Not part of
/// [`quorra_abi_check`](super::quorra_abi_check)**, deliberately — an event *arrives* whether or not the caller asked, so its
/// count has to be checked before the first one turns up; a control kind is the answer to a call
/// the caller wrote.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_control_kind_count() -> u32 {
    ControlKind::COUNT
}

/// The name of a control kind, NUL-terminated and never freed. `"unknown"` for one this build does
/// not define.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_control_kind_name(kind: u32) -> *const c_char {
    let name = ControlKind::from_code(kind).map_or("unknown\0", ControlKind::name);
    name.as_ptr().cast::<c_char>()
}

/// How many panel row actions this library defines.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_row_kind_count() -> u32 {
    RowKind::COUNT
}

/// The name of a panel row action, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_row_kind_name(kind: u32) -> *const c_char {
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
// No entry point below takes or returns a struct by value. `QUORRA_ABI_VERSION` therefore does not
// move, which is the whole reason the shapes are handles and out-parameters.
// ---------------------------------------------------------------------------------------------

/// Every occurrence of a string on the page being shown, as shapes to draw over it.
///
/// `quorra_find_start` searches the *document* one page per step; this answers for the page in front
/// of the reader, out of a readback that already exists, so a find bar may ask it on every repaint.
/// A caller had the first and not the second until ADR 0576.
///
/// # Safety
///
/// See the module documentation. `needle` is NUL-terminated and UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_find_matches(
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
    match viewer.reads().find(&needle) {
        Ok(found) => {
            *out = Box::into_raw(Box::new(found));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `quorra_find_matches` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_matches_free(matches: *mut Matches) {
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
pub unsafe extern "C" fn quorra_matches_len(matches: *const Matches) -> usize {
    matches.as_ref().map_or(0, Matches::len)
}

/// The shapes covering one occurrence, as a `quorra_quads *` the caller frees.
///
/// **One occurrence is several quadrilaterals**, because a term wrapped across a line is merged
/// per run of a line — so *next match* is this index and never the next shape.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_matches_quads(
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
/// `quorra_layout` sets the arrangement a *reader* chose; this is the one the *document* opens in,
/// and a caller had no way to ask for it. Both native hosts obey the catalogue on open.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_opening(
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
/// The pair `quorra_control_kind_count` and `quorra_control_kind_name` make, for the same reason and
/// deliberately not in `quorra_abi_check`: this is the answer to a call the caller wrote. Table 29
/// gained `/UseOC` in PDF 1.5 and `/UseAttachments` in PDF 1.6, which is why it is counted at all.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_page_mode_count() -> u32 {
    PageModeKind::COUNT
}

/// The name of a page mode, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_page_mode_name(mode: u32) -> *const c_char {
    let name = PageModeKind::from_code(mode).map_or("unknown\0", PageModeKind::name);
    name.as_ptr().cast::<c_char>()
}

/// One entry of §12.2's Table 147, as a number.
///
/// **One keyed accessor rather than nineteen symbols or a struct**, and the argument is this
/// module's own transposed from a command to a table: a struct passed by value would put Table
/// 147's size in the ABI, and a symbol apiece would be nineteen exports for one table. An entry
/// added by a later part of ISO 32000 is a new `QUORRA_PREF_…` constant beside a function every
/// compiled caller already links.
///
/// A boolean answers zero or one, an enumerated name answers its own `QUORRA_…` number, and a count
/// answers itself. `QUORRA_NO_ANSWER` is the three entries Table 147 leaves genuinely open —
/// `/Duplex`, `/PickTrayByPDFSize` and `/NumCopies` — where the document states none, because
/// "the document says nothing" and "the document says the default" are different facts.
/// `QUORRA_WRONG_KIND` is `QUORRA_PREF_PRINT_PAGE_RANGE`, which is a list: see
/// `quorra_preference_ranges`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_preference(
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
pub unsafe extern "C" fn quorra_preference_ranges(
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

/// One sub-range: "\[t]he first and last pages in a sub-range", one-based as the entry states them.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_preference_range(
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
pub extern "C" fn quorra_preference_key_count() -> u32 {
    PreferenceKey::COUNT
}

/// The Table 147 key a number names, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_preference_key_name(key: u32) -> *const c_char {
    let name = PreferenceKey::from_code(key).map_or("unknown\0", PreferenceKey::name);
    name.as_ptr().cast::<c_char>()
}

/// §14.3.3's Table 349 and §14.3.2's metadata stream, as a `quorra_panel *` the caller frees.
///
/// Both tables, shown rather than merged: §14.3.4 leaves a disagreement between them "at the
/// discretion of the PDF processor", and a panel that merged them would hide one rather than
/// resolve it. Read with the `quorra_panel_…` accessors and released with `quorra_panel_free`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_properties_read(
    viewer: *const Session,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().properties() {
        Ok(rows) => {
            *out = Box::into_raw(Box::new(rows));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §12.4.3's article threads, as a `quorra_panel *` the caller frees.
///
/// Every row is a `QUORRA_ROW_ACTIVATE`, which is the same message an outline row sends: the
/// *document* decides what activating a thread means, and following one lands on Table 163's `/R`
/// rather than on the page its first bead sits on.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_articles_read(
    viewer: *const Session,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().articles() {
        Ok(rows) => {
            *out = Box::into_raw(Box::new(rows));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}
