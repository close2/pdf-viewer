# Q51 — Should the project buy ISO/IEC 15444-1 and -2, the JPEG 2000 specifications?

Source: `crates/pdf-archive/src/table/graphics.rs`, the seven rows of ISO 19005-2 §6.2.8.3 /
ISO 19005-4 §6.2.7.3, every one of which is `Check::Unchecked` for the same reason.
Status: **open** — answered when `A51-buying-the-jpeg-2000-specifications.md` exists beside this file.

## Why it needs the owner

It is a purchase decision, like `Q49`'s, and it is the only thing that would unblock seven rows of
the PDF/A table at once. Three rounds have now reached the same wall independently, so the wall is
real rather than a want of looking.

**What the clause asks cannot be answered from the PDF objects.** Both parts state seven
requirements about a `JPXDecode` image: the number of colour channels; that at most one colour
space specification carries `0x01` in its `APPROX` field; that the `METH` entry of its `colr` box
is `0x01`, `0x02` or `0x03`; that enumerated colour space 19 (CIEJab) is not used; that the
bit-depth is between 1 and 38 and the same on every channel; that only the JPX baseline feature
set is used; and that a JPEG 2000 image effectively using a device colour space obeys the device
colour rules. Every one of those is a field **inside the JPEG 2000 data**.

The corpus confirms it exactly. `doc/veraPDF-corpus/PDF_A-2b/6.2 Graphics/6.2.8 Images/6.2.8.3
JPEG2000` holds five failing witnesses and two passing ones; each failing witness has the image
dictionary

```
/Subtype /Image /Filter /JPXDecode /BitsPerComponent 8 /Width 640 /Height 480
```

with **no `ColorSpace` entry** — byte for byte the dictionary of `t01-pass-b`, which is expected to
pass. The witnesses differ from their passing sibling only in the JP2 boxes: the `NC` field of the
image header box, how many `colr` boxes there are, the `METH` value, the enumerated colour space
number, and the `bpcc` box. A validator that read only the PDF dictionary would have to pass all
five or fail all seven files, and either would be a lie.

**What is missing is the box and marker layout, not a decoder.** None of the seven rules needs a
single sample decoded. They need to know where the `jp2h` box is, what fields `ihdr`, `colr` and
`bpcc` carry, and what "the JPX baseline set of features" is. That is ISO/IEC 15444-1 (the core
coding system and the JP2 file format, Annex I) and ISO/IEC 15444-2 (the extensions and the JPX
format, whose Annex M.9.2 is where both parts' NOTE 1 says the baseline set is defined).

**Both parts name ISO/IEC 15444-2 normatively**, not merely in a note: ISO 19005-2 §6.2.8.3 and
ISO 19005-4 §6.2.7.3 each close with a `shall` sentence requiring JPEG 2000 images to be created
and read as that document describes. So this is not a case where the reading could be reconstructed
from ISO 32000 — the base standard hands the question over, and `CLAUDE.md` principle 5 forbids
reading the layout out of somebody else's implementation instead.

## The free route was checked, and it is closed

ITU-T publishes most of its Recommendations without charge, and T.800 and T.801 are the same text
as ISO/IEC 15444-1 and -2. They are the exception: the T.800 catalogue page at `itu.int` says of
the in-force edition that *this text was produced through a joint activity with ISO and IEC* and
that *according to the agreement with our partners, this document is only available through
payment*. Checked on 2026-09-07. So there is no Adobe-style free copy the way `Q49` turned out to
have, and no reading of these clauses without buying something.

## What the tree does meanwhile

All seven rows are present, named and reported. Six carry one shared reason —
`JPEG2000_NEEDS_A_CODESTREAM_READER` in `graphics.rs` — and the seventh names the same absence from
the colour side. `doc/questions/Q20`'s discipline means a verdict on a document holding a
`JPXDecode` image says in as many words that these seven were not checked, so nothing is claimed
that was not read.

Nothing else is blocked. A document with no JPEG 2000 image is unaffected, and the corpus's own
`6.2.8.3` directory is seven files out of a few thousand.

## What it would cost, honestly

Two documents, and the second is the larger. Against that: seven rows, five corpus witnesses, and
a class of image that a *conversion* to PDF/A would otherwise have to refuse outright rather than
validate. It is also the last unread normative reference in clause 6.2 — every other subclause
there is now decided from documents this tree holds.

## Recommendation

**Buy ISO/IEC 15444-1 first and stop there for now.** The five witnesses turn on `ihdr`'s `NC`,
`colr`'s `METH`, `APPROX` and `EnumCS`, and `bpcc` — every one of them a JP2 box defined in part 1
Annex I. Four of the seven rows would close on part 1 alone, and a fifth (the device colour rule)
becomes answerable because the colour space the codestream declares becomes readable.

**Defer ISO/IEC 15444-2**, which buys exactly one row: "only the JPX baseline set of features".
That rule has no corpus witness, it is the one that would need real codestream analysis rather
than box walking, and it can stay `Unchecked` with a truthful reason indefinitely.

And **buy neither if PDF/A is not going to a conforming-validator claim**. If the target is the
converter of `Q46` and JPEG 2000 images are to be refused rather than validated, the honest thing
is to say so in `doc/pdf-a-conversion-limits.md` and leave all seven rows named — which is what the
tree does today, at no cost.
