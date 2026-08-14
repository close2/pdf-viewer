# 0353 — The verdict nobody measured, and the diagnoses that moved one const up

**Status.** Accepted.

## Context

The oracle's `ambiguous` bucket is 786 pages and every one of them carries a written diagnosis:
`ambiguous_undiagnosed.txt` has been empty for many rounds and the gate holds it to equality in
both directions. `doc/todo/00` therefore describes the standing work as the ratchet plus step 7's
ink sweep.

Every instrument that file has ever had measures **our page**. `Distance::nearest` and
`Distance::furthest` are our distance from the references; step 5's ink is ours against theirs;
step 6's ladder is ours against a limit; step 7's gap is our ink minus the lightest reference's.
`ambiguous` is not a statement about our page at all — it is the statement *no two voting
references agreed* — and nothing printed, ranked or watched by how much they missed.

That is the gap trap 9's fifth shape names and does not close: "shared code does not only
manufacture agreement; it can also manufacture the *absence* of one, and the second is invisible
where the first is at least listed." It was found once, by hand, on nineteen JBIG2 refinement
pages in the hundred-and-seventy-sixth session, and no instrument was built from it.

## Decision

### 1. Rank the bucket by how hard the consensus failed

`Examined::consensus_missed_by` is the smallest `outside_by` over `Triangulation::between_references`
— how far the *closest two voting references* sit outside the bound the page was judged by — and
`rank_the_manufactured_ambiguity` prints the ten largest beside our own distance. On an ambiguous
page it cannot be below 1 by construction, which is what makes its magnitude legible: a little
above 1 is trap 12's arithmetic, and twenty is a renderer that failed.

It changes no verdict, gates nothing and costs nothing: `between_references` was already computed
and already discarded.

**Its head is two JPEG 2000 pages and then the whole of `AMBIGUOUS_SHARED_JBIG2_DECODER`** —
`jp2k-resetprob.pdf` at 35.12 bounds between the closest pair against our 5.03, `issue5475.pdf` at
**31.63** against our **0.00**, then `bitmap-refine-tpgron.pdf` at 28.91 and seven more
`bitmap-*-refine` pages at 28.58. The second half is the instrument reproducing by itself the
finding it was built from: `mupdf` paints that sheet black at ink 255.000 and `ghostscript` paints
it white at 0.000, the full range apart, because `jbig2dec` gives up on a refinement region in two
different ways.

**The first half is new and it is the `objdump` finding below arriving as a measurement.** All
three voting references link the same `libopenjp2.so.7`, so on a `JPXDecode` page they are one
decoder with three callers — and on `issue5475.pdf` those three span 9.03 to 19.08 of 255 among
themselves while ours and `mupdf` are **0.0002 apart over 262 144 pixels**. Shared code
manufacturing the absence of a consensus *without having failed*, which is one step past the JBIG2
pages that named the shape. `tests/jpeg2000.rs` is what settles that page, because it asks
ISO/IEC 15444-5's own software and no renderer.

### 2. What the other ordering's head turned out to mean

Ranking instead by our nearest **over** the closest voting pair puts 56 of the 786 above 1 — pages
where we sit further from every voting reference than the closest two sit from each other, which
step 1 reads as *we are alone*. The head is not an accusation and the reason is worth the ADR:

- `issue4260_reduced.pdf` at 8.27 — §10.7.4 says a zero-area fill's pixel is painted "no matter
  how small the intersection is"; ours and `hayro` put down a full mark at ink 19.79 and 19.83,
  and the closest voting pair, `poppler` and `mupdf` at 1.92, agree about painting a fifth of it.
- `bug1743245.pdf` at 5.34 — the closest voting pair is `mupdf` and `ghostscript` at **4.12**
  where every other pair on the page is 22 to 28, and what those two share is ignoring §10.7.5's
  "the stroke shall be rendered as a single-pixel line".
- `bug766086.pdf` at 2.98 — the same two at 3.03, agreeing about drawing no link border for two
  unrelated reasons, which is trap 9's fourth shape.

**So a high ratio means "the closest two references agree through a gap" at least as often as it
means anything about us**, and that calibration is written into `doc/todo/00` beside the ranking
so the next round does not read the list as a defect queue.

### 3. The bucket is two camps, and the camp that votes is the one that cannot agree with itself

Measured over all 786 pages, all ten renderer pairs, from artefacts already on disk:

| population | closest pair is `ours + hayro` | median ours-to-`hayro` | median closest voting pair |
|---|---|---|---|
| all 786 | **651** | 1.92 | 5.34 |
| the 670 judged as text | **612** | 1.94 | 5.39 |
| the 116 judged as vector | 39 | 0.30 | 2.09 |

`hayro` is a separate interpreter written by other people; it shares `skrifa` with this tree and
not an interpretation, and it is the one reference that may not vote. On nine ambiguous text pages
in ten it is nearer to us than any two of the three voting references are to each other.

**This is not evidence that we are right**, and the ADR says so because the number is the kind
that invites the opposite reading. Agreement with `hayro` is worth nothing under
`Reference::independence`. What the table establishes is what the verdict is *made of*: on a text
page in this bucket the absence of consensus is a property of the references — trap 9's third
shape, three C renderers and one FreeType, measured over a population instead of asserted from one
`ldd` — so `ambiguous` carries no information about our page unless a closed form supplies it.

The instrument's own known defect was ruled out rather than assumed: our panels and `hayro`'s
carry an alpha channel and the C references' do not, which is step 5's `-alpha off` trap and would
manufacture exactly this result. All **4535** panels on disk were tested and not one pixel is less
than fully opaque.

### 4. Three groups' diagnoses had migrated one `const` up, and a test now catches the next

A group is an array of page names with its argument in the doc comment above it. Rust attaches a
doc comment to whatever item follows it, so an edit that inserts a new `const` between an existing
comment and the const it documented welds two notes together and leaves an array with **none**.
Nothing is malformed, so `rustc`, `clippy` and every gate in this tree pass.

It had happened to `AMBIGUOUS_GLYPH_COVERAGE` (3 pages), `AMBIGUOUS_MASKED_BLUR` (1) and
`AMBIGUOUS_OURS_ON_THE_LIMIT` (3). Seven pages whose argument was written down — with ladders,
with clauses — filed above a group it does not describe, while the three groups themselves said
nothing and the bucket counted "0 undiagnosed".

All three are moved back, and `every_group_of_pages_carries_a_diagnosis_naming_one_of_them` reads
this file's own source: for every non-empty `AMBIGUOUS_*` or `CONTRADICTED_*` array, the comment
above it must name at least one document in it. **Deliberately the weakest rule that catches the
whole failure** — a group of 370 pages cannot name them all, and several notes cite a neighbouring
group's page on purpose to say how the two differ, so anything stricter would be a rule this file
has to fight. A welded comment names none of the array under it, because it was written about the
array above it. Checked by breaking it: inserting a new const between a comment and its const
makes the test name the group that lost it.

## Consequences

**Two corrections to written claims, both measured rather than argued.**

`Reference::independence` said `mupdf` and `ghostscript` "share `jbig2dec`, and only that", and
the handover's trap 9 said all three references "link the same `libfreetype.so.6`". `objdump -p`
— what a binary asks for, rather than `ldd`'s transitive closure — says neither. All three link
the same `libjpeg.so.8` and the same `libopenjp2.so.7`, so **on a JPEG or JPEG 2000 page the three
voting references are one decoder**; `poppler` and `ghostscript` share `liblcms2`; and
`ghostscript` links no FreeType at all — `libgs.so.10` defines 194 `FT_*` symbols and leaves none
undefined, so it carries a statically linked copy, configured differently from the system one
(the system library exports `FT_Palette_Select` and Ghostscript's does not). The substance of the
trap survives and is stronger for being measured; the sentence did not.

`AMBIGUOUS_SUBSTITUTED_FACE` said of `bug1671312_ArialNarrow.pdf` that "we are the only renderer
that finds a *narrow* face at all, and the four that do not draw a better-fitting line" — two
halves that cannot both be true, and the true reading is the other one. **Ours is the wide face**:
the ink's bounding box is x[10, 149] y[15, 34] in ours against x[10, 147] y[15, 34] in `poppler`'s
and `mupdf`'s, so the advances and the extent are honoured, and inside that same box we mark 983
pixels against 844, 825, 812 and 702. At 576 dpi the modal dark run across the x-height band is 14
device pixels in ours and 12 in `poppler`'s, where the `/StemV 66` the file states is 10.56. The
four-panel strip says it without a number: our letters collide and the other four do not.

**And that page is the witness `doc/todo/21` item 4 said would open the `/CapHeight` question**,
which it does in one half and not the other. It states a full Table 120 descriptor for a
non-embedded face — but its `/CapHeight 922` is also its `/Ascent 922`, and Arial's cap height is
716, so the number is stated and not usable, which sharpens ADR 0267's condition rather than
retiring it. The *width* metrics are the half that opens: `/StemV`, `/AvgWidth`, `/MaxWidth` and
`/FontBBox` now have a measured cost, and unlike the cap height, reading them would be scaling to
a number the **file** states rather than to where another program's font sits — which is the
distinction ADR 0267 turned on. Recorded in §9.8.1's ledger row and `doc/todo/21`, and not taken:
§9.8.1 states no `shall`, the change is a substitution policy rather than a line, and one witness
is not a population.

**No pixel moved.** The whole diff is a test binary's comments, one new field, one new printed
ranking, one new test, two documents and a ledger row. The oracle's per-page lines are identical.
