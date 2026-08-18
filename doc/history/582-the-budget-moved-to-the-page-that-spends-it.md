# 582 — The budget moved to the page that spends it

`pdf_model::colour::MAX_PRESSES` is a budget on one interpretation instead of on the process, over
a bounded cache of the sampling that a measurement said had to stay shared — so the same file draws
the same way whatever else the process opened first, and the two tests that could not be written
while the table was a `static` are written.

Date: 2026-08-18.
Argued by: [ADR 0417](../adr/0417-the-budget-moved-to-the-page-that-spends-it.md), which amends
ADR 0416.

Touched: `crates/pdf-model/src/colour.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/{colour,image,pattern,report,transparency}.rs`,
`crates/pdf-model/src/{image,mesh,shading,thumbnail}.rs`,
`crates/pdf-model/tests/{transparency_groups,image_reuse}.rs`,
`crates/pdf-model/examples/press_cost.rs` (new), `tools/safedocs/src/{main,survey}.rs`,
`doc/conformance/ledger.toml`, `doc/verify.md`, `doc/todo/{03,49,README}.md`.

## The demand half

Session 581 found the bound and left it, pricing three roads and saying only one worked. This round
took that one and amended it: road 3 priced the budget and the *store of sampled presses* as one
object, and they are two. A budget decides what is drawn and must be the file's; a store decides how
fast an answer is reached and never what it is, so it may be shared. `examples/press_cost` — new, and
the benchmark the cache is now answerable to — put sampling a press at 17 to 46 ms against a 14 to
18 ms interpretation of the same page, so a table with the interpretation's lifetime and nothing
behind it would have doubled to quadrupled every page turn of a document whose pages share a press.
Splitting the two keeps 581's fix and costs no memory: the process holds the same eight presses it
always did, and the refusal is now a function of the file.

The press moved into `Compositing` as an `Arc<Press>`, because an index into a table is only cheap
while the table is process-wide and a colour is resolved *per colour*. `Compositing` lost `Copy`,
about forty signatures take it by reference, and its ordering and hashing are written out over a
`PressIdentity` rather than derived.

What it cost: **+0.0045%** of the instructions to interpret an ordinary page (`callgrind_interpret`,
1 216 672 247 → 1 216 726 530), and about 5% of the survey's wall clock, most of which is 27 more
documents being drawn in ink rather than reported. What it bought: the survey over the 287
press-naming crawled documents prints **19 incomplete on every run with every verdict line
byte-identical**, where three runs before printed 45, 46 and 47 with the lines differing — and 19 is
what 581's `MAX_PRESSES = 256` scratch build printed with the bound removed altogether.

581's census was re-run over all 145 archives, twice: byte-identical, and the population is
unchanged — 65 703 documents opening, 2296 stating §11.4.7's condition, 287 naming a press through
an evaluable four-component profile, 28 distinct presses.

## The spec half

§8.9.5.1, a `partial` row whose note is a list of Table 87's entries and which of them are read —
and which carries its own warning that the list has been wrong three times about itself. It had a
fourth hole of the opposite kind: four entries it disposes of neither way. `/Measure` and `/PtData`
are a boundary (their only readers take the dictionary off a *viewport*); `/AF` has a reader and no
caller, so an image `XObject`'s associated files are reachable by nobody and §14.13.7's row
overstates; and `/Intent` is the one that can move a pixel — §8.6.5.9's black point compensation,
which `AbsoluteColorimetric` turns off and which this tree obeys for a path and for a glyph, reaches
**no image sample, shading ramp or mesh vertex by any route**, because those three convert with a
literal `true`. Trap 5's archetype one level along. Condition derived, population measured (0 of the
974, 0 of the 275, 2 of the 65 944), written into §8.9.5.1's and §8.6.5.8's rows rather than built
in a round that was already a refactor.
