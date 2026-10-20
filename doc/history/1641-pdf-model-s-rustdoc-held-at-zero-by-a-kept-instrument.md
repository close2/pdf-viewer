# 1641 — `pdf-model`'s rustdoc held at zero by a kept instrument

Slot 3 of batch one hundred, 2026-10-20, a docs round under the owner's switch to code improvement.
No ADR (2118 and 2119 not used): every answer is one ADR 1938 gives, and its status line now names
`pdf-model`, the instrument and `--keep-going`. No Q536. No ledger row moves; no clause is read.

**Premise.** Held: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items -p
pdf-model` failed at HEAD with 785 errors at 785 locations. **The hypothesis held in kind, not in
share**: 603 bracketed letters (77 %, not nine in ten), 130 private links, 52 other.

**Done.** `tools/rustdoc-brackets.py` reads rustdoc's log and makes ADR 1938's two mechanical answers
at the bracket the column points into, walking back over at most one backtick and asserting the `[`
and the comment line (habit 135); a failed anchor stops the run with nothing written; a reference
definition (CommonMark's: never a sentence's second line) or an explicit target goes to a hand. Its
`--self-test` plants each class and each failure and `tools/conformance/tests/rustdoc_brackets.rs`
runs it; six defects planted in the script each failed it. One pass: 603 letters escaped, 127 private
links made code spans, 730 edits in 106 files. By hand, anchored: 15 definitions of private or
unreachable items removed, their uses code spans; 11 paths re-pointed (`crate::view::Appearance`,
`crate::page::ContentIssue`, `text::TextObject`, `crate::structure::{Priority, TableStack,
list_predecessors}`, `pdf_render::ImageAtDeviceScale::samples` by a definition at the block's end;
`Interpreter::lattice` inline); 12 private or test-only names code spans; three stale names the item
they became (`kept` → `Parts::keep`, `accessibility::nodes` → `speech`, `path::extend_subpath` →
`Interpreter::extend_subpath`); `format()`, `markup()`, `text_string()` for names that are two items;
one redundant target dropped. `tools/state.sh docs` holds seven crates and builds with `--keep-going`.

**The one non-comment hunk.** §14.8.6.1's bare URL is a code span inside its quotation now, which
answers `bare_urls`; that left `DEFAULT_STANDARD_NAMESPACE`'s `#[expect(clippy::doc_markdown)]`
unfulfilled, and its five lines are removed — a lint expectation, no code.

**Files.** 110 in `crates/pdf-model/src/` (doc comments), `tools/rustdoc-brackets.py`,
`tools/conformance/tests/rustdoc_brackets.rs`, `tools/state.sh` (`section_docs`), ADR 1938's status.

**Gates.** Multiset of non-comment lines over the 110 files against HEAD: 109 unchanged, the one
difference the five attribute lines (shown a planted `Vec<Spokn>` first); every other changed line is
`///` or `//!`. `tools/state.sh --round 1641 docs` exit 0 over seven crates; a planted `[t]he` in
`pdf-font` and `pdf-model`: exit 101 naming both; at `-j 1` without `--keep-going` only `pdf-model` is
named. Public-only `cargo doc -p pdf-model` 0. `cargo fmt --check -p pdf-model -p conformance` 0. `-D
warnings` clippy `--keep-going -p pdf-model -p conformance --all-targets`: clean but slot 2's scratch
`examples/r1640_isolate.rs` (7 errors). `cargo test -p conformance --no-fail-fast`: 448 passed, 1
failed (`round_numbers`, on that scratch file); `--bin quotations`, `--bin pointers` 0. About 1 850 s.
