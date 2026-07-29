# ADR 0028 — The device's own colourants, and its thinnest line

Status: accepted, 2026-07-29.

## Context

Two questions were open, and they turned out to be one question asked twice: **what does this
device actually have?**

The first was the ledger's largest debt. The eighteenth session read §11.7 and found
overprinting unread — `/OP`, `/op` and `/OPM` appear nowhere in this tree, 63 of the corpus's
first-page `/ExtGState` dictionaries set one of the two booleans true, and the finding went in
as **six `silent` rows**, the ledger's worst status: drawn wrong with nothing said. This file's
predecessor named it first on the list of three silences and said what it was owed — a
condition narrow enough to report, per trap 11.

The second was §10.7.5, automatic stroke adjustment, the next silence on the same list: `/SA`
read nowhere, 49 corpus documents setting it true, and a clause requiring a stroke thinner than
half a device pixel to be drawn as a whole one.

Taking them meant reading §8.6.6 and §8.6.7 — the special colour spaces and overprint control
— because overprinting is stated in terms of the colourants a `Separation` or `DeviceN` space
names, and §8.6.6 is where those are defined. That family review is where most of this record
comes from, and it produced code in a place neither question was pointing at.

## Decision

### Overprinting is not a gap on this device, and the clause says so twice

Two independent readings reach the same answer.

**§8.6.7, the opaque model.** Overprinting decides what happens to the device colourants a
painting operation does not name: with the parameter false "painting a colour in any colour
space shall cause the corresponding areas of unspecified colourants to be erased", and with it
true, "and the output device supports overprinting, erasing actions shall not be performed".
That condition is not decoration. NOTE 1:

> Not all devices support overprinting. Furthermore, many PostScript language compatible
> printers support it only when separations are being produced, and not for composite output.
> If overprinting is not supported, the value of the overprint parameter shall be ignored.

This device is a screen with three additive process colourants and no separations. The
overprint *mode* is settled by a `shall not` in the clause's body rather than by a NOTE:

> It also shall not apply if the native colour space of the output device does not include
> CMYK device colourants; in that case, source colours shall be converted to the device's
> native colour space, and all components participate in the conversion, whatever their values.

Converting every component whatever its value is exactly what `ColourSpace::to_rgb` does.

**§11.7.4, the transparent model.** Here the answer is derived rather than asserted, and the
derivation is the interesting half. §11.7.4.3 defines the special overprinting blend mode and
its own verb is permissive — "a PDF processor **may** consider implementing a special blend
mode" — but permission is not what settles the row. Table 146 is, and it *collapses*. Its rows
are indexed by the source colour space and by which component **of the group colour space** is
affected; here the group space is device RGB, three process components and no spot colourants
at all:

- The three `Spot colourant` rows are the only ones whose `OP true` cells give the backdrop
  `C_b`. There is no spot colourant for them to affect.
- Row 1's `OPM 1` cell — `C_s if C_s ≠ 0, C_b if C_s = 0`, which is the whole of what a
  producer writing `/OPM 1` is asking for — applies to a `C, M, Y, or K` component *of the
  group space*. §11.7.4.3's first bullet says the same thing in words: it applies only "if the
  overprint mode is 1 … and the current colour space and group colour space are both
  DeviceCMYK".
- Every remaining reachable row — a `DeviceCMYK` source affecting a non-CMYK process
  component, any process space affecting a process component, and a group — gives `C_s` in all
  three columns.
- The `Separation` and `DeviceN` rows are unreachable, and that is not an accident of this
  implementation. §8.6.6.4 requires an additive device to substitute the alternate space
  *always*, and §11.7.4.3's NOTE 2 says the current colour space is the alternate for "Separation
  and DeviceN spaces that revert to their alternate colour space". So a spot colour arrives as a
  process space and takes row 4.

`B(C_b, C_s) = C_s` for every row this device can reach, which is the Normal blend function.
That is what these pixels composite through, so §11.7.4.2, §11.7.4.3 and §11.7.4.5 are
satisfied here, and §11.7.4.1 states no requirement of its own.

**The honest limit, stated because a convenient conclusion needs one.** The derivation rests on
the group colour space having three process components and no spot colourants. Exactly one
configuration breaks it: a group whose blending colour space *is* `DeviceCMYK`, where row 1's
`OPM 1` cell would apply. That is not a new gap — §11.6.6 and §11.7.2 already report a group
colour space that is not the device's three components, on 4 corpus documents. So overprinting's
one visible case here is a gap that was already named, and the day this renderer composites in a
document's colourants it owes the whole of Table 146.

**And one requirement of §11.7.4 is a real gap, on a condition that has nothing to do with
overprinting.** §11.7.4.4's second bullet applies whenever overprinting is *disabled*, which
here is always:

> In all other cases, a non-isolated knockout group shall be established. Within the group, the
> fill and stroke shall be performed with their respective prevailing alpha constants and the
> prevailing blend mode. The group results shall then be composited with the backdrop, using an
> alpha value of 1.0 and the Normal blend mode.

`B`, `B*`, `b`, `b*` and text rendering modes 2 and 6 become a `Fill` and a `Stroke` here, so
the band they share composites twice. That is §11.6.2's gap seen from clause 11's other end, it
is reported on the same condition, and its fix is §11.4.6's knockout groups.

`/OP`, `/op` and `/OPM` are therefore **named in `content.rs` as deliberately unread, with the
clause beside them**, which is the same treatment `/HT`, `/TR` and `/BG` already have. A key
that is ignored for a reason recorded next to it is a decision; a key that is ignored silently
is the thing the ledger exists to find.

### `/None` and `/All` are colourants, not tints

Reading §8.6.6.4 for the overprinting question found two rules this tree had never read, and
both were being answered by running a tint transform the clause says to ignore.

> The special colourant name None shall not produce any visible output. Painting operations in
> a Separation space with this colourant name shall have no effect on the current page.

`/None` exists so that a producer can put marks in a file that are not meant to be inked — die
lines, technical annotations, varnish plates. This tree painted them, in whatever colour the
alternate space made of the tint. §8.6.6.5 extends the rule to a `DeviceN` whose component
names are *all* `None`, which "shall always discard its output … it shall never revert to the
alternate colour space" — while a `DeviceN` with only *some* `None` components does revert and
passes them to the transform, which is what this tree already did and what the seventeenth
session's `oracle.rs` note about `issue9940.pdf` correctly says.

> The special colourant name All shall refer collectively to all colourants available on an
> output device, including those for the standard process colourants. … When outputting to an
> additive device, such as a computer monitor, the subtractive tint values of the All colourant
> shall be complemented by subtracting from 1 before applying to all available colourants.

So `/All` at tint `t` is the grey `1 − t` in every component: full ink is black, no ink is
white. Not the tint transform's answer, and not `DeviceGray`'s reading of the same number
either — the complement is the whole point of the sentence.

Both are decided **before** the alternate space and the tint transform are parsed, because the
clause requires both to be ignored "although valid values shall still be provided". A file that
fails to provide them has still named a colourant "a PDF processor shall support … on all
devices, even if the devices are not capable of supporting any others", so refusing the space
for an unreadable function would be refusing the one thing the clause makes mandatory.

The corpus sizes this at 2 first pages for `/All` and **none at all** for a `Separation`
`/None`. That is trap 8 in one line rather than evidence the rule is unimportant: `/None` is a
print-production feature and the pdf.js corpus is a collection of web bug reports.

### One device pixel is a decision in `pdf-render`, not a rasteriser's convention

§8.4.3.2:

> A line width of 0 shall denote the thinnest line that can be rendered at device resolution:
> 1 device pixel wide.

§10.7.5, when `/SA` is enabled:

> If stroke adjustment is enabled and the requested line width, transformed into device space,
> is less than half a pixel, the stroke shall be rendered as a single-pixel line.

These are the same width, and §10.7.5's NOTE says so: the second case "is equivalent to the
effect produced by setting the line width to 0". They are therefore one function,
`Stroke::device_width`, in `pdf-render` beside `Image::is_smoothed` and `Image::area_averaged`
— the other two decisions that depend on the device and must not be made twice.

**Putting it there is what found a defect fifteen sessions old.** `tiny-skia` treats a width of
`0.0` as a hairline and so answers §8.4.3.2 for free; the CPU backend had relied on that, with a
comment saying the two semantics "line up in our favour". Vello has no hairline mode, and
`kurbo` expands a zero-width stroke into an **empty outline**. Every `0 w` line in every
document was invisible on the GPU backend. `zerowidthline.pdf` is in the corpus, is named for
this, and says on its own face "second should be 1 device pixel, third should also be 1 device
pixel (but scaled 2x)": the GPU drew neither line, and drew none of the page's stroked text
either, because text rendering mode 1 with a zero width is the same case. Nothing reported it —
the interpreter had no complaint and the page had the right ink everywhere else — and the
eleven cross-backend scenes could not see it, because every one of them stroked a width the
document stated. Trap 2, in the one place this project trusts most, for the second session
running.

The minimum is stated in device pixels and applied in path space, so it is `1.0 / stretch`.
Where the transform scales the two axes differently there is no single answer, and §8.4.3.2
says as much — "the thickness of stroked lines in device space shall vary according to their
orientation" — so `Transform::max_stretch` takes the widest direction: the substituted stroke is
exactly one pixel at its widest, and §10.7.5's half-pixel test fires only where the stroke is
thin in *every* direction. For a similarity transform, which is what every page transform this
renderer builds is, the two singular values coincide and the choice does not arise.

`max_stretch` also corrected a claim: `command_bounds` computed a stroke's margin from
`determinant().abs().sqrt()` with a comment calling that "an over-estimate for a sheared one,
which is the safe way round". It is the wrong way round. A shear leaves the determinant at 1
while tripling a length, so the margin was too small and §11.4.6's overlap report could miss an
overlap. The determinant is the *geometric mean* of the two singular values, never the larger.

### What §10.7.5's other half gets, and why it is a departure rather than a gap

The clause's first requirement is that "the line width and the coordinates of a stroke shall
automatically be adjusted as necessary to produce lines of uniform thickness … no more than
half a pixel different". That is grid-fitting, and it is not implemented.

The reason it is recorded as a departure rather than as work owed is what the requirement is
*for*. The non-uniformity it removes is an artefact of binary scan conversion — Figure 70 shows
an aliased line whose rasterised width alternates between one pixel and two — and §10.7.4
requires exactly that binary conversion, which this tree already departs from by anti-aliasing
(ADR 0025 records it as the first of three departures in that subclause). An anti-aliased
stroke's coverage-weighted thickness is the requested width at every position along it, which is
the clause's stated purpose reached by a different route. What is genuinely not delivered is
uniformity of the *rasterised* line on a device that quantises coverage to whole pixels, and
this is not such a device. §10.7.1's NOTE — "the specifics of the scan conversion algorithm are
not defined as part of PDF" — is what licenses this one as it licenses the other three.

Nothing reports it, and that is deliberate rather than an oversight of trap 11: a report names
pages where the device could have done better, and there is no such page here.

## Consequences

**The ledger's `silent` count goes from 8 to 1.** Six rows of §11.7.4 are re-decided by
derivation, §10.7.5's is implemented in the half that a display can state and recorded as a
departure in the half it cannot. What is left is §8.11.4.4, usage dictionaries, which needs a
layer panel to be worth more than a report.

**A gap the demand track had sized at 63 documents was not a gap.** That is worth stating in
the direction it happened: the corpus count was real — 63 first pages do enable overprinting —
and it was counting a key, not a difference. The eighteenth session's own note for §11.7.4 said
"presence of the key is not the condition", and trap 11 says to instrument before believing a
count. What it did not anticipate is that the narrow condition could turn out to be *empty*.
The instrument that settled it was not a corpus run but Table 146 and a list of this device's
colourants.

**The two gates did not move, and one page's picture did.** The corpus gate is unchanged at 220
incomplete — nothing new reports and nothing stopped. The oracle is unchanged at 688 agreeing
and 96 contradicted, and the reason is measurable rather than assumed: of the corpus's first
pages, 17 paint a stroke while `/SA` is in force and 4 have one thin enough to adjust at 72 dpi;
of those 4, two agree with the references either way and two are pages the references cannot
agree about among themselves. On those two the change moves us *closer* — `bug1721218_reduced.pdf`
from mean 0.28 to 0.27 and worst tile 18.58 to 18.41 — which is the direction the derivation
predicts and too small for the gate to see. The GPU defect is the one with a picture, and it is
not on either gate at all, because both gates measure the CPU backend.

**A `Separation` of `/None` is now transparent rather than coloured**, which reaches images and
shadings by the same route as fills, since all three go through `ColourSpace::to_rgb`. An alpha
of zero is exact: §11.3.6's compositing formulae leave the backdrop untouched at zero alpha
under every blend mode, so nothing downstream needs to know the colour is special.

**A cost worth naming.** `Stroke::device_width` runs once per stroke command and computes a
square root; `max_stretch` computes two. That is per *command*, not per pixel, and the corpus
gate's wall clock is unchanged at 1.9 s. The alternative — caching a scale on the target — would
put a device-dependent number in a display list that deliberately has none.
