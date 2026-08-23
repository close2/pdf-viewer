# 703 — Four diagnoses, and the three that were somebody else's

Fifth merge round of the block. Four branches, one conflict, and a batch in which **every round
corrected the diagnosis it was sent with** — including one this merge round had published two days
earlier as established.

## The sequence, whole, on a quiet machine (load 0.88)

`fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 ·
`nextest` **2513 passed, 18 skipped** · conformance 182 + 5 + 1 + **1** (the new one) ·
`cargo deny` all four ok · corpus **974 documents, 67 incomplete** · oracle **1945 pages — 983
agrees, 65 contradicted, 832 ambiguous, 42 not comparable** · `render-quorra` **957 pages — 932
agree, 23 differ, 2 refused** · accessibility census **1336** · `fixed_documents` 40/0 · text,
selection, dates, XMP, JPEG 2000. §5's binaries rebuilt and installed. **Both workers built first**,
which is this round's own subject.

Ledger **445 implemented, 222 partial**, 0 unreviewed — 701's §14.6.1 and 702's §8.5.2.1.

## The correction this round owes

The four-hundred-word account session 698 wrote of trap 16 — *Cargo unifies features across
whatever is in the build, and the program then counts nine structure elements differently* — **is
wrong**, and 700 established it rather than arguing it.

**It is trap 10.** `pdf-sandbox-worker` is a separate program; Cargo never builds another package's
binaries when testing this one; and a build without it decodes **no `CCITTFaxDecode`, `JBIG2Decode`
or `JPXDecode` image at all**. `hayro-ccitt`, which session 660 blamed and 698 repeated, **has no
`[features]` section at the pinned revision**.

700 did not merely deny the feature theory — it enumerated it out. Unit-graph diffs across three
scopes found ten crates resolving differently (`num-traits`, `once_cell`, `rustix`, `linux-raw-sys`,
`bytemuck`, `log`, `either`, `enumflags2`, `syn`, `proc-macro2`), **and traced each to its consumer
to show that none changes a computed value**. Then one test binary, one digest, run twice each way:

| the one binary, the one directory | placed by own marks | no place | ratchet |
|---|---|---|---|
| no worker beside it | 93 258 | **1345** | **fails** |
| worker beside it | 93 267 | **1336** | passes |

The runs differ on **one file**: `issue5481.pdf`, a `JPXDecode` image. §14.8.3.3 derives an element's
rectangle from what its marked content *drew*; a refused image drew nothing; nine elements lose
their only place.

**The finding is larger than the one it replaces.** Six of the eight corpus gates never checked that
the worker exists — only `pdf-model`'s `corpus` and `oracle` did — and the behaviour of the other
six without it was measured rather than assumed: the census **passes with nine elements in the wrong
column**, `jpeg2000` and `fixed_documents` fail *in words that name no cause*, and two move nothing.
All eight check now, and `tools/conformance/tests/sandbox_gates.rs` reads `doc/todo/02` §2's own
command block — the single place the sequence is stated — and fails any gate line whose file neither
checks nor says why it needs none. Calibrated per trap 13 against `HEAD`'s versions of three files
it names.

The three questions 698 left open are answered: **1336 is right and no floor moves** (they were
never two readings of the standard — one reading by two programs, one missing a component it ships
with); the **shipped binary carries the whole-workspace feature set**, so the odd scope was the
gate's and not the user's; and the feature question has no answer because it was never the question.

**The lesson 698 drew still holds and is the reason this was findable**: *a claim that a defect does
not reproduce is a claim about the conditions you reproduced it under.* What it got wrong was the
next step — naming a mechanism on the strength of one correlation. Four scopes agreed with the
feature theory and one binary refuted it.

## 699 — quorra's questions, and the offset that was ours

`97ad95ac` → `3b105847`, and their API claim **verified rather than assumed**: `quorra-scene` has no
diff at all, every `pub` declaration in both crates is identical between revisions, and the only
addition is `RenderError::OutlineConversionBudgetExceeded`. Exactly one page line of 957 moves, by
+0.0411 of mean, which is their §1.2 to the fourth decimal.

**Their §4 prediction fell against them, honestly.** They argued the per-command offset was not
theirs, on the evidence that our own §31 table's default column is our oracle column under a single
affine — and wrote the falsifier themselves: *if the two transforms are equal to the bit, our
conclusion is wrong and we have a defect we have not found.* They are **equal to the bit, on all 536
rules, both axes**. And they have no defect either: the offset is `pdf_render::sub_pixel_bands`,
**our own §10.7.4 substitution**, and their fitted scale 0.998899 is `16.48164 / 16.5` — the page's
stated pitch over a whole pixel, which is our snap, derived rather than fitted. **Our §31 is
retracted in place.**

**Their §36 question found a defect of ours that moves pixels toward correctness.** `alpha_is_shape`
is `false` on all ten groups of that page and the excess is **knockout**: `pdf-model` had a
`!knockout &&` guard in front of its proof whose stated reason — `Command::Shaped` elements — the
per-element test already enforces, so it refused the case it named *twice* and every other knockout
group once. Removing it converges the page onto their composite and moves its oracle line toward
`poppler`, `mupdf` and `ghostscript`. No boolean is owed them. And measured corpus-wide their proof
is **strictly weaker** than our flag — 61 groups ours reaches that a command-list proof cannot, none
the other way.

**Their §3 routing question is a no with a number**: at the magnification `viewer-ui` actually takes
that lane, their rule would divert **88.31%** of its marks *and would not remove the
non-conformance*, because the zero is a lattice property rather than a width property — their own §3
says so. Their §10.7.4 reading is right, and it is now this tree's fifth recorded departure.

**`#[non_exhaustive]` is a recommendation for the project owner, not a decision taken here.** Cost
verified as zero — every mention of either enum is a `#[from]`, a catch-all, two `matches!` or a
return type. Take it, and `DeviceError` with it. But the round drew a distinction worth keeping:
**one enum they did not name should stay open.** `viewer-ui`'s `swapchain()` holds a genuinely
exhaustive match over `SurfaceProblem`, whose variants mirror `wgpu`'s closed set — there, the
compiler noticing a new arm *is* the feature.

## 702 — the clause settled it, and the census had counted keywords

§8.5.2.1: *"Most operators that add a segment to the current path start at the current point; if the
current point is undefined, an error shall be generated."* That settles which operators — "Most"
excludes exactly the two the paragraph above names, leaving `l`, `c`, `v`, `y` — and when, and that
an error is raised, and **settles nothing about what is drawn**. So two of the three candidates the
briefing offered are refused by the clause's own words: beginning a subpath at the operator's
coordinates contradicts the sentence that only `m`/`re` do that, and is undefined for `c` whose first
two operands are control points. *Add nothing* is taken, and one part of it is **derived**: an
operator that added no segment leaves the current point undefined, so the run of segments after the
error vanishes whole until an `m` or `re`.

**And it corrected the population that sent it.** `issue6342.pdf` does *not* draw the origin line —
its display list holds 36 painted paths, every one beginning with a `MoveTo`. 696's figures are
**keyword counts and an upper bound**, because the interpreter also requires an operator's operands
to be numbers and that file's `c` operators are preceded by byte soup the lexer splits into keywords
of its own. Trap 13 gains a second shape. The real witness is in the crawl — **3 of 65 659, 660
segments** — and it is visible: `1284945.pdf` loses a **yellow wedge running out of the page's
bottom-left corner, 1.09% of its pixels**.

Trap 2 honoured: `tiny-skia` injected the origin move and `kurbo` fires a `debug_assert!` on the same
shape, so the decision is `pdf-model`'s now. Two gate failures, both its own and both correct — one
of them a shape worth naming: **`hostile_budgets`' fixture had been writing `c` with no `m` since it
was written, a fixture violating a clause other than the one it tests.**

## 701 — a claim held in duplicate, and the sweep that was declined

Its selection criterion is an addition to 691's and 697's method: it took §14.6 because **all three
of its rows are `partial` and two state the same list** — *a claim held in duplicate has somewhere
to disagree with itself.* All four findings were disagreements inside the family: §14.6 cited
§8.11.3.3 twice for §8.11.3.2's mechanism while §14.6.1 one line below had it right; "four tags are
read by name" is **five**, the row naming the fifth one sentence earlier; §14.6's reason for
`partial` denied two whole clause families its own rows deny; and §14.6.2 claimed *twice in one
sentence* a report that has never existed.

**One row moved on a clause reading rather than on work.** §14.6.1's recorded debt is nobody's
requirement: §14.7.5.2 says the tag "is not directly related to the document's logical structure"
and makes sameness a `should` on the **producer**. The one modal there binding a *reader* — that a
page's `/Contents` array is one stream for marked-content purposes — holds, and **nothing asserted
it** until now.

**And it measured the instrument 697 asked for, and declined it.** 794 rows with a note, 259
asserting a term, 930 assertions, **46 contradicted inside one note, 24 already marked as
corrections, all 22 remaining noise**. Two structural reasons: ADR 0523 deliberately makes a
corrected note keep both halves, so the population is by definition the notes somebody already
fixed — and it **would not have printed either of this round's own contradictions**. The measuring
program was not committed, on ADR 0481's precedent.

## Errata: the eighth consecutive round

**#302** adds `BX…EX` and `q…Q` to §14.6.1's properly-nested pairs. It is a **licence** rather than
a correction — a conforming file may no longer close a `q` across the `EMC` that `appearance::spliced`
cuts at — and its entire strikeout is the word *or*, under `check`'s four-word floor again. 702
checked the other way and found **no erratum on §8.5.2.1**, which is worth as much: #549 amends only
§8.5.3.1, so the two clauses really are deliberately different.

## Owed

- **The `#[non_exhaustive]` decision**, which quorra says is the owner's to time.
- **`render-quorra`'s corpus** is the one of the six gates not measured without the worker — it
  costs a device run.
- **The census counts no reports**, so a refused image is loud in the interpreter and silent in the
  census; and **nothing guards a measurement that is not a line in §2's sequence**.
- **`viewer-ui` still `exit(1)`s** on `Event::OpenFailed` and on a zero-page document.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
