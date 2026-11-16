# 2907 — A Type 3 glyph procedure's width is written as its font's `/Widths`, under the loss

Session 2045, a PDF/A round in the audit worktree, on the owner's answer `doc/questions/A865`.
Status: accepted and built. In `pdf-transform`: `archive/type3_widths.rs` (`restate`,
`RestatedType3Width`), the loss `type3-width-as-drawn` and the rewrite `type3-width-restated`
(`archive/decision.rs`, `archive/rewrite.rs`, `archive/rewrite/streams.rs`), the row's remedy in
`archive/decision/content_stream_fences.rs`, the photograph's skip in `archive/fenced.rs`, the history
in `archive/prepare.rs` and `prepare/metadata.rs`, the report in `archive/report.rs`;
`doc/profiles/as-shown.toml` and `as-if-printed.toml`.
Tests: `crates/pdf-transform/tests/archive/type3_width_as_drawn.rs` (four),
`crates/pdf-transform/tests/archive/photographed.rs` (the photograph now under `state-every-loss`).
Context: ADR 0816 (the fence this opens), ADR 2638 (`content-as-drawn`, the construction), ADR 2847 (the
photograph this replaces under the two shipped profiles), ADR 2516.
Clauses: ISO 19005-4 section 6.2.10.5; ISO 32000-2 §9.6.4 (Tables 110 and 111), §9.2.4.

## Context

Section 6.2.10.5 of ISO 19005-4 holds a Type 3 font's `d0` and `d1` operands to the glyph's width by
ISO 32000-2's Table 111, whose `w x` "shall be consistent with the corresponding width in the font's
Widths array". The operand is inside a glyph procedure, a content stream ADR 0816's fence carries byte
for byte, so `as-shown` refused the row and `as-if-printed` photographed every page reaching the font
(ADR 2847). Q865 asked whether the operand may be written as the `/Widths` entry; A865 answers yes under
both shipped profiles, with `refuse-any-loss` still stopping, and adds the owner's standing rule:

> the corpus is not what counts! Even if we have no document at all, we should consider such
> mitigations.

## Decision

1. **The fence opens for this one edit and no further.** The first operand of a procedure's opening
   `d0` or `d1` is written as the font's `/Widths` entry for the code reaching it, in §7.3.3's
   spelling; every other byte of the procedure — `w y`, `d1`'s bounding box, every operator after —
   is the producer's. No other operand of any content stream is opened by this record.
2. **It moves no mark.** §9.2.4 makes `/Widths` what advances the text position and this reader reads
   `w x` nowhere (`pdf_model`'s `run_type3_op`), so the window lays the page out alike before and
   after; a test compares the text layer's positions.
3. **It is a loss**, `type3-width-as-drawn`: the operand the producer wrote is gone from the
   procedure. `xmpMM:History` names each procedure, glyph, operand and width written (a condition of
   the rewrite, as `content-as-drawn`'s runs are), and the report lists them.
4. **A procedure two fonts reach at two widths is copied.** The fonts whose widths the procedure
   already states keep it; the procedure is written in place for the first other width and copied at
   a spare object number for each further width, and each font needing a copy is given a `/CharProcs`
   naming it. One font reaching one procedure by two codes at two widths keeps the refusal: no operand
   and no copy answers it without re-encoding the font, which would change what its codes name.
5. **The profiles.** `as-shown` and `as-if-printed` grant the loss; the photograph is then not taken
   for this row. `state-every-loss` states no line for it yet and keeps ADR 2847's photograph;
   `refuse-any-loss` stops on it.
6. **The corpus is evidence and not the gate** (A865): this remedy is built from the clauses, and its
   fixtures are the clauses' cases, whether or not any held document needs the copy.

## Consequences

`issue19634` and `issue19971` convert with their text rather than as photographs where this row was
the only reason. A procedure reached through an annotation's appearance is written as any other: the
edit is to an object, not to a page.
