# 1256 — A fax decoder of this tree's own conceals the damaged rows Table 11 tolerates

Date: 2026-09-28. Branch `batch-1254-1259`, worktree `/home/AI/pdf-viewer-rounds`, shared with five
sibling rounds. ADR: [1349](../adr/1349-a-fax-decoder-of-this-trees-own-conceals-the-damaged-rows-table-11-tolerates.md).
Row §7.4.6 `partial` → `implemented`.

## What was built

- ITU-T T.4 (07/2003) and T.6 (11/1988), fetched free from ITU after its server answered `500` for
  ten minutes, extracted to `doc/md/T.4.md` and `doc/md/T.6.md`, entered in
  `doc/third-party-data.md`: all rights reserved, so cited and paraphrased, never quoted.
- `crates/pdf-ccitt`: Group 3 one-dimensional, mixed and Group 4 from T.4's tables, with
  `/DamagedRowsBeforeError` built in — a damaged row's end found from its own first bit, the row
  delivered as the one above or white, the error at the row past the count. The concealment does
  not apply to Group 4 or `/EndOfLine` false, as Table 11 states; the brief's Group 4 fixture was
  the premise the clause corrects (ADR 1349 section 4).
- `pdf-sandbox` decodes with it in the worker; `Bilevel::concealed` and
  `CcittParameters::damaged_rows_before_error` cross the pipe; `build.rs` hashes the new crate
  into the worker's identity. `pdf-model` no longer refuses the entry and says a concealment beside
  the drawing.
- Fixtures: T.4 Figures 10 and 11's thirteen coding examples, the concealment's cases, and a
  hand-built four-line page in `tests/ccitt_bound.rs`. A fuzz target, `fuzz/fuzz_targets/ccitt.rs`.
- `examples/ccitt_decoder_census.rs`: every corpus fax stream decoded by both decoders, bit for bit.

## What the census found

190 537 fax streams over 90 763 documents: 190 500 identical bit for bit, 36 different, every one
damaged or malformed (25 that `mupdf` reports damaged too, where `hayro-ccitt` shortened runs or
retried codes in the other colour's table). 17% less decode time. Its first run corrected two
rules of mine: an end-of-line code demanded after every line, and empty runs refused (ADR 1349
section 5).

## Left

- `4113564.pdf` writes `stream`, a space and a line feed; §7.3.8.1 allows only CRLF or LF, and
  `pdf-syntax` hands the space and line feed to the filter as data. Not this filter's to fix.
- Inline images are not in the census (their data is the interpreter's to find).
