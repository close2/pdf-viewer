# 1324 — An arc is cut as cubics within a stated bound, and a matte's mask follows a reduced grid

Status: accepted and **built**.
Context: `crates/pdf-transform/src/redact.rs` (`ARC_TOLERANCE`, `Walk::stroke_outline`),
`crates/pdf-transform/src/redact/paths.rs`, `crates/pdf-transform/tests/redact.rs`
(`a_stroke_whose_outline_holds_an_arc_is_cut_at_the_region_edge`); `crates/pdf-model/src/image.rs`
(`alpha_on_grid`, `prematte_alpha`, `decode_jpx`), `crates/pdf-model/tests/matte_reduced.rs`.
Supersedes: ADR 1236's refusal of a stroke whose outline holds an arc; ADR 1268's shortfall for a
`/Matte` on a `JPXDecode` parent decoded at a reduced level.

Two refusals stood on the same premise — that a construct re-expressed by this program is not the
producer's — and each falls to the same question: what bound does the standard already accept?

## 1. §12.5.6.23: an arc is cut

ADR 1236 refused a round cap, a round join and a stroked curve because their outline comes back
from the expansion as an approximation. But no path can state a circle — "[c]urved path segments
shall be specified as cubic Bézier curves" (§8.5.2.2) — so §8.4.3.3's "semicircular arc" is an
approximation in every reader, and §10.7.2 states curve rendering as "an approximation" to "the
mathematically correct path" within a device tolerance. What a redaction owes outside the region is
the producer's mark to a bound, not byte-identity, and the polygonal case already accepts one (the
writer's decimals and §7.3.3's single-precision reading, which `Cut::margin_holds` proves).

**The choice**: the outline is fitted within `ARC_TOLERANCE`, a hundredth of `REGION_PAD`
(1e-4 in the display list's units), carried into the path's user space by the mapping's norm, and
the cubics are cut at their roots exactly as ADR 1236 cuts a Bézier. It bounds fidelity only: the
cut is exact on the fitted outline, so nothing written can lie in the region whatever the value.

**Measured against the clause, not against the oracle**: at 150 dpi the redacted round cap and
stroked curve sit within one step of the analytic coverage of §8.4.3.3's semicircle and of the set
within half the width of §8.5.2.2's polynomial, where `render-cpu`'s own stroker departs by up to
eight. The test holds every changed pixel to that geometry, and fails at a tolerance of 30 ×
`REGION_PAD`.

**Unchanged**: a `JPXDecode` image decoded at a reduced level is still refused by the redaction. The
region maps onto the reduced grid by the same matrix, and a reduced write-back would remove "all
traces" of it — but it would resample every sample *outside* the region too, which is content the
annotation did not identify, and the budget that reduced it is principle 3's and is not lifted
for one verb.

## 2. §11.6.5.2: the mask is carried onto the reduced grid

Table 143 pairs a `/Matte`'d mask with the parent's stated grid. §7.4.9 NOTE 3's reduced decode is a
display choice under the worker's budget; the mask's samples are the file's. Pre-blending is linear
in the colour, so a reduced parent sample is `m + mean(α(c − m))` over its footprint, and dividing
by the mask's mean over the same footprint gives the opacity-weighted mean colour — what the full
grid, undone and reduced premultiplied, would draw. `alpha_on_grid` takes that mean. It is exact
where the codestream's reduction is the footprint's mean and within the wavelet filter's departure
from one elsewhere, which is the approximation the reduced level already is. The same function
carries a mask whose *own* decode came back on another grid onto the parent's, which the route used
to pair by position unchecked.

The DCT route's shortfall stays: a JPEG frame whose dimensions disagree with the dictionary is the
file's contradiction (§7.4.8), not a display choice.
