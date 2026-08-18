# ADR 0417 — The budget moved to the page that spends it

Status: accepted, 2026-08-18. Session 582. Makes `pdf_model::colour::MAX_PRESSES` a budget on one
interpretation instead of on the process, keeps the sampling in a bounded process-wide cache behind
it, removes the two `Interpretation` fields ADR 0416 added to tell a process-decided verdict from a
file-decided one, and pins both halves with tests a `static` had made unwritable. Amends the ledger
rows for §11.4.7, §11.6.6, §11.7.2, §8.6.5.8 and §8.9.5.1, and `doc/todo/49`'s third-bound section.

## What ADR 0416 left, and whether its ranking was right

Session 581 attributed `tools/safedocs survey`'s nondeterminism to `colour::MAX_PRESSES` — 8 slots
in a `static` table, filled from the front, never evicted, so the ninth *distinct* four-component
blending space a **process** met was refused and which document that fell on was decided by the
order rayon ran them in. It made the refusal legible and priced three roads in `doc/todo/49`:

1. **raise the constant** — rejected: it moves the line rather than the payer, curve-fits to one
   corpus, and the flipping returns at the new number;
2. **reclaim a slot nothing is using**, with leases and a generation in the handle — rejected:
   more than eight presses can be live at once under a 24-way walk, so a refusal would still
   depend on the neighbours, and then on the machine's core count, which is worse than depending
   on the scheduler because it *looks* reproducible;
3. **make the budget per-interpretation** — the only road whose refusal is a function of the file.

**The ranking is right and the third entry is under-specified**, which is the amendment this round
makes. Road 3 as written treats the budget and the *table* as one object — "memory becomes live
interpretations × their own presses" — and they are two. A budget decides what is drawn and must
therefore be the file's; a store of sampled presses decides how fast an answer is reached and never
what it is, so it may be shared where a budget may not. `doc/todo/49`'s own "keep" list already
states that rule for `Document` — "a **cache beside** it breaks nothing — purity is about the
answer, not about how fast it is reached" — and it applies here unchanged.

**The measurement is what forced the split rather than the argument.** `examples/press_cost`
interprets page one of a press-naming document twice in one process and subtracts:

| document | cold | warm | warm again |
|---|---|---|---|
| `0100306.pdf` (page group `/CS`) | 53.0, 61.8, 58.0 ms | 14.3, 15.5, 16.3 ms | 14.1, 15.4, 15.9 ms |
| `0000004.pdf` (output intent) | 34.3, 37.3, 37.8 ms | 17.5, 17.2, 17.7 ms | 17.2, 17.6, 18.2 ms |

Sampling a press is 17 to 46 ms — 83 521 profile evaluations for the grid, plus 4913 ink searches
for the conversion back in — against a 14 to 18 ms interpretation of the same page. A table whose
lifetime is the interpretation's and nothing behind it therefore **doubles to quadruples every page
turn** of a document whose pages share a press, which is the case the process-wide table existed
for. `CLAUDE.md` principle 2 asks a benchmark of an optimisation; this is that question asked about
*removing* one, and the answer is that the sharing has to stay and only the budget has to move.

## What was built

**`MAX_PRESSES` is a budget on one interpretation.** `colour::Presses` holds the distinct presses a
page has named and refuses the ninth. It is created once per `interpret_with`, so §11.4.7's pair —
one content stream interpreted twice, once carrying cyan, magenta and yellow and once carrying
black — names one press between its two runs, and so does the device rerun beside them. The budget
is asked *before* the sampling rather than after it, because a refusal that costs the work it
refused is not one.

**`MAX_CACHED_PRESSES` is the store, and it is a cache.** `colour::SAMPLED` holds eight sampled
presses most-recently-used first and drops the tail; a document naming a press it has evicted pays
for the sampling again, which is slower and is the same picture. Eight is what this process spent
on presses before, so the change costs no steady-state memory. A cache bound *may* be sized against
a measured population where a budget may not — the crawl names 28 distinct presses — and the number
is left where it is because what raising it buys is measured below and is small.

**The press itself moved into `Compositing`.** It was a `Copy` index into the `static` table, and
an index is only cheap while the table is process-wide: scoping it means the press has to be
reachable wherever a colour is resolved, and a colour is resolved *per colour*. So
`Compositing::Subtractive` carries an `Arc<Press>` — a pointer already in hand, so the read on the
hot path is what it was — and what is cloned is a refcount, once per structure that holds one.
`Compositing` loses `Copy`, and the ~40 signatures that threaded it by value take `&Compositing`
instead; its `Ord` and `Hash`, which `crate::shading`'s cache is keyed on, are written out over a
`PressIdentity` rather than derived, because two `Arc`s of one profile are one press and eviction
makes that reachable.

**ADR 0416's two fields are gone with the distinction they drew.**
`Interpretation::press_beyond_this_process` and `reports_beyond_this_process` existed to say which
half of a verdict was not the file's; every half is the file's now.
`transparency::BeyondPress::this_process` goes with them, and so does the reading-order rule that
put a file-stated reason ahead of the process's inside `blending_undrawable` — with nothing to
order, the order is back to the plain one and any of its answers is the same on every run.
`tools/safedocs survey` no longer subtracts anything from its own incomplete count or marks a
document with `[this process's press budget]`; its one remaining press line says how many presses
the run sampled and how many it still holds, which is a statement about speed.

## What it cost, measured

**On an ordinary page, nothing.** `callgrind_interpret` over page 101 of ISO 32000-2, fifty
repetitions: **1 216 672 247 instructions before, 1 216 726 530 after — +0.0045%**. That page names
no press, so what is measured is the `Arc` in the enum and the reference-taking around it.

**On the survey, the cache pays twice and it does not show.** Three runs before and three after,
over the 287 crawled documents whose page-one blending space is named by a four-component ICC
profile this tree evaluates, the machine otherwise quiet:

| | incomplete | of those, the process's | verdict lines |
|---|---|---|---|
| before | 45, 46, 47 | 26, 27, 28 | **differ between runs** |
| after | **19, 19, 19** | — | **byte-identical, over six runs** |

19 is exactly what ADR 0416's `MAX_PRESSES = 256` scratch build printed with the bound removed
altogether, which is what says the fix is the fix rather than a different arrangement of the same
mistake. Wall clock: 44.7, 46.2, 45.9 s before against 47.6, 49.8, 48.9, 47.5, 47.9, 46.5 s after
— **about 5%**, and it is not overhead: 27 more documents are now *drawn in ink*, which is a page
interpreted twice for §11.4.7's pair. The run reports 60 to 69 samplings against a cache of eight,
which is the thrashing a 24-way walk over 28 presses produces; a cache of 32 would hold the whole
crawl at 34.6 MB, and it is not taken because what it would buy is inside the spread above.

**The population is unchanged and still stable.** `examples/press_census`, one process per archive
over all 145, run twice: byte-identical, 65 703 documents opening, **2296** stating §11.4.7's
condition, **287** naming their press through a four-component profile this tree evaluates, **28**
distinct presses. The same three numbers ADR 0416 established.

## The three long-lived consumers, and the test that was unwritable

The defect is a *viewer* defect before it is an instrument one, and the three programs that hold a
document open across many pages are where it bit: `viewer-ui`'s window, `viewer-confined`'s worker
across a whole session, and the gates walking a corpus in one process. All three are answered by
the same sentence — a press is refused only where the page names more than eight — and one of them
is now a test.

ADR 0416 recorded, deliberately, that no gate pinned any of this: "filling eight slots needs eight
distinct four-component ICC profiles and the table is `static`, so the test would decide the answer
for every other test in its binary". That objection expires with the `static`, and both tests are
in `crates/pdf-model/tests/transparency_groups.rs`:

- **`every_document_in_one_process_is_drawn_in_the_press_it_names`** opens ten documents in one
  process — two more than the old bound, so the run passes it rather than reaching it — each naming
  a press of its own, and asserts every one carries §11.4.7's pair and reports no blending colour
  space. That is the long-running viewer, and it **fails with the process-wide table put back**.
- **`a_pages_press_budget_is_its_own_and_it_spends_no_other_pages`** states nine sibling
  transparency groups on one page, each naming a distinct press, and asserts the ninth is reported;
  then interprets an eight-press page naming a **disjoint** set *after* it and asserts that one is
  complete. The disjointness is what makes the second half discriminate — two pages naming the same
  eight presses are drawn identically by a process-wide table too. Both of its assertions fail with
  the process-wide table put back, and the first also fails with the budget itself removed, so
  neither is passing for want of the thing it is about.

Each was confirmed to fail by putting the mechanism back rather than by inspection, which is
`doc/HANDOVER.md` trap 2's rule about a scene that guards nothing.

## The spec half: §8.9.5.1, and a parameter with three routes and one arrival

A `partial` row read against the code, in a family this round does not touch. §8.9.5.1's note is a
list of Table 87's entries and which are read, and it carries its own warning — three entries have
been recorded there as unread while the tree read them, and it says "[a] list that has been wrong
three times about itself is a list to check rather than to read". It had a fourth hole of the
opposite kind: **four entries it disposes of neither way**, `/Intent` (PDF 1.1), `/AF`, `/Measure`
and `/PtData` (all PDF 2.0). `conformance --bin entries` prints them; what they are worth was read
one at a time.

- **`/Measure` and `/PtData` are a boundary rather than a gap.** The tree's one reader of
  `"Measure"` is §12.10.2's viewport and its one reader of `"PtData"` is §12.9's, and both take the
  dictionary off a viewport, so an image's own scale and geospatial point are those clauses' rows
  to owe.
- **`/AF` has a reader and no caller.** `attachment::associated` reads §14.13's array off any
  dictionary, and the only site in this tree that hands it one hands it the **catalog's** — so an
  image `XObject`'s associated files are reachable by nobody, and §14.13.7's row overstates by
  saying the entry is "read by the same function against that dictionary". That is `doc/todo/01`'s
  fifth sweep's shape: the model implements this, and who calls it?
- **`/Intent` is the one that can move a pixel, and the finding is wider than the entry.**
  §8.6.5.8's row already records that this third route to the rendering intent is unread. What
  neither row said is that the parameter reaches **no image sample by any route**: `image.rs`
  converts every sample with black point compensation on, as a literal `true` at each of its six
  `Compositing::paint` calls, and so do `shading.rs`'s ramp and `mesh.rs`'s vertices. The parameter
  is threaded as far as `content/colour.rs` and stops there. So `ri /AbsoluteColorimetric`, which
  §8.6.5.9 requires to turn compensation off and which this tree obeys for a path and for a glyph,
  is obeyed for nothing an image, a shading or a mesh paints — which is trap 5's own archetype,
  *where a clause gives a parameter two routes, implementing one of them is the failure mode that
  reports nothing*, one level along.

The condition for a mark to move is derived rather than guessed, which is trap 11's rule: an intent
of `AbsoluteColorimetric` in force, a sample in a CIE-based or ICC space, and a profile whose black
point is not the connection space's. The population is measured and tiny — **0 of the 974 corpus
documents and 0 of the 275 under `doc/corpora/` contain the name `AbsoluteColorimetric` at all, and
2 of the 65 944 crawled ones do**, a cleartext grep and therefore an undercount for a content
stream and not for a dictionary. So it is written into §8.9.5.1's and §8.6.5.8's rows with its
condition and its cost, rather than built in a round that is already a refactor.

## What is not done

- **The cache bound is eight and the crawl names 28.** A survey pays 60 to 69 samplings for 28
  presses. Raising it is a number with a measurement attached and the measurement says the
  difference is inside the run-to-run spread, so it stays where it is and this is the sentence that
  says why.
- **Concurrent interpretations are bounded one at a time and not together.** The budget is per
  interpretation, exactly as `MAX_TILES` and `MAX_OPERATIONS` are, so a host running N of them may
  hold N × 8 presses. That is the same shape every other budget in this tree has and the same
  number `doc/todo/49` priced for road 3 — 8.6 MB for a viewer, 207 MB for a 24-thread survey at
  its worst — and no host in this tree approaches it.
- **§8.9.5.1's `/Intent` is read nowhere and reported nowhere.** Named, conditioned and sized
  above; `doc/todo/49` is not its home and neither is this round.
