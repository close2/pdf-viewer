# ADR 0805 — A pool with no next name says so on the line: the contradicted ranking names each page's holder

Status: accepted. Session 873.
Clauses: ISO 32000-2 §10.7.4 (read; nothing implemented), §10.4.2.5 and §7.4.7 (re-read on the
pool's head, nothing changed).
Code: `crates/pdf-model/tests/oracle.rs` (`CONTRADICTED_GROUPS`, `held_by`,
`name_the_pages_no_group_holds`, `rank_the_contradicted_by_the_bound`, `check_the_ratchets`).
Tests: `crates/pdf-model/tests/oracle.rs::every_contradicted_group_is_in_the_table`.
Documents: `doc/todo/00`'s closing section, `doc/oracle-and-corpus.md` §3b, §10.7.4's ledger row.

## Context

The round was handed the oracle's contradicted list with one instruction: *take the worst-ranked
contradicted page whose cause is not already diagnosed and held by name*. The gate prints two
rankings of that pool and neither says, on the row, whether the page is held or by which group.
The answer was in the file — twenty-three `const CONTRADICTED_*` declarations spread over eleven
thousand lines of `oracle.rs`, twelve of them non-empty — and reconstructing it for sixty pages by
hand was the round's first hour. It came out **none**: every one of the sixty is held by name, the
ratchet is green, and the one instrument outside the gate whose question this is agrees:
`unpriced` reads 89 failing bounds over 60 pages, finds 89 of them named by the note that holds the
page, and finds no contradicted page outside every page-list note. `quoted` and `overtaken` were
run over the same log and name ten and twelve `CONTRADICTED_*` notes respectively; both say on
their last line that a hit is a reading list and not a verdict, and neither asks whether a page is
held — the first sitting of this round wrote them down as *one figure* and *no page*, and the
second, re-running them, found that is not what they print.

That is `CLAUDE.md`'s own rule about counted facts — *a fact that can be counted is not written
down; what is written down is the command that counts it* — arriving on a list rather than a
number, and it is ADR 0772's finding one instrument over: a population handed on in prose, or in
this case not handed on at all, is one the next round reconstructs.

## Decision

1. **One table, two readers.** `CONTRADICTED_GROUPS` names every gated group beside its page list.
   `check_the_ratchets` chains it into the single ratchet the pool has always been held by — the
   groups ratchet *together* because which group a page belongs to is a hypothesis, and that does
   not change. `rank_the_contradicted_by_the_bound` prints `held by <group>` beside each of its
   rows, and `name_the_pages_no_group_holds` prints under the list how many of the whole pool no
   group holds, naming them. `CONTRADICTED_ON_A_PAGE_WE_REPORT` stays out of the table — its pages
   are outside the ratchet by construction and held by their own staleness check — and `held_by`
   looks there second so that the ranking, which does not filter on `complete`, still names it.

2. **The table is checked against the file it lives in.** A group declared and not tabled would
   leave the ratchet one group short exactly as the old hand-written chain would have, so
   `every_contradicted_group_is_in_the_table` reads `oracle.rs`'s own `const CONTRADICTED_*: [&str;`
   declarations — the rule `tools/conformance`'s `overtaken` already reads the file by, which is
   what keeps the table itself out of its own count — and holds both directions. It is not
   `#[ignore]`d: it reads no corpus.

3. **The line under the list says what a fully held pool means for the next round**, because the
   round that wrote it needed the sentence: the next page to take is the highest row whose note
   names a departure of *ours* rather than a reference's.

## What that row is, and why it was declined

The highest such row is `issue4436r.pdf` at 1.16× on the differing fraction,
`CONTRADICTED_SUBPIXEL_IMAGE`. Its note and §10.7.4's ledger row already state the shape: a 1×1
image mask whose device region is y `[25, 25.48)`, row 25's centre at 25.5 outside it, so the
clause's image paragraph — "only those pixels whose centres lie within the region shall be
painted" — paints nothing; the four references paint the whole row (the shape rule applied to an
image); ours paints 0.502 of it, departure (1), the anti-aliasing rasteriser's coverage.

The clause is a `shall` and it is ours to carry out, so the round read it against the tree's
standing decision before deciding that it is not this round's fix:

- **The rule is one an aliased scan converter states exactly and an anti-aliasing one cannot.**
  tiny-skia's non-anti-aliased converter samples at pixel centres; its anti-aliased one paints
  coverage. Carrying the paragraph out is `anti_alias = false` on the image's fill and nothing
  else — one line — which is exactly why it has to be argued rather than done.
- **Departure (1) is a priced decision, not an omission.** `doc/todo/11` §5 prices what a mark
  drawn at its analytic coverage costs (the seam two abutting half-covered edges leave) against
  what removing it costs, and declines both cures; the §10.7.4 row records the departure for every
  mark. An image edge drawn aliased beside anti-aliased fills, strokes and glyphs is a change to
  that decision, and a decision changed for one paragraph's witness is not a reading of the
  paragraph.
- **It moves the page's verdict nowhere.** The row's one failing bound is the differing fraction,
  a threshold count: row 25 differs from the references' whole row at 0.502 and at 0.000 alike.
  The page's own note converts that to the digit — 1.3575 points — and the number does not depend
  on which departure we take.
- **And the three backends would owe it together.** `render-quorra` draws an image through a
  device whose edge rule is its own, and trap 2's rule is that a decision one backend can take
  alone is a decision neither has taken.

So the departure is left as it is, said so on the ledger row, and the page stays where it was. The
one thing this changes for the round after is where it is sent: `doc/todo/00`'s closing section
says the contradicted list is not where the next defect is.

## What was verified on the way

The pool's head was opened rather than trusted, with every reference re-rendered by today's
binaries — the gate's cache hit rate in this fresh worktree was 0.1%, which is the control that no
verdict below rests on a stale panel (trap 10a). The three groups at the head hold:

- `bitmap-halftone-composite.pdf` (33.47× on the worst tile): ours and `hayro` draw the `bitmap-*`
  family's one drawing; `mupdf` and `ghostscript` draw `jbig2dec`'s garbled halftone region and
  `poppler` a stray bar of its own. ADR 0381's self-comparison stands.
- `function_based_shading_cmyk.pdf` and `postscript_type4_many_outputs.pdf` (29.19×, 23.88× on the
  differing fraction): ours and `poppler` saturated, the three on a SWOP characterisation
  desaturated together. ADR 0773's removal stands.
- `xobject-image.pdf` (127.75× on the mean, incomplete): a file that contradicts itself, our choice
  reported beside the picture.

## Consequences

- The oracle's by-the-bound ranking answers the question a round is sent to it with, on the row.
- A group added to `oracle.rs` must be added to `CONTRADICTED_GROUPS`, and the build says so.
- No pixel moved. The oracle's counts before and after are the same run's: 60 contradicted, 980
  agreeing, and the full `doc/todo/02` §2 sequence was run because a change in `pdf-model` runs
  everything.
