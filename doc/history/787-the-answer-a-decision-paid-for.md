# 787 — The answer a decision paid for, and the run was about to give

The batch's general-improvement round, chosen rather than assigned. It took the one line
`doc/todo/41` still carried — *check whether §12.5.5's route still needs the damage answer before
the run* — answered it, and confined `NestedContent::damage`'s pump-to-the-end to the one route
that owes it (ADR 0723).

## Why this item

The briefing weighted a robustness or performance subject after three coverage rounds, and the
instruments offered exactly one that is both: `doc/todo/41`'s closing bullet, a full decode of a
windowed appearance stream run at *decision* time, once per stored appearance per interpretation,
in front of a draw that decodes the same stream again. Every bomb is in that population by
construction (the decoded-stream memo declines what it cannot hold), so it is principle 3's kind
of cost; and a benign appearance above the memo's allowance paid it on every re-interpretation —
which §12.5.3 makes a per-wheel-tick event on a view-dependent page. `doc/todo/47`'s resize
attribution was the other candidate and is fenced off with the owner's measurement loop.

## What was found that the item did not say

The pre-pass also **double-reported**: a damaged windowed appearance that was then drawn carried
the same shortfall twice, once named by the decision ("a Square annotation's appearance stream")
and once generic from the run ("an annotation's appearance stream"). Nothing pinned the count,
because the only fixture was held-whole, where the decision's answer is free and the run says
nothing. And ADR 0359's argument — the answer must precede the run because regeneration splices
the bytes — turns out to bind exactly one route, `Content::Constructed`, which
`appearance::regenerates` confines to a text or choice widget under `/NeedAppearances` or a
replaced value.

## What moved

One `match` in `annotation::appearance_damage`, taking the decided `Content`: a stream the draw
will read answers with `stated_damage` (free; the windowed shape's damage is met and reported by
the run itself), and only the regenerated route still pays the pump it genuinely owes.

| witness (one page, one Square, windowed `/AP /N`) | before | after | |
|---|---|---|---|
| ADR 0586's hex-armoured zlib bomb, 4 174 537 B encoded | 23 617 148 028 instr; 1.14–1.38 s | 22 052 215 589 instr; 1.06–1.14 s | −6.63 % |
| benign 5.24 MiB appearance that draws marks | 233 371 525 instr | 167 667 365 instr | **−28.2 %** |

Wall clock three runs an arm, alternating, both arms built in one sitting; the machine ran load
12–42 throughout (three sibling documentation rounds), which is why the instruction counts are
the authority. ISO 32000-2's 1023 pages: 35 430 168 416 → 35 431 060 484 (**+0.0025 %**),
identical command totals — the corpus population is entirely held-whole, where the two spellings
of the question read the same memo field. The bomb row is the first interpretation's price; the
benign row repeats forever, because a stream that draws marks is never memoised as a refusal.

### The generator, so that no round rebuilds it

ADR 0586's bomb (`doc/history/712-…` has `bomb()`), with the form `XObject` moved into an
annotation's `/AP /N`:

```python
# both witnesses: one page, /Annots [5 0 R], object 5 =
#   << /Type /Annot /Subtype /Square /Rect [10 10 60 60] /F 4 /AP << /N 6 0 R >> >>
# bomb arm, object 6:
#   << /Type /XObject /Subtype /Form /BBox [0 0 50 50]
#      /Filter [/ASCIIHexDecode /FlateDecode] >> with binascii.hexlify(bomb(2)) + b">"
# benign arm, object 6:  /Filter /FlateDecode over zlib.compress(content, 9) where
content = (b"1 0 0 rg 0 0 20 20 re f\n"
           + b"% eight b\n" * (512 * 1024)      # 5.24 MiB decoded, over the 4 MiB budget
           + b"0 30 20 20 re f\n")
```

Timed with `open_one` (the "interpreted at" line), instruction-counted with
`valgrind --tool=callgrind` over the same binary pair.

## The sequence

Whole, this being a change in `pdf-model`. `fmt` clean · `clippy --workspace --all-targets`
under `RUSTFLAGS="-D warnings"`, exit 0 · `nextest` **2728 passed, 18 skipped** · doctests ·
the `fuzz/` check · both workers built first · corpus gate ok in 2.64 s · `pdfref-hayro` built ·
oracle ok in 109.50 s at load ~13 — the standing verdicts unmoved, 61 contradicted · text
extraction **98.26%**, 486 of 508 documents in bounds · selection census ok · accessibility
census ok · dates · XMP · JPEG 2000 · `render-quorra` corpus **932 agree, 22 differ, 3 refused,
17 not comparable** · `fixed_documents` 40 of 40 · `cargo test -p conformance` **200 passed**,
re-run last after the ledger and history edits.

**`doc/todo/00`'s step 7 was not re-run and this says why**: no display list moves — the ISO
sweep's command totals are identical, and the corpus, oracle and quorra gates report unchanged.
What the change can alter is a report string on a damaged windowed appearance, a population no
corpus document is in (the corpus's damaged streams are held whole or are forms inside
appearances, which the run has always reported).

§4's sweeps, against a pristine baseline (the main checkout at `b8f44a0c`), every delta
accounted: `callers` moves `NestedContent::stated_damage`'s listed caller to `annotation.rs`
(this round's); `overtaken` counts one more decision record (ADR 0723, no new hits);
`pointers` +2, both this round's files; `quotations` +4, all in the new ADR and none claiming
to be the standard's; `tables` +1 agreeing (Table 224's `/NeedAppearances`). The three
`looked in: tmp/hayro/…` lines differ because the baseline ran in the main checkout, whose
gitignored `tmp/` the worktree does not have — environmental, not a delta of this round.

§5's seven binaries and `libviewer_ffi.so` built and installed; `target/` held none of them when
the round started.

## Ledger

§12.5.5's row: the two sentences about where the damage answer lives amended (the full answer
before the run is the regenerated route's alone), the two new tests added. The row's `partial`
status is unchanged and about other clauses of §12.5.5. The `ledger` binary re-run over the edit.

## Tests

`a_windowed_appearance_streams_damage_is_reported_once_by_the_run` ·
`a_regenerated_widgets_stored_stream_still_reports_its_damage` — **both planted before they were
believed** (trap 13), above the calibration commit: with the old shape restored (always
`damage()`) the first fails on a doubled report and nothing else fails; with the
over-correction (always `stated_damage()`) the second fails on a lost report and nothing else
fails. The pinned held-whole test is byte-identical throughout.
