# 2266 — Twenty-four held files read, and the launch floors re-taken

Lane `gwg`, the silent-repair census burn-down, ninth pass (ADRs 3477, 3521, 3757). ADR 3765.

- **New classes**, each section 5.1 alone, answered by the whole-file rewrite and the output's own
  check: `model/outline-entry-unusable` (Table 151's `/F` and `/C`), `model/page-label-entry-
  unusable` (Table 161's `/St` below 1 or of another type), `model/soft-mask-backdrop-unusable`
  (Table 142's `/BC`, which `table_142_faults` already said).
- **On existing classes**: a field `/Ff` of another type (`field-entry-unusable`); Table 194's `/H`
  and `/V` and Table 169's `/I` (`annotation-entry-unusable`); a `CMap` file's own `/WMode` other
  than 0 or 1, now read as the nearer mode as the dictionary's is (`font/wmode-unusable`).
- **A defect**: an unreadable selector in a `cidrange`, `bfrange` or a range's array was dropped
  without marking the map damaged; `font/cmap-damaged` now hears it.
- **Marked**, each read against its source: §12.3.2.3's first page for a structure destination
  with no content (a fallback to `/D` was made and reverted on reading the function's own doc),
  §9.6.5.4's `post` "(if one is present)", Table 165's `/Next`, the catalog declines, the stream
  `/Length` readings, and the declines whose refusal is named where a decode or draw meets it.
- `HELD` 122 → 74: the twenty-four named files to zero.
- **Main's launch miss settled**: `8f54e0483` (ADR 3729) fits every `/SA` stroke. Both page ones
  draw the PDF Association's boxes in `/SA true` strokes of about 1.6 pixels, now two whole pixels
  on the processor (render_at at 1 and 2, before and after: 5398 and about 10 600 pixels changed
  inside the boxes' 511-pixel square, every box drawn); on the device a fitted stroke no longer
  queues a parallel sheet job. A true saving: floors re-taken (ADR 3765).
- Left, in order: the next held files; a negative `/DA` `Tf` size (Table 103 does not forbid it,
  so the auto-sizing is a reader's question, not a class); `pdf-transform`'s `/Annots` readers.

- `tools/state.sh --round 2266 launch`: 36 figures banded, 2 outside, the two floors above —
  `PDF20_AN001-BPC` 5.000, WTPDF 8.590, re-taken to 4.0 .. 6.5 and 7.5 .. 10.5 over rounds 2259
  (two runs), 2262, 2264 and this one. No per-object check on a hot path: `parser.rs`,
  `parsing.rs`, `truetype.rs`, `cff.rs` and `dct.rs` changed comments only, and `cmap.rs`'s
  change is once per `CMap` file and per unreadable entry, so no instruction count was taken.
- Census with and without the pack, after the rebase onto 2263: each run's twelve summary lines are
  2264's, and no line names a class this round logs. veraPDF 1.30.3 over the 5523 outputs
  written as conforming: 40 not compliant, 12 unfinished, as 2264's.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`, rebased on `13328b40e`: fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 6322 of 6325 — the
manifest's two entries (§10.5) and this record before its gates were written; rustdoc of the four
library crates touched 0; fuzz 0; `cargo test -p conformance` 0.
