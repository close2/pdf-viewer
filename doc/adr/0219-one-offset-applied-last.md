# ADR 0219 — One offset, applied last

Status: accepted, session 382. Amends **ADR 0139**, whose closing property — "the picture does not
depend on how it was divided" — was false on a document this repository commits, and cannot be made
true. What is true instead is stated here, with the measurement behind each half.

## The counter-example, and what it was not

`doc/PDF20_AN001-BPC.pdf` page 1, drawn 500 pixels wide (500×707), differed at exactly one pixel
between one strip and every division from two to thirty-two: **(411, 659) is 95 whole and 79
split**, alpha unmoved. Session 381 found the same defect at a slightly different scale — (117,
636), 127 against 111 — when it drew a page in one strip on purpose, because a confined process may
not ask how many cores it has (ADR 0218).

`doc/todo/12` recorded a diagnosis and it was wrong, which is worth stating because it was
plausible: it said a composed matrix is not bit-for-bit the unmultiplied one, so the whole-page
path departed by taking `target.transform` itself while every strip took
`target.transform.then(translate(0, -top))`. **`Transform::then` with a pure translation is exact.**
Every component of the product is `x·1 + y·0` except the vertical translation, which is `f − top`,
and subtracting a whole number of rows from a device coordinate of this size rounds not at all. The
first strip's matrix is the page's, bit for bit; that is not where the pixel went.

## The mechanism

It is one composition later. `encode_in_strips` folded the strip's offset into the page transform
*before* handing it down, so a mark's own transform was composed with the **shifted** page
transform:

```text
current:  mark.then(page.then(translate(0, −k)))     f = fl(fl(mark.f·d) + (F − k))
wanted:   (mark.then(page)).then(translate(0, −k))   f = fl(fl(mark.f·d) + F) − k
```

Both compute the same real number and they round in different places, because the two sums fall in
different binades. For the glyph that owns pixel (411, 659) — an 8-point mark at the foot of the
page, whose top edge lands within a thousandth of a pixel of row 659 — the two answers are

| | `f` | bits |
|---|---|---|
| whole page | 663.4785 | `4425dea0` |
| strip, offset folded in first | 427.478**55** | `43d5bd41` |
| strip, offset composed last | 427.478**52** | `43d5bd40` |

and the third is `663.4785 − 236` exactly. One `ulp`. Drawing that one mark under each of the three
matrices: the whole page gives 95, the folded-in strip gives 79 and 1 pixel differs, the
composed-last strip gives 95 and **0 pixels differ over the whole mark**.

**Why one `ulp` is worth 16 of 255**: `tiny-skia` supersamples, and 255 ÷ 16 is 16. An edge within
an `ulp` of a sample row falls on the other side of it and one of the sixteen samples changes hands.
Every departure in this ADR is a multiple of 16 for that reason, and ADR 0138's chopped curves
reached 32, 48 and 64 — two, three and four samples at once. That is one of the two ways the causes
are told apart; the other is *how many* pixels move, because a chopped path re-parameterises a whole
edge and moves hundreds while a shifted origin moves the handful that sit on a sample row.

## Which of 95 and 79 is right

**Neither.** `doc/todo/00` step 6's closed form — the same mark at rising resolution, the block of
subpixels that becomes this pixel averaged — converges from the other side of both:

| | 1× | 2× | 4× | 8× | 16× | 32× |
|---|---|---|---|---|---|---|
| mean of the block | 95.000 | 87.500 | 84.562 | 87.609 | 86.641 | 86.560 |

The geometry covers this pixel to **86.6 of 255**, and 86.6 is within half a point of the midpoint
of 95 and 79. The edge sits *on* a supersample row: the two renders are the two adjacent values
`tiny-skia` can express there, and the ladder says the truth is between them. There is no answer to
keep on accuracy grounds, and a session that had picked one on those grounds would have been
choosing noise.

So the choice is made on a different ground, and it is the project's: **the page is what is being
drawn and the division is an implementation detail, so the answer must be the one that exists when
there is no division.** The whole-page arithmetic is kept — 95 — and every strip now reproduces it.
It is also the arithmetic every reference render, every oracle verdict, every corpus count and every
cross-backend tolerance in this tree was recorded against.

## What was built

`Surface` and `ToDevice`, replacing the strip's shifted `TargetSpec`:

- **The page's own target travels into a strip**, together with the page rows the strip holds. A
  strip no longer has a transform of its own.
- **`Band` counts page rows.** A strip of a page and the band a clip admits were two coordinate
  systems that had to agree; they are now one, so a clip's band is the same run of rows whether it
  is measured on the page or inside a strip, and the group buffer and soft-mask surface that follow
  it are the strip's rows rather than the page's.
- **`ToDevice::of` composes the offset last**: `mark.then(page).then(translate(0, −rows))`, where
  `rows` is the *sum* the surface and the band contribute — one integer, one subtraction, at the end.
- Culling (`misses_surface`) and clip bounds are measured on the page's grid too, so a strip asks
  the same question of the same numbers the whole page asks, over a different run of rows.

Two tests state the two halves of the argument, because between them they say the residual below is
not ours:

- `the_offset_is_composed_last_and_costs_nothing` (unit): over three page transforms, four mark
  transforms and seven offsets, every component but `f` is the page's bits, and `f` is the page's `f`
  minus the offset **with no rounding**, asserted in `f64` so that it is about the arithmetic rather
  than about itself.
- `a_surface_that_starts_elsewhere_is_not_invariant` (`strip_cut_exactness.rs`): `tiny-skia` drawing
  the *same* path under the *same* matrix into a surface whose first row is elsewhere is **not** the
  same drawing.

## What is left, and why it cannot be removed

`tiny-skia` maps a point as `y·sy + ty` in `f32`. Handing it `ty − k` instead of `ty` moves the sum
into a different binade, and `fl(p + ty) − k` is not `fl(p + ty − k)`. That is the dependency's
arithmetic, and no arrangement of ours reaches it: the only way to make the sums identical is to
give every strip the page's own origin, which means a page-sized pixmap per strip and every command
drawn in full into every one of them. Sixteen times the memory and the replay, to remove the last
fifteen pixels of a two-megapixel page.

The probe measures it: 40 shapes at 7 offsets, **2 of 280 pairs** differ, never by more than one
supersample — which the test asserts, because a larger difference would mean something other than
the origin had moved. It fires on shifts that cross a binade, and it does *not* fire on the
coordinates ADR 0139's earlier probes used —
111.75, 903.25, 37.25 are dyadic fractions, and subtracting an integer from one of those is exact.
A suite of shapes is a suite of shapes, and so is a suite of coordinates.

## The property, restated

> A page drawn in strips is the page drawn whole, **up to `tiny-skia`'s arithmetic at a shifted
> origin**: this backend hands a strip the matrix it hands the page with a whole number of rows
> subtracted, and what remains is one supersample on an edge that lands within an `ulp` of a sample
> row.

Measured over three real pages, at nine divisions each, before and after:

| page | before: pixels moved / worst | after |
|---|---|---|
| `PDF20_AN001-BPC.pdf` p1, 500×707 | 1 / 16 | **0 / 0** |
| ISO 32000-2 p6, 1191×1684 | 20–28 / **32** | 15–25 / 16 |
| ISO 32000-2 p101, 800×1131 | 16–23 / 16 | 10–19 / 16 |

The worst byte on page 6 halving from 32 to 16 is the same sentence read twice: two supersamples
were being lost where the composition rounded, and one is what a shifted origin costs.

`crates/pdf-model/tests/strip_parallelism.rs` is the gate, and it asserts three things rather than
one: the counter-example page is **exact**, no pixel anywhere moves by more than one supersample,
and fewer than one pixel in ten thousand moves at all. The first two fail on the code this ADR
replaces. It lives in `pdf-model` because a real page needs a parser, an interpreter and a font
stack — which is trap 12b answered rather than restated: `render-cpu`'s own guard draws six
`test-scenes` fixtures and passed for two hundred and twenty-six sessions while the property was
false.

## What it cost

`callgrind`, 20 renders of the specification's own pages at the machine's own strip count:

| page | before | after | |
|---|---|---|---|
| ISO 32000-2 p6 | 3 975.4 M | 3 978.5 M | **+0.08%** |
| ISO 32000-2 p101 | 5 483.1 M | 5 496.3 M | **+0.24%** |

One extra `Transform::then` per drawn command and per clip shape — six multiplications and four
additions — against everything else a command costs. The wall clock over `strip_spans` moved by less
than its own spread in the same sitting, and in both directions: at 16 strips page 6 went 5.2 ms
before to 5.4 after and page 101 went 7.9 to **7.7**. Which is why the counter is quoted and the
clock is not — `doc/HANDOVER.md`'s "quote the clock for a parallel change and the counter for a
serial one" is about a change to the *division*, and this one is a change to what every command
does.

## The habits

- **A plausible diagnosis in a todo file is a hypothesis, and the cheap step is to run it.** The
  written diagnosis named the right *line* and the wrong *mechanism*, and the difference decided the
  fix: composing the same way everywhere would have left the departure exactly where it was.
- **Ask which answer is correct before making two answers agree.** The ladder said neither was, which
  changed the argument from accuracy to consistency and made the choice defensible instead of
  arbitrary.
- **A claim a project makes about itself decays like any other.** ADR 0139's sentence was true of
  what it measured and false of what it claimed, and nothing in the tree could tell the difference
  until a process that could not count its own cores drew a page in one strip.
