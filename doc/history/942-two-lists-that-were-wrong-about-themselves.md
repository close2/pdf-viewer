# 942 — Two lists that were wrong about themselves

Date: 2026-09-10.
ADR: 0933 (a clarification reaches the parts it names and no others); 0934 (what was left of the
matrix that cancels the page).
Question: `doc/questions/Q53` — the basic types A020 lists and the two it does not.
Files: `crates/pdf-archive/src/clarification.rs`, `src/survey.rs`, `src/table/graphics.rs`,
`src/table/metadata.rs`, `crates/pdf-model/src/annotation.rs`, `src/appearance.rs`,
`src/content/annotations.rs`, `crates/pdf-model/examples/fixed_print_census.rs` (new),
`doc/conformance/ledger.toml`, `doc/rfc/0004`, `doc/todo/25` (**done**).

Both tracks: the spec-driven half took TechNote 0010's owed items, the demand-driven half closed
`doc/todo/25`'s last remainder.

## A row that bound one part because only one part said it

TechNote 0010's A002 resolves that ISO 19005-2 and -3 are read as if section 6.2.2 carried a
sentence it does not: that the explicitly associated `Resources` dictionary shall **define** all
named resources the content stream references. Part 4 states that in its own words, so
`graphics/named-resources-are-defined` bound part 4 alone — and a PDF/A-2 file naming a resource
nothing defines was passing.

**Session 941 had A002 filed under "already true".** That list was written by the round that
introduced the whole category, one day earlier, and it was wrong about one of its own entries. The
row's neighbour was wrong in the same direction: `survey.rs`'s `MissingResource` documentation
said only the part 4 row reads it.

The corpus cannot rank this. Every witness that exercises the rule sits under `PDF_A-4`, so the
sweep does not move at all and a test pins it instead — the third time this week that a real
defect has been invisible to the instrument that would normally find it.

**A026 changes nothing, and the reason is worth more than the change would have been.** Session
941 flagged it as the likeliest false-failure, `DeviceGray` in soft-mask images. It cannot be: the
survey records an image's colour space only for an image a content stream *draws*, and never
descends into an `SMask`, so no soft-mask image's colour space has ever reached the device-colour
rule. That reading is written at the rule and at the walk's own boundary, because it binds any
later round that widens the walk.

And using ADR 0931 exposed a hole in it. Its fourth condition is that an item names the parts it
reaches — but the lookup was by row id alone, so a row stating one rule in both parts printed a
resolution naming parts 2 and 3 underneath a **PDF/A-4** verdict. Nothing false was said; a
resolution printed beside a verdict reads as its ground. ADR 0933.

## The sentence that was smaller than it read

§12.5.6.22's `/FixedPrint` needed one term derived: "a matrix B that maps a scaled and rotated
page into the default user space". The paragraphs around it introduce themselves as being about
situations other than the usual case where the page size equals the media size — tiling, n-up. But
the clause's own on-screen sentence makes the page's media box *be* the media. **So a screen is
that usual case by construction**: B's scale and rotation are the identity by stipulation, and
what is left is the translation between two origins.

One genuine choice, recorded as one: `/Rotate` is not cancelled, because §12.5.3's `NoRotate`
exists to depart from turning with the page and reading a second such mechanism into the B
sentence would make that flag redundant on this one subtype.

**The population was counted.** Of 4 172 documents in this tree that open, five state a watermark
annotation and **one** states a `/FixedPrint`. The gate corpus states none; all four
`doc/corpora` submodules state none. That witness checks the arithmetic and cannot rank it — its
rectangle, matrix and percentages transform to exactly its own `/Rect`, so all four terms must be
right for the mark not to move, and the rendered pages are byte-identical either way. The fixtures
are therefore hand-built, they rasterise and measure rather than asserting a matrix, and they are
**calibrated**: planting the computation back to nothing fails three of the four, and the fourth
stays green because that is the document's own property.

## What this round is really about

Both halves found the same shape. A list of clarifications said an item was already true and it
was not; a `Resources` doc comment said which part reads it and named the wrong one; a ledger row
said `/FixedPrint` is reported rather than applied after the round that would apply it; an RFC
bullet said the screen case is still only reported. None of the four was found by a gate, and
three of them could not have been — no corpus document ranks them.

`doc/todo/02` §1 asks for both tracks every round because one alone finishes when the corpus goes
quiet. This round is the argument for the other half of that sentence: **the corpus was already
quiet on everything it found.**
