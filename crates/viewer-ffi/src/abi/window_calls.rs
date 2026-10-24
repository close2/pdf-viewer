//! `quorra_window`: a native window's toolkit-free half, driven from C.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::layouts::PdfvFrame;
use super::shared::{
    copy_floats, copy_out, hand_over, owned_text, script_answer, write_script_question,
};
use crate::answers::{Matches, Miniature, Popups};
use crate::form::Form;
use crate::keyboard::KeyKind;
use crate::kinds::{PixelFormat, PointerKind, ScriptTextKind};
use crate::panels::{Outline, Panel};
use crate::session::FrameInfo;
use crate::settings::Stated;
use crate::shapes::Quads;
use crate::status::Status;
use crate::window::{OwedKind, Owing, WheelKind, Window, WindowChrome};
use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------------------------
// `quorra_window`: a native window's toolkit-free half, driven from C (RFC 0010 section 3.2,
// ADR 2088). What the window owes its caller arrives as an owned batch after each call returns,
// never as a callback into the caller.
// ---------------------------------------------------------------------------------------------

/// Opens a window on the file at `path`: its bytes are read from disk and the document opens on a
/// thread of its own, joined by the first [`quorra_window_resize`]. `fragment` may be null.
///
/// `QUORRA_UNREADABLE` where the file cannot be opened for reading.
///
/// # Safety
///
/// See the module documentation. `path` is NUL-terminated UTF-8, `fragment` null or the same, and
/// `window` receives an owning handle to be freed with [`quorra_window_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_open(
    path: *const c_char,
    fragment: *const c_char,
    window: *mut *mut Window,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    let (Ok(path), Ok(fragment)) = (owned_text(path), owned_text(fragment)) else {
        return Status::NotUtf8.code();
    };
    let Some(path) = path else {
        return Status::NullArgument.code();
    };
    match Window::open(
        std::path::Path::new(&path),
        fragment,
        viewer_host::policy::Words::default(),
    ) {
        Ok(opened) => {
            *window = Box::into_raw(Box::new(opened));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// [`quorra_window_open`], under what `settings` states rather than the defaults — the levels and
/// the reader's other words (ADRs 2120, 2156). The window takes a copy: `settings` stays the
/// caller's, to free or to open another window under.
///
/// # Safety
///
/// See the module documentation. `path` is NUL-terminated UTF-8, `fragment` null or the same,
/// `settings` a handle from [`quorra_settings_new`], and `window` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_open_under(
    path: *const c_char,
    fragment: *const c_char,
    settings: *const Stated,
    window: *mut *mut Window,
) -> c_int {
    let (Some(window), Some(settings)) = (window.as_mut(), settings.as_ref()) else {
        return Status::NullArgument.code();
    };
    let (Ok(path), Ok(fragment)) = (owned_text(path), owned_text(fragment)) else {
        return Status::NotUtf8.code();
    };
    let Some(path) = path else {
        return Status::NullArgument.code();
    };
    match Window::open(std::path::Path::new(&path), fragment, settings.words()) {
        Ok(opened) => {
            *window = Box::into_raw(Box::new(opened));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The reader's levels at a window's defaults — what a window with no command line holds — for a
/// host to state its own on, one call at a time (RFC 0010 section 5, ADR 2120).
///
/// # Safety
///
/// See the module documentation. `settings` receives an owning handle to be freed with
/// [`quorra_settings_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_new(settings: *mut *mut Stated) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    *settings = Box::into_raw(Box::new(Stated::default()));
    Status::Ok.code()
}

/// Releases the levels. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_free(settings: *mut Stated) {
    if !settings.is_null() {
        drop(Box::from_raw(settings));
    }
}

/// One of the acts a document asks of this machine, `QUORRA_POLICY_*`, at `QUORRA_LEVEL_*`: the
/// level `viewer_host::Settings::set` puts it at, which is the level the policy's own command-line
/// word names in the same place. `QUORRA_WRONG_KIND` for a number neither names, and for
/// `QUORRA_POLICY_SEPARATIONS` at `ASK` or `WARN`; nothing changes.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_set(
    settings: *mut Stated,
    policy: u32,
    level: u32,
) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    match settings.set(policy, level) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// One restriction's level, `QUORRA_RESTRICTED_*` at `QUORRA_RESTRICT_*`, as
/// [`quorra_restrict_operation`](super::quorra_restrict_operation) takes them; every other operation stays where it stood.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_restrict(
    settings: *mut Stated,
    operation: u32,
    level: u32,
) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    match settings.restrict(operation, level) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// One word of a command line, read as `quorra`, `quorra-gtk`, `quorra-qt` and `quorra-confined`
/// read theirs (`viewer_host::policy::Words::take`): `taken` is true where it was one of the
/// settings words — `--restrictions=`, `--ignore-restrictions`, `--links=`, `--remote-documents=`,
/// `--embedded-documents=`, `--submissions=`, `--separations=` — or `--accept-unknown-revocation`,
/// and is now in force, false where it is none of them. `QUORRA_WRONG_KIND` where it is one and
/// names no level it takes, or is one of the reader's words that wants the word after it, which
/// [`quorra_settings_take_next`] reads; the sentence is [`quorra_settings_refusal`]'s.
///
/// # Safety
///
/// See the module documentation. `word` is NUL-terminated UTF-8; `taken` is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_take(
    settings: *mut Stated,
    word: *const c_char,
    taken: *mut bool,
) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    let word = match owned_text(word) {
        Ok(Some(word)) => word,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    match settings.take(&word, None) {
        Ok(was) => {
            if let Some(taken) = taken.as_mut() {
                *taken = was > 0;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One word of a command line and the word after it, read as the four windows read theirs, so
/// that a host with a command line takes every word they take with the same refusals (ADR 2156).
/// `*used` is how many of the two were taken: `0` where `word` is none of the words, `1` where it
/// is one that takes no value, `2` where `next` was its value. `next` may be null, as at the end of
/// a command line. `QUORRA_WRONG_KIND` where the word is refused — a value missing, a directory
/// that is not one, a level its word does not take — with the sentence from
/// [`quorra_settings_refusal`]; nothing changes.
///
/// # Safety
///
/// See the module documentation. `word` is NUL-terminated UTF-8, `next` null or the same; `used`
/// is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_take_next(
    settings: *mut Stated,
    word: *const c_char,
    next: *const c_char,
    used: *mut usize,
) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    let (Ok(word), Ok(next)) = (owned_text(word), owned_text(next)) else {
        return Status::NotUtf8.code();
    };
    let Some(word) = word else {
        return Status::NullArgument.code();
    };
    match settings.take(&word, next.as_deref()) {
        Ok(taken) => {
            if let Some(used) = used.as_mut() {
                *used = taken;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of the reader's words that names a value, `QUORRA_READER_*`, with `value`: a directory for
/// `TRUST_ANCHORS` and `REFERENCE_FILES`, read on the window's document thread when it opens; a
/// name, title or organisation added to Table 100's lists; a language tag replacing the last
/// (ADR 2156). `QUORRA_WRONG_KIND` for a number that names none, or where the value is refused as
/// the word's would be, with the sentence from [`quorra_settings_refusal`]; nothing changes.
///
/// # Safety
///
/// See the module documentation. `value` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_reader(
    settings: *mut Stated,
    word: u32,
    value: *const c_char,
) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    let value = match owned_text(value) {
        Ok(Some(value)) => value,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    match settings.state(word, &value) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// Whether a signature whose certificates' revocation status nobody could establish is accepted,
/// as `--accept-unknown-revocation` says it is (ADR 2156). A new handle does not accept it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_accept_unknown_revocation(
    settings: *mut Stated,
    accept: bool,
) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    settings.accept_unknown_revocation(accept);
    Status::Ok.code()
}

/// The conversion ISO 19005's archive key asks for, as `--archive` names one: indices into
/// [`quorra_archive_target`](super::quorra_archive_target)'s and [`quorra_archive_profile`](super::quorra_archive_profile)'s lists, either negative for none,
/// which a new handle holds and under which the key puts up the chooser (ADRs 1954, 2156).
/// `QUORRA_OUT_OF_RANGE` past either list; nothing changes.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_archive(
    settings: *mut Stated,
    target: i32,
    profile: i32,
) -> c_int {
    let Some(settings) = settings.as_mut() else {
        return Status::NullArgument.code();
    };
    match settings.archive(target, profile) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// Why the last word [`quorra_settings_take`] refused was refused, in the sentence each window
/// prints, in the two-call idiom; `QUORRA_NO_ANSWER` where the last word was not refused.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_settings_refusal(
    settings: *const Stated,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(settings) = settings.as_ref() else {
        return Status::NullArgument.code();
    };
    match settings.refusal() {
        Some(sentence) => copy_out(sentence, out, cap, needed),
        None => Status::NoAnswer.code(),
    }
}

/// Releases a window and everything it holds. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_free(window: *mut Window) {
    if !window.is_null() {
        drop(Box::from_raw(window));
    }
}

/// The viewport changed size, in device pixels, at `scale` device pixels per logical pixel. The
/// first call joins the document the window opened.
///
/// `QUORRA_NUMBER_OUT_OF_RANGE` for an empty viewport or a scale that is not a finite positive
/// number, and nothing changes.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_resize(
    window: *mut Window,
    width: u32,
    height: u32,
    scale: f32,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.resize(width, height, scale) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// One look at what runs beside the window: the document still opening, a server's answer, the
/// outline read beside page one, the pages drawn. Called when [`quorra_window_interval`] says.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_tick(window: *mut Window) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    window.tick();
    Status::Ok.code()
}

/// The window was hidden (`shown` false: minimised, or no longer visible) or shown again: §12.4.4's
/// presentation parks while nobody can see it, and the time it was hidden is not time its page
/// was displayed (ADR 2300). Called from the platform's own visibility notice.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_shown(window: *mut Window, shown: bool) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    window.shown(shown);
    Status::Ok.code()
}

/// In how many milliseconds the window wants [`quorra_window_tick`] again: `-1` while nothing
/// beside it is running, which is when the caller's timer stops. Asked after every call.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_interval(window: *const Window) -> i32 {
    window
        .as_ref()
        .and_then(Window::interval)
        .map_or(-1, |millis| i32::try_from(millis).unwrap_or(i32::MAX))
}

/// A key, as a `QUORRA_KEY_*`, with Shift and the platform's command modifier: acted on as every
/// window acts on it, and `*act` the `QUORRA_ACT_*` that only the caller's window can do —
/// `QUORRA_ACT_NOTHING` where the window did all of it. `*by` is a scroll's distance, where it is
/// not null. Presentation acts already update library mode and effective chrome; the returned
/// act still asks the caller for physical window placement.
///
/// # Safety
///
/// See the module documentation. `by` is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_press(
    window: *mut Window,
    key: u32,
    shift: bool,
    command: bool,
    act: *mut u32,
    by: *mut f32,
) -> c_int {
    let (Some(window), Some(act)) = (window.as_mut(), act.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(key) = KeyKind::from_code(key) else {
        return Status::WrongKind.code();
    };
    let held = viewer_host::Modifiers {
        shift,
        ctrl: command,
    };
    let (kind, distance) = window.press(key.key(), held);
    *act = kind.code();
    if let Some(by) = by.as_mut() {
        *by = distance;
    }
    Status::Ok.code()
}

/// The wheel or a scroll bar moved the view, in device pixels.
///
/// `QUORRA_NUMBER_OUT_OF_RANGE` for a distance that is not finite, and nothing moves.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_scroll(window: *mut Window, dx: f32, dy: f32) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    if !dx.is_finite() || !dy.is_finite() {
        return Status::OutOfRange64.code();
    }
    window.scroll(dx, dy);
    Status::Ok.code()
}

/// Control and the wheel: whole zoom steps about (`x`, `y`), the pointer in device pixels of the
/// viewport, which stays where it is on the page. `travel` is the wheel turned away from the
/// person in `unit`, a `QUORRA_WHEEL_*`; the fraction of a step a high-resolution wheel leaves is
/// carried to the next call, as the toolkit windows carry it (ADRs 1118, 2152). `*steps`, where not
/// null, is the steps taken, positive in.
///
/// `QUORRA_WRONG_KIND` for a unit this build does not define, and `QUORRA_NUMBER_OUT_OF_RANGE` for
/// a travel or a point that is not finite; nothing changes for either.
///
/// # Safety
///
/// See the module documentation. `steps` is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_zoom_wheel(
    window: *mut Window,
    travel: f32,
    unit: u32,
    x: f32,
    y: f32,
    steps: *mut i32,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    let Some(unit) = WheelKind::from_code(unit) else {
        return Status::WrongKind.code();
    };
    if !travel.is_finite() || !x.is_finite() || !y.is_finite() {
        return Status::OutOfRange64.code();
    }
    let taken = window.wheel_zoom(travel, unit, (x, y));
    if let Some(steps) = steps.as_mut() {
        *steps = taken;
    }
    Status::Ok.code()
}

/// The pointer at (`x`, `y`) in device pixels of the viewport, as a `QUORRA_POINTER_*`: moved,
/// pressed, dragged or released. `*over_link`, where not null, is whether it is now over §12.5.6.5's
/// activation region, which is the cursor every window changes. `QUORRA_WRONG_KIND` for an action
/// this build does not define, and `QUORRA_NUMBER_OUT_OF_RANGE` for a point that is not finite.
///
/// # Safety
///
/// See the module documentation. `over_link` is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_pointer(
    window: *mut Window,
    x: f32,
    y: f32,
    action: u32,
    over_link: *mut bool,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    let Some(action) = PointerKind::from_code(action) else {
        return Status::WrongKind.code();
    };
    if !x.is_finite() || !y.is_finite() {
        return Status::OutOfRange64.code();
    }
    let over = window.pointer((x, y), action.action());
    if let Some(over_link) = over_link.as_mut() {
        *over_link = over;
    }
    Status::Ok.code()
}

/// Annex O's search for `needle` through the whole document, from the page in front, backwards
/// where `backward` is true; the pages after the first are read on the window's tick, and each
/// step is said. An empty needle searches for nothing.
///
/// # Safety
///
/// See the module documentation. `needle` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_find(
    window: *mut Window,
    needle: *const c_char,
    backward: bool,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match owned_text(needle) {
        Ok(Some(needle)) => {
            window.find(&needle, backward);
            Status::Ok.code()
        }
        Ok(None) => Status::NullArgument.code(),
        Err(()) => Status::NotUtf8.code(),
    }
}

/// The find bar closed: the search ends and its highlights go.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_find_stop(window: *mut Window) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    window.find_stop();
    Status::Ok.code()
}

/// How many documents the window's strip holds. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_document_count(window: *const Window) -> usize {
    window.as_ref().map_or(0, Window::document_count)
}

/// What the strip calls the document at `index`, in the two-call idiom.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_document_label(
    window: *const Window,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(window) = window.as_ref() else {
        return Status::NullArgument.code();
    };
    match window.document_label(index) {
        Some(label) => copy_out(label, out, cap, needed),
        None => Status::OutOfRange.code(),
    }
}

/// Where in the strip the document in front is. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_focused_document(window: *const Window) -> usize {
    window.as_ref().map_or(0, Window::focused_document)
}

/// Brings the document at `index` in the strip to the front. `QUORRA_OUT_OF_RANGE` for an index
/// the strip does not hold.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_choose_document(window: *mut Window, index: usize) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.choose_document(index) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// Opens the file at `path` in a tab of its own beside the documents shown — a file chooser's
/// answer — once those already waiting have opened. What it comes to is owed as sentences.
///
/// # Safety
///
/// See the module documentation. `path` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_open_beside(
    window: *mut Window,
    path: *const c_char,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match owned_text(path) {
        Ok(Some(path)) => {
            window.open_beside(std::path::Path::new(&path));
            Status::Ok.code()
        }
        Ok(None) => Status::NullArgument.code(),
        Err(()) => Status::NotUtf8.code(),
    }
}

/// Closes the document in front and brings its neighbour forward. `*last` is true where it was
/// the only one, which is closing the window: the window is left as it was, and freeing it is the
/// caller's.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_close_document(
    window: *mut Window,
    last: *mut bool,
) -> c_int {
    let (Some(window), Some(last)) = (window.as_mut(), last.as_mut()) else {
        return Status::NullArgument.code();
    };
    *last = window.close_document();
    Status::Ok.code()
}

/// §7.6.4.1: the password typed into the prompt a `QUORRA_OWED_PASSWORD` asked for, or null for a
/// prompt closed without one. `QUORRA_NO_ANSWER` where no prompt is out for `document`.
///
/// # Safety
///
/// See the module documentation. `password` is null or NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_password(
    window: *mut Window,
    document: u64,
    password: *const c_char,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    let Ok(password) = owned_text(password) else {
        return Status::NotUtf8.code();
    };
    match window.password(document, password) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// The answer to the question a `QUORRA_OWED_QUESTION` put: `proceed` for the window's go-ahead,
/// false for its "do not". `QUORRA_NO_ANSWER` where no question is waiting, which is a dialogue
/// closed twice answering once.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_answer(window: *mut Window, proceed: bool) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.answer(proceed) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// The answer to an `ARCHIVE` record: the target and the profile at these indices into
/// [`quorra_archive_target`](super::quorra_archive_target)'s list and [`quorra_archive_profile`](super::quorra_archive_profile)'s, or — either negative — a
/// chooser closed with nothing chosen, which is said. The window keeps the choice for its life
/// and converts (ADRs 1966, 1985). `QUORRA_NO_ANSWER` where no chooser is out; `QUORRA_OUT_OF_RANGE`
/// for an index past a list, which leaves the chooser out.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_archive(
    window: *mut Window,
    target: i32,
    profile: i32,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.archive(target, profile) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// The answer to a `SCRIPT` record, in [`quorra_answer_script`](super::quorra_answer_script)'s numbers: `QUORRA_SCRIPT_ANSWER_*`,
/// with `button` for a press and `text` for a typed answer; a dialogue closed without a press
/// answers with the `dismissed` button the record gave. `QUORRA_NO_ANSWER` where no question from
/// `document` is out — answered already, or withdrawn.
///
/// # Safety
///
/// See the module documentation. `text` is NUL-terminated UTF-8, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_answer_script(
    window: *mut Window,
    document: u64,
    how: u32,
    button: u32,
    text: *const c_char,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    let answer = match script_answer(how, button, owned_text(text)) {
        Ok(answer) => answer,
        Err(status) => return status.code(),
    };
    match window.answer_script(document, answer) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// The print a `PRINT` record let through is over, whether it ran or the person cancelled:
/// §8.11.4.5's groups revert, as the toolkit windows end their print. `QUORRA_NO_ANSWER` where no
/// print is standing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_print_finish(window: *mut Window) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.print_finish() {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// The caller put the page area on the screen. The first call that shows a page is the launch's
/// first frame, and what waits behind page one starts then.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_presented(window: *mut Window) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    window.presented();
    Status::Ok.code()
}

/// The title bar's text as it now stands, in the two-call idiom [`quorra_events_describe`](super::quorra_events_describe) uses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_title(
    window: *const Window,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(window) = window.as_ref() else {
        return Status::NullArgument.code();
    };
    copy_out(&window.title(), out, cap, needed)
}

/// The effective chrome to apply after a chrome record. This reads resolved window state,
/// including presentation mode; it is not the document's raw preference dictionary.
///
/// # Safety
///
/// See the module documentation. `out` is writable, and neither pointer may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_chrome(
    window: *const Window,
    out: *mut WindowChrome,
) -> c_int {
    let (Some(window), Some(out)) = (window.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    *out = window.chrome();
    Status::Ok.code()
}

/// The status line's sentence as it now stands, in the two-call idiom.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_status(
    window: *const Window,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(window) = window.as_ref() else {
        return Status::NullArgument.code();
    };
    copy_out(window.status(), out, cap, needed)
}

/// How many pictures the page area shows: the viewer's frames, and the stand-ins every window
/// paints where a page has no frame of its own yet.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_frame_count(window: *const Window) -> usize {
    window.as_ref().map_or(0, Window::frame_count)
}

/// One picture of the page area: the raster's facts with its origin where the window shows it,
/// and the extent it is shown at in `*shown_width` and `*shown_height` where they are not null —
/// which differs from the raster's for a frame from before a zoom.
///
/// # Safety
///
/// See the module documentation. `shown_width` and `shown_height` are writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_frame(
    window: *const Window,
    frame: usize,
    info: *mut PdfvFrame,
    shown_width: *mut u32,
    shown_height: *mut u32,
) -> c_int {
    let (Some(window), Some(info)) = (window.as_ref(), info.as_mut()) else {
        return Status::NullArgument.code();
    };
    match window.frame(frame) {
        Ok((
            FrameInfo {
                page,
                width,
                height,
                format,
                bytes,
                origin,
            },
            (shown_across, shown_down),
        )) => {
            *info = PdfvFrame {
                page,
                width,
                height,
                format: PixelFormat::of(format).code(),
                bytes,
                origin_x: origin.0,
                origin_y: origin.1,
            };
            if let Some(shown_width) = shown_width.as_mut() {
                *shown_width = shown_across;
            }
            if let Some(shown_height) = shown_height.as_mut() {
                *shown_height = shown_down;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Copies one picture's pixels into a buffer the caller owns, sized from [`quorra_window_frame`]'s
/// `bytes`; a shorter one answers `QUORRA_BUFFER_TOO_SMALL` and nothing is written.
///
/// # Safety
///
/// See the module documentation. `into` is writable for `cap` bytes; `written` is writable, or
/// null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_frame_copy(
    window: *const Window,
    frame: usize,
    into: *mut u8,
    cap: usize,
    written: *mut usize,
) -> c_int {
    let Some(window) = window.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    let room = core::slice::from_raw_parts_mut(into, cap);
    match window.frame_copy(frame, room) {
        Ok(bytes) => {
            if let Some(written) = written.as_mut() {
                *written = bytes;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Everything the window has come to owe its caller since the last call, as an owned batch to be
/// freed with [`quorra_owed_free`]: read it after every other `quorra_window_*` call.
///
/// # Safety
///
/// See the module documentation. `owed` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_take_owed(
    window: *mut Window,
    owed: *mut *mut Owing,
) -> c_int {
    let (Some(window), Some(owed)) = (window.as_mut(), owed.as_mut()) else {
        return Status::NullArgument.code();
    };
    *owed = Box::into_raw(Box::new(window.take_owed()));
    Status::Ok.code()
}

/// Releases a batch of what a window owed. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_free(owed: *mut Owing) {
    if !owed.is_null() {
        drop(Box::from_raw(owed));
    }
}

/// How many records the batch holds. Zero for a null pointer, which is also the empty answer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_count(owed: *const Owing) -> usize {
    owed.as_ref().map_or(0, Owing::len)
}

/// The `QUORRA_OWED_*` of one record.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_kind(
    owed: *const Owing,
    index: usize,
    kind: *mut u32,
) -> c_int {
    let (Some(owed), Some(kind)) = (owed.as_ref(), kind.as_mut()) else {
        return Status::NullArgument.code();
    };
    match owed.kind(index) {
        Some(owed_kind) => {
            *kind = owed_kind.code();
            Status::Ok.code()
        }
        None => Status::OutOfRange.code(),
    }
}

/// The text of a `SAID`, `TITLE`, `PASSWORD`, `QUESTION` or `CLIPBOARD` record, in the two-call
/// idiom; `QUORRA_WRONG_KIND` for a record that carries none.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_text(
    owed: *const Owing,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(owed) = owed.as_ref() else {
        return Status::NullArgument.code();
    };
    match owed.text(index) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// The document a `PASSWORD` record is about, which [`quorra_window_password`] answers for;
/// `QUORRA_WRONG_KIND` for a record that names none.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_document(
    owed: *const Owing,
    index: usize,
    document: *mut u64,
) -> c_int {
    let (Some(owed), Some(document)) = (owed.as_ref(), document.as_mut()) else {
        return Status::NullArgument.code();
    };
    match owed.document(index) {
        Ok(id) => {
            *document = id;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The target and the profile an `ARCHIVE` record's chooser starts on, as indices into
/// [`quorra_archive_target`](super::quorra_archive_target)'s list and [`quorra_archive_profile`](super::quorra_archive_profile)'s; `QUORRA_WRONG_KIND` for
/// another record.
///
/// # Safety
///
/// See the module documentation. Both out-parameters are writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_archive(
    owed: *const Owing,
    index: usize,
    target: *mut usize,
    profile: *mut usize,
) -> c_int {
    let Some(owed) = owed.as_ref() else {
        return Status::NullArgument.code();
    };
    match owed.archive(index) {
        Ok((starts_on, profile_starts_on)) => {
            if let Some(target) = target.as_mut() {
                *target = starts_on;
            }
            if let Some(profile) = profile.as_mut() {
                *profile = profile_starts_on;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A `SCRIPT` record's question, in [`quorra_event_script_question`](super::quorra_event_script_question)'s numbers and shape;
/// `QUORRA_WRONG_KIND` for another record.
///
/// # Safety
///
/// See the module documentation. Every out-parameter is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_script_question(
    owed: *const Owing,
    index: usize,
    kind: *mut u32,
    icon: *mut u32,
    buttons: *mut u32,
    dismissed: *mut u32,
    password: *mut bool,
) -> c_int {
    let Some(owed) = owed.as_ref() else {
        return Status::NullArgument.code();
    };
    match owed.script(index) {
        Ok(question) => {
            write_script_question(
                crate::ScriptQuestionNumbers::of(question),
                kind.as_mut(),
                icon.as_mut(),
                buttons.as_mut(),
                dismissed.as_mut(),
                password.as_mut(),
            );
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of a `SCRIPT` record's strings, `QUORRA_SCRIPT_TEXT_*`, as [`quorra_event_script_text`](super::quorra_event_script_text)
/// copies it: a title or a label the script did not give is `QUORRA_NO_ANSWER`.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_script_text(
    owed: *const Owing,
    index: usize,
    part: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(owed) = owed.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(part) = ScriptTextKind::from_code(part) else {
        return Status::WrongKind.code();
    };
    match owed.script(index) {
        Ok(question) => match crate::events::script_part(question, part) {
            Some(text) => copy_out(text, out, cap, needed),
            None => Status::NoAnswer.code(),
        },
        Err(status) => status.code(),
    }
}

/// A `PRINT` record's pages, and whether §7.6.4.2's bit 12 limits the quality a copy may be
/// rebuilt from; `QUORRA_WRONG_KIND` for another record.
///
/// # Safety
///
/// See the module documentation. Both out-parameters are writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_print(
    owed: *const Owing,
    index: usize,
    pages: *mut usize,
    degraded: *mut bool,
) -> c_int {
    let Some(owed) = owed.as_ref() else {
        return Status::NullArgument.code();
    };
    match owed.print(index) {
        Ok((count, limited)) => {
            if let Some(pages) = pages.as_mut() {
                *pages = count;
            }
            if let Some(degraded) = degraded.as_mut() {
                *degraded = limited;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A chrome notification's one-shot panel request: -1 closes, a tab index opens.
/// `NoAnswer` preserves the reader's pane choice. The record lives until the batch is freed.
///
/// # Safety
///
/// See the module documentation. `panel` is writable and both pointers are required.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_chrome_panel(
    owed: *const Owing,
    index: usize,
    panel: *mut c_int,
) -> c_int {
    let (Some(owed), Some(panel)) = (owed.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    match owed.chrome_panel(index) {
        Ok(requested) => {
            *panel = requested;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// What a `REPAINT` record changed, a word of `QUORRA_REPAINT_*` bits: the pictures' pixels,
/// their places, the marks over them (ADR 2168); `QUORRA_WRONG_KIND` for another record.
///
/// # Safety
///
/// See the module documentation. `changed` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_repaint(
    owed: *const Owing,
    index: usize,
    changed: *mut u32,
) -> c_int {
    let (Some(owed), Some(changed)) = (owed.as_ref(), changed.as_mut()) else {
        return Status::NullArgument.code();
    };
    match owed.repaint(index) {
        Ok(repainted) => {
            *changed = repainted.changed();
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Whether a `REPAINT` record's picture `frame` has pixels the caller has not been handed, so that
/// only those are copied with `quorra_window_frame_copy` (ADR 2168); `QUORRA_OUT_OF_RANGE` past the
/// pictures the page area shows, `QUORRA_WRONG_KIND` for another record.
///
/// # Safety
///
/// See the module documentation. `fresh` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_owed_repaint_fresh(
    owed: *const Owing,
    index: usize,
    frame: usize,
    fresh: *mut bool,
) -> c_int {
    let (Some(owed), Some(fresh)) = (owed.as_ref(), fresh.as_mut()) else {
        return Status::NullArgument.code();
    };
    match owed
        .repaint(index)
        .and_then(|repainted| repainted.fresh(frame))
    {
        Ok(new) => {
            *fresh = new;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many record kinds this build defines, against the header's `QUORRA_OWED_KIND_COUNT`: a
/// caller compiled before a kind was added says so at startup rather than meeting it unarmed.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_owed_kind_count() -> u32 {
    OwedKind::COUNT
}

/// The name of a record kind, NUL-terminated and never freed. `"unknown"` for one this build does
/// not define.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_owed_kind_name(kind: u32) -> *const c_char {
    let name = OwedKind::from_code(kind).map_or("unknown\0", OwedKind::name);
    name.as_ptr().cast::<c_char>()
}

/// §12.3.3's outline of the document in front, flattened as [`quorra_outline_read`](super::quorra_outline_read) hands it over
/// and released with [`quorra_outline_free`](super::quorra_outline_free); read again on a `QUORRA_OWED_PANELS`.
///
/// # Safety
///
/// See the module documentation. `outline` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_outline_read(
    window: *const Window,
    outline: *mut *mut Outline,
) -> c_int {
    let (Some(window), Some(outline)) = (window.as_ref(), outline.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().outline(), outline)
}

/// §8.11.4.3's `/Order` of the document in front, as [`quorra_layers_read`](super::quorra_layers_read) hands it over.
///
/// # Safety
///
/// See the module documentation. `panel` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_layers_read(
    window: *const Window,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(window), Some(panel)) = (window.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().layers(), panel)
}

/// §7.11.4's embedded files of the document in front, as [`quorra_attachments_read`](super::quorra_attachments_read) hands them
/// over.
///
/// # Safety
///
/// See the module documentation. `panel` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_attachments_read(
    window: *const Window,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(window), Some(panel)) = (window.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().attachments(), panel)
}

/// §14.3's information of the document in front, as [`quorra_properties_read`](super::quorra_properties_read) hands it over.
///
/// # Safety
///
/// See the module documentation. `panel` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_properties_read(
    window: *const Window,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(window), Some(panel)) = (window.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().properties(), panel)
}

/// §12.4.3's article threads of the document in front, as [`quorra_articles_read`](super::quorra_articles_read) hands them
/// over.
///
/// # Safety
///
/// See the module documentation. `panel` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_articles_read(
    window: *const Window,
    panel: *mut *mut Panel,
) -> c_int {
    let (Some(window), Some(panel)) = (window.as_ref(), panel.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().articles(), panel)
}

/// How many pages the document in front has: the rows of §12.3.4's panel.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_page_count(
    window: *const Window,
    pages: *mut usize,
) -> c_int {
    let (Some(window), Some(pages)) = (window.as_ref(), pages.as_mut()) else {
        return Status::NullArgument.code();
    };
    match window.reads().page_count() {
        Ok(count) => {
            *pages = count;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// §12.4.2's label for one page of the document in front, in the two-call idiom;
/// `QUORRA_NO_ANSWER` where the page states none.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_page_label(
    window: *const Window,
    page: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(window) = window.as_ref() else {
        return Status::NullArgument.code();
    };
    match window.reads().page_label(page) {
        Ok(label) => copy_out(&label, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// §12.3.4's thumbnail for one page of the document in front, as [`quorra_thumbnail_read`](super::quorra_thumbnail_read) hands
/// it over and released with [`quorra_thumbnail_free`](super::quorra_thumbnail_free).
///
/// # Safety
///
/// See the module documentation. `thumbnail` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_thumbnail_read(
    window: *const Window,
    page: usize,
    thumbnail: *mut *mut Miniature,
) -> c_int {
    let (Some(window), Some(thumbnail)) = (window.as_ref(), thumbnail.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().thumbnail(page), thumbnail)
}

/// §12.7's fields with a widget on the page shown — the controls the caller places over the page —
/// as [`quorra_fields_read`](super::quorra_fields_read) hands them over; read again on a `QUORRA_OWED_CONTROLS`.
///
/// # Safety
///
/// See the module documentation. `fields` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_fields_read(
    window: *const Window,
    fields: *mut *mut Form,
) -> c_int {
    let (Some(window), Some(fields)) = (window.as_ref(), fields.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().fields(), fields)
}

/// §12.5.6.14's open popup windows, placed, as [`quorra_popups_read`](super::quorra_popups_read) hands them over; read again
/// on a `QUORRA_OWED_POPUPS`.
///
/// # Safety
///
/// See the module documentation. `popups` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_popups_read(
    window: *const Window,
    popups: *mut *mut Popups,
) -> c_int {
    let (Some(window), Some(popups)) = (window.as_ref(), popups.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().popups(), popups)
}

/// The shapes covering what is selected, in device pixels of the viewport, drawn over the frames
/// on every `QUORRA_OWED_REPAINT`.
///
/// # Safety
///
/// See the module documentation. `quads` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_selection_quads(
    window: *const Window,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(window), Some(quads)) = (window.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().selection_quads(), quads)
}

/// Annex O's highlighted rectangles on the page shown, as [`quorra_highlight_quads`](super::quorra_highlight_quads) hands them
/// over: shapes, because Table Annex O.4 leaves the nature of the highlighting to the
/// implementation.
///
/// # Safety
///
/// See the module documentation. `quads` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_highlight_quads(
    window: *const Window,
    quads: *mut *mut Quads,
) -> c_int {
    let (Some(window), Some(quads)) = (window.as_ref(), quads.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.reads().highlight_quads(), quads)
}

/// Every occurrence of what the find bar holds on the page shown — the needle the last
/// [`quorra_window_find`] gave, and none after [`quorra_window_find_stop`] — as
/// [`quorra_find_matches`](super::quorra_find_matches) hands them over.
///
/// # Safety
///
/// See the module documentation. `matches` receives an owning handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_find_matches(
    window: *const Window,
    matches: *mut *mut Matches,
) -> c_int {
    let (Some(window), Some(matches)) = (window.as_ref(), matches.as_mut()) else {
        return Status::NullArgument.code();
    };
    hand_over(window.matches(), matches)
}

/// Which annotation holds §12.5.1's focus, and the quadrilateral the caller draws its ring on.
///
/// # Safety
///
/// See the module documentation. `quad` is writable for eight `float`s, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_focused_annotation(
    window: *const Window,
    number: *mut u32,
    generation: *mut u16,
    quad: *mut f32,
) -> c_int {
    let Some(window) = window.as_ref() else {
        return Status::NullArgument.code();
    };
    match window.reads().focused_annotation() {
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

/// §12.9's traced path while measuring, in device pixels of the viewport as flat `x, y` pairs,
/// drawn over the page on a repaint whose marks changed; none while the mode is off (ADR 2168).
/// Two-call idiom over `float`s: `needed` is twice the points.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` `float`s, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_traced_path(
    window: *const Window,
    out: *mut f32,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(window) = window.as_ref() else {
        return Status::NullArgument.code();
    };
    copy_floats(&window.traced_path(), out, cap, needed)
}

/// What the caller measured of the controls it placed over the page: `count` controls, four
/// `int32_t`s each — the width and height it asked the control to occupy, then the least width
/// and height the toolkit says the control can be — in logical pixels (ADR 2169). `*magnification`,
/// where not null, is the magnification at which every control fits, which `QUORRA_KEY_W`'s fit
/// sends, or 0 where they fit now.
///
/// # Safety
///
/// See the module documentation. `sizes` is readable for `4 * count` `int32_t`s, or null where
/// `count` is 0; `magnification` is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_controls_measured(
    window: *mut Window,
    sizes: *const i32,
    count: usize,
    magnification: *mut f32,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    let Some(values) = count.checked_mul(4) else {
        return Status::OutOfRange.code();
    };
    let measured: Vec<[i32; 4]> = if values == 0 {
        Vec::new()
    } else if sizes.is_null() {
        return Status::NullArgument.code();
    } else {
        core::slice::from_raw_parts(sizes, values)
            .chunks_exact(4)
            .filter_map(|four| <[i32; 4]>::try_from(four).ok())
            .collect()
    };
    let fits = window.controls_measured(&measured);
    if let Some(magnification) = magnification.as_mut() {
        *magnification = fits.unwrap_or(0.0);
    }
    Status::Ok.code()
}

/// Shows the page a thumbnail row names (§12.3.4), by its zero-based index.
/// `QUORRA_NO_ANSWER` where no document is focused, `QUORRA_OUT_OF_RANGE` past its page list.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_go_to_page(window: *mut Window, page: usize) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.go_to_page(page) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// Activates an object: an outline item's or a thread's row, or a push-button's widget.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_activate(
    window: *mut Window,
    number: u32,
    generation: u16,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    window.activate(number, generation);
    Status::Ok.code()
}

/// A placed control's text, as typed; §12.7.5.3's file-select field takes the file it names.
/// `QUORRA_NO_ANSWER` where no field of that qualified name has a widget on the page shown.
///
/// # Safety
///
/// See the module documentation. `field` and `text` are NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_set_field_text(
    window: *mut Window,
    field: *const c_char,
    text: *const c_char,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match (owned_text(field), owned_text(text)) {
        (Ok(Some(field)), Ok(Some(text))) => match window.set_field_text(&field, text) {
            Ok(()) => Status::Ok.code(),
            Err(status) => status.code(),
        },
        (Ok(None), _) | (_, Ok(None)) => Status::NullArgument.code(),
        (Err(()), _) | (_, Err(())) => Status::NotUtf8.code(),
    }
}

/// §12.7.5.4: which of Table 234's `/Opt` entries a placed choice control has selected, by index;
/// none is a field with nothing chosen.
///
/// # Safety
///
/// See the module documentation. `field` is NUL-terminated UTF-8; `options` is readable for
/// `count` elements, or null when `count` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_set_field_options(
    window: *mut Window,
    field: *const c_char,
    options: *const usize,
    count: usize,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    let field = match owned_text(field) {
        Ok(Some(field)) => field,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let chosen = if count == 0 {
        Vec::new()
    } else if options.is_null() {
        return Status::NullArgument.code();
    } else {
        core::slice::from_raw_parts(options, count).to_vec()
    };
    match window.set_field_options(&field, chosen) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// Empties a placed control's field: §12.7.6.3's "its V entry shall be removed".
///
/// # Safety
///
/// See the module documentation. `field` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_clear_field(
    window: *mut Window,
    field: *const c_char,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match owned_text(field) {
        Ok(Some(field)) => match window.clear_field(&field) {
            Ok(()) => Status::Ok.code(),
            Err(status) => status.code(),
        },
        Ok(None) => Status::NullArgument.code(),
        Err(()) => Status::NotUtf8.code(),
    }
}

/// A placed control finished with: Enter in a single-line field, or the keyboard taken elsewhere.
///
/// # Safety
///
/// See the module documentation. `field` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_commit_field(
    window: *mut Window,
    field: *const c_char,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match owned_text(field) {
        Ok(Some(field)) => match window.commit_field(&field) {
            Ok(()) => Status::Ok.code(),
            Err(status) => status.code(),
        },
        Ok(None) => Status::NullArgument.code(),
        Err(()) => Status::NotUtf8.code(),
    }
}

/// A placed check box or radio button, by its widget annotation, clicked to `on`; a refusal the
/// field's flags state is said. `QUORRA_NO_ANSWER` where no widget on the page is that annotation,
/// `QUORRA_WRONG_KIND` where it toggles nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_toggle_field(
    window: *mut Window,
    number: u32,
    generation: u16,
    on: bool,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.toggle_field(number, generation, on) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}

/// §8.11's switch on a layer's row moved; a locked group (Table 99) is said and left.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_set_group(
    window: *mut Window,
    number: u32,
    generation: u16,
    on: bool,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    window.set_group(number, generation, on);
    Status::Ok.code()
}

/// §7.11.4: takes out the file an attachment's row names, by its `/EmbeddedFiles` key.
///
/// # Safety
///
/// See the module documentation. `name` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_extract(window: *mut Window, name: *const c_char) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match owned_text(name) {
        Ok(Some(name)) => {
            window.extract(name);
            Status::Ok.code()
        }
        Ok(None) => Status::NullArgument.code(),
        Err(()) => Status::NotUtf8.code(),
    }
}

/// How many panels a window offers, against the header's `QUORRA_TAB_COUNT`: `viewer_host::Tab`'s
/// list, in the order every window offers it, so that a panel added fails a C host's startup check
/// as it fails to compile in the toolkit windows (RFC 0010 section 5, ADR 2104).
#[unsafe(no_mangle)]
pub extern "C" fn quorra_tab_count() -> u32 {
    u32::try_from(viewer_host::Tab::ALL.len()).unwrap_or(u32::MAX)
}

/// What a panel's tab says — the one wording every window shows — in the two-call idiom;
/// `QUORRA_OUT_OF_RANGE` for a `QUORRA_TAB_*` this build does not define.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_tab_label(
    tab: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let label = usize::try_from(tab)
        .ok()
        .and_then(|index| viewer_host::Tab::ALL.get(index))
        .map(|tab| tab.label());
    match label {
        Some(label) => copy_out(label, out, cap, needed),
        None => Status::OutOfRange.code(),
    }
}

/// The number of rows in the window's shared permissions menu; zero for null.
///
/// # Safety
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_restriction_count(window: *const Window) -> usize {
    window
        .as_ref()
        .map_or(0, |window| window.restriction_rows().len())
}

/// Reads a permissions menu row: part 0 is its label, part 1 its explanatory note.
/// Depth 0 is a scope, 1 an operation, 2 a selectable level. Only levels can be checked.
///
/// # Safety
/// See the module documentation. `depth` and `chosen` are writable; `out` follows the text idiom.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_restriction_read(
    window: *const Window,
    index: usize,
    part: u32,
    depth: *mut u32,
    chosen: *mut bool,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let (Some(window), Some(depth), Some(chosen)) =
        (window.as_ref(), depth.as_mut(), chosen.as_mut())
    else {
        return Status::NullArgument.code();
    };
    let Some(row) = window.restriction_rows().get(index).copied() else {
        return Status::OutOfRange.code();
    };
    let (nesting, checked, label, note) =
        match row {
            viewer_host::Row::Scope { label, note, .. }
            | viewer_host::Row::Machine { label, note } => (0, false, label, note),
            viewer_host::Row::Operation { label, note, .. }
            | viewer_host::Row::Act { label, note } => (1, false, label, note),
            viewer_host::Row::Level(entry) => (2, entry.chosen, entry.label, ""),
            viewer_host::Row::ActLevel(entry) => (2, entry.chosen, entry.label, ""),
        };
    let text = match part {
        0 => label,
        1 => note,
        _ => return Status::WrongKind.code(),
    };
    *depth = nesting;
    *chosen = checked;
    copy_out(text, out, cap, needed)
}

/// Chooses a permissions menu level by its row index; a heading is `QUORRA_WRONG_KIND`.
///
/// # Safety
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_window_restriction_choose(
    window: *mut Window,
    index: usize,
) -> c_int {
    let Some(window) = window.as_mut() else {
        return Status::NullArgument.code();
    };
    match window.choose_restriction(index) {
        Ok(()) => Status::Ok.code(),
        Err(status) => status.code(),
    }
}
