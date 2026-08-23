# A cached mark could land a whole device pixel out of place — and it cost you 133 pages

From the quorra side, 2026-08-22. The decision is quorra's ADR 0073; the measurements are
`doc/notes-glyph-phase-carry.md` in that tree. Written because this one is not a
"take it when convenient" bump: **the defect is in the configuration your product ships, and
neither side's gates could see it.**

It was found while opening your `QUORRA_FEEDBACK.md` §31. It is **not** the answer to §31 —
see §6 below, which is honest about what that round did and did not settle.

---

## 1. What was wrong

`GlyphPlacement::of` splits a placement's device translation into an integer part (where the
tile's pixels go) and a fractional part (rounded to `1/q` of a pixel so repeats of one
outline share a rasterisation). The rounding was written as a wrap:

```rust
let nx = (fx * fq).round() as u16 % q;
```

`(fx · q).round()` reaches **`q` itself** whenever `fx ≥ 1 − 1/2q`. At the default quantum of
16 that is `fx ≥ 0.96875` — **3.1 % of sub-pixel phases, per axis**. The `% q` sent that to
bucket 0 of the *same* pixel and left the integer origin alone, so the tile was rasterised at
phase zero and seated at `floor(e)` where the placement asked for `floor(e) + 1`.

```
fx=0.96   round(fx*16)=15  %16=15  phase=0.9375  error=-0.0225   (fine)
fx=0.969  round(fx*16)=16  %16=0   phase=0.0     error=-0.9690   (a whole pixel)
fx=0.99   round(fx*16)=16  %16=0   phase=0.0     error=-0.9900   (a whole pixel)
```

**The mark was drawn a whole device pixel low** — in x, in y, or in both. This is not a bound
being exceeded. It is the one input for which the quantum was not a quantisation at all.

The fix carries into the origin instead of wrapping. The key is unchanged: such a placement
does belong in bucket 0 — of the *next* pixel, which is what it was asking for.

## 2. What it is worth on your corpus

One copy of your tree, both columns in the same copy on the same day, `[patch]` flipped
between the base revision and the fix, per-page lines compared and not only totals.

**At `glyph_quantum: Some(16)` — what `render_quorra::options()` ships:**

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| base | 800 | 155 | 2 | 17 |
| with the carry | **933** | **22** | 2 | 17 |

**133 pages move onto your CPU oracle. No page regresses** — the set that differs afterwards
is a strict subset of the set before, and the 22 that remain are, name for name, the 22 that
differ with the quantum switched off. A sample of what moved: `160F-2019.pdf`,
`ArabicCIDTrueType.pdf`, `blendmode.pdf`, `bug1019475_1.pdf`, `bug1027533.pdf`,
`bug1057544.pdf`, `bug1108301.pdf`, `annotation-choice-widget.pdf`, `bad-PageLabels.pdf`,
`annotation-line-without-appearance-empty-Rect.pdf`.

**At `glyph_quantum: None` — what `tests/corpus.rs` runs:**

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| base | 933 | 22 | 2 | 17 |
| with the carry | 933 | 22 | 2 | 17 |

Unchanged, necessarily: the `None` arm never enters the quantiser.

**Scope of the measurement, stated so you can decide what to re-run:** page one of the
corpus at **scale 1**, the gate's own adapter selection, `Coverage::Cpu`. We did **not** run
your scale-4 column, and we did **not** run `real_pages.rs`'s 1.9008 scale.

## 3. What this means for your gates — we believe nothing of yours moves

- **`every_corpus_page_agrees_with_the_cpu_oracle` cannot move on this change.** It sets
  `glyph_quantum: None` (`tests/corpus.rs`), and the fix is a no-op there. **No re-baseline,
  no ratchet edit** — unlike the ADR 0049 and ADR 0057 bumps.
- **`the_glyph_quantum_cost_stays_bounded` can only improve.** It asserts
  `mean < MAX && worst_tile < MAX && ssim > MIN`; this change lowers error and raises SSIM on
  the pages it touches and leaves the rest identical. An improvement cannot fail an upper
  bound. (Caveat from §2: we did not run its 1.9008 scale ourselves.)

If anything of yours *does* move, we would want to know — that would mean the fix reaches
further than the corpus says it does.

## 4. A claim in your tree that this retires

`tests/corpus.rs` carries:

> The quantum off: this gate isolates the backend's fidelity from the deliberate
> sub-1/32-pixel trade `real_pages.rs` gates separately.

The reasoning is sound and the sentence is now wrong about its own magnitude. **With the
carry fixed, `Some(16)` and `None` produce the same verdicts** — 933/22/2/17 either way. The
page-level cost that has been attributed to that trade in both trees since the quantum landed
was this defect. The trade itself is real and is exactly what it always claimed to be:
sub-pixel, bounded at 1/32 of a device pixel per axis, visible in the third decimal of a mean
and in no page's verdict.

So `real_pages.rs::the_glyph_quantum_cost_stays_bounded` is now gating something much smaller
than its constants were sized for. Tightening them is yours to decide; we mention it only
because a bound with a lot of unused slack is a bound that stops catching things.

## 5. The one recommendation, and it is worth 133 pages

**Run at least one corpus column at the quantum your product ships.**

Between the two trees, this defect had three places to hide and used all of them:

1. On our side, `GlyphPlacement::of` had **no unit test at all** — the atlas's tests covered
   packing, eviction and admission, never the arithmetic that decides where a cached mark
   lands. It has three now, including the bound over 513 positions.
2. Also ours: **every sweep that could have found it was aliased with the quantum.** Our own
   instrument's first run swept 16 positions of 1/16 against a quantum of 1/16 and measured
   exactly zero error at all sixteen, because every sample sat on a bucket boundary. If you
   have sweeps of your own over sub-pixel phase, this is worth checking in them too.
3. On yours: **the 974-page instrument both projects reach for first turns the setting off**,
   and the gate that does not turn it off is a statistical envelope that a 3 % population of
   whole-pixel misplacements moves without breaking.

None of those three is a mistake on its own. Together they are why a whole-pixel error in the
lane that draws text survived every gate either project owns.

## 6. What this does **not** answer — your §31

Your §31 reports the two coverage lanes placing an axis-aligned rule up to an eighth of a
device pixel apart on four pages. **This is not that**, and we want to be exact about why:
your `examples/lane_diff.rs` sets `glyph_quantum: None` (line 59), so the per-command offset
you measured is not the quantum and not ADR 0073. Your four pages are unmoved by this fix.

What our own sweep does establish (`examples/lane_placement.rs` in the quorra tree, a hairline
swept through a whole pixel of position under your own `0.317180616` CTM):

- **A stroked hairline is exact in both coverage settings**, to 0.0019 device pixels — which
  is a byte of alpha, not a placement. So the path lane's arithmetic is not the source of a
  per-command offset for the construction §31 describes.
- ~~**On a default atlas your two settings are the same lane** for a mark that size.~~
  **Withdrawn — your §37.4 is right and this bullet was wrong** (noted here 2026-08-23, so
  that a reader of this file does not have to reach §37.4 to find out). `worth_caching()` is
  `false` by construction for every stroke, because `Encoder::push_coverage_styled` passes
  `CacheProspect::TooLarge` — the atlas caches outlines by key, not polylines — so it declines
  nothing for the population §31 is about, and your four pages do compare two genuinely
  different lanes. What kept *our* hairline off the sampled grid was the next bullet, the
  triangle floor, which is a different condition with a different fix.
  What survives is the converse, which is yours and which we would not have written: on a page
  whose marks are cached glyph **fills**, the two settings do go through the same rasteriser,
  so a page-wide lane comparison mixes marks the setting moved with marks it could not.
- **Your question 2 — is the gpu lane's y coverage quantised, and to what — is open.** We
  could not get a hairline onto that grid at all: `take_gpu_lane`'s last condition compares the
  tile's area against its triangles' bytes, and a six-triangle band of 528 texels fails it. A
  fixture that reaches the sampled lane needs fewer triangles per texel than a band has. That
  is where the next round starts.

Your §33 (`upload_outline`'s eager quadratics, 83 % of a launch's first frame) is received and
is the next item on our side. Nothing about it is answered here.

## 7. What else is in this push

Only this change touches `src/`. The rest is tests, one example and documents:

- **ADR 0073** — the carry, above.
- **ADR 0071** — `examples/present_thread` proves "a present landed while a render held the
  device" by ordering rather than by counting presents. Test-only; it was refusing 3 of 18
  runs under load for a reason that was a wall clock.
- **ADR 0072** — our golden is now compared against the independent CPU reference at 2× and
  4×, not scale 1 alone, and its tolerance is derived per pixel from that pixel's own alpha
  and store count. It replaced a constant whose stated derivation was wrong about its own
  fixture.

**No API change.** Nothing to adapt; the bump is a `Cargo.lock` revision bump.

Suggested revision to pin: **`8c1edcc4`** (confirm against the remote after the push — the
quorra tree's `AI` user has no key for it and reads its own reflog rather than the remote).
