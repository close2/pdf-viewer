# 905 — The fifth writer shown to another reader, and a refusal that said the wrong thing

Date: 2026-09-03.
ADR: [0852](../adr/0852-the-fifth-writer-shown-to-another-reader-and-a-refusal-that-said-the-file-defeated-us.md).
Touched: `crates/pdf-transform/tests/foreign_corpus.rs`, `crates/pdf-transform/src/lib.rs`,
`crates/pdf-transform/src/optimize.rs`, `crates/pdf-transform/tests/optimize.rs`,
`crates/pdf-transform/tests/optimize_corpus.rs`, `doc/conformance/ledger.toml`
(§7.5.5, §7.5.7, §7.5.8, §7.7.2), `doc/todo/02-every-round.md`,
`doc/todo/57-the-transform-suite.md`, ADR 0852, this file; and the merge commit before this
round's own. **No pixel moves** — nothing here is on a path that draws.

## The merge

**`round-867` (9699ab08) is on `main` as `c84b122c`**, `--no-ff`, on top of round 903. It is round
900: RFC 0002's `optimize`, the fifth and last of the suite's writing verbs, and with it the
producer half of §7.5.7 that session 886 recorded as owed the day it landed the serializer. Four
lossless passes each derived from a sentence — §7.5.5's reachability, §7.5.7's object streams with
NOTE 4's two ceilings measured rather than borrowed, §7.5.8's cross-reference stream that they
force, and §7.4.4.1's `FlateDecode` kept only where it is smaller — for 13.62% of the pdf.js
sample. `r867` is **not** closed; §"what is left" below says what it still owes.

**Git found no conflict at all**, in any of the three files where one was expected. The branch had
merged `main` once at `d72241d1` and `main` has moved only in round 903 since.
`doc/conformance/ledger.toml` was checked **row by row** rather than by reading the diff — the file
is one TOML table per clause, so the check that matters is per row and not per line: `main`'s three
changed rows (§10.7.5, §8.4.1, §8.4.3.2) and the branch's seven (§7.3.7, §7.4.4.1, §7.5.5, §7.5.7,
§7.5.8, §7.5.8.3, §7.7.2) are each identical to their own side in the merged file, no third row
moved, and the count is 876 on all four versions. `doc/todo/02-every-round.md` took the sixth
transform walk beside round 899's `pdf-vfs` row, and `main` has written nothing in `doc/todo/57`
since the branch left.

**The whole `doc/todo/02` §2 sequence then ran on the merged `main` — all twenty-six lines, every
one exit 0** — on a quiet machine, the walking lines under `tools/bounded.sh` (`--tree 8` for a
build, `--data 12 --tree 12` for a walk) one at a time, nothing beside them. The figures are in
§"the gates" below, where they are this round's *final* run's rather than the merge's, on
`doc/todo/02` §2's own rule that a number is current only for the round that ran the gate last.
The merge's own run agreed with it everywhere except in the places a wall-clock budget moves.

## `optimize` joins the foreign readback

Session 900 named this as the smallest thing left and said why it is not the least important: the
other four writers carry a producer's objects into a new file with their numbering, their filters
and their object-stream membership mostly as they were, and `optimize` is the one whose *point* is
that the bytes are different. ADR 0843 §2 is the standing evidence for how badly this tree's own
instruments see that — a recompressed image whose `/DecodeParms` was rebuilt in the source's
numbering left every raster bit-identical while the file was corrupt.

The lane writes what `pdf-transform optimize` writes for a person, object streams and recompression
included, because those are the two passes another reader may decline and a walk that turned them
off to be safe would measure a configuration nobody runs.

**It found no defect, and the interesting rows are the ones about §14.7 and about object streams.**
`optimize` agrees with the source's parent tree on more documents than any other verb (76, against
75, 74, 74 and 10) and states **no §14.7 fault at all**, where `split` and `merge` state two each
and `pages` one — which the clauses predict rather than surprise: those faults are elements the
*source's* parent tree names and whose hierarchy reaches nothing of them, and a verb that carries
the whole document has no piece to leave them out of. And **no installed reader declined an object
stream we wrote**, which was the risk the lane was added for, §7.5.7 and §7.5.8 being 1.5
constructs and this the first thing this project writes that uses them. That claim is about qpdf
12.4.1, poppler 26.08.0 and mupdf 1.28.0 and about nothing else.

**qpdf's verdict is now read as a fall of one step or of two.** ADR 0839 compared the derived
file's verdict with the source's — right, and the only honest comparison — but collapsed both
possible falls into "no worse", so a file qpdf had nothing to say about becoming one it *warns*
about was unsayable. That is the smaller signal `optimize` is likeliest to produce. `qpdf gained a
warning` is its own lane now, and it is empty for all five writers, which is what makes asserting
on it honest rather than aspirational (trap 11).

**And the direction, measured over the whole corpus rather than the walk's stride-8 sample**, since
a claim of absence is refuted by one witness and the sample is a bound on wall clock: 955 of
`doc/pdf.js`'s 974 documents are rewritten and **not one file's qpdf verdict got worse**. 243 go
from warnings to clean and 7 from *errors* to clean, 22 stay at warnings and 6 at errors. Evidence
about the reading, never a target (principle 5).

## A refusal that said the file defeated us

Found in the exit statuses that whole-corpus scan produced, and it is a defect against this
project's own document rather than against the standard. RFC 0002 §4.4: "2 means the *file*
defeated us, 4 means *we* declined, and a caller scripting the suite can tell them apart without
parsing stderr."

Session 900 refuses a document whose `/Root` or `/Pages` this tree only reaches through §C.4's
recovery, on the ground that Table 15 and Table 29 each make theirs "( Required; shall be an
indirect reference )" and a rewrite of a reconstruction would state a structure no producer wrote.
Its own message says *refused by name*; its own history file says "which is trap 5". The code
returned `Refusal::Assembly`, whose status is **2**. Nothing about those files defeated anybody —
this tree opens them, pages them and draws them.

`Refusal::Reconstructed` is that classification, carrying exit 4. Nine corpus documents change
status, four on Table 15's `/Root` and five on Table 29's `/Pages`; the ten refusals that are not
this — nine for a password, one for an encryption this tree does not open — correctly stay at 2.
Run against the condition in both directions before it was believed, on real documents, which is
trap 13.

**The shape is worth more than the instance.** The refusal's message, the ADR that introduced it
and the round's own record all said one thing and the code said another, and no gate compared them,
because the exit status is the one thing none of the five transform walks reads. It is
`doc/habits.md`'s decaying claim wearing a CLI's hat. `tests/optimize_corpus.rs`'s census prints
each refusal's status beside its message now, so the corpus says which side of §4.4's line every
refused document falls on.

## The gates

**The whole `doc/todo/02` §2 sequence ran three times this round — on the merged `main`, on the
finished working tree, and once more on the *committed* tree — all twenty-six lines, every one
exit 0 every time.** The figures below are the last of the three, because that is the tree that is
on `main` and `doc/todo/02` §2's rule is that a number belongs to the round that ran the gate
last. Verbatim where it matters:
`Summary [69.181s] 3186 tests run: 3186 passed (1 slow), 27 skipped`; corpus **974 documents in
11.7s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64 incomplete, 0 slow**; oracle
**1945 pages in 48.1s (1841 we call complete, 104 incomplete)** with
`our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; text extraction
**11 094/11 131 matched words in bounds (99.67%), 493 of 503 documents fully in**; selection census
**1000/1011 words (98.91%) over 453 documents**; accessibility census green over **102 853 elements
reached, 57 116 a caret can move through**; dates **1514 of 1545 (97.99%)**; XMP **318 of 319
streams read**; quorra **958 pages compared: 929 agree, 22 differ, 7 refused, 16 not comparable**;
fixed documents **69 checked, 0 absent, 69 rows**; the transform gate **200.0 pages/s over a floor
of 40**; `optimize`'s walk **974 documents in 30.5s** with every one of its property counters at
zero; the foreign readback **203 of 974 documents in 116.8s** across five writers, the new lane
**202 written, qpdf held 202, qpdf gained a warning 0, poppler identical 201, mupdf identical 198,
§14.7 shapes agreed 76, §14.7 faults 0, drew differently 0, 1 refused by name**; conformance **875
subclauses, 13 866 citations, 1241 quotations verbatim**.

**Every verdict is identical across the three runs and four numbers are not**, which is worth
recording because it is the shape a round mistakes for a regression. The oracle took 32.1s, 43.1s
and 48.1s; quorra 29.8s, 145.8s and 37.4s; the transform gate reported 189.1, 154.9 and 200.0
pages/s against a floor of 40. Those are wall clocks on a machine running parallel rounds, which is
`doc/todo/02` §2's own sentence about a gate that spawns another program — the number measures two
programs and a loaded machine is a silent third. The fourth is the readback's own: `optimize`'s
*poppler identical* was 201, 200 and 201, and ADR 0852's table says why that row counts a page
rather than measuring one — a foreign reader that outruns the walk's 20-second budget on the
**source** takes that document out of its own comparison. The rows that cannot move that way — the
faults, the differences, the warnings gained — are 0 in all three.

**§5 ran, this being a fifth round, and it ran twice for a reason worth keeping**: the first
install was taken from the merged tree, and this round's own commit then changed
`crates/pdf-transform/src/lib.rs`, so what a person could have picked up in between was a binary
without `Refusal::Reconstructed` in it — a stale binary is a measurement of the past, and here it
would have been a measurement of the defect. The eight binaries and `libviewer_ffi.so` were rebuilt
with `--release` in one invocation from the committed tree and installed into the project's own
`target/`; `tools/state.sh binaries` shows all nine newer than `HEAD`.

## What is left

`doc/todo/57` is the order and `r867` stays open for it: **`split --at-bookmarks`** (which wants
`pdf_model::retrieval::sections`, that exists, and an outline subset for the piece, which does
not), the aligned rotated comparison ADR 0831 §1 priced, a per-input password for `merge`, the
confinement tranche, and the RFC 0003 hand-off. RFC 0002 §13's second question — a DCT encoder —
still gates two features at once, `optimize --images` and JPEG output from `render`.

What the readback still does not cover is unchanged and is listed in `doc/todo/57` §5: it is a
sample, it draws page 1 only, it skips a document that needs a password, and it says nothing about
the outline, the name trees or the form.
