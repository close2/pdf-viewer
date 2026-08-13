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

**What is left of it is the other two backends.** `render-cpu` and `render-gpu` still recompute per
draw, and `Image::reduction` is available to both. It matters for one host rather than for the
window: `viewer-confined`'s `pdf-view-worker` rasterises with `render-cpu` and returns pixels, so a
confined host redrawing a scanned page pays the 9 ms the window no longer does. Neither has a
per-frame resource cache to hang an entry on, so each would need its own bound and its own liveness
rule — which is why this was not done in the same round, and it wants a measurement in *that* host
before it is worth the lines.

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

**`elsewhere` was decided and the decision is a retraction.** It is not a duration: `execute` comes
from the adapter's own timestamp queries and `device` is a host `Instant`, so the remainder carries
whatever the two clocks disagree by along with the acquire, the present and the readback. The
summary says so in a line of its own now, and the two ways out are quorra's. ADR 0228 §4.

**What is still ours in this row**: this host builds a fresh `quorra_scene::Scene` every frame, so
nothing inside `encode` *can* be reused. A retained scene is the lever, it is named in
`doc/performance.md` as one of `RENDER_LIBRARY.md`'s five, and the number it would have to beat is
3.86 µs a command.

## 4. There is no second machine

Everything above is `llvmpipe` under `Xvfb`. The owner's figures — median 60.4 ms, p90 157, max 514,
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
