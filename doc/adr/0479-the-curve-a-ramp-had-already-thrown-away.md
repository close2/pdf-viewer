# 0479 — The curve a ramp had already thrown away

Status: accepted.
Session: 650. Follows ADR 0469, which found that §10.5's transfer function reaches no colour a
shading carries, reported it, and priced the fix at "`Shading::with_alpha`'s walk done again with
a closure, in `pdf-render`". **That price was wrong, and this ADR is mostly about why.** Follows
ADR 0068, whose ramp simplifier is the reason it was wrong.

## The decision

**§10.5's transfer function is applied where a shading's colours are *made*, not to the shading
the display list carries.** Concretely, in `pdf_model::shading::kind_of` and `pdf_model::mesh`,
which reach all four kinds a display list has:

| kind | where the colours are | where the function is applied |
|---|---|---|
| axial (2), radial (3) | `Ramp` | inside `Ramp::sample_across_at`'s closure (`shading::ramp`) |
| mesh (4, 5) with stated colours | `Corners::Colours` | `mesh::transferred_corners`, after the patch subdivision |
| mesh (6, 7) with a `/Function` | `Ramp` | `MeshReader::colour_of_parameter` |
| function-based (1) | `DeferredColours` | `FunctionColours::row`, per grid cell |

`pdf-render` needed no change at all, which is the opposite of what 469 expected.

Two consequences follow and are carried in the code with their reasons:

- **A shading built under a transfer is not cached.** `shading::Cache` keys on `(ObjectId,
  resolution, Conversion)`, and a `Transfer` is a group of parsed §7.10 functions with no identity
  a key could be built from. Not caching is exact; it is also the answer the table already gives a
  `/ColorSpace` stated as a name.
- **A type 1 shading's device program is withdrawn.** `ShadingProgram` is §7.10.5's function
  lowered to instructions `render-quorra` evaluates on the GPU, which produces the colour and
  nothing else — there is nowhere on that path to put a transfer, exactly as there is nowhere to
  put §11.6.4.4's constant alpha, which is why `Shading::with_alpha` already drops it. The grid
  producer carries the transfer and is what draws.

## What the clause says a transfer function maps

§10.5 is a rule about a *component*, and it says so three times without ever naming an object:

> In the sequence of steps for processing colours, the PDF processor shall apply the transfer
> function after performing any needed conversions between colour spaces.

> The input shall be the value of a colour component in the device's native colour space, either
> specified directly or produced by conversion from some other colour space. The output shall be
> the transformed component value to be transmitted to the device (after halftoning, if
> necessary).

> Each colour component shall have its own separate transfer function; there shall not be
> interaction between components.

That decides the placement twice over. *After the colour-space conversion* puts it downstream of
`Conversion::paint`, which is where every shading colour in this tree is born. *The value
transmitted to the device* means the colour at a point, not a parameter of the object — so there is
no reading under which a ramp's stops are exempt because they are a ramp.

§10.1's list of rendering steps orders it against the rest of clause 10, and the order is the one
this tree already had for a solid colour: convert the colour, apply the transfer, **then** scan
convert. A shading is scan converted from the colours the display list gives it, so the function
has to be inside them before they get there.

## Why "map the finished shading" is the wrong answer, in one number

ADR 0469 priced this as a walk over `Ramp`, `Corners::Colours` and `DeferredColours` — the same
walk `Shading::with_alpha` does for §11.6.4.4's constant alpha. That is right for alpha and wrong
for a transfer, and the difference is ADR 0068.

A `Ramp` is not the shading's colour function. It is a *sampling* of it — `Ramp::RESOLUTION`
samples, or more where §10.7.3's `/SM` asks — and then `simplify` drops every stop that lies within
half an eight-bit level of the line its neighbours draw, because both rasterisers interpolate
linearly between consecutive stops and a dropped stop produces the same byte. That is exact for
what the display list is *for*. It also means:

**A `/FunctionType 2` interpolation with `/N 1` — most of the shadings that exist — reaches the
display list as two stops.**

Map those two stops through a transfer function and let the rasteriser interpolate, and what gets
drawn is the chord between the transferred ends. What the clause asks for is the transfer of the
colour at each point. For a transfer that squares its input, the ramp's midpoint should be 0.5² =
0.25 and the chord gives 0.5: **64 levels of 255, on the commonest shading in PDF**, from an
implementation that would have passed every other test in this tree.

Applying the function inside the sampling makes the ramp a sampling of the *composition*. The
simplifier then measures the colours that will actually be drawn, which is what it was written to
do, and a curve keeps the stops a curve needs. `a_ramp_is_the_composition_and_not_its_endpoints` is
the fixture, and it is the only test in this tree that separates the two designs — calibrated
against both defects (trap 13): it fails with the mapping removed altogether, and it fails with the
mapping done ADR 0469's way while the other four new tests pass.

**The general shape is worth more than the clause.** *A lossless simplification is lossless with
respect to the operations that were already going to be applied.* Insert a new one downstream and
it stops being lossless, silently, in the direction of "smoother than the file says". ADR 0068's
own doc comment states its bound exactly — half a level, given linear interpolation between the
survivors — and reading that sentence is what makes the trap visible before a picture does.

## What a mesh cannot have, and the clause that permits it

A mesh's stated corner colours are mapped **after** the patch subdivision rather than before it.
§8.7.4.5.7 makes the colour inside a Coons or tensor patch a bilinear mix of the four stated
corners, so mapping after subdivision maps `PATCH_STEPS`² samples of that mix instead of four
corners — the same argument as the ramp's, one geometry over.

What cannot be closed at all is the last step: a rasteriser interpolates linearly between the three
corners of each triangle it is handed, so between them it draws a mix of transferred colours where
the clause would have the transfer of the mixed colour. **This is §8.7.4.4's own permission and not
a departure invented here**:

> PDF processors may actually compute colour values only for some subset of the points in the
> target area, with the colours of the intervening points determined by interpolation between the
> ones computed.

Closing it entirely needs a per-pixel pass, which is the same machinery §11.7.5.2's per-region
model needs, and `doc/todo/13` prices the two together.

## What is still owed: the pattern selected a graphics state too early

§8.7.2 makes a pattern a colour, `scn` is where a colour is set, and a shading pattern's colours are
therefore resolved at the selection. The mark may be several graphics states later. A file that
states one `/TR` at the `scn` and another at the `f` is painted with the first, and this round did
not change that.

**It is reported rather than drawn in silence**: `PatternPaint::Shading` carries the transfer its
colours were built under, `Painted::of` compares it with the one in force at the mark, and
`Interpreter::note_transfer` raises `Unsupported::TransferFunction` when they differ. The comparison
is `Arc::ptr_eq` — an over-approximation in the safe direction, since two `gs` operators naming one
`/ExtGState` parse two equal and non-identical `Transfer`s — because equality of parsed §7.10
functions is a relation this tree does not have and would be inventing for a population of zero.

**It was not fixed because it is not this clause's question.** The `scn` is also where this tree
resolves §8.6.5.9's black point and §11.4.7's compositing target for the same colours, and neither
of those says a word when the state moves underneath it. The staleness is a property of *when a
shading pattern's colours are built*; a round that moves it should move all three, and
`doc/todo/13`'s new last section says what that costs.

## The population, measured before the code and over both corpora

`examples/transfer_function_census` gained a third figure and rayon, and it now takes the SafeDocs
crawl in under a minute.

- **`doc/pdf.js`**: 964 open, 13 state a Table 57 `/TR` or `/TR2`, **1** states one that is not
  `/Identity` or `/Default`.
- **The SafeDocs crawl**: 65 703 open, **1352** state one, **32** state a real one. That is a
  hundred times the pdf.js population and nobody had asked it before.
- **Documents that paint a shading while a real one is in force: zero, in both.** Five crawled
  documents state a real transfer function and paint a shading *on the same page*, which is the
  page-level over-approximation; all five render byte-identically with the mapping in and out, and
  the exact condition — the pre-round report's, run as a probe over all 32 — matches none of them.

So this round moves no document that exists, and the five fixtures are the whole of the evidence
(trap 8). That is worth stating plainly rather than hedging: the argument for the change is the
clause, and the argument for *this* placement of it is a number — 0.25 against 0.5 — that no corpus
on this disk can produce.
