//! The identity of the ABI: the revision, the event-kind count and the names of statuses and kinds. None takes a pointer.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::layouts::QUORRA_ABI_VERSION;
use crate::kinds::EventKind;
use crate::status::Status;
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// The identity of the ABI. None of these takes a pointer, so none of them is `unsafe`.
// ---------------------------------------------------------------------------------------------

/// The revision this library was built at.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_abi_version() -> u32 {
    QUORRA_ABI_VERSION
}

/// How many event kinds this library defines.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_event_kind_count() -> u32 {
    EventKind::COUNT
}

/// Whether a caller's header agrees with this library.
///
/// **The answer to "a variant added later", and the only one C admits.** A caller passes its own
/// `QUORRA_ABI_VERSION` and `QUORRA_EVENT_KIND_COUNT`; a mismatch answers [`Status::OutOfRange64`],
/// which is this boundary saying "a number moved" in the one channel it has. A caller that runs
/// this in `main` has turned "fails to compile in every consumer" into "fails to start, once,
/// naming what changed" — weaker than the Rust rule, and the strongest thing available.
///
/// **A larger count in the library is not an error and is deliberately not treated as one.** New
/// kinds only ever appear at the end, so an old caller meets one exactly when it receives one,
/// and `quorra_events_describe` is what it does about it then. What this catches is the caller
/// built against a *newer* header than the library it found.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_abi_check(version: u32, event_kinds: u32) -> c_int {
    if version != QUORRA_ABI_VERSION || event_kinds > EventKind::COUNT {
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
pub extern "C" fn quorra_status_message(status: c_int) -> *const c_char {
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
        9 => Status::Unreadable.message(),
        _ => "a status this build does not define\0",
    };
    message.as_ptr().cast::<c_char>()
}

/// The name of an event kind, NUL-terminated and never freed.
///
/// `"unknown"` for a kind this build does not define, which is what an *old* library answers a
/// *new* caller. The other direction — a new library and an old caller — is the one that matters
/// and is handled by `quorra_events_describe`.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_event_kind_name(kind: u32) -> *const c_char {
    let name = EventKind::from_code(kind).map_or("unknown\0", EventKind::name);
    name.as_ptr().cast::<c_char>()
}
