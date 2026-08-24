# 711 — The dependency that cited the wrong clause

`doc/todo/11` item 7's remainder had been blocked on item 5's seam since the
six-hundred-and-forty-sixth session, and the block named §11.3.7.3. **It is the wrong clause.**
§11.3.7.3's union is what the standard says to do with two *objects*; the blocked case is a path
stating several rectangles, which is one object's subpaths, and §11.6.2 governs those and answers
the opposite way — "[p]ortions of an object shall not be composited with one another" *forbids* the
construction item 7 was weighing against a cost. There was never a trade. Seven eighths of the case
is paid.

**And item 5's own foundation was the same mistake one level up.** ADR 0308 recorded the seam as
"not a deviation from the model — it is the model". The model's values live at *points* (§11.2, a
`shall`, and §11.6.4.2's "1.0 inside and 0.0 outside the path"), so §11.3.7.3's union across a seam
is 1.0 and **the clause states no seam anywhere**. The fraction enters through a `can` in
§11.3.7.2's NOTE 1 about rasterising to device pixels, and averaging does not commute with a
non-linear function — which is the loss §11.2's own NOTE 1 names, and attributes to committing to a
raster before the stack is rendered. The artefact, its measurement and its price are unchanged; what
changed is that it is a licensed *departure* from a value the standard defines rather than the
standard's own arithmetic.

Date: 2026-08-24.
ADRs: [0582](../adr/0582-the-seam-is-not-the-model.md) — the reading;
[0583](../adr/0583-several-rectangles-are-one-object.md) — the construction.
ADR number 0584 was allocated to this round and not used.

Habit: `doc/habits.md`'s *Reading the specification* gains the tell — a dependency between two owed
items whose reason is a clause number.

Touched: `crates/pdf-render/src/edge.rs` (`DeviceRectangles`, `device_rectangles`,
`share_a_device_pixel`, `RECTANGLES_PER_PATH`), `crates/pdf-render/src/lib.rs`,
`crates/render-cpu/src/lib.rs` (`rectangular_mark`, the fill path, the clip chain),
`crates/render-cpu/src/scan.rs` (`Exact`, `fill_rectangles`, `mask_fill`, `intersected`,
`mask_intersect`), `crates/render-cpu/tests/edge_coverage.rs` (three scenes),
`crates/render-quorra/tests/abutting_marks.rs` (one scene),
`crates/render-quorra/tests/corpus.rs` (`issue8187.pdf` left `DIFFERS_AT_THE_EDGES`),
`crates/pdf-model/examples/rectangular_path_census.rs` (new),
`doc/conformance/ledger.toml` (§10.7.4, §11.3.7.3, §11.6.2), `doc/todo/11`,
`doc/todo/_scan-conversion.md`, `doc/habits.md`, the two ADRs and this file.

## The order of the round

**The clause first, and it took three readings to find the right one.** §11.3.7.3 and §11.4.4's
NOTEs were where the briefing pointed; what settled item 5 was §11.2's second paragraph and its
NOTE 1, and what settled item 7 was §11.6.2 — a clause neither item cited and whose ledger row was
already `implemented` for a different population of the same sentence.

**Then the census, before the code** (trap 14). `pdf-model/examples/rectangular_path_census` over
first pages at scale 1: on the pdf.js corpus, 223 545 fills, 12 987 one rectangle, **3419 several
with no shared device pixel**, 505 several sharing one, 3084 with a rectangular subpath declined.
That is what decided the scope — seven eighths of the population needs no coverage buffer, so the
half that does was left with its price written down rather than built for.

**Then the construction, then the plant** (trap 13). Two new scenes in `edge_coverage.rs` were run
against the unfixed tree and failed: a three-rectangle path whose edges fall 0.05 of a pixel across
painted **nothing** there, and the same two rectangles as a *clipping region* were 0.1097 of a pixel
out — 28 levels of 255. The third scene, `two_portions_sharing_a_pixel_are_not_composited_with_one_another`,
passes both ways on purpose: it is the guard that fails if the construction is ever applied where
§11.6.2 forbids it. The backup-and-restore used a scratch copy of the four source files rather than
`git stash`, three neighbours being on the same repository.

**And the A/B arm had to be built from the sources rather than by disabling the branch.** An
`if true { return }` lets the optimiser delete the new function entirely, and the same page then
reads 0.1% low — +0.18% became +0.02% on the text page under that arm. Every figure below is against
a build of the unmodified `crates/`.

## Measurement

Machine load ranged from 2.8 to 40 over 24 cores across the round, so **no timing figure was taken**.
Every number is a raster value, an instruction count or a verdict, none of which a loaded machine
moves. `PDFREF_CACHE` pointed at the shared warm cache and the oracle reported a 100% hit rate, so
no reference renderer ran during any gate.

`callgrind_rasterise`, `RAYON_NUM_THREADS=1`, twenty rasterisations:

```text
  ISO 32000-2 p101 (text, no multi-rectangle fill)
                                    5,384,472,180 -> 5,388,457,698   +0.074%
  colors.pdf p1 (ADR 0476's witness)      521,681 ->       521,670   -0.002%
  issue840.pdf p1   (427 such fills) 5,420,592,984 -> 5,417,497,850   -0.057%
  issue1350.pdf p1  (142)            2,975,303,242 -> 2,964,226,161   -0.372%
  issue13447.pdf p1 (289)            6,769,221,234 -> 6,733,313,629   -0.531%
```

The +0.074% is why `device_rectangle` is now a *variant* of `device_rectangles` rather than a second
entry point: asking two functions walked every declining fill twice and cost +0.18% on that page.

`raster_digest` over the pdf.js corpus: **135 of 974 first pages move pixels**, 0.17% to 0.36% of
their bytes, worst channel 45 to 58 levels. A share of them is text, which item 7 did not predict and
which is correct — a glyph is a `Command::Fill` of its outline, so a glyph whose outline is two
axis-aligned rectangles is a two-subpath fill like any other.

## What the gates said, measured both ways

- **The reference oracle: byte-identical.** 983 agrees, 65 contradicted, 832 ambiguous, 3 our
  geometry, 2 reference geometry, 42 not comparable, 18 no render — before *and* after, with every
  ranking line identical. The before arm was a full run of the gate against the unmodified sources,
  not a number quoted from a report.
- **The cross-backend gate moved, in the direction it is allowed to.** 932 agree / 23 differ →
  **933 / 22**: `issue8187.pdf` left `DIFFERS_AT_THE_EDGES`. Its page is fourteen fills of which
  **fourteen** state several rectangles, and the processor was rounding all their edges to a quarter
  where quorra tracks the fraction to a level of 255. The processor moving to the device, as with
  ADR 0476's `issue18823.pdf`.
- **`doc/todo/00` step 7's ink sweep, both ways over all 768 ambiguous pages.** 73 rows moved, 29 up
  and 44 down, by at most **0.076** of 255; the **negative tail is byte-identical** — 19 at or past
  −1, head `issue12418_reduced.pdf` −19.447, `issue4722.pdf` −13.810, `issue15977_reduced.pdf`
  −12.927, `bug1050040.pdf` −11.272, `issue5801.pdf` −8.991, which are the five ADR 0433 names. Both
  arms reproduce the five-hundred-and-ninety-eighth session's figures to the thousandth, which is
  what says the recipe was re-implemented correctly rather than approximately.
- The rest of §2 green: `fmt` and `clippy` silent under `RUSTFLAGS="-D warnings"` (the `viewer-qt`
  `cargo:warning=` lines are gcc's on a cold build, `doc/todo/02` §2's documented non-lints), 2536
  workspace tests, the doctests, the fuzz targets' `check`, the corpus gate, both censuses,
  `text_extraction`, `dates`, `xmp`, `jpeg2000`, `fixed_documents` (40 checked, 0 absent) and
  `conformance`, which caught one renamed test in §10.7.4's list.

**135 pages moved pixels and not one oracle verdict, ranking line or figure moved.** That is a fact
about the gates rather than about the change, and it is the third time this block has recorded it —
ADR 0492 found the same for a group's clip composition. The instruments that could see this one are
`raster_digest`, the cross-backend gate and the two new scenes.

## Two things worth keeping that are not in either ADR

**ADR 0308's per-backend attribution has aged and nothing pointed at it.** It records the processor
at 0.2510 and `render-quorra` at 0.2471 on the abutting-marks fixture; run today the two are
**exchanged**, because ADR 0476 made the processor's rectangle exact so 0.75 of a pixel is measured
rather than supersampled and 0.25 rounds the other way. Both are within one level of the union's
0.2500 and neither says anything new — but the §11.3.7.3 ledger row had copied the attribution, and
that is trap 1's third shape: a sentence true when written that nothing pointed at when the tree
moved under it. The row states the run as where the figures are read now.

**The multi-rectangle case is where a conflation-free rasteriser would first be needed, and it is
much cheaper there than item 5's.** The 505 fills whose portions share a pixel need one coverage
buffer per mark with the portions' areas *summed* into it and the paint blitted once — which is
`scan::intersected`'s shape already (ADR 0355), inside one object, with no blend mode or group
question to answer, because §11.6.2 has already said the portions do not composite. A round that
wants item 5's rasteriser could learn its shape here for a twentieth of the price.
