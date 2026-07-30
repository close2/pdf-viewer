# ADR 0034 — What a sample means, and the family read around it

Status: accepted, 2026-07-30.

## Context

The ledger carried §8.9.5.2 as `partial` for fourteen sessions with one sentence of debt beside
it: *only the fully-inverted form `[1 0]` is applied, and an arbitrary linear map is silently
ignored.* `doc/HANDOVER.md` listed it under "what is not implemented" as **Small**, sized `—`,
with the note that "reporting it would be a good first move".

That estimate was wrong in both directions, and finding out which way took one measurement.

The corpus was walked — every image dictionary of all 974 documents, not only the ones page one
draws — for what `/Decode` arrays real files actually write:

| array | where | image objects |
|---|---|---|
| `[1 0]` | a stencil mask | 115 |
| `[1 0]` | `DeviceGray` | 10 |
| `[0 1]` | `DeviceGray` | 5 |
| `[0 255]` | `Indexed` | 4 |
| `[255 0]` | `Indexed` | 1 |
| `[0 1 0 1 0 1]` | `DeviceRGB` | 2 |
| `[1 0 1 0 1 0]` | `DeviceRGB` | 1 |
| `[1 0]` | `Separation` | 2 |

That is 140 image `XObject`s; inline images are inside content streams and are not in the
scan, and a second run — an `eprintln!` on the decode path while the corpus gate renders every
page one — shows them writing the same two forms and nothing else.

**Not one general pair in 974 documents.** Every array any of them writes is Table 88's own
default or its exact reversal, which is trap 8's shape exactly: the corpus states what documents
contain, and the standard states what a valid file may contain. So the general map is a rule no
corpus run can rank, and the report the handover suggested would have named nothing.

And in the other direction, three things the row's one sentence did not say:

- **The `DCTDecode` route never consulted the entry at all.** `zune-jpeg` delivers components
  rather than packed samples, so that route bypasses the unpacker where `/Decode` was read.
  `issue7406.pdf` writes `[1 0 1 0 1 0]` on a JPEG whose samples are stored inverted; its page
  one drew the pdf.js logo cyan on black against all four reference renderers' red on white.
  A page, visibly wrong, for the whole life of the project.
- **Table 88's `Lab` row is not the unit interval.** A `Lab` image's default decode is
  `[0 100 a min a max b min b max]`, and the unpacker scaled every space's sample onto 0.0 to
  1.0 with a comment saying that "is 0.0 to 1.0 for every space that gets here". `to_rgb` has
  taken real `Lab` values since ADR 0012, so the brightest colour such an image could hold was
  L = 1 — black to within a level.
- **The clause has a closing sentence.** "If an output value is not permitted for a component,
  it shall be adjusted to the nearest allowed value", which nothing did, on a map whose two
  numbers the clause explicitly permits to leave the component's range.

The other half of the session was §8.6.5, the CIE-based colour spaces: nine subclauses of which
one was reviewed and five were named in `REVIEW_OWED`. `/Decode`'s defaults are stated per
colour space, so the two halves are one reading.

## Decision

### The map is a table, not a formula

§8.9.5.2 is

> D min + x × (D max − D min) ÷ (2^n − 1)

evaluated per component per sample. Every input it takes but `x` is fixed by the image
dictionary before a byte is read, and `x` ranges over at most 2^n values — 2 or 256 here, since
the unpacker refuses 2, 4 and 16 bits. So `Decode` holds one lookup table per component, built
once, and the unpacker's arms became a lookup that no longer knows what a `/Decode` array is.

`palette` — the existing optimisation that converts each of a one-component space's possible
colours once instead of once per sample — falls out of the same shape and got *shorter*: it
had been reconstructing the inversion itself.

There are two tables per component, not one, and the second is an optimisation with a number
attached. A device space's components *are* eight-bit channels, so the arms producing one would
otherwise clamp, scale and round per pixel. Interpreting `issue19971.pdf`'s 2500×1364 `DeviceRGB`
photograph, by callgrind:

| | instructions |
|---|---|
| before this change | 161.54 G |
| quantising per pixel | 166.35 G |
| with the byte table | 162.40 G |

So the clause costs **+0.54%** on the corpus's largest image with the table and +2.98% without
it, and nothing measurable on a page carrying no image (+0.0004% on ISO 32000-2's page 101).

### The formula multiplies before it divides

An `Indexed` space's default pair is `[0 2^n − 1]`, which NOTE 2 says exists so that "component
values that index a colour table are passed through unchanged". Written as `x ÷ 255 × 255` in
`f32` that is not the identity — sample 254 comes back as 253.99998 — so the table is computed
as `((D max − D min) × x + D min × span) ÷ span`, which is exact on every pair whose arithmetic
is exact.

### Table 88 is read from the colour space, not assumed

`ColourSpace::default_decode` answers per family, and it has exactly two interesting rows:
`Indexed` gives `[0 2^n − 1]`, `Lab` gives its `/Range`, and everything else is the component's
own range. The clamp uses `component_range`, which already existed for §8.6.6.3's colour tables
and now has a second caller — the same function answering the same question about a *range*
rather than a value.

### `/Decode` on a JPEG is applied to its channels

The `DCTDecode` route applies the map after decoding, over channels rather than samples, which
is the same arithmetic on the same domain: a JPEG component is an integer in 0 to 255. It costs
nothing where the array is absent or the identity, which is every corpus document but one.

## §8.6.5, and what reading it found

Nine subclauses; §8.6.5.4 was already `implemented`. The other eight are now recorded, and three
of them found something.

**§8.6.5.1 was a refusal where the standard states an answer.** "A PDF reader shall ignore
CalCMYK colour space attributes and render colours specified in this family as if they had been
specified using DeviceCMYK" — a family withdrawn before it was ever completed, whose whole
remaining content is that sentence. This tree reported an unsupported colour space. It is one
match arm, no corpus document writes one, and the clause is unambiguous.

**§8.6.5.5 is trap 6 one level up, and it is measured.** A *fill* in an `ICCBased` space is
converted through the profile by the A2B evaluator; an *image* in the same space is not —
`image.rs` reduces the space to `DeviceGray`, `DeviceRGB` or `DeviceCMYK` by its `/N` and
unpacks it as a device space, under a comment calling that "an approximation, and the honest
one". It is honest about being an approximation and silent about its size: **31 corpus documents
carry 1037 `ICCBased` images, and on 15 of them the profile's own answer for a mid grey differs
from the device passthrough by 18 levels out of 255.** `colour_paths.rs` exists because three
`DeviceCMYK` conversions once disagreed; this is the same defect with an image on one side of
it, and it is left open rather than closed here because closing it is a question about where a
per-sample conversion is affordable, not about the clause.

**§8.6.5.6 is implemented for everything except images.** The clause says the remapping applies
to "a colour space given as an entry in an image XObject, inline image, or shading dictionary",
and an image's `/ColorSpace` is parsed against an *empty* resource dictionary, so a device space
named there is never replaced by `/DefaultRGB`. One corpus document names a default at all and
its images state their space directly, so no page changes — but the requirement is
unconditional, and it lands in the same function as §8.6.5.5's gap.

§8.6.5.7 is `inapplicable` and worth the minute it cost: it is a permission to *skip* a
conversion where the source space matches the output device, and its own NOTE 3 says the
conditions "cannot be specified in PDF". §8.6.5.2, §8.6.5.3, §8.6.5.8 and §8.6.5.9 were read
and found to say what the code already does, with one detail worth keeping: §8.6.5.9 spells the
intent `AbsColorimetric` where Table 69 spells it `AbsoluteColorimetric`, and the code tests for
the name a file may actually contain.

## Consequences

`issue7406.pdf` page one moved from a mean distance of 17.36 to 5.02 against the reference
consensus, worst tile 71.85 to 12.77, SSIM 0.7852 to 0.9237 — and its oracle verdict did not
change, because the page is text-heavy, the references disagree among themselves about the text,
and it is `ambiguous` either way. **A page can be visibly wrong inside a class the gate cannot
fail on**, which is the second time this project has found a defect the oracle could see and not
report.

Neither gate's counts moved: 823 documents draw with nothing reported, 814 pages agree, 102 are
contradicted. The ledger's unreviewed count fell 475 → 466 and `REVIEW_OWED` 16 → 11.

The alternative considered and rejected was the handover's own suggestion — *report* a general
`/Decode` array rather than implement it. Trap 11 costs a report in gated pages, and this one
would have cost nothing at all, because it would have fired on no corpus document. A report
that can never fire is not honesty; it is a comment in an expensive place. The three things a
report would have hidden are exactly the three the implementation found.
