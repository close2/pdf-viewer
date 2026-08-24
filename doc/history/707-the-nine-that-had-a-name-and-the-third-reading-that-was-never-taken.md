# 707 — The nine that had a name, and the third reading that was never taken

An instruments round on the two residues the seven-hundredth session left when it closed trap 16.
Neither is a pixel; both are an instrument that knew something and did not say it.

## 1. The accessibility census counts no reports — it does now (ADR 0573)

700's sentence: *"a refused image is loud in the interpreter and silent in the census."* §14.8.3.3
derives an element's rectangle from what its marked content **drew**, so a refused image drew
nothing, nine of `issue5481.pdf`'s structure elements lost the only place they had, and the census
counted both ends of the move with no cause beside it.

Three layers, one field each. The interpreter keeps a monotonic count of `Interpreter::note` calls;
`open_marking` records it and `close_marking` compares, so a note raised between a sequence's `BDC`
and its `EMC` was raised inside it — `MarkedSpan::enclosed_a_refusal`. `AccessibilityNode::
enclosed_a_refusal` unions it over the element's own sequences and its descendants', which is the
same enclosure `drawn` already takes, and crosses `viewer-confined`'s pipe. The census prints how
many elements enclose a refusal and — per page, with the page's own `Query::Reports` sentence — how
many have **both** no place and a refusal inside them.

**Trap 11 decided the condition, and the reflex one was not taken.** *Placeless on a page that
reported* fires on every placeless element of a page whose report is about something else on the
sheet, which is a condition the clause does not state; §14.8.3.3 states *enclosure*. And the class
claims enclosure rather than cause — `issue8702.pdf`'s two elements enclose §7.8.3's undefined
`/XObject` and still have a place, because they drew text as well, and that is the un-ignored test
that pins the whole chain without needing the worker.

**Trap 13's requirement met**: the instrument was run against the defect. One build, one binary,
one variable — `PDF_SANDBOX_WORKER` at a path that does not exist — and the census names the nine
exactly, three pages of `issue5481.pdf` at three apiece, under *an image (Im0: starting the sandbox
worker failed) was not drawn*. The +9 accounts for the whole of the −9 in *placed by their own
marks*. Nothing is ratcheted: ADR 0323's rule, and these counts are one round old.

The other two instruments were asked the same question rather than assumed to share the gap.
`text_extraction` already counts reports and prints them; `selection_census` does not, and its
shape turns a refusal into a *missed drag* rather than a smaller number, which is the loud
direction — what is owed there is narrower and is recorded rather than built.

## 2. Six pages judged on two references — read, and the rule left alone (ADRs 0574, 0575)

**First the instrument, because the six could not be read at all.** Four of them printed `PNG error
… unexpected end of file` — the harness's sentence over a log holding the renderer's. Two
mechanisms, both trap 3 one step further in than the invocation it is usually about: `mutool draw`
creates its `-o` file before deciding it cannot draw the page, so a zero-byte PNG passed
`exists()`, reached the decoder, and became a `HarnessError::Png` — the one failure `cache`
deliberately refuses to remember, so those pages re-ran `mutool` on every run inside a cache at a
99.8% hit rate. And `gs` writes `Error: /undefined in obj` to **stdout**, which the harness sent to
`/dev/null`, while `Reference::version` has carried the comment saying which stream `gs` speaks on
since it was written. Both fixed; `last_line` became `diagnosis`, first and last, because all three
renderers end with the consequence and begin with the cause.

**Then the question, which is a specification question**: is a consensus of two the same evidence
as a consensus of three? Same kind, one factor less — ADR 0005's inference is about a **pair**, so a
third multiplies the improbability rather than creating it. The arithmetic differs in two
directions that pull against each other and neither is measurable here, because the third reading
does not exist: the absent reference is absent because it cannot produce a picture of the document
at all.

**What the six are actually about is ADR 0541's precondition.** Five lost their third reading
because the *document* is outside what ISO 32000-2 describes — a §7.5.4 subsection header with an
object number of 2³², a file with no §7.5.2 header at all, a JP2 whose first box is not §7.4.9's
signature box, two page trees no repair recovers — so part of what the surviving pair agrees about
is how to **repair**, which no clause states. The sixth is a reference being wrong: `pr6531_2.pdf`'s
empty password authenticates against its `/O` under §7.6.4.4.11's Algorithm 12 over §7.6.4.3.4's
Algorithm 2.B, run by hand on the file's own bytes in twenty lines that read none of our code —
`poppler`, `gs` and this tree act on it, `mupdf` 1.28 accepts only the user password `asdfasdf`,
and `encryption.rs::an_empty_password_may_be_the_owner_password` has asserted it since it was
written. And `bug_jpx.pdf`'s poppler does not *refuse*: OpenJPEG dies on an assertion, and a
refusal is a reading where a crash is not.

**No verdict rule changed and no page moved.** `JUDGED_WITHOUT_A_THIRD_READING` carries the six
with the reading of each, and the gate now names any page in that population on no list — printed
rather than ratcheted, because membership depends on the machine's renderers as well as on the
corpus.

## What did not move

**No pixel.** Every one of `doc/todo/02` §2's lines ran and every figure is identical to the merge
round's, including all fourteen accessibility floors and all four defect ceilings. The oracle's
seven verdict counts are identical across four runs of this round — one before any edit, three
after — and the two new census counts read 2 and 0 with the worker present.

§4's `quotations`, `pointers`, `overtaken` and `quoted` report only their standing hits; the new
page-list note appears on none of them, which is the rule that a note cites its own ADR being kept.

§5's binaries were **not** rebuilt: not a fifth round, and the round took no measurement of the
launch path, a frame or a page turn.

## The machine, and one thing to know about the shared cache

`PDFREF_CACHE` pointed at a private directory hardlinked from the shared one at
`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache` — `rsync --link-dest --exclude='*.err'`, 0.6 s
and no extra disk — because **a remembered failure carries the wording of the run that stored it**
and this round changed that wording. The cache's key is the format tag, the renderer's version, the
document's digest, the page, the resolution and the invocation, and the harness's own prose is in
none of it; bumping `FORMAT` to flush 92 sentences would have invalidated 28 648 stored renders.
The shared cache's `.err` entries were **not** cleared, so a neighbouring round's oracle line for
those pages carries the previous wording until the entry is rewritten. It is a message rather than
a verdict, and trap 10a now says so.

Load average 3.4 at the start and the gates ran with nothing beside them. The oracle: 41.5 s at a
100.0% hit rate on the final run.
