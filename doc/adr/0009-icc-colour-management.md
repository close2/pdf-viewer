# ADR 0009 — Colour is resolved from the document, and only guessed as a last resort

Status: accepted, 2026-07-26.

## Context

The project owner reported having had "too many problems with CMYK and RGB". Looking for
the cause found three separate `DeviceCMYK` → RGB conversions in this tree, in `colour.rs`,
`content.rs` and `image.rs`, which disagreed with each other. `0.5 0 0 0.5 k` produced a
red channel of 0.25; the identical colour set through `scn` produced 0.0; a CMYK image
produced a third answer.

Nothing about a rendered page reveals that. Each drawing looks like a plausible colour.
They are simply not the *same* colour, and which one a document got depended on how its
producer happened to write it.

That is the shape of the whole problem. Colour errors do not announce themselves — a page
in the wrong colours is still a page — so they survive until someone compares against
something. This ADR is mostly about making sure there is something to compare against.

## The specification defines less than you would hope, and more than we were using

Reading ISO 32000-2 rather than assuming, the position is:

**There is no normative `DeviceCMYK` → RGB conversion.** §8.6.4.4 defines the components as
"concentrations of process colourants" and stops. §8.6.5.7 NOTE 3 says outright that
nothing in PDF describes the output device's calibration. The question has no abstract
answer: what a CMYK colour looks like is a property of a press.

**But the specification says where to ask.** Three mechanisms, and we were honouring only
one of them:

1. `/DefaultGray`, `/DefaultRGB`, `/DefaultCMYK` in the resources' `/ColorSpace`
   dictionary. §8.6.5.6: if present, the entry's value **shall** be used in place of the
   device space. This is normative and was being ignored entirely.
2. An output intent's `/DestOutputProfile` — §14.11.5, "an ICC profile stream defining the
   transformation from the PDF document's source colours to output device colourants". Also
   ignored. 17 documents of the 974-document corpus carry one.
3. An `ICCBased` colour space, whose embedded profile we were discarding in favour of its
   `/Alternate`. 303 of 974 documents embed at least one; there are 1270 `ICCBased` streams
   across the corpus.

`/DefaultCMYK` outranks an output intent because §8.6.5.6 says "shall" about the current
operation while §8.6.5.7 says an output intent "can suggest" a calibration for the
document. The nearer and stronger statement wins.

**Black point compensation is a specified concept, not a hack.** §8.6.5.9 defines
`/UseBlackPtComp` with values `ON`, `OFF` and `Default`, where `ON` means "according to the
provisions in ISO 18619" and `Default` — the initial value — is "left to the PDF processor
to determine". So compensating by default is a choice the specification explicitly delegates
to us, and `AbsoluteColorimetric` must switch it off.

## Decision

**One conversion, reached by every route.** `ColourSpace::to_rgb` is the only place a
colour becomes RGB. Fills, strokes, images, shadings and mesh vertices all go through it.
The test that guards this drives one CMYK value through the `k` operator, through `scn` in
an explicit `DeviceCMYK` space, and through the samples of a CMYK image, and asserts all
three agree. It was verified to fail when the old per-operator formula is restored.

**The A2B evaluator is written here rather than linked.** `icc.rs` parses `mft1`, `mft2` and
`mAB ` tags, the `curv` and `para` curve types, and evaluates the CLUT by multilinear
interpolation. Reasons, in order: principle 3 forbids C dependencies on the parsing path
without written justification, and an ICC profile is untrusted input arriving straight from
a PDF stream; principle 2 means not paying Little CMS's initialisation on the launch path;
principle 4 means a student can read what a profile actually does. It is about 700 lines.

**The fallback is documented as a choice, because that is what it is.** When a document
names no press at all, `CMYK_CORNERS` assumes standard process inks at their published sRGB
appearances and interpolates multilinearly between the sixteen corners. This is not derived
from anything and its doc comment says so. What it replaced — `1 - min(1, c + k)` — is
worse than a coarse approximation: it renders process magenta as `#FF00FF`, answering a
question about additive light that was asked about subtractive ink.

## On evidence and authority

Per principle 5, other renderers are evidence about our reading of the specification and
never the definition of correct. This work is where that rule was written down, because it
is where the temptation is strongest: colour has no self-evident right answer on screen, so
"matches poppler" is seductive in a way that "matches poppler" about, say, xref parsing is
not.

Three things came out of applying it:

- The tests that pinned the ICC evaluator against another reader's output were replaced by
  tests that construct a profile whose correct output follows from the ICC encoding alone.
  The corpus-profile comparison is kept, renamed to say it is corroboration, and its comment
  now states that if it ever disagrees the profile's own tables decide who is wrong.
- Writing those tests **found a bug**. The XYZ decoding test uses a degenerate profile whose
  darkest colour equals its white point; black point compensation divided by a span of
  floating-point noise and turned white into pure green. The guard asked whether the
  detected black was non-zero when it should have asked whether it was actually darker than
  white on every axis. No comparison against another renderer would have surfaced that,
  because no real profile is shaped that way — only a fixture built to isolate one rule is.
- The `DeviceGray`/`DeviceRGB` pass-through gained an actual justification. §8.6.4.3 defines
  a component as the intensity of one of the *device's own* primaries, and §8.6.5.7 NOTE 3
  says PDF carries nothing describing that device's calibration; applying any curve would
  assert a calibration the specification says is not in the file. Previously the comment
  said only that three other renderers do it this way, which is true and explains nothing.

The black point compensation work also turned up a source already sitting in `doc/`:
PDF 2.0 Application Note 001, written by ISO 32000's co-project-leader, which defines
compensation as "aligning the darkest colour that could be described by the colour space of
the data to be displayed with the darkest colour that the output profile for the display
device can produce". That sentence settles the design question the arithmetic cannot — the
black to align is the *source* space's, which is why it is found by pushing full ink through
the profile rather than by round-tripping its `B2A` table as Little CMS does. The two
constructions agree except in the darkest few percent.

## Consequences

`DeviceCMYK` now means what the document says it means whenever the document says anything.
`DeviceN`, `Separation`, `Indexed` and `Lab` resolve through the same single path, so a
tint transform landing in CMYK gets the same treatment as a direct CMYK fill.

ISO 18619 remains a normative reference this project does not hold. What is implemented
meets the goal the application note states, by a linear mapping between the two black
points, and the code says that this is not a transcription of the standard. If the darkest
few percent ever have to be defended, that is the thing to buy.

Rendering intents other than `AbsoluteColorimetric` are read and recorded but do not yet
select different profile tags — `A2B1` is used, with `A2B0` as fallback, which is
relative-colorimetric behaviour and matches PDF's default intent. Selecting `A2B0` for a
`Perceptual` intent is a small change and is not done because nothing has measured whether
it improves anything.
