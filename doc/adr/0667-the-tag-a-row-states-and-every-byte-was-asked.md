# ADR 0667 — The tag a row states, and every byte that was asked for it

Status: accepted, 2026-08-25. Session 752. §7.4.4.4's PNG row filter is hoisted out of the
per-byte loop: `Document::open` of ISO 32000-2 falls **11.15%**, byte-identically.
Clause: ISO 32000-2 §7.4.4.4, whose ledger row moves with it.

## How this was found, which is the part worth keeping

It was not looked for. ADR 0666's A/B of the release profile printed a per-function attribution of
`callgrind_open` under four builds, and the function whose *inlining* moved between them was
`pdf_syntax::filter::apply_predictor` — inlined into `decode_with_parms_reported` under a fat
link, standing alone at 238.6 M of 857.1 M instructions under a thin one.

Two things follow, and the second is the finding:

- **§7.4.4.4's predictor is roughly a quarter of `Document::open`**, on the largest document in
  this tree, and no document in this project said so. ADR 0180 attributed this path's cost to
  §7.5.6's "most recent copy" rule and to one allocation per object; the predictor sat underneath
  that measurement, unnamed, for four hundred rounds.
- **A hot loop whose cost swings by double digits with the optimiser's mood is a loop written to
  depend on the optimiser.** That is what made it worth reading rather than merely recording.

## What the loop was doing

`apply_predictor` walked every byte of every row and, per byte, fetched the left, upper and
upper-left neighbours and then ran `match tag` to decide which of them to use.

§7.4.4.4 defers the algorithms to ISO/IEC 15948, where **the type byte is a property of the row**:
one tag in front of each row selects one of five filters for all of that row's bytes. So the
per-byte `match` asked, once per byte, a question whose answer could not change within the row.
Worse, the three neighbour fetches were unconditional, so filter type 0 (`None`) and type 2 (`Up`)
— the latter being what essentially every cross-reference stream uses, via `/Predictor 12` — paid
for a `left` and an `up_left` they never read.

A cross-reference stream is the population that cares, and it is the shape ADR 0180 already
described in the comment above this loop: **one six-byte row per object, and ISO 32000-2 has
101 318 of them.**

## What changed

The tag now selects a loop. Each of the four filters that do work is a small named function with
its clause cited, and type 2 — the one on the launch path — reduces to a walk of two slices in
step, with no tag to re-test, no left neighbour to fetch and no bounds check:

```rust
fn unfilter_up(row: &mut [u8], above: &[u8]) {
    for (value, &up) in row.iter_mut().zip(above) {
        *value = value.wrapping_add(up);
    }
}
```

**No arithmetic changed.** The change moves *where the tag is examined* and nothing else, which is
what makes the output byte-identical by construction rather than by measurement.

## The one behaviour that is not obvious, and is preserved

`data.chunks(stride)` can yield a final chunk of a single byte — a tag with no row after it. In
the per-byte form the inner loop never ran for such a chunk, so **its tag was never validated**:
an undefined tag on an empty row was accepted and contributed nothing, where an undefined tag on a
non-empty row returned `None`. A naive hoist turns that into a refusal and changes what a
malformed file decodes to.

The match is therefore on `(tag, copy)` with `(_, 0)` first, and the comment says why. This crate's
whole subject is malformed input, so that is a behaviour to pin rather than to tidy.

## How byte-identity was checked, and how the check was checked

A differential example held the previous implementation verbatim beside the new one and compared
them over 200 000 generated cases — predictor codes 2 and 9 to 16, `colors` 1 to 4,
`bits` ∈ {1,2,4,8,16}, `columns` 1 to 9, data lengths 0 to 59 with tag bytes biased to 0..6 so the
defined filters are actually reached, and lengths that land before, on and after a row boundary.
**0 disagreements.**

Trap 13 says a sweep for a defect must be run against the defect before it is believed, and trap
746's lesson is that a plant must be asymmetric enough to fail. Three plants:

| plant | caught |
|---|---|
| `(_, 0)` dropped, so an undefined tag on an empty row is refused | 2190 cases |
| Paeth's `left` and `up` arguments transposed — asymmetric, unlike `midpoint` | 5102 cases, with C |
| `Sub`'s loop starting one byte late | (same run) |

The restored tree returns 0 again. The example was temporary and is not committed; what is
committed is the two permanent tests below.

## Two gaps the split made visible

Separating the filters into named functions exposed things the fused loop had hidden:

- **PNG filter type 4, Paeth, had no test at all.** `each_png_row_carries_its_own_filter_type`
  covers types 0, 1, 2 and 3 and stops. `the_paeth_filter_chooses_between_its_three_neighbours` is
  new, and it is built so that its three positions answer **up**, **up-left** and **left** in turn
  — because which neighbour the predictor returns is the whole of the filter, and a case that
  happened to pick the same one every time would also pass against a decoder that implemented
  `Up`. Its expected bytes are derived from ISO/IEC 15948's rule, position by position, in the
  test's own comment; principle 5's requirement is that an expected value comes from the
  specification, and this one does.
- **The trailing-tag behaviour above had no test.** It has one now, and the plant proves the case
  is reachable.

## What it is worth

Callgrind, `--profile release`, exactly reproducible across repeat runs:

| | before | after | |
|---|---:|---:|---|
| `callgrind_open` — ten opens of ISO 32000-2 | 763,278,781 | 678,200,421 | **−11.15%** |
| per `Document::open` | 76.33 M | **67.82 M** | −8.51 M |
| `callgrind_interpret` — page 101 ×50 | 1,294,054,067 | 1,285,546,294 | −0.66% |
| `callgrind_rasterise` — page 101 | 5,448,437,924 | 5,450,359,321 | +0.035% |

**The attribution is exact.** `decode_with_parms_reported`, which carries the inlined predictor,
goes 188,250,760 → 103,172,400 — a drop of **85,078,360**, which is the whole change in the total
to the instruction. `xref::read_section` is 251,086,670 in both. Every other row of the profile is
unchanged.

The `callgrind_rasterise` figure is a real +1.9 M rather than noise — the instrument is exact — and
it is code layout: that page decodes no predicted stream after its open, and a changed binary
shifts inlining decisions elsewhere. It is recorded rather than explained away.

## What this does not do

- **It does not vectorise anything by hand**, and it should not. `unfilter_up` is a form the
  compiler can vectorise; writing SIMD for it would be `unsafe` in the crate that most wants
  `#![forbid(unsafe_code)]`, and this is the same refusal `doc/performance.md` records for
  `memchr` on §7.3.8.2's `endstream` search.
- **It does not touch the TIFF predictor**, which has no per-row tag and never had the defect.
- **It does not remove the remaining bounds checks in `Sub`, `Average` and `Paeth`.** Those three
  need a neighbour at `index - bpp` and the `saturating_sub` that expresses it defeats the
  elision. They are no worse than before and the case that matters — the one a cross-reference
  stream takes — is now free of them. Whether the other three are worth a structural rewrite is
  not answered here, and no corpus document ranks it.
