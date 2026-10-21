# 1938 — The documentation build is held at zero, and each class of warning has one answer

Session 1551, on the project owner's word of 2026-10-14 ("switch to code improvements/refactoring
rounds"). Status: **accepted** and **built** for `render-cpu`, `raster-gpu` and `viewer-ui`, and for
`viewer-device`, `pdf-font` and `viewer-confined` (session 1629, the same answers), and for
`pdf-model` (session 1641: the two mechanical answers made by `tools/rustdoc-brackets.py` from
rustdoc's own log, which `cargo test -p conformance` runs on planted cases, and the gate's build
given `--keep-going`, without which the first failing crate hides every later crate's errors), and
for `viewer-core` and `render-raster` (session 1669: the instrument's pass and eight by hand; a
name in a crate that depends on the one documented is a code span, as a private item's is).
Context: ADRs 1023, 1273, 1912, 1914. No clause is read anew and no ledger row changes status.

## 1. What it was

No tier ran `cargo doc` (ADR 1273 says so), so nothing saw an intra-doc link go stale when the
module trees of ADRs 1906–1914 moved the items it named. `RUSTDOCFLAGS="-D warnings" cargo doc
--no-deps --document-private-items` failed on all three crates: 21 warnings on `render-cpu`, 59 on
`raster-gpu`, 15 on `viewer-ui`'s library and 10 on its `quorra` binary — 105, of four kinds.

## 2. Decision

**The build is tier 1's, through `tools/state.sh docs`**, a small walk whose section holds the list
of crates at zero. The list is a ratchet: a crate joins it the round its own build reaches zero and
does not leave. `--document-private-items` because this tree is read as source and its private
items carry most of its reasoning; a public-only build would check a tenth of the links.

**Each class has one answer**, so that the next crate to join is mechanical:

| class (count here) | answer |
|---|---|
| a letter a quotation lowered, `"[t]he` (33) | escaped, `"\[t]he`: `CLAUDE.md`'s mark of the change stays, rustdoc renders it as written, and the blockquote gate's `quote::normalise` drops the backslash with the brackets |
| a public page linking a private item (10) | the name in a code span, not a link. Not `pub(crate)` on the target, which is still private to rustdoc's lint, and never a widening: a public build would link to nothing |
| a name that is a function and a module (23) | `name()` for the function, `mod@name` for the module, or a reference definition `[`name`]: fn@name` where the sentence should read as it did |
| an item compiled only for tests (13) | a code span, because the documentation build does not compile `#[cfg(test)]`; re-pointed instead where a built item says the same thing (`stroke_pieces` for `stroke_polylines`, `Self::forecast` for a test-only `predict`) |
| an item a module split moved out of scope (15) | an explicit path, ADR 1912's practice: `[`LayerPlan`](crate::encode::LayerPlan)` |
| a name nothing has any more (4) | the item it became: `fill_rectangle` → `fill_rectangles`, `dispatch_into` → `ComputePersist::grow`, `Self::ask` → `Window::ask` |
| an explicit target the text already resolves to (7) | the target dropped |

The hypothesis the brief carried — that every private-item link resolves by `pub(crate)` on the
target or by prose — held for prose only: rustdoc's `private_intra_doc_links` reads `pub(crate)` as
private, so the prose half is the whole answer.

## 3. What it costs

2.1 GiB at its peak and 11.3 s warm for the three crates. It sees only `///` and `//!`: a `//`
comment naming a moved item (`render-cpu/src/image.rs`'s `images`) is still a grep's to find.

## 4. The proof that nothing moved

Every line this change touched in the three crates is a comment — `///` or `//!`, and the three `//`
lines the same round corrected beside them — which `git diff -U0` over its 54 files shows; a doc
comment compiles to an attribute no code reads.
