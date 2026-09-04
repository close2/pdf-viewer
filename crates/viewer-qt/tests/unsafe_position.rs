//! Where this crate's `unsafe` is, checked by reading the crate's own sources back.
//!
//! Every crate in this tree that touches PDF bytes holds `#![forbid(unsafe_code)]`, and
//! `viewer-gtk` holds it too. **This crate cannot**, because `#[cxx::bridge]` expands to
//! `unsafe extern "C"` declarations, `unsafe` blocks and `#[export_name]` functions, and a
//! `forbid` cannot be lifted by an inner `allow` — which is the whole point of `forbid`.
//!
//! `doc/todo/30` reserves the permission for `viewer-ffi` alone, so a crate that takes it early
//! owes a check rather than a promise. That is what this file is: **a `deny` with one exemption
//! is a rule with a hole in it, and a hole nobody measures is a hole that grows.**
//!
//! The position these tests fix is exact, and it is not "no `unsafe` here":
//!
//! - the crate root denies `unsafe_code` and lifts it once, on `mod bridge`;
//! - **the whole crate contains one hand-written `unsafe` token**, and it is the
//!   `unsafe extern "C++"` block header in `src/bridge.rs`. That token is `cxx`'s requirement
//!   that the author assert the declared C++ signatures are the ones C++ actually has and are
//!   safe to call with those types — which is a real obligation, discharged by `cpp/host.h`
//!   declaring exactly one function and `cpp/window.cpp` defining it;
//! - every other `unsafe` in the compiled crate is the macro's expansion, which no source line
//!   contains.
//!
//! Reading source text is a blunt instrument and it is the right one here: the property being
//! asserted is about what a person wrote, and no compiler-visible fact distinguishes a macro's
//! `unsafe` from an author's.
//!
//! Only `src/` and `build.rs` are read — what is compiled into the library and the binary. An
//! integration test is its own crate and carries none of the library's attributes, which is why
//! this file states its own.

#![deny(unsafe_code)]

use std::path::{Path, PathBuf};

/// What a Rust item declaration starts with.
///
/// Attributes in this crate span several lines, so the item an exemption attaches to is the first
/// *declaration* after it — which is what an attribute binds to in Rust.
const DECLARATIONS: [&str; 8] = [
    "mod ", "pub ", "fn ", "use ", "struct ", "enum ", "impl ", "const ",
];

/// Every `.rs` file that becomes part of this crate's library or its binary.
fn sources() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = vec![root.join("build.rs")];
    let mut stack = vec![root.join("src")];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            for entry in entries.flatten() {
                stack.push(entry.path());
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// A line with its comments and the lint's own name removed, so that only code is left.
///
/// The lint is called `unsafe_code` and the attributes that mention it are the *statement of the
/// rule* rather than a use of the permission, so they are taken out before the token is looked
/// for. Documentation goes for the same reason: this crate's prose says the word repeatedly.
fn code_only(line: &str) -> String {
    let without_comment = match line.find("//") {
        Some(at) => &line[..at],
        None => line,
    };
    without_comment.replace("unsafe_code", "")
}

/// Every source line of this crate that uses the permission, with where it is.
fn lines_using_the_permission() -> Vec<(PathBuf, usize, String)> {
    let mut found = Vec::new();
    for path in sources() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            if code_only(line).contains("unsafe") {
                found.push((
                    path.clone(),
                    number.saturating_add(1),
                    line.trim().to_owned(),
                ));
            }
        }
    }
    found
}

#[test]
fn the_crate_contains_exactly_one_hand_written_unsafe_and_it_is_the_bridges_own() {
    let found = lines_using_the_permission();
    let described: Vec<String> = found
        .iter()
        .map(|(path, number, line)| {
            let named = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            // The file and the token, not the line number: what is being fixed is *where* and
            // *what*, and a test that also pinned the line would fail on every paragraph added
            // above it, which teaches a later session to loosen the assertion rather than read it.
            let _ = number;
            format!("{named}: {line}")
        })
        .collect();
    assert_eq!(
        described,
        // The `cxx` block header, which is the author asserting that `cpp/host.h`'s declaration
        // matches what C++ defines. Anything else appearing here is a line to argue about rather
        // than a line to allow.
        vec![r#"bridge.rs: unsafe extern "C++" {"#.to_owned()],
        "this crate's `unsafe` is one token in one place"
    );
}

#[test]
fn the_exemption_is_one_line_and_it_is_the_bridge() {
    let lib = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("this crate has a lib.rs");
    let code: String = lib.lines().map(code_only).collect::<Vec<_>>().join("\n");
    assert_eq!(
        code.matches("#![deny()]").count(),
        1,
        "the crate root denies it"
    );
    assert_eq!(
        code.matches("#[allow()]").count(),
        1,
        "and lifts the denial exactly once"
    );
    // And the one lift is attached to the bridge rather than to the crate, to a function, or to
    // anything a later session might add beside it. Read over the *code* lines, because the
    // documentation above quotes the attribute and a text search would find the quotation first.
    let code_lines: Vec<String> = lib.lines().map(code_only).collect();
    let Some(at) = code_lines
        .iter()
        .position(|line| line.contains("#[allow()]"))
    else {
        unreachable!("the count above found one");
    };
    let next_item = code_lines
        .iter()
        .skip(at.saturating_add(1))
        .map(|line| line.trim().to_owned())
        .find(|line| DECLARATIONS.iter().any(|start| line.starts_with(start)))
        .unwrap_or_default();
    assert_eq!(next_item, "mod bridge;", "the exemption names the bridge");
}

/// And no crate the workspace does not name gained the permission alongside it.
///
/// `doc/todo/30`'s rule is about the tree rather than about one crate, so the check that matters
/// is the one that would notice a third exemption appearing somewhere else. This test said
/// `viewer-ffi` "will be the second when it arrives, and this test is where that has to be
/// written down"; it arrived in the four-hundred-and-eleventh session (ADR 0247) and
/// `pdf-vfs-ffi` in the nine-hundred-and-thirteenth (ADR 0868), so the list has **three** names
/// on it, each with a test of its own on where its `unsafe` sits — this file,
/// `crates/viewer-ffi/tests/unsafe_position.rs`, which additionally asserts that every crate
/// touching PDF bytes still *forbids* the permission, and
/// `crates/pdf-vfs-ffi/tests/unsafe_position.rs`.
///
/// **A third name arrived in the nine-hundred-and-thirteenth session, and this is where it was
/// argued for.** `pdf-vfs-ffi` is RFC 0003's C ABI over `pdf-vfs` — the boundary a KIO worker
/// forwards over, because RFC 0003 section 7 records that KF6 admits no Rust worker and
/// `KIO::WorkerBase` is a C++ class. It is the *same* reason the first name is on the list: a C
/// caller cannot be handed a `Result` or an owned `Vec`, so something has to hold raw pointers,
/// and `doc/todo/30`'s rule is that the something is one module with a test on where its `unsafe`
/// sits. `crates/pdf-vfs-ffi/tests/unsafe_position.rs` is that test, and ADR 0868 is the record.
///
/// The rule this list enforces is unchanged and is worth restating rather than assuming: **no
/// crate that touches PDF bytes lifts the denial**, and none of these three does — `pdf-vfs-ffi`
/// parses nothing at all, because RFC 0003 section 6 puts every byte of parsing in a confined
/// process. A fourth name appearing here is a change to a rule the project owner stated, and it
/// belongs in an ADR rather than in a commit that widens this list.
#[test]
fn only_the_three_named_crates_in_the_tree_lift_the_denial() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let Ok(entries) = std::fs::read_dir(&crates) else {
        println!("skipped: the crates directory is not readable from here");
        return;
    };
    let mut lifting = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let mut stack = vec![entry.path().join("src")];
        while let Some(path) = stack.pop() {
            if path.is_dir() {
                let Ok(inner) = std::fs::read_dir(&path) else {
                    continue;
                };
                for one in inner.flatten() {
                    stack.push(one.path());
                }
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                if text.contains("#[allow(unsafe_code)]") || text.contains("#![allow(unsafe_code)]")
                {
                    lifting.push(name.clone());
                    break;
                }
            }
        }
    }
    lifting.sort();
    lifting.dedup();
    assert_eq!(
        lifting,
        vec![
            "pdf-vfs-ffi".to_owned(),
            "viewer-ffi".to_owned(),
            "viewer-qt".to_owned(),
        ],
        "only the two C ABIs and the crate with the C++ bridge lift it"
    );
}
