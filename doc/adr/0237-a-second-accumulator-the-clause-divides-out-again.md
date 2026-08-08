# ADR 0237 — A second accumulator the clause divides out again

Status: accepted, 2026-08-08 (session 400).

## Context

`doc/todo/23` carried §11.4.4's NOTE 5 residue as one of two standing transparency departures,
and ADR 0234 had just written down what it was thought to need:

> §11.4.4's NOTE 5 residue needs the group's elements composited onto the *page's* own colour
> with the group alpha accumulated separately — NOTE 4 says why one raster cannot do it — which
> means a buffer whose colour is the backdrop while its alpha is zero, and neither `tiny-skia`
> nor Vello has one: both store premultiplied samples, where a colour at alpha zero does not
> exist.

That paragraph is the reason the population had stood for three hundred sessions. **It is
wrong**, and the round's first product is finding out why.

Six corpus documents stated the group. A census, taken by instrumenting the interpreter and
running the corpus, says what they actually are — and every one of the six has `alpha = 1` and
the **Normal** blend mode at the `Do`:

| document | isolated | knockout | inside a knockout group | soft mask at the `Do` |
|---|---|---|---|---|
| `issue12798_page1_reduced` | no | no | no | **yes** |
| `issue13520` | no | no | no | **yes** |
| `bug1755507` | no | no | no | **yes** |
| `issue18032` (three groups) | no | no | no | **yes** |
| `issue18032` (a fourth) | no | **yes** | no | no |
| `knockout_blend_multiply` | no | **yes** | no | no |
| `knockout_inner_backdrop` | no | no | **yes** | no |

So the population is not "a group with an alpha, a blend mode or a mask" in equal parts. It is
**a soft mask at the `Do`** in four documents out of six, and a knockout question in the other
two.

## The clause reading

### What §11.4.4 requires kept apart

Table 140 names two accumulators. The **group** shape and alpha `fgi`, `αgi` accumulate "the
accumulated source shapes of group elements E1 to Ei, **excluding the initial backdrop**"; the
complete alpha `αi` is "[a]ccumulated alpha after compositing element Ei, **including the
initial backdrop**". The colour recurrence uses the second; the group's *result* is the first,
and NOTE 3's backdrop removal divides by it:

```text
C = Cn + (Cn − C0) × (α0/αgn − α0)
```

NOTE 4 is the clause's own advice about how to hold the two:

> For shape and alpha, backdrop removal can be accomplished by maintaining two sets of
> variables to hold the accumulated values.

A raster of premultiplied samples has one set. Hence the conclusion this project recorded
twice: one raster cannot do it.

### Why one raster can do it

The conclusion mistakes a *statement about the intermediate* for a statement about the result.
`αgn` is divided out by NOTE 3's removal — and then **multiplied straight back in** when the
group's result is composited with the same backdrop under §11.3.3, because the group's object
alpha *is* `αgn`. Nothing else in the pipeline observes it.

Writing `B` for the backdrop and `E(B)` for the group's elements composited onto it, both
premultiplied, and `w` for the constant alpha times the soft mask at the pixel, the two steps
together are

```text
result = (1 − w) × B + w × E(B)
```

— an ordinary interpolation. `w = 1` gives `E(B)`, which is NOTE 5's flattening, so the two
readings agree where NOTE 5 says they must.

The derivation is four lines. With `α0` the backdrop alpha and `αn = Union(α0, αgn)` the
buffer's alpha after the elements,

- `αgn = (αn − α0) / (1 − α0)` — Table 140's own union, solved for the group alpha, so the
  second accumulator is a *function of* the first wherever `α0 < 1`;
- `C − C0 = (Cn − C0) × αn / αgn`, from NOTE 3's removal;
- §11.3.3 with the Normal blend function gives `Cr = C0 + (w·αgn / αr) × (C − C0)`, and the
  two `αgn` cancel;
- in premultiplied terms the whole thing is `(1 − w) × B + w × E(B)`, alpha included — which
  is also why `α0 = 1` is not a special case even though the first line divides by zero there.
  The quantity that could not be recovered is the quantity that does not matter.

**Checked against the clause's own formulas rather than against the algebra.** §11.4.4's
recurrence, §11.3.3's compositing formula and §11.3.7's shape-and-opacity products were
transcribed and run over 200 000 random inputs — arbitrary backdrop colour and alpha, one to
four elements with arbitrary shape and opacity, blend functions drawn from Normal, Multiply and
Screen, arbitrary `w`. **Worst deviation 5.6 × 10⁻¹⁶**, which is double-precision rounding.

### The one condition, and it is load-bearing

The step that cancels is §11.3.3 with the **Normal** blend function. Under any other, the
group's own colour `C` enters through `B(C0, C)` and `αgn` with it. The same simulation with
`Multiply` at the `Do` is **0.601 of full scale** apart at worst — so the condition is not a
convenience.

Two more conditions follow from what the construction does not model:

- **Not a knockout group.** §11.4.6 composites each element with the group's *initial*
  backdrop, which for a non-isolated knockout group is the page rather than transparency, so
  the two stages are not the pair ADR 0234's `Command::Shaped` states.
- **Not an element of one.** A knockout group weights its elements by their own shape, which
  this command does not carry.

And a fourth, which is about cost rather than correctness: **with every element painting Normal
the two models are the same page**, exactly — §11.4.4's NOTE 3, and §11.6.7's NOTE 1 states the
same equivalence for a pattern cell. The same simulation confirms it at 4.4 × 10⁻¹⁶. So the
harder construction is stated only where an element blends, which is the condition the *report*
already fired on.

## Decision

### The display list names the backdrop

`pdf_render::Command::Group` gains `isolated: bool`. `true` is §11.4.5 — "[a]n isolated group
is one whose elements shall be composited onto a fully transparent initial backdrop rather than
onto the group's backdrop" — which is what a layer in any rasterising library is. `false` is
§11.4.4's own model, and `pdf-model` emits it only under the four conditions above.

It belongs in the vocabulary rather than inside each rasteriser for ADR 0210's and 0234's
reason: *which* groups the collapse is exact for is a reading of clause 11, and a backend that
decided it alone would be a decision neither backend had made (trap 2).

### `render-cpu` builds it, in one pass

`initial_backdrop` seeds the group's buffer with a copy of the surface instead of transparency;
`blend::interpolate` writes `(1 − w) × destination + w × buffer` over the band, with `w` the
group's constant alpha times the clip-and-mask coverage `MaskCache` already builds.

**One pass rather than two Porter-Duff draws, and the reason is arithmetic.** Destination-Out
by `w` followed by `Plus` of the buffer at `w` computes the same expression — it is what ADR
0234's shaped element does — and it rounds twice: at `w = ½` over an opaque backdrop each draw
keeps 127 of 255 and the pair leaves **254**, so a page the clause makes fully opaque comes back
one level transparent and the medium's white shows through on every channel. Measured directly
against `tiny-skia`. One pass rounds once and `w + (1 − w) = 1` is exact in it.

Outside the group's marks the buffer *is* the backdrop, and interpolating a value with itself is
that value — so the copy costs the unmarked region nothing, and `interpolate` skips a pixel
whose two operands are equal.

### `render-gpu` and `render-quorra` refuse it, by name

A Vello layer always begins fully transparent and a scene cannot read what it has drawn so far,
so there is no way to seed one with the page; `quorra_scene::GroupSpec` opens its layer the same
way. Both refuse, naming §11.4.4 and §11.4.5. The frame goes to the CPU backend, which is what
`CLAUDE.md` keeps that backend for.

This is ADR 0234's quorra argument a second time, and it holds for the same reason: all four
pages used to be counted as **agreeing**, and that agreement was two backends substituting the
same wrong backdrop. `doc/QUORRA_FEEDBACK.md` section 16 is the request.

## Consequences

### The measurement, against the clause's own arithmetic

Three fixtures, each derived from §11.4.4 and §11.3.3 rather than from another renderer, and
each checked by putting the old route back and watching it fail.

| fixture | the clause | what the old route drew |
|---|---|---|
| an opaque blue under `Multiply` inside a non-isolated group at `ca 0.5`, over an opaque red page | Multiply is the componentwise product, so `E(B)` is black: `½ × (1,0,0) + ½ × (0,0,0)` = **(128, 0, 0)**, and **(255, 0, 0)** where the group marks nothing | **(128, 0, 128)** — the blue survives, because an isolated group's element multiplies against transparency |
| the same under a soft mask **and** the constant, `w = ¼` | `¾ × (1,0,0)` = **(191, 0, 0)** | **(191, 0, 64)** |
| the same group nested inside an *isolated* one whose backdrop is half-opaque red | `E(B)` is `(0, 0, 1 − 128/255)`, the interpolation gives premultiplied `(64, 0, 64; 192)`, and over the white page **(127, 63, 127)**; where the group marks nothing the backdrop returns unchanged at **(255, 127, 127)** | **(127, 63, 191)** drawn isolated, and **(255, 95, 95)** at the unmarked pixel if the buffer is composited back with source-over instead of interpolated |

The third is the one the other two cannot see, and it is why it exists. A page is opaque, and
over an opaque backdrop source-over and the interpolation agree: they differ by
`w × (1 − α_buffer)` of the backdrop, which is zero there. Nested inside an isolated group the
backdrop has an alpha, and the difference is **32 of 255 on two channels** — the same magnitude
ADR 0234 measured for §11.4.6's second stage, for the same reason.

### What moved on the gates

- **corpus 67 → 65 incomplete**, and nothing joined. `issue12798_page1_reduced.pdf` and
  `issue13520.pdf` lost their only report; `bug1755507.pdf` keeps §11.6.6's `/DeviceCMYK` and
  loses §11.4.4's; `issue18032.pdf` keeps all three of its, and both of its §11.4.4 lines are
  now its **knockout** group rather than the three under soft masks.
- **oracle: agrees 904, contradicted 69, ambiguous 786 — every total unmoved**, and the
  complete-page split moves: 1691 → **1693** pages called complete, ambiguous-and-complete
  751 → **753**. The two documents that stopped reporting were judged for the first time and
  both landed **ambiguous**, which is the verdict for a page no two references settle.
- **quorra 916 agree / 36 differ / 5 refused / 17 not comparable → 912 / 36 / 9 / 17**, with
  the argument above. The four new refusals are exactly the four documents the corpus gate
  stopped reporting §11.4.4 on, which is the check that the display list gained the
  construction where the report left.
- **text 99.2% (24043/24243 words), 25 below 90% — unmoved**; its ungated-incomplete count
  falls 64 → **62** as the two documents become gated.
- **dates, XMP and JPEG 2000 unmoved.** Tests **1394 → 1398**.

### The two new pages had to be diagnosed, and both are the references disagreeing

The oracle's ratchet on the ambiguous bucket refuses an undiagnosed arrival, so both got a
group with a two-ladder measurement.

`AMBIGUOUS_NON_ISOLATED_POSTER` — `issue12798_page1_reduced.pdf`, four commands, small white
type on a magenta band:

```text
             72 dpi   288 dpi   576 dpi
poppler     23.7199   23.8327   23.8408
mupdf       23.8604   23.9477   23.9610
ours        23.8002   23.8373   23.8438
```

Three ladders climbing in parallel and converged by 288 dpi; ours ends **0.0030 of 255** from
`poppler`'s limit where the two references' own limits are **0.120** apart. The five renderers
span 23.72 to 24.01 at the page's own scale. It is `ambiguous` because a worst-tile bound
measured over glyph edges is tighter than five renderers' scan conversion of 6-point type.

`AMBIGUOUS_STACKED_SCREEN_UNDER_MASKS` — `issue13520.pdf`, ten commands, one glossy blob of
nested groups with `/Luminosity` masks and `Screen` blends:

```text
             72 dpi   288 dpi   576 dpi
poppler     17.6287   17.4891   17.4397
mupdf       16.5747   16.7086   16.6970
ours        16.9987   17.1636   17.1585
```

The two references' limits are **0.743 of 255 apart and approach from opposite sides**, so there
is nothing to sit inside or outside; ours converges and ends between them. At the page's own
scale `ghostscript` is 20.03 and `hayro` 20.23 against `mupdf`'s 16.57 — **3.65 of 255 across
five renderers** on an illustration 209 pixels wide — and the strip shows five different
highlights on the same lozenge. §11.6.6's blending space, which `doc/todo/23` still owes.

### `doc/todo/00`'s step 7, run before and after

Our ink minus the lightest reference's, over all **786** ambiguous pages, from the artefacts
each oracle run leaves on disk — the *before* half taken by stashing the round and re-running
the gate. **Byte-identical**, head `issue12418_reduced.pdf` −19.447 through
`issue11915.pdf` −0.637 and tail `recursiveCompositGlyf.pdf` +198.653.

That is the expected result and it is worth saying why, because "nothing moved" is otherwise
indistinguishable from "the sweep did not run". The sweep's head and tail are the extremes, and
the two pages this round changed sit in the middle of the distribution both times:
`issue13520.pdf`'s gap goes **+2.554 → +0.698** — measured directly, ours 19.1288 → 17.2724
against `mupdf`'s 16.5747 — and `issue12798_page1_reduced.pdf` moves by **0.0003**, its
non-isolated groups being a few hundred pixels of a poster. So the sweep says what it is for:
nothing stopped being drawn, and one page moved *toward* the references by 1.86 of 255.

### The recursion limit, which is a note about the compiler and not about the code

`oracle.rs`'s `diagnosed_ambiguous` chains one iterator per `AMBIGUOUS_*` group, and the
sixty-third group overflowed rustc's default query depth of 128 while computing the chained
type's layout. `#![recursion_limit = "256"]` on the test crate, with the reason beside it:
folding the groups into a slice of slices would work too and would cost the property that a
group's name appears beside its diagnosis at the one place both are read.

### What §11.4 still owes

- **A knockout group whose elements blend**, which is the same sentence one clause over:
  §11.4.6's initial backdrop is then the group's own, and the two stages are not
  `Command::Shaped`'s pair. Three corpus documents — `issue18032.pdf`,
  `knockout_blend_multiply.pdf` and `knockout_inner_backdrop.pdf` — and the last of the three is
  a non-isolated group *inside* a knockout one, which is the third condition rather than the
  second.
- **A blend mode at the `Do`**, which is where the collapse genuinely fails and where NOTE 4's
  second accumulator would genuinely be needed. No corpus document states one.
- **§11.6.6's blending space for a painted group**, four documents, all `/DeviceCMYK`, unmoved
  and still a second raster format in three backends.
- **A group buffer seeded from its backdrop, in the other two backends.** Vello and quorra both
  open a layer on transparency; neither can express §11.4.4's initial backdrop, and until one
  can, four corpus pages are drawn by the CPU backend alone.
