# 1289 — A ratchet's log is a record, and a knob a document names is one the code reads

Batch forty-four, instruments slot. No ledger row moved: the contract named none. ADRs 1415, 1416.

## The comment history (`tools/state.sh comments`)
- Before: grep 226; history 62, unread 2, legitimate 83, code 79. After: grep 163; history 0,
  unread 0, legitimate 84, code 79; `raster/` 0 throughout.
- `pdf-archive`, 24 lines in 13 files: each rewritten as its current reason or deleted (the
  retired `components` paragraph in `reach.rs`); `survey.rs`'s "session" meant a sitting and says so.
- `pdf-model/tests/corpus.rs`: the 35-line-hit `INCOMPLETE` log (about 400 lines) moved verbatim to
  ADR 1415's appendix; the comment keeps the rule. ADR 0730 never kept it — the keeping was the
  comment's own sentence; ADR 1023 gives chronology to records; ADR 1401's names took its job. Three
  `Tally`/calibration lines outside the block rewritten (targeted, comment-only).
- Unread 2: `survey.rs:87` (a sitting, rewritten) and `quotations.rs:24` ("session / bookkeeping"
  across a wrap): the classifier now reads the next line's head when a comment ends in the word.

## Variables
- `doc/verify.md`, `doc/todo/02`: `PDFVIEWER_QUORRA_{COVERAGE,SCALE,GLYPH_QUANTUM}` →
  `PDFVIEWER_RASTER_*`; also `doc/todo/46`, `doc/todo/49`, `raster/doc/HANDOVER.md` (named edits).
- `tools/conformance/tests/variables.rs`: every `PDFVIEWER_*` a live document or a Rust comment names
  is a string literal the code reads; records excluded; a planted retired name calibrates it.

## Documents
- PLAN: the corpus gate's named population (ADRs 0730, 1401); `variables.rs`; `state.sh comments`.
- state-of-play: raster's set asked where answerable (ADR 1397), the fan-out (ADR 1395).
- crate-map: tagging and `Form` naming (ADRs 1393, 1394), `archive/transcode.rs` (ADR 1400),
  raster-gpu's ask and fan-out, `gate-ratchet`'s population holding `INCOMPLETE`.
- HANDOVER: a row for a round that measures latency. `state.sh navigation`: absent 11 → 12, the new
  one `doc/questions/Q171`, which lands with the owner's own files as A169/A170 do.

## Todo
- 65: §12.10 waits on Q171 (not decided); §7.4.9's texts per the note's date; curves re-searched:
  no `bp512`, `ed448-goldilocks` still pre-release, `ed448-goldilocks-plus` 0.18.1 named as a
  candidate for `doc/stack.md`'s terms.
- README + 12, 13, 14, 25, 26, 37, 39, 53 (ADR 1416): none is only a pointer, none reduced; each
  header names what cites it, and the eight moved to a table of done files apart from owed work.
