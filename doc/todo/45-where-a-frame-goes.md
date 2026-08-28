# Where a frame actually goes, now that the trace can say

Status: **open**, opened by the three-hundred-and-ninetieth session, which built the instrument
(ADR 0227) and then ran it. **Two of its four items were closed by the three-hundred-and-ninety-
first** (ADR 0228) and what item 2 left behind was closed by the four-hundred-and-sixty-second
(ADR 0297); this file is what is left.
Priority: 45 — performance, and each item below is *measured* rather than suspected. `doc/todo/44`
was the instrument; this is what it found, and it is the successor to that file rather than a
restatement of it.
Corpus: —, the witness is the project owner's own `tmp/windows/NorthAmerican.30MB.pdf` (65 pages,
30 MB), which is outside the corpus
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs`, `crates/render-quorra/src/cache.rs`,
`crates/pdf-render/src/paint.rs`, `doc/QUORRA_FEEDBACK.md`

## The measurement everything below comes from

39 frames of the owner's document — 38 page turns driven by `xdotool` — under `Xvfb` on
`llvmpipe`, `--trace=frames`. **This machine's software adapter is not the owner's Intel UHD
through DX12, so every ratio here is about shape and the absolute numbers are this machine's.**
Re-run before acting, and take three samples rather than one:

```sh
Xvfb :78 -screen 0 1200x1500x24 &
DISPLAY=:78 ./target/pdf-viewer --trace=frames tmp/windows/NorthAmerican.30MB.pdf
```

Sums in milliseconds over 39 frames, three runs of each, at 800×1000 (ADR 0228 §5):

| | before session 391 | after |
|---|---|---|
| frame | 1203.4 / 1242.3 / 1225.7 | **1071.6 / 1097.7 / 1074.0** |
| scene | 208.4 / 219.2 / 210.1 | **71.4 / 71.4 / 68.3** |
| device | 994.3 / 1022.4 / 1014.9 | 999.5 / 1025.7 / 1005.0 |
| attend | 92.8 / 94.1 / 94.3 | **10.3 / 12.4 / 10.7** |

## 1. ~~The accessibility publication costs 2 ms a page turn~~ — **closed, session 391**

It was not §14.7's tree. `Query::AccessibilityTree` is 0.13 to 0.25 ms and the whole publication
0.17 to 0.33; the 2 ms was `App::place_window`, two synchronous X11 round trips for a window
position a page turn cannot change. Asked at bridge-up, `Moved` and `Resized` now, with
`Bridge::wants_window_bounds` deciding it in the crate that owns the adapter — deliberately a
different question from `Bridge::shortfall`, because `doc/todo/31`'s two adapters take a window
handle and will need no bounds. ADR 0228 §3.

## 2. ~~Our display-list translation is bimodal~~ — **closed, session 391**

`Image::area_averaged`, and not the shading row reversal, which never runs on this document. The
cost is per *source sample*, so a 388-command page with one photograph on it cost sixteen times a
3675-command page of text. The column bands are computed once per image rather than once per output
cell and the output rows are divided across rayon above a measured floor: 22.4 ms → 2.9 on a
2700×3450 image, byte-identical, every gate unmoved. ADR 0228 §§1–2.

### 2a. ~~And the reduced raster is recomputed on every redraw~~ — **closed, session 462**

The witness this asked for is a *scroll* rather than a page turn, and taking it is the whole of why
it had not been taken: this file's own measurement is 38 page turns, and a page turn draws each page
once. Two `+` and twenty `Down` on the same document redraw one page twenty times, and each step
cost 12.7 to 16.8 ms of which **8.5 to 9.8 was `Image::area_averaged`** on a display list of one
command — the work is per *source* sample, so it does not shrink with the window and the twenty
steps recompute the same 1350×1725 raster from the same `Arc`.

`render-quorra` keeps it now, keyed by the source's `Arc` identity and the reduction factors, which
`pdf_render::Image::reduction` answers without producing the raster. Median frame **15.0 → 4.8 ms**,
uploads **23 → 2**, three runs an arm; every gate unmoved and the 4× lane byte-identical. ADR 0297.

### 2b. ~~And `render-cpu` recomputes it per draw~~ — **closed, session 797, and the measurement found something larger**

This item said `render-cpu` and `render-gpu` still recompute per draw, that it mattered for
`viewer-confined`'s worker rather than for the window, and that it wanted a measurement in *that*
host first. The measurement found a mechanism in front of the redraw and on **every** draw: **a
strip is a replay of the display list**, so a full-page image is reduced once per strip. The strip
planner cannot see it — `pdf_render::replay_ratio` counts the rows a command covers, and
`Image::area_averaged` costs per *source* sample — so `issue12963.pdf` page 1 reads as a replay of
1.00, is granted thirteen strips, and reduces the same 8.7 million samples thirteen times.

`render_cpu::images` memoises it on the rasteriser, on ADR 0297's key with the `Arc` pinned, so the
redraw case this item was about is answered by the same field: a rasteriser held across frames
keeps its reductions and one made per job keeps none. −42.1% of the whole page's instructions at
four strips with the reduction itself down exactly fourfold, and 22.7 ms → 7.4 at sixteen strips
where the page had been getting *slower* the more strips it was granted; every one of 957
rasterised first pages byte-identical. ADR 0731.

**What is left of it is `render-gpu`**, which still recomputes per draw and has no per-frame
resource cache to hang an entry on, and the **deferred** source — §11.6.5.2's soft-mask image,
which has no address that outlives a draw, so each strip still produces its own. Neither has a
witness: `render-gpu` is not the backend a host redraws with, and no corpus document reports an
`/SMask` at all.

## 3. Four fifths of a frame is inside `Device::render`, and the largest part of it is CPU

**Reported upstream in the three-hundred-and-ninety-first and open there**:
`doc/QUORRA_FEEDBACK.md` §13. Per frame over 38 real pages, 388 to 3675 scene commands:

| phase | median | sum | correlation with the command count | fit |
|---|---|---|---|---|
| `device` whole | 25.65 | 963.5 | +0.35 | 5.45 µs/cmd + 12.88 ms |
| — `encode` | 13.52 | **481.2** | **+0.58** | **3.86 µs/cmd + 3.84 ms** |
| — `upload` | 3.09 | 137.1 | +0.19 | 0.75 µs/cmd + 1.89 ms |
| — `execute` | 4.65 | 194.0 | +0.12 | 0.26 µs/cmd + 4.50 ms |
| — elsewhere | 3.16 | 151.3 | +0.15 | 0.58 µs/cmd + 2.66 ms |

`encode` is 45% of a page turn, is host processor time, and is the only phase that tracks the
scene's size. Nothing inside it is visible from here, which is what §13 asks for — an instrument
before an optimisation, the same argument the existing three phases won.

**The encode item has moved on and lives in [`47-the-encode-term.md`](47-the-encode-term.md)
now** — the three-phase instrument was built and read (ADR 0368), record replay was built and
measured no-win, and the current numbers are a fraction of this table's. This file keeps the
measurement that opened the question; that one carries its present state and the revisit
condition.

**`elsewhere` was decided and the decision is a retraction.** It is not a duration: `execute` comes
from the adapter's own timestamp queries and `device` is a host `Instant`, so the remainder carries
whatever the two clocks disagree by along with the acquire, the present and the readback. The
summary says so in a line of its own now, and the two ways out are quorra's. ADR 0228 §4.

**And the retraction's own enumeration was wrong, measured in session 552** (ADR 0387). "The
acquire, the present and the readback" named three things, and all three are now read rather than
assumed: `QuorraPresenter::last_phases` carries quorra's `Timings::phases` across the boundary, and
on the owner's own adapter `target acquire` is **0.035 ms** and `present` is **0.001 ms** on a zoom
frame whose `elsewhere` is over a hundred — three hundredths of one per cent of it. The readback is
zero for a window by construction and was the *other* half of the finding: `Timings::readback` is a
fourth measured phase this tree was not carrying at all, so on the offscreen rasteriser a
multi-megabyte copy was landing in this remainder. `FrameCost::readback` exists now.

**What is left in `elsewhere` is host time inside `Device::render` that quorra measures and
discards**, and it is legible from upstream's own source rather than inferred: `compose::submit_and_wait`
is timed as `execute_wall` and then replaced by the adapter's timestamp wherever timestamp queries
exist, and `record_content` — building the wgpu command buffer for 58 003 coverage tiles — is timed
by nothing. `doc/QUORRA_FEEDBACK.md` §29.2 is the ask, and it costs upstream one `push`.

**And `encode`'s own three phases have now been read, once** (session 533, ADR 0368):
`Options::instrument_encode` was switched on over `tmp/Entwurf.pdf` and answered **geometry 79.2%,
recording 16.2%, staging 4.6%** — quorra's coverage rasterisation, on one thread, for 59% of the
whole frame. That is the row's original question ("nothing inside it is visible from here")
answered for one page rather than for the corpus, and `doc/QUORRA_ENCODE_THREADS.md` is what went
back.

**That last question is closed** (session 552, ADR 0387): it is *not* a `FrameCost` field, because
`FrameCost` is `Copy` and `Timings::phases` is a `Vec`. It is `QuorraPresenter::last_phases` and
`QuorraRasterizer::last_phases` beside `last_frame`, cloned into a buffer the host keeps so that a
still window allocates nothing for it, and `crates/render-quorra/examples/zoom_frame.rs` is the
caller — `ZOOM_FRAME_ENCODE_PHASES=1` turns `Options::instrument_encode` on for a run that wants the
subdivision and pays quorra's few per cent for it. Seeing the three encode phases from a host no
longer costs a patch.

**What was ours in this row is done** (session 516, ADR 0351). It read: this host builds a fresh
`quorra_scene::Scene` every frame, so nothing inside `encode` *can* be reused — a retained scene is
the lever, and the number it would have to beat is 3.86 µs a command. Upstream built the retained
encode at `580fa4ac` (their ADR 0048, priced at `87898c69` by their ADR 0045) and this tree took
it: `render-quorra`'s `FrameSlot` keeps the frame's scene across frames, keyed on the page display
list's `Arc` identity, its placement, the window, the medium and the chrome by value, so a frame
that changed in none of those builds no scene and quorra replays its encode. On the owner's
document a still window's `scene` and `encode` both go to zero and `uploads` to none, byte-
identically. The conversation's remaining half is `doc/todo/44` §3.1's second bullet.

**This row's own witness has not been re-run under it.** Everything above is 38 *page turns* of
`NorthAmerican.30MB.pdf`, and a page turn is a rebuild by construction: every one of those frames
hands over a new display list, so ADR 0351 takes nothing off this file's own measurement and the
5.45 µs/cmd fit stands as the cost of the frames that *do* build. What it takes is the redraws
between turns, which is the same population ADR 0297's item 2a was about and which this file
learned once already to measure separately.

### 3a. The `upload` row had no denominator, and the frame line's `up` count was standing in for one

Opened and closed by the five-hundred-and-fifty-second session (ADR 0387). §3's table gives `upload`
a per-command fit, and the frame line prints a *resource* upload count beside a `transfer` duration —
two quantities with nothing to do with each other, on adjacent columns, one of them a count. On a
zoom step of `tmp/Entwurf.pdf` the count is **40** and the bytes are **8 475 012**, while the frame
that uploads **58 029** resources moves **6 898 596** — more bytes from fourteen hundred times fewer
uploads.

quorra has counted `bytes_uploaded` since ADR 0227 and `FrameCost` has carried it ever since;
nothing in this tree read it until the summary line that now sits under the upload count. **What the
denominator says is that the phase is not transferring**: 8.5 MB against tens of milliseconds is
about 65 MB/s on an integrated adapter, so it is the *preparing* half of quorra's own description of
the phase, and it is theirs. `doc/QUORRA_FEEDBACK.md` §29.1.

## 4. There is no second machine — **and the first one was reachable all along, without a window**

**Corrected by the five-hundred-and-fifty-second session, and it is the most useful thing that
round found.** This row said the numbers wanted the owner's own machine and that only the owner
could take them. Half of that is wrong: `render_quorra::options()` names no adapter, so a
**headless** quorra device — `QuorraRasterizer`, no surface, no event loop, no X authority cookie —
comes up on **AMD Radeon 890M (RADV STRIX1)** for the agent user, printed by
`adapter_description()` on every run. `examples/zoom_frame.rs` was written against that and ADR
0387's absolutes are the owner's hardware.

What still needs the owner's session is the *window*: the swapchain, the present, the cadence and
the compositor. So the rule this row should have stated is the narrow one — **an offscreen number
is takeable here on the real adapter, and only a windowed number is the owner's to run.** The
pinned-to-llvmpipe constructor (`new_headless_software`) stays what it is for, which is a gate that
must not depend on hardware.

Everything below this line is `llvmpipe` under `Xvfb`. The owner's figures — median 60.4 ms, p90 157, max 514,
and **eight budget refusals** that fell to the processor — do not reproduce here at all: `fallback`
is zero in every column of every run, before and after. A refusal is a fact about an adapter's
resource budget, so the eight are the Intel UHD's and cannot be chased from here. **The next run of
this file wants the owner's own machine**, with `--trace=frames` rather than `--trace`, which is 64
lines against 453 — and it now wants it more than it did, because the two items closed above were
this machine's largest and what is left is a phase whose ratio to `execute` is `llvmpipe`'s.

**And the eight are now worth re-measuring before anything is concluded from them**, which is a
change of the *subject* rather than of the method. They were taken against a library that allocated
every layer, mask and root at the size of the target; since the four-hundred-and-seventy-eighth
session took quorra's `a7babab`, each is the size of what it marks, and upstream prices the corpus's
layered frames at 4× down 41 % (`QUORRA_FEEDBACK.md` §22, quorra's ADRs 0036–0039). On this machine's
corpus every refusal that was a byte budget went to zero. A refusal is still a fact about the
owner's adapter and none of that predicts the Intel UHD — but eight refusals measured against the
old allocation are eight refusals against a frame that no longer exists, and re-running is a page
turn rather than an investigation.

## 5. A frame's cost is not always a *count* of commands, and this file's whole method assumes it is

Opened by the five-hundred-and-twenty-ninth session, which was handed a document by the project
owner that every fit above gets wrong by three orders of magnitude:

```
frame p1 1cmd presented 1143.9 | host 0.0 scene 1111.7 device 32.2 | 3 up, 0 culled
```

**One command, a second of `scene`.** §3's table fits `encode` at 3.86 µs a command and `device` at
5.45 µs + 12.88 ms, and every row of it is a page of *many* commands, so the model the file carries
is per-command throughput plus a constant. A `ShadingType 1` breaks it outright: since ADR 0339 its
function is evaluated once per device pixel the domain covers, so one command is arbitrarily much
work and a magnification the user chooses is what sets it. ADR 0364 took this instance from
1059–1175 ms of `scene` to 129–233 by removing a per-cell allocation and dividing the grid across
rows; what it did **not** do is make the shape go away.

So two things are owed here, and neither is that document:

- **The per-command fit wants a second variable**, or an honest statement that it holds only for
  pages whose commands are glyphs and paths. `pdf_render::command_extents` already knows how many
  device pixels a command covers; a fit against *pixels* rather than against commands is the same
  regression run over a column the file already has.
- **`scene` is measured and nothing inside it is.** `encode` earned three phases in ADR 0228 for
  exactly this reason and `scene` has none, so a round told that a frame is slow can say only that
  the display list took a long time to become a scene. On this document the answer was one paint;
  on a page of a thousand it will not be.

### 5a. A second witness for the first bullet, and it is not a shading (session 533, ADR 0368)

`tmp/Entwurf.pdf` breaks the fit from the other end: 58 009 commands, every one of them an ordinary
opaque fill, and a frame of 640 ms where §3's `5.45 µs/cmd + 12.88 ms` predicts 329. **The
direction is what settles it.** Over three magnifications of that one document, in two runs, the
frame that draws the *most* commands is the *cheapest*:

| commands drawn | `encode`, run A | run B, instrumented |
|---:|---:|---:|
| 58 009 (nothing culled) | **476 ms** | **513 ms** |
| 49 246 (8 763 culled) | 484 | 1598 |
| 40 023 (17 986 culled) | 868 | 993 |

(run B had `Options::instrument_encode` on, which inflates `encode`; only its ordering is evidence.)

The two zoom rungs are not ordered against each other consistently — that is a shared machine — but
no ordering by *count* survives either run, and the reason is plain once the phase is divided: 79%
of `encode` on this page is `encode: geometry`, quorra's coverage rasterisation, and a culled
command costs nothing while a surviving one costs its **area**. The page's summed bounding-box area
at the fit view is 401 059 px² over a 182 628 px² target, so 406 ms of geometry is about 1.0 µs a
covered pixel.

**So the second variable is device area covered**, and the regression this bullet asks for can now
be run against a column `command_extents` produces and a document that ranges over it.

**And the second bullet is half taken, by the population it was wrong about** (session 588, ADR
0423). On a *zoom* frame `scene` is 2.5% and nobody needed to divide it; on a **launch** it is a
fifth of the whole, because the first frame uploads every outline the page states — and the sentence
"nobody has needed to divide it" was true of the frames this file measures and false of the one a
person judges the program by. `FrameCost::handover` divides it in two: this host's own walk of the
display list, and the `upload_*` calls across quorra's boundary. On `tmp/Entwurf.pdf`'s cold frame,
on the real adapter, the second is **83%** of it — and `QuadOutline::from_segments` is six sevenths
of that under callgrind, building quadratics only the GPU coverage lane reads. `doc/todo/44` §6 has
the launch table it came out of and `doc/QUORRA_FEEDBACK.md` §33 is the ask.

**What is left of the bullet is the other side of the same division**: the walk itself is undivided,
and on a frame whose uploads are all cache hits it is what `scene` is made of. Nothing has needed
that yet — a warm frame's `scene` is 9.7 ms — which is the same sentence this paragraph just had to
correct, so it is written with the population named: *no frame measured so far*, and a launch was
not one of them.
