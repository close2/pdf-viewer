//! The helpers every entry point shares. Each is `unsafe fn` or a small plain helper, none exported.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use crate::kinds::{AlertButtonKind, ScriptAnswerKind};
use crate::status::Status;
use core::ffi::{CStr, c_char, c_int};

/// A script's answer from `QUORRA_SCRIPT_ANSWER_*`, a `QUORRA_ALERT_BUTTON_*` and the typed text
/// as read — `quorra_answer_script`'s and `quorra_window_answer_script`'s one reading.
pub(super) fn script_answer(
    how: u32,
    button: u32,
    text: Result<Option<String>, ()>,
) -> Result<viewer_core::ScriptAnswer, Status> {
    match ScriptAnswerKind::from_code(how) {
        Some(ScriptAnswerKind::Pressed) => AlertButtonKind::from_code(button)
            .map(|button| viewer_core::ScriptAnswer::Pressed(button.button()))
            .ok_or(Status::WrongKind),
        Some(ScriptAnswerKind::Typed) => text
            .map(viewer_core::ScriptAnswer::Typed)
            .map_err(|()| Status::NotUtf8),
        Some(ScriptAnswerKind::Unanswerable) => Ok(viewer_core::ScriptAnswer::Unanswerable),
        None => Err(Status::WrongKind),
    }
}

/// A script question's numbers written into whichever of the five out-parameters the caller gave,
/// what does not apply as 0 — `quorra_event_script_question`'s and `quorra_owed_script_question`'s
/// one shape.
pub(super) fn write_script_question(
    question: crate::ScriptQuestionNumbers,
    kind: Option<&mut u32>,
    icon: Option<&mut u32>,
    buttons: Option<&mut u32>,
    dismissed: Option<&mut u32>,
    password: Option<&mut bool>,
) {
    if let Some(kind) = kind {
        *kind = question.kind as u32;
    }
    if let Some(icon) = icon {
        *icon = question.icon.map_or(0, |icon| icon as u32);
    }
    if let Some(buttons) = buttons {
        *buttons = question.buttons.map_or(0, |buttons| buttons as u32);
    }
    if let Some(dismissed) = dismissed {
        *dismissed = question.dismissed.map_or(0, |button| button as u32);
    }
    if let Some(password) = password {
        *password = question.password;
    }
}

/// Writes what a read answered into the caller's out-parameter as an owning handle, or answers
/// the status that says why there is none; the out-parameter is written only on `QUORRA_OK`.
pub(super) fn hand_over<T>(read: Result<T, Status>, out: &mut *mut T) -> c_int {
    match read {
        Ok(value) => {
            *out = Box::into_raw(Box::new(value));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

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
pub(super) unsafe fn owned_text(text: *const c_char) -> Result<Option<String>, ()> {
    if text.is_null() {
        return Ok(None);
    }
    CStr::from_ptr(text)
        .to_str()
        .map(|text| Some(text.to_owned()))
        .map_err(|_| ())
}

/// The two-call idiom over `float`s, [`copy_out`]'s shape: `needed` is how many there are, and
/// `out` takes them when `cap` holds that many.
///
/// # Safety
///
/// `out` is null or writable for `cap` `float`s; `needed` is null or writable.
pub(super) unsafe fn copy_floats(
    values: &[f32],
    out: *mut f32,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    if let Some(needed) = needed.as_mut() {
        *needed = values.len();
    }
    if values.is_empty() {
        return Status::Ok.code();
    }
    if out.is_null() || cap < values.len() {
        return Status::BufferTooSmall.code();
    }
    core::slice::from_raw_parts_mut(out, values.len()).copy_from_slice(values);
    Status::Ok.code()
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
pub(super) unsafe fn copy_out(
    text: &str,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
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
