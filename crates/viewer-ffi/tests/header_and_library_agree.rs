//! `include/pdf_viewer.h` against `src/abi.rs`, read back as text.
//!
//! **This is what buys back the one thing `cbindgen` would have given.** The header is
//! hand-written on purpose — it is the artefact a C programmer reads, with the reason for each
//! shape beside it, rather than a derivative of Rust types that a person then has to read anyway
//! — and the price of that choice is that it can drift. So it is checked instead of generated:
//!
//! - every `#[unsafe(no_mangle)]` entry point is declared exactly once in the header, and every
//!   `pdfv_` function the header declares exists in the Rust. A missing declaration is a symbol a
//!   caller cannot reach; an extra one is a link error in somebody else's build;
//! - every `PDFV_` constant is the number the Rust enumeration gives it. **This is the one that
//!   would fail silently**: a `#define PDFV_EVENT_SAVED 12u` beside a Rust `Saved = 11` produces
//!   a program that compiles, links, runs, and quietly acts on the wrong events.
//!
//! `tests/a_c_program_drives_the_abi.rs` is the other half and catches a different class: this one
//! reads text, that one hands the header to a compiler and the symbols to a linker.

#![expect(
    clippy::expect_used,
    reason = "test code: a source file that cannot be read must fail loudly rather than pass by \
              doing nothing"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use viewer_ffi::{
    ControlKind, DelegateKind, EventKind, FocusKind, MarkupKind, PageTargetKind, PixelFormat,
    PointerKind, PresentKind, PurposeKind, RestrictKind, RowKind, SelectKind, Status, TextKind,
};

/// The header, with every comment removed.
///
/// Comments name functions and constants in prose — "`pdfv_frame_info()` below" — and a check
/// that counted those would be checking the documentation rather than the declarations.
fn header_without_comments() -> String {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/pdf_viewer.h"))
            .expect("this crate has a header");
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(at) = rest.find("/*") {
        out.push_str(&rest[..at]);
        let after = &rest[at.saturating_add(2)..];
        let Some(end) = after.find("*/") else {
            rest = "";
            break;
        };
        rest = &after[end.saturating_add(2)..];
    }
    out.push_str(rest);
    out
}

/// Every `pdfv_…` name the argument calls as a function.
fn called_names(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    while at < bytes.len() {
        if text[at..].starts_with("pdfv_") {
            let mut end = at;
            while end < bytes.len() && (bytes[end].is_alphanumeric() || bytes[end] == '_') {
                end = end.saturating_add(1);
            }
            // Only a name immediately followed by `(` is a declaration or a definition; a name
            // followed by ` *` is a type and a bare one is an argument.
            if bytes.get(end) == Some(&'(') {
                found.insert(text[at..end].to_owned());
            }
            at = end;
        } else {
            at = at.saturating_add(1);
        }
    }
    found
}

/// Every entry point the Rust exports, taken from the attribute rather than from a list.
fn exported_names() -> BTreeSet<String> {
    let abi = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/abi.rs"))
        .expect("this crate has an abi module");
    let mut found = BTreeSet::new();
    let mut lines = abi.lines();
    while let Some(line) = lines.next() {
        if line.trim() != "#[unsafe(no_mangle)]" {
            continue;
        }
        let declaration = lines.next().unwrap_or_default();
        let name = declaration
            .split("fn ")
            .nth(1)
            .and_then(|rest| rest.split(['(', '<']).next())
            .unwrap_or_default()
            .trim();
        assert!(
            name.starts_with("pdfv_"),
            "an exported function not called pdfv_…: {declaration}"
        );
        found.insert(name.to_owned());
    }
    found
}

/// Every `#define PDFV_NAME value` in the header.
fn defined_constants(text: &str) -> BTreeMap<String, i64> {
    let mut found = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("#define PDFV_") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let value = value.trim_end_matches('u');
        if let Ok(number) = value.parse::<i64>() {
            found.insert(format!("PDFV_{name}"), number);
        }
    }
    found
}

#[test]
fn every_entry_point_is_declared_once_in_the_header_and_nowhere_else() {
    let header = header_without_comments();
    let declared = called_names(&header);
    let exported = exported_names();
    assert_eq!(
        exported.len(),
        111,
        "the count `unsafe_position.rs` also states"
    );
    let missing: Vec<&String> = exported.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "exported by the library and not declared in the header: {missing:?}"
    );
    let extra: Vec<&String> = declared.difference(&exported).collect();
    assert!(
        extra.is_empty(),
        "declared in the header and not exported by the library: {extra:?}"
    );
    // And each appears exactly once: `called_names` is a set, so a duplicate declaration would be
    // invisible to the comparison above.
    for name in &exported {
        let occurrences = header.matches(&format!("{name}(")).count();
        assert_eq!(occurrences, 1, "{name} is declared {occurrences} times");
    }
}

#[test]
fn every_constant_in_the_header_is_the_number_the_library_gives_it() {
    let defined = defined_constants(&header_without_comments());
    let mut expected: BTreeMap<String, i64> = BTreeMap::new();
    the_identity_and_the_statuses(&mut expected);
    the_event_kinds(&mut expected);
    the_argument_enumerations(&mut expected);
    the_answered_enumerations(&mut expected);
    the_field_flags(&mut expected);

    assert_eq!(
        defined, expected,
        "the header's numbers and the library's have come apart"
    );
}

/// `PDFV_ABI_VERSION`, the event-kind count, and every `pdfv_status`.
fn the_identity_and_the_statuses(expected: &mut BTreeMap<String, i64>) {
    expected.insert(
        "PDFV_ABI_VERSION".to_owned(),
        i64::from(viewer_ffi::abi::PDFV_ABI_VERSION),
    );
    expected.insert(
        "PDFV_EVENT_KIND_COUNT".to_owned(),
        i64::from(EventKind::COUNT),
    );
    for (name, status) in [
        ("PDFV_OK", Status::Ok),
        ("PDFV_NULL_ARGUMENT", Status::NullArgument),
        ("PDFV_OUT_OF_RANGE", Status::OutOfRange),
        ("PDFV_WRONG_KIND", Status::WrongKind),
        ("PDFV_BUFFER_TOO_SMALL", Status::BufferTooSmall),
        ("PDFV_NO_ANSWER", Status::NoAnswer),
        ("PDFV_NOT_UTF8", Status::NotUtf8),
        ("PDFV_RENDER_REFUSED", Status::RenderRefused),
        ("PDFV_NUMBER_OUT_OF_RANGE", Status::OutOfRange64),
    ] {
        expected.insert(name.to_owned(), i64::from(status.code()));
    }
}

/// The sixteen kinds an event arrives as, which is the one enumeration `pdfv_abi_check` counts.
fn the_event_kinds(expected: &mut BTreeMap<String, i64>) {
    for (name, kind) in [
        ("PDFV_EVENT_OPENED", EventKind::Opened),
        ("PDFV_EVENT_OPEN_FAILED", EventKind::OpenFailed),
        ("PDFV_EVENT_PASSWORD_REQUIRED", EventKind::PasswordRequired),
        ("PDFV_EVENT_CLOSED", EventKind::Closed),
        ("PDFV_EVENT_PAGE_CHANGED", EventKind::PageChanged),
        ("PDFV_EVENT_NEEDS_RENDER", EventKind::NeedsRender),
        ("PDFV_EVENT_DAMAGE", EventKind::Damage),
        ("PDFV_EVENT_OPEN_URI", EventKind::OpenUri),
        ("PDFV_EVENT_NEEDS_FILE", EventKind::NeedsFile),
        ("PDFV_EVENT_TRANSITION", EventKind::Transition),
        ("PDFV_EVENT_DIRTY", EventKind::Dirty),
        ("PDFV_EVENT_SAVED", EventKind::Saved),
        ("PDFV_EVENT_EXTRACTED", EventKind::Extracted),
        ("PDFV_EVENT_REFUSED", EventKind::Refused),
        ("PDFV_EVENT_REPORTED", EventKind::Reported),
        // **This row did not exist until the five-hundred-and-eleventh session, and neither did
        // the `#define`.** `Event::Searched` moved `PDFV_EVENT_KIND_COUNT` from 15 to 16 in the
        // four-hundred-and-fourteenth and no constant was added for the kind itself, so a C caller
        // switching on kinds had to write `15` by hand — and this test could not see it, because
        // it compares the constants the header *has* against the ones it is told to expect and
        // nobody told it to expect this one. The lesson is the map's rather than the header's: a
        // table of expectations is only as complete as the person who wrote it.
        ("PDFV_EVENT_SEARCHED", EventKind::Searched),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
}

/// The enumerations this ABI *takes*, which refuse a number they do not define.
fn the_argument_enumerations(expected: &mut BTreeMap<String, i64>) {
    for (name, kind) in [
        ("PDFV_PAGE_INDEX", PageTargetKind::Index),
        ("PDFV_PAGE_FIRST", PageTargetKind::First),
        ("PDFV_PAGE_LAST", PageTargetKind::Last),
        ("PDFV_PAGE_NEXT", PageTargetKind::Next),
        ("PDFV_PAGE_PREVIOUS", PageTargetKind::Previous),
        ("PDFV_PAGE_RELATIVE", PageTargetKind::Relative),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("PDFV_ZOOM_FIT_PAGE", viewer_ffi::ZoomKind::FitPage),
        ("PDFV_ZOOM_FIT_WIDTH", viewer_ffi::ZoomKind::FitWidth),
        ("PDFV_ZOOM_FIT_HEIGHT", viewer_ffi::ZoomKind::FitHeight),
        ("PDFV_ZOOM_SCALE", viewer_ffi::ZoomKind::Scale),
        ("PDFV_ZOOM_IN", viewer_ffi::ZoomKind::In),
        ("PDFV_ZOOM_OUT", viewer_ffi::ZoomKind::Out),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    // The enumerations the five-hundred-and-eleventh session added, each read off its own
    // discriminant rather than written twice.
    for (name, kind) in [
        ("PDFV_POINTER_MOVED", PointerKind::Moved),
        ("PDFV_POINTER_PRESSED", PointerKind::Pressed),
        ("PDFV_POINTER_DRAGGED", PointerKind::Dragged),
        ("PDFV_POINTER_RELEASED", PointerKind::Released),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("PDFV_SELECT_ALL", SelectKind::All),
        ("PDFV_SELECT_NONE", SelectKind::None),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("PDFV_FOCUS_NEXT", FocusKind::Next),
        ("PDFV_FOCUS_PREVIOUS", FocusKind::Previous),
        ("PDFV_FOCUS_NONE", FocusKind::None),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("PDFV_RESTRICT_ON", RestrictKind::On),
        ("PDFV_RESTRICT_OFF", RestrictKind::Off),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("PDFV_PRESENT_OFF", PresentKind::Off),
        ("PDFV_PRESENT_ON", PresentKind::On),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("PDFV_DELEGATE_DRAWN", DelegateKind::Drawn),
        ("PDFV_DELEGATE_DELEGATED", DelegateKind::Delegated),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("PDFV_MARKUP_HIGHLIGHT", MarkupKind::Highlight),
        ("PDFV_MARKUP_UNDERLINE", MarkupKind::Underline),
        ("PDFV_MARKUP_STRIKE_OUT", MarkupKind::StrikeOut),
        ("PDFV_MARKUP_SQUIGGLY", MarkupKind::Squiggly),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    expected.insert(
        "PDFV_PURPOSE_IMPORT_DATA".to_owned(),
        i64::from(PurposeKind::ImportData.code()),
    );
    // Table 164's two two-valued entries, which `pdfv_event_transition` answers as numbers.
    for (name, value) in [
        ("PDFV_DIMENSION_HORIZONTAL", 0),
        ("PDFV_DIMENSION_VERTICAL", 1),
        ("PDFV_MOTION_INWARD", 0),
        ("PDFV_MOTION_OUTWARD", 1),
    ] {
        expected.insert(name.to_owned(), value);
    }
}

/// The enumerations this ABI *answers with*, each named and each with a count of its own.
fn the_answered_enumerations(expected: &mut BTreeMap<String, i64>) {
    expected.insert(
        "PDFV_FORMAT_RGBA8".to_owned(),
        i64::from(PixelFormat::Rgba8.code()),
    );
    for (name, kind) in [
        ("PDFV_CONTROL_ENTRY", ControlKind::Entry),
        ("PDFV_CONTROL_CHECK", ControlKind::Check),
        ("PDFV_CONTROL_RADIO", ControlKind::Radio),
        ("PDFV_CONTROL_PUSH", ControlKind::Push),
        ("PDFV_CONTROL_COMBO", ControlKind::Combo),
        ("PDFV_CONTROL_LIST", ControlKind::List),
        ("PDFV_CONTROL_SIGNATURE", ControlKind::Signature),
        ("PDFV_CONTROL_UNSTATED", ControlKind::Unstated),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
    expected.insert(
        "PDFV_CONTROL_KIND_COUNT".to_owned(),
        i64::from(ControlKind::COUNT),
    );
    for (name, kind) in [
        ("PDFV_ROW_ACTIVATE", RowKind::Activate),
        ("PDFV_ROW_TOGGLE", RowKind::Toggle),
        ("PDFV_ROW_EXTRACT", RowKind::Extract),
        ("PDFV_ROW_INERT", RowKind::Inert),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
    expected.insert("PDFV_ROW_KIND_COUNT".to_owned(), i64::from(RowKind::COUNT));
    for (name, kind) in [
        ("PDFV_TEXT_QUALIFIED", TextKind::Qualified),
        ("PDFV_TEXT_SHOWN", TextKind::Shown),
        ("PDFV_TEXT_PARTIAL", TextKind::Partial),
        ("PDFV_TEXT_LABEL", TextKind::Label),
        ("PDFV_TEXT_EXPORT", TextKind::Export),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
}

/// `pdfv_field_control`'s flag word.
fn the_field_flags(expected: &mut BTreeMap<String, i64>) {
    // Written out here rather than derived, because the numbers
    // *are* the ABI: a bit that moved would be a caller acting on the wrong flag, and a loop over
    // `1 << n` would agree with whatever the source said rather than with what was published.
    for (name, bit) in [
        ("PDFV_FIELD_READ_ONLY", viewer_ffi::form::FLAG_READ_ONLY),
        ("PDFV_FIELD_REQUIRED", viewer_ffi::form::FLAG_REQUIRED),
        ("PDFV_FIELD_NO_EXPORT", viewer_ffi::form::FLAG_NO_EXPORT),
        ("PDFV_FIELD_MULTILINE", viewer_ffi::form::FLAG_MULTILINE),
        ("PDFV_FIELD_PASSWORD", viewer_ffi::form::FLAG_PASSWORD),
        ("PDFV_FIELD_FILE_SELECT", viewer_ffi::form::FLAG_FILE_SELECT),
        (
            "PDFV_FIELD_DO_NOT_SPELL_CHECK",
            viewer_ffi::form::FLAG_DO_NOT_SPELL_CHECK,
        ),
        (
            "PDFV_FIELD_DO_NOT_SCROLL",
            viewer_ffi::form::FLAG_DO_NOT_SCROLL,
        ),
        ("PDFV_FIELD_COMB", viewer_ffi::form::FLAG_COMB),
        ("PDFV_FIELD_RICH_TEXT", viewer_ffi::form::FLAG_RICH_TEXT),
        (
            "PDFV_FIELD_NO_TOGGLE_TO_OFF",
            viewer_ffi::form::FLAG_NO_TOGGLE_TO_OFF,
        ),
        (
            "PDFV_FIELD_RADIOS_IN_UNISON",
            viewer_ffi::form::FLAG_RADIOS_IN_UNISON,
        ),
        ("PDFV_FIELD_ON", viewer_ffi::form::FLAG_ON),
        ("PDFV_FIELD_EDITABLE", viewer_ffi::form::FLAG_EDITABLE),
        (
            "PDFV_FIELD_MULTI_SELECT",
            viewer_ffi::form::FLAG_MULTI_SELECT,
        ),
        (
            "PDFV_FIELD_COMMIT_ON_SEL",
            viewer_ffi::form::FLAG_COMMIT_ON_SELECTION,
        ),
        ("PDFV_FIELD_OBSCURED", viewer_ffi::form::FLAG_OBSCURED),
    ] {
        expected.insert(name.to_owned(), i64::from(bit));
    }
}
