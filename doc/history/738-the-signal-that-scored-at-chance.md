# 738 — The signal that scored at chance

Twelfth merge round of the block. Four branches, **no conflicts**, and a batch whose four rounds each
replaced something this project had been doing on a plausible assumption with something it had
measured.

## The sequence, whole, on a quiet machine (load 3.27)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2638 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 61 contradicted, 836 ambiguous, 3 our
geometry, 2 reference geometry, 42 not comparable, 18 no render** · `render-quorra` **933 agree, 22
differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445
implemented, 223 partial, 0 unreviewed**; the tree is clean after the ledger command.

## 734 — the incumbent selection rule scores at chance, and the measurement says so

730 found the pairwise ranking's head exhausted. **734 measured eleven candidate signals against the
rows nine rounds actually found defects in** — 21 rows that were `partial` or `reported` at their own
round's base, each scored against the ledger *as it stood before that round*, so nothing is fitted
after the fact. Best signal: the 38.6th percentile against 50 for chance. **The incumbent pairwise
score is 48.3 — chance, at row level**, which is honest rather than damning: it ranks *families* and
says so. Even the hypothesis anyone would have bet on — how many commits have rewritten this note,
since the recurring shape is a correction that reached one sentence and not another — scored **46.0,
worse than word count**. So "pairwise with read pairs excluded" would have chosen rows by a measure
that does not discriminate rows.

**The successor is not a ranking over notes**: rank each live row by the errata annotations falling on
it whose issue number this tree names **nowhere**, reassemble each issue from every clause `emit`
files it under, and read it whole. And it has the property the incumbent lacks — **a head that runs
down**: reconstructed at all nine base commits the unread-issue population falls monotonically **103 →
86**, about two a round, where the pairwise score *rises* on the family just read. Eight of the eleven
errata those rounds recorded were in the population at their base. At this base: 356 issues carry
annotations, the tree names 115, **241 are named nowhere**, and 63 of those change text and land on a
live row.

**Its first use found three unnamed issues where 710 had reported one**, on pages 710 had opened with
the same instrument. One moves a sentence off the clause it was attached to, and a defect was exactly
there: a flag row has two sentences and only the first had a reader, so an unknown annotation subtype
*without* an appearance stream was reported with a claim about *"its clause"* — **a clause a subtype
outside Table 171 does not have** — while the row's own "if any" says nothing is owed. Population
measured before the fix: **0 in the 974, 134 in 4 of 65 703 crawled**.

**And a fourth blindness that is not the errata tool's but the tree's grep**: `&#124;` is a Markdown
escaped pipe and the only numeric character reference under `crates/`, `doc/` or `tools/`, so a bare
search for `124` answers "recorded". **One collision exists in the whole tree and it is on the issue
that went unrecorded.**

## 736 — the boundary wired, and the cost that was on the other side

All three of ADR 0626 §6's blockers, **in the order they actually block each other** — the first two
being one question and its consequence. Which host rasterises follows from 724's constraint: a process
holding a graphics device cannot be confined at any ordering, so the device is the host's *by
necessity* and `viewer-confined` takes **no rasteriser dependency**. Breaking every consumer is then
small — three, all in-tree. `MAGIC` moved **exactly once**, because something new genuinely crosses.

**The briefing's framing was half wrong and the half it missed was the expensive one.** The cost is
not only a decode in the privileged process; it is an **encode in the confined one, on every frame** —
6.6 ms and 33.7 MB on one scan, 16.0 ms and 80.1 MB on another, **paid by exactly the pages that get
nothing for it**. ADR 0626 had documented that cost as acceptable, which was right for a codec run once
per corpus and wrong on the frame path. `crossing` hands the encoder the raster's size and it stops
there: **4 and 6 microseconds**, in two checks that are not a second statement of the format.

| page | crosses as | payload | crossed in |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` p1 | marks | 35 580 B vs 4 075 200 B | 0.074 ms |
| ISO 32000-2 p1 | marks | 1 011 568 B | 0.778 ms |
| two scans | pixels | 4 194 000 B | 2.758 / 3.312 ms |

The new arm needed a bound of its own, and it is 719's finding somewhere new: **a render target is the
one length here with no bytes behind it** — eight bytes that become the *host's* allocation, out of a
frame under 200 bytes. Zero dimensions, dimensions past the extent, and pixel counts past the ceiling
are all refused, and the wire test asserts all three.

**Moving `MAGIC` exposed a stale second implementation with a real consequence**: the fuzz seeder
asserted the *previous* magic, so it had been refusing to run and **that target's corpus had been empty
since the last bump landed**. A fuzz target clean on an empty corpus is a gate reporting success for
doing nothing. It reads `MAGIC` out of the source now; both targets re-run clean at 1 500 000 and
25 550 340.

## 735 — the target was wrong, and the standard is why

**"Nine of nine" is not obtainable.** Three of the nine widgets belong to fields the document marks
read-only, and a fourth is Table 229 bit 15 — *"selecting the currently selected button has no
effect"*, and the table's own first three words are "(Radio buttons only)". **A host toggling nine
would be disobeying the file.** Parity means all three windows refusing the same four for the same
printed reasons, which is what they now do — **6 of 9, 3 refused by name, identical line for line**,
where before this round the two native windows gave a value to *none*.

**And 731's "three of nine" was an artefact of its measurement**: a batch of nine clicks followed by
one read measures the *net* of a walk in which a radio set's second click undoes its first.

**Two defects the measurement found, neither the thing built.** A screen reader was told **every
button of a radio set was selected as soon as one was** — the map was keyed by the annotation but
stored the *field's* control under each widget, the right fact sitting beside the wrong one with its
doc comment saying which is which. And **`viewer-gtk` never wrote a toggle back**, since ADR 0244:
trap 1 in its purest form, because with the write-back removed **the bus still reports 6 of 9 toggling
and the window's pixels do not move at all.** Only photographing it caught that.

It also replaced **three copies of §12.7.5.2, one per window, which had already drifted**: only
`viewer-ui` asked Table 227 before sending an edit, the other two relying on a disabled control —
which is a fact about a *person's* click and not about the two other ways one arrives.

## 737 — the ordering an ADR argued by hand in session 514

Its criterion was 722's rule for choosing: **do not invent a criterion where an existing one has an
unevaluated precondition** — and the pool had exactly that, recorded as owed in the *Owed* sections of
722, 727 **and** 729. **Nothing ranked the pool by how far outside its bound each page sits.** ADR 0349
took that ordering by hand, found its head was a page the printed ranking never prints, and wrote the
argument instead of the code; it could only be built now because ADR 0617 supplied what 0349 could not
settle — *which* consensus the ratio is taken over on a page carrying two. 729's rule answers it: the
**smallest** of their numbers, since a contradiction is what every set reaches.

**Calibrated at both ends against figures written by rounds that could not see this code**: head
127.75×, ADR 0349's own hand-taken number; foot 1.00×, ADR 0606's six differing channels of eighty
thousand.

Two findings. **The ranking beside it is blind to the bound most of the pool fails on** — `Distance::of`
keeps three measures and not the differing fraction, which is ADR 0242's own defect surviving one level
up: that round fixed the per-page *line* and left the *order* in the unit that cannot see it. And **one
page is convicted twice at half the price**: two maximal consensuses, one holding us 2.30× outside and
the other 1.12×, both rejecting so no verdict moves — but the exemption is worth 1.12×, not the 2.30×
its line and its note quote.

## What the batch has in common

Each round replaced an assumption with a measurement, and in three of the four the measurement said
the assumption was wrong:

| assumed | measured |
|---|---|
| the pairwise ranking would still pick rows with read pairs excluded | it scores 48.3 — chance — at row level |
| the codec's frame cost is a decode in the host | the larger cost is an encode in the *confined* process, on pages that gain nothing |
| nine of nine widgets should toggle | four must not, and the clause says which |
| the pool's remainder needed a new criterion | an old one had never had its precondition built |

## Owed

- **One `viewer-core` change** so the confined worker stops drawing a page it ships as marks — it
  needs an outcome meaning *the host took the request's own list*, because the nearest existing one
  takes `MAX_PIXELS` off a confined process's raster. `doc/ui-boundary.md`, `doc/todo/15` and
  `doc/todo/34` all state it.
- **The differing fraction in `Distance::of`'s ordering** (737), unpriced until now.
- **Orca on all three binaries, by a person** — how a screen reader behaves with two applications of
  the same name on the desktop is 731's one consequence only a real client can judge.
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
