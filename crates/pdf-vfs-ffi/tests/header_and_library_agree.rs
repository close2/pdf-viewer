//! `include/quorra_vfs.h` against `src/abi.rs`, read back as text.
//!
//! **This is what buys back the one thing `cbindgen` would have given.** The header is
//! hand-written on purpose — it is the artefact a C++ plugin author reads, with the reason for
//! each shape beside it — and the price of that choice is that it can drift. So it is checked
//! instead of generated, exactly as `viewer-ffi`'s is:
//!
//! - every `#[unsafe(no_mangle)]` entry point is declared exactly once in the header, and every
//!   `quorra_vfs_` function the header declares exists in the Rust. A missing declaration is a symbol
//!   a caller cannot reach; an extra one is a link error in somebody else's build;
//! - every `QUORRA_VFS_` constant is the number the Rust gives it. **This is the one that would fail
//!   silently**: a `#define QUORRA_VFS_MEANS_DELETE_PAGE 3u` beside a Rust `MEANS_DELETE_PAGE = 2`
//!   produces a plugin that compiles, links, runs, and quietly tells a file manager that a page
//!   can be embedded.
//!
//! `tests/a_c_program_drives_the_abi.rs` is the other half and catches a different class: this
//! one reads text, that one hands the header to a compiler and the symbols to a linker.

#![expect(
    clippy::expect_used,
    reason = "test code: a source file that cannot be read must fail loudly rather than pass by \
              doing nothing"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use pdf_vfs_ffi::Status;
use pdf_vfs_ffi::refusal::KIND_COUNT;
use pdf_vfs_ffi::tree::{
    KIND_DIRECTORY, KIND_FILE, LEVEL_ASK, LEVEL_OFF, LEVEL_ON, LEVEL_WARN, MEANS_DELETE_PAGE,
    MEANS_EMBED_FILE, MEANS_INSERT_PAGES, MEANS_NOTHING, MEANS_REMOVE_ATTACHMENT,
    MEANS_SET_INFORMATION, VERB_DELETE, VERB_READ, VERB_WRITE, VERDICT_ASK, VERDICT_PROCEED,
    VERDICT_REFUSE, VERDICT_WARN,
};

/// The header, with every comment removed.
///
/// Comments name functions and constants in prose — "`quorra_vfs_worker_program()` and
/// `quorra_vfs_worker_variable()` are those two names" — and a check that counted those would be
/// checking the documentation rather than the declarations.
fn header_without_comments() -> String {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/quorra_vfs.h"))
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

/// Every `quorra_vfs_…` name the argument calls as a function.
fn called_names(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let characters: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    while at < characters.len() {
        if text[at..].starts_with("quorra_vfs_") {
            let mut end = at;
            while end < characters.len()
                && (characters[end].is_alphanumeric() || characters[end] == '_')
            {
                end = end.saturating_add(1);
            }
            // Only a name immediately followed by `(` is a declaration; a name followed by ` *`
            // is one of the five opaque types and a bare one is an argument.
            if characters.get(end) == Some(&'(') {
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
            name.starts_with("quorra_vfs_"),
            "an exported function not called quorra_vfs_…: {declaration}"
        );
        found.insert(name.to_owned());
    }
    found
}

/// Every `#define QUORRA_VFS_NAME value` in the header.
fn defined_constants(text: &str) -> BTreeMap<String, i64> {
    let mut found = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("#define QUORRA_VFS_") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let value = value.trim_end_matches('u');
        if let Ok(number) = value.parse::<i64>() {
            found.insert(format!("QUORRA_VFS_{name}"), number);
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
        40,
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

    expected.insert(
        "QUORRA_VFS_ABI_VERSION".to_owned(),
        i64::from(pdf_vfs_ffi::abi::QUORRA_VFS_ABI_VERSION),
    );
    expected.insert("QUORRA_VFS_ERRNO_KIND_COUNT".to_owned(), i64::from(KIND_COUNT));
    for (name, status) in [
        ("QUORRA_VFS_OK", Status::Ok),
        ("QUORRA_VFS_NULL_ARGUMENT", Status::NullArgument),
        ("QUORRA_VFS_OUT_OF_RANGE", Status::OutOfRange),
        ("QUORRA_VFS_BUFFER_TOO_SMALL", Status::BufferTooSmall),
        ("QUORRA_VFS_NOT_UTF8", Status::NotUtf8),
        ("QUORRA_VFS_REFUSED", Status::Refused),
        ("QUORRA_VFS_NO_ANSWER", Status::NoAnswer),
        ("QUORRA_VFS_NO_DOCUMENT", Status::NoDocument),
        ("QUORRA_VFS_NUMBER_OUT_OF_RANGE", Status::NumberOutOfRange),
    ] {
        expected.insert(name.to_owned(), i64::from(status.code()));
    }
    for (name, value) in [
        ("QUORRA_VFS_KIND_DIRECTORY", KIND_DIRECTORY),
        ("QUORRA_VFS_KIND_FILE", KIND_FILE),
        // `CLAUDE.md` principle 3's four levels, all four, and since ADR 0874 all four are
        // answerable by this face.
        ("QUORRA_VFS_RESTRICT_OFF", LEVEL_OFF),
        ("QUORRA_VFS_RESTRICT_ON", LEVEL_ON),
        ("QUORRA_VFS_RESTRICT_ASK", LEVEL_ASK),
        ("QUORRA_VFS_RESTRICT_WARN", LEVEL_WARN),
        // The verb a consultation is about, and the verdict it comes back with (ADR 0874).
        ("QUORRA_VFS_VERB_READ", VERB_READ),
        ("QUORRA_VFS_VERB_WRITE", VERB_WRITE),
        ("QUORRA_VFS_VERB_DELETE", VERB_DELETE),
        ("QUORRA_VFS_VERDICT_PROCEED", VERDICT_PROCEED),
        ("QUORRA_VFS_VERDICT_WARN", VERDICT_WARN),
        ("QUORRA_VFS_VERDICT_ASK", VERDICT_ASK),
        ("QUORRA_VFS_VERDICT_REFUSE", VERDICT_REFUSE),
        // RFC 0003 section 5.2's five verbs, and the zero a refused row is.
        ("QUORRA_VFS_MEANS_NOTHING", MEANS_NOTHING),
        ("QUORRA_VFS_MEANS_INSERT_PAGES", MEANS_INSERT_PAGES),
        ("QUORRA_VFS_MEANS_DELETE_PAGE", MEANS_DELETE_PAGE),
        ("QUORRA_VFS_MEANS_EMBED_FILE", MEANS_EMBED_FILE),
        ("QUORRA_VFS_MEANS_REMOVE_ATTACHMENT", MEANS_REMOVE_ATTACHMENT),
        ("QUORRA_VFS_MEANS_SET_INFORMATION", MEANS_SET_INFORMATION),
    ] {
        expected.insert(name.to_owned(), i64::from(value));
    }

    assert_eq!(
        defined, expected,
        "the header's numbers and the library's have come apart"
    );
}

/// The two names the header hands a caller for the confined generator are the core's own.
///
/// A second copy of `pdf-vfs-worker` spelled by hand here would be a plugin looking for a program
/// nobody builds, and the sentence it printed would name it correctly.
#[test]
fn the_worker_names_this_boundary_states_are_the_cores() {
    // SAFETY: both take nothing and answer a pointer into static memory. The `unsafe` is the
    // signature's rather than the body's; see `src/abi.rs`.
    let (program, variable) = unsafe {
        (
            std::ffi::CStr::from_ptr(pdf_vfs_ffi::abi::quorra_vfs_worker_program()),
            std::ffi::CStr::from_ptr(pdf_vfs_ffi::abi::quorra_vfs_worker_variable()),
        )
    };
    assert_eq!(program.to_str(), Ok(pdf_vfs::WORKER_PROGRAM));
    assert_eq!(variable.to_str(), Ok(pdf_vfs::WORKER_PATH_VARIABLE));
}
