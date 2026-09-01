# 868 — A CPU-second is not a unit of work, and the transform gate

2026-09-01. Argued in [ADR 0801](../adr/0801-a-cpu-second-is-not-a-unit-of-work-and-a-font-cache-per-split.md).
The second implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`, which started by merging `main` (rounds 864 and 865's commits)
cleanly.

Touched: `crates/pdf-transform/src/render.rs`, `src/images.rs`, `src/attachments.rs`,
`src/lib.rs`, `src/bin/pdf-transform.rs`, `tests/verbs.rs`, `tests/gate.rs` (new),
`tests/support/mod.rs` (new); `doc/todo/02-every-round.md` (the gate line and a map row),
`tools/state.sh` (a `transform` section); `doc/performance.md`, `doc/state-of-play.md`,
`doc/todo/README.md`, `doc/todo/57-the-transform-suite.md`; `doc/adr/0801-…` (new), this file.

## 1. The CPU gap, measured rather than argued

Round 867 recorded 2.4× the CPU for the same pages at 24 threads and named two suspects. The
thread curve had a bump at two threads — no faster than one, twice the CPU — that neither suspect
could make; a shared `FontCache` removed the bump and left the 24-thread figure where it was; and
24 single-threaded processes sharing nothing cost the same 20 s of CPU. So the gap is the
machine's — twelve cores, two hardware threads each, an all-core clock below the boost — and the
defect that was ours was `map_init` making a font cache per rayon *split*. ADR 0801 has the
tables; `src/render.rs`'s module comment carries the one that justifies the change.

## 2. The gate

`cargo test --profile gates -p pdf-transform --test gate -- --ignored --nocapture`, on §2's
sequence: `render` of 200 pages through the built program against a floor of 40 pages/s (this
round measured about five times that on 24 threads), one page held byte for byte to the oracle,
and the `images` and `attachments` inventories held to walks of the document written in the test.
`tools/state.sh transform` prints its lines.

## 3. Inline images, `--native`, file attachment annotations

All three landed, each with a test on a document that has one — `issue11124.pdf`'s 48-byte
inline image with a false `EI` inside it, the standard's own JPEGs and a corpus CCITT inline
image for `--native`, and the standard's own six annotation-borne files, which `attachments` had
listed as none. A census run with the tool over the pdf.js corpus for the fixtures found twelve
documents with inline images and one fuzzed document whose inline image the verb now warns about
rather than misses.

## 4. Gates

The six core lines, `cargo test -p conformance` and the new gate were run in this worktree at
`-j 12`; the results are in the round's report and not here (`doc/todo/02` §2's rule). No
first-row crate changed, so the corpus gates were not owed.

## 5. What the next transform round does first

`doc/todo/57`'s order, as rewritten this round: the two `images` and `render` flags that need no
writer, then everything that waits on the owner's answers to RFC 0002 §13.
