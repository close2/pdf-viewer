//! What only a host can supply: the clock, policy values, printing, trust anchors, the toolkit's own controls and the keyboard.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::layouts::QUORRA_RESTRICT_INHERIT;
use super::shared::{copy_out, owned_text, script_answer};
use crate::events::Events;
use crate::keyboard::{ActKind, KeyKind};
use crate::kinds::{
    AcceptKind, AnsweredKind, DataFormatKind, DelegateKind, LayoutKind, OversizeKind, PresentKind,
    RestrictKind, RestrictedKind, ScriptsKind,
};
use crate::session;
use crate::session::Session;
use crate::status::Status;
use core::ffi::{CStr, c_char, c_int};

// ---------------------------------------------------------------------------------------------
// §12.4.4's clock, and the three policy values only a host can supply.
// ---------------------------------------------------------------------------------------------

/// §12.4.4.1: time has passed, in milliseconds.
///
/// **This library has no clock**, so a presentation that advances by itself can only know a second
/// went by by being told. A page stating no `/Dur` swallows every tick — "\[i]f no Dur entry is
/// specified in the page object, the page shall not advance automatically."
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_tick(
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
/// [`quorra_tick`] (ADR 0316). Entering saves §8.11's group states and leaving restores them, which
/// is NOTE 2's own instruction.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_present(
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
pub unsafe extern "C" fn quorra_layout(
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
/// should obey. **`QUORRA_RESTRICT_OFF` is the default, for every operation**, because the
/// sentence before the one above is "\[t]hey are low priority" and the one after it is "this
/// program is the reader's" (ADR 1144); what is refused arrives as a
/// `QUORRA_EVENT_REFUSED`, which is deliberately not a `QUORRA_EVENT_REPORTED`: one says what the
/// *document* could not do and the other what the reader's own policy did.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_restrict(
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

/// The same, for one operation alone — `CLAUDE.md`'s four levels, per restriction.
///
/// §7.6.4.2's Table 22 does not state one permission; it states eight positions with eight
/// different subjects, and a reader who wants to be asked before text leaves the program has said
/// nothing whatever about whether they want to be asked before each keystroke into a form field.
/// `operation` is a `QUORRA_RESTRICTED_*` and `level` a `QUORRA_RESTRICT_*`; every other operation's
/// level is left where this caller last put it, so a policy is built one call at a time.
///
/// **This takes no struct by value**, so [`QUORRA_ABI_VERSION`](super::QUORRA_ABI_VERSION) does not move: an entry point
/// *added* is one an old caller never calls, and `quorra_restrict` still sets all six at once.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_restrict_operation(
    viewer: *mut Session,
    operation: u32,
    level: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let (Some(restricted), Some(kind)) = (
        RestrictedKind::from_code(operation),
        RestrictKind::from_code(level),
    ) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(
        viewer.restrict_operation(restricted.operation(), kind.level()),
    ));
    Status::Ok.code()
}

/// The same, for the document this caller has open alone — the scope a window's policy cannot
/// express.
///
/// **A viewer-wide policy is a statement about the reader** (ADR 0604), which is what makes it
/// unable to say *for this document, ask before copying*: a level set to catch one file catches
/// every file this viewer opens afterwards. `operation` is a `QUORRA_RESTRICTED_*`; `level` is a
/// `QUORRA_RESTRICT_*`, or `QUORRA_RESTRICT_INHERIT` to give the operation back to the window's
/// policy. Every other operation's departure is left where this caller last put it, and **all of
/// them are forgotten when a document is opened**, because a departure lasts exactly as long as
/// the document it was about.
///
/// **This takes no struct by value**, so [`QUORRA_ABI_VERSION`](super::QUORRA_ABI_VERSION) does not move, for
/// `quorra_restrict_operation`'s reason: an entry point added is one an old caller never calls.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_restrict_document_operation(
    viewer: *mut Session,
    operation: u32,
    level: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(restricted) = RestrictedKind::from_code(operation) else {
        return Status::WrongKind.code();
    };
    // `INHERIT` is deliberately not a `RestrictKind`: it is the absence of a level, and a kind
    // for it would be a fifth level every other entry point would then have to refuse (ADR 1145).
    let level = if level == QUORRA_RESTRICT_INHERIT {
        None
    } else {
        let Some(kind) = RestrictKind::from_code(level) else {
            return Status::WrongKind.code();
        };
        Some(kind.level())
    };
    *events = Box::into_raw(Box::new(
        viewer.restrict_document(restricted.operation(), level),
    ));
    Status::Ok.code()
}

/// A person pressed copy: §7.6.4.2's bit 5, asked as an operation.
///
/// **The one way text leaves the document.** `quorra_selection_text` is a readback, and a drag
/// asks for one sixty times a second in order to draw a highlight; bit 5 restricts taking text
/// *out of* the document — "\[c]opy or otherwise extract text and graphics from the document" — and
/// only a caller owning a clipboard can say that this is what is happening. So this is the
/// gesture, the reader's policy is consulted exactly where every other operation's is, and the
/// text arrives as a `QUORRA_EVENT_COPIED`, which `quorra_selection_copy_text` then also reads
/// back. Under `QUORRA_RESTRICT_ON` a `QUORRA_EVENT_REFUSED` arrives instead; under
/// `QUORRA_RESTRICT_ASK` a `QUORRA_EVENT_ASKING` does, and the copy follows a `quorra_answer` of
/// `true`.
///
/// Nothing selected sends nothing at all.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_copy(viewer: *mut Session, events: *mut *mut Events) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.copy()));
    Status::Ok.code()
}

/// A person asked to print: §7.6.4.2's bit 3, asked as an operation.
///
/// **The beginning of §8.11.4.5's print operation, and it has to have an end** — that clause's
/// `Print` usage applications "persist only for the duration of the print operation; then all
/// groups shall revert to their prior states", which is `quorra_print_finish`. Between the two,
/// every page this document interprets is interpreted for paper: §12.5.3's Table 167 bit 3
/// decides which annotations are drawn, §8.11.4.5's `Print` event decides which layers are, and
/// §12.5.6.22's watermarks are placed against the sheet rather than against the media box.
///
/// `media` is the sheet in default user space units, lower-left corner first — the origin
/// §12.5.6.22 measures Table 194's percentages from — and `scale` is pixels per unit at the
/// printer's resolution, so 300 dpi is `300.0 / 72.0`. A sheet whose numbers are not finite, or a
/// scale that is not a positive finite number, is refused here rather than one page at a time.
///
/// The grant arrives as a `QUORRA_EVENT_PRINTING`. Under `QUORRA_RESTRICT_ON` with the bit
/// withheld a `QUORRA_EVENT_REFUSED` arrives instead; under `QUORRA_RESTRICT_ASK` a
/// `QUORRA_EVENT_ASKING` does, and the operation begins after a `quorra_answer` of `true`.
///
/// **What this ABI does not offer is the pages themselves.** `viewer_core::Query::PrintPage`
/// answers with a display list, which has no C representation in this library, so a caller here
/// prints what it can already see: the window's own pages, which are interpreted for paper for as
/// long as the operation stands. ADR 1180 says what a page accessor would need.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_print(
    viewer: *mut Session,
    media: *const f32,
    scale: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    if media.is_null() {
        return Status::NullArgument.code();
    }
    let media: [f32; 4] = std::array::from_fn(|corner| *media.add(corner));
    if !scale.is_finite() || scale <= 0.0 || !media.iter().all(|corner| corner.is_finite()) {
        return Status::OutOfRange.code();
    }
    *events = Box::into_raw(Box::new(viewer.print(viewer_core::Printing::Start(
        viewer_core::Sheet {
            media: Some(media),
            scale,
            page_scale: 1.0,
        },
    ))));
    Status::Ok.code()
}

/// `quorra_print` with §12.5.6.22's matrix B stated — a page placed on the media rather than at
/// its own size.
///
/// **A second entry point rather than a fifth argument on the first**, which is this header's own
/// rule: a function added later is a symbol an old caller never looks up, while a changed
/// signature is a call an old caller has already compiled and no diagnostic would catch.
///
/// `page_scale` is what the page is scaled by onto the media — 1.0 is `quorra_print`, and a
/// caller that shrinks a page to fit the paper or composes an n-up sheet states the factor. A
/// fixed print watermark is drawn immune to it: §12.5.6.22's two post-EXAMPLE bullets both
/// require the mark at the size the *media* was measured in. ADR 1204.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_print_placed(
    viewer: *mut Session,
    media: *const f32,
    scale: f32,
    page_scale: f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    if media.is_null() {
        return Status::NullArgument.code();
    }
    let media: [f32; 4] = std::array::from_fn(|corner| *media.add(corner));
    if !scale.is_finite()
        || scale <= 0.0
        || !page_scale.is_finite()
        || page_scale <= 0.0
        || !media.iter().all(|corner| corner.is_finite())
    {
        return Status::OutOfRange.code();
    }
    *events = Box::into_raw(Box::new(viewer.print(viewer_core::Printing::Start(
        viewer_core::Sheet {
            media: Some(media),
            scale,
            page_scale,
        },
    ))));
    Status::Ok.code()
}

/// A `QUORRA_EVENT_PRINTING`'s two facts: how many pages, and whether the job is degraded.
///
/// `degraded` is §7.6.4.2's Table 22 bit 12 answered — see `quorra_print`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_event_printing(
    events: *const Events,
    index: usize,
    pages: *mut usize,
    degraded: *mut bool,
) -> c_int {
    let (Some(events), Some(pages), Some(degraded)) =
        (events.as_ref(), pages.as_mut(), degraded.as_mut())
    else {
        return Status::NullArgument.code();
    };
    match events.printing(index) {
        Ok((found, was_degraded)) => {
            *pages = found;
            *degraded = was_degraded;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The end of the print operation `quorra_print` began — §8.11.4.5's revert.
///
/// Sent whether the job finished or the person cancelled: the clause's sentence is about the
/// duration of the operation and says nothing about how it ended. Harmless where none is running,
/// and it asks nothing — a reader allowed to start is not asked again for permission to stop.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_print_finish(
    viewer: *mut Session,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.print(viewer_core::Printing::Finish)));
    Status::Ok.code()
}

/// §12.8.1's third question: the certification authorities this reader will end a path at.
///
/// **A library that ships no root list, because ADR 1039 priced two defaults and refused both.** RFC
/// 5280 section 6.1.1 makes the trust anchors input (d) of nine and says whose choice they are:
/// "The selection of a trust anchor is a matter of policy: it could be the top CA in a hierarchical
/// PKI, the CA that issued the verifier's own certificate(s), or any other CA in a network PKI." So
/// this library holds none, reads no file and opens no socket, and a caller that never calls this
/// gets exactly the answers it got before the entry point existed: every signature's third question
/// is *nobody named one*, which is a statement about this program rather than about a document.
///
/// `certificates` is `count` pointers to DER `Certificate` encodings and `lengths` their lengths.
/// `names` may be null, or `count` NUL-terminated strings naming each one — whatever the caller
/// knows them by, which is what a refusal to read one is reported against. `source` is the sentence
/// saying where they came from, which is not decoration: a reader told that a signature is valid is
/// owed *valid according to whom*, and only the caller that read the certificates can say. `at` is
/// section 6.1.1's input (b), seconds since the Unix epoch, because this library asks no clock.
/// `acceptance` is `QUORRA_ACCEPT_*`.
///
/// A `count` of zero clears the store, which is how a caller withdraws one. The bytes are copied
/// before this returns, so the caller may free them immediately.
///
/// **This takes no struct by value**, so [`crate::abi::layouts::QUORRA_ABI_VERSION`] does not move: an entry
/// point *added* is one an old caller never calls.
///
/// # Safety
///
/// See the module documentation. `certificates` and `lengths` are readable for `count` elements,
/// each `certificates[i]` readable for `lengths[i]` bytes; `names`, where not null, is readable for
/// `count` pointers to NUL-terminated strings; `source` is a NUL-terminated string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_trust_anchors(
    viewer: *mut Session,
    certificates: *const *const u8,
    lengths: *const usize,
    names: *const *const c_char,
    count: usize,
    source: *const c_char,
    at: i64,
    acceptance: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(acceptance) = AcceptKind::from_code(acceptance) else {
        return Status::WrongKind.code();
    };
    if count > 0 && (certificates.is_null() || lengths.is_null()) {
        return Status::NullArgument.code();
    }
    let mut held = Vec::with_capacity(count);
    for index in 0..count {
        let pointer = *certificates.add(index);
        let length = *lengths.add(index);
        if pointer.is_null() {
            return Status::NullArgument.code();
        }
        let name = if names.is_null() {
            format!("certificate {}", index.saturating_add(1))
        } else {
            let given = *names.add(index);
            if given.is_null() {
                format!("certificate {}", index.saturating_add(1))
            } else {
                CStr::from_ptr(given).to_string_lossy().into_owned()
            }
        };
        held.push((name, core::slice::from_raw_parts(pointer, length).to_vec()));
    }
    let source = if source.is_null() {
        String::new()
    } else {
        CStr::from_ptr(source).to_string_lossy().into_owned()
    };
    let policy = viewer_core::TrustPolicy {
        anchors: pdf_signature::trust::Supply::of(
            held,
            source,
            pdf_signature::x509::Instant::from_unix_seconds(at),
        ),
        acceptance: acceptance.acceptance(),
    };
    *events = Box::into_raw(Box::new(viewer.trust(policy)));
    Status::Ok.code()
}

/// ISO 32000-2 §8.10.4's target documents: which files a reference `XObject` may import a page from.
///
/// **A library with no filesystem, so the files arrive from the caller or not at all.** §8.10.4.1
/// writes a `shall` for a processor that imports the referenced page and a `shall` for one that
/// draws the proxy instead, and which of the two this library is depends entirely on whether the
/// target file has been handed to it. A caller that never calls this gets what it got before this
/// entry point existed: every reference `XObject` draws its proxy, and nothing is reported.
///
/// **Which file a reference names is decided by §14.4's identifier and never by a path.** That
/// clause states the match itself — the permanent identifier decides which file, and a difference
/// in the changing one means "a different version of the correct PDF file has been found", which
/// is said and drawn rather than refused. Nothing a *document* writes selects a member of this
/// list, which is what keeps a file from choosing what this process reads.
///
/// `files` is `count` pointers to whole PDF files and `lengths` their lengths. `names` may be null,
/// or `count` NUL-terminated strings naming each one — whatever the caller knows them by, which is
/// what a refusal to open one is reported against. `source` is the sentence saying where they came
/// from; a reader shown a page out of another file is owed *which file, on whose say-so*.
///
/// A `count` of zero withdraws the files. The bytes are copied before this returns, so the caller
/// may free them immediately.
///
/// **This takes no struct by value**, so [`crate::abi::layouts::QUORRA_ABI_VERSION`] does not move: an entry
/// point *added* is one an old caller never calls.
///
/// # Safety
///
/// See the module documentation. `files` and `lengths` are readable for `count` elements, each
/// `files[i]` readable for `lengths[i]` bytes; `names`, where not null, is readable for `count`
/// pointers to NUL-terminated strings; `source` is a NUL-terminated string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_reference_files(
    viewer: *mut Session,
    files: *const *const u8,
    lengths: *const usize,
    names: *const *const c_char,
    count: usize,
    source: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    if count > 0 && (files.is_null() || lengths.is_null()) {
        return Status::NullArgument.code();
    }
    let mut held = Vec::with_capacity(count);
    for index in 0..count {
        let pointer = *files.add(index);
        let length = *lengths.add(index);
        if pointer.is_null() {
            return Status::NullArgument.code();
        }
        let name = if names.is_null() {
            format!("file {}", index.saturating_add(1))
        } else {
            let given = *names.add(index);
            if given.is_null() {
                format!("file {}", index.saturating_add(1))
            } else {
                CStr::from_ptr(given).to_string_lossy().into_owned()
            }
        };
        held.push((name, core::slice::from_raw_parts(pointer, length).to_vec()));
    }
    let source = if source.is_null() {
        String::new()
    } else {
        CStr::from_ptr(source).to_string_lossy().into_owned()
    };
    *events = Box::into_raw(Box::new(viewer.reference_files(
        viewer_core::ReferenceFiles {
            files: held,
            source,
        },
    )));
    Status::Ok.code()
}

/// ISO 32000-2 §8.11.4.4's two usage categories about the *reader*: who is reading, and in what
/// language.
///
/// **A library that is told, or a library that says it was not told.** Table 100 lets a document
/// name "one or more users for whom this optional content group is primarily intended" and the
/// language of a group's content; §8.11.4.4 says what a processor does with each — match the
/// names "with the user's identification", and select "based on the language and locale of the
/// application". Neither is a fact a PDF holds, and a document that could assert who is reading
/// would be choosing its own audience. A caller that never calls this gets what it got before
/// this entry point existed: both categories reported unanswered, and every group left at the
/// state the document's configuration gave it.
///
/// The three name lists are Table 100's three `/User` `/Type` values and may not be merged: `Ind`
/// is the individual, `Ttl` the title or position, `Org` the organisation, and a document asking
/// about one of them is not asking about the others. Each may be null with a count of zero.
///
/// `language` is a NUL-terminated language tag as §14.9.2.2 defines one — "a Language-Tag as
/// defined in BCP 47", such as `es-MX` — or null, which is *nobody has said*. A null tag and an
/// empty one are different: §14.9.2.2 gives the empty string a meaning of its own, "to indicate
/// that the language is unknown", and this entry point does not put that meaning in a caller's
/// mouth for passing a pointer to a zero byte, so an empty string is read as null.
///
/// Calling it with every count zero and a null `language` withdraws the answer. The strings are
/// copied before this returns.
///
/// **This takes no struct by value**, so [`crate::abi::layouts::QUORRA_ABI_VERSION`] does not move: an
/// entry point *added* is one an old caller never calls.
///
/// # Safety
///
/// See the module documentation. Each of `names`, `titles` and `organisations`, where not null, is
/// readable for its count of pointers to NUL-terminated strings; `language` is a NUL-terminated
/// string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_audience(
    viewer: *mut Session,
    names: *const *const c_char,
    name_count: usize,
    titles: *const *const c_char,
    title_count: usize,
    organisations: *const *const c_char,
    organisation_count: usize,
    language: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let read = |list: *const *const c_char, count: usize| -> Option<Vec<String>> {
        if count == 0 {
            return Some(Vec::new());
        }
        if list.is_null() {
            return None;
        }
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            let given = *list.add(index);
            if given.is_null() {
                return None;
            }
            out.push(CStr::from_ptr(given).to_string_lossy().into_owned());
        }
        Some(out)
    };
    let (Some(individual), Some(title), Some(organisation)) = (
        read(names, name_count),
        read(titles, title_count),
        read(organisations, organisation_count),
    ) else {
        return Status::NullArgument.code();
    };
    let language = if language.is_null() {
        None
    } else {
        let tag = CStr::from_ptr(language).to_string_lossy().into_owned();
        (!tag.is_empty()).then_some(tag)
    };
    *events = Box::into_raw(Box::new(viewer.audience(
        pdf_model::optional_content::Audience {
            reader: pdf_model::optional_content::Reader {
                individual,
                title,
                organisation,
            },
            language,
        },
    )));
    Status::Ok.code()
}

/// §10.8.3's separation simulation: which of two pictures of the same document this reader wants.
///
/// **A preference and not a restriction level.** §10.8.3 conditions itself on something no file
/// states — "\[i]f it is important for the colours of the display for a PDF, on a device that
/// normally would not be used to produce separations, to more closely match those produced when
/// using separations" — and §10.8.1 says whose choice that is: "\[w]hether separations are produced
/// is up to the processing software." So it is one value a caller supplies, with two states and
/// nothing to ask anybody.
///
/// A caller that never calls this gets what it got before this entry point existed: §10.8.2's
/// alternate colour space and its tint transform, which that clause states as what a device not
/// making separations does. Changing it re-interprets every page, because the answer decides what
/// colour every mark on them is.
///
/// **This takes no struct by value**, so [`crate::abi::layouts::QUORRA_ABI_VERSION`] does not move: an
/// entry point *added* is one an old caller never calls.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_separations(
    viewer: *mut Session,
    simulate: bool,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.separations(simulate)));
    Status::Ok.code()
}

/// The name a document opened **beside** the one showing would be called by.
///
/// ISO 32000-2 Table 203's `/NewWindow` (§12.6.4.3's remote go-to) and Table 204's (§12.6.4.4's
/// embedded one) each say a destination in another file may be opened in a window of its own, and
/// each defers the absent case to "its preference". Whether *this caller* has a second place to
/// put a document is a fact about the caller and about no file, so it arrives here.
///
/// `reserved` is false to empty the reserve, which is what every caller had before this entry
/// point existed: a destination replaces the document it was reached from, and the difference is
/// said out loud on a `QUORRA_EVENT_REPORTED`. `reserved` true holds `name` out for the next
/// action that asks for a new window — **once**: the moment a document opens under it the reserve
/// is empty again, so a caller offers a fresh name per tab. A name offered and not used opens
/// nothing.
///
/// The name is the caller's, exactly as [`quorra_open`](super::quorra_open)'s is, and a caller learns which of the two
/// happened from the document on the `QUORRA_EVENT_OPENED` that comes back
/// ([`quorra_event_opened`](super::quorra_event_opened)). Offering a name already open would make the action *replace* that
/// document, which is [`quorra_open`](super::quorra_open)'s own rule; offer one nothing is open under.
///
/// **This takes no struct by value**, so [`crate::abi::layouts::QUORRA_ABI_VERSION`] does not move: an
/// entry point *added* is one an old caller never calls.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_beside(
    viewer: *mut Session,
    reserved: bool,
    name: u64,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.beside(reserved.then_some(name))));
    Status::Ok.code()
}

// ---------------------------------------------------------------------------------------------
// The eight commands a toolkit's own controls, its clock, its network and its scripts need — RFC
// 0010 section 3.1's table, each in C's shape (ADRs 2060, 2061).
// ---------------------------------------------------------------------------------------------

/// §12.7.4.3: a caller's own control lost the keyboard, or Enter in a single-line text field —
/// the moment Table 199's `/K` in its commit form and `/V` wait for.
///
/// A field nobody is typing into does nothing. Where the field's scripts refuse the value, the
/// field shows what it showed before the typing began and the sentence arrives on a
/// `QUORRA_EVENT_REPORTED`.
///
/// # Safety
///
/// See the module documentation. `field` is NUL-terminated UTF-8, §12.7.4.2's fully qualified name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_commit_field(
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
    *events = Box::into_raw(Box::new(viewer.commit_field(field)));
    Status::Ok.code()
}

/// §7.9.4's instant from the caller's clock, which Table 166's `/M` is written as on every
/// annotation a save writes; `known` false withdraws it.
///
/// **`pdf_syntax::Date`'s fields, one argument each** (RFC 0010 section 3.1): `zoned` false is a
/// date that states no zone, which is not the same claim as `Z`, and `offset_minutes` is minutes
/// east of UT. A field outside the clause's range is [`Status::OutOfRange64`] and nothing is sent
/// (ADR 2060).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_clock(
    viewer: *mut Session,
    known: bool,
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    zoned: bool,
    offset_minutes: i16,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let now = if known {
        let Some(date) = session::clock_date(
            (year, month, day),
            (hour, minute, second),
            zoned.then_some(offset_minutes),
        ) else {
            return Status::OutOfRange64.code();
        };
        Some(date)
    } else {
        None
    };
    *events = Box::into_raw(Box::new(viewer.clock(now)));
    Status::Ok.code()
}

/// §12.7.8's form data a server answered with — a `QUORRA_EVENT_SUBMIT`'s answer or a fetched
/// import's, `QUORRA_ANSWERED_*` — imported into the document that asked.
///
/// `source` is where the request went, which the sentences name; `format` is `QUORRA_DATA_*`. A
/// document no longer open takes nothing. What reaches the network, and whether, is the caller's.
///
/// # Safety
///
/// See the module documentation. `source` is NUL-terminated UTF-8; `bytes` is readable for `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_respond(
    viewer: *mut Session,
    document: u64,
    answers: u32,
    source: *const c_char,
    format: u32,
    bytes: *const u8,
    len: usize,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events), false, false) = (
        viewer.as_mut(),
        events.as_mut(),
        source.is_null(),
        bytes.is_null(),
    ) else {
        return Status::NullArgument.code();
    };
    let (Some(answers), Some(format)) = (
        AnsweredKind::from_code(answers),
        DataFormatKind::from_code(format),
    ) else {
        return Status::WrongKind.code();
    };
    let Ok(Some(source)) = owned_text(source) else {
        return Status::NotUtf8.code();
    };
    let body = core::slice::from_raw_parts(bytes, len).to_vec();
    *events = Box::into_raw(Box::new(viewer.respond(
        document,
        answers.answered(),
        source,
        format.format(),
        body,
    )));
    Status::Ok.code()
}

/// What this caller does with a page past `MAX_PIXELS`, or one whose window keeps few of its
/// commands: `QUORRA_OVERSIZE_*`.
///
/// Under `QUORRA_OVERSIZE_LISTED` such a page is asked for at its whole target; a request
/// [`quorra_render_request_oversized`](super::quorra_render_request_oversized) names is answered with [`quorra_render_ready_listed`](super::quorra_render_ready_listed) and
/// drawn a window at a time with [`quorra_render_request_rasterise_window`](super::quorra_render_request_rasterise_window).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_oversize(
    viewer: *mut Session,
    how: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(how) = OversizeKind::from_code(how) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.oversize(how.oversize())));
    Status::Ok.code()
}

/// What the keyboard holds: a script's `event.shift` and `event.modifier`, and whether an arrow
/// key made a choice field's selection.
///
/// Sent whenever it changes, before the pointer or key that raises an event; every open document
/// holds it until it is sent again. `modifier` is the platform's command modifier, named by role
/// (RFC 0010 section 7).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_keys(
    viewer: *mut Session,
    shift: bool,
    modifier: bool,
    arrows: bool,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let keys = pdf_model::view::Keys {
        shift,
        modifier,
        arrows,
    };
    *events = Box::into_raw(Box::new(viewer.keys(keys)));
    Status::Ok.code()
}

/// RFC 0008 section 6.3's level for every document's scripts: `QUORRA_SCRIPTS_*`.
///
/// At every level but off each document is handed a `pdf-script-worker`, started at its first
/// trigger and never before, found beside the running executable or where `PDF_SCRIPT_WORKER`
/// names it (ADR 2061).
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_scripts(
    viewer: *mut Session,
    level: u32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let Some(level) = ScriptsKind::from_code(level) else {
        return Status::WrongKind.code();
    };
    *events = Box::into_raw(Box::new(viewer.scripts(level.level())));
    Status::Ok.code()
}

/// The answer to a `QUORRA_EVENT_ASKING_TO_RUN_SCRIPTS`, for the document the event named.
///
/// `true` runs what was withheld; `false` keeps every script withheld until the document closes.
/// A document that is not asking does nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_answer_scripts(
    viewer: *mut Session,
    document: u64,
    proceed: bool,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    *events = Box::into_raw(Box::new(viewer.answer_scripts(document, proceed)));
    Status::Ok.code()
}

/// The answer to a `QUORRA_EVENT_SCRIPT_ASKING`: `QUORRA_SCRIPT_ANSWER_*`.
///
/// `button` (`QUORRA_ALERT_BUTTON_*`) is read for a press and `text` for a typed answer, where a
/// null `text` is `app.response`'s `null`; the other is ignored. An answer for a script no longer
/// waiting does nothing.
///
/// # Safety
///
/// See the module documentation. `text` is NUL-terminated UTF-8, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_answer_script(
    viewer: *mut Session,
    document: u64,
    how: u32,
    button: u32,
    text: *const c_char,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(events)) = (viewer.as_mut(), events.as_mut()) else {
        return Status::NullArgument.code();
    };
    let answer = match script_answer(how, button, owned_text(text)) {
        Ok(answer) => answer,
        Err(status) => return status.code(),
    };
    *events = Box::into_raw(Box::new(viewer.answer_script(document, answer)));
    Status::Ok.code()
}

/// Which `QUORRA_DATA_*` a server's answer is in, by its media type — §12.7.8.1's
/// `application/vnd.fdf` and the registry's names, as every window reads them.
///
/// # Safety
///
/// See the module documentation. `media_type` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_data_format_of_media_type(
    media_type: *const c_char,
    format: *mut u32,
) -> c_int {
    let (Some(format), false) = (format.as_mut(), media_type.is_null()) else {
        return Status::NullArgument.code();
    };
    let Ok(Some(media_type)) = owned_text(media_type) else {
        return Status::NotUtf8.code();
    };
    *format = DataFormatKind::of(session::data_format_of_media_type(&media_type)) as u32;
    Status::Ok.code()
}

/// Which `QUORRA_DATA_*` an import's file is in, by the name its URI or file specification ends
/// in — `pdf_model::action::data_format`, the one opinion both §12.7.6.4's route and Annex O's take.
///
/// # Safety
///
/// See the module documentation. `name` is NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_data_format_of_name(
    name: *const c_char,
    format: *mut u32,
) -> c_int {
    let (Some(format), false) = (format.as_mut(), name.is_null()) else {
        return Status::NullArgument.code();
    };
    let Ok(Some(name)) = owned_text(name) else {
        return Status::NotUtf8.code();
    };
    *format = DataFormatKind::of(pdf_model::action::data_format(&name)) as u32;
    Status::Ok.code()
}

// ---------------------------------------------------------------------------------------------
// The keyboard: `viewer_host::keys`' table for a C caller (RFC 0010 step N1, ADR 2072).
// ---------------------------------------------------------------------------------------------

/// A key press that reached the page: `key` is `QUORRA_KEY_*`, `shift` and `command` what was
/// held — `command` the platform's command modifier, Control here and Command on macOS — `present`
/// is `QUORRA_PRESENT_*` as the window stands, and `warned` whether the window is saying that a
/// draw is taking long.
///
/// Asked as the three windows ask: a push-button §12.5.1's focus is on takes Space and Enter
/// first, then `viewer_host::meaning`'s table. A command the table means is sent here and
/// `events` is what it produced; what only the window can do is `act`, `QUORRA_ACT_*`, with the
/// logical distance a `QUORRA_ACT_SCROLL_BY` moves in `by`. `QUORRA_ACT_SAY` leaves a sentence for
/// [`quorra_key_said`]. A key or a presentation state this build does not define is
/// `QUORRA_WRONG_KIND` and nothing is sent.
///
/// # Safety
///
/// See the module documentation. `by` is writable, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_key_press(
    viewer: *mut Session,
    key: u32,
    shift: bool,
    command: bool,
    present: u32,
    warned: bool,
    act: *mut u32,
    by: *mut f32,
    events: *mut *mut Events,
) -> c_int {
    let (Some(viewer), Some(act), Some(events)) = (viewer.as_mut(), act.as_mut(), events.as_mut())
    else {
        return Status::NullArgument.code();
    };
    let (Some(key), Some(present)) = (KeyKind::from_code(key), PresentKind::from_code(present))
    else {
        return Status::WrongKind.code();
    };
    let held = viewer_host::Modifiers {
        shift,
        ctrl: command,
    };
    let mode = match present {
        PresentKind::Off => viewer_host::Mode::Reading,
        PresentKind::On => viewer_host::Mode::Presenting,
    };
    let waiting = if warned {
        viewer_host::Waiting::Warned
    } else {
        viewer_host::Waiting::Nothing
    };
    let (produced, kind, distance) = viewer.press(key.key(), held, mode, waiting);
    *act = kind.code();
    if let Some(by) = by.as_mut() {
        *by = distance;
    }
    *events = Box::into_raw(Box::new(produced));
    Status::Ok.code()
}

/// The sentence the last [`quorra_key_press`] answered `QUORRA_ACT_SAY` for, in the two-call idiom
/// [`quorra_events_describe`](super::quorra_events_describe) uses. `QUORRA_NO_ANSWER` where the last press owed none.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_key_said(
    viewer: *const Session,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(viewer) = viewer.as_ref() else {
        return Status::NullArgument.code();
    };
    match viewer.said() {
        Some(sentence) => copy_out(sentence, out, cap, needed),
        None => Status::NoAnswer.code(),
    }
}

/// How many keys this library binds, against the header's `QUORRA_KEY_COUNT`.
///
/// **Not part of [`quorra_abi_check`](super::quorra_abi_check)**, for [`quorra_control_kind_count`](super::quorra_control_kind_count)'s reason: a key is a
/// number the caller passes and an undefined one is refused by name. A larger count than the
/// header's is a key this caller does not translate yet.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_key_count() -> u32 {
    KeyKind::COUNT
}

/// How many acts [`quorra_key_press`] answers with, against the header's `QUORRA_ACT_COUNT`.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_act_count() -> u32 {
    ActKind::COUNT
}

/// The name of an act, NUL-terminated and never freed. `"unknown"` for one this build does not
/// define.
#[unsafe(no_mangle)]
pub extern "C" fn quorra_act_name(act: u32) -> *const c_char {
    let name = ActKind::from_code(act).map_or("unknown\0", ActKind::name);
    name.as_ptr().cast::<c_char>()
}

/// §6.3.2.2's "unless otherwise instructed": who draws §12.7's widget appearances.
///
/// `QUORRA_DELEGATE_DELEGATED` removes from the page **exactly the widgets [`quorra_fields_read`](super::quorra_fields_read)
/// answered for**, so a caller that has placed real controls over them draws each part of the page
/// once. A widget §12.7.4.2 leaves "simply a Widget annotation" keeps its appearance, because no
/// control replaced it. Changing this re-interprets the page, because §12.5.5's appearance streams
/// are drawing commands rather than pixels.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_delegate(
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
