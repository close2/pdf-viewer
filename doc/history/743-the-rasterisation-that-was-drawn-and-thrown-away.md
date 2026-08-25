# 743 — The rasterisation that was drawn and thrown away

Thirteenth merge round of the block. Four branches, **no conflicts**, and the batch where the
confinement boundary stopped being a design and started being a number: **8.73 ms to 2.82 ms** on a
small page, and **26.5 seconds to 18 milliseconds** on a fixture built to be hostile.

## The sequence, whole, on a quiet machine (load 1.16)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2648 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 61 contradicted, 836 ambiguous, 3 our
geometry, 2 reference geometry, 42 not comparable, 18 no render** · `render-quorra` **933 agree, 22
differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445
implemented, 223 partial, 0 unreviewed**, tree clean after the ledger command.

## 740 — the outcome that was owed, and the test that broke for the right reason

`Rendered::Listed` — *the host took this request's own list* — the **first change to that vocabulary
since it was frozen**, and taken only after checking the three mechanisms `doc/ui-boundary.md`
prefers: a field on the request cannot say what the host did *after* it; a variant changing shape
would put two facts of different lifetimes at one width; a `Query` is no use for an answer that
arrives unasked. Three alternatives were rejected **on the budget rather than on taste**, including
the one that looked cleanest — letting a page appear without pixels ships an unreachable arm to four
hosts and a C frame with a size and no bytes. `MAGIC` did not move; the format is byte for byte
`PDFVCF04`.

| page | arm | before | after |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` p1 | marks | 8.73 (5.56) | **2.82 (2.41)** |
| ISO 32000-2 p1 | marks | 48.86 (46.72) | **40.38 (36.50)** |
| `scan-bad.pdf` p1 | pixels | 71.60 | 70.78 |
| `issue12841_reduced.pdf` p1 | pixels | 178.72 | 171.43 |

**The two pixel-arm controls are flat, and that is what makes the attribution sound** — they say the
top two rows are the rasterisation leaving rather than the machine wandering. The amplification
fixture, 1.5 kB and ten thousand page-covering fills: **26.5 s → 18 ms.**

**A test broke and the reason is the round's most durable output.** The cancel test failed in 31 ms —
its hostile document's forty seconds were all in the *rasterisation*, and its marks are smaller than a
window's raster, so the worker shipped it undrawn and **there was nothing to cancel.** The fixture is
deeper now and the test *checks that premise* before blocking on it. What follows is a sentence about
the boundary rather than about a fixture: **a cancel stops the work the worker does, and on the marks
arm that is the interpretation, not the drawing.** Not a hole this round opened — since ADR 0633 the
host has had to draw those marks outside the confinement anyway, and the worker was making a second
copy and discarding it — but **a host taking marks owns their draw**, and needs an answer for a page
whose *cost* its size said nothing about. `doc/todo/15` carries it; `doc/todo/34` §3 states the
cancel's scope exactly.

## 742 — a regression of this tree's own making, priced by the round that made it

The round chose its own subject and chose well: session 714's chain pump took a hex-armoured bomb's
peak from 1 070 828 KB to 22 608 **and recorded, in the same table, the wall clock going the other
way** — 154.98 µs to 14.62 s. A 25 000× cost shipped 28 rounds ago in exactly the input class
principle 3 exists for. Its first act was re-deriving the price: the generator reproduced both
witnesses **to the byte** and both still cost 13.7 s.

| twenty pages, one form | before | after |
|---|---|---|
| 4 174 537 B encoded | 14.32–17.99 s | **186–287 µs** |
| 12 523 517 B encoded | 14.24–16.89 s | unchanged, on ADR 0586's argued refusal |

Callgrind over 1023 pages: **+0.0031%**, and the sign is honest rather than hoped for.

**The item's price was one fact short, and the missing half is the interesting part.** It said "one
fact travelling one hop". But a window hands over everything up to the bound and *then* says it
stopped, so remembering *too large* alone would make the first read draw a prefix and every later read
draw nothing — **a page that is a function of whether the cache still holds an entry.** What travels
is *too large **and** empty*, which is exactly reproducible and which `Interpreter::run` is the one
place able to see both halves of. The tree had already written that rule down for the other half of
the same memo and nobody had carried it across.

## 741 — the fourth measure asked and declined, on a calibration

737 handed on that `Distance::of` keeps three measures and not the differing fraction. **The decision
rests on a calibration rather than on the cost of moving a hundred notes**: session 518 took the same
reading by hand, in levels of 255, over 786 pages and recorded **56**. Over this run's 836, three
measures name **58 — 6.9% against 7.1%** — and four measures name **583, or 70%**. *The unit that
reproduces an independently taken figure is the one already in the code.* So the fourth measure sits
**beside** `Distance::of` and no published figure moved.

Why four fails is `doc/todo/12`'s bound doing two jobs: the differing fraction is the largest ratio on
**762 of 804** complete pages, so folding it in orders the whole bucket by one measure — while ours
sits at a median 2.08× the class floor against the closest reference pair's 1.96×, and ours is the
*smaller* on 222 of the 804.

**And the defect it found while measuring is ADR 0242's, one level up again**:
`rank_the_manufactured_ambiguity` prints two numbers on one line and **they are two different
instruments**, four bounds against three, while its own doc comment and `doc/todo/00` step 1 ask a
reader to compare them.

```
35.12 between them,  5.03 ours in three measures,  32.42 in four
```

As printed, the references appeared to disagree **seven times more** than we differ from the nearest
of them; in one unit they are eight percent apart. Over the pool the mixed reading names **13** pages
as ones we are alone on where either single unit names 48 or 569 — *not a conservative version of
either question, an answer to neither.*

**It also corrected the briefing on a load-bearing point**: the ranking it was sent to fix prints
**zero rows**, filtering to undiagnosed pages, a list empty since ADR 0543. Re-ordering it would have
changed nothing.

## 739 — the new rule's own step was wrong, and the second collision family

734's successor rule on its second use, and **its step 2 was wrong**. The recipe's grep demands an
`Issue #` prefix while `doc/errata-read.md` writes numbers **bare, in a table column**, so the
prefixed grep finds 113 of 351 where that one file records **159** — pushing already-read rows up the
ranking. **A bare-number grep is not the repair**: `doc/HAYRO_ISSUES.md` lists *another project's*
GitHub issues and names four of the five errata this round then read. So 734's escaped-pipe collision
is not the only family, and this one is larger and lands on live errata numbers. Step 2 is two greps
now, unioned, with character references stripped.

**With it repaired the head is exactly where 734's nine-base reconstruction said it had been at every
base with nobody on it** — so the rule holds and the instrument reading it was short.

The work behind it: a row said "Table 255 entire" and named thirteen of eighteen entries; four of the
five missing are declined in the entries' own words, and the fifth was real — *"The value is 1 if the
Reference dictionary shall be considered critical to the validation of the signature"*, the one
sentence of that entry addressed to a **validator**. This program evaluates no transform method, so a
file writing it names **the part of its own validation this program skips**, and nothing here read the
entry. **0 curated documents, 2 of 65 703 crawled**, each carrying both reference kinds; calibrated
two ways.

And a second place the errata collection disagrees with itself: an erratum strikes `(Required)` from a
Table 256 entry and **leaves the NOTE below it standing, still reporting a correction the erratum
undid.**

## Owed

- **A host taking marks owns their draw**, and needs an answer for a page whose cost its size did not
  predict (740). `doc/todo/15`.
- **`doc/todo/00` step 1's comparison**, now printed in both units with a census line beneath it —
  what it names is a queue nobody has read.
- **Orca on all three binaries, by a person** — 731's one consequence only a real client can judge.
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
