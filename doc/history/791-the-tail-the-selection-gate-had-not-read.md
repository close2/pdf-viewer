# 791 — The tail the selection gate had not read

Sent at the text-extraction tail — the selection-geometry ratchet's out-of-bounds documents,
never read as a population since ADR 0421 set the list. Parallel round, worktree `r791`,
branch `round-791` from `f81e038f`. ADR 0726.

## What the round found and did

**The gate now prints the classification it already computes.** Each out-of-bounds document's
line says how its words divide between the verdict's two bounds, a summary line classifies the
tail as a population, and `PDFVIEWER_SELECTION_DETAIL=1` prints every out-of-bounds word with
both deltas. Calibrated by hand off the same listing before being believed (trap 13): the
baseline summary's 5 / 17 / 0 matches the per-document lines, and the out-of-bounds words sum
to the verdict line's own 11163 − 10969.

**A pair is judged in the word's own reading frame.** The tight 0.5 pt bound is §9.4.4's
positioning arithmetic and the unjudged extent is each extractor's ascent/descent convention
(ADR 0323 Finding 3) — statements about the text's reading and cross axes, which a `/Rotate
90` page and §9.7.5.1's vertical writing mode swap. `hello_world_rotated.pdf` was failing at
7.45–10.15 pt of pure convention while its reading-axis placement agreed to a hundredth of a
point; it is in bounds now and off the ratchet list. Two simpler orientation rules were tried
and watched fail before the kept one (ADR 0726 has both misfires with their documents): no
box's *shape* may decide the frame, only the placement, which the interpreter knows
(`WordBox::vertical`) and both boxes assent to.

**The tail is classified by mechanism, each document priced by its failing bound**, in the
note above `SELECTION_BELOW_FLOOR` — seven mechanisms over 21 documents, every one read
against its own content stream:

- §12.7.4.3's layout hand-off (6, exactly 1.00/2.00 pt horizontal): `/NeedAppearances` text
  fields; poppler's inset, measured from `/Rect`, is the `/BS` width plus 2 pt, ours §12.5.4's
  border width; the clause hands positioning to the processor and matching poppler would be
  principle 5's forbidden direction.
- Table 120's pair obeyed against ADR 0216's refusal (3, vertical centre, dx 0.00):
  `pdftotext` obeys `/Ascent 8 /Descent -2` literally — a 0.2 pt tall box — where this tree
  answers §9.2.2's em box.
- No stated pair (2, vertical centre): em box against the reference's font-derived box.
- Vertical writing judged on its reading axis (2): `vertical.pdf` isolates a quad convention
  of ours — the horizontal ascent/descent band carried along the reading axis where §9.7.4.3's
  vertical displacement is the honest extent — named as owed.
- A rotated page with nothing embedded (1), and substituted metrics (4): each reader's own
  face supplies the advances the file does not.
- Text at an angle the frame cannot follow (2): oblique and sheared text put the convention on
  both axes.
- `issue6127.pdf` (1), undiagnosed and ours: two words 3.02 pt — one space advance — from
  where `pdftotext` **and** `mutool` both put them, so two independent readers agree against
  this tree on one line's §9.4.4 arithmetic.

## Measured

Baseline (pristine, warm shared extraction cache, 958 hits 0 misses): 98.26% words in bounds,
10969/11163, 486 of 508 documents, 22 out of bounds — the briefing's figures reproduced off
the run. After: **98.28% (10971/11163), 487 of 508, 21 out of bounds**, no document newly out,
pairs total identical, judged set unchanged at 508. The two intermediate orientation rules
printed 483 and 485 of 508 and were discarded for it.

Full §2 sequence on this worktree after the final code edit, green end to end: `fmt` clean
(after one round of `cargo fmt` on the new code — its first run failed the check, which is the
sequence doing its job), `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`
clean, `nextest` **2728 passed, 18 skipped**, doctests, fuzz check, corpus (**974 documents, 67
incomplete**), oracle (**983 agrees, 61 contradicted, 836 ambiguous, 3 our geometry, 2
reference geometry, 42 not comparable, 18 no render** — byte-for-byte round 780's census,
rendered fresh into this worktree's own cache, 6698 produced at a 0.1% hit rate, 177 s), the
three text gates (**99.2%** against `pdftotext` with `TEXT_BELOW_FLOOR`'s 22 unchanged,
**99.8%** against PDFBox with its 4 unchanged, and the selection verdict above), selection
census (0 panics), accessibility census, dates, xmp, jpeg2000, quorra corpus (**932 agree, 22
differ, 3 refused, 17 not comparable**), fixed documents (**40 checked, 0 absent**) and
conformance, all `ok`. Load ran around 7–12 throughout with sibling rounds building; no
budget-shaped failure appeared, and the oracle line's own census matching the warm-cache
baseline is the control.

Sweeps, before and after against the same run's oracle log (the diff touches nothing the
oracle links, and the same runtime-reading binary measured both sides): `--bin unpriced` **93
failing bounds over 61 pages, 93 named, 0 not**, unchanged; `--bin quoted` **237 figures read,
123 confirmed**, unchanged; `--bin quotations` 6648 → **6650**, both new quotations verbatim
(§9.4.4's displacement sentence in the gate, §12.7.4.3's hand-off in the note), 38 diverging
unchanged; `--bin pointers` 98 absent and 13 undefined symbols, unchanged; `--bin overtaken`
47 → **46** over one more decision record — the one hit that cleared is
`SELECTION_BELOW_FLOOR` itself, whose newest citation was ADR 0421 and is now this round's ADR
0726, which is the sweep's own rule for a rewritten note. Every delta accounted.

## Changed

- `crates/pdf-model/tests/text_extraction.rs` — the per-document bound classification and its
  summary line; `PDFVIEWER_SELECTION_DETAIL`; `WordBox::vertical` and the reading-frame
  judgment in `PairDelta::in_reading_frame`; `SELECTION_BELOW_FLOOR` down to 21 with the
  population note. The derivation and both bounds untouched.
- ADR 0726.
- No ledger row: the clauses the change cites — §9.4.4, §9.7.5.1, §12.7.4.3 — are
  `implemented`, `implemented`, `partial` already, and the round implements no new normative
  requirement.

## Owed

- `vertical.pdf`'s quad convention: a vertical-mode glyph's text-layer quad should take
  §9.7.4.3's vertical displacement as its reading-axis extent; a change to
  `Interpreter::show`'s quad construction with the selection census in its blast radius.
- `issue6127.pdf`: the one tail document where both references agree against this tree
  (3.02 pt, one space advance, a four-font line with `Tc` kerning); the note names it as ours
  to explain.
- The oblique/sheared class has no judgeable frame under a 90-degree transposition; if it is
  ever worth judging, the frame must come from the text matrix itself.
