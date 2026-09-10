# 944 — A band is not a dot, and an item that names one part

Date: 2026-09-10.
ADR: 0941 (the item that names one part, and the exemption nobody measured); 0945 (a band is not
a dot).
Files: `crates/pdf-render/src/sub_pixel.rs`, `crates/render-cpu/src/lib.rs`,
`crates/render-cpu/tests/anisotropic_sub_pixel_stroke.rs` (new),
`crates/pdf-model/examples/anisotropic_band_census.rs` (new),
`crates/pdf-archive/examples/unreferenced.rs` (new), `crates/pdf-archive/src/clarification.rs`,
`src/table/fonts.rs`, `tools/conformance/tests/conformance.rs`, `doc/todo/00`, `doc/todo/62` (new),
`doc/habits.md`, `doc/conformance/ledger.toml`.

## Three pages the gate calls complete

Session 943's sweep put nineteen entries past −1. Sixteen are on pages this tree *reports* as
incomplete, where lightness is expected and says nothing. Three are not — this tree drawing less
than every reference and saying nothing about it, which is the shape trap 5 exists for. Two of the
three turned out not to be defects, and saying which is which is most of the work.

**`issue12295.pdf` is the defect and it is ours.** Its ECG strips sit under `diag(0.1366,
−0.0054)`, and §10.7.4's substitution for a sub-pixel stroke was stating the swept band at
`1/min_stretch` — one device pixel across *whichever way the mark runs*. That is §8.5.3.2's dot's
question, not a band's: §8.4.3.2 says a stroke's thickness "shall vary according to their
orientation", and a band along `u` is `w·|u|·|det T|/|T u|` thick. A QRS spike 0.1366 of a pixel
wide was painted across **25 columns** at an alpha of 1.38 levels, of which eight bits hold one —
ink spread onto pixels the clause's opening sentence does not reach, and 28% of it lost in the
alpha floor underneath.

**The page's own note was four ink ladders and a closed form, and none of them can see a mark in
the wrong place at the right weight.** That is trap 1 in one sentence, on a page that had a
diagnosis already written.

Priced both ways over 975 corpus first pages: of 4 010 408 strokes reaching the widening, the old
width gave **51 414** of them a band between 2 and 25 device pixels; the new one leaves a residual
of **two**, both on one document. `raster` never had the defect — its anisotropic route outlines
in path space at the stated width — so its own corpus row moving is an independent check rather
than curve-fitting.

**And the sweep number went the wrong way, which is the honest result.** The step-7 row moves
−2.362 → −3.198, because our ink moved *onto* the geometry: at 8×, where no substitution fires,
both builds agree at 6.8047, and 1× ink falls 8.0147 → 7.1795. `doc/todo/02` §7 says a count that
improves is not a picture; this round is the same sentence inverted, and the ladder is recorded
beside the row so the next reader does not "fix" it back.

The two non-defects are diagnosed rather than left as numbers, which is the difference between a
bucket that shrinks and a bucket that is understood. `issue16038.pdf` states an interior coverage
of 0.13333 through an uncoloured tiling pattern, twice over; ours and hayro are the only columns
on the clause's arithmetic and the only two that give both patterns the same answer, where
ghostscript returns exactly 1/3 and 2/3 — one and two whole pixels of a three-pixel period.
`issue14297.pdf` is a resolution ladder in which the *references* lose ink as pixels arrive while
ours gains 0.09 and lands 0.05 from poppler's limit.

## An item that names one part, and an ordering reversed by measurement

TechNote 0010's A014 was the last unclassified item. It reaches ISO 19005-1 alone, on two
independent arguments — the structural one being that every item whose validator paragraph this
crate acts on carries a TWG proposal heading whose adoption is what carries it into part 2, and
A014 has no proposal heading at all; the substantive one being that its rule is stated against
*the applicable PDF specification*, and both ISO 32000-1 and -2 define `OpenType` as a
`/FontFile3` subtype in full. No row was added, because the residue is a base-standard
requirement this crate has decided in writing not to take on.

**And `doc/todo/62` reverses ADR 0935's ordering, by measuring rather than reasoning.** That ADR
had A010 as a narrowing to apply after section 6.2.2's published exemption. Over 1 552 documents
by six targets: 169 state a named-resource entry nothing references, 147 hold such an object, and
12 fail a row whose every finding is on one — all twelve failing nothing else. Every one of the
twelve fails under a clause the applicable carve-out **keeps**. So the exemption as it stands
costs the corpus nothing, and part 2's published sentence *without* A010 would withdraw seven
failures this crate and the corpus agree on. A010 is what makes the exemption safe.

## A citation that was wrong and passed its gate

`fonts.rs` cited §9.9's Table 128 for the three font-file keys. Table 128 is *Entries in a Type 1
halftone dictionary*. The checker passed it because it asks whether a cited table exists, not
whether it is the one the sentence is about — and that weaker rule is correct: the test's own
documentation records the stronger one being measured and rejected, failing 14 of 25 references
with all fourteen correct writing.

What the checker owes instead is a listing a person can read in one glance, and it was printing
titles without saying who cited them. It now prints the citing file beneath each title.

`doc/habits.md` gained the two instrument failures this week produced: a gate read through a
filter that hides its answer, and a stale test binary in the shared build directory answering for
a tree that no longer exists — four agents, none of whom suspected it first, because its failures
look like findings.
