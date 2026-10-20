# 2090 — A relocated appearance's matrix is written to the bit it was computed at

Session 1627, the clause slot of batch ninety-eight. Status: **accepted**; the code is a `-U0`
hunk handed to the archive slot, whose directory `crates/pdf-transform/src/archive/` is.
Code: `crates/pdf-transform/src/archive/preserve.rs` (`matrix_operand`, its two call sites, a unit
test), `crates/pdf-transform/tests/archive.rs`
(`relocated_marks_are_placed_by_the_matrix_the_annotation_was_drawn_with`).
Clauses: ISO 32000-2 §12.5.5, §8.3.4, §8.10.1, §7.3.3. Companion: ADR 2091.

## 1. What was measured

Round 1615 found that the veraPDF witness `PDF_A-4/…/6-3-1-t01-fail-f` — one `FileAttachment`
whose `/Rect` is `[75.0341 720.288 82.0341 737.288]` — drew 47 pixels differently (at most 17 of
255) at 72 dpi after ADR 1123 relocated its paperclip, 477 at 720 dpi (at most 64). The brief's
hypothesis was that the annotation path snaps the origin to a device pixel. **It does not.** The
display lists of the annotation and of the same stream under `q 1 0 0 1 75.0341 720.288 cm /X Do Q`
carry the same transform, `e: 75.0341, f: 720.288`, and the relocation wrote `1 0 0 1 75.03 720.29
cm`: `preserve::number` formats with two decimal places. Rewriting only that operand to
`75.0341 720.288` in the converted file took the page to byte identity at 72 dpi and to one pixel
at 720 dpi (that pixel is section 3's). A control, the same stream re-encoded unchanged, was
identical to the converted file, so the rewrite itself moved nothing.

## 2. Decision

**The matrix's six operands are written as `pdf_syntax::write` writes every real**: the `f32`
widened to `f64` and spelled as the shortest decimal that reads back as that double, an integral
value as an integer, never in exponential format (§7.3.3: "A PDF writer shall not use the
PostScript language syntax for numbers … in exponential format"). The reader holds a real as an
`f64` and the interpreter narrows it, so the `f32` the annotation path composed is the one the
relocated `cm` composes — §12.5.5 computes `A` with no rounding step and §8.3.4 concatenates
whatever operands it is given. Spelling the `f32` itself (`75.0341`) is shorter and would be
decimal-to-double-to-single, which rounds twice; the widened spelling is exact by construction,
and ADR 0818 is the same argument for the serializer.

`number`'s two places stay for the appended page's own layout — a type size, a margin, a
baseline — which are this program's choices and not a clause's.

## 3. What is not this decision

The one pixel left at 720 dpi, `(798, 567)`, is no placement's: its smallest mark is the
paperclip's stroke, the geometry covers 0.949 of the pixel, and the stroke drawn with §8.10.2's
`/BBox` clip — which the `Do` path keeps — gives that (`(5, 6, 15)`), while the same stroke drawn
with no clip — which the annotation path has, ADR 0165 taking a box that removes no geometry back
off — gives full coverage `(0, 0, 0)`. The same marks drawn straight from the page with and
without `0 0 7 17 re W n` reproduce it. It is `render-cpu`'s, reported and not fixed here.

## 4. Measured

Over the veraPDF corpus's annotation witnesses converted under `as-shown` (2b, 4, 4e, 4f), the
pages whose relocated output differs from the annotation path at 72 dpi: `6-3-1-t01-fail-e` 160
pixels (at most 37) → 0, `6-3-1-t01-fail-f` 47 (at most 17) → 0; `fail-g` is ADR 2091's.
