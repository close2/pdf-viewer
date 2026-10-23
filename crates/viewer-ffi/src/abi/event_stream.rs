//! Events, the owned batch a command answers with, and the accessors that read one event's fields.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::shared::{copy_out, write_script_question};
use crate::events::Events;
use crate::kinds::ScriptTextKind;
use crate::status::Status;
use core::ffi::{c_char, c_int};
use viewer_core::RenderRequest;

// ---------------------------------------------------------------------------------------------
// Events. Owned, so that the viewer's borrow ends before the caller sees anything.
// ---------------------------------------------------------------------------------------------

/// Releases a batch of events. A null pointer is ignored.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_events_free(events: *mut Events) {
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
pub unsafe extern "C" fn quorra_events_len(events: *const Events) -> usize {
    events.as_ref().map_or(0, Events::len)
}

/// Which kind the event at `index` is, written through `kind`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_events_kind(
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
pub unsafe extern "C" fn quorra_events_describe(
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
pub unsafe extern "C" fn quorra_event_opened(
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
pub unsafe extern "C" fn quorra_event_page_changed(
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

/// [`viewer_core::Event::PageChanged`]'s section: the title of §12.3.3's outline item the page
/// falls under, in the two-call idiom [`quorra_events_describe`] uses.
///
/// `QUORRA_NO_ANSWER` where it falls under none, or where the outline was not yet read when the
/// page was announced — which is every announcement before [`quorra_prepared_hand_back`](super::quorra_prepared_hand_back) or a
/// first use.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_page_section(
    events: *const Events,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.page_section(index) {
        Ok(Some(section)) => copy_out(section, out, cap, needed),
        Ok(None) => Status::NoAnswer.code(),
        Err(status) => status.code(),
    }
}

/// [`viewer_core::Event::Searched`]: what a step of a document-wide search found.
///
/// Every out-parameter may be null. `found` is what says whether `page`, `from` and `to` mean
/// anything; `remaining` is what says whether to call [`quorra_find_continue`](super::quorra_find_continue) again.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_searched(
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
/// [`quorra_render_request_rasterise`](super::quorra_render_request_rasterise), and hand it back with [`quorra_render_ready_raster`](super::quorra_render_ready_raster). It must
/// be released — by handing it back, or with [`quorra_render_request_free`](super::quorra_render_request_free).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_render_request(
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

// --------------------------------------------------------------------------------------------- The
// event accessors beyond the first ABI's (ADR 0346).
// ---------------------------------------------------------------------------------------------

/// Which document the event at `index` is about.
///
/// [`Status::WrongKind`] for a `QUORRA_EVENT_DAMAGE`, which is about the *viewport* and names no
/// document.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_document(
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

/// The bytes of a `QUORRA_EVENT_SAVED` or a `QUORRA_EVENT_EXTRACTED`.
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
pub unsafe extern "C" fn quorra_event_bytes(
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

/// A `QUORRA_EVENT_EXTRACTED`'s file name, and whether a person asked for it.
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
pub unsafe extern "C" fn quorra_event_extracted(
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

/// How many published policy copies a `QUORRA_EVENT_SIGNATURE_POLICIES_PUBLISHED` names.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_policy_count(
    events: *const Events,
    index: usize,
    count: *mut usize,
) -> c_int {
    let (Some(events), Some(count)) = (events.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match events.policies(index) {
        Ok(policies) => {
            *count = policies.len();
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The URL of policy `policy` a `QUORRA_EVENT_SIGNATURE_POLICIES_PUBLISHED` names, and the policy's
/// identifier, each in the two-call idiom [`quorra_events_describe`] uses.
///
/// The URL is the signature's word, and fetching it is a decision about this machine at the
/// reader's network level — the caller's, as a submission is (ADR 1738).
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_policy_url(
    events: *const Events,
    index: usize,
    policy: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.policies(index) {
        Ok(policies) => match policies.get(policy) {
            Some(named) => copy_out(&named.url, out, cap, needed),
            None => Status::OutOfRange.code(),
        },
        Err(status) => status.code(),
    }
}

/// Binds a copy of policy `policy`'s document, which the caller fetched from its URL, against the
/// digest the signer signed (ETSI EN 319 122-1 clause 5.2.9.1), and says what it comes to.
///
/// `opens` is true only where the copy is the signer's and a PDF, which the caller may then open
/// beside the document with [`quorra_open`](super::quorra_open); the sentence, in the two-call idiom, says which it is
/// and that §12.8.3.4.4's constraints were not enforced — the three windows' own words (ADR 1753).
/// A null `bytes` with a zero `len` is an empty copy.
///
/// # Safety
///
/// See the module documentation. `bytes` is readable for `len`, or null with a zero `len`; `out`
/// is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_policy_bind(
    events: *const Events,
    index: usize,
    policy: usize,
    bytes: *const u8,
    len: usize,
    opens: *mut bool,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    let copy = if bytes.is_null() {
        if len > 0 {
            return Status::NullArgument.code();
        }
        Vec::new()
    } else {
        core::slice::from_raw_parts(bytes, len).to_vec()
    };
    match events.policy_bound(index, policy, copy) {
        Ok((bound, sentence)) => {
            if let Some(opens) = opens.as_mut() {
                *opens = bound;
            }
            copy_out(&sentence, out, cap, needed)
        }
        Err(status) => status.code(),
    }
}

/// A `QUORRA_EVENT_OPEN_URI`'s resolved URI.
///
/// Handed over rather than opened, and that is not squeamishness: the string is one the *document*
/// controls, and handing it to a browser is a decision about this machine.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_open_uri(
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

/// A `QUORRA_EVENT_COPIED`'s text, in whichever of §14.8.2.5's two orders the core could give.
///
/// `logical` is written 1 where the string is §14.8.2.5.1's logical content order — the document's
/// structure tree, depth first — and 0 where it is page content order, the sequence the content
/// stream showed the glyphs in. The clause says the two "should" coincide without requiring it, so
/// a caller that says which it got is telling a person something true; one that does not is
/// guessing on their behalf.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null; `logical` is writable
/// or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_copied(
    events: *const Events,
    index: usize,
    logical: *mut bool,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.copied(index) {
        Ok((text, is_logical)) => {
            if let Some(slot) = logical.as_mut() {
                *slot = is_logical;
            }
            copy_out(&text, out, cap, needed)
        }
        Err(status) => status.code(),
    }
}

/// A `QUORRA_EVENT_NEEDS_FILE`'s purpose and the document's own words for the file.
///
/// Answer it with [`quorra_supply`](super::quorra_supply), including with a null buffer, which is a caller declining.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_needs_file(
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

/// A `QUORRA_EVENT_NEEDS_FILE`'s `beside`: whether the file opens beside the document that asked.
///
/// Table 203's and Table 204's `/NewWindow true`, `false` for every other purpose — the words a
/// caller owes a person it asks about the file (ADR 1335).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_needs_file_beside(
    events: *const Events,
    index: usize,
    beside: *mut bool,
) -> c_int {
    let (Some(events), Some(beside)) = (events.as_ref(), beside.as_mut()) else {
        return Status::NullArgument.code();
    };
    match events.needs_file_beside(index) {
        Ok(found) => {
            *beside = found;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A `QUORRA_EVENT_DAMAGE`'s rectangle, `[x0, y0, x1, y1]` in device pixels.
///
/// A bound on what changed rather than a promise that everything inside it did.
///
/// # Safety
///
/// See the module documentation. `into` is writable for four `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_damage(
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

/// A `QUORRA_EVENT_DIRTY`'s answer: whether the document now differs from the file it came from.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_dirty(
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

/// A `QUORRA_EVENT_TRANSITION`'s Table 164 numbers, without the style.
///
/// `seconds` is `/D`, and zero for `R`, whose row says "the D entry shall be ignored".
/// `dimension` is `QUORRA_DIMENSION_*` and `motion` is `QUORRA_MOTION_*`. `directed` says whether `/Di`
/// states an angle at all, as against the name `None`; `degrees` is that angle counterclockwise
/// from a left-to-right direction, which the table warns "differs from the page object's Rotate
/// entry, which is measured clockwise from the top".
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_transition(
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

/// A `QUORRA_EVENT_TRANSITION`'s Table 164 `/S`, as the table spells it.
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
pub unsafe extern "C" fn quorra_event_transition_style(
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

/// A `QUORRA_EVENT_SUBMIT`'s request: `QUORRA_METHOD_*` and how many fields its body names. The
/// body is [`quorra_event_bytes`]'; the URL and the media type are the two accessors below.
///
/// # Safety
///
/// See the module documentation. `method` and `fields` are writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_submit(
    events: *const Events,
    index: usize,
    method: *mut u32,
    fields: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.submit(index) {
        Ok((how, count, _, _)) => {
            if let Some(method) = method.as_mut() {
                *method = how as u32;
            }
            if let Some(fields) = fields.as_mut() {
                *fields = count;
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A `QUORRA_EVENT_SUBMIT`'s URL: Table 239's `/F`, with the form data as its query for a GET.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_submit_url(
    events: *const Events,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.submit(index) {
        Ok((_, _, url, _)) => copy_out(url, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// A `QUORRA_EVENT_SUBMIT`'s media type, the one to send the body under — §12.7.5.3's
/// `multipart/form-data` with its boundary where a file-select control made it one.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_submit_media_type(
    events: *const Events,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.submit(index) {
        Ok((_, _, _, media_type)) => copy_out(media_type, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// How many owed sentences a `QUORRA_EVENT_SUBMIT` carries: what the action asked for that its
/// composition does not do, which a caller prints beside the request (ADR 2072).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_submit_owed_count(
    events: *const Events,
    index: usize,
    count: *mut usize,
) -> c_int {
    let (Some(events), Some(count)) = (events.as_ref(), count.as_mut()) else {
        return Status::NullArgument.code();
    };
    match events.submit_owed(index) {
        Ok(owed) => {
            *count = owed.len();
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Owed sentence `owed` of a `QUORRA_EVENT_SUBMIT`, in the two-call idiom
/// [`quorra_events_describe`] uses.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_submit_owed(
    events: *const Events,
    index: usize,
    owed: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.submit_owed(index) {
        Ok(sentences) => match sentences.get(owed) {
            Some(sentence) => copy_out(sentence, out, cap, needed),
            None => Status::OutOfRange.code(),
        },
        Err(status) => status.code(),
    }
}

/// A `QUORRA_EVENT_SCRIPT_ASKING`'s question, in Adobe's numbers: `QUORRA_SCRIPT_QUESTION_*`; for
/// an alert its `QUORRA_ALERT_ICON_*`, its `QUORRA_ALERT_BUTTONS_*` and the `QUORRA_ALERT_BUTTON_*`
/// a dialogue closed without a press answers with (ADR 1628's choice); for a response whether its
/// entry is a password's. What does not apply is written 0.
///
/// # Safety
///
/// See the module documentation. Every out-parameter is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_script_question(
    events: *const Events,
    index: usize,
    kind: *mut u32,
    icon: *mut u32,
    buttons: *mut u32,
    dismissed: *mut u32,
    password: *mut bool,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    match events.script_question(index) {
        Ok(question) => {
            write_script_question(
                question,
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

/// One of a `QUORRA_EVENT_SCRIPT_ASKING`'s strings, `QUORRA_SCRIPT_TEXT_*`. A title or a label the
/// script did not give is [`Status::NoAnswer`].
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_script_text(
    events: *const Events,
    index: usize,
    part: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(events) = events.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(part) = ScriptTextKind::from_code(part) else {
        return Status::WrongKind.code();
    };
    match events.script_text(index, part) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}
