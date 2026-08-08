# Where a frame actually goes, now that the trace can say

Status: **open**, opened by the three-hundred-and-ninetieth session, which built the instrument
(ADR 0227) and then ran it. **Two of its four items were closed by the three-hundred-and-ninety-
first** (ADR 0228) and this file is what is left.
Priority: 45 — performance, and each item below is *measured* rather than suspected. `doc/todo/44`
was the instrument; this is what it found, and it is the successor to that file rather than a
restatement of it.
Corpus: —, the witness is the project owner's own `tmp/windows/NorthAmerican.30MB.pdf` (65 pages,
30 MB), which is outside the corpus
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs`, `doc/QUORRA_FEEDBACK.md`

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

**What it left behind, and it is a real question rather than a leftover**: the reduced raster is
still *transient*, recomputed on every frame that draws the image. On this witness that costs
nothing — each page is drawn once — but a scroll, a selection, a caret blink or a resize redraws
the same page, and at 2.9 ms an image that is the whole page. A cache keyed by the source image's
`Arc` identity **and the reduction factors** would be exact, because those two decide every output
byte. What it needs before it is worth building is a witness: a redraw-heavy session measured with
`--trace=frames`, which nothing in this project has taken.

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
`doc/performance.md` (`doc/HANDOVER.md` §4's pointer) as one of `RENDER_LIBRARY.md`'s five, and the number it would have to beat is
3.86 µs a command.

## 4. There is no second machine

Everything above is `llvmpipe` under `Xvfb`. The owner's figures — median 60.4 ms, p90 157, max 514,
and **eight budget refusals** that fell to the processor — do not reproduce here at all: `fallback`
is zero in every column of every run, before and after. A refusal is a fact about an adapter's
resource budget, so the eight are the Intel UHD's and cannot be chased from here. **The next run of
this file wants the owner's own machine**, with `--trace=frames` rather than `--trace`, which is 64
lines against 453 — and it now wants it more than it did, because the two items closed above were
this machine's largest and what is left is a phase whose ratio to `execute` is `llvmpipe`'s.
