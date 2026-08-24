# 718 — A gibibyte that is never allocated

Eighth merge round of the block. Four branches, **no conflicts**, and the batch with the largest
single number this project has measured: **1 070 828 KB of peak memory to 22 608 KB**.

## The sequence, whole, on a quiet machine (load 2.90)

Both workers built first. `fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 ·
the fuzz check, exit 0 · `nextest` **2561 passed, 18 skipped** · conformance 182 + 5 + 1 + 1 ·
`cargo deny` all four ok · corpus **974 documents, 67 incomplete** · oracle **1945 pages — 983
agrees, 65 contradicted, 832 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18
no render** · `render-quorra` **933 agree, 22 differ, 2 refused** · both censuses ·
`fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. §5's binaries rebuilt and installed. Ledger
**445 implemented, 223 partial, 0 unreviewed**.

**All seven oracle verdicts are printed above, and that is a correction.** Several of this block's
briefings quoted four of them, and 715 pointed out that a baseline missing three counts is a
baseline a round cannot reproduce.

## 714 — the pump, and the clause that licensed each filter

**All five of §7.4's byte filters pump**, each on its own clause's arithmetic rather than on a
measurement: §7.4.2's "one byte of binary data for each pair" (1:2), §7.4.3's "5 ASCII characters for
every 4 bytes" read backwards (4:1), §7.4.5's "1 to 128 bytes" per length byte, §7.4.4.1's NOTE 2
"approaching 1365:1", and Flate's measured 1032:1. **The five absent are not byte filters at all** —
four image codecs, and `Crypt`, which §7.6 answers before any filter runs. That is a stronger answer
than a ratio, and it is why the table is closed rather than open-ended.

| twenty pages, one form apiece | before peak | after peak |
|---|---|---|
| control: single `/FlateDecode`, 2 GiB of zeros | 13 920 KB | 14 272 KB |
| `[/AHx /Fl]`, 4 174 537 B encoded | **1 070 828 KB** | **22 608 KB** |
| `[/AHx /Fl]`, 12 523 517 B encoded | **1 103 596 KB** | **55 016 KB** |

`display_list_digest` **byte-identical** over 958 corpus pages; `callgrind` +0.050% on a text page.

**Its central finding is that session 712's redirect — which this round's briefing relayed as the
lesson — was half wrong.** ADR 0586 predicted a chain pump would cost "kilobytes on every read".
Kilobytes of *memory*, yes; but the read is still the bomb's whole decode, and the 25 000× 712
measured was the distance between a refusal **remembered** and one **re-reached**. A window
re-reaches it. What makes that a statement rather than a regression is the control row: the same bomb
*unwrapped* already cost this tree seconds and 14 MB, so the window makes the wrapped case behave
like the unwrapped one. The memo line went back to `doc/todo/41`, **smaller**: the window's refusal is
the same variant under the same key, so what is owed is one fact travelling one hop.

**And one unification was proved wrong by a fuzzed corpus document.** Making the ASCII85 stage keep
its prefix on error breaks `PDFBOX-3148-2-fuzzed.pdf`, which states its **cross-reference stream**
that way with a bad byte eight in: refusing sends the parser to its header scan and the page is
found, while handing back eight bytes makes them a cross-reference *section* with almost every entry
missing and **the document loses its only page in silence**. *A prefix of a table is not a shorter
table.* Reverted, both halves pinned by one test, and the lesson is in trap 5.

Two more: `doc/todo/14` cited **§7.4.7** for its own witness, which is `JBIG2Decode` — the hex-then-
flate arrangement is **§7.4.1's EXAMPLE 3**, over "a stream, containing the marking instructions for
a page", a stronger citation because it is a *content* stream. And a pass that filled a link and
drained it in one visit stalled the chain at 8 KB a turn and **terminated early with a plausible
prefix**, visible only above the link size — which is why the agreement test spans 1 to 100 000 bytes.

## 715 — item 7 is paid, and its own defect nearly reappeared inside the fix

The 505 fills sharing a device pixel are drawn. **711's price was right down to the buffer**:
`scan::intersected`'s existing one, not item 5's rasteriser. What had to change is one sentence —
`intersected` declined wherever there was no clipping *region*, and §11.6.2 asks for the same buffer
**with no clip at all**, so two clauses now ask for it and either is sufficient.

**What 711 had not priced**: adding each portion's *rounded* level is a level or two out per shared
pixel, and a coverage rounded away is item 7's whole subject. So the summing is two passes — each
portion at its own area with ADR 0476's interior run kept, then only the pixels two footprints share
revisited and the total written in one addition.

The strongest number is the **confinement**: `raster_digest` moves **22 of 958 first pages, every one
named by the census beforehand and no other** — including one named that correctly did *not* move,
its fills landing where the quarter and the exact area agree. Trap 1 honoured: `issue8187.pdf` is a
**barcode**, and its bars now carry the levels their sub-pixel widths imply. Oracle: **not one
verdict moved**, 21 per-page lines shifting, none across a bound. Cross-backend gate unmoved at
933/22 with two lines moving toward the device. Cost −32.3% on the barcode, **+0.95%** on the one page
with 97 such fills — recorded rather than optimised.

Two pieces of instrument work worth keeping. **The `quoted` sweep earned its keep**, finding the one
figure the change made stale — and opening that note showed its ink ladder had **drifted 0.01–0.03
across rounds since session 383** with nothing pointing at it. And **an optimisation that looked free
was measured and removed**: restoring a guard that duplicates the callee's own first decline *cost*
13 000 instructions.

## 717 — a floor read off the widget list, and a `shall` in the other direction

`doc/todo/30`'s last item called Table 233 bit 19 *a genuine toolkit floor*. It is not, and the reason
generalises: **the block was read off the widget list.** `GtkDropDown` has no entry and
`GtkComboBoxText` is deprecated — both true and both beside the point, because the bit does not ask
for a widget. It asks for "an editable text box as well as a drop-down list", which is an entry
beside a menu button over a list box in one linked container. **The feature floor did not move**, and
that is ADR 0508's rule paying twice on one clause.

**The finding is behind the item**: the bit's second clause is a `shall` in the other direction — *if
clear, it shall include only a drop-down list* — and the host that broke it is the one ADR 0509 calls
ahead. `viewer-ui` read *has a text value* as *takes typed characters*, so **a person could type
`Purple` into a drop-down stating Red and Blue, and the file took it.**

And the fix went in the right order: the tier-2 host **could not choose an option at all** in either
of §12.7.5.4's controls, so refusing the keyboard first would have closed a clause violation **by
removing a capability**. The drop-down was built before the refusal was added. Twelfth consecutive
round needing no new message.

## 716 — the ranking is self-reinforcing, and a decision made twice

**A third rule for reading ADR 0567's ranking, and it is about the instrument rather than the
ledger**: measured with one instrument on both sides of 710's commit, the pair 710 read goes **17 →
21** and §12.5's total **221 → 225**, because 710 rewrote both rows in one voice. **The head family
can be the last round's own writing.** So the pair to take is the strongest one the previous round
*named and did not read*.

**Trap 13 caught a duplicated decision in the act.** Calibrating a renamed test by removing
`RClosedArrow` from `filled` — *the test passed*. `draw_ending` asked `filled` for three shapes and
the arrowhead arm asked its own equivalent condition three matches below. Provably the same value, so
no pixel was ever wrong; what the duplication cost was **the reach of a correction**, and the two
shapes outside `filled`'s reach were the two Errata Issue #515 is about. That erratum — a caret with
no strikeout, the first of `check`'s three blindnesses met since 710 named the third — adds "filled
with the annotation's interior colour, if any" to a Table 179 row that our code had **derived** and
never recorded, while **four places said "four" over five arms**.

And a hit standing at the *head* of the quotations sweep is explained by Issue #513: an editor's note
saying the ISO PDF's own row height obscures the end of a sentence, which is exactly why `doc/md/`
splits that cell mid-word. Our quotation is right, the instrument is short, and **`doc/md/` is
deliberately not patched**.

## Two rounds, one machine finding

**A ratio test can fail under load**, and 716 and 717 hit it independently — the same
`an_outline_resolves_against_the_page_tree_once`, at load 136 and 331. It is written as a *ratio*
precisely so a slow machine cannot fail it, and a ratio survives slowness but **not a scheduler
stalling one of its two phases**. `doc/todo/02` §2's loaded-machine rule is written about gates that
spawn reference renderers; this is one process, and the rule is more general than it is stated.

## Owed

- **The memo line back in `doc/todo/41`**, smaller: one fact travelling one hop.
- **709's two residues**, now `doc/todo/30`'s named next-UI-round work: `AccessibilityNode::lines`
  not crossing the ABI, and sorting the eleven queries each window does not reach into debts and
  non-debts.
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
