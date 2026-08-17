# 0402 — The outline a shear made new every frame

Status: accepted
Date: 2026-08-18
Session: 567

Takes two findings the quorra developers reported about *this* tree — `render-lib`'s
`doc/notes-atlas-budget.md` §5 and its `doc/adr/0057`'s consequence — and settles both: the first
by fixing it, the second by running the gate and declining the change it asked for. Amends
`crates/render-quorra`'s cache, stroke encoder and cost reporting.

## Context: what the library on the other side of the boundary could see and we could not

quorra keeps a glyph atlas, and every tile in it is keyed on the identifier of the outline the
mark came from. Its ADR 0050 bounds a repack at one per changed page: a page that changes settles
after at most one, and `Counters::atlas_repacked` reading `true` frame after frame is the
pathology that bound was written to make visible.

Measuring their own corpus profile they found ours doing exactly that, and wrote it down:

> **On the caller's single-list `rasterize` path it oscillates with period two.** `issue12295.pdf`
> at 4× against a 1 MiB atlas — working set 742 008 bytes, so squarely inside ADR 0050's band —
> nine frames of the same page:
>
> ```
> frame   0    1    2    3    4    5    6    7    8
> repack   0    1    0    1    0    1    0    1    0
> ```
>
> For ever. […] The driver is that the page's atlas keys are not the same from one render to the
> next: on a fresh device the same page's two frames left 66 261 and then 132 097 entries, i.e.
> the second render inserted 65 836 keys the first had already inserted. […] It was not isolated
> further: doing so means instrumenting their cache, which is their tree.

Their diagnosis of the mechanism was right and their guess at the cause was not — they named
`ResourceCaches`' `begin_frame` and `evict_settled`, which are innocent: an entry the current
frame looked up is never evicted, and the eviction pass only runs against the device's own byte
budget. The instability was one level along.

## What the measurement said

`crates/render-quorra/examples/outline_stability.rs` is the instrument, and its first two lines
are the whole finding. `issue12295.pdf` page 1 at 4× on the Radeon 890M:

```
fills 450 (collapsing 0), images 0, groups 0
strokes 65859 — degenerate 0, dashed 0, anisotropic 65854
```

The page is not a page of glyphs and not a page of fills. It is **sixty-five thousand strokes,
all but five of them under an anisotropic placement** — and that is the branch of
`crates/render-quorra/src/stroke.rs` that does not hand quorra a stroke at all.

§8.4.3.2's own note says the thickness of a stroked line in device space varies with its
orientation, so a scalar device width is exactly wrong under a shear or an uneven scale. quorra
widens from one scalar, so wherever the placement's anisotropy exceeds `MAX_ISOTROPY_ERROR` this
crate expands the stroke itself, in path space, through the same `kurbo::stroke` the Vello backend
uses — and then filled the resulting outline. That outline is **computed geometry**, so it went
through `Encoder::transient_outline`: uploaded, drawn, and released at the end of the frame.

A transient is by construction a new identifier every frame. Every one of the page's 65 854 tiles
therefore arrived at the atlas with a key nothing had seen, the atlas filled twice over, and the
repack that cleared it handed the next frame an empty sheet to do it again. The window path never
showed it because a window *retains* its scene and does not rebuild it; the single-list
`rasterize` path — which is every gate in this tree, and every page the oracle judges — rebuilds
on every call.

## Decision 1 — an expanded stroke outline is cached, keyed by its source and its arguments

`ResourceCaches` gains a fifth map. Its key is `StrokeKey`: the source `Arc<Path>`'s address, plus
every value that decided what `kurbo::stroke` produced from it — the path-space width, the
flattening tolerance, the mitre limit, the cap and the join. The pin is the **source path**, which
is the same shape `store_image` already has for a reduced raster, and it carries the module's ABA
argument unchanged: the key is an address and the pin is what keeps that address unique, whatever
the bytes on the device look like.

Two things about the key are deliberate.

**It is the arguments, not the expansion.** Hashing the outline itself is the obvious alternative
and it is worse twice: it walks geometry that is usually larger than the path it came from, and it
has to be *computed* before it can be looked up — where this key is what lets a hit skip the
expansion altogether. So `Encoder::expanded_stroke` takes the expansion as a closure and a hit
never runs `kurbo::stroke`. That is the larger half of what the cache buys.

**The tolerance is in the key, and it must be.** It is a quarter device pixel expressed in path
units, so it is a function of the placement's stretch: a page drawn at two magnifications genuinely
expands its curves twice, and a key that omitted it would draw a zoomed page from the flattening of
an unzoomed one.

**What is not cached is stated rather than absorbed.** A **dashed** or **degenerate** stroke has no
stable source to key on — `kurbo::dash` and `split_degenerate` produce a `Path` this frame built —
so those stay transients, which is exactly the division the scalar branch beside it already makes
for the same reason. That is a limit rather than an invariant, and `tests/stable_ids.rs` says so
in prose and deliberately does not assert it: a test that failed when somebody widened the cache
would be a ratchet pointing the wrong way.

**One consequence is a weakening and it is written down where it lives.**
`ResourceCaches::drop_unreachable` used to be a *proof*: a pin held by nobody else meant no display
list anywhere carried that allocation. One source path can now be pinned by two entries — its own
upload and an expansion of it, or two expansions at two widths — so such a group keeps itself alive
and the pass is now conservative. Nothing leaks: those entries stop being used, so the budget pass
takes them oldest-first like any other settled entry. What would buy the promptness back is a
reference count of this cache's own, which is a second bookkeeping of a thing `Arc` already counts.

`pdf_render::LineCap` and `LineJoin` gain `Hash`, which is the whole of the change outside this
crate.

## Decision 2 — the single-list path reports what it cost

`FrameCost` existed. `FrameCost::uploads`' own rustdoc says it is "a count rather than a duration,
and the reason is in `crate::cache`: the caches are keyed by `Arc` identity, so a display list
rebuilt from scratch every frame re-uploads everything, and a number that stays high on a page that
has not changed is what says so." `FrameCost::atlas_repacked`'s says "`true` frame after frame is
the pathology".

**And only `rasterize_frame` — the window-shaped path no gate in this tree takes — ever filled
either of them.** `Rasterizer::rasterize`, which the corpus gate, the oracle, `tests/corpus.rs`
and every example run through, computed none of it and `last_frame()` answered with a default.
So the one path this project measures everything on was the one path with no instrument attached,
and the defect was found by the library on the other side of the boundary instead.

`QuorraRasterizer::render` now takes the cost as an out-parameter and fills it as it goes — the
same bargain `present::FrameSlot::render` already keeps with its caller, so a frame that *refuses*
still reports what it uploaded and settled before refusing. §11.4.7's four-component page is two
renders and the two costs add, through `FrameCost::add`, which existed for the window's two lanes.

**The general lesson, and it is not about strokes.** A field whose documented purpose is to detect
a class of defect detects nothing on a path that does not fill it, and *nothing says so* — the
default is a plausible zero. Ask of every reported number which entry points compute it, and
whether the ones a gate actually takes are among them.

## What it is worth

`examples/outline_stability.rs`, `issue12295.pdf` page 1 at 4× (2448 × 3168), Radeon 890M
(RADV STRIX1), release, eight frames of the same display list.

| | frame 1 uploads | frame 2–8 uploads | `atlas_repacked`, 1 MiB atlas |
|---|---:|---:|---|
| before | 65 979 | 65 855 each | `. y . y . y . y` |
| after | 65 979 | **1** each | `. . . . . . . .` |

The repack column reproduces the quorra developers' 0,1,0,1 exactly, at the budget they measured
it at. At quorra's default 8 MiB budget the same oscillation is present and slower to appear —
the budget decides how soon the atlas is full, not whether the keys move — which is why the
instrument takes it as an argument.

The wall clock, interleaved A/B over four rounds of eight frames each — 28 timed repeat frames per
arm, the two binaries alternating so a load spike lands on both, both built from the code that
ships, machine at load average 4.75 (this one is shared; a first run of the same script at load 19
gave 497.4 / 141.2 minima, which is the same ratio through more noise):

| | minimum | median | maximum |
|---|---:|---:|---:|
| before | 357.5 ms | 535.7 ms | 594.9 ms |
| after | **110.4 ms** | **140.5 ms** | **219.0 ms** |

**3.24× on the minimum, 3.81× on the median**, and the two distributions do not overlap: the
slowest frame after is half the fastest frame before. The first frame is unchanged in both, as it
must be — it is the frame that does the expanding.

**Where a person feels it.** Not the still window, which retains its scene and re-encodes nothing.
A **scroll** does: it rebuilds the scene with the same scale, so the tolerance and the width are
unchanged and every expansion is now a hit. A **zoom** changes the tolerance and re-expands, which
is correct.

The pixels do not move. The corpus gate at 1× passes with `REFUSED` and both differing lists held
to equality, and at 4× with `REFUSED_AT_FOUR` held.

Four tests. The two in `tests/stable_ids.rs` need a device and were **each confirmed to fail when
the code they guard is removed** — the cache disabled makes the second frame upload again, the
width dropped from the key makes two widths of one path draw the same pixels. The two in `cache.rs`
need none: one asks the key itself, where all six fields can be moved one at a time because the key
is the thing under test rather than the picture; the other asks that an expansion nothing holds is
still released, since its pin is the *source* path and not the geometry on the device.

## Decision 3 — `bug1703683_page2_reduced.pdf` stays in `REFUSED_AT_FOUR`, and the run is why

The quorra developers asked for it to be dropped: "ADR 0057 draws it now; their ratchet fails
loudly until they re-baseline." **The gate run says it does not**, and the list is left alone.

`cargo test --profile gates -p render-quorra --test corpus` at `PDFVIEWER_QUORRA_SCALE=4`, on the
default lane, 951 pages compared:

```
refused: bug1703683_page2_reduced.pdf: frame refused: the frame's rasterised coverage
         outgrew the 16384x16384 scratch image this adapter allows
```

All four names, in order, and the assertion passes untouched.

**Their claim is true and it is true of their tree.** quorra's ADR 0057 sizes a clipped mark's
coverage tile by its chain's own bounding box, and their §1 measured this very page asking
1 008 561 911 texels where its 141 chains admit 2 297 897 — a factor of 439, which is not a page
squeaking under a ceiling. It landed on `cafadeb`, 2026-08-17 00:50. **This tree's `Cargo.lock`
pins `eada81ec`, 2026-08-16 21:08** — four hours and, by now, ninety-five commits earlier. The
refusal we measure is the behaviour of the revision we depend on, and it is the correct behaviour
of that revision.

So the report and the run do not disagree about anything; they are statements about two different
revisions, and the ratchet is measured on ours. **Taking the release is a round of its own**, and
`doc/todo/02-every-round.md` §2 already says what it owes: a round that takes a quorra release runs
this gate on *both* coverage lanes, because a release may be entirely inside a lane §2 does not
exercise. Bumping the pin inside this round would also have confounded decision 1's measurement,
which is taken against one unchanging device.

The general point is `CLAUDE.md` principle 5's, one boundary over from where it usually applies:
**a report from another implementation is evidence, and the thing it is evidence about is that
implementation.** A ratchet held to equality by name exists precisely so that a name is taken off
by a run rather than by a message — and here the message was accurate, the inference from it was
not, and only the run could tell the two apart.

`REFUSED_AT_FOUR`'s doc comment records the pending fix beside the refusal, so the next round to
take a quorra release knows what to expect rather than rediscovering it.
