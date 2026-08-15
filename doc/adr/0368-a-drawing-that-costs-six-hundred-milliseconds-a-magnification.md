# 0368 — A drawing that costs six hundred milliseconds a magnification, and the phase nobody had divided

**Status.** Accepted. Session 533, a design round: it measures, it enumerates, and it builds
nothing.

## Context

The project owner asked one question about `tmp/Entwurf.pdf` — one page, 49.7 MB, one content
stream inflating to 141 MiB, 3 185 295 operators, 58 009 display commands, a vector drawing with
no text and no images:

> Could it be rendered every frame?

and added the two things that make it a design question rather than an optimisation: it commits
nobody to building anything if the cost is too high, and the answer may be that **the boundary
between quorra and this viewer should move**, or that **this tree's IR should change**, or
something else entirely.

Two rounds have already answered halves of it. ADR 0341 took the lexer's borrowed tokens and
39.8% of the interpretation's instructions; ADR 0351 adopted upstream's retained frame, so a
window showing an unchanged page builds no scene and encodes nothing. What neither touched is a
**zoom step**, and quorra's own ADR 0045 says why: the device transform is inside every atlas key,
every flattening tolerance and every lane choice, so no design replays an encode across a
magnification. This round asks whether that conclusion survives moving the boundary — and it can
only ask it with numbers, because every candidate below is plausible in prose.

## The measurement

`Xvfb :91` at 900×1100, llvmpipe, the release binary of this tree, three scripted sessions of
`tmp/Entwurf.pdf` driven by `xdotool`: settle, `+`, `+`, `-`, `-`. **Load average 1.6 to 13 on a
shared 24-core machine**; the absolute numbers are this machine's software adapter and the shares
are what carry. The cull counts are deterministic and identical across every run and across the
owner's own trace — 8763 and 17986 at the two zoom steps — which is what says the runs are the
same five frames and only the clock differs.

### Where the launch goes

| | run A | run B | run C |
|---|---:|---:|---:|
| document joined | 64.1 ms | 102.5 | 71.5 |
| interpreted, 58 009 cmd | **+996.3** | **+1038.1** | **+996.5** |
| first scene built | +317.1 | +327.1 | +324.7 |
| first present | +669.5 | +739.7 | +643.0 |
| **first present, absolute** | **2047.0** | **2207.3** | **2035.8** |

Interpretation is half of it, and the 317–327 ms of `first scene built` is **not** the scene walk:
it is the first frame's 58 029 resource uploads, which every later frame reads from
`render-quorra`'s cache. A zoom frame's `scene` is 11.4 to 15.8 ms.

**Interpretation, attributed by callgrind** — `callgrind_interpret tmp/Entwurf.pdf 1`,
`RAYON_NUM_THREADS=1`, one open plus one interpretation: **14 662 M instructions** for 58 009
commands. By self cost:

| self | Ir | share |
|---|---:|---:|
| `Lexer::next_token` + `read_regular_run` | 5 517 M | 37.6% |
| `zlib_rs` inflate, through `content::reader::Window::refill` | 2 497 M | 17.0% |
| the allocator (`malloc`, `free`, `realloc`, `RawVec` growth) | ~2 049 M | ~14.0% |
| `Interpreter::run_reader` and its closure | 2 467 M | 16.8% |
| operand conversion (`numbers_from`, `token_to_object`, `points_from`) | 1 275 M | 8.7% |
| `Parser::parse_stream_data` | 447 M | 3.0% |
| `drop_glue::<Object>` | 268 M | 1.8% |

Two things worth naming, because both are a *change* rather than a level. **Lexing was 63.6% of
this document before ADR 0341 and is 37.6% now** — the borrowed token did what it was measured to
do, and what is left is a different program. And **the flate is interleaved rather than a
prologue**: ADR 0365 reads `/Contents` through a 64 KiB window, so §7.4's inflation appears inside
`run_reader` and the 141 MiB is never a buffer. The total is 14 662 M against ADR 0341's 13 487 M,
+8.7%, which is ADR 0362's predicted +4.10% for the window plus what else landed in the four
sessions between; the peak resident it bought is 194 MB against 381.

### Where a frame goes, at a real magnification

The frame each session ends on: the whole page visible at the fit view, 58 009 commands, nothing
culled, the display list unchanged, the *magnification* new. No instrument beyond the frame line
and quorra's own `Timings`:

| | ms | share of the frame |
|---|---:|---:|
| `scene` — this tree's display-list-to-scene walk | 15.8 | 2.5% |
| `encode` — quorra, host processor | **475.9** | **74.4%** |
| `transfer` | 65.4 | 10.2% |
| `execute` — the adapter's own timestamps | 29.1 | 4.5% |
| `elsewhere` — a bound, not a duration | ~52 | ~8.1% |
| `settle` | 0.8 | 0.1% |
| **whole frame** | **639.8** | |

The same frame in the other two sessions: 660.0 and 661.9 ms. **The graphics device is 4.5% of
it.** Everything else is one host thread.

**And `encode` divides**, which is what `doc/todo/45` §3 asked upstream for and what nobody had
switched on here. With `Options::instrument_encode` — which costs what quorra says it costs, 512.8
against 475.9 on this frame, so the shares below are what to read and not the absolutes:

| encode phase | ms | share of `encode` |
|---|---:|---:|
| **geometry** — flattening outlines, expanding strokes, the scanline rasteriser | **406.3** | **79.2%** |
| recording — clip resolution, culling, instance building, plan assembly | 82.9 | 16.2% |
| staging — packing coverage into the scratch sheet and the atlas | 23.6 | 4.6% |

**So 59% of the whole frame is one thread turning 3 011 879 path segments into 58 003 coverage
tiles.**

### The control that identifies the phase rather than merely naming it

The fourth frame of every session zooms *back* to the magnification the second frame drew: the same
58 009 commands at the same transform, in a scene `FrameSlot` rebuilt from nothing —
`encode_source` reads `Encoded` on both.

| the same view, twice in one session | frame | `encode` |
|---|---:|---:|
| first draw, no instrument | 629.7 | 483.8 |
| **second draw, no instrument** | **140.0** | **90.6** |

and the second draw, subdivided in the instrumented session: `encode` 93.8 of which **geometry
1.7**, staging 0.2, **recording 91.8**.

**Geometry all but vanishes when quorra has rasterised these tiles at this transform before, and
`recording` does not move at all.** That is what says the 406 ms is coverage rasterisation and
nothing else — and it puts a floor under the design: 92 ms of instance building, culling and plan
assembly that no cache removes, which is five and a half times a 60 Hz frame on its own.

**One thing the pair does *not* establish, and this round leaves it open rather than guessing.**
The *fifth* frame returns to the fit view the *first* frame drew, and pays full geometry again
(406.3 ms). So the reuse is bounded by something between the two — the atlas's own capacity across
three intervening magnifications, or a transform that differs where this round did not look. It
does not affect the attribution: what the control establishes is which phase the 406 ms belongs to.

## The design space, enumerated and priced

`command_extents` and the display list's own contents, measured rather than assumed:

- 58 003 fills, 6 strokes, **no images, no groups, and not one command under a clip or a soft
  mask** — which also settles one of `take_gpu_lane`'s three conditions below: the lane was
  declined on the triangle test alone;
- **3 011 879 path segments**, 51.9 per fill;
- **2 073 distinct paints** over 58 003 fills;
- maximal runs of *consecutive* commands sharing paint, clip, mask, blend and fill rule:
  **57 419 runs — 590 commands merged away, 1.0%, longest run 3**;
- at the fit view (801×228): **27 891 commands, 48.1%, have a bounding box under one device
  pixel**; 42 954 under four; summed bounding-box area 401 059 px² over a 182 628 px² target,
  2.20× overdraw. At 1601×455 the sub-pixel share is 17.5%, at 3201×910 it is 4.7%.

| | what it costs this tree | what it costs quorra | what it buys on this document |
|---|---|---|---|
| **(a) nothing** | — | — | a still window already replays: 21.5–34.7 ms a frame, `scene` and `encode` zero (ADR 0351). A zoom step is 640 ms |
| **(b) a page-space scene under a root affine** | `render-quorra`'s `Encoder` stops composing `TargetSpec::transform`; `SceneKey` drops the placement; every window-pixel overlay must be pre-transformed by the root's inverse | nothing — `Viewport` already takes a full affine, and the brief's §2.3 already asked for exactly this | **the `scene` phase alone: 11.4–15.8 ms of 640, 2.4%.** quorra's ADR 0045 is right and this round confirms it from the other side: `encode` is 74% and none of it survives a transform change |
| **(c1) batching by paint state** | a merge pass, a second display list, and a change to what the oracle compares | nothing | **1.0%** — 590 of 58 009 commands. And it is a *loss*: merging non-adjacent fills of one paint gives each merged path the bounding box of the union, and quorra bills coverage per tile **area** — 406 ms of geometry over 401 059 px² of summed bounding box is about 1.0 µs a covered pixel |
| **(c2) a geometry buffer uploaded once and re-rasterised** | the IR would have to name geometry no transform has touched | the path coverage lane would have to be device-side | **measured and declined by quorra's own rule.** Forcing `Coverage::Gpu` for the whole session changes nothing: `encode: geometry` 418.5 ms against 406.3, frame 732.7 against 660.0. `take_gpu_lane` requires `tile area ≥ triangles × 3 × 32 bytes`, and a 52-segment outline is ≥ 5 000 bytes of vertices against a tile of about three device pixels |
| **(d) tiling or region invalidation** | `Viewport::damage` already exists and is already plumbed | nothing | **zero.** A zoom damages the whole window; and quorra's `encode` never reads the damage list at all — damage is planned target-side (`QUORRA_RETAINED_FRAME.md` §4) |
| **(e) level of detail** | a rule the standard forbids | — | **forbidden**, §10.7.4, below |
| **(f) `encode` on more than one thread** | nothing | a deterministic thread pool inside `Device::render` | **the only item with a factor in it**: 406 ms of a 640 ms frame is one thread doing embarrassingly parallel work |

### (e) against §10.7.4, read rather than assumed

48.1% of this page's commands cover less than one device pixel at the view a person opens it in,
which is exactly the population a level-of-detail scheme would drop. The clause forbids it in a
`shall`:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears
> as a result of unfavourable placement relative to the device pixel grid, as might happen with
> other possible scan conversion rules. The area covered by painted pixels shall always be at
> least as large as the area of the original shape.

This tree already spends code on the *opposite* reading: ADR 0226's `sub_pixel_bands` gives a mark
thinner than a device pixel the whole pixel line it lies in, at its own area's coverage, because a
coverage that rounds to nothing is the same disappearance by another road. Dropping a sub-pixel
mark is not an optimisation this project is permitted.

**What §10.7.2 does permit is not the same thing.** "PDF processors may choose to ignore any
flatness tolerance specified within a PDF file" is about how finely a *curve* is approximated, and
its NOTE 2 warns that where "the parameter's value is large enough to cause visible straight line
segments to appear, the result is unpredictable". quorra's flattening tolerance is already in
device pixels, so a curve whose whole
extent is under a pixel already flattens to almost nothing: the 406 ms is not segments *emitted*,
it is 3.0 M segments **examined**, and no tolerance removes an examination.

### Every candidate against the boundary's own test, and against the oracle

`doc/ui-boundary.md`'s test is for the *vocabulary*: a `Command`, `Event` or `Query` is added only
for a question a host cannot answer for itself. **None of (a) to (f) adds a message, and none of
them is a boundary change in that sense** — (b) and (c) live inside `render-quorra` and
`pdf-render`, below `viewer-core` entirely, and a host cannot tell them apart.

The oracle's requirement bites on exactly one of them. `interpret` must stay a pure function of
the document's bytes and the view state, and (c1) keeps that — a merge pass is deterministic. What
it does not keep is the *instrument*: merging opaque same-paint fills changes pixels on every page
that has two, and it changes them in a direction §10.7.4 approves of, because §11.3.7.3's union of
two half-covered marks is three quarters and `doc/todo/11` item 5 measures 0.1937 of a layer
shining through this very document at page scale. A change that moves the oracle's comparison is a
decision of its own with its own ADR, exactly as ADR 0367 said of the admission classifier — and
it may not be taken as a performance change worth 1.0%.

## Decision

**Build nothing. The boundary does not move and the IR does not change. Write one ask to quorra,
and state in it that it does not deliver a frame.**

### 1. The boundary is where it should be, and the number says so

This tree's own share of a frame at a new magnification is `scene`, and `scene` is **2.5%**.
Whatever is done on this side of the boundary is bounded above by that plus whatever a smaller
command count does to quorra's per-command constants — and the measured batching opportunity is
1.0% of the commands. **A boundary is in the right place when moving it cannot buy anything, and
that is now a measurement rather than a conviction.**

### 2. The IR needs no change, and it is worth saying what it already does

`pdf_render::Command` carries a transform into **page space**, not device space; the device
transform arrives separately as `TargetSpec`. Paths are shared behind `Arc`. State is resolved
once. So the IR is *already* the page-space, viewport-independent artefact the brief's §2.3 asks a
scene to be — the transform this document re-bakes every frame is composed one layer lower, in
`render-quorra`'s `Encoder`. Candidate (b) is therefore not an IR change at all; it is fifty lines
in one backend, and it is worth 2.4%.

The one thing the measurement *does* ask of the IR is nothing anybody proposed: it asks for no
coarsening. A coarser IR would take work away from a phase that is 16% and give it to a phase that
is 79% and bills by area.

### 3. The ask, and it is one thing

`doc/QUORRA_ENCODE_THREADS.md`, written beside `doc/QUORRA_FUNCTION_PAINT.md` in the same voice:
**divide `encode` across more than one thread.** The device is the right place because every datum
the phase touches is quorra's — the polylines, the tiles, the scratch sheet, the atlas — and
because their own crossover rule has already declined the alternative for this page's shape. What
it must not cost is determinism: this project's CI rests on two adapters agreeing byte for byte,
and a parallel rasteriser must agree with itself across thread counts too.

**And the ask states its own ceiling, because a request that oversells is worse than none.** If
geometry went to *zero* the frame is still 235 ms: `recording` 83, `transfer` 65, `execute` 29,
`elsewhere` 52, `scene` 16. Three orders of magnitude of drawing do not become a frame by
threading one phase of it. What the ask buys, if it buys everything it could, is a zoom step that
feels like a step.

### 4. So the owner's question has three answers, not one

- **A still window: already yes**, and ADR 0351 is why — 21.5 to 34.7 ms a frame here, on a
  software adapter, of which the encode is nothing at all.
- **A pan by whole device pixels: not measured, and the prediction is the cheap one.** quorra's own
  survival table makes a whole-pixel scroll a re-encode, because every instance is an absolute
  device position — but it also says the atlas tiles stay valid, and this round's control is
  exactly that case: geometry cached, `recording` re-run. So a scroll should cost the 140 ms frame
  rather than the 640 one. Stated as a prediction, because no frame of this round was a scroll.
- **A new magnification: no**, and not by any change either tree can make cheaply. The floor a
  perfect cache leaves is `recording`'s 92 ms, and the floor a perfect thread pool leaves is
  about 235 ms.

**And a fourth thing the round found on the way, worth a round of its own rather than a sentence
here**: a magnification quorra has drawn before costs 140 ms where a new one costs 640, and this
session's own script hit that reuse once and missed it once. Whatever bounds it — the atlas across
three intervening views, or a transform that differs in a place nobody looked — a *policy* that
kept the two or three magnifications a person moves between resident would be worth 4.5× on the
gesture that matters, and it needs nothing from anybody. Nobody has measured what it would cost to
hold.

## Consequences

- `doc/todo/44` §4's remaining item — the page-space scene — is **priced at 2.4% of a zoom frame**
  and stays open on that basis rather than on an estimate. It is no longer the file's largest
  question; `encode: geometry` is.
- `doc/todo/45` §5 asked for "a fit against *pixels* rather than against commands", and this round
  hands it the evidence that the question is right, stated only as far as two runs agree: **the
  frame that draws the most commands is the cheapest of the three.** 58 009 commands drawn cost
  `encode` 476 ms and 513 ms in the two runs; 49 246 cost 484 and 1598; 40 023 cost 868 and 993
  (the second column is the instrumented session, so its absolutes are inflated and only its
  ordering is evidence). The two zoom magnifications are not ordered against each other
  consistently — that is a shared machine talking — but a *culled* command costs nothing and a
  surviving one costs its area, and no ordering by count survives either run. The variable the file
  wants is device area covered.
- **`Options::instrument_encode` has now been used once**, by a probe this round removed. `todo/45`
  §5's second bullet — "`scene` is measured and nothing inside it is" — is unchanged, and on this
  document `scene` is 2.5% of a frame, which is the reason it stays unchanged.
- Nothing in this tree is different. `fmt`, `clippy --workspace --all-targets`, the workspace test
  run, the doctests and the conformance checker were run to prove that, not to prove a change.

## The instruments, so the next round need not rebuild them

Everything above came from instruments this tree already had — `crates/pdf-model/examples/
callgrind_interpret.rs`, `pdf_render::command_extents`, the frame line and summary of
`pdf-viewer --trace=launch,frames`, and quorra's `Options::instrument_encode` — plus two temporary
probes, which are named here rather than kept:

1. `instrument_encode` set from an environment variable in `QuorraPresenter`'s two surface
   constructors, and `Timings::phases` printed. **`render-quorra` reads `Timings` and drops
   `phases` on the floor**, so there is no way to see the subdivision from a host today. Whether
   that becomes a `FrameCost` field is a question for the round that next needs it; on this
   document the answer it gave was worth having exactly once.
2. `coverage_for` in `viewer-ui` forced to `Coverage::Gpu`, to test (c2) rather than argue it.

Both were reverted before the commit, and the display-list census was a third example, also
removed. A round that wants them again will find this paragraph faster than it will rewrite them.
