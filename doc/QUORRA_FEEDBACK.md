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

**§8 was answered at `7d5dafb` and §9 is open.** Both were requests rather than defects, and both
exist because the project owner's decision that page one goes to the graphics device put your
bring-up on this viewer's critical path. §8 asked for a field split and an entry point and got
both, plus a refusal of the knob it said not to add; §9 is from the other end of the same launch —
the first frame allocates ~12 ms that every frame after it reuses, and it is provably not the
shaders.

**Where it stands, at the page's own scale:**

| | first run | now |
|---|---|---|
| agree | 900 | **913** |
| differ | 50 | **43** — 28 of them the antialiasing floor (§4) |
| refused | 7 | **1** — and 6 for one day, which is §10 |
| median page | 2.64× the CPU backend | **2.05×** |

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

## 9. The first frame pays ~12 ms that every frame after it does not, and it is not the shaders

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
