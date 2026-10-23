//! ISO 19005's conversion: the permission asked, the converter run, the report read back.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::{copy_out, owned_text};
use crate::events::Events;
use crate::session::Session;
use crate::status::Status;
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// ISO 19005's conversion: Table 22's bit 11 asked, the converter run here, the report read back.
// ---------------------------------------------------------------------------------------------

/// How many profiles this library ships for `quorra_archive`, which are
/// `pdf_transform::profiles::SHIPPED` — the command line's `--profile` list (ADR 1954).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archive_profiles(count: *mut usize) -> c_int {
    let Some(count) = count.as_mut() else {
        return Status::NullArgument.code();
    };
    *count = pdf_transform::profiles::SHIPPED.len();
    Status::Ok.code()
}

/// One shipped profile's name, by index, as UTF-8 with a trailing NUL.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archive_profile(
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(shipped) = pdf_transform::profiles::SHIPPED.get(index) else {
        return Status::OutOfRange.code();
    };
    copy_out(shipped.name, out, cap, needed)
}

/// One shipped profile's sentence about itself, by index, as UTF-8 with a trailing NUL — what the
/// windows offer beside the name when a person chooses (ADR 1966), so a C caller's chooser can say
/// the same thing.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archive_profile_sentence(
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(offered) = viewer_host::archiving::offered().into_iter().nth(index) else {
        return Status::OutOfRange.code();
    };
    copy_out(&offered.sentence, out, cap, needed)
}

/// How many targets ISO 19005's conversion writes, in the order every window's chooser offers
/// them — `pdf_archive::Target::ALL` (ADRs 1966, 2121).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archive_targets(count: *mut usize) -> c_int {
    let Some(count) = count.as_mut() else {
        return Status::NullArgument.code();
    };
    *count = pdf_archive::Target::ALL.len();
    Status::Ok.code()
}

/// One target by index, as every window's chooser names it (`PDF/A-2b`, …) and `quorra_archive`
/// takes it, as UTF-8 with a trailing NUL.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archive_target(
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(target) = pdf_archive::Target::ALL.get(index) else {
        return Status::OutOfRange.code();
    };
    copy_out(&target.to_string(), out, cap, needed)
}

/// Asks for the focused document as ISO 19005 (PDF/A) at `target` — `2b`, `2u`, `2a`, `4`, `4f`
/// or `4e` — under the shipped profile `profile`.
///
/// Table 22's bit 11 is asked at the reader's level, as `quorra-transform archive` asks it. The
/// grant arrives as a `QUORRA_EVENT_ARCHIVING`, and **the conversion has run before this returns**
/// — on the calling thread, so a caller that wants it elsewhere calls from elsewhere; under
/// `QUORRA_RESTRICT_ASK` it runs inside the `quorra_answer` that says yes. `quorra_archived` then
/// says how it went. The file is the caller's to write: this library opens no path.
///
/// `QUORRA_OUT_OF_RANGE` where `target` names no target this converter writes. A profile this
/// library does not ship is not refused here: the conversion says so in its report, naming the
/// ones it does (ADR 1954).
///
/// # Safety
///
/// See the module documentation. `target` and `profile` are NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archive(
    viewer: *mut Session,
    target: *const c_char,
    profile: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let (Ok(Some(target)), Ok(Some(profile))) = (owned_text(target), owned_text(profile)) else {
        return if target.is_null() || profile.is_null() {
            Status::NullArgument.code()
        } else {
            Status::NotUtf8.code()
        };
    };
    let Some(target) = pdf_archive::Target::parse(&target) else {
        return Status::OutOfRange.code();
    };
    *events = Box::into_raw(Box::new(
        viewer.archive(viewer_core::Archiving { target, profile }),
    ));
    Status::Ok.code()
}

/// How the last conversion went: the converted file's length — zero where none was written —
/// and how many lines its report has.
///
/// `QUORRA_NO_ANSWER` where no conversion has been granted in this session.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archived(
    viewer: *const Session,
    bytes: *mut usize,
    lines: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(converted) = viewer.archived() else {
        return Status::NoAnswer.code();
    };
    if let Some(bytes) = bytes.as_mut() {
        *bytes = converted.written.as_ref().map_or(0, Vec::len);
    }
    if let Some(lines) = lines.as_mut() {
        *lines = converted.lines.len();
    }
    Status::Ok.code()
}

/// Copies the converted file into a buffer the caller owns.
///
/// Size it from `quorra_archived`'s `bytes`; anything shorter answers `QUORRA_BUFFER_TOO_SMALL`
/// and writes nothing. `QUORRA_NO_ANSWER` where no file was written.
///
/// # Safety
///
/// See the module documentation. `into` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archived_copy(
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
    let Some(file) = viewer
        .archived()
        .and_then(|converted| converted.written.as_ref())
    else {
        return Status::NoAnswer.code();
    };
    let room = core::slice::from_raw_parts_mut(into, cap);
    let Some(room) = room.get_mut(..file.len()) else {
        return Status::BufferTooSmall.code();
    };
    room.copy_from_slice(file);
    if let Some(written) = written.as_mut() {
        *written = file.len();
    }
    Status::Ok.code()
}

/// One line of the last conversion's report, by index, as UTF-8 with a trailing NUL: the profile
/// once, then each site, then anything the run declined.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archived_line(
    viewer: *const Session,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(converted) = viewer.archived() else {
        return Status::NoAnswer.code();
    };
    let Some(line) = converted.lines.get(index) else {
        return Status::OutOfRange.code();
    };
    copy_out(line, out, cap, needed)
}

/// The last conversion's report as RFC 0002 section 4.5's JSON, with its `profile` field, as UTF-8
/// with a trailing NUL.
///
/// `QUORRA_NO_ANSWER` where no conversion was granted, or where it stopped before a report —
/// `quorra_archived_line` says why.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_archived_json(
    viewer: *const Session,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(json) = viewer
        .archived()
        .and_then(|converted| converted.json.as_deref())
    else {
        return Status::NoAnswer.code();
    };
    copy_out(json, out, cap, needed)
}
