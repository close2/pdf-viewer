# 2837 — XFA's dynamic half is counted by this tree's reader, and priced at page one

Session 2009, a measurement round (Annex K; XFA 3.3 chapters 8 and 9). Status: accepted. Carries
out `doc/todo/76` row 10 and `doc/questions/A828` group 4; the result is `doc/todo/78` and
`doc/questions/Q840`. Builds nothing of the dynamic half, which stays excluded.

## Context

`doc/scope-ratifications.md` keeps XFA's dynamic half excluded "until a measurement round has priced
laying out page one of the corpora's `/NeedsRendering` documents". RFC 0011 section 4 counted 1 479
files with `mutool` in 2026-10; this machine now holds the safedocs cache too, and the question is
no longer *how many* alone but *what page one would cost*.

## Decision

- **The count is this tree's own reader's.** `pdf-model`'s `xfa_dynamic_census` example opens every
  file through `pdf_syntax::Document` and asks `pdf_model::xfa::stated` for Table 29's flag and
  `Template::dynamic` for page 333's test — the functions the window already acts on, so the census
  and the program cannot disagree about which documents are dynamic. A file is read where its
  offsets point (`FileBytes::on_disk`), because the corpus disk is rotating and shared and a whole
  read of 93 GB at the seek rate it gave would have taken most of a day; 24 readers side by side under the
  walk bound took the run to 76 minutes.
- **The layout grammar is counted outside Rust, and the instrument is planted.** `tools/xfa-census.py`
  counts, per dumped resource, each element chapter 8 and 9 give a layout meaning; its
  `--self-test` plants one of each and a resource with none, and a conformance test runs it, so a
  clean corpus answer is read from an instrument shown to find what was planted (trap 13). It
  repairs one malformation — a bare `&` in a producer's data — and says so in the row.
- **A fetched sample stands beside the held corpora and is never in the repository.** The held
  corpora contain very few `/NeedsRendering` documents, too few to price a layout from; pdf.js's
  `xfa_*.link` files name 55 public forms, fetched for this reading only. They describe what a
  dynamic form contains, never how common one is.
- **The price is of page one, read from the processing model.** XFA 3.3 merges and runs
  `initialize` before laying out (pages 288, 388), so page one costs the merge, the scripts and the
  drawing whole, and saves only pagination past the first content area.

## Consequences

The amendment is the owner's (Q840). A later census re-runs the two commands `doc/todo/78` names;
the example's verdict column is the window's own test, so a change to `Template::dynamic` changes
both together.
