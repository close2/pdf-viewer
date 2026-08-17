# 567 — The outline a shear made new every frame

2026-08-18. Takes two findings the quorra developers reported about this tree:
`render-lib/doc/notes-atlas-budget.md` §5 (their atlas repacks at period two on our single-list
`rasterize` path) and the consequence of their ADR 0057 (`bug1703683_page2_reduced.pdf` is drawable
at 4× now). ADR 0402 has the argument; this is what the round did and what it ran.

## The diagnosis, and the two guesses that were wrong

Theirs: `render_quorra`'s `ResourceCaches` "releases and re-uploads on that path
(`caches.begin_frame()` plus `evict_settled` after every frame)". Mine, before measuring: the
`Arc<Path>` cache must be missing because something rebuilds the display list. **Both wrong**, and
in the same way — each named a mechanism that would have made *every* resource unstable, and the
census says only one kind was.

`crates/render-quorra/examples/outline_stability.rs` settled it in two lines:

```
fills 450 (collapsing 0), images 0, groups 0
strokes 65859 — degenerate 0, dashed 0, anisotropic 65854
```

`issue12295.pdf` is not a page of glyphs and not a page of fills. It is sixty-five thousand
**strokes under an anisotropic placement**, which is the one branch of `stroke.rs` where §8.4.3.2's
note makes quorra's scalar device width wrong and this crate expands the outline itself with
`kurbo::stroke`. The result is computed geometry, so it went to `transient_outline` — a new
identifier every frame, by construction.

The lesson worth keeping is the shape rather than the branch: **a defect that looks like "the cache
is not working" was one route through the cache and four routes past it**, and a census of which
route each command takes cost ten minutes where reasoning about the cache cost an hour and got the
wrong answer twice.

## What was built

- `cache::StrokeKey` and a fifth map in `ResourceCaches` — keyed by the source path's address plus
  the width, tolerance, mitre limit, cap and join the expansion was made with; pinned by the source
  path, the same shape a reduced raster already had.
- `Encoder::expanded_stroke`, taking the expansion as a **closure**, so a hit skips `kurbo::stroke`
  and not merely the upload.
- `stroke::expanded` and its `Expansion`, which is the anisotropic branch lifted out of
  `stroke::encode` — code motion, forced by `clippy::too_many_lines` and worth having: the branch
  now carries §8.4.3.2's argument for its own existence in a doc comment instead of in the middle
  of a function.
- `pdf_render::LineCap` and `LineJoin` gain `Hash`. That is the whole change outside the crate.
- `tests/stable_ids.rs`, two device tests, **each confirmed to fail when the code it guards is
  removed** — the cache disabled makes the second frame upload again, the width dropped from the
  key makes two widths of one path draw the same pixels. Plus two in `cache.rs` that need no
  adapter: every argument moves the key, and an expansion nothing holds is released.
- `QuorraRasterizer::rasterize` fills `FrameCost` — see below, which is the half of this round that
  is not about strokes.

## The instrument that did not exist

`FrameCost::uploads` and `FrameCost::atlas_repacked` are both documented as the numbers that detect
exactly this defect. **Only `rasterize_frame` filled them** — the window-shaped path no gate in this
tree takes — so `Rasterizer::rasterize`, which the corpus gate, the oracle and every example run
through, answered `last_frame()` with a default. A plausible zero, on the one path everything is
measured on. That is why the finding arrived from the other side of the boundary.

## What was measured

`examples/outline_stability.rs`, `issue12295.pdf` page 1 at 4× (2448 × 3168), Radeon 890M, release:

| | frame 1 uploads | frames 2–8 | repack, 1 MiB atlas |
|---|---:|---:|---|
| before | 65 979 | 65 855 each | `. y . y . y . y` |
| after | 65 979 | **1** each | `. . . . . . . .` |

The repack column reproduces their 0,1,0,1 at the budget they measured it at. Wall clock,
interleaved A/B (two binaries alternating, four rounds of eight frames each, 28 timed repeat frames
per arm, both binaries built from the code that ships): **357.5 → 110.4 ms minimum, 535.7 → 140.5 ms
median, and the two distributions do not overlap** — the slowest frame after is half the fastest
before.

## What was run

The quorra corpus gate at 1× under `--profile gates`, with `REFUSED` and both differing lists held
to equality: unchanged, so the fix moves no pixels. The same gate at `PDFVIEWER_QUORRA_SCALE=4`,
which is where `REFUSED_AT_FOUR` is held — the run that verified the second finding rather than
taking it on report. Plus `cargo nextest run`, `cargo clippy --all-targets -- -D warnings` and
`cargo fmt --check`.

## The ratchet that was not touched

The second finding was "drop `bug1703683_page2_reduced.pdf` from `REFUSED_AT_FOUR`; ADR 0057 draws
it now". **The gate says it still refuses**, on the 16 384-row coverage-sheet ceiling, and all four
names hold in order. Nothing was changed.

The two are not in conflict. quorra's ADR 0057 landed on their `cafadeb`, 2026-08-17 00:50;
`Cargo.lock` pins `eada81ec`, 2026-08-16 21:08 — four hours and ninety-five commits earlier. Their
claim is about their tree and the ratchet is measured on ours. Taking the release is a round of its
own, and `doc/todo/02` §2 already says it owes both coverage lanes at 4×.

`REFUSED_AT_FOUR`'s doc comment now records the pending fix beside the refusal, so the next round
to take a release expects the name to leave rather than rediscovering why it did.
