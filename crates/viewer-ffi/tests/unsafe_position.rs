//! Where this crate's `unsafe` is, checked by reading the crate's own sources back.
//!
//! `doc/todo/30` reserves the permission for this crate: *"the **only** crate in the tree
//! permitted `unsafe`. Every crate touching PDF bytes keeps `#![forbid(unsafe_code)]`; the FFI
//! crate touches messages, not documents, so the compiler-enforced rule survives."* Being the
//! crate the rule names is not a licence to stop counting — it is the reason the count has to
//! exist, because a permission nobody measures is a permission that spreads.
//!
//! `viewer-qt` took one token of it a round early, with a test on its position (ADR 0246). This
//! file is that test's twin, and the position it fixes is:
//!
//! - the crate root **denies** `unsafe_code` and lifts it **once**, on `mod abi`;
//! - every `unsafe` token in the crate is in `src/abi.rs`, and every one of them is of one of
//!   three forms: the `#![allow(unsafe_op_in_unsafe_fn)]` that module argues for, an
//!   `#[unsafe(no_mangle)]` attribute, or the `unsafe` in a `pub unsafe extern "C" fn` or an
//!   `unsafe fn` helper's signature;
//! - **no `unsafe` block appears anywhere**, which is what the module-level lift buys and is the
//!   thing that would otherwise be uncounted;
//! - the number of entry points is stated here, so that one added without a header declaration is
//!   a failure rather than a symbol nobody knows about.
//!
//! Reading source text is a blunt instrument and is the right one: the property is about what a
//! person wrote, and no compiler-visible fact distinguishes an author's `unsafe` from a macro's.

#![deny(unsafe_code)]
#![expect(
    clippy::expect_used,
    reason = "test code: a source file that cannot be read must fail loudly rather than pass by \
              doing nothing"
)]

use std::path::{Path, PathBuf};

/// Every `.rs` file compiled into this crate's library.
fn sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
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
/// The lints are called `unsafe_code` and `unsafe_op_in_unsafe_fn`, and an attribute mentioning
/// either is the *statement of the rule* rather than a use of the permission — except for the one
/// lift, which is counted separately below. Documentation goes for the same reason: this crate's
/// prose says the word on nearly every page.
fn code_only(line: &str) -> String {
    let without_comment = match line.find("//") {
        Some(at) => &line[..at],
        None => line,
    };
    without_comment.replace("unsafe_code", "")
}

/// Every source line of this crate that uses the permission, with the file it is in.
fn lines_using_the_permission() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for path in sources() {
        let text = std::fs::read_to_string(&path).expect("a source file of this crate");
        let named = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        for line in text.lines() {
            if code_only(line).contains("unsafe") {
                found.push((named.clone(), line.trim().to_owned()));
            }
        }
    }
    found
}

#[test]
fn every_unsafe_token_in_this_crate_is_in_one_file_and_is_one_of_three_forms() {
    let found = lines_using_the_permission();
    let mut elsewhere = Vec::new();
    let mut lift = 0usize;
    let mut no_mangle = 0usize;
    let mut signatures = 0usize;
    let mut blocks = Vec::new();
    for (file, line) in &found {
        if file != "abi.rs" {
            elsewhere.push(format!("{file}: {line}"));
            continue;
        }
        if line.starts_with("#![allow(unsafe_op_in_unsafe_fn)]") {
            lift = lift.saturating_add(1);
        } else if line.starts_with("#[unsafe(no_mangle)]") {
            no_mangle = no_mangle.saturating_add(1);
        } else if line.starts_with("pub unsafe extern \"C\" fn") || line.starts_with("unsafe fn ") {
            signatures = signatures.saturating_add(1);
        } else {
            blocks.push(format!("{file}: {line}"));
        }
    }

    assert_eq!(
        elsewhere,
        Vec::<String>::new(),
        "the permission is used outside `src/abi.rs`"
    );
    assert!(
        blocks.is_empty(),
        "an `unsafe` of a form this crate does not use: {blocks:?}"
    );
    assert_eq!(lift, 1, "the module lifts `unsafe_op_in_unsafe_fn` once");
    // One `#[unsafe(no_mangle)]` per exported entry point, and one `unsafe fn` per entry point
    // that takes a pointer plus the three helpers they share. The numbers are here so that a
    // function added without a line in `include/quorra.h` fails a test rather than becoming a
    // symbol nobody has declared — `header_and_library_agree.rs` is the other half of that. An
    // entry point added moves these two numbers and nothing else: a `Command` and a `Query` are
    // symbols, and only an `Event` kind moves `QUORRA_EVENT_KIND_COUNT`. `QUORRA_ABI_VERSION`
    // stays where it is, because a caller compiled before an entry point existed calls nothing of
    // that shape — which holds for a struct crossing by value through a new entry point too
    // (ADR 0737). An entry point that takes no pointer is not `unsafe`: the `_count`/`_name`
    // pairs that let a caller print a number this build does not define are the example
    // (ADR 0576). Which entry points serve which clause is each one's ADR — 0357, 0519, 0576,
    // 0737, 0814, 1090, 1101, 1106, 1144, 1145, 1167, 1168, 1180, 1191, 1203, 1204, 1228, 1251,
    // 1252, 1263, 1335, 1553, 1602, 1604, 1655, 1667, 1702, 1726 and 1753.
    assert_eq!(no_mangle, 232, "one `#[unsafe(no_mangle)]` per entry point");
    assert_eq!(
        signatures, 219,
        "216 `unsafe` entry points and three helpers"
    );
}

#[test]
fn the_crate_root_denies_the_permission_and_lifts_it_once_on_the_abi() {
    let root = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("this crate has a lib.rs");
    let code: Vec<String> = root.lines().map(code_only).collect();
    let joined = code.join("\n");
    assert_eq!(
        joined.matches("#![deny()]").count(),
        1,
        "the root denies it"
    );
    assert_eq!(
        joined.matches("#[expect()]").count(),
        1,
        "and lifts the denial exactly once"
    );
    let at = code
        .iter()
        .position(|line| line.contains("#[expect()]"))
        .expect("the count above found one");
    let next = code
        .iter()
        .skip(at.saturating_add(1))
        .map(|line| line.trim().to_owned())
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    assert_eq!(next, "pub mod abi;", "the exemption names the ABI module");
}

/// And the crates that touch PDF bytes still forbid it, which is the rule this one is an
/// exception to.
///
/// Named rather than swept, because the list is the claim: `doc/todo/30` says the compiler-enforced
/// rule survives this crate's arrival, and the way to know is to check the crates it is about.
#[test]
fn the_crates_that_touch_pdf_bytes_still_forbid_the_permission() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for name in [
        "pdf-syntax",
        "pdf-model",
        "pdf-font",
        "pdf-render",
        "render-cpu",
        "viewer-core",
        "viewer-host",
    ] {
        let root = crates.join(name).join("src/lib.rs");
        let text = std::fs::read_to_string(&root)
            .unwrap_or_else(|_| panic!("{name} has a lib.rs at {}", root.display()));
        assert!(
            text.contains("#![forbid(unsafe_code)]"),
            "{name} no longer forbids `unsafe`"
        );
    }
}
