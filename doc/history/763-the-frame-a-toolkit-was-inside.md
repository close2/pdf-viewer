# 763 — The frame a toolkit was inside

Seventeenth merge round of the block, and the first that was **deliberately serialised**: 759 ran
alone on a quiet machine because the measurement it owed could not be taken beside three neighbours.
It found a 44 ms regression that four rounds' worth of load had hidden, which is the whole argument
for paying that cost once.

## The sequence, whole, on a quiet machine (load 0.80)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2684 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 61 contradicted, 836 ambiguous, 3 our
geometry, 2 reference geometry, 42 not comparable, 18 no render** · `render-quorra` **933 agree, 22
differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445
implemented, 223 partial, 0 unreviewed**.

**One thing had to be repaired before §5 would build, and it is 759's own hazard arriving one round
late.** See the last section.

## 759 — the regression the load was hiding

754 measured this A/B at load 25–40 and got *before 9–100 ms, after 62–448 ms* — both arms drifting
an order of magnitude — and **refused to average over it**. Re-taken at load 1.5–2.8, twenty
alternating pairs per host, both stamps inside one process so the column is not the machine's:

| `opened` → first frame, ms | mean | range |
|---|---|---|
| `viewer-gtk` before | **9.5** | 8–12 |
| `viewer-gtk` after | **53.4** | 49–66 |
| `viewer-qt` before | 10.6 | 9–13 |
| `viewer-qt` after | 7.9 | 6–11 |

**The GTK ranges do not overlap at all** — every after-run slower than every before-run. A real 44 ms
regression, about 40% of a 110 ms launch, against the 2.1 ms ADR 0668 §7 predicted structurally.

**And the thread spawn was never the cost.** The trace says so in one line: *rasterised in 3.25 ms,
waited 61.53 ms*. `open_document` runs inside the first size allocation, so the moment the pump
returns GTK begins **its own** first frame — GSK's renderer bring-up, which under Xvfb's software
Vulkan holds the main loop ~58 ms — and `Drawing::POLL`'s one-shot cannot be dispatched until it comes
out. Control: `GSK_RENDERER=cairo` drops the same wait to 11.3–12.5 ms. The four words 0668 §7 could
not test were *"with the loop otherwise idle"*. **Trap 21.**

Two findings the briefing did not anticipate. **`viewer-qt` showed no regression, and that is what
located the fault** — two hosts on one arrangement are a differential instrument. And **Qt's *faster*
arm was the worse picture**: its first frame carried exactly half the bytes, one of the two pages
Table 29 shows. Trap 1, in the launch stamp itself.

Fixed by `Drawing::settle(budget)` — a `recv_timeout` in front of `collect`, used **only while the
host has put no frame on the screen**, so ADR 0657's refusal of an automatic deadline stands with a
test. The budget is the launch's rather than the call's and is accounted as time *actually blocked*,
so a thousand-page document's open does not eat it. GTK back to **9.9 ms** (before-754 was 9.8), Qt
11.7 with both pages restored.

## 762 — the plan that cost as much as the drawing

The batch's other measurement round, and it took the shape 757 sharpened: **a composition decays
faster than the total it adds up to.** Page 101's *total* has been re-taken seven times, most
recently four rounds ago; its **breakdown had not moved in ~600 rounds**. Two of the three items it
names are gone. Third in the current profile was a function **no document in this tree had ever
named** — 448.3 M inclusive, 8.28% of the page — walking every fill's, clip's and mask's path to
name rows a cut may not fall on, at **76 991 calls per render**, on an operation that only ever
*sets* a row.

**The 9% understates it, and that is the generalisable part**: the prologue is **serial** and the
4.9 G beneath it is not. Eleven strips, the slowest holding 10.6% of the cost — so the drawing
contributed ~24.7 M to the critical path and **the planner was 24.7 M of it. On this page the plan
cost as much as the drawing.** Now a habit: *on a parallel path a share of the total is not a share
of the critical path; callgrind is honest about work and silent about latency.*

| | before | after | |
|---|---:|---:|---|
| ISO 32000-2 p. 101 ×20 | 5 422 M | 5 039 M | **−7.08%** |
| `tracemonkey.pdf` p. 1 | 2 370 M | 2 232 M | −5.81% |
| ISO 32000-2 p. 6 | 3 642 M | 3 525 M | −3.19% |

Every page falls, none rises; `segments` 448 M → 57 M, the whole prologue 494 M → 102 M.

**Trap 13 in its purest form, and it named why**: a byte-identical raster proves **nothing** here,
because the strips are exact by construction and a planner forbidding too few cuts draws the same
picture. Calibration had to be a row-for-row differential — **0 disagreements over 3412 page-scales
and ~18 000 crawled documents**, after three plants were caught with 1833, 928 and 1176 disagreeing
cases.

## 760 — a row that denied what an erratum permits

The errata rule's fifth use. **Its finding is the shape no sweep here can catch**: §9.6.4's row said
an inline image in a Type 3 glyph description "draws nothing yet and reports, which is §8.9.7's gap
rather than this one's: 10 corpus documents are in that position" — since the tenth session.
`pdf_model::inline_image` landed in the **eleventh**. Measured: it draws, placed by the description's
own `cm`, silently. **All three claims false, under `implemented`, about a sibling row's status in
another clause family** — which is precisely the relation none of the sixteen sweeps reads.
Issue #111's inserted NOTE 2 ("can use any PDF operator from any operator category") is what made the
denial checkable.

Two things about the instrument. **The live head has not moved in four uses**, because the full
ranking out-ranks it every time the fourth step runs — a property, not stagnation. And a **seventh
blindness, which is the gate's rather than an erratum's**: an erratum's *added* text cannot be a
rustdoc blockquote, because `every_quotation_is_the_standards_own_words` asks `doc/md/` for every
blockquote under `crates/` and an inserted sentence is in no clause of that conversion.

**Trap 13 sprang on its own calibration in its own words** — the transpose plant passed because the
font matrix composed with the text rendering matrix is diagonal, the shape 746 recorded. Its fixture
carries a shear now.

## 761 — the criterion was already in the gate

`Judgement::CORPUS` widens a consensus's bounds to **twice its members' own spread** — the bound the
gate applies wherever a consensus *does* form. 751's filter tests against the class **floor**, which
is only what `decide` returns *because* no consensus formed, so above 1.0 "we are alone" is a **true
but weak** sentence and most of the list is in that condition.

So the test is exact now: widen the class bounds by the gate's own factor times the closest pair's
comparison, and measure ours against that. Marked means **no reading of these references forgives
us**; unmarked means a consensus at that spread would have accepted us and the answer is in the
divisor. **A round works the marked rows and stops at the first unmarked one.** The marked head is
**13 of 26**. And the readable shorthand is *not* the same test — both sides of the printed ratio are
a max over three measures, so a page whose worst measure differs from the pair's can be marked well
below the obvious threshold; two pages this run are exactly that.

Best diagnosis: a book cover that is one large JPEG **whose lettering is inside the image**, so the
glyph detector gives it the *vector* tolerance and a page of pure image edges is held to bounds
measured on flat fills. Four ladders inside 0.68 of 255, ours joint most central.

**And one page below the mark produced a rule.** Its group note contained *both* a closed-form ink
table clearing it *and* a later sentence saying it had never been opened — **both right**, because
the ink says how much and is silent about where. **A disclaimer inside a group note is a page nobody
is holding.**

Two corrections to its brief: all 26 pages are grouped and eleven carry priced readings, so the real
gap is **which measure** — nine of the eleven price a mechanism without naming which of the three
their number is. And `open_one`'s second argument is a **scale, not a page**, which `doc/todo/00`
step 4's example reads like and is not.

## The residue that outlived the round that made it

**§5 would not build after the merge.** `cargo build --release` died with *data/cmaps is readable: No
such file or directory*, naming a path under `scratchpad/before-759/` — 759's before-arm export,
deleted when the round ended.

759 recorded the *measurement* half of this hazard in `doc/verify.md`: two arms of an A/B need target
directories of their own, because a worktree inherits `/home/AI/.cargo/config.toml`'s and would
otherwise measure whichever linked last. What it could not have seen is the **residue**: a build
script is compiled with its own `CARGO_MANIFEST_DIR` baked in, so an arm sharing the directory leaves
scripts naming a tree that is about to be deleted — and **the round that measured is finished while
the round that pays is the next one to build.** Trap 10b's shape with the staleness pointing at a
tree that is *gone* rather than one that moved.

`touch crates/*/build.rs` and rebuild; nothing else is affected. The note now carries both halves,
and observes that the rule the paragraph already states for measurement accuracy **prevents the
residue as well** — one rule, two hazards.

## Owed

- **Which measure a note's number is** (761): nine of eleven priced readings do not say.
- **A sweep worth building if a second instance appears** (760): a settled row's note carrying a
  `partial`-shaped denial that names another clause as the reason.
- **Orca on all three binaries, by a person.**
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
