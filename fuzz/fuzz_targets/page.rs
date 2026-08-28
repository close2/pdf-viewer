//! Fuzzes interpreting a page — clauses 8, 9 and 11 over a whole document the fuzzer wrote.
//!
//! **This target exists because of a gap the four-hundred-and-twenty-fifth session found from the
//! other side.** A 696-byte document whose type 3 function's `/Functions` array named its own
//! object overflowed the stack of every program in this tree, and it was found by downloading a
//! corpus rather than by fuzzing it. Thirteen targets and not one could have reached it: three of
//! them hand the fuzzer's bytes to `Document::open` (`document`, `crypt`, `forms_data`) and stop
//! at the object graph, and the only one that calls [`pdf_model::interpret`] is `variable_text`,
//! whose page is fixed, empty, and carries no `/Resources` at all — and `nm` says it more sharply
//! still: eleven of the thirteen binaries do not contain `parse_stitching` and twelve do not
//! contain `pdf_model::interpret`. So no fuzzer here had ever constructed a shading, a pattern,
//! a form `XObject`, an image or a `/Function` of any type — and
//! `pdf-model`'s content interpreter, which is the largest body of untrusted-input code in the
//! tree, was reachable by no target at all. ADR 0264.
//!
//! **The input is a whole document and nothing wraps it**, which is the point. The other
//! document-shaped targets fix a skeleton and vary one field inside it, so the object graph they
//! can express is the skeleton's; here the fuzzer owns the trailer, the cross-reference section,
//! every object and every reference between them, which is what it takes to write a cycle. It is
//! also what makes real files usable as seeds directly, and `fuzz/seed_page.py` puts **1882** of
//! them in — a target over documents whose corpus is not documents tests the recovery scanner and
//! little else. Measured on the neighbouring target: seeding `document` from the same files took
//! it from 3010 covered edges to 4351.
//!
//! Three properties are under test, and "the page is drawn correctly" is not among them: a fuzzer
//! cannot know what a page it invented should look like.
//!
//! - **No panic, no abort and no unbounded recursion.** §7.3.10 makes a reference something a
//!   reader follows and nothing in ISO 32000-2 forbids an object from naming itself, so every
//!   descent the interpreter makes — a form `XObject` inside a form `XObject`, a pattern whose
//!   content paints a pattern, a stitching function whose subfunction is itself — is a cycle a
//!   valid file
//!   may state. Each has a bound; this is what checks that the bound is reached before the stack
//!   is.
//! - **Interpretation terminates.** Every loop whose trip count comes out of the file is bounded
//!   by a budget, and libFuzzer's own timeout is what notices a budget that stopped bounding.
//! - **Interpretation is a function of the bytes.** `CLAUDE.md` rests the whole cross-backend
//!   comparison on this — "`interpret` remains a pure function of what the file says" — and no
//!   test in the tree checks it under an input nobody wrote on purpose. Interpreting twice and
//!   comparing the geometry digest is what would notice a cache keyed on something the document
//!   does not determine.
//!
//! What this target deliberately does **not** do is rasterise. The display list is where the
//! budgets live and where every document-controlled decision is made; putting pixels behind it
//! would cost an order of magnitude of exec rate to fuzz a backend whose input is no longer the
//! document.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_model::{Pages, interpret};
use pdf_syntax::Document;

/// Inputs past this are refused before opening.
///
/// Not a statement about what this tree can open — the corpus gate reads a 96 MB document — but
/// about what a *fuzzer* learns per second. Interpreting a page is milliseconds where the other
/// thirteen targets are microseconds, so the whole value of this one is in how many object graphs
/// it gets to try; `seed_page.py` picks its seeds under the same ceiling for the same reason.
const MAX_INPUT: usize = 256 * 1024;

/// Inputs up to this are interpreted twice, for the third property below.
const PURITY_LIMIT: usize = 16 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT {
        return;
    }

    let Ok(document) = Document::open(data.to_vec()) else {
        // A refusal is a result. Every one is a typed `SyntaxError` rather than a panic, which
        // is the property this arm exists to exercise rather than to skip.
        return;
    };

    let pages = Pages::new(&document);
    let Some(page) = pages.get(0) else {
        return;
    };

    let first = interpret(&document, &page);

    // The purity check is the second interpretation, and it is paid for only by the short inputs.
    // Measured: one 236 KB document from the corpus takes 0.8 s in a release binary and 15 s in
    // this one, where the sanitiser, the debug assertions and two passes multiply — so charging
    // every large seed twice would halve an exec rate that is already 10/s. Short inputs are
    // where a fuzzer spends its runs and where the crasher this target was written for lived
    // (696 bytes), so the property is checked exactly where it is checked cheaply.
    if data.len() > PURITY_LIMIT {
        return;
    }

    // Interpreting the same bytes again must give the same geometry. The digest rather than the
    // whole list because it is what `interpret_with` itself compares the two halves of a
    // `DeviceCMYK` page by, so a shape this cannot tell apart is one that clause already treats
    // as identical.
    let again = interpret(&document, &page);
    assert_eq!(
        first.display_list.geometry_digest(),
        again.display_list.geometry_digest(),
        "interpreting the same page twice drew two different geometries"
    );
    assert_eq!(
        first.text, again.text,
        "interpreting the same page twice read two different texts"
    );
    assert_eq!(
        first.glyphs, again.glyphs,
        "interpreting the same page twice marked a different number of glyphs"
    );
});
