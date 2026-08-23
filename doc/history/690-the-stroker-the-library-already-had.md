# 690 — The stroker the library already had

A stroke's coverage now meets its clip as a *set* rather than as a product, because
`tiny_skia::Path::stroke` and `tiny_skia::Path::dash` are public and are the same stroker and
dasher `stroke_path` calls — so the composition needed no stroker of ours, which is the price
`doc/todo/11` item 4 had been carrying since ADR 0280.

Date: 2026-08-23. Argued in **ADR 0535**.

## What moved

- `crates/render-cpu/src/lib.rs` — `draw_stroked_outline`, the fourth branch of `draw_stroke`,
  and `RECTANGULAR_OUTLINE_VERBS` with the unit test that pins it.
- `crates/render-cpu/src/scan.rs` — `stroke`'s doc comment, which claimed a price that no longer
  exists and a residue that is now different.
- `crates/render-cpu/tests/clip_intersection.rs` — a `Mark` axis (`f` against `S` of the same
  outline) and three scenes: the identity at three scales, the identity under a soft mask, and
  the two operators asserted to be one mark.
- `crates/pdf-model/examples/coincident_edge_probe.rs` — the operator as a second axis, so the
  ladder is eight rungs of a fill beside eight of a stroke.
- `crates/render-quorra/tests/corpus.rs` — `issue19083.pdf` added to `DIFFERS_IN_SHAPE` with why.
- `doc/conformance/ledger.toml` §10.7.4; `doc/todo/11` items 4 and 7; `doc/todo/_scan-conversion.md`
  departure (4) and departure (1); `doc/QUORRA_FEEDBACK.md` section 24b;
  `doc/traps/pixels-and-rasterisers.md` trap 2's sixth instance.

## The order of the round

**Planted first (trap 13).** The three scenes were written and run against the unfixed tree before
any source moved: 51 levels of 255 apart at the named boundary pixel, 24 under a soft mask, 3
against the fill of the same outline. The backup-and-restore of `lib.rs` used a scratch copy rather
than `git checkout --`, which is what cost session 645 two ledger corrections.

**Then the reading.** Neither §10.7.4's clipping paragraph nor §8.5.4 names an operator, and §8.4.3
gives a stroke a shape. Then the library: `painter.rs`'s `stroke_path` is `dash` + `stroke` +
`fill_path`, and both of the first two are public API this tree already calls in three other places.

**Then the boundary.** `treat_as_hairline` maps the width along the transform's two basis vectors
and compares an approximate length against 1; `pdf_render::thinnest_line` is a singular value. Equal
for a similarity, a factor of √2 apart under a shear — so the library was choosing on a boundary
neither backend had chosen. It is `pdf-render`'s now, and that is trap 2's new sixth instance.

**Then what fell out.** Once a stroke's mark is a fill's mark, ADR 0476's exact rectangle coverage
reaches a butt-capped axis-aligned rule. That is what makes the two operators assertably one mark;
without it the fill-against-stroke scene reads three levels apart forever.

## Measurement

The machine's load average was **37 over 24 cores** for the whole round, so **no timing figure was
taken**. Every number below is either a raster value or an instruction count, both of which a loaded
machine does not move. `PDFREF_CACHE` pointed at the shared warm cache
(`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`).

Both arms of every A/B were built and run in one sitting, the "before" arm produced by disabling the
new branch at its single call site rather than by reverting the tree.

Gates: `fmt` and `clippy` with `RUSTFLAGS="-D warnings"` silent (the `viewer-qt` `cargo:warning=`
lines are gcc's, on a cold build, and are `doc/todo/02` §2's documented non-lints); the whole of §2
green including the oracle, the corpus, both censuses, the quorra corpus gate and `fixed_documents`;
`conformance` green after two corrections it caught — a `§` pointing at `QUORRA_FEEDBACK.md`, which
the citation checker rejects by design.

Sweeps: `quotations` and `pointers` name nothing of this round's; `overtaken` does not name
`DIFFERS_IN_SHAPE`, because the note cites this round's own ADR, which is the rule that keeps it off.

§5's binaries were rebuilt and installed into **this worktree's own** `target/`, which is 673's
choice rather than 670's: nothing of an unmerged branch is put where the main tree's `target/`
would hand it to a person, and the merge round still owns the main tree's copy.

One caveat this round did not choose: `PDFREF_CACHE` pointed at the **shared** 2.2 GB cache rather
than at a copy of it, because that is what the round was told to do. 663, 667 and 670 copied it, on
the argument that the oracle's agreements should not be a read of a directory three neighbours are
writing. Nothing in this round's numbers looks like a cache defect — the before and after arms were
run minutes apart against the same directory, and 1769 of 1794 per-page lines are byte-identical
between them — but the difference is recorded rather than assumed away.

## What is left of item 4

An image's edge, a non-isolated group's raster, a group whose opacity is below 1.0 somewhere, and
both other backends. The stroke bullet is closed and the item is one bullet shorter than it has been
since the four-hundred-and-forty-third session.
