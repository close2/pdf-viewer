# 844 — The bound is not what varies; the consensus is

Date: 2026-09-01. ADR **0771**. An oracle round on `doc/todo/12`, whose question is now answered.

Touched: `crates/pdf-model/tests/oracle.rs`, `tools/pdfref/src/lib.rs` (doc comments only),
`doc/conformance/ledger.toml` (§9.8.3), `doc/todo/12-one-bound-two-jobs.md`,
`doc/traps/oracle-and-references.md`, `doc/adr/0771-…`. **No rendering code, no pixel, no
verdict**: the oracle prints 980 agrees / 60 contradicted / 836 ambiguous before and after.

## What the round was asked, and what it found

Is a consensus formed by the pair that shares a glyph rasteriser weaker evidence on text pages
than the current rule credits, and should a conviction resting on it alone be a different verdict
or a wider bound? Both branches were followed to a measurement and both close.

**The wider bound.** ADR 0243 left the floor where it was because it could only be derived from a
pair including a non-hinting renderer with neither member ours, and `hayro` shares `skrifa` with
this tree. That requirement rested on the `ldd` trap 9 corrected in session 656: `objdump -p` says
`libpoppler.so` and `libmupdf.so` name one `libfreetype.so.6` while `libgs.so.10` names none and
carries 194 `FT_*` symbols of its own. So `ghostscript` against either is a pair whose FreeType
copies are separate and neither member is ours, and the derivation instrument had been averaging
across that boundary because `PairKind` had two variants where the machine has three. Split three
ways, the text-page differing fraction runs median 0.86% inside the sharing pair and 2.50% across
it — while the class's other three measures do not move across it at all, and on vector pages the
boundary does nothing and points the other way.

The floor was then derived at ADR 0243's own rule, the 99th percentile, giving **12.0372%**;
implemented in `conclude` for our judgement alone with consensus formation untouched; and run.
**1017 / 24 / 835.** Thirty-six pages leave `contradicted` and none arrives — and **six of the
thirty-six are why it was not taken**: `CONTRADICTED_CALRGB_TO_SCREEN`'s five and
`CONTRADICTED_SUBPIXEL_IMAGE`'s one, a §8.6.5.3 colour reading and a §10.7.4 departure, each
measured in its own note. A differing fraction is a threshold count, so a sub-pixel phase on every
glyph edge and a small colour error over a large area reach the same 5–12%. **A bound cannot
separate what a mechanism separates**, and that — not caution — is why the two jobs stay one
number.

**The different verdict.** The candidate rule was trap 12's own control: where the consensus would
contradict the voting reference it excludes, the bound is not one an independent implementation
meets. Measured over the whole pool, it holds on **52 of the 60** contradicted pages, across the
JBIG2 pages, the colour pages and the link border alike. ADR 0717's *32 of 32* is the pool's base
rate rather than that population's signature, and a rule resting on it would acquit us wherever
two references agree for any reason at all. The gate counts it every run now, from numbers it
already had.

**And `widened_to`'s standing request has an instrument instead of a missing renderer.** It asked
for a fourth independent rasteriser; the question a verdict asks does not need one, because
`decide` takes the *closest* pair in the room, so the bound is a selected minimum and what a third
implementation owes is to sit as near that pair as the excluded one of three manages.
`substitutions_of` runs the gate's own judgement with a reference standing where our render
stands, over the corpus, beside our own verdict from the same pair on the same page. On text
pages the `poppler` + `mupdf` consensus contradicts `ghostscript` on 9.1% of what it judges and us
on 5.1%; `mupdf` + `ghostscript` contradicts `poppler` on 3.4% and us on 2.4%; **`poppler` +
`ghostscript`, the one pair whose members do not share the object, contradicts `mupdf` on 0.6% and
us on 0.9%.** Same number, same class, same corpus, fifteenfold difference in the instrument's
error rate.

## What was checked before it was believed

The substitution instrument reproduces the gate's own verdict on `franz_2.pdf` exactly, including
which measure and which pair. The gate's 52-of-60 was taken first by an independent Python loop
over `compare_rasters` on the artefact directories under a stricter reading, which gave 48 of 59.
Seven `CONTRADICTED_GLYPH_EDGES` pages chosen off the verdict list rather than off its note were
measured pair by pair and reproduce the group in all three of its instruments — ours nearer the
convicting pair than `ghostscript` is on all seven, ink within 0.43 of 255. The linkage claim was
re-run rather than inherited.

## Second track

§9.8.3's ledger note said `partial` for `/Lang` **and** `/CIDSet` and deferred to §9.8.3.1, which
had withdrawn `/CIDSet` three rounds earlier — a parent as stale as its deferral, which is
`doc/todo/01`'s fifth failure shape and is the very shape §9.8.3's own note names two sentences
above. `partial` for `/Lang` alone now, with `/CIDSet`'s exculpation written where the parent
states it: a subset's membership is stated by the embedded program itself, a non-embedded CIDFont
has no subset, and Table 122 names the alternative indication.

## Gates

The whole §2 sequence, green, run alone. Sweeps run before it: `overtaken`, `blockers`,
`pointers`, `overstated`, `quotations`, `quoted`, `unpriced` — `unpriced` clean at 89 of 89
bounds named, nothing new from this round's notes. §5's binaries rebuilt and installed.

## What is left, and it is smaller than the item was

`doc/todo/12` keeps three things: the consensus half and its 278 pages, untouched and still a
programme; the **three** pages on which a reference outside the consensus meets the bound while we
do not (`bug847420.pdf`, `issue19633.pdf`, `issue7891_bc1.pdf`), which is the sharpest population
the oracle produces and has never been read as one; and the substitution table's vector row, which
nobody went looking for — the `mupdf` + `ghostscript` consensus contradicts `poppler` on 119 of
226 vector pages against 13 of us.
