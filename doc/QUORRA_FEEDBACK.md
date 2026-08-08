# quorra, measured against the corpus — findings, and what came back

Written 2026-08-03 at the end of this viewer's hundred-and-ninety-fifth session, from the run
described in ADR 0156; **rewritten the same day, after every finding in it was answered.** It is
the counterpart to `RENDER_LIBRARY.md` — that document is the brief this project wrote for a team
building a renderer, and this one is what came back when the renderer met 974 real documents,
and then what the team did about it.

Each finding below keeps its evidence and carries what closed it, because a feedback document
that still reads as a complaint after the complaint was answered is worse than no document.

**§10 was the newest, was a defect rather than a request, and was answered at `0a1ffb13` the same
day it was reported** — one unconditional line priced a texture the default coverage lane never
allocates, five real pages were refused on it, and the fix went in one level deeper than the one
proposed.

**§12 was the newest, was a request rather than a defect, and was answered at `2531f447` with
exactly the parameter it asked for** — `create_instance_with(backends)` — plus one that was not
asked for and closes a trap the first would have opened, `Device::adapter_names_on`. The
question §12 left to you was decided rather than defaulted: `WGPU_BACKEND` is not read, here or
anywhere, and the argument for that silence is your ADR 0017's. This side's `pdf-viewer` now has
the `--backend vulkan|dx12|metal|gl` it said it would not add until it could be honest (our ADR
0221).

**§11 was a defect, was answered at `52b07f29`, and is closed**: the GPU coverage lane drew the
*wrong glyph* — a lowercase `t` as a capital `T` — after a frame at a larger magnification, and
the winding texture's size was what survived the frame. Re-verified at `2531f447`: `zoom_ladder`
is identical to the digit at every rung and `viewer-ui`'s own overlay gate is green.

**§13 is the newest, is a request for an *instrument* rather than for speed, and is open.** A page
turn of the project owner's own 30 MB document is 45% `encode` — host processor time, the only one
of `Device::render`'s three phases that tracks the scene's command count, fitting **3.86 µs a
command plus 3.84 ms**. That phase is now the largest and is itself unsplit, so §13 asks for the
same subdivision one level down that the existing three already won the argument for. It also
retracts something this side printed: our `elsewhere` row subtracts a timestamp-query duration from
a host wall clock, so it is a bound rather than a measurement, and the output says so now.

**§8 was answered at `7d5dafb` and §9 is still open.** Both were requests rather than defects,
and both exist because the project owner's decision that page one goes to the graphics device put
your bring-up on this viewer's critical path. §8 asked for a field split and an entry point and
got both, plus a refusal of the knob it said not to add — which ADR 0017 has now superseded in
part, for a reason §8.3 never weighed; §9 is from the other end of the same launch — the first
frame allocates ~12 ms that every frame after it reuses, and it is provably not the shaders.

**Where it stands, at the page's own scale:**

| | first run | now |
|---|---|---|
| agree | 900 | **914** |
| differ | 50 | **42** — 28 of them the antialiasing floor (§4) |
| refused | 7 | **1** — and 6 for one day, which is §10 |
| median page | 2.64× the CPU backend | **2.05× to 2.33×**, run to run |

The first three are `2531f447`'s, and the same three the gate has printed since the
three-hundred-and-sixty-eighth session; **this table said 913 / 43 for sixteen sessions after that
and was corrected in the three-hundred-and-eighty-fourth**, which is the ledger's own disease one
document over. The median is a *timing* and moves between runs on one revision — 2.05× and 2.33×
are two runs of the same gate — so it is quoted as a band rather than as a figure that moved.

---

## 0. The instrument, and how to run it

```sh
cargo test --release -p render-quorra --test corpus -- --ignored --nocapture
```

`crates/render-quorra/tests/corpus.rs`. Every one of the 974 pdf.js corpus documents' first
pages, interpreted **once** and handed to `render-cpu` and to quorra as the *same display list*,
compared with `raster-compare`. Nothing in it is about PDF semantics: a difference is two
rasterisers disagreeing and a refusal is a command quorra cannot draw.

- `PDFVIEWER_QUORRA_ONLY=a,b` restricts it to matching file names.
- `PDFVIEWER_QUORRA_SCALE=2` renders at another scale. Both skip the ratchets and say so.
- Every page that differs writes both renders to `target/tmp/quorra/<stem>/{cpu,quorra}.png`.

The glyph-phase quantum is **off** in this gate, so what it measures is the adapter and the
translation rather than a trade `real_pages.rs` gates separately.

The differing and refused pages are held by name in that file. A page arriving in either list
fails the build; so does a page leaving it, because a hole that closes should be noticed — which
is how the numbers in this document were kept honest between the two runs.

---

## 1. §10.7.4's degenerate fill was not asked for — **answered**

**Was: a page of ruling lines came out blank.**

`issue4260_reduced.pdf` rules its grid with zero-height rectangles — `848 1085 10159 0 re f` —
and the CPU backend drew the grid while quorra drew the surrounding box and nothing inside it.
Mean 14.19, structural similarity **0.49**, the worst page in the run.

Not a rasterisation difference. ISO 32000-2 §10.7.4:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears

A subpath with no extent along one axis has zero area, so *any* coverage-based rasteriser
computes nothing for it. This viewer therefore states the answer once, in the crate both backends
consume, so that neither decides it alone: `pdf_render::thinnest_line` for the width and
`pdf_render::split_collapsed_fill` for the split, with the marks filled under the **non-zero**
rule whatever the command's own rule is — a mark added to an even-odd path's winding would punch
a hole in what it was meant to draw.

**Answered**: fills run through `split_collapsed_fill`, as the two sibling backends do.
`issue4260_reduced.pdf` goes from similarity **0.49 to 0.9938** (mean 14.19 → 1.73) and leaves
the shape list for the antialiasing floor.

**It was never a criticism of the library, and the timing is why**: the rule landed in this
viewer in ADR 0154, three sessions before the backend was measured, and nothing announces a new
device decision to a backend. That is the standing argument for keeping such decisions in
`pdf-render`, and the reason this gate exists.

---

## 2. The resource caches never evicted — **answered**

**Was: a long-lived rasteriser stopped drawing, and only a corpus-scale run could see it.**

At four times the page's own scale, **533 of 952 pages were refused**:

```text
resource upload refused: uploading would hold 536871036 resource bytes
(536870896 already resident), over the stated budget of 536870912
```

536 870 896 bytes resident is the 512 MB budget, full. The proof that it was not the pages:
`tiling-pattern-box.pdf` was refused in the full run and **passed on its own**.

`QuorraRasterizer` holds one `Device` and three maps keyed by pinned `Arc` identity, and the
design note beside them is right that this is what lets the cache span `rasterize` calls. What it
had no way to do was stop: the entry pinned the allocation for as long as it lived, and nothing
decided that it should stop living. A per-document suite starts with an empty device every time
and cannot see this; a viewer with a document open all afternoon is exactly the long-lived
instance it describes.

**Answered**, and by the first of the three shapes this document asked for — a policy inside the
device, so the caller need not know: entries carry recency, and after every frame (refused ones
included) the least-recently-used entries the frame did not touch are released until the device
holds no more than **half** its budget. Half rather than all, so eviction is not a cliff and a
hot entry is never evicted.

**The 4× run's 533 resource refusals are zero**, and that scale went from 413 pages agreeing to
**918**.

---

## 3. A refusal message whose arithmetic contradicted it — **answered**

**Was:** six refusals said "frame needs N bytes of instance data, over the stated budget of
33554432" with `N` equal to 21 093, 114 140, 1 170 768, 3 763 825, 20 263 595, 29 621 489 and
29 666 103 — every one of them *under* the budget it was said to exceed.

**Answered on quorra's side.** The refusals that remain add up, and the one that replaced the
mis-stated limit says what it is:

```text
frame needs 1411676992 bytes of instance data, over the stated budget of 268435456
the frame's rasterised coverage outgrew the 16384x16384 scratch image this adapter allows
```

Six of the seven pages the old message refused now draw. **One refusal is left at the page's own
scale** — `bug1721218_reduced.pdf`, this viewer's own worst page by some margin, on the scratch
image's limit — and eighteen at 4×, all with coherent arithmetic.

---

## 4. What is **not** a finding

Twenty-nine of the forty-four differing pages have structural similarity **above 0.99** — the
same shapes in the same places — and twenty of those are one document family (`tracemonkey.pdf`
and its relatives) sitting at mean 1.52 with a worst tile of 5.09. That is a page of dense text
measured against a different glyph rasteriser. `real_pages.rs` measures the specification's own
pages at 1.18 and reports the Vello backend at 1.16 on the same cases: **the two rasterisers'
antialiasing is essentially the whole floor**, and quorra is not adding to it.

The floor also shrinks as the page grows, which is what a floor should do: at 2× only 17 pages
differ at all and at 4× only 16, against 44 here.

Fifteen pages still differ structurally, largest first: `issue16038.pdf` (6.16),
`issue16316.pdf` (5.06), `copy_paste_ligatures.pdf` (3.86), `issue4402_reduced.pdf` (3.86),
`issue12295.pdf` (2.16, similarity 0.879). `issue2177.pdf`, `issue6769.pdf`,
`issue6769_no_matrix.pdf` and `bug946506.pdf` left this list when strokes under an
anisotropic transform started being outlined in path space — a fix nobody asked for, found by
reading the shape list rather than by being told. Their artefact pairs are on disk after a run;
this side has not opened the fifteen, and the list is the offer.

---

## 5. Speed, offscreen

Page one of every document, both backends, same display list, AMD Radeon 890M (RADV), release.
**The GPU figures include the readback to system memory**, which a windowed host does not pay —
`RENDER_LIBRARY.md` section 6.1 measures that at 55% to 92% of a frame — so these are offscreen
numbers and say nothing directly about presenting.

| scale | pages drawn | `render-cpu` | quorra | median page | was |
|---|---|---|---|---|---|
| 1.0 — the page's own | 956 | **2.55 s** | 6.26 s | quorra **2.05×** | 2.64× |
| 2.0 — a window's | 948 | **5.21 s** | 10.16 s | quorra **2.87×** | 3.18× |
| 4.0 | 934 of 952 | **11.34 s** | 24.13 s | quorra **3.24×** | 3.77×, over 419 pages |

**Quote the total against the median and say which.** The totals ratio *improves* with scale
(2.45× → 1.95×) while the median ratio *worsens* (2.05× → 3.24×), and both are true: this
viewer's CPU rasterisation grows with the pixels, so the heavy pages close the gap in the total,
while the median page is small enough to be dominated by a per-frame floor that does not shrink.
The 4× row is comparable with the others for the first time — before the eviction fix it covered
419 pages and the survivors were the ones whose resources fit.

For contrast, the *presented* path — `pdf-viewer` on `Xvfb`, ISO 32000-2 through quorra — puts
page one on screen in **44.6 ms** and turns to page 6 in **9.3 to 17.4 ms** a page. That is the
number the swap was made for, and this gate is deliberately not it.

**The instrument had the same shape of defect the library did, and it is recorded here so that
nobody trusts an earlier draft**: a refused frame is a fast frame, so the first 4× run reported a
median of `0.00×`. Only frames that were produced are timed now.

---

## 6. One thing this side owed back — **settled both ways**

`cargo test -p conformance` used to fail on ten citations in `crates/render-quorra` — `§4.5`,
`§2.2`, `§4.6` — which are sections of `RENDER_LIBRARY.md` rather than of ISO 32000-2. This
tree's rule is that a bare `§` means one document, and the brief never said so.

Settled from both ends: the citations now read `RENDER_LIBRARY.md section 4.5`, and the
conformance checker was taught to say what is wrong when a project document's name precedes a
`§` — so the next person to write one is told rather than left to guess.

---

## 7. A refused frame wedged the surface — **answered**

**The report, from a person using the viewer:** dragging a selection across a page made the window
stop answering, and a resize sometimes recovered it.

**Half of it was ours** and is fixed (ADR 0176): the host drew one `Multiply` fill per selection
quad, quorra gives every non-`Over` blend its own compositor layer, and
`compose::internal_texture_bytes` prices them all before allocating any. The refusal's arithmetic
is exact —

```text
(63 + 1) × 2 × 800 × 1000 × 4 = 409 600 000     the number the refusal printed
```

— so a selection quad cost 6.4 MB of frame budget and 63 of them, one short paragraph of text,
spent the 256 MiB. One fill of one path with one subpath per quad draws the same pixels for two
layers instead of sixty-four. **No complaint about the pricing**: counting before allocating is
§5, the message named both numbers, and this side was asking for something absurd.

**The other half is not ours, and it is the one the report was about.** After the refusal, every
subsequent present blocked for **exactly one second** and returned `SurfaceProblem::Timeout`, for
ever. The process sits at 4% CPU, so it is blocked rather than spinning.

Reading `quorra-gpu` at `3f45555` names a mechanism, and this side offers it as a reading rather
than as a diagnosis it has instrumented:

- `Device::render` calls `bind_target` — which for `Target::Surface` **acquires the swapchain
  texture** — and only then prices the compositor's internal textures and returns
  `RenderError::FrameBudgetExceeded`. So a refused frame drops a `wgpu::SurfaceTexture` that was
  never presented. `wgpu` discards it on drop, but the acquire semaphore was never waited on by
  any submission, which is the shape that exhausts a Vulkan swapchain.
- `Surface::acquire` sets `needs_reconfigure` for `Suboptimal` and `Outdated` and **not for
  `Timeout`**, so nothing ever reconfigures the surface again. A host resize changes the
  configured size and reconfigures, which is exactly why a resize recovers the window.

**Answered on quorra's side at `4aab7e2`, in all three of the shapes this section asked for.**
The budget is priced *before* `bind_target`, with the reason written into the comment there, so a
refused surface frame costs no acquire at all; `Timeout` now sets `needs_reconfigure`, as `Lost`
does; and `Device::invalidate_surface` gives a host somewhere to go when the device says no.

**Measured against the original report, on the same recipe.** The per-quad selection was restored
locally to reproduce it exactly, and the same drag over `issue14821.pdf` now gives:

```text
SELECTION quads 70   present -> failed: frame needs 454400000 scene-derived bytes,
                               over the stated budget of 268435456 … in 6.4 ms
SELECTION quads 56   present -> failed: … in 6.6 ms
SELECTION quads 42   present -> failed: … in 6.3 ms
```

**Every refused present costs 6 ms instead of blocking for 1.008 s, no `Timeout` is reported at
all, and the drag keeps updating throughout** — the quad count falls as the pointer moves, which
is the window answering. A page the device refuses for the *other* reason draws through the CPU
fallback and presents: `bug1721218_reduced.pdf` outgrows the 16384×16384 scratch image, comes back
on the processor in 1.68 s, and zoom, scroll and the sidebar all work afterwards.

**So `doc/todo/13` is closed and deleted**, both halves: the selection's cost by ADR 0176 and the
refused frame's recovery by this. Nothing is owed on this side; the CPU fallback still re-presents
the same overlay lists, so an over-budget *overlay* leaves the page not updating rather than the
window dying — which is a visible consequence with a live window behind it, and no longer a defect
worth a file.

---

## 8. Bring-up is on the critical path now, and a host cannot see into it or start it early — **answered**

**New in this viewer's two-hundred-and-seventy-fifth session, and it is a request rather than a
defect.** The project owner decided one session earlier that **page one goes to the graphics
device**: no CPU first frame, no probe, no `wait_until_warm`. `CLAUDE.md` records what follows as
an obligation — "creating the device and compiling the pipelines is now part of time-to-first-page,
so it is a number to measure and to keep small" — and this side now measures the whole launch as a
timeline (ADR 0179). On this machine, under `Xvfb` with `lavapipe`:

```text
trace: launch path, process start to first present:
trace:   document read             8.079 ms  (+8.065)
trace:   chrome fonts              9.457 ms  (+1.378)
trace:   document open            37.225 ms  (+27.768)
trace:   event loop               45.236 ms  (+8.011)
trace:   window                   45.392 ms  (+0.156)
trace:   graphics device          90.519 ms  (+45.127)
trace:   first present           144.609 ms  (+54.090)
```

**Bring-up is 31% of it**, and `StartupTimings` is what this side has to reason with. Two things
would help, and one thing that looks like it would does not.

### 8.1 `adapter_enumeration` names one step and measures three — **answered**

`Device::build` takes `started` from *before* `wgpu::Instance::new` in both constructors, so the
figure a host reads as `adapter_enumeration` is instance creation **plus** surface creation
**plus** `select_adapter`. Measured with `wgpu` directly, one measurement per process — three
processes each, on this machine's three adapters (RADV, `llvmpipe`, `radeonsi`):

| backends | `Instance::new` | `request_adapter` | `request_device` | total |
|---|---|---|---|---|
| all | 21–32 ms | 34–36 ms | 1.7 ms | 57–70 ms |
| Vulkan only | 9–16 ms | 39–43 ms | 1.7–2.8 ms | 55–57 ms |

So the one number quorra reports is split roughly two-to-three between two steps with completely
different causes — one is the driver loader, the other is physical-device enumeration — and a host
watching it for a regression cannot say which moved. **The ask is three fields where there is one**
(`instance_creation`, `surface_creation`, `adapter_selection`), or, at minimum, starting the clock
after `Instance::new` so that the name is true.

`crates/render-quorra/examples/bring_up.rs` in this tree is the measurement, and it is offered as
much for its own first version's mistake as for its numbers: it created two instances in one
process and reported 26.0 ms against 4.4 ms for the same work in the other order, which is entirely
the driver loader being warm the second time. **One configuration per process.**

### 8.2 An instance needs no window, and a host cannot supply one — **answered**

`Device::for_surface` creates the instance itself. That is the right default and it costs this
side the one lever the numbers above actually offer: **instance creation needs no surface, no
window and no event loop**, so it can be done on a thread started at `main`'s first line, while the
document is being read and the window created. Today it happens after both, because it happens
inside the constructor that takes the window.

Measured, same recipe, four processes (`bring_up overlap`): opening ISO 32000-2 — 13 MB, 1023
pages, 101 318 objects, `Document::open` + `Pages::new` + `Outline::read` — and creating a wgpu
instance, one after the other against both at once:

| | |
|---|---|
| document then instance | 44.4 / 46.1 / 49.7 / 50.0 ms |
| both at once | 22.9 / 27.0 / 28.9 / 28.9 ms |

**About 20 ms of a 145 ms launch, and it needs one entry point.** Either `Options::instance:
Option<wgpu::Instance>`, or a `Device::for_surface_with(instance, window, options)` beside the
existing one. `wgpu::Instance` is `Send + Sync`, so the thread that made it can hand it over.

What *cannot* be hoisted, and this side is not asking for: `request_adapter` takes
`compatible_surface`, so it is genuinely downstream of the window. The honest claim is "the
instance's share", not "bring-up's".

### 8.3 What is **not** a finding: the backend set — **and the knob was not added**

The obvious first guess — `Backends::all()` loads the GL backend for nothing on a Vulkan machine —
is wrong here, and the table in 8.1 is why: restricting the instance to Vulkan halves
`Instance::new` and gives every millisecond of it back in `request_adapter`. The total is the
invariant. **So this side is not asking for a backend knob in `Options`**, and would rather record
having measured it than have the knob added on the strength of a plausible argument.

---

## 9. The first frame pays ~12 ms that every frame after it does not, and it is not the shaders — **open**

**Status re-checked at `2531f447` in the three-hundred-and-eighty-fourth session, because that
revision touched `spawn_warm_up` and it would be easy to read it as an answer.** It is not:
ADR 0018 gives the warm-up thread a `JoinHandle` so a `Device` cannot outlive it, which is about
*teardown*. What this section asks for is the opposite end — that the per-device resources a first
frame creates be created on that same background thread — and nothing in either new commit does
it. The ask stands, unchanged.

**Measured in this viewer's two-hundred-and-eightieth session**, on the machine's real adapter —
AMD Radeon 890M, RADV, Vulkan — headless, page 7 of ISO 32000-2, ten renders of the same display
list to the same target, nothing waited for:

```text
bring-up   33.6 ms
frame 1    18.17 ms
frame 2     4.86 ms
frames 3–10 3.7 … 5.1 ms
```

The same shape at other scales, so it is a fixed cost rather than a proportional one:

| target | frame 1 | steady | difference |
|---|---|---|---|
| 596 × 842 | 18.17 ms | 4.86 | **13.3** |
| 1191 × 1684 | 24.11 | 9.79 | **14.3** |
| 2382 × 3368 | 57.24 | 39.15 | **18.1** |

**It is not pipeline compilation, and the experiment that says so is one argument.**
`crates/render-quorra/examples/first_frame.rs` takes a settle time and sleeps for it between
bring-up and the first render, which is more than enough for `spawn_warm_up`'s background thread
to finish (`StartupTimings::pipeline_compilation` reports 5.3–5.7 ms):

```text
settle    0 ms → frame 1  16.05 ms
settle  300 ms → frame 1  15.26 ms
settle 1000 ms → frame 1  16.65 ms
```

Unchanged. So what the first frame is paying for is **first-use resource creation** — buffers,
bind groups, the atlas texture, whatever a `Device` makes once and reuses — and it is paid at
exactly the moment `CLAUDE.md` cares about, because page one goes to the device and nothing on the
launch path is allowed to wait for warmth.

**The ask, and it is the same shape as the warm-up you already have**: warm the *allocations* as
well as the shaders. `spawn_warm_up` already runs a background thread whose whole purpose is to
have things ready before a frame asks; if the per-device resources a first frame creates could be
created there too, ~12 ms comes off every cold launch of every host, and nothing about the API
changes.

**This is a good-news finding as much as a request.** The rule this project holds you to — return
a usable device before it is warm, never block on warmth — costs *nothing measurable* on this
adapter: the shaders are ready before anything asks for them, three times over. The
`wait_until_warm` that a nervous host might reach for would buy zero milliseconds and hide the 12
that matter.

### 8.4 What came back, and what it is worth here

**Answered at `7d5dafb`, in both shapes §8 asked for and with §8.3's silence kept deliberately.**
quorra's ADR 0014 is the argument; what this side can add is the measurement from the other end
of the same launch.

- **`StartupTimings` is five fields where it was three.** `instance_creation` (an `Option`, `None`
  when the host supplied the instance — "reporting zero for work someone else timed would be a
  number that lies about what it measured"), `surface_creation`, `adapter_selection`,
  `device_creation`, `pipeline_compilation`, and a `blocking_total` that excludes the last because
  nothing waits for it. `adapter_enumeration` is **gone rather than deprecated**, which is the
  right call: keeping a name known to misdescribe its contents preserves the defect.
- **`create_instance`, `headless_with_instance` and `for_surface_with_instance`.** The instance is
  quorra's own — the descriptor has to match, and a host that guessed it would find out at
  `create_surface` — so `render-quorra` re-exports it as `QuorraPresenter::instance` and the
  viewer's `main` spawns a thread for it at its first line.
- **No backend knob**, with §8.3's measurement quoted in the ADR as the reason the silence is
  deliberate.

**What it is worth, measured here** (ADR 0185): `pdf-viewer --trace` on ISO 32000-2, under `Xvfb`
with `lavapipe`, three runs each.

| step | before | after |
|---|---|---|
| graphics instance | *inside* bring-up | +0.006 to +2.6 ms, hidden behind `EventLoop::new` |
| graphics device | +33.4 to +45.1 ms | **+13.2 to +19.2 ms** |
| process start → first frame | 145 to 152 ms | **110 to 119 ms** |

And the split does what §8.1 asked: `instance None, surface 0.03 ms, adapter 5.3, device 8.4` is a
line a host can read a regression out of. **Your headless measurement and ours disagree about the
share and both are right** — you measured 3.2–4.4 ms of adapter selection headless, we measure
5.3–6.8 with a `compatible_surface` under a virtual X server. That is the argument for the split,
made by the field.

---

## 10. The CPU coverage lane was charged for the GPU lane's winding texture — **answered**

**Five corpus pages that drew at `7d5dafb` are refused at `7599081`**, all with the same message:

```
frame refused: frame needs 616862585 scene-derived bytes, over the stated budget of 268435456
```

`bug1703683_page2_reduced.pdf` (616 862 585), `issue14497.pdf` (312 400 361), `issue12810.pdf`
(280 762 806), `issue1905.pdf`, `issue9418.pdf`. The sixth refusal is `bug1721218_reduced.pdf`,
which has been refused for coverage extent since §0's first run and is not this.

**`DEFAULT_MAX_FRAME_BYTES` did not move** — the constant and its comment are byte-identical
between the two revisions. What changed is what a frame is *charged*.

### The two sites disagree

`encode.rs`, at the end of `encode`:

```rust
let mut winding = std::mem::take(&mut encoder.winding);
let scratch = std::mem::replace(&mut encoder.scratch, ScratchPacker::new(1, 1)).finish();
if let Some(sheet) = scratch.as_ref() {
    winding.width = sheet.width;
    winding.height = sheet.height;
}
encoder.charge(winding.device_bytes())?;          // ← unconditional
```

`device.rs`, where the texture is actually made:

```rust
*bytes = bytes.saturating_add(scratch.data.len() as u64);
if !winding.is_empty() {                           // ← guarded
    *bytes = bytes.saturating_add(winding.device_bytes());
    crate::winding::render_into(…)?;
}
```

`Winding::device_bytes` is `width × height × 8` (rgba16float) plus the vertex and tile buffers,
and `width`/`height` are taken from **the whole scratch sheet** — which is sized by the CPU lane's
tiles just as much as by the GPU lane's, because both share one sheet. `is_empty()` is
`tiles.is_empty() || vertices.is_empty()`.

So under `Coverage::Cpu` — the default, and what an offscreen `Device::headless` gets — a frame is
charged eight bytes per texel of its entire coverage sheet for a texture that is then **not
allocated, not counted at the allocation site, and never rendered into**. A page whose sheet
reaches 32 M texels is refused on a phantom quarter-gigabyte. The pre-flight check is stricter
than the thing it is checking, which is the one direction a budget must not be wrong in.

### The fix, and what it restores

Guarding the charge the way the allocation is guarded:

```rust
if !winding.is_empty() {
    encoder.charge(winding.device_bytes())?;
}
```

**Reproduced here** by patching a local checkout of `7599081` with exactly that and re-running §0's
instrument. Before: 6 refused. After:

```
957 pages compared in 23.8s: 913 agree, 43 differ, 1 refused, 17 not comparable
```

913 / 43 / 1 / 17 is this gate's recorded state from before the coverage lane landed, to the
number — so the change costs nothing else and restores exactly what it took.

### One thing worth a second look while you are there

`device_bytes()` returning a non-zero size for an empty sheet is the proximate cause, and the
deeper shape is that `width`/`height` are assigned from the shared sheet before anyone asks
whether this frame has a GPU lane at all. Either `device_bytes` answers zero when `is_empty()`, or
the dimensions are only stamped when there are tiles to stamp them for. The first is smaller; the
second makes the invariant hold at the point where it is easy to see.

**And the constant's comment had a counterexample, which the fix withdrew.** It says 256 MiB is
"roughly eight million rectangle commands — beyond any real page by orders of magnitude", and five
of 974 real first pages were within reach of it, one at 2.3×. With the phantom texture gone, none
is: all five draw, and the largest genuine charge in the corpus is back below the budget. The
comment is safe, and it is safe *because somebody checked* rather than because nothing complained.

### What came back — `0a1ffb13`

**The fix went one level deeper than the one this section proposed**, and it is the better of the
two options named above. Rather than guarding the call site, `Winding::device_bytes` answers zero
for an empty sheet, above a comment that says why that is not merely tidier arithmetic:

> Not merely an optimisation of the arithmetic below: `is_empty` is exactly the condition
> `Device::upload_scratch` allocates under, and saying it once is what stops the pre-flight and the
> allocation from disagreeing again.

Which is the right reading of the defect: the two sites did not disagree by accident, they
disagreed because the condition was written twice. Guarding the charge would have made it three.

**Re-measured here** after bumping `Cargo.lock` to `0a1ffb13`, §0's instrument unchanged:

```
957 pages compared in 24.0s: 913 agree, 43 differ, 1 refused, 17 not comparable
```

913 / 43 / 1 / 17 — this gate's exact state from before the coverage lane, restored, with the one
remaining refusal the coverage-extent one that has been argued since §0's first run.

## 11. The GPU coverage lane draws the **wrong glyph** after a larger frame — and it stays wrong — **answered**

**New, 2026-08-05, and it is a defect rather than a request.** The project owner reported it from
the window: *"I do not get the same output at the same zoom level. When I zoom in, the output looks
fine, but then it starts being wrong, and zooming out again keeps having broken fonts."* The
screenshot shows a page of text where some letters are missing and at least one is a different
letter — `extensive` comes back as `extens:ve`.

It reproduces **offscreen, on the software adapter, in two frames**, with no window, no surface and
no chrome involved.

### The recipe

`crates/render-quorra/examples/zoom_ladder.rs` in this tree walks a page up a ladder of
magnifications and back down it, through **one** `QuorraRasterizer`, switching to
`Coverage::Gpu` at 10× exactly as `viewer-ui` does — and compares every rung against
`render-cpu`, which is this project's correctness oracle.

```sh
cargo run --release -p render-quorra --example zoom_ladder -- doc/PDF20_AN001-BPC.pdf 3
```

```text
 leg      zoom        target       mean     worst      ssim
  up      800%   4761 × 6734      0.1175      1.51   0.99950
  up     1600%   9523 × 13468     0.0347      1.50   0.99978
  up     3200%  19046 × 26937     0.0166      0.48   0.99991
  up     6400%  38092 × 53875     7.6295    191.25   0.94068     ← wrong
down     3200%  19046 × 26937     0.0166      0.48   0.99991
down     1600%   9523 × 13468     7.1524    173.98   0.91892     ← wrong, and it was right on the way up
down      800%   4761 × 6734      0.1175      1.51   0.99950
```

The `-- <file> <page> <out-dir>` form writes both backends' rasters per rung. At 6400% the page
reads `ort` on the CPU and **`orT`** on the GPU lane: a lowercase *t* drawn as a capital *T*, at
the right position and the right size. That is the owner's `extens:ve` — a glyph replaced by
another glyph, not a glyph lost.

### It is state, not magnification

| what is asked | result |
|---|---|
| a **fresh** device whose first frame is 6400% | **clean**: mean 0.0134, ssim 0.99993 |
| a device that drew one 3200% frame, then 6400% | **wrong**: mean 7.6295, worst tile 191.25 |
| the same device afterwards at 1600% | **wrong**: mean 7.1524 — and 1600% was 0.0347 on the way up |
| the same device afterwards at 3200% and 800% | clean |

So the minimal reproduction is **two frames on one device**, and the damage **reaches backwards**
to a magnification that was correct on that same device minutes earlier. Nothing in the display
list changes between the two: it is the same `Arc<DisplayList>`, the same commands, the same
`Arc<Path>` glyph outlines, and only `TargetSpec::transform` differs.

### What it is not

- **Not the driver.** This is `lavapipe` under `Xvfb` — a software adapter. The owner sees it on
  RADV, so it is common to both.
- **Not the surface or the presenter.** `QuorraRasterizer::rasterize` with `Target::Readback`,
  no swapchain.
- **Not the coverage lane's *quality*.** With `Coverage::Cpu` at every rung the same ladder is
  clean at every rung, up and down — 0.0166 at 3200%, 0.0301 at 6400%. The lane switch is what
  admits the defect.
- **Not this project's chrome or its transform arithmetic.** The comparison is against
  `render-cpu` on the *same* target spec, and the CPU raster is right at every rung.
- **Not the atlas budget**, as far as this side can tell: squeezing `Options::atlas_budget` from
  the default 8 MiB down to 4 KiB changes nothing at 2× (`examples/atlas_squeeze`).

### Where this side would look first

Your ADR 0016 is quoted in this tree's own constant: a glyph's rasterised coverage is kept in an
atlas **until the glyph exceeds 128 device pixels**, past which it is rasterised again every
frame. Every broken rung is past that threshold and so are two of the clean ones, so the threshold
is not the whole story — but a cache whose key is a *size bucket* and whose slots are shared with
the large-glyph path would produce exactly this shape: a much larger glyph rasterised into, or
invalidating, a slot another bucket still answers from, so a later frame at the smaller size draws
whatever is in the slot now.

The selectivity is the clue worth having: after the 6400% frame, **1600% is wrong while 3200% and
800% are right**. Whatever is overwritten is not "everything smaller".

### What it costs this viewer

`viewer-ui` switches to the GPU lane above 10× magnification because the two lanes' cost curves
cross there (`doc/quorra-gpu-coverage.md`: 0.44 ms a frame at 8× against 4.4 ms at 12×). Until
this is fixed, a person who zooms past 1000% and comes back sees a page of wrong letters and has
no way to clear it except reopening the document. The obvious mitigation on this side — stop
switching lanes — costs the ten-fold frame time the switch was measured to buy, so it is being
held until you have looked.

### What came back — `52b07f29`

**The state that survived the frame was the winding texture, and what leaked was not its
contents but its size.** ADR 0016 keeps that texture between frames and grows it to the largest
sheet any frame has needed, because allocating and zeroing it per frame cost 10.7 ms of a 15 ms
frame at 20×. What the growth also did was break an equality nothing had written down: clip space
spans the *attachment*, and `vs_winding` reaches it by dividing by the *sheet*. While the two were
equal the mapping was pixels. Once the texture could outlive a taller frame, a smaller frame's
geometry was stretched over the whole of it — every sheet pixel written `held ÷ sheet` times too
far down — while the resolve pass went on reading sheet coordinates as texels of the same texture.
Each tile then resolved whatever the stretch had put under it: another glyph's coverage, at the
tile's own place and size. `orT` for `ort`, and `extens:ve` for `extensive`.

That also settles the selectivity this section called the clue worth having, and it is worth
printing rather than reasoning about. The scratch sheet is the device's maximum dimension wide
always, so only its *height* varies, and the height is what a frame's tiles **pack** — not what
its magnification is. Instrumenting `render_into` on this very ladder, at `0a1ffb13`:

| rung | tiles | sheet | texture held | verdict |
|---|---|---|---|---|
| up 1600% | 23 | 16384 × **417** | none — first GPU frame | right |
| up 3200% | 6 | 16384 × **533** | 417, grown to 533 | right |
| up 6400% | 4 | 16384 × **349** | **533** | **wrong** |
| down 3200% | 6 | 16384 × **533** | 533 | right |
| down 1600% | 23 | 16384 × **417** | **533** | **wrong** |
| down 800% | — | CPU lane | — | right |

**The 3200% sheet is the tallest**: six mid-sized tiles pack more rows than four huge ones do, and
the shelf packer will not put a tile in a shelf more than twice its height, so a mixture of sizes
opens shelves rather than filling them. The high-water mark was therefore set at 3200%, and every
wrong rung is a rung whose own sheet was shorter than it — 349 stretched over 533 at 6400%, 417
over 533 at 1600%. Nothing was overwritten and nothing was evicted; a wrong rung is a rung that
was made to read its own sheet through the wrong scale factor.

The fix is a viewport of the sheet's extent at the winding target's origin, which makes the two
passes agree without either shader learning the size of a texture that is the module's business
alone. The invariant is now stated where the texture is grown — **the sheet is the top-left of
this texture** — and ADR 0016's bullet about keeping it between frames says what the keeping
costs.

**Measured against this ladder.** `zoom_ladder` reproduced there against `0a1ffb13` to the digit,
which is what says the instrument crossed the tree intact; the same run against `52b07f29`:

```text
 leg      zoom        target       mean     worst      ssim
  up     1600%   9523 × 13468     0.0347      1.50   0.99978
  up     3200%  19046 × 26937     0.0166      0.48   0.99991
  up     6400%  38092 × 53875     0.0134      2.79   0.99993
down     3200%  19046 × 26937     0.0166      0.48   0.99991
down     1600%   9523 × 13468     0.0347      1.50   0.99978
down      800%   4761 × 6734      0.1175      1.51   0.99950
```

6400% is 0.0134 / 0.99993, which is **this section's own fresh-device control to the digit** — the
lane on a device with no history was already right, and now a device with a history draws the same
thing. Every rung of the descent equals its ascent. §0's corpus gate is unmoved at 913 / 43 / 1 /
17.

The regression test is `tests/frame_independence.rs` in that tree, and it is deliberately not a
test about zoom: it renders a scene on a device that has already drawn a *larger* frame and
requires the pixels to equal what a device that has drawn nothing produces, under both lanes. Two
frames on one device, which is all this ever needed.

**One caveat about the measurement, because a number should say what was actually run.** The
numbers above were taken in a copy of this tree with a `[patch]`
pointing at quorra's working tree, not by bumping `Cargo.lock` — so they say the fix is right, and
this side should still re-run the ladder and the gate against the published revision before the
lane switch in `viewer-ui` is considered safe again.

### And a second symptom of the same thing, on this side — the *overlays*

**2026-08-06.** This tree had a second high-zoom defect open, reported by the project owner and
believed unrelated: `viewer-ui`'s sidebar stops being drawn above about 2000% magnification. It is
the same defect, and the reproduction is worth having because it is a **different kind of
geometry**.

`crates/viewer-ui/examples/chrome_ladder.rs` draws the window's whole frame offscreen — the page
under its target transform, and a display list of window-pixel chrome at identity over it, which
is what `present` composes into one scene — and crops the panel's own 300 columns out of each
rung. The panel is the same list at the same target on every rung, so its pixels may not depend on
the page's magnification.

On `lavapipe`, `doc/PDF20_AN001-BPC.pdf` page 3, 900 × 1100, GPU coverage lane above 10×:

| zoom | page target | panel mean vs the first GPU-lane rung | panel ink | one device | a device per rung |
|---|---|---|---|---|---|
| 1200% | 7143 × 10102 | reference | 19.57 | — | — |
| 1900% | 11309 × 15995 | 0.0002 | 19.57 | same | same |
| **3000%** | 17857 × 25254 | **3.7733** | **14.53** | **wrong** | same (0.0003) |
| **4600%** | 27380 × 38723 | **3.9170** | **15.09** | **wrong** | same (0.0003) |
| 6400% | 38093 × 53876 | 0.0003 | 19.57 | same | same |

Same signature as the section above: **clean on a device with no history, wrong on a device that
has drawn a taller frame, and not monotone in the zoom.** At 3000% the panel is its background
rectangle alone, shifted about **43 px down**, with the tab strip and every row gone.

Two things in it may be useful to you:

- **The displacement reaches a plain filled rectangle**, not only glyphs. The panel's background
  is one `rect` at identity and it moves with the rest, which fits `held ÷ sheet` stretching every
  sheet pixel rather than anything about the glyph atlas.
- **The overlay's own geometry is tiny and at identity** while the page's is enormous, and they
  are in one scene. So the stretch is not a property of the commands being magnified — it is the
  frame's sheet against the texture's, applied to whatever is in the frame.

Nothing is asked for here: `52b07f29`'s fix is a viewport at the sheet's extent, which covers both.
It is recorded so that your regression test knows a second shape to check, and so that this side
can say what it expects to see when the fix is published: `chrome_ladder` saying `same` on every
rung of its one-device pass.

### Verified on this side at `52b07f29` — **closed**

**2026-08-06.** `Cargo.lock` moved from `0a1ffb13` to `52b07f29` and both ladders were re-run on
this machine. Nothing here is a request; it is the receipt.

**`zoom_ladder`, one device, up and back down**, GPU coverage lane above 10× as `viewer-ui`
switches it:

```text
 leg      zoom        target       mean     worst      ssim
  up      100%    595 × 841       0.5579     17.79   0.99270
  up      200%   1190 × 1683      0.9502     17.73   0.99127
  up      400%   2380 × 3367      0.3797     23.37   0.99778
  up      800%   4761 × 6734      0.1175      1.51   0.99950
  up     1600%   9523 × 13468     0.0347      1.50   0.99978
  up     3200%  19046 × 26937     0.0166      0.48   0.99991
  up     6400%  38092 × 53875     0.0134      2.79   0.99993
down     3200%  19046 × 26937     0.0166      0.48   0.99991
down     1600%   9523 × 13468     0.0347      1.50   0.99978
down      800%   4761 × 6734      0.1175      1.51   0.99950
down      400%   2380 × 3367      0.3797     23.37   0.99778
down      200%   1190 × 1683      0.9502     17.73   0.99127
down      100%    595 × 841       0.5579     17.79   0.99270
```

**Every rung of the descent equals its ascent to the digit**, and 6400% is 0.0134 / 0.99993 —
this section's own fresh-device control, which is exactly what your note predicted. Against
`0a1ffb13` the same run gave 7.6295 / 191.25 / 0.94068 going up and 7.1524 / 173.98 / 0.91892
coming back down.

**`chrome_ladder`, the overlay shape above**: the one-device pass now equals the device-per-rung
pass at every rung — 0.0003 mean, worst 16, ink 19.57 throughout, where 3000% and 4600% were
3.7733 and 3.9170 at ink 14.53 and 15.09.

**§0's corpus gate is unmoved**: 913 agree, 43 differ, 1 refused, 17 not comparable, on this
machine's real adapter (RADV, Radeon 890M). The one refusal is the coverage-extent one that has
been argued since that section's first run.

`viewer-ui` keeps switching lanes at 10×, which was the question the mitigation would have
answered: there is nothing to mitigate. And this side now has a gate for the overlay shape —
`viewer-ui/tests/chrome_over_a_magnified_page.rs`, seven frames on a software adapter — checked
by pinning `0a1ffb13` back for one run and watching it fail with the number above.

**Re-verified at `2531f447`** in the three-hundred-and-eighty-fourth session, because a revision
bump can move anything: `zoom_ladder` on the same recipe prints the thirteen rows above **to the
digit**, the overlay gate is green in `cargo test --workspace`, and §0's corpus gate is unmoved at
914 / 42 / 1 / 17.

---

## 12. A caller cannot choose the backend, and on Windows wgpu chooses Vulkan — **answered**

**New, 2026-08-07, and it is a request rather than a defect** — the defect it is a way around is
an Intel Vulkan driver's. The project owner ran this viewer on a Windows machine with Intel
graphics and **it crashed inside the Vulkan driver**. That is nobody's code here; what makes it a
report is that there is no way to ask for the other backend, and the machine has one.

### What a caller can ask for today, read out of `0a1ffb1`

- `quorra_gpu::create_instance()` is `wgpu::Instance::new(InstanceDescriptor::new_without_display_handle())`.
  That descriptor's `backends` is `Backends::default()`, which is `Backends::all()` — on Windows,
  Vulkan **and** DX12 **and** GL — and it is built **without** `.with_env()`, so `WGPU_BACKEND` is
  not consulted either. There is no parameter and no second entry point.
- `Options` carries `adapter: Option<String>`, `max_frame_bytes`, `max_resource_bytes`,
  `atlas_budget`, `glyph_quantum`, `coverage` and `coverage_samples`. None of them names a backend.
- So the answer is: **not at all**, by any route — argument, option or environment.

**And `Options::adapter` is not a way round it**, which is worth saying because it looks like one.
`select_adapter` enumerates `Backends::all()` and filters on a case-insensitive substring of
`get_info().name`. One GPU is enumerated once per backend that can drive it, and the name it
reports is the *device's* — so on a machine with one Intel GPU, "Intel" matches the Vulkan adapter
and the DX12 adapter equally, ties are broken by name order, and a name cannot express "this GPU,
through DX12". The filter selects hardware; the question here is which driver stack talks to it.

**Which one wgpu picks with no filter is not the caller's choice either.** `select_adapter`'s
`None` arm asks `request_adapter` with `PowerPreference::HighPerformance`; among adapters of equal
device type that resolves in wgpu's own hub order, and Vulkan precedes DX12. On Windows that is
how the machine above reached the driver that crashed it. We are not asking you to change the
preference — `HighPerformance` is right — only to make the set it chooses from something a caller
can state.

### The ask, in the same shape as §8's

**One parameter, at the instance**, because backends are an instance-level choice and the instance
is made before `Options` exists:

```rust
pub fn create_instance_with(backends: wgpu::Backends) -> wgpu::Instance;
```

`create_instance()` stays exactly as it is and keeps its meaning. A host that has been told by its
user to use DX12 passes `Backends::DX12`; a host that has not, calls the existing function and
nothing changes for it. `Backends` is already in your public surface through the `wgpu` re-export,
so this adds no type.

**A second question, and it is yours rather than ours**: whether `create_instance` should read
`WGPU_BACKEND` — `wgpu-types` offers `.with_env()` for exactly this and you are deliberately not
calling it. There is an argument for not calling it (a library that changes behaviour from the
environment is hard to reason about, and §4.6's determinism is a stated value of this project), and
an argument for calling it (every other wgpu program on the machine honours that variable, so a
person debugging a driver expects it to work). We have no preference and would rather it were
decided than defaulted. **What we would not want is the environment being the *only* route**: a
viewer needs to be able to put the choice on its own command line.

### What this side will do meanwhile

Nothing that hides it. `pdf-viewer` will gain a `--backend` flag whose whole implementation is the
parameter above, and until it exists the flag cannot be honest, so it is not being added. The
half of the owner's report that **is** ours — that `--cpu` brings a graphics device up anyway, so
a driver that crashes during bring-up crashes a run that asked for the processor — is
`doc/todo/12` in this tree and is not your problem.

### What we cannot measure

**No machine here runs Windows.** Everything above is read out of your source and wgpu's; the
crash is the project owner's report from their own machine, and this project has no Intel adapter
and no DX12 to reproduce it on. Treat the mechanism as argued rather than observed, and the
missing parameter as the only claim we are certain of.

### What came back — `2531f447`

**Exactly the parameter this section asked for, and two things it did not.**

- **`create_instance_with(backends)`**, at the instance, with `create_instance()` unchanged and
  now that function with `Backends::all()`. Nothing this side had to bend: the ask and the answer
  are the same signature.
- **`Device::adapter_names_on(&instance)`**, which was not asked for and should have been. A host
  that restricted its backends and then listed adapters through `Device::adapter_names` — which
  makes its own all-backends instance — would offer a choice its own constructors could not
  honour. The parameter created that trap and the same commit closed it. Our `--backend`'s
  refusal message prints *both* lists for exactly that reason, and the difference between them is
  the diagnosis: `adapters behind it: none` is a backend this machine has no adapter for, while a
  non-empty list under a failed device is an adapter that cannot present to this surface.
- **`WGPU_BACKEND` is not read**, and ADR 0017 gives the argument: a library that renders through
  a different driver because a variable was exported has a failure mode that reproduces nowhere.
  This side had no preference and asked only that it be decided; it was, and the one thing we said
  we did not want — the environment being the *only* route — is not a route at all. `pdf-viewer`
  does not read the variable either, and the reason is the same one in the host's own words: the
  command line is where a person can see what they asked for.

**What it is worth here.** `pdf-viewer --backend vulkan|dx12|metal|gl`, one `match` and one
argument as §12 predicted, refusing rather than falling back where the machine has no adapter
behind the name (ADR 0221). And the **Windows default is now a choice this project makes**:
`#[cfg(windows)]` asks for DX12 first, where before the answer came from wgpu's hub order putting
Vulkan ahead of it. That default *gives way* to every backend where no DX12 adapter exists, while
a backend a person named is refused — because one is our guess about their machine and the other
is their answer about it.

**And one thing that came with it and is worth a host knowing** — `7cbf6e8`, ADR 0018, in the same
pull. `spawn_warm_up`'s thread was detached, so a `Device` dropped before it was warm could reach
`exit()` with a thread still inside `vkCreateGraphicsPipelines` while Mesa's atexit handlers tore
the driver down underneath it; 13 of 15 runs of their new `device_lifecycle.rs` died *after* the
tests passed. `Device` now joins the handle in `Drop`. It costs ~5 ms to a device dropped before
it is warm and nothing to one dropped after, so it falls on the probe, which is the case that was
crashing. **Nothing in this tree was reported as crashing on exit** — but this tree does construct
and drop devices per gate run, and a crash in teardown after `test result: ok` is precisely the
shape nobody attributes.

### What still cannot be measured, and what the owner must run

**No machine here runs Windows, has an Intel adapter or has DX12.** That the crash goes away
under DX12 is a hypothesis about somebody else's driver and this side has not tested it; what is
certain is that until `2531f447` the question could not be asked. Our side's half — that `--cpu`
now opens no driver at all — **is** demonstrated, on Linux, with `strace`: 56 shared objects and
three Vulkan libraries before, 17 and none after.

---

## 13. Half a page turn is `encode`, and it is CPU — **open, and it is a request for an instrument before it is a request for speed**

**New, 2026-08-08, from this viewer's three-hundred-and-ninety-first session.** It is not a defect
and nothing here is wrong; it is a measurement, taken because the project owner's own document felt
slow and this side finally built a trace that can say *which stage* a frame went into (our ADR
0227). What the trace found is that four fifths of a frame is inside `Device::render` and half of
*that* is `encode` — which is host processor time, not the device's.

### The measurement

`NorthAmerican.30MB.pdf`, 65 pages, 30 MB, the project owner's own file. 38 page turns driven by
`xdotool` under `Xvfb` at 800×1000, `pdf-viewer --trace=frames`, release build. **The adapter is
`llvmpipe`, so every absolute number below is this machine's software rasteriser and the
`execute` row in particular says nothing about a GPU.** What survives that caveat is `encode`,
which is the same host code on any adapter, and the *shape* of what depends on what.

Per frame, over the 38 frames that draw a real page (388 to 3675 scene commands, 78 to 793
resource uploads). `r(cmd)` is the correlation with the scene's command count and the fit is a
least-squares line through it:

| phase | min | median | max | sum | r(cmd) | fit |
|---|---|---|---|---|---|---|
| `device` whole | 6.91 | 25.65 | 86.44 | **963.5** | +0.35 | 5.45 µs/cmd + 12.88 ms |
| — `encode` | 1.42 | 13.52 | 32.18 | **481.2** | **+0.58** | **3.86 µs/cmd + 3.84 ms** |
| — `upload` | 0.50 | 3.09 | 19.17 | 137.1 | +0.19 | 0.75 µs/cmd + 1.89 ms |
| — `execute` | 2.89 | 4.65 | 15.24 | 194.0 | +0.12 | 0.26 µs/cmd + 4.50 ms |
| — elsewhere | 1.09 | 3.16 | 20.71 | 151.3 | +0.15 | 0.58 µs/cmd + 2.66 ms |

Against a whole session of **1074 ms** of frames, of which this host's own work — every query it
asks, the display lists it translates into a `Scene`, the resources it hands over, the transients
it releases — is **71 ms**.

**So `encode` is 45% of a page turn and it is the only phase that tracks the scene's size.**
`upload` follows the uploads instead (r = +0.76 against them), which is exactly what it should do;
`execute` is nearly flat, which on this adapter says more about `llvmpipe`'s own floor than about
anything else.

### What we can and cannot see into

Plainly, because it decides what this section is worth:

- **We can see** the three durations `Frame::timings` reports, the `TimingProvenance` beside
  `execute`, and the counters — commands, culled commands, bytes transferred. Those are read
  rather than manufactured, which is what made this table possible at all, and they are the
  reason this report exists rather than a guess.
- **We cannot see anything inside `encode`.** Whether those 3.86 µs a command are path
  flattening, bind-group churn, buffer writes, sorting, or `wgpu`'s own command recording is
  invisible from here, and profiling it from this side would be profiling a build of yours we did
  not configure.
- **We have ruled out our own end of it**, which is why this is not a report about our numbers.
  The same session found and fixed the one thing on this side that was large — a per-source-sample
  image reduction, which took our `scene` stage from 210 ms of the session to 71 (our ADR 0228) —
  and the `device` figures did not move at all: 994/1022/1015 ms before against 999/1026/1005
  after, three runs each. `encode` is 469/486/484 before and 480/490/483 after. It is independent
  of everything we changed.
- **We are not asking you to work around a scene we rebuild.** This host builds a fresh
  `quorra_scene::Scene` every frame, so nothing inside `encode` *can* be reused across frames, and
  that is ours to change rather than yours. The per-command figure is worth your knowing anyway,
  because it is what a retained scene would have to beat.

### The ask

**One instrument, in the shape you already chose.** `Frame::timings` splits `Device::render` into
three phases and that split is what turned "the frame is slow" into this table. `encode` is now
the largest of the three and is itself unsplit, so the same question repeats one level down: a
subdivision of it — however coarse, two or three parts — would say whether 3.86 µs a command is
geometry, binding or recording. **An instrument before an optimisation**, and it is the same
argument your three phases already won.

**And a second thing, which is about arithmetic rather than speed.** Our summary prints an
`elsewhere` row — `device` minus your three phases — to name the swapchain acquire, the present
and the timestamp readback rather than leave an unnamed remainder. It is 151 ms of 963 here, 16%,
and **we no longer believe it is a duration of anything**: where `execute` comes from timestamp
queries it is the *adapter's* clock, and subtracting it from our host-side wall clock around
`Device::render` leaves whatever the two disagree by mixed in with the three things we meant to
name. So we have downgraded our own row to a bound and said so in the output. Two ways out, and
either would do:

- report the acquire and the present as phases of their own, leaving a remainder small enough that
  nobody has to trust it; or
- say which clock each phase is on, so a caller knows the three are not summable with a host
  timer.

**We are not asking for the wait to be removed.** `Device::render` blocking on the device before it
returns is what makes `execute` reportable at all, and this side depends on that: it is the reason
our trace can say a frame's cost without introducing a fence of its own.
