# 1109 — A `shall` conditioned on an operation nobody performs

Date: 2026-09-15. Branch `batch-1105-1110`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
No ADR: the clause and `doc/questions/A63` settle it between them.

## The question, and the answer

Is a viewer preference about printing owed by a viewer that does not print? **Per entry, not per
half** — each of Table 147's eight print entries states its own condition, deciding who owes it.

- **Five name a dialogue** — `/PrintScaling`'s first sentence, `/Duplex`, `/PickTrayByPDFSize`,
  `/PrintPageRange`, `/NumCopies`. A party displaying no dialogue never reaches the condition; it
  reads the value, keeps Table 147's stated default distinguishable from its "implementation
  dependent" silence, and hands it where the party that *does* display one takes it —
  `Query::Preferences` and `quorra_preference` already do, so those five are discharged here.
- **Three name no dialogue** — `/PrintArea`, `/PrintClip`, and `/PrintScaling`'s second sentence
  ("[i]f the print dialogue is suppressed … this entry nevertheless shall be honoured"). Their
  condition is the print operation, which `CLAUDE.md` does not exclude and RFC 0004 proposes. **That
  is `partial`**, not A63's `departed` (one sentence decided against with an ADR) — nothing here was
  decided against. §12.5.6.22's tiling and n-up bullets get the same verdict for a stronger reason:
  each is conditioned on a *selection*, so no file states it and no census can rank it. **Both rows
  stay `partial`; neither was handed to 1105.**

## What was measured, and what it found

`crates/pdf-model/examples/print_preference_census` reads the print half through this crate's own
reader rather than the tokens. Of the 974: 58 state a `/ViewerPreferences`, 2 a non-default print
entry. Of the 65 944 crawled: 20 283 state the dictionary, 165 a non-default print entry. **Not one
of the 66 918 states Table 148's `/Enforce`**, so §12.2's sharpest print sentence has no witness.

**The census found a defect, not only a population.** All three crawl documents stating
`/PrintPageRange` begin a pair at 0, one ending at -1 — against "[t]he first page of the PDF file
shall be denoted by 1". Such an array states no sub-range and now falls to the entry's
implementation-dependent default, as an odd-length one already did; reading them zero-based was
refused as guessing at an intent the file does not state.

`crates/viewer-core/tests/print_preferences.rs` pins both halves — the eight reach a host entry by
entry, and the page's raster is byte-identical to the same page whose catalogue states no dictionary.
Calibrated by planting the confusion it guards: `page.rs` reading `print_area` where it reads
`view_area` fails it, its neighbour green (trap 13).
