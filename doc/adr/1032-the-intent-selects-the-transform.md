# ADR 1032 — §8.6.5.8's rendering intent selects the profile's transform

## Status

Accepted.

## Context

ISO 32000-2 Table 69 names four rendering intents and §8.6.5.8 gives an object three routes to
one: the `ri` operator, an `/ExtGState`'s `/RI`, and §8.9.5.1 Table 87's `/Intent` on an image
dictionary. All three routes have been read here since the six-hundred-and-seventh session, and
the four names have been kept apart since then rather than collapsed.

**One of the four did something and three did nothing.** `AbsoluteColorimetric` turns black point
compensation off, which §8.6.5.9 requires of it. `RelativeColorimetric`, `Saturation` and
`Perceptual` were carried the length of `crate::colour` and consumed nowhere, because
`crate::icc::Profile` parsed a profile's `A2B1` — or `A2B0` where there was no `A2B1` — and
evaluated that one table whatever any intent said. So a page asking for a perceptual rendering got
a colorimetric one, silently, with three ledger rows recording it: §8.6.5.8, §8.9.5.1 and
§11.7.5.3.

The question that had never been asked is where a reader is supposed to find out what
`Perceptual` *means* for a given colour. Table 69's own answer is a sentence about appearance —
"[c]olours shall be represented in a manner that provides a pleasing perceptual appearance" — which
states a goal and no arithmetic, and it is tempting to record that as a silence the specification
leaves (`CLAUDE.md` principle 5's standing shape). It is not one. Two clauses answer it between
them:

- §8.6.5.8 says where the names came from: "[t]hese intents have been chosen to correspond to
  those defined by the International Color Consortium (ICC), an industry organisation that has
  developed standards for device-independent colour."
- §10.3.1 says whose arithmetic performs the conversion: "[c]onversion from a CIE-based source
  colour to a CIE-based destination colour shall be performed based on ISO 15076-1:2010
  (ICC.1:2010)."

And §10.4.2.1 ranks that route above the classic conversions for a processor that can take it —
this tree's standing branch, already recorded in its own row. So the intent is not an appearance
this reader has to invent: it is an index into the profile, and ISO 15076-1:2010's tag listing
gives `A2B0`, `A2B1` and `A2B2` the device-to-connection transform of one intent each.

## Decision

**§8.6.5.8's intent selects which of a profile's "to CIE" transforms a colour goes through**, by
this mapping, which a later round should not re-derive:

| Table 69's name | the transform | why |
|---|---|---|
| `Perceptual` | `A2B0` | ISO 15076-1's perceptual table |
| `RelativeColorimetric` | `A2B1` | its media-relative colorimetric table |
| `AbsoluteColorimetric` | `A2B1` | **there is no fourth tag**: the standard derives this intent from the media-relative one rather than tabulating it |
| `Saturation` | `A2B2` | its saturation table |

Four consequences are part of the decision rather than of its implementation.

**An intent whose table the profile does not state falls back to the colorimetric one.** §8.6.5.8
disposes of an intent a *processor* cannot honour by sending it to `RelativeColorimetric`, and a
profile that tabulates no transform for the intent asked leaves a processor in exactly that
position; ISO 15076-1:2010 requires `A2B1` of an output profile, so the fallback is a table the
file has. A profile whose `A2B0` and `A2B1` are one tag is one table under two names and is not
parsed twice.

**The two alternates are parsed on first use, not when the profile is.** `CLAUDE.md` principle 2's
"nothing eager": a document naming no intent of its own must not pay a CLUT parse for two tables
it will never read, nor hold them. What is kept instead is the tag's bytes, a quarter of the size
of the parsed table.

**The parameter travels as a pair.** `crate::icc::Rendering` carries the transform beside
§8.6.5.9's black point, in one value, and `crate::colour::Conversion` carries that. The reason is
the one that type already had written on it: the six-hundred-and-seventh session made the black
point impossible to omit from a `paint` call by pairing it with the compositing target, after it
had been omitted from every call in `crate::image`, `crate::shading` and `crate::mesh`. A second
parameter that could be omitted independently would be the same defect waiting.

**What is deliberately not done, and is therefore owed rather than decided:**

- `AbsoluteColorimetric`'s white point. Table 69 says "no correction shall be made for the output
  medium's white point (such as the colour of unprinted paper)"; ISO 15076-1:2010 derives the
  intent from the media-relative transform through the profile's `wtpt`, and nothing here reads
  that tag. So an absolute intent is the colorimetric table with compensation off — §8.6.5.9's
  half of the name — and media-relative at the highlight end. §8.6.5.8's row is what says so.
- The `B2A` direction. The conversion *into* a four-component `ICCBased` blending space still
  takes `B2A1`-else-`B2A0` whatever the intent says (ADR 0796).
- §11.7.5.3's second bullet, the conversion *out* of a group at the `Do`. `colour::sample_press`
  samples the grid once per profile and caches it across interpretations, keyed on the profile
  alone; selecting it by the `Do`'s intent means keying the press on the intent too.

## Consequences

A `Perceptual` or `Saturation` page over a profile with a distinct table now renders differently,
and that is the point. The corpus cannot rank it — 0 of the 974 documents under `doc/pdf.js` name
`AbsoluteColorimetric` at all, on the cleartext grep §8.9.5.1's row records — which is `CLAUDE.md`'s
two-denominator rule doing its work: coverage is the specification's question and a corpus count
cannot answer it. The fixtures are hand-built for that reason (trap 8): one ICC profile whose three
`A2B` tables answer a mid-tone red, neutral and blue, so a test reads which table was selected and
nothing else. Calibrated against the defect (trap 13): with the selection disabled the four
selection tests fail and the two that pin what did not move still pass.

A `Conversion` is now distinguished by its intent as well as by its black point, so
`image::RasterCache` and `shading::Cache` hold one entry per intent for one stream. That is
correct and was already true of the black point.
