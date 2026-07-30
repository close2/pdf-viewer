# ADR 0042 — The clause that said what we said it did not

Status: accepted, 2026-07-30.

## Context

Clause 10 was 30 `unreviewed` rows of 36, one of them (`10.3.1`) on `REVIEW_OWED`, and it
looked like the cheapest family left: halftones and transfer functions describe a marking
device, so most of it should have been `inapplicable` at a minute a row. The demand item beside
it was the last one-document entry on the not-implemented list, §8.4.5's Table 57 `/Font`.

The cheap family review found the most consequential thing this project has been wrong about.

## §10.4.2.5 exists

`CMYK_CORNERS`'s doc comment opened, and this project's handover and `CLAUDE.md` both repeated,
that **ISO 32000-2 defines no `DeviceCMYK` to RGB conversion at all** — the standing example of
"where the specification genuinely defines nothing". The evidence was §8.6.4.4, which says of
the components only that they "shall represent the concentrations of these process colourants".

That is true of §8.6.4.4 and false of the standard. §10.4.2.5, "Conversion from DeviceCMYK to
DeviceRGB", states one outright: each additive component is one minus the sum of its
complementary ink and the black, clamped at one. It is exactly the formula the comment
dismissed as "naive" — "an answer to a question about additive light, asked of subtractive ink"
— without noticing whose formula it is.

**The code was right and its justification was wrong**, which is a worse state than the reverse:
a departure nobody knows is a departure cannot be revisited.

### What the standard actually does is rank two answers

§10.4.2.1 is the sentence that had never been read:

> Although ICC enabled PDF processors should always follow the provisions and recommendations
> provided in 10.3, "CIE-Based colour to device colour", a less-capable PDF processor may
> choose to use the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5.
> These algorithms are, however, very simple and as perceived by a human viewer they produce
> only crude approximations of the original colours.

So there are two branches and the standard prefers one. This tree is on it: `/DefaultCMYK`
(§8.6.5.6), an output intent's `/DestOutputProfile` (§14.11.5) and an `ICCBased` space are all
implemented and all outrank the table. What was missing was the clause that licenses the table
*itself*, and it is §10.3.2:

> A PDF processor should establish CIE-based colour specifications for device colour spaces (
> DeviceGray , DeviceRGB , or DeviceCMYK ), and thus implicitly remap device colour spaces into
> CIEbased colour spaces, when those device colour spaces do not match that of the raster
> output device.

A display's native space is not CMYK (§10.2), so the remapping is asked of us; §10.3.1 puts the
choice of destination "beyond the scope of this document" and its NOTE lists "assumptions made
by the PDF processor software" among the ways it may be made. Assuming standard process inks is
such an assumption, made in the one place the clause leaves for it.

### And the crude answer was measured rather than dismissed

Over the whole oracle, replacing the sixteen-corner table with §10.4.2.5's formula moves **802
agreeing and 88 contradicted pages to 800 and 90**. The standard's own lower branch is worse
here than its higher one, which is what §10.4.2.1 says to expect of it.

## The demand item: a font with no name

§8.4.5's Table 57 `/Font` is "[ font size ], where font shall be an indirect reference to a font
dictionary … however, the first operand shall be an indirect object reference instead of a
resource name". This crate's font cache was keyed by resource name, so there was nowhere to put
a font that has none, and `extgstate.pdf` — whose page reads "I should be courier!" — reported
that it could not address the font rather than drawing it. It was the last one-document entry on
the not-implemented list, and it had been there since the twenty-fourth session put it there.

`FontKey` is now `Named(String)` or `Referenced(ObjectId)`, which is the smallest honest
change: a name and an object identity are different keys, and a document reaching one font both
ways loads it twice, for one parse.

## Decision

- **`CMYK_CORNERS` stays, and its argument is rebuilt from §10.3.2 and §10.4.2.1** rather than
  from a silence that is not there.
- **Table 57's `/Font` is applied**, with the font cache keyed by either kind of identity.
- **The whole of clause 10 is reviewed** — 30 rows, of which 19 are `inapplicable` because
  halftoning and transfer functions describe a marking device, 1 is `reported` (§10.8.3's
  separation simulation, which a document cannot ask for), and the rest record the branch.

## Consequences

| | before | after |
|---|---|---|
| corpus documents drawing with nothing reported | 847 | **848** |
| agreeing with the reference consensus | 801 | **802** |
| ledger subclauses nobody has read | 399 | **369** |
| ledger rows that are `inapplicable` | 16 | **33** |
| cited clauses still owing a review | 3 | **2** |

**`CLAUDE.md` still names `DeviceCMYK` → RGB as the standing example of a case the standard
does not define, and that sentence is now wrong.** It is the project owner's file and is left
untouched here; the correction is recorded in `doc/HANDOVER.md`, on `CMYK_CORNERS`, and in the
ledger's §10.4.2.5 row. A better standing example is available from this same review: **how a
fractional page becomes a whole number of pixels**, which `CLAUDE.md` already names beside it
and which no clause of the standard addresses.

The lesson is the sharper form of one this project already had. "The clause says nothing" and
"the clause says the opposite" are different findings, and the twenty-fifth session learned it
about §10.7.4. This is the third time a claim of silence has turned out to be a clause four
subclauses away from one the tree cites constantly — and the second time it was in clause 10.
