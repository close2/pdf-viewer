//! Where the `unsafe` is, asserted rather than promised.
//!
//! `viewer-qt` established this shape and `viewer-ffi` follows it; this is the third crate in the
//! tree to take the permission and the second to be a C boundary, so it takes the same test
//! rather than inventing a second way of saying the same thing.
//!
//! What it holds:
//!
//! - the crate denies `unsafe_code` and lifts the denial for exactly one module, [`abi`];
//! - inside that module every `unsafe` token is in an entry point's **signature** or in one of
//!   the three shared helpers' — never a block in a body, because these bodies are entirely the
//!   unsafe operation and a block around all of one would mark nothing out;
//! - no other file in the crate holds the word at all.
//!
//! **A count is part of it**, and the count is the thing that makes this a gate rather than a
//! description: a round that adds an entry point without a header declaration fails the test
//! beside this one, and a round that adds an `unsafe` block in a body fails this one.

#![expect(
    clippy::expect_used,
    reason = "test code: a source file that cannot be read must fail loudly rather than pass by \
              doing nothing"
)]

use std::path::Path;

/// Every `.rs` file of this crate's `src/`, with its path.
fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let entries = std::fs::read_dir(&root).expect("this crate has a src directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|kind| kind == "rs") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            let text = std::fs::read_to_string(&path).expect("a source file this crate has");
            found.push((name, text));
        }
    }
    found.sort();
    assert!(found.len() >= 4, "this crate has four modules and a lib");
    found
}

#[test]
fn the_only_unsafe_in_this_crate_is_the_abi_modules() {
    for (name, text) in sources() {
        if name == "abi.rs" || name == "lib.rs" {
            continue;
        }
        // Documentation about the word is not a use of it — `tree.rs` opens by saying that
        // nothing in it is unsafe, which is the sentence this test exists to keep true.
        let code = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"));
        for line in code {
            assert!(
                !line.contains("unsafe"),
                "{name} holds the word `unsafe`, and the permission is the abi module's alone: \
                 {line}"
            );
        }
    }
}

#[test]
fn every_unsafe_token_in_the_abi_module_is_in_a_signature() {
    let abi = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/abi.rs"))
        .expect("this crate has an abi module");
    let mut entry_points = 0usize;
    let mut helpers = 0usize;
    for line in abi.lines() {
        let line = line.trim();
        // Documentation and the module's own lint lift are prose about `unsafe`, not uses of it.
        if line.starts_with("//") || line.starts_with("#![") {
            continue;
        }
        if line == "#[unsafe(no_mangle)]" {
            entry_points = entry_points.saturating_add(1);
            continue;
        }
        if !line.contains("unsafe") {
            continue;
        }
        if line.starts_with("pub unsafe extern \"C\" fn ") {
            continue;
        }
        if line.starts_with("unsafe fn ") {
            helpers = helpers.saturating_add(1);
            continue;
        }
        panic!("an `unsafe` that is not a signature: {line}");
    }
    assert_eq!(
        entry_points, 35,
        "the count `header_and_library_agree.rs` also states"
    );
    assert_eq!(helpers, 3, "owned_text, copy_out and refused");
}

#[test]
fn the_crate_denies_unsafe_and_lifts_it_for_one_module() {
    let lib = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("this crate has a lib");
    assert!(lib.contains("#![deny(unsafe_code)]"));
    assert_eq!(
        lib.matches("#[allow(unsafe_code)]").count(),
        1,
        "the permission is granted once, to one module"
    );
}
