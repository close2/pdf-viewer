# ADR 0363 — A set and a value fold into one buffer, and only one of them belongs there

Date: 2026-08-15 (session 528)
Status: accepted

## Context

ADR 0355 composed a *clipping region* with a filled mark's own coverage by `min`, and named what
it did not take:

> A clip folded into a soft mask keeps multiplying, which is the witness's remaining factor above
> and is the cheapest of these to take next.

`scan::Clip` grew three variants there — `Unclipped`, `Region` and `Value` — and `Value` declined
the intersection on a reason that reads well: §11.6.5's soft mask *is* a product the standard
states, and `MaskCache::combine` had already multiplied the clip into it, so what arrived at the
draw was one buffer with no set left inside it.

The reason is right about the soft mask and wrong about the fold. **The standard states the two
in an order**, and the order is what the fold destroyed.

## The clause, in three sentences that have to be read together

§8.5.4, on what a clipping path constrains:

> The effective shape is the intersection of the object's intrinsic shape with the clipping path;
> the source shape value shall be 0.0 outside this intersection.

§11.6.4.2, on what that intrinsic shape is for a fill or a glyph:

> For objects defined by a path or a glyph and painted in a uniform colour with a path-painting or
> text-showing operator (8.5.3, "Path-painting operators", and 9.4.3, "Text-showing operators"),
> the shape shall always be 1.0 inside and 0.0 outside the path.

§11.3.7.2, on what then happens to it — with the soft mask as one of the three inputs:

> The three shape inputs shall be multiplied together, producing an intermediate value called the
> source shape.

So the clip meets the object's **own** shape, by intersection, and the result of *that* is what
the mask shape multiplies. Written out, with `fⱼ` the object shape, `C` the clipping path and
`fₘ` the mask shape:

```text
   fₛ = (fⱼ ∩ C) · fₘ          and not          fⱼ · (C · fₘ)
```

Two footnotes, both load-bearing:

- **The mark's fractional coverage is `fⱼ` and the standard says so.** §11.3.7.2's NOTE 1: "when
  such objects are rasterized to device pixels, the shape values along the boundaries can be
  anti-aliased, taking on fractional values representing fractional coverage of those pixels. When
  such anti-aliasing is performed, it is important to treat the fractional coverage as shape rather
  than opacity." This tree's departure (1) is therefore *inside* the model rather than beside it,
  which is what makes the identity above bind on a boundary pixel at all.
- **`/AIS` does not change the answer.** §11.6.4.3 lets the same mask be read as `fₘ` or as `qₘ`,
  and §11.3.7.1 defines alpha as the product of shape and opacity, so the mask multiplies the
  clipped shape either way. Nothing here depends on which of the two rows of Table 136 it lands in.

## The closed form, and why it needs no third buffer

The cache holds `P = C · S` because that is what every draw but a fill wants. The composition
wants `min(M, C) · S`, with `M` the mark's own coverage. Multiplication by a non-negative value is
monotone, so it distributes over a minimum:

```text
   min(M, C) · S  =  min(M · S, C · S)  =  min(M · S, P)
```

— so the soft mask's own rows beside the product are the whole of what the composition needs, and
the clip's region by itself is never wanted. **Eight-bit rounding costs nothing here either**,
which is worth stating rather than bounding: rounding is monotone too, and a minimum commutes with
a monotone map, so `min(round(M·S), round(C·S))` *is* `round(min(M·S, C·S))`, exactly.

The unit ladder, a half-plane whose edge falls at device 2.25 under a clip with the same edge and
a soft mask of 128 of 255 — `scan.rs`'s `a_clip_folded_into_a_soft_mask_still_takes_nothing_from_
the_mark`:

```text
   the mark's own coverage, unmasked and unclipped   192 of 255
   the mark under the soft mask alone                 96          = round(192 × 128 / 255)
   the product taken as a value, which was drawn      72          = round(192 ×  96 / 255)
   min(M · S, C · S), which is drawn now              96          — the clause's own (fⱼ ∩ C) · fₘ
```

### A second ladder, because the first one's clip is coincident with its mark

**Every scene written for this round put the clip's edge on the mark's, and on that axis the
composition cannot be told from one that never applies the value at all.** It is arithmetic rather
than luck: where `C = M` the wrong form `min(M, C·S)` is `min(M, M·S)`, which is `M·S` because
`S ≤ 1` — the right answer, arrived at by the scene rather than by the code. Deleting `scaled` from
`intersected` leaves all three of the first scenes green, which is trap 2's fifth instance
("a scene must fail at the defect's magnitude as well as in its axis") in the axis ADR 0046 names:
ask what parameter every scene leaves at its default, and here it was the clip's *coincidence*.

So a second ladder offsets the two edges by half a pixel inside one column, where the mark's own
coverage falls below the product and the two forms part —
`a_clip_that_contains_the_mark_takes_nothing_from_it_under_a_soft_mask`, the mark at device 2.75
under a clip at 2.25:

```text
   the mark's own coverage                             64 of 255
   the clip's                                         192
   the product C · S, which bounds the composition     96          = round(192 × 128 / 255)
   min(M · S, C · S) = M · S, drawn now                32          — the mark lies inside the clip
   min(M, C · S), the value never applied to the mark  64          — what the first three scenes miss
   M · P, the product taken as a value                 24          — what was drawn before
```

## Decision — `MaskCache` keeps the soft mask's rows beside the product, and `scan::fill` composes

`Built` gains a `value` field holding the soft mask's own values over the product's rows; `combine`
already assembled exactly those rows to build the product and now keeps them instead of dropping
them. `scan::Clip::Both` carries the pair, `Clip::mask()` still answers the product so that every
non-fill draw is untouched, and `intersected` takes `(admitted, value, scratch)` with `Region` as
the case `value = None`, `S ≡ 1` — one code path, since a clip alone is a clip beside a mask of
one everywhere.

**The decline test generalises with it.** ADR 0355 declined where the clip was already a set —
0 or 255 under the mark, where a product *is* an intersection. With a mask beside it the clip is a
set exactly where `P` is 0 or `S`, and that test is the old one when `S` is 255.

**It is inexact in one direction, by less than half a level, and that is a bound rather than an
observation.** `P = round(C·S/255)` equals `S` for any `C ≥ 255 − 127/S`, so a faint mask can read
as a set where the clip is not one. The two compositions differ there by `S·(min(M,C) − M·C/255)/255`,
and the same inequality holds that under `½` for every `M`: if `M ≤ C` it is at most
`S/255 · M(1 − C/255) ≤ 127.5/255`, and if `M > C` at most `S·C/255 · (1 − M/255) ≤ 127.5/255`. Half
a level is below what an eight-bit raster can hold, so declining there costs the page nothing it
could show.

An entry that holds both now costs twice a band's bytes, charged to the same budget through
`Built::held`.

**The rounding is one function and not two copies**, `scan::scaled`, which `MaskCache::combine`
also builds the product with. The exactness above holds only while both sides of the `min` round
the same way, so an agreement a comment asks for is an agreement a call guarantees.

## What it moves in the corpus: **nothing**, and the population says why

Instrumented over the 974 first pages of the pdf.js corpus:

```text
   commands taking a clip and a soft mask together            120
   of those, fills reaching the composition                    27
   declined because the clip is already a set under the mark   14
   composed                                                    13   over five documents
```

The five are `bug1703683_page2_reduced.pdf`, `bug1721218_reduced.pdf`, `issue16287.pdf`,
`issue17069.pdf` and `issue18032.pdf`, and **each of the ten rasters — page one at scale 1 and at
4× — is byte-identical before and after**, ink to four decimals included. The composition fires
thirteen times and changes not one pixel, because at all thirteen the mark is *whole* where the
clip is fractional: `min(M, C) · S` and `M · C · S` part only where both `M` and `C` are
fractional in the same pixel, and no corpus first page states a fill whose own anti-aliased
boundary lands there with a soft mask in force.

So this round answers the **coverage** question and not the robustness one, which `CLAUDE.md`
states as a case rather than an excuse: a corpus cannot rank a requirement no document exercises.
What the round has instead of a moved page is a closed form, two unit ladders — one failing by 24
levels of 255 with the composition removed and 8 with the value dropped from it, the other by 8 and
32 respectively — and a display-list scene that fails by 28.

## The finding that is worth more than the change: **ADR 0355 named the wrong residual**

ADR 0355 said of `issue21346.pdf` — ADRs 0279's, 0280's and 0355's witness — that "[t]he mark
carries a soft mask *and* a clip, and `MaskCache::combine` multiplies the two into one buffer
before either reaches a draw — so what arrives at `scan::fill` is a `Clip::Value`". **Nothing on
that page arrives at `scan::fill` that way.** Instrumented, its page one takes the clip-and-mask
pair exactly twice, and both times the consumer is a *group's* blit — `rows.draw_pixmap(…,
clip.mask())`, the isolated separable-blend path — while the only two marks that reach
`intersected` are `Clip::Region`s with no mask at all. Its similarity is 0.9846 before this round
and 0.9846 after, and the ledger's and `doc/todo/11`'s account of what holds it down has been
wrong since ADR 0355 wrote it.

**And the group case is owed, where ADR 0355 recorded it as not owed.** That ADR said "what a
group's buffer carries at a pixel is §11.4.5's group alpha rather than one mark's coverage, so
there is no second shape here for §10.7.4's intersection to be taken with". §8.5.4's own third
sentence contradicts it:

> Similarly, the shape of a transparency group (defined as the union of the shapes of its
> constituent objects) shall be influenced both by the clipping path in effect when each of the
> objects is painted and by the one in effect at the time the group's results are painted onto its
> backdrop.

(`doc/md` splits that sentence's last word as "bac kdrop", which is the conversion rather than the
standard; the quote above is the document's.)

A group *has* a shape, the standard defines it, and the clip in force at the blit intersects it.
What is true is the narrower statement: this backend's group buffer carries **alpha**, which is
shape times opacity (§11.3.7.1), and the two coincide only where every element's opacity is 1 —
which §11.6.4.2 makes the default and `ca`, `CA` and a nested soft mask make false. So the
construction is owed and what it needs is a *shape channel beside the group's raster*, which is
`doc/todo/11`'s to price and is not this round's.

**It is worth measuring rather than asserting**, and it is: with the group's raster meeting the
clip as a set instead of by the product — the identity's own answer, since that page's group lies
inside its clip — `issue21346.pdf`'s device column 14 of row 89 goes `(240, 245, 249)` to
`(227, 237, 244)` against an interior of `(206, 223, 235)`, which is **0.306 → 0.571** of the mark
where departure (1) gives 0.827 and the clause gives 1.000. So the group blit is one of the
witness's remaining factors and not the last of them.

**What was measured is `min(group alpha, clip)`, the blit then carrying no mask**, with the
band offset the buffer and the mask disagree by taken into account — the buffer covers the whole
surface and the mask is band-tall, which on this page is thirteen rows apart. Alpha stands in for
the shape it is not, so the figure prices the direction rather than the answer, and that is the
same approximation the paragraph above says has to go away.

## What it costs, measured

`examples/callgrind_rasterise`, instructions for one page, `RAYON_NUM_THREADS=1`:

| page | before | after | |
|---|---|---|---|
| ISO 32000-2 page 101, a page of text | 5 517 804 054 | **5 511 329 099** | **−0.12%** |
| `bug1721218_reduced.pdf`, 3554 clips | 20 722 234 743 | **20 777 904 644** | **+0.27%** |
| `knockout_smask.pdf` | 259 867 644 | **261 449 950** | **+0.61%** |
| `issue21346.pdf`, the witness | 345 497 608 | **345 670 848** | **+0.05%** |
| `issue18529.pdf` | 25 461 847 | **25 489 779** | **+0.11%** |

The page of text has no clip-and-mask pair at all and its −0.12% is the layout noise of a rebuild;
what the other four price is one more slice compare per composed row plus the `u16` multiply, and
the buffer ADR 0355 already reuses is what keeps it there. **The heaviest of them is the one to
read**: 3554 clips for a quarter of a percent, against the +54% ADR 0355's first version cost on
that same page before it reused a buffer.

The same binaries, run without callgrind, print the page's own ink, and it is **identical to the
digit** on all five documents that compose and on the three measured beside them — which is the
raster-level form of the gate result below.

## The gates

Every one of them is byte-identical across the change, per page and not only in total: the
oracle's 1794 lines (906 agree / 67 contradicted / 786 ambiguous / 13 not comparable / 19 no
render), `doc/todo/00` step 7's ink sweep over all 786 ambiguous pages — 0 rows moved of 786, with
the negative tail's four complete names in the same order — the cross-backend gate at page scale
(930 agree / 24 differ / 2 refused / 18 not comparable) and its magnified lane
(`PDFVIEWER_QUORRA_COVERAGE=gpu PDFVIEWER_QUORRA_SCALE=4`: 937 / 9 / 5 / 23), and the corpus, text,
dates, XMP and JPEG 2000 summaries. `doc/QUORRA_FEEDBACK.md` §24's ask is unchanged in its
substance and gains the soft-mask half of it, because quorra folds the two the same way and no
corpus page can currently tell either backend's fold from the other's.

## The alternatives, and why neither was taken

- **Keep the clip's region as a third buffer.** It makes `is_a_set` exact rather than
  half-a-level loose, and costs a third of a band's bytes per entry plus a rebuild whenever the
  clip's own entry has been evicted — which would make what is drawn depend on the cache's
  eviction order, and this backend is the oracle.
- **Recover `S` from `P` by division.** `S = P / C` needs `C`, and `C = P / S` needs `S`; either
  way the recovery is exact nowhere and worst exactly where the mask is faint, which is where the
  half-level bound above is already tight.
