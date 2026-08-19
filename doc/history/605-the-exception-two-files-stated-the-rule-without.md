# 605 — The exception two files stated the rule without

The closing round of sessions 576–605. Its block summary is in `doc/history.md`, which is the one
place a round may add to besides its own file — **and neither of the two documents that state that
prohibition stated its exception.** `doc/HANDOVER.md` said a round does not append to
`doc/history.md` "which holds sessions 5 to 445 and is closed"; `doc/history/README.md` said the same
from the other side. The exception lived in `doc/history.md`'s own preamble, in a file the handover
tells a round it needs "only when it is asking *when*" — so a closing round could learn its one extra
duty only by opening the file it was not sent to. Both now carry it.

Date: 2026-08-19.
ADR: [0440](../adr/0440-the-half-of-an-erratum-the-marker-could-not-see.md), for the one decision
this round took.

Touched: `tools/spec-errata/src/applied.rs`, `doc/HANDOVER.md`, `doc/history/README.md`,
`doc/history.md` (the block summary, and six words of the preamble), `doc/todo/README.md`,
`doc/adr/0440-…`, this file.

## The block summary

Thirty rounds, and what they have in common is a sentence about this tree's own prose rather than
about the standard or the corpus: **the sentence explaining a refusal was wrong more often than the
refusal was**, and the question that found it was *whose is this?* — the file's, the process's, the
library's, the device's, the clause's, or ours. Twenty-four of the twenty-nine working rounds
corrected a claim this tree had made about itself, and six of them were refusals whose stated owner
was somebody else. The summary is in `doc/history.md` under its own heading and is not repeated here.

The judgement the summary rests on is a count of rounds, so it names the five that did **not** find
such a claim — 584, 592, 596, 599, 604 — which is what makes it checkable rather than a flourish.

## The four sweeps, and the one that paid

`pointers`, `retired`, `quotations` and `spec-errata applied`, as the closing round is told to run
them.

**`pointers`, `retired` and `quotations` found nothing this block broke.** All 120 absent path
pointers resolve to ADRs citing files a later round deleted — which ADR 0232 §2 says explicitly is
not to be edited — to quorra's own tree, or to `doc/errata.md`, which is generated and gitignored.
The `doc/todo/54` citations, five of them, are all in ADRs 0412, 0413 and 0435, which is correct: the
file was deleted in the six-hundredth and its index line went with it. `quotations`' 31 document-side
divergences include this block's two newest, and both are right — session 585's record and ADR 0420
quote Errata Collection 3's *replacement* for §8.5.3.2, which by construction is not in `doc/md/`.

**`applied` paid, on the instrument rather than on the tree.** Its read-first list held 22 hits and
**every one under `crates/` was correct writing**: `structure.rs` saying "Errata Collection 3
replaced it", `type3.rs` saying an erratum "replaces" a sentence, `write.rs` saying it "has edited
that sentence", three places round §12.5.2 saying #287 "sharpens" the `/BS` precedence. The marker
list carried the verbs for what an erratum *removes* — `struck`, `strikes`, `retired`, `no longer` —
and none for what it *puts there*. An erratum has two halves and the list had one.

`replace` and `sharpen` join it; **`makes it` does not**, and that is the decision rather than the
addition. It would have marked one more hit and it would also have marked this sweep's founding
defect, whose note opens "Errata Collection 3 makes it enclosure (Issue #437)" and quotes the struck
sentence three sentences later — ADR 0426's own argument about `said` and `this row`, with a second
instance behind it now. The line is a test rather than a sentence:
`a_phrase_that_only_says_an_erratum_changed_something_does_not_mark`.

Read-first **22 → 10**. The ten are five `crates/` sites whose verbs are rarer still, two ledger
rows where the quoted phrase survives in the clause's *other* sentence, and three dated ADR records.
It is not empty and is not meant to be: the mark is a sort order and every hit is still printed,
which is what makes widening it safe.

## The claims the block changed, checked

`doc/todo/README.md`'s line for item 30 said "what is left is one tail — Qt measuring its controls on
the far side of the `cxx` bridge", which the six-hundred-and-first closed. The *file* was current and
named the item that arrived as that one left — `viewer-gtk` not obeying Table 234's `/TI`, on a GTK
4.12 binding floor — and the index line was one round behind. Corrected. **This is the third time
that index has cost a round something** (the five-hundred-and-eighty-eighth spent an hour on it, the
five-hundred-and-ninetieth found two closed items in one line), and the paragraph above the index
already says why: a summary that restates a file's header is a second copy to keep in sync.

`doc/history.md`'s preamble enumerated the block summaries — "there are two, for 315–334 and for
416–445" — and had been one block behind since the four-hundred-and-eighty-fourth appended the third.
It is a count in prose in the one file that is otherwise a record, which is exactly what `CLAUDE.md`
forbids; the sentence now says a closing round writes one and lets the headings say how many there
are.

`doc/todo/02-every-round.md` was read whole against what the block did and states nothing that
stopped being true: §2's sequence carries both censuses the block added, its `cmin` paragraph carries
the five-hundred-and-ninety-third's correction to its own prediction, and §5's cadence is the one
`tools/round.sh` reports against.

## Gates

`doc/todo/02` §2's full sequence, run whole, then run whole again after the last edit.

**The first run failed on its own first line** and the failure was mine: `cargo fmt --all --check`
disagreed with a `format!` I had wrapped by hand in the new test. This is the second consecutive
block in which the sequence's first line caught the round that ran it (the five-hundred-and-ninety-
ninth records the first), and it is the argument for the line being first.

## §5's binaries

Rebuilt and installed — `tools/round.sh` had flagged them older than `HEAD` for four rounds and the
six-hundred-and-third and -fourth each correctly declined, being neither a fifth round nor a
measurement. A closing round owes it, and what a person can now run is this commit.

## What the next round should take first

The block summary's own last paragraph names it: **the two gates that count pages could not see this
block's work.** The oracle moved by one page and quorra's line is character-identical across thirty
rounds that closed road D, inverted a transparency refusal, changed the processor's compositing
pipeline and rewrote how a tiling pattern is drawn. `doc/todo/03` §16 has the question that follows
— what a verdict means over the crawl, where nothing supplies an expected value — and the
six-hundred-and-third's chunk is 2000 of 65 944 documents ranked, at about eleven minutes per 2000.
That is the instrument the next block needs, and it is the one that found the last defect no ratchet
in this tree could see.
