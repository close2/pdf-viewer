//! `include/quorra.h` against `src/abi.rs`, read back as text.
//!
//! **This is what buys back the one thing `cbindgen` would have given.** The header is
//! hand-written on purpose — it is the artefact a C programmer reads, with the reason for each
//! shape beside it, rather than a derivative of Rust types that a person then has to read anyway
//! — and the price of that choice is that it can drift. So it is checked instead of generated:
//!
//! - every `#[unsafe(no_mangle)]` entry point is declared exactly once in the header, and every
//!   `quorra_` function the header declares exists in the Rust. A missing declaration is a symbol a
//!   caller cannot reach; an extra one is a link error in somebody else's build;
//! - every `QUORRA_` constant is the number the Rust enumeration gives it. **This is the one that
//!   would fail silently**: a `#define QUORRA_EVENT_SAVED 12u` beside a Rust `Saved = 11` produces
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
    AttachKind, BoundaryKind, BoxKind, CollectionViewKind, ColumnKind, ColumnTextKind, ControlKind,
    DelegateKind, DirectionKind, DuplexKind, ElementKind, EventKind, FocusKind, FolderTextKind,
    InitialKind, MarkupKind, NoteKind, OrderKind, PageModeKind, PageTargetKind, PixelFormat,
    PointerKind, PreferenceKey, PresentKind, PrintScalingKind, PurposeKind, RestrictKind, RowKind,
    ScopeKind, SelectKind, ShortfallKind, Status, TextKind,
};

/// The header, with every comment removed.
///
/// Comments name functions and constants in prose — "`quorra_frame_info()` below" — and a check
/// that counted those would be checking the documentation rather than the declarations.
fn header_without_comments() -> String {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/quorra.h"))
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

/// Every `quorra_…` name the argument calls as a function.
fn called_names(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    while at < bytes.len() {
        if text[at..].starts_with("quorra_") {
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
            name.starts_with("quorra_"),
            "an exported function not called quorra_…: {declaration}"
        );
        found.insert(name.to_owned());
    }
    found
}

/// Every `#define QUORRA_NAME value` in the header.
fn defined_constants(text: &str) -> BTreeMap<String, i64> {
    let mut found = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("#define QUORRA_") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let value = value.trim_end_matches('u');
        if let Ok(number) = value.parse::<i64>() {
            found.insert(format!("QUORRA_{name}"), number);
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
        177,
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
    the_other_half_of_the_queries(&mut expected);

    assert_eq!(
        defined, expected,
        "the header's numbers and the library's have come apart"
    );
}

/// `QUORRA_ABI_VERSION`, the event-kind count, and every `quorra_status`.
fn the_identity_and_the_statuses(expected: &mut BTreeMap<String, i64>) {
    expected.insert(
        "QUORRA_ABI_VERSION".to_owned(),
        i64::from(viewer_ffi::abi::QUORRA_ABI_VERSION),
    );
    expected.insert(
        "QUORRA_EVENT_KIND_COUNT".to_owned(),
        i64::from(EventKind::COUNT),
    );
    for (name, status) in [
        ("QUORRA_OK", Status::Ok),
        ("QUORRA_NULL_ARGUMENT", Status::NullArgument),
        ("QUORRA_OUT_OF_RANGE", Status::OutOfRange),
        ("QUORRA_WRONG_KIND", Status::WrongKind),
        ("QUORRA_BUFFER_TOO_SMALL", Status::BufferTooSmall),
        ("QUORRA_NO_ANSWER", Status::NoAnswer),
        ("QUORRA_NOT_UTF8", Status::NotUtf8),
        ("QUORRA_RENDER_REFUSED", Status::RenderRefused),
        ("QUORRA_NUMBER_OUT_OF_RANGE", Status::OutOfRange64),
    ] {
        expected.insert(name.to_owned(), i64::from(status.code()));
    }
}

/// The sixteen kinds an event arrives as, which is the one enumeration `quorra_abi_check` counts.
fn the_event_kinds(expected: &mut BTreeMap<String, i64>) {
    for (name, kind) in [
        ("QUORRA_EVENT_OPENED", EventKind::Opened),
        ("QUORRA_EVENT_OPEN_FAILED", EventKind::OpenFailed),
        ("QUORRA_EVENT_PASSWORD_REQUIRED", EventKind::PasswordRequired),
        ("QUORRA_EVENT_CLOSED", EventKind::Closed),
        ("QUORRA_EVENT_PAGE_CHANGED", EventKind::PageChanged),
        ("QUORRA_EVENT_NEEDS_RENDER", EventKind::NeedsRender),
        ("QUORRA_EVENT_DAMAGE", EventKind::Damage),
        ("QUORRA_EVENT_OPEN_URI", EventKind::OpenUri),
        ("QUORRA_EVENT_NEEDS_FILE", EventKind::NeedsFile),
        ("QUORRA_EVENT_TRANSITION", EventKind::Transition),
        ("QUORRA_EVENT_DIRTY", EventKind::Dirty),
        ("QUORRA_EVENT_SAVED", EventKind::Saved),
        ("QUORRA_EVENT_EXTRACTED", EventKind::Extracted),
        ("QUORRA_EVENT_REFUSED", EventKind::Refused),
        ("QUORRA_EVENT_REPORTED", EventKind::Reported),
        // **This row did not exist until the five-hundred-and-eleventh session, and neither did
        // the `#define`.** `Event::Searched` moved `QUORRA_EVENT_KIND_COUNT` from 15 to 16 in the
        // four-hundred-and-fourteenth and no constant was added for the kind itself, so a C caller
        // switching on kinds had to write `15` by hand — and this test could not see it, because
        // it compares the constants the header *has* against the ones it is told to expect and
        // nobody told it to expect this one. The lesson is the map's rather than the header's: a
        // table of expectations is only as complete as the person who wrote it.
        ("QUORRA_EVENT_SEARCHED", EventKind::Searched),
        // The eight-hundred-and-eighty-fifth session's three, and the row above is why they are
        // written here in the same commit as the `#define`: `CLAUDE.md`'s *ask* and *warn*
        // levels, and §7.11.4's list moving (ADR 0814).
        ("QUORRA_EVENT_ASKING", EventKind::Asking),
        ("QUORRA_EVENT_WARNED", EventKind::Warned),
        (
            "QUORRA_EVENT_ATTACHMENTS_CHANGED",
            EventKind::AttachmentsChanged,
        ),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
}

/// The enumerations this ABI *takes*, which refuse a number they do not define.
#[expect(
    clippy::too_many_lines,
    reason = "one block per enumeration a caller passes in, and the count is the ABI's. Splitting \
              it would put half the header's constants in another function and lose what this \
              hand-written map is for: a second statement of every number, made independently"
)]
fn the_argument_enumerations(expected: &mut BTreeMap<String, i64>) {
    for (name, kind) in [
        ("QUORRA_PAGE_INDEX", PageTargetKind::Index),
        ("QUORRA_PAGE_FIRST", PageTargetKind::First),
        ("QUORRA_PAGE_LAST", PageTargetKind::Last),
        ("QUORRA_PAGE_NEXT", PageTargetKind::Next),
        ("QUORRA_PAGE_PREVIOUS", PageTargetKind::Previous),
        ("QUORRA_PAGE_RELATIVE", PageTargetKind::Relative),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_ZOOM_FIT_PAGE", viewer_ffi::ZoomKind::FitPage),
        ("QUORRA_ZOOM_FIT_WIDTH", viewer_ffi::ZoomKind::FitWidth),
        ("QUORRA_ZOOM_FIT_HEIGHT", viewer_ffi::ZoomKind::FitHeight),
        ("QUORRA_ZOOM_SCALE", viewer_ffi::ZoomKind::Scale),
        ("QUORRA_ZOOM_IN", viewer_ffi::ZoomKind::In),
        ("QUORRA_ZOOM_OUT", viewer_ffi::ZoomKind::Out),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    // The enumerations the five-hundred-and-eleventh session added, each read off its own
    // discriminant rather than written twice.
    for (name, kind) in [
        ("QUORRA_POINTER_MOVED", PointerKind::Moved),
        ("QUORRA_POINTER_PRESSED", PointerKind::Pressed),
        ("QUORRA_POINTER_DRAGGED", PointerKind::Dragged),
        ("QUORRA_POINTER_RELEASED", PointerKind::Released),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_SELECT_ALL", SelectKind::All),
        ("QUORRA_SELECT_NONE", SelectKind::None),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_FOCUS_NEXT", FocusKind::Next),
        ("QUORRA_FOCUS_PREVIOUS", FocusKind::Previous),
        ("QUORRA_FOCUS_NONE", FocusKind::None),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_RESTRICT_ON", RestrictKind::On),
        ("QUORRA_RESTRICT_OFF", RestrictKind::Off),
        // `CLAUDE.md`'s other two levels, since the eight-hundred-and-eighty-fifth session: each
        // arrived with the event and the entry point that answer it (ADR 0814).
        ("QUORRA_RESTRICT_ASK", RestrictKind::Ask),
        ("QUORRA_RESTRICT_WARN", RestrictKind::Warn),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    // §7.11.4.1's two homes, which is what `quorra_attach` chooses between.
    for (name, kind) in [
        ("QUORRA_ATTACH_DOCUMENT", AttachKind::Document),
        ("QUORRA_ATTACH_PAGE", AttachKind::Page),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_PRESENT_OFF", PresentKind::Off),
        ("QUORRA_PRESENT_ON", PresentKind::On),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    // Table 29's six arrangements, in the order that table states them.
    for (name, kind) in [
        (
            "QUORRA_LAYOUT_SINGLE_PAGE",
            viewer_ffi::LayoutKind::SinglePage,
        ),
        ("QUORRA_LAYOUT_ONE_COLUMN", viewer_ffi::LayoutKind::OneColumn),
        (
            "QUORRA_LAYOUT_TWO_COLUMN_LEFT",
            viewer_ffi::LayoutKind::TwoColumnLeft,
        ),
        (
            "QUORRA_LAYOUT_TWO_COLUMN_RIGHT",
            viewer_ffi::LayoutKind::TwoColumnRight,
        ),
        (
            "QUORRA_LAYOUT_TWO_PAGE_LEFT",
            viewer_ffi::LayoutKind::TwoPageLeft,
        ),
        (
            "QUORRA_LAYOUT_TWO_PAGE_RIGHT",
            viewer_ffi::LayoutKind::TwoPageRight,
        ),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_DELEGATE_DRAWN", DelegateKind::Drawn),
        ("QUORRA_DELEGATE_DELEGATED", DelegateKind::Delegated),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_MARKUP_HIGHLIGHT", MarkupKind::Highlight),
        ("QUORRA_MARKUP_UNDERLINE", MarkupKind::Underline),
        ("QUORRA_MARKUP_STRIKE_OUT", MarkupKind::StrikeOut),
        ("QUORRA_MARKUP_SQUIGGLY", MarkupKind::Squiggly),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    expected.insert(
        "QUORRA_PURPOSE_IMPORT_DATA".to_owned(),
        i64::from(PurposeKind::ImportData.code()),
    );
    // Table 164's two two-valued entries, which `quorra_event_transition` answers as numbers.
    for (name, value) in [
        ("QUORRA_DIMENSION_HORIZONTAL", 0),
        ("QUORRA_DIMENSION_VERTICAL", 1),
        ("QUORRA_MOTION_INWARD", 0),
        ("QUORRA_MOTION_OUTWARD", 1),
    ] {
        expected.insert(name.to_owned(), value);
    }
}

/// The enumerations this ABI *answers with*, each named and each with a count of its own.
fn the_answered_enumerations(expected: &mut BTreeMap<String, i64>) {
    expected.insert(
        "QUORRA_FORMAT_RGBA8".to_owned(),
        i64::from(PixelFormat::Rgba8.code()),
    );
    for (name, kind) in [
        ("QUORRA_CONTROL_ENTRY", ControlKind::Entry),
        ("QUORRA_CONTROL_CHECK", ControlKind::Check),
        ("QUORRA_CONTROL_RADIO", ControlKind::Radio),
        ("QUORRA_CONTROL_PUSH", ControlKind::Push),
        ("QUORRA_CONTROL_COMBO", ControlKind::Combo),
        ("QUORRA_CONTROL_LIST", ControlKind::List),
        ("QUORRA_CONTROL_SIGNATURE", ControlKind::Signature),
        ("QUORRA_CONTROL_UNSTATED", ControlKind::Unstated),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
    expected.insert(
        "QUORRA_CONTROL_KIND_COUNT".to_owned(),
        i64::from(ControlKind::COUNT),
    );
    for (name, kind) in [
        ("QUORRA_ROW_ACTIVATE", RowKind::Activate),
        ("QUORRA_ROW_TOGGLE", RowKind::Toggle),
        ("QUORRA_ROW_EXTRACT", RowKind::Extract),
        ("QUORRA_ROW_INERT", RowKind::Inert),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
    expected.insert("QUORRA_ROW_KIND_COUNT".to_owned(), i64::from(RowKind::COUNT));
    // §14.8.2.5's two content orders, and there is deliberately no `_COUNT` beside them: the
    // clause defines exactly two, so the guard the other answered enumerations carry against
    // growing under a compiled caller has nothing here to guard (ADR 0519).
    for (name, kind) in [
        ("QUORRA_ORDER_LOGICAL", OrderKind::Logical),
        ("QUORRA_ORDER_PAGE_CONTENT", OrderKind::PageContent),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
    for (name, kind) in [
        ("QUORRA_TEXT_QUALIFIED", TextKind::Qualified),
        ("QUORRA_TEXT_SHOWN", TextKind::Shown),
        ("QUORRA_TEXT_PARTIAL", TextKind::Partial),
        ("QUORRA_TEXT_LABEL", TextKind::Label),
        ("QUORRA_TEXT_EXPORT", TextKind::Export),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
}

/// `quorra_field_control`'s flag word.
fn the_field_flags(expected: &mut BTreeMap<String, i64>) {
    // Written out here rather than derived, because the numbers
    // *are* the ABI: a bit that moved would be a caller acting on the wrong flag, and a loop over
    // `1 << n` would agree with whatever the source said rather than with what was published.
    for (name, bit) in [
        ("QUORRA_FIELD_READ_ONLY", viewer_ffi::form::FLAG_READ_ONLY),
        ("QUORRA_FIELD_REQUIRED", viewer_ffi::form::FLAG_REQUIRED),
        ("QUORRA_FIELD_NO_EXPORT", viewer_ffi::form::FLAG_NO_EXPORT),
        ("QUORRA_FIELD_MULTILINE", viewer_ffi::form::FLAG_MULTILINE),
        ("QUORRA_FIELD_PASSWORD", viewer_ffi::form::FLAG_PASSWORD),
        ("QUORRA_FIELD_FILE_SELECT", viewer_ffi::form::FLAG_FILE_SELECT),
        (
            "QUORRA_FIELD_DO_NOT_SPELL_CHECK",
            viewer_ffi::form::FLAG_DO_NOT_SPELL_CHECK,
        ),
        (
            "QUORRA_FIELD_DO_NOT_SCROLL",
            viewer_ffi::form::FLAG_DO_NOT_SCROLL,
        ),
        ("QUORRA_FIELD_COMB", viewer_ffi::form::FLAG_COMB),
        ("QUORRA_FIELD_RICH_TEXT", viewer_ffi::form::FLAG_RICH_TEXT),
        (
            "QUORRA_FIELD_NO_TOGGLE_TO_OFF",
            viewer_ffi::form::FLAG_NO_TOGGLE_TO_OFF,
        ),
        (
            "QUORRA_FIELD_RADIOS_IN_UNISON",
            viewer_ffi::form::FLAG_RADIOS_IN_UNISON,
        ),
        ("QUORRA_FIELD_ON", viewer_ffi::form::FLAG_ON),
        ("QUORRA_FIELD_EDITABLE", viewer_ffi::form::FLAG_EDITABLE),
        (
            "QUORRA_FIELD_MULTI_SELECT",
            viewer_ffi::form::FLAG_MULTI_SELECT,
        ),
        (
            "QUORRA_FIELD_COMMIT_ON_SEL",
            viewer_ffi::form::FLAG_COMMIT_ON_SELECTION,
        ),
        ("QUORRA_FIELD_OBSCURED", viewer_ffi::form::FLAG_OBSCURED),
    ] {
        expected.insert(name.to_owned(), i64::from(bit));
    }
}

/// The constants the other half of the queries brought with it (ADR 0576).
///
/// Eleven `Query` variants reached no symbol at all until the seven-hundred-and-ninth session, and
/// what came with them is sixteen small enumerations. Every one is read off its own discriminant
/// here rather than written twice — except the two flag words, which are literals for
/// [`the_field_flags`]'s reason: a bit that moved would be a caller acting on the wrong flag, and
/// a loop over `1 << n` would agree with whatever the source said rather than with what was
/// published.
#[expect(
    clippy::too_many_lines,
    reason = "one block per enumeration, and the count is the ABI's. Splitting it would put half \
              of one round's constants in another function and lose what this hand-written map is \
              for: a second statement of every number, made independently"
)]
fn the_other_half_of_the_queries(expected: &mut BTreeMap<String, i64>) {
    for (name, kind) in [
        ("QUORRA_PAGE_MODE_USE_NONE", PageModeKind::UseNone),
        ("QUORRA_PAGE_MODE_USE_OUTLINES", PageModeKind::UseOutlines),
        ("QUORRA_PAGE_MODE_USE_THUMBS", PageModeKind::UseThumbs),
        ("QUORRA_PAGE_MODE_FULL_SCREEN", PageModeKind::FullScreen),
        ("QUORRA_PAGE_MODE_USE_OC", PageModeKind::UseOptionalContent),
        (
            "QUORRA_PAGE_MODE_USE_ATTACHMENTS",
            PageModeKind::UseAttachments,
        ),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
    expected.insert(
        "QUORRA_PAGE_MODE_COUNT".to_owned(),
        i64::from(PageModeKind::COUNT),
    );
    for (name, key) in [
        ("QUORRA_PREF_HIDE_TOOLBAR", PreferenceKey::HideToolbar),
        ("QUORRA_PREF_HIDE_MENUBAR", PreferenceKey::HideMenubar),
        ("QUORRA_PREF_HIDE_WINDOW_UI", PreferenceKey::HideWindowUi),
        ("QUORRA_PREF_FIT_WINDOW", PreferenceKey::FitWindow),
        ("QUORRA_PREF_CENTER_WINDOW", PreferenceKey::CenterWindow),
        (
            "QUORRA_PREF_DISPLAY_DOC_TITLE",
            PreferenceKey::DisplayDocTitle,
        ),
        (
            "QUORRA_PREF_NON_FULL_SCREEN_PAGE_MODE",
            PreferenceKey::NonFullScreenPageMode,
        ),
        ("QUORRA_PREF_DIRECTION", PreferenceKey::Direction),
        ("QUORRA_PREF_VIEW_AREA", PreferenceKey::ViewArea),
        ("QUORRA_PREF_VIEW_CLIP", PreferenceKey::ViewClip),
        ("QUORRA_PREF_PRINT_AREA", PreferenceKey::PrintArea),
        ("QUORRA_PREF_PRINT_CLIP", PreferenceKey::PrintClip),
        ("QUORRA_PREF_PRINT_SCALING", PreferenceKey::PrintScaling),
        ("QUORRA_PREF_DUPLEX", PreferenceKey::Duplex),
        (
            "QUORRA_PREF_PICK_TRAY_BY_PDF_SIZE",
            PreferenceKey::PickTrayByPdfSize,
        ),
        ("QUORRA_PREF_NUM_COPIES", PreferenceKey::NumCopies),
        (
            "QUORRA_PREF_ENFORCE_PRINT_SCALING",
            PreferenceKey::EnforcePrintScaling,
        ),
        ("QUORRA_PREF_PRINT_PAGE_RANGE", PreferenceKey::PrintPageRange),
    ] {
        expected.insert(name.to_owned(), i64::from(key.code()));
    }
    expected.insert(
        "QUORRA_PREF_KEY_COUNT".to_owned(),
        i64::from(PreferenceKey::COUNT),
    );
    for (name, kind) in [
        ("QUORRA_DIRECTION_L2R", DirectionKind::LeftToRight),
        ("QUORRA_DIRECTION_R2L", DirectionKind::RightToLeft),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_BOUNDARY_MEDIA", BoundaryKind::Media),
        ("QUORRA_BOUNDARY_CROP", BoundaryKind::Crop),
        ("QUORRA_BOUNDARY_BLEED", BoundaryKind::Bleed),
        ("QUORRA_BOUNDARY_TRIM", BoundaryKind::Trim),
        ("QUORRA_BOUNDARY_ART", BoundaryKind::Art),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        (
            "QUORRA_PRINT_SCALING_APP_DEFAULT",
            PrintScalingKind::AppDefault,
        ),
        ("QUORRA_PRINT_SCALING_NONE", PrintScalingKind::NoScaling),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_DUPLEX_SIMPLEX", DuplexKind::Simplex),
        ("QUORRA_DUPLEX_FLIP_SHORT_EDGE", DuplexKind::FlipShortEdge),
        ("QUORRA_DUPLEX_FLIP_LONG_EDGE", DuplexKind::FlipLongEdge),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_SHORTFALL_EMPTY_MAPPING", ShortfallKind::EmptyMapping),
        (
            "QUORRA_SHORTFALL_INCOMPLETE_TO_UNICODE",
            ShortfallKind::IncompleteToUnicode,
        ),
        ("QUORRA_SHORTFALL_UNLISTED_NAME", ShortfallKind::UnlistedName),
        ("QUORRA_SHORTFALL_UNNAMED_CID", ShortfallKind::UnnamedCid),
        (
            "QUORRA_SHORTFALL_UNADDRESSABLE_CID",
            ShortfallKind::UnaddressableCid,
        ),
        ("QUORRA_SHORTFALL_UNNAMED_GLYPH", ShortfallKind::UnnamedGlyph),
        ("QUORRA_SHORTFALL_UNNAMED_TOTAL", ShortfallKind::UnnamedTotal),
        (
            "QUORRA_SHORTFALL_WITHOUT_A_GLYPH",
            ShortfallKind::WithoutAGlyph,
        ),
        (
            "QUORRA_SHORTFALL_BLANK_GLYPH",
            ShortfallKind::ReachingABlankGlyph,
        ),
        (
            "QUORRA_SHORTFALL_UPRIGHT_VERTICAL_FORM",
            ShortfallKind::WithoutAVerticalForm,
        ),
    ] {
        expected.insert(name.to_owned(), i64::from(kind.code()));
    }
    expected.insert(
        "QUORRA_SHORTFALL_KIND_COUNT".to_owned(),
        i64::from(ShortfallKind::COUNT),
    );
    for (name, kind) in [
        ("QUORRA_NOTE_TITLE", NoteKind::Title),
        ("QUORRA_NOTE_CONTENTS", NoteKind::Contents),
        ("QUORRA_NOTE_MODIFIED", NoteKind::Modified),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_ELEMENT_ROLE", ElementKind::Role),
        ("QUORRA_ELEMENT_NAME", ElementKind::Name),
        ("QUORRA_ELEMENT_LANGUAGE", ElementKind::Language),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_BOX_STATED", BoxKind::Stated),
        ("QUORRA_BOX_DRAWN", BoxKind::Drawn),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_SCOPE_ROW", ScopeKind::Row),
        ("QUORRA_SCOPE_COLUMN", ScopeKind::Column),
        ("QUORRA_SCOPE_BOTH", ScopeKind::Both),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_COLLECTION_DETAILS", CollectionViewKind::Details),
        ("QUORRA_COLLECTION_TILE", CollectionViewKind::Tile),
        ("QUORRA_COLLECTION_HIDDEN", CollectionViewKind::Hidden),
        ("QUORRA_COLLECTION_NAVIGATOR", CollectionViewKind::Navigator),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_INITIAL_CONTAINER", InitialKind::Container),
        ("QUORRA_INITIAL_EMBEDDED", InitialKind::Embedded),
        ("QUORRA_INITIAL_FIRST_FILE", InitialKind::FirstFile),
        ("QUORRA_INITIAL_EMPTY", InitialKind::Empty),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_COLLECTION_FIELD_TEXT", ColumnKind::Text),
        ("QUORRA_COLLECTION_FIELD_DATE", ColumnKind::Date),
        ("QUORRA_COLLECTION_FIELD_NUMBER", ColumnKind::Number),
        ("QUORRA_COLLECTION_FIELD_FILE_NAME", ColumnKind::FileName),
        ("QUORRA_COLLECTION_FIELD_DESCRIPTION", ColumnKind::Description),
        (
            "QUORRA_COLLECTION_FIELD_MODIFICATION_DATE",
            ColumnKind::ModificationDate,
        ),
        (
            "QUORRA_COLLECTION_FIELD_CREATION_DATE",
            ColumnKind::CreationDate,
        ),
        ("QUORRA_COLLECTION_FIELD_SIZE", ColumnKind::Size),
        (
            "QUORRA_COLLECTION_FIELD_COMPRESSED_SIZE",
            ColumnKind::CompressedSize,
        ),
        ("QUORRA_COLLECTION_FIELD_OTHER", ColumnKind::Other),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_COLUMN_NAME", ColumnTextKind::Name),
        ("QUORRA_COLUMN_KEY", ColumnTextKind::Key),
        ("QUORRA_COLUMN_SUBTYPE", ColumnTextKind::Subtype),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    for (name, kind) in [
        ("QUORRA_FOLDER_NAME", FolderTextKind::Name),
        ("QUORRA_FOLDER_DESCRIPTION", FolderTextKind::Description),
    ] {
        expected.insert(name.to_owned(), kind as i64);
    }
    // The two flag words, written out for `the_field_flags`'s reason: the numbers *are* the ABI.
    for (name, bit) in [
        ("QUORRA_THUMBNAIL_COLOUR_SPACE_UNPERMITTED", 1),
        ("QUORRA_THUMBNAIL_SUBTYPE_UNPERMITTED", 2),
        ("QUORRA_COLUMN_VISIBLE", 1),
        ("QUORRA_COLUMN_EDITABLE", 2),
    ] {
        expected.insert(name.to_owned(), bit);
    }
}
