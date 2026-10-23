# 2277 — The family's reading is told by allocation, and priced by population

Status: accepted for integration. Session 1721, 2026-10-23.
Context: ADRs 2223, 2249, 2265 (the protocol), 2276 (the decision), 2012 (where the family is read).

## Identity

A selection is the `DeviceCMYK` family where `ColourSpace::device_family` answered it, and the
interpreter cannot ask the parser which branch answered. It does not need to: that branch hands
out the group's or the output intent's own profile allocation, and `DeviceCmykReadings` compares a
selected `ICCBased` space against those two by `Arc::ptr_eq`. Every other route to a selected
`ICCBased` space parses its profile afresh — a §8.6.5.6 default, an `Indexed` base, a tint
transform's alternate, a resource stating `ICCBased` — so the test is exact, and a failing profile
read as its `/Alternate` `DeviceCMYK` (ADR 2012) reads as the family because it is. The one
allocation two routes share is a space `Interpreter::icc_spaces` remembers, which
`blending_space_of` also answers a group's `/CS` from; a remembered space is asked about the output
intent alone, and an `sc` on a whole `ICCBased` space takes its family from the source its `cs`
left (`SolidSource::Cmyk` or not). What that leaves unread is a damaged `ICCBased` resource whose
alternate is the family, selected a second time inside the same group and painted in a nested
group of another profile.

`SolidSource::Cmyk` keeps ADR 2265's identity by value; the memo's context key already holds the
group's and the intent's profile identities, which are what the paint reads the family by.
`SpaceSource` and `TintSource` carry the reading as an `Option<ColourSpace>`, a reference count,
and `read_where_painted` clones the space only where the paint reads the family otherwise.

## Proof

The four controls of ADR 2276, and four mutations each planted alone in an isolated export with
its own target, each failing on assertions (exit 101): a whole space painted without
`read_where_painted` (two pixels, the `Separation` in both controls), an uncoloured pattern painted
without it (two), every four-channel `ICCBased` read as the family whatever its allocation (one, the
default kept), and the paint's ranking with the intent before the group (twelve).

## Cost

Six balanced pairs of `black_generation_order --icc-cost`, pinned to CPU 2 after 60 s of
settling, in one `--clock` hold with the turn gate; all 12 sides exit 0 at load 1.16–2.21, and
every population's sample is identical across the two release binaries, whose only production
difference is `content/colour.rs`, `content/pattern.rs` and `pattern/tiling.rs`. The eight
populations named `*-oi-*`, `sep-cmyk-*` and `tint-cmyk-*` are new to the instrument and are in
both binaries: a page under an output intent whose profile is a small generated press, its group
stating the same profile stream, so that the two binaries draw the same pixels and what is
measured is the path alone. Paired medians per 20 000 marks (20 interpretations of 1 000):

| Population | Before ms | After ms | Paired ratio | Delta ms | Ratio range |
|---|---:|---:|---:|---:|---:|
| cmyk-oi-repeat | 5.241 | 4.790 | 0.91311 | -0.451 | 0.88092–1.06188 |
| cmyk-oi-select | 16.581 | 16.071 | 0.96486 | -0.510 | 0.94736–1.00882 |
| cmyk-oi-restore | 13.530 | 13.126 | 0.97379 | -0.404 | 0.92891–1.03647 |
| cmyk-oi-inherit | 26.953 | 26.571 | 0.98334 | -0.383 | 0.94634–1.03615 |
| sep-cmyk-select | 11.845 | 11.936 | 1.00181 | +0.091 | 0.98248–1.03418 |
| sep-cmyk-restore | 11.509 | 11.688 | 1.01500 | +0.178 | 0.98398–1.05122 |
| sep-oi-inherit | 24.091 | 25.497 | 1.05464 | +1.406 | 1.02688–1.10412 |
| tint-cmyk-repeat | 40.537 | 41.258 | 1.00234 | +0.722 | 0.98885–1.08648 |
| cmyk-select | 35.084 | 35.470 | 1.01043 | +0.386 | 0.95505–1.04773 |
| cmyk-restore | 21.305 | 21.208 | 1.00183 | -0.097 | 0.89932–1.01494 |
| icc-tint-values | 49.207 | 51.641 | 1.03883 | +2.434 | 0.99703–1.07094 |
| icc-solid-repeat (untouched) | 5.473 | 5.178 | 0.94475 | -0.295 | 0.71111–0.95712 |

A `k` under an output intent got cheaper — about 20–25 ns a mark — because it is now four inline
floats where it was an `Rc<IccSource>` allocated at each selection. A `Separation` over the family
costs 5–9 ns a selection for the walk that finds its reading. The one population that pays is a
`Separation` restored inside a group that reads the family otherwise: `read_family_as` clones the
space at each memo miss, about 70 ns a mark (5.5 %), on a construction no corpus page is known to
repeat; caching the replaced space in the source would buy it back at a cell of state per source,
and is not justified by this figure. The other 39 populations read paired ratios between 0.906 and
1.034, and `icc-solid-repeat`, whose path the change does not reach, reads 0.945: that is the
instrument's floor on this pair of binaries, and the uncoloured `ICCBased` tint populations, whose
`TintSource` gained the field and no work, sit at its edge. These are whole interpretations of
generated fixtures, not conversion timings. `t2-turn_path` in the same hold: exit 0, 33 figures
banded, 33 judged, 0 outside, at load 2.07–7.94.
