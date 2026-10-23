//! Reads about pages: labels, measures, thumbnails, read-back and the shortfall names.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::copy_out;
use crate::answers::Miniature;
use crate::kinds::{PixelFormat, ShortfallKind};
use crate::session::Session;
use crate::status::Status;
use core::ffi::{CStr, c_char, c_int};

/// §12.4.2's label for one page, where the document states one.
///
/// `QUORRA_NO_ANSWER` for a page that states none, which is most pages of most documents: §12.4.2
/// makes the integer index what identifies a page and the label an addition, so a caller falls
/// back to the number rather than to nothing.
///
/// **Separate from `quorra_thumbnail_read` on purpose.** A caller drawing a page list needs a name
/// per row and a picture only for the rows it is showing; one call answering both would make
/// listing a thousand pages decode a thousand images.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_page_label(
    viewer: *const Session,
    page: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.reads().page_label(page) {
        Ok(label) => copy_out(&label, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// §12.9's measurement of a path a person traced across the page.
///
/// `points` is `count` pairs of device pixels of the viewport, in the order they were put down,
/// and `part` chooses what comes back: `0` is the whole sentence — the only form §12.10's
/// geospatial reading crosses in, because every part of that reading is a statement about the map
/// rather than a quantity — and `1` to `6` are the viewport's name, the drawing's scale ratio, and
/// the strings §12.9.2's algorithm produced from Table 267's `/D`, `/A`, `/T` and `/S`.
///
/// `QUORRA_NO_ANSWER` where the page states no viewport containing the first point, where the path
/// leaves the page it started on, and where the document states nothing for the part asked for.
///
/// # Safety
///
/// See the module documentation. `points` must name `count` pairs of `float` or be null with a
/// `count` of zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_measure(
    viewer: *const Session,
    points: *const f32,
    count: usize,
    part: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    if points.is_null() && count != 0 {
        return Status::NullArgument.code();
    }
    let Some(pairs) = count.checked_mul(2) else {
        return Status::OutOfRange.code();
    };
    let path: Vec<[f32; 2]> = if count == 0 {
        Vec::new()
    } else {
        core::slice::from_raw_parts(points, pairs)
            .chunks_exact(2)
            .map(|pair| [pair[0], pair[1]])
            .collect()
    };
    match viewer.measure(&path, part) {
        Ok(said) => copy_out(&said, out, cap, needed),
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
/// `QUORRA_NO_ANSWER` for a page with no `/Thumb`, and for one this reader could not decode.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_thumbnail_read(
    viewer: *const Session,
    page: usize,
    thumbnail: *mut *mut Miniature,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), thumbnail.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().thumbnail(page) {
        Ok(miniature) => {
            *out = Box::into_raw(Box::new(miniature));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §12.3.6's preview picture for one of §12.3.5's attachments, by its `/EmbeddedFiles` key.
///
/// The picture is that attachment's own first page's §12.3.4 `/Thumb` — the miniature its
/// producer wrote — because that is the one picture of a document's page the standard defines, and
/// Table 160's `FilmStrip`, `FreeForm` and `Linear` are built out of pictures of the attachments
/// (ADR 1251).
///
/// **The same handle `quorra_thumbnail_read` produces**, so `quorra_thumbnail_info`,
/// `quorra_thumbnail_copy` and `quorra_thumbnail_free` serve both: one picture idiom rather than
/// two. `QUORRA_NO_ANSWER` where the tree holds no such key, and where the attachment is not a PDF
/// or states no `/Thumb` on its first page — which is most attachments, and is a document being
/// quiet rather than a failure.
///
/// **One attachment at a time and no list-valued form of this call exists**, which is
/// `CLAUDE.md` section 2 by construction, exactly as for a page's thumbnail: a preview is an
/// embedded document opened, and a caller asks for the rows it is about to draw.
///
/// # Safety
///
/// See the module documentation. `name` is a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_attachment_preview_read(
    viewer: *const Session,
    name: *const c_char,
    thumbnail: *mut *mut Miniature,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), thumbnail.as_mut()) else {
        return Status::NullArgument.code();
    };
    if name.is_null() {
        return Status::NullArgument.code();
    }
    let Ok(name) = CStr::from_ptr(name).to_str() else {
        return Status::NotUtf8.code();
    };
    match viewer.attachment_preview(name) {
        Ok(miniature) => {
            *out = Box::into_raw(Box::new(miniature));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `quorra_thumbnail_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_thumbnail_free(thumbnail: *mut Miniature) {
    if !thumbnail.is_null() {
        drop(Box::from_raw(thumbnail));
    }
}

/// The miniature's size, how many bytes `quorra_thumbnail_copy` writes, and §12.3.4's two
/// producer-side constraints.
///
/// The format is always `QUORRA_FORMAT_RGBA8`, like every other picture this boundary hands over.
/// `permitted_colour_space` and `permitted_subtype` are the clause's constraints **carried rather
/// than enforced**: a file breaking either is wrong and its picture is still what the file says,
/// so the image is decoded either way and a caller with somewhere to put a note can say so.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_thumbnail_info(
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
pub unsafe extern "C" fn quorra_thumbnail_copy(
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
/// `quorra_reported_pages`'s counterpart, and one entry per page for the same reason: a column shows
/// several pages, and a caller given one page's counts for four would be silent about three.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_readback_pages(viewer: *const Session) -> usize {
    viewer.as_ref().map_or(0, Session::readback_pages)
}

/// Which page one of those entries is about, zero-based.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_readback_page(
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
/// `quorra_report` would say the opposite. What a caller does with them is what a person needs: say
/// that a search found nothing on a page whose text cannot be read, or that a copied selection is
/// short.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_readback_count(
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
pub extern "C" fn quorra_shortfall_kind_count() -> u32 {
    ShortfallKind::COUNT
}

/// The name of one of those counts, NUL-terminated and never freed.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_shortfall_kind_name(which: u32) -> *const c_char {
    let name = ShortfallKind::from_code(which).map_or("unknown\0", ShortfallKind::name);
    name.as_ptr().cast::<c_char>()
}
