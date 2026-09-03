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
    // 114 exported entry points, of which 104 take a pointer and are therefore `unsafe fn`, plus
    // the two `unsafe fn` helpers they share. The numbers are here so that a function added
    // without a line in `include/pdf_viewer.h` fails a test rather than becoming a symbol nobody
    // has declared — `header_and_library_agree.rs` is the other half of that. Four arrived in the
    // four-hundred-and-fourteenth session, for Annex O's `search`: three verbs and one accessor.
    // **Sixty-eight arrived in the five-hundred-and-eleventh**, which is `doc/todo/30`'s whole
    // remaining list — the pointer and the selection, §12.7's form and the four edits, save and
    // extract with a byte accessor apiece, the layer and attachment panels, §12.4.4's clock and
    // transitions, and the three policy values. Not one of them moved `PDFV_EVENT_KIND_COUNT`,
    // because a `Command` and a `Query` are symbols and only an `Event` is a number. **The
    // hundred-and-twelfth arrived in the five-hundred-and-twenty-second**, for Annex O's
    // `highlight`: a question the annex asks and no host can answer for itself, and a symbol
    // again rather than a number (ADR 0357). **Two more came with Table 29's `/PageLayout`**:
    // `pdfv_layout`, which is the arrangement a reader chose, and `pdfv_frame_count`, which is
    // the one thing a C caller could not have deduced — a C consumer cannot fail to compile, so
    // an arrangement putting a second page on the screen has to be something it can *ask* about.
    // **And two in the six-hundred-and-tenth**, which is that same sentence about a *report*:
    // `pdfv_reported_pages` counts the pages the arrangement has anything to say about and
    // `pdfv_reported_page` says which page an entry is, because a status bar under a column that
    // named no page would be attributing one page's refusals to whichever page a reader is
    // looking at. Both existing report entry points also gained the entry index, so a caller
    // written against the old shape fails to compile rather than reading page one's sentences
    // for four pages. **And one in the six-hundred-and-eighty-third**, which is the first
    // `Query` a C caller could not reach at all rather than a shape that moved:
    // `pdfv_selection_copy_text` answers with §14.8.2.5's logical content order where the
    // document's structure tree gives one, which is what a caller needs in order to put a
    // selection on a clipboard this ABI knows nothing about (ADR 0519).
    // **And fifty-two in the seven-hundred-and-ninth**, which is the rest of that same sentence:
    // eleven `Query` variants reached no symbol at all and now every one does (ADR 0576). The
    // shapes are handles, keyed accessors and out-parameters and **not one struct passed by
    // value**, which is why `PDFV_ABI_VERSION` did not move for the largest addition this ABI has
    // had. Six of the fifty-two take no pointer and are therefore not `unsafe` — the three
    // `_count`/`_name` pairs that let a caller print a number this build does not define.
    // **And three in the seven-hundred-and-twenty-sixth**, which `doc/todo/30` had priced at "two
    // accessors": §14.7's per-character offsets and boxes — `AccessibilityNode::lines`, which is
    // what AT-SPI's `Text` interface is built on and the one part of the accessibility answer a C
    // caller could not reach. Three rather than two because this ABI asks a *count* before an
    // indexed accessor, and a line has two counts: how many lines an element drew, and how many
    // character codes a line holds. No struct crossed by value, so `PDFV_ABI_VERSION` did not move.
    // **And two in the eight-hundred-and-fifth**, which are one question in both directions:
    // `pdfv_view` says where the reader is looking and `pdfv_set_view` puts them back there,
    // because the commands that make a view are relative and clamped and no caller can invert
    // them (ADR 0737). A struct *did* cross by value this time — `pdfv_viewing` — and
    // `PDFV_ABI_VERSION` still did not move, for the reason a new entry point does not move it:
    // a caller compiled before it existed passes nothing of that shape.
    // **And three in the eight-hundred-and-eighty-fifth**, which are one feature and its policy:
    // `pdfv_attach` and `pdfv_detach` put §7.11.4's file into one of §7.11.4.1's two homes or take
    // one out, and `pdfv_answer` is the second half of `PDFV_RESTRICT_ASK` — a level is not a
    // level until something can answer its question (ADR 0814). Three event kinds came with them,
    // so `PDFV_EVENT_KIND_COUNT` moved 16 → 19; `PDFV_ABI_VERSION` did not, for the standing
    // reason that a caller compiled before an entry point existed calls nothing of that shape.
    assert_eq!(no_mangle, 177, "one `#[unsafe(no_mangle)]` per entry point");
    assert_eq!(signatures, 163, "161 `unsafe` entry points and two helpers");
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
        joined.matches("#[allow()]").count(),
        1,
        "and lifts the denial exactly once"
    );
    let at = code
        .iter()
        .position(|line| line.contains("#[allow()]"))
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
