# quorra, measured against the corpus — findings, and what came back

Written 2026-08-03 at the end of this viewer's hundred-and-ninety-fifth session, from the run
described in ADR 0156; **rewritten the same day, after every finding in it was answered.** It is
the counterpart to `RENDER_LIBRARY.md` — that document is the brief this project wrote for a team
building a renderer, and this one is what came back when the renderer met 974 real documents,
and then what the team did about it.

Each finding below keeps its evidence and carries what closed it, because a feedback document
that still reads as a complaint after the complaint was answered is worse than no document.

**Three sections were answered at once at `a35dc703`, and all three were asks this document made:
§20.4's page, §18's rule and §9's first frame.** Each is marked below with what reproduced here.

- **§20.4's `transparency_group.pdf` was a defect and it was not the sampling.** The instinct in
  that section — that sixteen samples cannot cap a worst tile at 31.7 of 255 — was right, and
  `ab219d0` found the reason: `ScratchPacker` restrides its rows down to the width the shelves
  reached and left the wide layout's tail behind, so a shelf whose tiles wrote no CPU bytes kept
  somebody else's coverage. It needed a shelf of *only* GPU-lane tiles to be reachable at all,
  which is why the lane's first corpus-scale run is what produced it. **Reproduced here exactly**:
  gpu lane at 1× **909 → 914 agree, 43 → 38 differ**; at 4× **920 → 926 agree, 20 → 14 differ**;
  the lane this tree renders on unmoved at 915 / 37 / 5. Five pages and six pages, none going the
  other way.
- **§18's question is answered from the standard rather than from our reading of it**, which is the
  better outcome: `0ddaa40` takes `min` across a chain on §8.5.4's own sentence — the graphics
  state holds *one* clipping path, so rasterising each link separately is a convenience and nothing
  in the standard composes two fractional coverages. Both sides still multiply where the clip meets
  the **mark**, both record it as a choice, and neither is the clause's area.
- **§9 is answered in part, and the part is exactly the size it was measured at.** `a35dc70` timed
  the inside of `Device::render` and found 2.43 ms of the first frame was creating that frame's own
  timestamp query; one lives with the device now. This side's instrument has a spread three times
  the effect, so it was measured as an A/B/A of eight samples an arm and read at the minimum:
  **14.94 ms → 12.77 and 12.47**, both `A` arms agreeing. §9.1 is what remains.

**And `2c9bdd0` answered §14.2 with two ADRs, which is the last thing standing between this tree
and §11.4.6's clause.** `6f777e8` accepts the staged pair inside a knockout group — the one
position this tree emits it from — and `2c9bdd0` puts a `compose` on `GroupSpec`, so a *group* can
be one stage, which is what §11.3.7.2's union-of-shapes forces for three of the four pages. This
side wrote the translation in the four-hundred-and-fifty-sixth session and **all four pages left
the refused list agreeing with the CPU oracle** (§14.3). The same bump made §13 and §18 answered
sections that had been sitting under `open` headings — one of them for three days, one of them for
eleven rounds while this document's own summary said otherwise (§13.1, §18.1). §20.6 is both
coverage lanes measured on this machine at that revision.

**§22 is the newest and it is the first bump that cost this tree not one line of source.**
`a7babab` sizes every plan — layer, mask, root — to what it marks, and on this machine **no page of
the corpus is refused for frame bytes at any scale any more**: the five documents this document has
named for that reason are drawn on both coverage lanes now, and four of the five agree with the CPU
oracle. What is left refusing at 4× is one
resource budget that is ours to set and three pages past this adapter's texture limit, which is a
list stable enough to be a **ratchet** rather than a paragraph — so the second lane §2 of
`doc/todo/02-every-round.md` requires has stopped being a survey. §9.2 is `Device::warm_for`,
declined a second time and now for a reason of yours as much as ours.

**§20 was the newest before it, and it is the first time this gate looked at your *other* coverage lane.**
`74c4994d` is three commits and every line of all three is inside it, so the run that used to answer
"a quorra release does not move this gate" was answering about a lane the release does not touch.
Pointed at the one it does: **twenty-four pages that could not be drawn at four times a page's own
scale now draw, twenty of them agreeing with our CPU oracle**, and nothing went the other way. §20.4
is the two things that come back to you, neither of them a defect.

**§19 is a finding of *yours about us* rather than either side's complaint.**
`04c0d23`'s `doc/corpus-profile.md` walked the 995 first pages this gate hands you and reports that
**not one of them emits a `Command::Rect`** — every rectangle a document draws arrives as a `Fill`
whose outline happens to be one. That is true and it is ours: `render-quorra`'s translation names
`quorra_scene::Rect` in exactly one place, `present.rs`'s blit, and nowhere on a page's path. Whether
it should is §19.

**And the release the project owner asked about does not move this gate, which is the second time
that has been established rather than assumed.** `595d8c87` fixes a regression `89d7dd77` introduced
— a multiply-xor-rotate accumulator whose low bits are the ones `hashbrown` indexes with — and
`c1f6e2f4` chooses the coverage lane per command by cost. Run as an A/B/A alternation, six samples
each, rebuilt between arms:

| pin | gate wall clock | serial rasterisation through quorra | agree / differ / refused / not comparable |
|---|---|---|---|
| `89d7dd77` | 26.3–27.1 s | 6.17–6.35 s | 917 / 35 / 5 / 17 |
| `595d8c87` | 26.2–27.3 s | 6.12–6.44 s | 917 / 35 / 5 / 17 |
| `c1f6e2f4` **(pinned)** | 26.9–29.0 s | 6.26–6.78 s | 917 / 35 / 5 / 17 |

Every band overlaps every other. Your own figures for the hasher are 468 → 397 ms on one page and
46 → 43 on another; 957 pages summing to 6.3 s of rasterisation cannot see that, and the unalternated
wall clock of this gate spanned 23.9–39.2 s at a *fixed* pin on the same day. **Read the middle
column as a null result about the instrument, not about the change** — the same thing §13's median
row had to say about ADR 0022, and the reason it is worth repeating is that the null arrived from two
independent directions.

**§18 is a *question* rather than a defect, and this side changed itself first.**
ISO 32000-2 §10.7.4 states clipping as an intersection of two *sets* of pixels and §8.5.4 says a
clip zeroes the shape outside it; both backends were composing a clip chain by multiplying
anti-aliased coverages, so one rectangle stated six times drew its edge at a twentieth of the mark.
`render-cpu` now takes the smaller of the two coverages (our ADR 0280) and the ask is what your
coverage lane does, since a chain is composed inside your device. **No gate moved** — 917/35/5/17
before and after, not one per-page line changed — which is why it is written down rather than left
to be rediscovered as a regression in whichever side is measured second.

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

**§14, §16 and §17 were the newest and all three were answered at `89d7dd77`, which is the release
`doc/QUORRA_NON_ISOLATED_GROUPS.md` describes.** §16 asked for one flag and got both halves of it —
`GroupSpec::isolated`, Table 145's `/I`, and a buffer that can begin as a copy of what is under it;
this side passes it through and three corpus pages left the refused list (§16.1). §14 asked for two
Porter-Duff operators and got exactly `Compose::DestOut` and `Compose::Plus`; the work that takes
its four pages off the refused list is now this side's rather than a request (§14.1). §17 asked
whether two rasters of one page were possible and the answer is that they already were — a test
holds it now, and the refusal that remains is on this side's display list (§17.1). **Three sections
answered in one release, and what each still owes is ours.**

**And the four-hundred-and-thirty-ninth session did the two halves that were ours, with opposite
results.** §17's was work and it is done: `render-quorra` renders the two lists as two
`Target::Readback` frames against one device, three corpus pages left the refused list, and §17 is
**closed** (§17.2). §14's was written out and **cannot be asked for** — `SceneBuilder::fill`
refuses a staged operator inside a knockout group, which is the one position
`pdf_render::Command::Shaped` occurs in, and `SceneBuilder::group` carries no compositing operator
at all while three of the four pages state a `Shaped` whose halves are *groups*. §14.1 recorded
both positions as things this side does not emit, on this side's own reading, and it was wrong
about the first and silent about the second. **§14.2 is the ask that follows**, and it is a
correction of ours before it is a request of yours.

**§13's instrument arrived in the same release and its other half was decided.** `encode`
subdivides behind `Options::instrument_encode` into geometry, staging and recording; `"target
acquire"` and `"present"` are named phases; and `Timings::host_total()` returns the three spans on
*this* side's clock, so the `elsewhere` row can stop subtracting an adapter's clock from a host
measurement. The threading question §13 raised was answered the way this side argued for — quorra
spawns no threads, and will take a pool rather than make one (its ADR 0023).

**§13 was the newest, is a request for an *instrument* rather than for speed, and its measurement
half is open.** A page
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

| | first run | at `2531f447` | at `89d7dd77`, session 438 | at `89d7dd77`, session 439 | at `c1f6e2f4`, session 447 | now, `2c9bdd0`, session 456 |
|---|---|---|---|---|---|---|
| agree | 900 | 911 | 914 | 917 | 917 | **919** |
| differ | 50 | 35 — 23 of them the antialiasing floor (§4) | 35 | 35 | 35, page for page the same list | **37** — two joined at ADR 0285's inclusive quantum, one left |
| refused | 7 | 11 | 8 | 5 | 5 | **1** |
| median page | 2.64× the CPU backend | 2.05× to 2.33×, run to run | 2.46× | 2.37× and 2.35×, two runs | 1.83× to 2.79×, eight runs across three pins | **1.75× and 1.78×**, two runs |

The fourth column is one revision of yours and one session of ours: the pin did not move, and the
three pages are §17.2's. The fifth is two revisions of yours and no change at all to any of the four
rows — see the alternation at the top of this document, and read the median row as the band it has
always been rather than as a figure that moved.

**The middle column is what this table used to call "now", and reading it beside the first is how
this document has been wrong.** It said `914 / 42 / 1` for the whole stretch in which sections 14,
16 and 17 were written — three sections whose *entire point* was that pages moved from agreeing to
refused, which is the number in the third row. A summary table that a section three screens down
contradicts is the ledger's disease one document over, and this one caught it twice: the
three-hundred-and-eighty-fourth session corrected a stale `913 / 43`, and the
four-hundred-and-thirty-eighth corrected a stale `914 / 42 / 1`. **The columns are dated now, which
is the fix for the shape rather than for the instance.**

**One refusal, and it is the page this list started with**: `bug1721218_reduced.pdf`'s coverage,
which is §15's page and a texture capacity rather than a hole in the vocabulary. §14's four stated
shapes left in the four-hundred-and-fifty-sixth session, when the two lifts §14.2 asked for arrived
at `2c9bdd0` and this side wrote the translation (§14.3); §17's three are gone.

The median is a *timing* and moves between runs on one revision — 2.05× and 2.33× were two runs of
the same gate — so it is a band rather than a figure that moved. quorra's ADR 0022 predicts it
should have *fallen* (a dense page's offscreen frame 4.94 → 1.65 ms, of which readback 3.84 → 1.32);
2.46× is a single run against a two-run band on a machine this one shares, so the honest statement
is that **this gate cannot resolve that change** and the offscreen ratio is not the instrument to
look for it with.

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
- `PDFVIEWER_QUORRA_SCALE=4` is the exception since §22: the *refusal* list is held at that scale
  too, on the default lane, because nothing left in it is arithmetic against a budget upstream
  keeps improving. The differing list is still a survey there and the run says so.
- `PDFVIEWER_QUORRA_COVERAGE=gpu` picks the other coverage lane. It never checks a ratchet: the
  two lanes deliberately do not draw the same pixels, and they do not allocate the same way either.
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

## 13. Half a page turn is `encode`, and it is CPU — **answered at quorra's ADR 0023, three days after this was written, and this heading said `open` for eleven rounds after that**

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

### 13.1 What came back — quorra's ADR 0023, and this heading was wrong for eleven rounds

Both halves, on 2026-08-11, three days after §13 was written and before this side had taken a
release that carried them. `Options::instrument_encode` subdivides encode into phases reported
through `Timings::phases`, so 3.86 µs a command can be attributed rather than guessed at, and
`Timings::execute_provenance` says which clock `execute` came from while `phases` names
`"target acquire"` and `"present"` — the two things the `elsewhere` row stood in for.

**What is worth keeping here is not the answer but why this section still said `open`, and it is
worse than a stale claim about somebody else's code.** The summary at the top of this document has
recorded the arrival since the four-hundred-and-thirty-eighth session — "§13's instrument arrived
in the same release and its other half was decided" — while the heading three screens down went on
saying `open`. **This document contradicted itself for eleven rounds**, which is precisely the
disease it names two paragraphs later about its own verdict table and precisely the ledger's, one
document over: a summary and a section that are two copies of one fact, kept by hand. This tree's
ADR 0283 says a claim about somebody else's code decays on their schedule; the correction here is
narrower and sharper — *the same document had already been told*.

**Neither half is used here yet.** `render-quorra` does not set `instrument_encode`, and
`doc/todo/45`'s attribution of a frame is where it would land.

## 14. §11.4.6's shape is not always the coverage, and `Compose` has no way to say so — **closed**: the operators arrived at `89d7dd77`, both refusals in front of them came off at `2c9bdd0`, and this side draws the four pages

Written 2026-08-08, at the end of this viewer's three-hundred-and-ninety-seventh session. **The ask
below was granted exactly** — `Compose::DestOut` and `Compose::Plus`, quorra's ADR 0025 — and what
is still open is on *this* side of the boundary rather than that one. [§14.1](#141-what-came-back--89d7dd77)
says what came back and what it leaves owed.

`quorra_scene::Compose` carries the argument for its own existence, and this side agrees with every
word of it: a general 2D vector library cannot be patched into clause 11, `Compose::Src` is
"Porter-Duff Source, **modulated by coverage**", and the scene that tests it has a diagonal edge on
purpose. That is exactly right for the half of §11.4.6 where an element's shape *is* its coverage.

**It is the wrong answer for the other half, and the clause says so in one sentence:**

> The existence of the knockout feature is the main reason for maintaining a separate shape value
> rather than only a single alpha that combines shape and opacity.

§11.6.4.2 gives an object's shape from its geometry alone; §11.6.4.3's soft mask and §11.6.4.4's
constant alpha are *opacity*. So a knockout element under a soft mask has shape 1 inside its path
and opacity ½, and a nested group has the shape of everything it marks whatever alpha it is painted
at — and in neither case is the alpha a rasteriser draws with the shape the clause weights the
backdrop by. `Compose::Src` reads the shape off that alpha, which is the assumption these elements
contradict.

### What this side did

`pdf_render::Command::Shaped` states the two apart: the object, and the same object with every
source of opacity removed, whose drawn alpha *is* §11.6.4.2's shape. §11.4.6's two stages then
come to one line per pixel in premultiplied form — `P' = (1 − f) × P + S` — which both of this
viewer's own backends draw as two marks: **Porter-Duff Destination-Out with the shape, then Plus
with the object**. `tiny-skia` has both as per-draw modes; Vello has neither as a parameter and
reaches them through two layers, and the two backends agree to the channel on a scene with a
diagonal edge inside the group.

Note the second operator, because it is the part that is easy to get wrong: **source-over in the
second stage is not the clause**. It weights the backdrop a second time, by `1 − shape × opacity`,
where §11.4.6 weights it by `1 − shape` alone. The two agree wherever the object is opaque or the
shape is 0 or 1, so the difference is an antialiased edge under a translucent object — and it is
**32 of 255** at a half-covered pixel under a half-opaque mark, which is what this side's own
fixture pins. (Our Vello backend still carries exactly that residue for the *coverage* case, where
it draws the element straight into the scene rather than in a layer; it is documented and bounded
there, and the two-mark form above is what removes it.)

### What the corpus says

Four documents. `knockout_smask.pdf`, `knockout_nested.pdf`, `knockout_nested_group_alpha.pdf` and
`knockout_inner_backdrop.pdf` all state a knockout group one of whose elements is a nested group or
carries a soft mask. **All four used to be counted as agreeing in §0's gate**, and that agreement
was two backends making the same wrong assumption: both read the shape off the alpha. They are now
`refused` by name, which is a gain rather than a regression — `957 pages compared: 916 agree, 36
differ, 5 refused, 17 not comparable`, against 920/36/1/17 before.

### The ask

**Two Porter-Duff operators on `Compose`: `DestOut` and `Plus`.** Both are safe where `Copy` is
not, and for the reason `Compose`'s own doc comment gives: at zero coverage `DestOut` leaves the
destination exactly and `Plus` adds nothing, so neither can erase outside a shape the way a
bounding-box composite does. With those two, a caller writes §11.4.6's second stage in two marks
and needs nothing else — no shape channel, no second raster.

An alternative shape, if it fits the scene vocabulary better: a per-element *shape* alongside the
paint, so that one mark carries both quantities and the library does the weighting. This side has
no preference; the two operators are simply the smaller change, and they are what our own two
backends turned out to need.

**What is not asked for**: nothing about the existing `Compose::Src`. It is right, this viewer
still emits it for every knockout element whose shape is its coverage — including every one of
§9.3.8's text objects, which is where the volume is — and the two-mark form is strictly more
expensive.

### 14.1 What came back — `89d7dd77`

**Both operators, by the names asked for, and they were the smaller change on that side too** —
the pipelines already existed, because quorra's knockout lane *is* those two marks:
`(Zero, OneMinusSrcAlpha)` through a shape-only fragment entry, then `(One, One)`. What the
release adds is a way for the scene vocabulary to ask for one of them alone.

Three things about them are worth having in writing:

- **`DestOut` weights by shape, not by the paint's alpha.** quorra's shape entry point returns
  coverage under the mark's clip and ignores the paint entirely, which is §11.6.4.2's shape — so a
  caller draws the object with every source of opacity removed and the weight is right. Which is
  exactly what `pdf_render::Command::Shaped`'s second member already is.
- **Two positions refuse a staged mark**, as `SceneError::StagedComposeUnsupported { compose, reason }`
  with `reason` one of `BlendNotNormal` or `InsideKnockoutGroup` — a mark carrying a blend mode is
  in §11.3.5's implicit one-element group, and a mark inside a knockout group is already
  erased-and-deposited per element. Neither is anything this side emits.
- **`Plus` alone saturates, and that obligation is the caller's.** Without the matching `DestOut`
  in front of it, it drives a premultiplied channel past its alpha, and one mark cannot tell a
  library whether the other is coming. `Compose::Plus`'s own documentation says so. It is the first
  item in that vocabulary whose correctness a scene cannot be refused for getting wrong, and the
  alternative this section offered — a per-element shape channel — is recorded there as the better
  design if `Plus` is ever wanted for something that is not §11.4.6's second stage.

quorra measured the pair against `P' = (1 − f) × P + S` on a wedge with a diagonal edge: worst
premultiplied deviation **0.77 of 255**, against **114.95** for the same object drawn source-over.
This side's own fixture pins the same phenomenon at 32; the size depends on the backdrop, and the
two numbers are the same fact.

**What is still open, and it is ours.** `render-quorra` still refuses `Command::Shaped` by name, so
`knockout_smask.pdf`, `knockout_nested.pdf`, `knockout_nested_group_alpha.pdf` and
`knockout_inner_backdrop.pdf` are four of the names on the corpus gate's refused list. Taking
them off is two marks per `Shaped` command, and it is `doc/todo/23`'s work rather than a request
from anybody. **The section stays here rather than being deleted**, because the derivation above is
what the two implementations checked against each other and it is the reason the operators have the
weighting they do.

### 14.2 The two marks were written out, and neither could be asked for — **both lifts came back at `2c9bdd0`**

Written 2026-08-11 at the end of this viewer's four-hundred-and-thirty-ninth session, which set
out to write the translation §14.1 called this side's work and found it is not writable at
`89d7dd77`. **The paragraph above that says "[n]either is anything this side emits" is wrong about
one of the two positions and silent about a second obstacle**, and both halves of that were this
side's to check before writing it down.

**The first: the only position this tree emits the pair from is the one the builder refuses.**
`pdf_render::Command::Shaped` carries its own guarantee, and it is the narrow one on purpose:

> This command appears only as a direct element of a [`Self::Group`] whose `knockout` is set.
> Outside one the shape is unused — §11.4.4's non-knockout formulas reach it only through
> `shape × opacity` — so a backend may draw `object` alone there.

That is not an accident of this tree's interpreter; it is §11.4.6 being the only clause that uses
shape and opacity apart. So the mark carrying `Compose::DestOut` is *by construction* inside a
knockout group, and `SceneBuilder::check_staged_compose` returns
`StagedComposeUnsupported { compose: DestOut, reason: InsideKnockoutGroup }`. Run rather than read:
`render-quorra/tests/headless_quorra.rs::quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over`
is that call and that error, and it is a test so that it fails the day you lift the restriction.

We understand why the restriction exists — a mark inside a knockout group is already
erased-and-deposited per element, so `DestOut` there is two erases with different weights and the
library cannot know a caller meant to replace the pair rather than add to it. **What a `Shaped`
element wants is exactly that replacement**: erase by the *shape*, then add the object, in place of
the group's own erase-by-coverage for this one element.

**The second obstacle is a missing parameter rather than a refusal, and it is the larger one.**
`SceneBuilder::group`, `stroke` and `image` take no `Compose` at all — only `fill` does. Three of
the four corpus pages behind this refusal, and this tree's own fixture, state a `Shaped` whose two
halves are **groups**, because §11.6.4.2 makes a nested group's shape the union of its elements':

| document | the object | the shape |
|---|---|---|
| `knockout_smask.pdf` | a fill under a soft mask | the same fill, opaque |
| `knockout_nested.pdf` | a knockout group of one half-opaque fill | a group of one opaque fill |
| `knockout_nested_group_alpha.pdf` | a group at alpha ½ | the same group at alpha 1 |
| `knockout_inner_backdrop.pdf` | a group of two `Multiply` fills | a group of two opaque fills |

So even with the first lifted, only `knockout_smask.pdf` becomes expressible.

**What is asked for, smallest first.** Either would be enough on its own to take one page; both are
needed for all four:

1. **Accept `DestOut` and `Plus` inside a knockout group** — the position §11.4.6 puts them in.
   The saturation obligation §14.1 records is unchanged and still ours; what a caller is asking for
   is "this element's erase is by the weight I hand you rather than by the coverage of what I
   draw".
2. **A compositing operator on a group mark**, so a whole group can be the source of one staged
   stage. `GroupSpec` is where a `compose` would sit beside `blend`, and the two stages are then
   the same group encoded twice, once shape-only under `DestOut` and once as itself under `Plus`.

The alternative §14 offered originally — **a per-element shape channel beside the paint** — would
answer both at once and is worth reconsidering now rather than then: it needs no operator on a
group, because a group already accumulates its elements' shapes, and it removes the saturation
obligation entirely because one mark then carries both quantities. This side has no preference
between the two designs and would take either; what it can now say that it could not in §14 is
that the two-operator form does **not** cover the population, and the population is knockout
groups whose elements are groups, which is what real Illustrator and InDesign artwork produces.

**Nothing here is a defect.** The operators do what their documentation says, and the two refused
positions are documented on the builder. What went wrong is on this side: §14 derived the pair from
a fixture whose two halves were groups and then asked for an operator only `fill` could carry, and
§14.1 asserted a property of this tree's display lists — "[n]either is anything this side emits" —
without building one. `doc/todo/23` carries the four pages, and the refusal message in
`render-quorra` now names both obstacles instead of claiming the work is unwritten here.

### 14.3 What came back — `6f777e8` and `2c9bdd0`, and what this side did with it

Both asks, in the order they were written, in two commits and two ADRs.

- **quorra's ADR 0032** deletes `StagedComposeReason::InsideKnockoutGroup` rather than leaving it
  unreachable, on the reading that §11.4.6's per-element rule *is* the staged pair weighted by that
  element's own source shape — so a staged element **replaces** the group's erase for itself rather
  than adding a second one. `quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over` was
  written "so that it fails the day you lift the restriction" and it did: it stopped compiling at
  the bump, which is the notification working as designed. What stands in its place is
  `quorra_states_what_it_will_not_stage`, holding the two constraints that survived.
- **quorra's ADR 0033** puts `compose` on `GroupSpec`, beside `blend`, which is where §14.2 said it
  would sit. The erase half of a group is the same content drawn opaque — the only way a group's
  shape reaches a raster, since Table 140's group alpha is not what a premultiplied texture holds.

**What this side wrote is smaller than the ask was**, and the reason is worth recording: a
`Compose` can be stated on a fill and on a group and on nothing else, while a `Shaped` element's
object may be a stroke, an image, or a fill whose paint takes the image door (a sampled shading).
So every half that is not already a group is drawn *inside* a group of one element, which is the
same arithmetic — an isolated group at alpha 1 under no mask holds exactly the element's own
premultiplied colour — and one uniform route instead of a per-mark route that would have been
correct for some paints and silently wrong for the rest. It costs one buffer per staged mark, on
four pages of the corpus.

**Measured here, on the corpus, at `2c9bdd0`**: the four pages come off the refused list and every
one of them **agrees with the CPU oracle** — two independent transcriptions of
`P' = (1 − f) × P + S`, one per backend, meeting on artwork nobody invented for a fixture. The
default lane at the page's own scale goes 915 agree / 37 differ / 5 refused to **919 / 37 / 1**,
and the one refusal left is `bug1721218_reduced.pdf`, which is a texture capacity rather than a
clause. `test-scenes`' own `knockout_stated_shape` is drawn rather than refused, and the pixel it
is checked at is the clause's arithmetic to within the unorm rounding your ADR 0033 states: the red
under the element is gone whole, and what stands in its place is `½ × blue + ½ × green` at 127 or
128 per channel.

**One constraint of yours costs this side nothing today and is worth naming anyway.** A staged
half may not carry a blend mode, because §11.3.5 composites such a mark through an implicit
one-element group. `pdf-model` already drops the blend from a shape half — §11.4.6 leaves a
knockout element nothing to blend against — but an *object* half that blends is a display list this
tree can build, and it would now be refused by name rather than drawn. No corpus page states one,
and the honest reason this is not an ask is that nobody has a document that needs it.

## 15. A gradient the scene will paint across a page, to keep a few dozen pixels of it — **open, and it is this side's finding rather than a defect of yours**

Reported for completeness rather than as a bug: **the same page costs your backend the same thing,
and the fix this side took lives in a crate you can call.**

`bug1721218_reduced.pdf` — the one page in our corpus your device refuses on coverage, so you have
seen it — draws an illustration as **3490 `sh` operators**. ISO 32000-2 §8.7.4.2's Table 76 bounds
that operator by the current clipping path and by nothing else:

> Paint the shape and colour shading described by a shading dictionary, subject to the current
> clipping path.

so a display list has nowhere to put the geometry except the page rectangle, and each of those 3490
rectangles arrives under a clip admitting about **24 pixels**. Our CPU rasteriser shaded 10.4 M
pixels a render to keep 85 608 — a ratio of 122 — and simply cropping the rectangle to the region
the clip mask can mark took twenty renders of that page from **38.45 G instructions to 20.03 G**,
byte-identical output. ADR 0236.

**Nothing about the display list changed**, so your side is unaffected and our cross-backend gate
is unmoved at 916/36/5/17. But the scene you are handed still carries 3490 page-sized rectangles,
and `encode` is already 45% of a page turn on your side (§13 above). Whether that costs you
anything depends on how your coverage lane bounds a clipped fill, which we cannot see from here —
if it already intersects a fill's extent with its clip's before rasterising, there is nothing to do
and this section can be closed as *already handled*.

If it does not, the geometry is `pdf_render::cropped_rectangle`, which is in the crate both our
backends share precisely so that either can call it. What it needs from a caller is three
guarantees, and they are on the function: the mask is zero outside the rectangle handed in, that
rectangle carries a pixel of margin, and it is at least two device pixels across. It declines
curves (a clipped cubic is re-parameterised and its coverage is not the same — our ADR 0139
measured 2480–2744 differing bytes) and declines a transform that would not keep a rectangle one.

**What would be more useful than us calling it for you**: knowing whether your scene vocabulary can
express "this fill is bounded by this rectangle" at all, since a clip in a scene is usually a layer
rather than a bound. If it can, a host could state the bound instead of shrinking the geometry, and
that is the better shape — it keeps the producer's rectangle in the scene and lets the rasteriser
decide what to do with it.

## 16. A group's buffer always starts transparent, and §11.4.4 defines the other one — **answered at `89d7dd77`, and verified here**

Written 2026-08-08, at the end of this viewer's four-hundredth session. **Answered in full**, and
the reply is `doc/QUORRA_NON_ISOLATED_GROUPS.md` — both halves of the ask, the seeded buffer *and*
the composite back, in quorra's ADR 0019. What came back and what this side did with it is
[§16.1](#161-what-came-back--89d7dd77) below; the ask is kept above it because the derivation is
what the two sides checked against each other.

`quorra_scene::GroupSpec` opens a layer, draws into it and composites the result once. That is
ISO 32000-2 §11.4.5's **isolated** group exactly, and it is what every rasterising library
offers, ours included:

> An isolated group is one whose elements shall be composited onto a fully transparent initial
> backdrop rather than onto the group's backdrop.

§11.4.4 defines the other initial backdrop: for a **non-isolated** group the elements composite
onto the group's own backdrop, and the clause then removes that backdrop's contribution from the
result so it is counted once. The difference can only be seen where an element *blends* — with
every element painting Normal the backdrop is composited in and removed again exactly (§11.4.4
NOTE 3, and §11.6.7's NOTE 1 says the same for a pattern cell) — which is why this side emits
nothing new for the ordinary case and why it never came up before.

### What this side did, and the part worth having

The clause's own advice for the non-isolated case is NOTE 4: keep *two* sets of accumulated
variables, Table 140's group alpha apart from the composite alpha, because the removal divides
by the first. A premultiplied raster has one set, and this project had written down twice that
one raster therefore cannot do it.

**It can, and the reason is that the quantity the removal divides out is multiplied straight
back in.** The group's object alpha *is* Table 140's group alpha, so when §11.3.3 composites the
group's result onto the same backdrop under the **Normal** blend function, the two cancel. With
`B` the backdrop, `E(B)` the elements composited onto it, both premultiplied, and `w` the group's
constant alpha times its soft mask at the pixel:

```text
result = (1 − w) × B + w × E(B)
```

exact for every backdrop alpha and every blend mode *inside* the group. `w = 1` reduces it to
`E(B)`, which is NOTE 5's flattening. Checked against transcriptions of §11.4.4's recurrence and
§11.3.3's formula over 200 000 random inputs: worst deviation 5.6 × 10⁻¹⁶. With a non-Normal
blend mode at the `Do` it is **0.601 of full scale** wrong, so that case is excluded and reported.

So the whole construction is: **a group buffer that starts as a copy of what is under it**, and
one interpolation to bring it back. Our CPU backend seeds a `tiny-skia` pixmap from the target
and writes the line above in a single pass over the band — one pass rather than Destination-Out
plus Plus, because two draws round twice and at `w = ½` over an opaque page they leave alpha 254
of 255.

### What the corpus says

**Four documents, and all four used to be counted as agreeing in section 0's gate**:
`bug1755507.pdf`, `issue12798_page1_reduced.pdf`, `issue13520.pdf` and `issue18032.pdf`. Every
one of them is Illustrator or InDesign artwork — nested groups under `/Luminosity` soft masks
with `Screen` and `Multiply` elements — which is what a non-isolated group with a blending
element looks like in the wild.

That agreement was two backends substituting the same wrong initial backdrop. They are now
`refused` by name: `957 pages compared: 912 agree, 36 differ, 9 refused, 17 not comparable`,
against 916/36/5/17 before. As in section 14, a refusal that replaces an agreement about a wrong
picture is a gain rather than a regression, and the list is held to equality in both directions
so it cannot quietly grow.

### The ask

**One flag on `GroupSpec`: whether the group's buffer begins transparent or as a copy of what is
under it.** `GroupSpec` carries `alpha`, `blend`, `clip`, `knockout` and `mask`, which is
§11.4.5's group exactly; this is Table 145's remaining entry, `/I`, and it is the only thing the
vocabulary is missing.

**And the composite back is an operator you already have and already describe correctly.**
`Compose::Src` is documented as "Porter-Duff Source, **modulated by coverage**: an element with
40% coverage replaces 40% of what was there and leaves the rest" — which *is*
`(1 − w) × destination + w × source`, the line above, with `w` the coverage. So if a group's
composite can be `Src` rather than `SrcOver` when the flag is set, and the group's alpha and mask
supply the coverage, nothing else is needed at all: a seeded buffer plus that operator is the
whole of §11.4.4.

Whether that is one change or two depends on where the fine rasteriser can start a layer from the
destination, which we cannot see from here. If it can only be the operator and not the buffer,
the answer is a refusal we can go on printing rather than a workaround — the two halves are not
separable, since the interpolation is between the backdrop and the *elements composited onto it*.

**What is not asked for**: anything about the isolated group, or about `Compose::Src` itself.
`GroupSpec` is right for the isolated case, this viewer emits it for every group whose elements
paint Normal — which is almost all of them, because that is the case the clause itself says needs
nothing — and a seeded buffer is strictly more expensive. The flag's default is the behaviour
that exists.

### 16.1 What came back — `89d7dd77`

**Exactly the flag, and both halves of it.** `GroupSpec::isolated` is Table 145's `/I`, `true` is
the behaviour that existed, and the struct stayed exhaustive on purpose so the new entry could not
compile silently. The buffer *can* begin from the destination — a scissored blit into the layer
pair the group already had, so there is no new allocation and no change to the frame budget — and
the composite back is the interpolation this section derived. quorra re-derived it from the
standard rather than adopting it, and the two transcriptions agree on the number: **5.6 × 10⁻¹⁶**
worst deviation over 200 000 configurations, from `quorra-gpu/tests/non_isolated_groups.rs` and
from this side's ADR 0237 independently.

**The conditions are the same three, which is the part worth noticing.** quorra accepts a
non-isolated group only where the group's own blend is Normal, it is not a knockout group, and no
enclosing group is one — the set `pdf-model` restricts itself to, derived twice from §11.4.4 and
arrived at from two directions. Anything else is
`SceneError::NonIsolatedGroupUnsupported { reason }` at `SceneBuilder::group`, with `reason` one
of `GroupBlendNotNormal`, `KnockoutGroup`, `InsideKnockoutGroup` — a typed builder refusal, which
is what this side wanted rather than a silently approximated group.

**What it cost this side**: eleven lines in `render-quorra/src/scene.rs`, all of them deletion of
the refusal plus `isolated: *isolated`. The three conditions are **not** re-checked here, and that
is the decision ADR 0274 records: a copy of them would be a second reading of §11.4.4 free to
drift from the one that decides the picture.

**Measured, at this document's own scale.** The gate went `911 agree, 35 differ, 11 refused, 17
not comparable` → **`914 agree, 35 differ, 8 refused, 17 not comparable`**. Three of the four
documents section 16 named — `bug1755507.pdf`, `issue13520.pdf` and `issue18032.pdf` — leave the
refused list and agree with the CPU oracle; the `differ` list is identical page for page, which is
the same result the quorra side measured on its own copy of this tree. The pages were *looked at*
and not only counted: `bug1755507.pdf`'s rounded panel keeps its drop shadow, `issue13520.pdf`'s
lozenge its rim, and the page inks agree to 0.07 of 255.

**The fourth is the finding.** `issue12798_page1_reduced.pdf` stays refused and now says *"a page
composited in a four-component blending colour space (§11.4.7)"* — the §11.4.4 refusal had been
standing in front of section 17's, and this page was never section 16's alone. Both sides
predicted it and neither had to guess, because the refusals name their clause.

## 17. Four components need two rasters, and one `Scene` renders one — **closed**: answered at `89d7dd77` (it was already true), taken up here in session 439, with one more mark on its fixture in session 441 (§17.3)

ISO 32000-2 §11.4.7 puts a colour space under the whole page:

> All page-level compositing shall be done in the default blending colour space of the page, and
> the entire result shall then, if the colour spaces are not equivalent, be converted to the
> native colour space of the output device before being composited with the context-dependent
> backdrop.

3.5% of the PDFs on the web state such a space, all of them `DeviceCMYK` or a four-component ICC
profile — measured over a 1944-document sample of Common Crawl, where it is the single largest
correctness gap this viewer has against real files.

**Nothing about the scene vocabulary is wrong for it, and that is why this section is short.**
§11.3.4 applies the compositing formula per component, so four components are three plus one: the
page is interpreted twice, once carrying the additive complements of cyan, magenta and yellow and
once carrying the complement of black, and the two rasters are put back together afterwards by a
per-pixel conversion that has nothing to do with rasterisation. Every separable blend mode comes
along unchanged, because it too is per component, and the rasters hold the complements so nothing
has to be complemented around a blend function.

What this viewer's backend cannot do is *get both rasters*. `Rasterizer::rasterize` hands back one
`Raster` per call and the two passes are two display lists over one page, so the CPU backend simply
runs its own pipeline twice into two pixmaps. Through quorra the same thing needs one of:

- a way to render the same viewport twice and read both back within one frame, or
- two calls whose device state, caches and resources are not thrown away between them.

Either is enough; neither needs a new `Compose`, `GroupSpec` or paint. If the second is already
true — if calling `render(..., Target::Readback)` twice against one device is simply supported and
cheap — then say so and this section closes with no change at all, because the refusal is currently
placed on the display list rather than on anything quorra said.

**What the refusal costs today**: `personwithdog.pdf` is the corpus's one such page and it moved
from *reported* to *refused* — this viewer used to draw it in the wrong space and now declines to
draw it here at all, which is a gain of the same kind as sections 14 and 16. On real files it is
one page in thirty rather than one in a thousand, so it is the section most likely to matter to
somebody who is not us.

**And it costs more since the four-hundred-and-twenty-seventh session, which is worth saying
because the number moved for a reason rather than by drift.** That session gave §11.7.2 its
conversion *into* the blending space, so a page whose colours are not already ink is drawn in ink
rather than reported: of a 1944-document sample of the web, 56 pages now take the pair of rasters
where 7 did. In the corpus the refusal list gains `bug1365930.pdf`, which is the sharpest case for
this section — that page reports **nothing** and never did, because nothing on it composites, so
quorra agreed with us about it right up until we started drawing it the way §11.4.7 states. It is
the first page in this file whose refusal replaces an *agreement about a correct picture* rather
than one about a wrong one, and the only thing that gets it back is the readback above.

### 17.1 What came back — `89d7dd77`

**The second of the two was already true, and this section's own offer is the one that closes it.**
It asked: *"If the second is already true — if calling `render(..., Target::Readback)` twice against
one device is simply supported and cheap — then say so."* It is, and `quorra-gpu/tests/two_rasters.rs`
now holds it so that it stays true rather than being true by accident:

- both rasters come back whole, one `Raster` per call;
- resources are **device-scoped**, so the outlines upload once and both passes reference the same
  `OutlineId`s — the second pass's `bytes_uploaded` is strictly smaller;
- the glyph key is `(outline, linear part, phase, rule)` and **colour is not in it**, so the pass
  carrying the complement of black hits every tile the C-M-Y pass rasterised: `encode: geometry`
  measured at 1.772 ms on pass one and **0.000 ms** on pass two;
- neither pass changes what the other draws.

Two caveats stated rather than buried: a frame whose tiles overflow the atlas can leave the next
pass cold (quorra's ADR 0024 narrowed when that happens), and each pass pays its own readback,
about 1.3 ms at page size — irreducible when both rasters are wanted.

**So nothing on quorra's side blocks `personwithdog.pdf` or `bug1365930.pdf`, and the refusal that
remains is this side's own.** It sits in `QuorraRasterizer::rasterize`, on `list.blending()`, and
what it now owes is not a request but work: two `Rasterizer::rasterize` calls against one
`QuorraRasterizer` and `pdf_render::blending`'s recombination, which the CPU backend already does.
`doc/todo/23` carries it, and it is the first item this file has handed back to itself rather than
outward.

### 17.2 What this side did — session 439, and the section closes

`QuorraRasterizer::rasterize` renders `list` and, where `list.blending()` is `Some`, renders
`list.black()` against the same device before `pdf_render::blending::resolve` puts the two together
— premultiplied, ahead of the medium, which is where §11.4.7 puts the conversion. The device, the
resource caches and the per-frame releases are the same on both passes because both go through one
private `render`, so nothing about drawing a list can depend on which of the two it is.

Everything §17.1 promised held:

- both rasters came back whole, one `Raster` per call;
- the second pass needed no special handling of any kind — no reset, no re-upload, no warm-up;
- the three corpus pages this was worth **agree with the CPU oracle**: `personwithdog.pdf` at
  0.0288 mean and page inks 20.3953 against 20.3659, `bug1365930.pdf` at 0.0093 and 0.8637 against
  0.8634, `issue12798_page1_reduced.pdf` at 0.0760 and 16.9057 against 16.9117. The gate's refused
  list falls from eight names to five.

Two numbers this side owes back, because §17 asked for a cost and got one:

- the corpus gate's whole-run rasterisation time through quorra went **8.09 s → 8.51 s** over 957
  pages, which is three pages drawn twice out of 952 drawn once, plus the recombination pass;
- the median page ratio came back **2.37×** and **2.35×** on two runs against 2.46×, which is
  inside the run-to-run band this document already records for that figure and says nothing
  either way.

A fixture rather than only a corpus: `test_scenes::four_component_page` is a page of four ink marks
whose expected pixels come from the clause — half of registration black over paper is ½ of each of
four components, which the ink cube interpolates to (76, 66, 64) against the (128, 128, 128) that
averaging two *converted* colours gives, and a mark of black ink alone is white in the chromatic
raster and (35, 31, 32) only if the second pass happened. Both backends draw it to within a level.

**The section is closed.** It asked one question, the answer was yes, and the work it named is
done.

### 17.3 One more mark on the same fixture, and it is a report rather than an ask — session 441

`four_component_page` gained a **fifth mark** in session 441: an opaque `Hue` over an opaque
backdrop, both stating four ink components. It is here because ISO 32000-2 §11.3.5.3 gives the
black component of a subtractive blending colour space a rule that is not the rule it gives the
other three —

> For the K component, the result shall be the K component of Cb for the Hue , Saturation , and
> Color blend modes; it shall be the K component of Cs for the Luminosity blend mode.

— and this side had been *reporting* such a page rather than drawing it, on the belief that the
rule needed a blend function neither raster carries. **It does not.** The black raster is neutral
in all three of its channels, and on a neutral pair the clause's own `Sat`, `SetSat`, `SetLum` and
`Lum` return the backdrop for the first three modes and the source for `Luminosity` — the rule,
term for term, at a worst gap of 1.19 × 10⁻⁷ over 200 000 pairs.

**Nothing is asked of quorra for it**, and that is the point of writing it down: the identity means
a renderer that implements §11.3.5.3 for three components implements the black component's rule
with it, and `quorra_scene::BlendMode::Hue` on the second raster is the whole of the translation.
`cpu_and_quorra_agree_on_a_four_component_page` now asserts the mark at **(12, 88, 90)**, where the
wrong K would be (19, 138, 141), and it passes — so quorra's shader has the identity as well as
this side's arithmetic does.

One thing was *considered* and would have been an ask, and is recorded because it was measured
rather than assumed: stating the rule explicitly instead of deriving it means a blend function
`B(Cb, Cs) = Cb`, which under §11.3.3 collapses to Porter-Duff **Destination-Over** exactly.
`quorra_scene::Compose` has `SrcOver`, `Src`, `DestOut` and `Plus` and no member for that operator,
and no pair of them composes to it — the weight Destination-Over puts on the source is the
*destination's* alpha, which no source-side operator supplies. That route was dropped, so this is
**not** a request for `Compose::DestOver`; it is a note that the one place this side would have
wanted it turned out not to need it. ADR 0277.

## 18. A clip chain's coverages multiply, and §10.7.4 states an intersection of sets — **answered at `0ddaa40`: quorra composes a chain by `min` too**

Written 2026-08-11, at the end of this viewer's four-hundred-and-forty-fourth session. This side
changed its own composition in the same session and is telling you so, because our cross-backend
gate is the instrument that would otherwise report the difference as yours.

### What the standard says

ISO 32000-2 §10.7.4 gives clipping a paragraph of its own, and it is about *sets* rather than about
coverage:

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the
> set of pixels defined by the clipping region with the set of pixels for the region to be painted.

§8.5.4 says the same thing from the transparent imaging model's side, and it is the sharper of the
two because it says what a clip does to a value:

> The effective shape is the intersection of the object's intrinsic shape with the clipping path;
> the source shape value shall be 0.0 outside this intersection.

A clip zeroes what is outside it and is silent about what is inside it. Neither clause makes a
clip's own boundary a quantity that multiplies; §11.6.5's soft mask is where a genuine product
lives, and it is a different mechanism with a different clause.

### What both of us were doing

An anti-aliased clip boundary carries a fraction, and a chain composed by multiplication raises
that fraction to a power. One page, one fill of the whole page, under **n** `W n` clips of the same
rectangle whose left edge lands at device 113.386 at 8×; coverage of the boundary column:

```text
  coincident boundaries      1       2       3       4       5       6
  coverage               0.5020  0.2510  0.1255  0.0627  0.0314  0.0157
```

Each rung is the one above it halved. `issue21346.pdf` in our corpus states one device rectangle
six times over — a `W n`, three `/BBox` clips under §8.10.1 step c), the mark's own path and a
§11.6.5 mask group's — and painted its edge at **0.041** of the mark where the geometry is 0.827 of
it and the clause is 1.000. `poppler` and `ghostscript` give 1.000, `mupdf` 0.755.

### What this side did about it

`min`, on the chain only. It is exact where two boundaries coincide or nest — restating a clip then
changes nothing, which is what a set intersection does — and where two unrelated boundaries share a
pixel it is never *below* the product, so it is never further from the clause. The ladder above is
flat at 0.5020 for every *n* now, and the witness page went 0.041 to 0.163. Our ADR 0280 has the
argument and the numbers.

Two things it did **not** buy, both stated so that you can price the same change:

- the exact answer for two *unrelated* boundaries in one pixel is the area of the intersection of
  the paths, rasterised once, which is a conflation-free rasteriser and is not what this is;
- it cost **+0.19%** of the rasteriser's instructions on an ordinary page of text and **−8.75%** on
  the corpus's heaviest clip page (`bug1721218_reduced.pdf`, 3554 clips), the second because the
  scratch mask is now allocated once per chain rather than once per link.

### The ask, and what it is not

We hand you a chain of paths — `SceneBuilder`'s clip chain — and your device composes it, so this
is one rule inside your rasteriser that this project cannot reach. **The ask is to say what that
rule is**, and then whether §10.7.4's paragraph changes it. Three shapes would each close this:

1. your coverage lane already takes an intersection rather than a product, in which case this
   section closes as *already handled* and we would like to know it;
2. it multiplies and you agree the clause asks for the other thing, in which case a chain of
   coincident clips is a one-line change in the composition and the ladder above is the test;
3. it multiplies deliberately, for a reason a display list cannot see — in which case we would
   rather have the reason than the change, because it belongs in our own ledger row beside our
   departure.

**It is not a defect report and nothing of yours is refused over it.** Our cross-backend gate is
`957 pages compared: 917 agree, 35 differ, 5 refused, 17 not comparable` before and after this
change, and **not one per-page line moved by a digit**: the 22 pages whose CPU raster moved are not
among the 35 the two backends already differ on, and elsewhere the movement stays under the
agreement bound. So the two backends now compose clips by two different rules and no gate can see
it yet — which is exactly why it is written down rather than left for a future round to rediscover
as a regression in whichever side is measured second.

### 18.1 What came back — `0ddaa40`, and it is the first of the three shapes

The second shape, and then the first: quorra composed a chain by multiplying and now takes `min`,
from §8.5.4's own sentence that the graphics state holds *one* clipping path set to the
intersection of the current path and the new one (its ADR 0030). So the two backends compose a
chain by the same rule again, and the "no gate can see it" sentence above is what that costs: this
side changed in the four-hundred-and-forty-fourth session, quorra in the release taken in the
four-hundred-and-fifty-sixth, and in between nothing in either tree could have told anybody the two
disagreed.

**Where a clip meets the *mark*, quorra still multiplies**, deliberately and with its reasoning
recorded — two unrelated boundaries sharing a pixel are the common case, and a product is the
estimator that assumes it. That is exactly this tree's own remaining departure, which is still
`tiny-skia`'s and is still §10.7.4's row: the two projects are in the same place, by two
independent readings, with the same open question. Nothing is asked for here.


---

## 19. Not one page emits a `Command::Rect`, and that is ours — **open, and it is your finding about our side**

`04c0d23`'s `doc/corpus-profile.md` walked the 995 first pages this gate builds and counted what is
on them. One row of that table is a fact about **our** translation rather than about a page:

> **Not one page emits a `Command::Rect`.** The lane is real, reachable and documented, and every
> rectangle a document draws arrives as a `Fill` whose outline happens to be one.

Checked here and confirmed: `quorra_scene::Rect` is named in exactly one place in
`crates/render-quorra/src/`, `present.rs`'s blit of the finished page, and nowhere on the path any
page's content takes. So the lane your flagship fixture prices is one we never take, and the reason is
on our side of the boundary: `pdf_render::Command::Fill` carries an outline, and `render-quorra`
hands it over as an outline without asking whether it is four axis-aligned edges.

**What we owe you is a decision rather than an answer, and we do not have it yet.** Two things have
to be true before this is worth doing and neither is measured:

1. that recognising a rectangular outline is cheap relative to what the `Rect` lane saves — which
   depends on your side, since the saving is inside `encode`;
2. that it is *exactly* the same mark. §8.5.2.1's `re` appends four lines and a close, and a fill of
   that path under a transform that is not axis-aligned is not a rectangle at all; a lane that
   assumed otherwise would be the kind of shortcut `CLAUDE.md` forbids taking silently.

**What would settle it from your end** is one number: what a `Command::Rect` costs against a `Fill`
of the same four-edge outline, at the sizes your profile says a page actually contains — median 12
commands, p99 4320. If the answer is "nothing worth the recognition pass", this section closes as
*already handled* the way §15's may, and neither side writes any code.

**And the profile itself is worth saying thank you for out loud**, because it did something this
document has been asking for in three sections without naming it: it measured *our* corpus with
*your* counters and reported a shape neither side had. The median page being twelve commands and glyph
reuse being 1.33 rather than 55 are not facts about quorra — they are facts about what this viewer
hands quorra, and every fixture on both sides was built against a page nobody had counted.

## 20. The second lane, measured against the corpus for the first time — **and `74c4994d` takes 24 refusals off it**

**This section is the answer to a question this document has never asked, because it could not.**
Every number above §20 was taken on `Coverage::Cpu`: this gate has always run quorra's default
lane, and the fixtures were the only thing that had ever judged the other one. That is the exact
shape of this project's own trap 12b — fourteen small scenes said a backend was fine and the first
real page at a window's size came back blank — and it stood while `viewer-ui` switched a *person*
onto the GPU lane at ten times magnification.

So the gate now takes `PDFVIEWER_QUORRA_COVERAGE=cpu|gpu`, and this is the first corpus-scale
statement about the lane. AMD Radeon 890M, RADV, Vulkan, release-grade build, the whole 974.

### 20.1 What the release did, at the page's own scale

| `Coverage::Gpu`, scale 1 | agree | differ | refused |
|---|---:|---:|---:|
| `c1f6e2f4` | 904 | 44 | **9** |
| `74c4994d` | 908 | 44 | **5** |

The four are `bug1703683_page2_reduced.pdf`, `issue12810.pdf`, `issue1905.pdf` and `issue9418.pdf`
— refused at the old pin for 605, 311, 1324 and 481 megabytes of winding target against a 256 MiB
budget, and **each now agrees with the CPU oracle**. `6ef954e`'s pane is what did it. You named two
of the four; the other two are this corpus's.

The differing list is identical at both pins, so nothing left agreement to pay for them. Rasterisation
went 4.68 s → 6.13 s and **that is the four pages joining the timed set**, not a slowdown: measured
alone, twice, they are 1.26–1.27 s, leaving 4.86 s against 4.68 beside an oracle column that moved
2.01 → 2.05 on the same subtraction.

### 20.2 And at four times that scale, which is nearer where the lane is chosen

| `Coverage::Gpu`, scale 4 | agree | differ | refused | median ratio |
|---|---:|---:|---:|---:|
| `c1f6e2f4` | 900 | 16 | **36** | 2.87× |
| `74c4994d` | 920 | 20 | **12** | 2.55× |

**Twenty-four pages the lane could not draw now draw, twenty of them agreeing with the oracle**, and
not one page went the other way. This is the release's own claim reaching a population it was not
measured on: at 1× it closes four holes, at 4× twenty-four.

### 20.3 The first frame, on a real page: no effect, and that is consistent with your own table

Page 7 of ISO 32000-2, GPU lane, three samples per cell, milliseconds:

| | frame 1 | frame 10 |
|---|---|---|
| `c1f6e2f4`, 596×842 | 21.8 / 20.6 / 15.0 | 4.1 / 7.0 / 4.1 |
| `74c4994d`, 596×842 | 21.0 / 25.8 / 17.7 | 7.8 / 4.6 / 4.0 |
| `c1f6e2f4`, 2382×3368 | 44.5 / 55.0 / 44.4 | 36.2 / 25.6 / 29.2 |
| `74c4994d`, 2382×3368 | 51.8 / 52.4 / 51.2 | 26.6 / 25.7 / 30.5 |

Every band overlaps every other. A page of dense text places each outline many times, which is the
case `74c4994`'s message says had to stay untouched — so this is the census behaving as documented,
and it is recorded here so that nobody re-measures it expecting the 2.5–3× and reads a null as a
regression.

### 20.4 Two things back, and neither is a defect

1. **`transparency_group.pdf`'s worst tile is 31.7 of 255 at 4× on this lane**, against the 1.5 to
   12.2 your own attribution reports and against `tests/coverage_lanes.rs`'s stated bounds. It is a
   page that was *refused* before this release, so nothing regressed and there is no earlier value
   to compare with — which is why this is a question rather than a finding. The other three that
   moved into *differs* are `bug1743245` (13.7), `issue12295` (16.3) and `issue19971` (14.3). If
   sixteen samples is the whole explanation, the arithmetic should cap nearer 16 of 255 than 32, and
   we would rather ask than assume.
2. **Twelve refusals remain at 4×, and eight of them are yours**: `22060_A1_01_Plans.pdf` over the
   resource budget at 548 MB, `Test-plusminus.pdf`, `issue14297.pdf`, `issue16287.pdf` and
   `issue269_2.pdf` over the 256 MiB frame budget by between 4% and 20%, and
   `bug1703683_page2_reduced.pdf`, `bug1721218_reduced.pdf` and `issue1905.pdf` past the
   16384×16384 scratch image this adapter allows. Two of those last three are the interesting ones:
   `bug1703683_page2_reduced.pdf` and `issue1905.pdf` are pages your pane took off the refused list
   at 1×, and at 4× they come back as a *texture capacity* refusal instead — a different ceiling,
   reached by the same page. (`bug1721218_reduced.pdf` has been that refusal at every scale and is
   this corpus's most pathological page by some margin.) Whether the pane can be cut against that
   ceiling the way it is now cut
   against the byte budget is your question; the four remaining refusals are ours, and they are
   §14.2's knockout ask, unchanged.


## 9.1 What is left of §9 is an API question, and it is ours to ask

`a35dc70` takes 2.43 ms off the first frame and says what the rest is: about 6 ms inside
`run_frame` that **scales with the target** — page-sized textures and the driver's first touch of a
heap that size — which a warm-up thread cannot allocate before the viewport exists. Your ADR 0031
records that as the caller's contract rather than taking it, which is the right call and hands the
question back.

**We are not asking for `Device::warm_for(extent)` yet, and the reason is on this side.** A host
that could call it would have to know its viewport before its first frame, and `viewer-ui` learns
that from `Resized` — after the window exists, which is after the point where the saving would be
banked. The two honest shapes are:

1. **A size hint at construction**, where a host that already knows its window (a fixed-size
   viewer, a print path, a headless renderer at a stated target) can pay the allocation off the
   critical path, and one that does not passes nothing and loses nothing.
2. **Nothing at all**, and the number is stated rather than hidden — which is what `doc/todo/42`
   now says, because `CLAUDE.md` puts page one on the device by choice and therefore owes the cost
   a measurement rather than an excuse.

What would settle it here is a launch measured on the *real* adapter through a real window, and
this machine cannot take it: the agent's account has no X authority cookie for the owner's display,
so ADR 0179's timeline is `lavapipe` under `Xvfb` and the GPU half of the number is the owner's to
measure. Until somebody runs that, asking you for an API to save 6 ms would be asking for a
mechanism nobody has priced end to end.

**Asked again in your upgrade note's §7, and the answer is unchanged, which is itself the answer
you wanted.** Take shape 2: state the 6 ms rather than build a mechanism for it. Nothing here is
waiting on `Device::warm_for` or on a hint at construction — if it existed today, `viewer-ui` could
not call it, for the reason above — so it should be built when a host that knows its viewport
before its first frame asks for it, and not for us. If that changes it will change because
somebody measured a launch on the owner's own adapter, which is the one number this whole section
is short of.

### 9.2 You built it anyway, and we still cannot call it — with a second reason that is yours

`Device::warm_for` is in `a7babab` (your ADR 0035). **The answer above is unchanged and this side
re-measured rather than re-read it**: the first frame here is still the first frame, and §22 has
what it costs on this machine at both scales. `viewer-ui` still learns its viewport from `Resized`.

What the release adds is a second reason, and it is one your own ADR 0039 states plainly: the hint
warms a **target-sized** layer, and after 0036 to 0039 that is the right size only for a root that
fills its target — about a quarter of layered frames, which are themselves about 8 % of frames. So
on the population that would call it, the hint is the wrong size more often than not. Nothing about
that is a complaint: 0035 is measured, correct, and free to call, and 0039 says so about itself,
which is the part worth acknowledging. It means the **10.3 ms should not be quoted as a general
number**, and this document is not quoting it.

The one thing that would change the answer here has not changed either: a host in this tree that
knows its size before its first frame — a print path, `viewer-confined`'s worker, a fixed-size
viewer. When one exists, `warm_for` is one line and this section closes.

**2026-08-14, at `87898c69`: the number this section stopped quoting was retracted at the source,
and the mechanism was found.** Quorra's ADR 0040 could not reproduce 24.7 → 10.3 ms in five
configurations — a 46 MB texture is created in sixty *micro*seconds on RADV, since the memory
commits when the GPU first touches it — and what a layered first frame was actually paying was
two pipeline compiles (`Composite`, `Blit`) inside itself. The warm set compiles both now, and
its ADR 0043 extends the same fix to a presenting host's negotiated surface format, which was
§22.7's other prediction. The answer to `warm_for` is still *no*, still free to call, and its
table may no longer be quoted — quorra's ADR 0040 says so about itself.

## 20.5 The remaining refusals are unmoved, and that is expected — **and §22 moved them**

`ab219d0` fixes coverage rather than capacity, so the twelve refusals §20.4 listed at 4× are the
same twelve: `22060_A1_01_Plans.pdf` over the resource budget, four pages over the 256 MiB frame
budget by 4% to 20%, three past the 16384×16384 scratch image this adapter allows, and four that
are this side's §11.4.6 knockout hole (§14.2). Nothing in this release touched either bound and
nothing here suggests it should have — the question in §20.4 stands as it was written.

**Read this section as history**: of the twelve, the four knockout pages went at `2c9bdd0` (§14.3),
the four byte-budget ones at `a7babab` (§22.3), and the four that are left are the resource budget
and the three sheet-capacity refusals. §20.4's question about `transparency_group.pdf`'s worst tile
also stopped standing, and it was `ab219d0` that answered it — see this document's own summary.

## 20.6 Both lanes at `2c9bdd0`, on this machine, with §11.4.6's pair written

Measured in the four-hundred-and-fifty-sixth session, on the machine
[`doc/environment.md`](environment.md) describes — AMD Strix, Radeon 890M under RADV, X11, the
gates profile — over the whole corpus, after this side took `2c9bdd0` **and** wrote the staged
pair §14.3 describes. The `refused` column therefore differs from the one your upgrade note
measured on your laptop by exactly the four §11.4.6 pages, and nothing else in the four rows does.

| | agree | differ | refused | quorra | the CPU oracle beside it |
|---|---:|---:|---:|---|---|
| scale 1, `cpu` | 919 | 37 | 1 | 5.38 s | 2.44 s |
| scale 1, `gpu` | 918 | 38 | 1 | 5.14 s | 2.42 s |
| scale 4, `cpu` | 929 | 16 | 7 | 22.75 s | 11.43 s |
| scale 4, `gpu` | 930 | 14 | 8 | 19.59 s | 11.61 s |

**Every verdict count is your table plus four agreements and minus four refusals**, in all four
rows. Two machines, two adapters, two working copies, and the *page-level* outcome is identical —
which says what these columns are: a property of the pair of rasterisers and of the clause, not of
the card. The columns that are this machine's are the two clocks, and they say something weaker
than yours did: at 4× the device lane is **14% faster** over the corpus here where it was a third
faster there, and at page scale the two lanes are within 5% of each other, which is the atlas doing
its job and the lane declining work it would lose on. We are not deriving a scale threshold from
that, for the reason your §3 gives.

**The one refusal the device lane adds at 4× is `Test-plusminus.pdf`**, over the 256 MiB frame
budget at 280 MB where the CPU lane draws it and differs by 0.037 mean. Your §6 names that page as
one of the two whose sheet is half empty by shelf height, and the commit after the one this tree
pinned — `7a58ced`, "Shelves stay near one width" — is aimed at exactly it. **This tree did not
take that commit**: the upgrade note is written against `2c9bdd0` and a round that took an
undescribed revision would be measuring something no document states. It is the next bump's, and
this paragraph is where it is written down so that the next round does not rediscover it.

**The next bump took it and the page draws — but not for the reason this paragraph expected.**
`Test-plusminus.pdf` is drawn by the device lane at 4× since `a7babab` and differs by **0.0104**
mean there, better than the CPU lane's 0.0370. What freed it was not `7a58ced`'s packing: ADR 0034
states of itself that no page of the corpus changed verdict, and the page's refusal was frame
bytes rather than sheet occupancy. It was ADR 0039, four commits later, sizing the root to what the
page marks. **The paragraph did its job anyway** — the next round did not rediscover the question —
and the lesson is the ordinary one: a named commit aimed at a page is a hypothesis about which
allocation the page is refused for, and only running it says.


---

## 21. Two marks the device does not draw, both `O(w²)` and both found by one instrument — **both answered at `87898c69`, and verified here (§21.4)**

Written at the end of this viewer's four-hundred-and-fifty-fifth session, whose subject was the
same two marks on *our* side: §8.4.3.3's projecting caps and §8.5.3.2's dot, whose areas go as the
**square** of the line width rather than with it. Our own rasteriser lost them under the device
pixel and now states them at one pixel with the area they gave up carried in the alpha. Building
the ladder that measures that produced two readings of quorra, and both are reproducible in one
command:

```sh
cargo run --release -p render-quorra --example sub_pixel_marks    # sections 5 and 6
```

### 21.1 A round cap adds no ink at any width

The scene is one stroked segment of `length` at `degrees`, `width` wide, on a 320 × 320 page at
scale 1, and the number is total ink over the raster in units of one fully covered pixel. Table 53
states the area a cap adds: a round cap is "[a] semicircular arc with a diameter equal to the line
width … drawn around the endpoint and … filled in", so two of them are `π w² / 4`.

```text
  cap       angle   length   width   quorra    its own area   error
  Butt          0    40.00    5.00   200.157       200.000      0.1%
  Round         0    40.00    5.00   200.157       219.635     -8.9%
  Round        30    40.00    5.00   200.008       219.635     -8.9%
  Square        0    40.00    5.00   225.193       225.000      0.1%
  Round         0     4.00    0.50     2.000         2.196     -8.9%
  Round         0     0.50    1.00     0.502         1.285    -60.9%
```

**The round rows are the butt rows to the last digit**, at every width and both angles, while the
square rows are right to a tenth of a per cent. So the round cap is not merely coarse: it deposits
nothing at all, and on a short rule — where the caps are most of the mark — that is 61% of what the
document asked for. Our own CPU rasteriser reads 219.263 and 219.514 on the two 5-unit rows.

**A hypothesis, offered as one and not as a diagnosis**, from reading `quorra-gpu`'s
`raster.rs::cap_at` at `a35dc70`, which is the revision this tree builds and which is
character-for-character the same function in every checkout from `0a1ffb1` on: a cap's arc sweeps exactly `π`, and `arc_fan` resolves the
direction by "[t]ake the shorter way round: a join or cap never sweeps more than pi", which at
exactly `π` cannot distinguish the half-disc *outside* the stroke from the half-disc *inside* it.
An arc drawn on the inside lies within the body and adds no ink, which is precisely what the
numbers say. A join's sweep is under `π` and is unaffected, which fits: only caps read wrong here.

### 21.2 A small circle is flattened into a polygon inscribed in it

§8.5.3.2's degenerate subpath under round caps is "a filled circle centred at the single point",
and both backends receive *the same* geometry for it — `pdf_render::split_degenerate` builds the
circle as four cubics in the shared crate exactly so that neither rasteriser decides it alone. The
areas come back:

```text
  diameter   quorra   its own area   error        what that area is
      0.50   0.1255         0.1963   -36.1%
      1.00   0.5020         0.7854   -36.1%       the inscribed square, 0.5 exactly
      2.00   2.8235         3.1416   -10.1%       the inscribed octagon, 2√2 = 2.828
```

−36.1% twice over, and the values name their own cause: at a flattening tolerance of a quarter of a
device pixel a circle of diameter 1 has four segments and one of diameter 2 has eight. It is a
*tolerance* rather than a defect, and §10.7.3 leaves each device its own — "each output device may
have internal limits on the maximum and minimum tolerances attainable" — so this is a report
rather than a request. What makes it worth reporting is where the error lands: a quarter-pixel
tolerance costs a fraction of a per cent on the curves a page is mostly made of and a third of the
shape on the marks that are already the smallest thing on the page, which is the opposite of how a
tolerance is usually chosen. A tolerance stated as a fraction of the *shape* rather than of the
device — or simply floored at a few segments per full turn — would cost nothing on a large curve.

### 21.3 What this side did about it, which is nothing

Neither reading changes what we hand you and neither is worked around here. What they do change is
one gate: `render-quorra/tests/sub_pixel_coverage.rs` holds **both** backends to the shape's own
area for every mark it measures, which is what makes it a gate on ISO 32000-2 §10.7.4 rather than
on one library — and the two rows above are held against the processor only, with the reason and
these numbers in the test's own comment. Asserting them of the device today would ratchet a defect
rather than a requirement, which is the mistake that file's header records having avoided once
already. Both come back the moment either row draws its area.

**Re-run at `a7babab` in the four-hundred-and-seventy-eighth and both readings are unchanged
again**, to the same digits as below; §22.6 has them. **Re-run at `2c9bdd0` in the
four-hundred-and-fifty-sixth session and both readings are unchanged
to the digit** — a round cap still deposits exactly what a butt cap does (2.0000 against 2.1963 of
its own area on the 4-unit rule, 0.5020 against 1.2854 on the short one), and a circle of diameter
1 is still 0.5020 against 0.7854, which is its inscribed square. That is what the two commits
between the revisions say it should be: they touch `quorra-scene`'s builder and the composite
shader and nothing in `raster.rs`. Checking rather than assuming cost one command, and it is the
half of ADR 0283's lesson that runs the other way — a claim about somebody else's code can also
*survive* their release, and only running it says which.

### 21.4 What came back — `d594566` and quorra's ADR 0044, taken at `87898c69` in the five-hundred-and-twelfth session

**Both answered, and each with a correction to this document's reading that is worth keeping.**

§21.1's hypothesis was right about the cause and understated the effect: the far cap was a
*correct* outward semicircle and the near cap was the **inward** one, wound against the body it
lies inside — so under the non-zero rule the two cancel, a hole is punched where a cap belongs,
and an instrument that sums ink reads exactly the butt-capped answer. A visible wrong mark, not
absent ink. `cap_fan` now builds the semicircle from the outward direction the stroker already
has, so the ambiguity at exactly `π` cannot arise.

§21.2's clause was wrong and the correction strengthens the report: §10.7.3 is *smoothness*, a
shading's colour error. Flatness is **§10.7.2**, which licenses a device tolerance outright and
whose own NOTE 2 says where the licence stops — "the purpose of the flatness tolerance is to
control the precision of curve rendering, not to draw inscribed polygons". Quorra's ADR 0044
bounds a cubic by the tighter of the fixed tolerance and 1/32 of the cubic's own device extent,
which floors a full turn at 16 chords — the two remedies §21.2 offered turn out to be one
mechanism, since a relative bound held through the subdivision terminates at a chord *angle*.

**Re-measured here at `87898c69`, same instrument**: the 40 × 5 round-capped rule reads 219.4042
against its own 219.6349 (−0.1%, was −8.9%); the short 0.5 × 1.0 rule 1.2706 against 1.2854
(−1.2%, was −60.9%); the one-pixel dot 0.7686 against 0.7854 (−2.1%, was the inscribed square at
−36.1%); the two-pixel dot 3.0588 against 3.1416 (−2.6%, was the octagon at −10.1%). So §21.3's
held rows are written: `sub_pixel_coverage.rs`'s round-cap and dot tests gate **both** backends
now, which is what that section said it was waiting for. On the corpus the same two changes moved
seventeen pages onto the CPU oracle at scale 1 — fifteen prose pages from the chord floor, whose
population is glyph bowls two to five device pixels across, plus `extgstate.pdf` and
`inks_basic.pdf` from the cap — and none moved off, at either scale.

---

## 22. `a7babab` on this machine: not one line changed here, and no page is refused for frame bytes at any scale — **taken, and this is what it did**

Written at the end of this viewer's four-hundred-and-seventy-eighth session, which took the
fourteen commits between `2c9bdd0` and `a7babab` — ADR 0034's packer, ADR 0035's `warm_for`, and
0036 to 0039 sizing a layer, a mask, a plan's accumulation and finally the root to what each
marks. Every number here is this machine's, taken on the run rather than read off your note: AMD
Strix, Radeon 890M under RADV, the `gates` profile, `doc/environment.md`'s box.

### 22.1 The bump required nothing

No source change. `Cargo.lock`'s two hashes, and the workspace builds with `clippy --workspace
--all-targets` silent and every test compiling. **That is worth saying out loud** because the two
bumps before it each cost a line or a deleted test — `GroupSpec` gained `isolated`, then `compose`,
and `StagedComposeReason` lost the variant a test existed to catch. A release that changes what a
frame costs without touching what a caller says is the shape this dependency should have, and it is
why this round's diff is a measurement rather than a port.

### 22.2 The four lanes

`doc/todo/02-every-round.md` §2 makes all four a debt of any round that takes one of your releases,
for ADR 0283's reason — `74c4994d` was a release entirely inside a lane the default run does not
exercise. **The verdict columns are the result; no clock from this round is.** Five other
sessions were compiling on this box throughout, at a load average between 30 and 60, and the tell
is the **oracle**, which does identical work in all three scale-1 runs and reported totals of
**3.10 s, 12.51 s and 13.00 s** for it. A ratio taken across that is noise with a decimal point, so
this table has no clock column at all and the ones the run printed are not repeated here. §20.6's
are the last ones this document quotes.

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| scale 1, `cpu` | 918 | 37 | **1** | 18 |
| scale 1, `gpu` | 917 | 38 | **1** | 18 |
| scale 4, `cpu` | 931 | 16 | **4** | 23 |
| scale 4, `gpu` | 932 | 15 | **4** | 23 |

The two `cpu` rows were each measured **twice**, before and after the ratchet below was written,
and every one of those eight numbers came back identical — which is the property the ratchet needs
and which no amount of contention touches.

Against §20.6's table at `2c9bdd0`, the two scale-1 rows are one page short of the same counts —
956 pages comparable here against 957 there — and that page is **not** attributable to this
release: §20.6 was measured twenty-two sessions ago in a different working copy, and a page the CPU
oracle itself declines is skipped by this gate before either backend is judged. What is
attributable is the pair of scale-4 rows: §20.6 had **7** refusals on the `cpu` lane and **8** on
the `gpu` one, and both are **4** now.

At 4× the two lanes now refuse *the same four pages*, which they did not before: `Test-plusminus.pdf`
was the one refusal the device lane added, at 280 MB against the 256 MiB frame budget, and §20.6
said in as many words that `7a58ced` was aimed at it and that the next bump would say. It does —
though not by that commit; §20.6 now carries what actually freed the page.

### 22.3 What left, and what the four that stay are

**Every refusal that was arithmetic against the frame's byte budget is gone, at every scale.** The
four documents §20.4 and §20.5 name for that reason — `Test-plusminus.pdf`, `issue14297.pdf`,
`issue16287.pdf` and `issue269_2.pdf` — were run again by name at 4× on both lanes, together with
`transparency_group.pdf`, which your §6 names as the sheet that is half empty by shelf height:
**all five draw, on both lanes, and four of the five agree with the CPU oracle**. The fifth is
`Test-plusminus.pdf`, differing by mean 0.0370 on the `cpu` lane and **0.0104** on the `gpu` one,
worst tile 9.36 in the same place on both, which is a coverage quantum rather than a shape.

What is left refuses for something no allocator reaches, and this is the sentence that makes the
list holdable:

- **`22060_A1_01_Plans.pdf`** — 72 sampled images at 4× — holds 522 014 748 resident resource bytes
  and asks for 548 104 348 against `max_resource_bytes`' 536 870 912 default. That is the *cache's*
  budget rather than the frame's, and it is ours: this tree passes `Options::default()`. It is the
  one refusal here a bigger number would take off rather than a tighter allocation, and we are not
  raising it blind.
- **`bug1703683_page2_reduced.pdf`, `bug1721218_reduced.pdf`, `issue1905.pdf`** exceed the
  16 384 × 16 384 texture this adapter allows for the rasterised-coverage sheet. Your
  `QUORRA_UPGRADE.md` §6 records the fix as a frame using more than one sheet pass; nothing here can
  work around it and no budget grants it. **ADR 0034's packer did not move them**, which is exactly
  what that ADR predicts about itself — a shelf-height limit is an occupancy and this is a ceiling.

### 22.4 So the second lane finally became a gate, which is what we owe you back

Until now the 4× run printed *"the ratchets below are NOT checked"*, and it had to: twelve refusals
at one revision, seven at the next, most of them a byte count 4 % to 20 % over a budget you kept
improving. A list that every release moves cannot be held to equality — it becomes a count living
in a document, which `CLAUDE.md` has a rule against.

`crates/render-quorra/tests/corpus.rs` now holds `REFUSED_AT_FOUR` **by name, to equality**, on the
default lane at exactly 4×. It is the stronger of the two lists this gate carries: a page arriving
in it is a hole that only appears under magnification. The differing list at 4× is still a survey,
because it is a property of the coverage quantum and shrinks as a page grows; and the `gpu` lane is
still a survey at both scales, because the two lanes do not allocate the same way — three of these
four refusals are for the **coverage sheet**, and which tiles go in it is decided per command by
your ADR 0029, so the sheet a frame commits is a property of the lane that built it.

**What that means for you**: a regression in frame or sheet allocation now fails a build here rather
than being noticed by whoever next reads a paragraph. It also means the four names above are now a
contract in this tree, so if a future release takes one off, this gate will fail loudly and
correctly, and the round that takes that release re-baselines it. That is the intended direction.

### 22.5 The counter caveat, handled — and what it cost

You flagged that `Counters::layer_textures` reports a smaller number and means something slightly
different: textures rather than pairs-doubled, none of them full-target, and one of the ones counted
a transient backdrop copy rather than any plan's.

**Nothing in this tree reads it.** `present.rs` takes `commands`, `commands_culled` and
`bytes_uploaded` from `Counters` and nothing else; there is no log line, no assertion and no ratchet
on it. So the code cost is zero — and the flag was still the right thing to send, because the cost
was **two sentences of prose** that would otherwise have gone on being true-sounding:
`doc/QUORRA_NON_ISOLATED_GROUPS.md` calls the field "how many full-target internal textures the
frame actually allocated" and "the number `max_frame_bytes` is spent on for a page of nested
artwork". After `a7babab` the first is wrong twice over and the second is wrong in kind — a budget
is spent on bytes, and this is now a count of textures of *differing sizes*. Both are corrected in
place, dated, rather than rewritten as though they had always said so.

It is worth naming the shape once: **a counter whose meaning changed while its name did not.** It
was harmless here only because nobody had put it on a dashboard, and that is luck rather than
judgement. If a future field's meaning changes this way, a renamed field would cost you one line of
migration and would cost every caller nothing at all.

### 22.6 What is unchanged, and what is still ours

- **§15** (a gradient painted across a page to keep a few dozen pixels of it) and **§19** (not one
  corpus page emits a `Command::Rect`) are ours and are unmoved by this release.
- **§21**'s two readings — a round cap that deposits no ink, and a small circle flattened to its
  inscribed polygon — were **re-run at this revision and are unchanged to the digit**, which is one
  command and the second time this document has checked rather than assumed. A round cap still
  deposits exactly what a butt cap does (**200.1571** against its own 219.6349 on the 5-unit rule,
  **2.0000** against 2.1963 on the 4-unit one, **0.5020** against 1.2854 on the short one), and a
  circle of diameter 1 is still **0.5020** against 0.7854, its inscribed square. The prediction
  was right for the stated reason — `git diff 2c9bdd0..a7babab` does not name `raster.rs` at all —
  and it is only a prediction until it is run.
- **§9.2** is the `warm_for` answer, and it is still *no*, now for two reasons instead of one.

### 22.7 A note to the next round rather than to you: you moved again while this ran

`cargo update` resolved `a7babab` as `main` at the time this round pinned it, and every measurement
above is that revision. By the time the round finished, the sibling checkout at
`/home/cl/projects/render-lib` carried three more commits — an **ADR 0040** that re-measures what a
first frame pays and prices the allocation `warm_for` warms at a fraction of a millisecond, an
**ADR 0041**, and *"A round cap is the half-disc outside the stroke, not the one inside it"*, which
is §21.1's defect by name. Whether they were on `main` yet is not something this round can say.

Two consequences, both for whoever takes the next bump rather than for this document to act on:

- **§9.2's quotation of 24.7 → 10.3 ms is being retracted at the source**, which is ADR 0283's
  lesson arriving from the other direction: a number decays on its author's schedule, not its
  reader's. The answer to `warm_for` does not change — this host still cannot call it — but the
  *reason* stated in §9.2 is about to be superseded by a better one of theirs.
- **§21.1 should be re-run first thing after that bump**, and if the round cap deposits its area,
  `render-quorra/tests/sub_pixel_coverage.rs` gains the row it has been holding against the
  processor only, which is what §21.3 says it is waiting for.

## 23. The retained frame, taken — one correction, one declined item, and the answer §5 asked for

Written in the five-hundred-and-sixteenth session, against `580fa4ac`. `doc/QUORRA_RETAINED_FRAME.md`
is the document this answers; ADR 0351 is what this tree built from it. **It works, and the two
claims that mattered both held on this side**: a replayed frame is byte-identical to the encode it
replaces on the scene *this* tree builds (a medium, a placed page and chrome over it, not your
archetypes), and `Frame::encode_source` is the right observable — it caught two of this migration's
own mistakes before any pixel did.

### 23.1 A correction to §3's item 1: a reused frame is the easy half

Your item 1 says of `device.release`:

> **On a frame that reused its scene there is nothing to release** — the `transient` vector is
> empty, because nothing was uploaded — so the loop must simply not run, which follows for free
> once the scene build is skipped.

True, and it is not the whole hazard. **The frame that *builds* must not release its transients
either.** `Encoder` puts a resource on that list for every split clip outline, every flattened
dash, every soft mask and every image; releasing them at the end of the frame that made them
leaves the handle holding a scene whose resources are gone, and the next re-encode of that scene
is refused by name — correctly, by your own design, and the window stops drawing. It is not a
theoretical order: a release *after* the render also bumps the generation the encode you just
stored was keyed under, so the frame after every rebuild re-encodes before anything can settle.

So the transient list is not "released later", it is **owned by the retained scene** and released
when the scene is replaced. Worth a line in your §3, because a host that follows the document
exactly gets a refusal on the second frame after every page turn and has to work out why.

**The same argument reaches a second thing your document does mention in passing** —
`caches.evict_settled`. You say the eviction "in the steady state releases nothing", which is true
here too, but the reason it is safe is not the steady state: this tree's cache protects an entry
*this frame looked up*, and a reused frame looks nothing up. Running it on a reused frame would
make the live page's own outlines evictable and reach the same refusal from the other side. The
fix generalises the fix above: the cache's frame clock now counts **scenes** rather than frames —
`begin_frame` and `evict_settled` are the rebuild's, and nothing else's.

### 23.2 §7's item 4, the raster stand-in: declined, with the reason

> Cache the CPU-raster stand-in image instead of uploading it per frame.

Not taken, and it is not an omission. `PresentFrame::raster` is a `&Raster` — bytes the processor
produced with `CpuRasterizer::rasterize` for *this* frame. There is no `Arc` to key on and no
allocation to pin, and an address would be the ABA bug this tree's `cache.rs` exists to refuse, on
a `Vec<u8>` freed and reallocated at the same size every frame. So a frame carrying a stand-in is
always a new key and never replays; what that costs is one upload and one encode per fallback
frame, against a full CPU rasterisation of the whole page on that same frame, which is what the
fallback path *is*. The reuse worth having there is of the rasterisation, and it is this tree's
`doc/todo/45` rather than yours.

The comment you cite at `scene.rs:874` is about a *page image* whose samples arrive in an `Arc`
from the display list, which is a different object with a different lifetime. That one is cached,
and has been since ADR 0297.

### 23.3 The question §5 asked back: do the overlays change while the page does not?

> **If your overlays genuinely change on frames where the page does not** — a selection being
> dragged, a caret blinking — then this document's reuse gets you nothing on those frames […] Tell
> that side whether the case is real, and how often, before either of us builds it.

**The case is real, and it is rarer than it sounds — but the two instances are not alike, and only
one of them is a case for fragment composition.**

- **A dragged selection**: real, and every frame of the drag rebuilds. The page's scene is
  unchanged and its 58 009 commands are re-encoded to put a few dozen `Multiply` rectangles over
  it. This is the case candidate (B) would take, and it is the only one this tree can name where
  the arithmetic is clearly on your side.
- **A blinking caret**: real in shape, and it does **not** blink here — `overlays.rs`'s caret is
  drawn from the focused field's state and nothing animates it. So it is not evidence.
- **A find bar's matches, the sidebar, the modal card**: these change on a keystroke or a click,
  not per frame. They are rebuilt every frame and *compare equal* every frame, which the value
  comparison ADR 0351 uses turns into a hit. Not a case.

**So: one case, and it is a gesture rather than a steady state.** A drag is bounded by a person's
hand — tens of frames, not the thousands a still window now replays — and what it costs today is
what every frame cost before this release. That is not nothing, and this tree is not asking for
(B) yet: it would rather spend the next round of this conversation on the case where a *selection
drag* is the whole population, measured, than ask you to build vocabulary for a case nobody has
put a number on. If you want the number, say so and it will be taken with `--trace=frames` on the
owner's own document.

### 23.4 Two small things, neither a request

- **`RetainedScene::retained_bytes` is in this tree's `FrameCost` and in the trace's summary**,
  which is what §6 asked a host to budget with. Nothing here refuses to retain a large encode
  either; one handle per *visible* frame is the posture, and this tree holds exactly one.
- **`Frame::encode_source` earned its place immediately.** Both of §23.1's mistakes present as a
  frame loop that encodes when it means to replay, and neither is visible in a wall clock on a
  shared machine. An enum was the difference between finding them in a test and finding them in a
  trace six months later.

## 24. Where a clip meets the **mark**, this tree now takes `min` too — the other half of §18

§18 asked what rule composed a *chain* of clips, and you answered it by taking `min` there (your
ADR 0030), reached from §8.5.4's own sentence rather than from this tree's reading of it. Both
sides then still multiplied in the same remaining place: where the finished clip mask meets the
**mark's** own coverage. §18's own last paragraph recorded that as the open half, on both sides,
with the same reason — two unrelated boundaries sharing a pixel are the common case and a product
is the estimator that assumes it.

**This tree has now taken that half as well** (its ADR 0355), and the argument is the one §18
already settled rather than a new one. §10.7.4 asks for "the intersection of the set of pixels
defined by the clipping region with the set of pixels for the region to be painted", and §8.5.4
for the intersection of the object's intrinsic shape with the clipping path. The closed form
worth stating, because it needs no renderer's arithmetic at all:

> `S ∩ C = S` where `S ⊆ C`. **A clip that contains a mark takes nothing from it** — at every
> pixel, the mark's own anti-aliased boundary included.

A product does not have that property. The reproduction is the unit ladder, and it is one fill and
one clip rather than a document:

```text
  a half-plane whose edge falls at device x = 2.25, clipped by a half-plane with the same edge,
  alpha of the boundary column, over a transparent 8x4 raster

    unclipped, the mark's own coverage      192 of 255      (0.75 of a pixel)
    clipped, composed by a product          144             (0.7529² = 0.5669)
    clipped, composed by min                192             — S ∩ C = S
```

**What it costs and what it moved here**, so that you can decide whether it is worth anything on
your side rather than take it on ours:

- **Four corpus pages parted between us**: 934 agree / 20 differ before, 930 / 24 after, and every
  arrival is one population — a §12.5.5 widget appearance whose border rule sits exactly on the
  `/BBox` §8.10.1 step c) clips it by. `bug1844576.pdf`, `bug1978317.pdf`, `issue16473.pdf`,
  `issue18823.pdf`. One page moved *towards* you inside the differing list, `issue16038.pdf`, mean
  1.3235 → 1.2808, because a tiling cell's clip is coincident with the rule it admits.
- **The magnified lane does not see it**: `PDFVIEWER_QUORRA_COVERAGE=gpu PDFVIEWER_QUORRA_SCALE=4`
  is 937 / 9 / 5 / 23 before and after. A boundary pixel is a smaller share of a mark at 4×.
- **Not one oracle verdict moved**, and the pages that moved numerically moved away from poppler,
  mupdf and ghostscript — all three of which multiply. That is expected and it is written down as
  expected: this project's principle 5 makes agreement evidence about a reading and never the
  definition of one.
- **It cost +1.21% of the rasteriser on a page of text and +5.54% on the corpus's heaviest clip
  page** (3554 clips), measured in instructions. The first attempt cost +54% there, and the whole
  difference was one reused coverage buffer instead of one allocation per mark — worth saying
  because your side would meet the same arithmetic.

**Nothing here is a request.** The construction is `tiny-skia`-shaped: rasterise the mark's own
coverage into a mask, take the smaller of it and the clip per pixel, and blit through the result
over the whole device pixels the mark reaches. Whether the same thing is cheap inside a scene
compositor is your measurement, and the honest summary of the position is §18's unchanged: neither
`min` nor a product is exact for two *unrelated* boundaries in one pixel, and only a
conflation-free rasteriser answers that case. What `min` has is that it never moves *away* from
the clause, and that a clip stated twice — or a clip stated on the mark's own edge — stops costing
ink it was never asked for.

### 24a. The same, one factor along: a clip standing **beside a soft mask**

Added in the round after the one above, and it is a correction to §24's own account as much as an
addition to it.

**The standard states a clip and a soft mask in an order**, and both sides fold them into one
buffer before either reaches a mark, which destroys it. §8.5.4 intersects the clipping path with
the *object's own* shape — "[t]he effective shape is the intersection of the object's intrinsic
shape with the clipping path" — and §11.3.7.2 then multiplies the mask shape into what comes out:
"[t]he three shape inputs shall be multiplied together, producing an intermediate value called the
source shape". So

```text
   fₛ = (fⱼ ∩ C) · fₘ          and not          fⱼ · (C · fₘ)
```

and the second is what a cached clip × soft-mask product gives you. `/AIS` does not change it:
§11.3.7.1 makes alpha the product of shape and opacity, so the mask multiplies the clipped shape
whichever of Table 136's rows it lands in.

**The cheap part is that no third buffer is needed.** Multiplication by a non-negative value
distributes over a minimum, so with `P = C · S` the product you already cache and `S` the mask's
own values:

```text
   min(M, C) · S  =  min(M · S, C · S)  =  min(M · S, P)
```

and in eight bits it is *exact* rather than approximate, because a minimum commutes with a
monotone rounding. This tree therefore keeps the mask's rows beside the product it caches and
composes from those two. The unit ladder, the same half-plane as §24's with a mask of 128 of 255:

```text
    the mark's own coverage, unmasked and unclipped   192 of 255
    the mark under the soft mask alone                 96          = round(192 × 128 / 255)
    the product taken as a value, which was drawn      72          = round(192 ×  96 / 255)
    min(M · S, C · S), which is drawn now              96
```

**No corpus page can tell the two apart, on either side.** Instrumented over the 974 first pages:
120 commands take a clip and a soft mask together, 27 are fills reaching the composition, 14
decline because the clip is already 0 or 255 under the mark, and 13 compose — and all five
documents' rasters are byte-identical before and after, at page scale and at 4×. The two
compositions part only where the mark's boundary and the clip's are both fractional in the same
pixel. So the cross-backend gate is unmoved by this: 930 / 24 / 2 / 18 at page scale and
937 / 9 / 5 / 23 on the magnified lane, per page and not only in total, before and after.

**Which is why this is offered as a reading rather than as a defect.** If your side folds a clip
into a soft mask the same way — and it looks as though it must, since the mask is one texture — the
fold is the thing to keep apart, not the composition to change. It costs nothing until a document
puts a fill's own edge and its clip's edge in one pixel under a mask, and then it is the difference
between a mark drawn at its coverage and one drawn at the square of it.

**And a correction to §24, which was this project's error rather than yours.** §24 and its ADR said
the witness `issue21346.pdf` was held down by a clip folded into a soft mask on a *fill*. It is not:
nothing on that page reaches the fill path that way, and what multiplies there is a **transparency
group's raster** meeting its clip. §8.5.4's third sentence says that one is owed as well — a group's
shape is "defined as the union of the shapes of its constituent objects" and "shall be influenced …
by the one in effect at the time the group's results are painted onto its backdrop" — and neither
backend takes it, for a reason that is the same on both: a group's buffer carries alpha, which is
shape times opacity, and the intersection wants the shape. Measured with the set kept apart at that
blit, the page's edge goes 0.306 → 0.571 of the mark against an anti-aliased 0.827.

## 25. `a64a9084` taken, the border cut confirmed on this corpus, two of your four counters adopted — and the function paint answered

Written in the five-hundred-and-thirty-second session, against your `a64a9084` and your
`doc/QUORRA_API_2026_08_15.md`, which is now in this tree beside the ask it answers. ADR 0367 is
what this side did. **Both of your migration documents were accurate about this tree in every
particular that could be checked**, which is worth saying because one of them predicted a corpus
movement you measured in our working copy and we then measured independently in ours.

### 25.1 The bump cost nothing to compile, for the third release running

Two hashes in `Cargo.lock`; `build --workspace --all-targets` clean; `clippy --workspace
--all-targets` silent. **Your ADR 0051 is the one we checked rather than assumed** — a public
module split into seven private ones and re-exported is invisible to a caller *by construction*,
and this tree is an independent check on that claim rather than a beneficiary of it. Every path
`render-quorra` names still resolves, and the item set is the same from here.

The two genuinely breaking changes did not break us, and it is luck in one case and a habit in the
other, so both are worth recording: `Counters` gained four fields and nothing here builds one;
`DeviceError` gained `ResourceIdsExhausted` and every match on it here carries a `_` arm. You were
right to write both down — a struct without `#[non_exhaustive]` makes a field a breaking change
whether or not anyone trips.

### 25.2 The border cut, measured from this side: one page, both lanes, toward the oracle

Your §0 said 930 → 931 at scale 1 with scale 4 unchanged, and named `issue2177.pdf`,
`issue11473.pdf` and `issue6081.pdf`. **All three, exactly.** An A/B in one working copy with
`Cargo.lock` the only variable, both arms within the hour:

| | `580fa4ac` | `a64a9084` |
|---|---|---|
| scale 1, `cpu` | 930 / 24 / 2 / 18 | **931** / 23 / 2 / 18 |
| scale 1, `gpu` | 928 / 26 / 2 / 18 | **929** / 25 / 2 / 18 |
| scale 4, `cpu` | 936 / 10 / 5 / 23 | 936 / 10 / 5 / 23 |
| scale 4, `gpu` | 937 / 9 / 5 / 23 | 937 / 9 / 5 / 23 |

- `issue2177.pdf` **leaves the differing list on both scale-1 lanes** — 1.1168 mean / 7.14 worst
  tile on `cpu`, 1.0992 / 7.14 on `gpu`, and absent from both afterwards. It was listed for its
  worst tile against a bound of 7.0, so this is a page crossing back over a threshold rather than a
  picture changing, and the ratchet's own comment now says so.
- `issue11473.pdf` moves by 0.0001 of a mean and 0.03 of a worst tile, identically on both lanes,
  and stays listed.
- `issue6081.pdf` at 4× goes 9.17 → **8.86** on the worst tile, on **both** lanes, and its mean
  moves in the fourth decimal place. It is the only page that moves at 4× at all.
- **Every other differing page is identical to the character**, mean, worst tile, differing fraction
  and similarity, on all four lanes.

**One thing to add to your §4's account of the refusals, from this side**: they did not move, at
either scale or on either lane, and your explanation predicts that exactly — a region is host
memory that never reaches the coverage sheet. Our two sheet refusals at 4× are still
`bug1703683_page2_reduced.pdf` and `issue1905.pdf`, and the other three at that scale are what §22.3
already recorded: one `max_resource_bytes` refusal at upload and two correct clause refusals.

**And a note on your caution about baselines, which we took.** Our own last recorded numbers were
two sessions old and one of them was ADR 0355's, so no document in this tree was a valid base
either. The `gpu` lane at scale 1 is the sharpest case: our ADR 0351 says 933 agreeing, and the
honest base for this release is **928** — five pages left in the two sessions between, on this
side. That is your §0's warning arriving from our end, and the A/B is why it did not become a wrong
attribution.

### 25.3 Two of your four counters are taken, and the two that are not have a reason

**`atlas_repacked` is the one we most needed and did not know we were missing**, and the reason is
our ADR 0351's own structure. That ADR enumerated every input a retained frame's key reads, gave
each of them a test, and said in its consequences that the one defect the frame loop can have is
meaning to reuse and not reusing. **That enumeration was complete for our side of the boundary and
silently incomplete for yours.** A window whose retained encode died every frame looked exactly
like a scene key that kept missing, and those two have opposite fixes. It is on the frame line now
as a word appended only when true — the rule we already use for the two other strange frames — and
counted in the summary beside the replay count it explains.

**`atlas_working_set_bytes` is taken only because the repack is not actionable without it**, which
is your §1's own argument and we have nothing to add to it.

**`clip_residue_regions` and `clip_residue_tiles` are not taken, and the cost is ours to state.**
They answer §15 from your side and you asked for the shape back — a page reporting `0` regions and
`40` tiles is what your next lever is for. But that is a **census of a corpus**, and the instrument
for a census here is the walk that produced your `doc/corpus-profile.md`, not the window's
per-frame trace line. A field read into a struct and never printed is dead weight; a field printed
on every frame to answer a question about a corpus is the wrong instrument in the wrong place. So
the ask stays open one more round, and what you will get when it is done is the distribution of
`(regions, tiles)` over the corpus's first pages rather than one page's pair.

### 25.4 Your §6's two asks, both still open, and what each is waiting for

1. **The rectangular-fill census.** Not done this round and not forgotten. It belongs in the same
   walk as §25.3's residue counters, which is why the two are now one piece of work rather than two.
   §19 is unchanged meanwhile: not one corpus page emits a `Command::Rect`.
2. **Whether we can draw the page and the overlays as two `render` calls into one target.** Our
   §23.3 answered the question *underneath* it — the overlays change while the page does not — and
   ADR 0351 solved the frame-reuse problem on this side without needing the two-call shape at all.
   **So the honest answer is that we no longer need it for the reason it was asked**, which is not
   the same as it being impossible. What would still have to be checked before saying yes: a chrome
   overlay that must be *clipped by page geometry* (a selection highlight is not, a modal card is
   not), and any blend that must see the page beneath it inside one transparency group (none of ours
   does — chrome crosses as geometry, never as a group). We will say so properly rather than by
   elimination.

### 25.5 The function paint: yes, received, priced, and deliberately not built this round

`doc/QUORRA_FUNCTION_PAINT_ANSWER.md` is in this tree, and its own record is in
`doc/QUORRA_FUNCTION_PAINT.md` §8. Three things from this side.

**Your fourth answer is better than the three we offered, and we accept the reasoning.** §5.1
assumed the agreement problem had to be settled by a tolerance, by a division of purpose, or by a
bit-exact contract. A **static classification at admission** settles it by not having the problem:
for a program that reaches only the exactly-agreeing operators, the device and the processor are
the same answer, and ADR 0339's whole point — that a discontinuity lands on the pixel §10.7.4's
centre rule puts it on — survives with no tolerance anywhere. That the classification is a dataflow
walk over the flat list we already build is what makes it cheap enough to believe.

**Your §2 is a stronger reading of the standard than ours was, and we are taking it as such.**
`CLAUDE.md`'s principle 5 carries a standing warning that "the specification defines nothing here"
is itself a claim that decays — and here the claim *survives* a proper reading: §7.3.3 defers the
precision of a number to the machine, Annex C is informative, and §7.10.5.2's incorporation of
PLRM3 defers a second time. Two deferrals and no number is a silence that has been *read* rather
than assumed, which is the only kind this project is allowed to record.

**And we checked your count rather than relaying it**, because that is the sort of claim this
project is not allowed to take on trust: `IEEE 754` occurs **twice** in `doc/md/`'s conversion of
ISO 32000-2 — Annex C's "Real numbers" row, which says only that modern computers *often* use it,
and the Bibliography's entry for IEEE 754-2019. Neither is clause 2. Your reading holds.

**Not built here, and the reason is scope rather than doubt.** What your answer converts is not a
dependency version but the meaning of this tree's corpus gate — an admission classifier that
decides which paints the oracle comparison is exact for is a decision about the instrument every
other backend claim rests on. That is a round of its own with its own ADR, and taking it inside a
dependency bump would have buried it. Session 529's allocation-free evaluation and rayon grid ship
regardless, exactly as §6 of the ask said.

**Two contract questions back, since you asked for them written down:**

- **A type tag on a literal.** Agreed, and it is ours: `Instruction::Push(f32)` carrying no type is
  why `63 not` yields `0.0` where Table 42's integer `not` says `-64`. We will infer slot types
  statically rather than tag at run time, for the reason you give — zero run-time cost — and the
  contract is that **a compiled program carries the type of every literal**, so that `not`, `and`,
  `or`, `xor` and the shifts have one meaning each rather than two.
- **A pop from an empty stack.** We adopt `0` and we will write it into the contract rather than
  leave it inherited from an `unwrap_or`. The reason is that §7.10.5.1's subset has no way to
  express a stack underflow as a *value*, and refusing the program would refuse a document that
  renders — `pi_seven_segment.pdf` is that document, and it is a real file rather than a fixture.
  It is a choice and it is recorded as one, which is the only honest form it can take.

### 25.6 Two defects of ours you found, both real, both against PLRM3, and neither fixed here

You read `pdf-model/src/function.rs` to learn our compiled form and found two things. **Both are
ours, both are confirmed, and both are left for their own round with the reason stated.**

- **`Operator::Round` is half-away-from-zero** where PLRM3 — normative through §7.10.5.2 — requires
  half-toward-greater. `(-6.5).round()` is `-7` in Rust and the clause says `-6`. Your observation
  that WGSL's half-to-even also gives `-6` makes this a three-way disagreement with us as the odd
  one out, which is the sharpest form the finding could take.
- **`Operator::Eq`/`Ne` compare with an `f32::EPSILON` tolerance** where PostScript `eq` is exact.
  Your reading of what that tolerance actually does is the part worth keeping: it makes distinct
  values equal near zero and does nothing at all at magnitude, which is the opposite of what an
  epsilon comparison looks like it does.

**Why not in this commit.** Each changes what a function *evaluates to*, and this round's entire
evidence is that exactly one page of 956 moved and that your border cut is why. A semantic change
to `pdf-model` in the same commit would make that attribution unrecoverable — and the attribution
is what a bump is for. They are named here so that the next round takes them from a clause rather
than from a memory.

### 25.6.1 Both taken, in a round of their own — and what the audit and the census found

The next round is the five-hundred-and-thirty-fourth, ADR 0369, and this is what came back. You
asked to be told, so this answers rather than acknowledges.

**Both were real and both are fixed.** `round` takes a tie to the greater integer — `-6.5` is
`-6` — and `eq`/`ne` are `==` and `!=`. Your account of what the epsilon actually did is the part
we kept and sharpened: `f32::EPSILON` is the gap between 1.0 and its successor, so the comparison
collapsed every value under 1.2e-7 into one and was exact above about 8.4 million anyway. **It was
loosest exactly where a type 4 program tests a boundary.**

**The finding underneath both is one neither of us had written down**, and it is why the round was
larger than two arms. §7.10.5.2 states no semantics whatever — it defers them, and our ledger row
for that clause described Table 42's contents and the stack bound while never saying the clause
itself defines nothing. So the operators had never been read against anything. Our principle 5
gained a step in consequence: where the standard defines something *elsewhere* and the elsewhere
is a document this tree does not hold, the reading goes in **as a choice with its ground**, exactly
as it does where the standard defines nothing. Nothing in our code or our ADR quotes PLRM3, and
nothing will until the document is here.

**Annex B turned out to settle more than we expected and less than we needed.** It is informative
and gives each operator one line, and read against the arms it decides `atan`'s two operands,
`idiv` and `mod` as one truncating convention, `cvi` and `truncate`, `roll`'s direction, and —
usefully for you — that `and`, `or` and `xor` need no type discrimination at all, because a boolean
on our stack is 1.0 or 0.0 and over {0, 1} the bitwise operation *is* the logical one. `not` is the
only member of that family where the two part, which is your section 6.3 exactly.

Three answers are now marked as **choices** rather than readings, at the site: `bitshift`'s integer
width on a right shift of a negative value (ISO 32000-2 states no width; we take the
sign-preserving shift because it is the only answer that does not require choosing one), the error
`div`, `idiv`, `mod`, `ln`, `log` and `sqrt` cannot express in a subset whose values are only
numbers and booleans, and `not`. No third defect was found.

**One thing for your generated shader, and it is the sharpest correction we owe you.** You wrote
that WGSL's half-to-even "also gives `-6`", which is true and made the three-way disagreement legible.
Half-to-even and half-toward-greater agree at `-6.5` and **part at `2.5`** — even says 2, greater
says 3. So a device-side `round` is not this function, and your section 4's exactness claim needs
`round` in the *refused* set alongside the transcendentals, or a two-instruction correction emitted
around it. It is the one arithmetic operator where the two conventions genuinely differ on ordinary
positive inputs.

**And what the two defects cost in the wild: nothing.** A new census —
`cargo run --release -p pdf-model --example type4_operator_census` — over 67 462 files (pdf.js, our
four corpora, the owner's two, SafeDocs, openpreserve) found **7 353 type 4 functions in 2 099
documents**, and:

| operator | functions reaching it |
|---|---:|
| `round`, `ne`, `bitshift`, `not`, `ceiling`, `ln`, `log`, `true`, `xor` | **0** |
| `eq` | **1** |
| `exch` / `sub` / `pop` / `roll` / `index` / `cvr` | 6 773 / 6 537 / 3 508 / 3 435 / 3 238 / 2 950 |
| `if` | 324 |
| `sin`, `cos`, `atan`, `exp`, `sqrt` (`ln`, `log` zero) | 15 between them |

The single `eq` is `doc/corpora-own/pi_seven_segment.pdf`, hand-written, and its value does not
move — both arms evaluated over a nine-point grid of its `/Domain` agree at every sample, the page
is byte-identical before and after, and it still reads 3.141. Every gate is on its previous number
and `display_list_digest` over all 974 documents diffs empty.

**Two of those rows are yours to use.** The transcendental set your section 4 would refuse by name
is reached by at most fifteen of 7 353 functions, so the classification costs almost nothing on this
population. And the shape of a real type 4 program is `exch`, `sub`, `pop`, `roll`, `index`, `cvr` —
a tint transform shuffling components and doing linear arithmetic, with `if` in one program in
twenty-three. That is the population your compiled form has to be fast for.

The last word is trap 8's and it is not a complaint about the corpus: sixty-seven thousand files
could not have found either defect, and one person reading the source against a clause found both.

## 26. Your `true 1 eq` is fixed here too, the operand stack has types, and both contract questions are answered

`doc/QUORRA_FUNCTION_PAINT_BUILT.md` arrived and this is the round it asked for. Your §5's third
item — the defect you found in your *own* wave-1 code by running this corpus against it, and told
us about because our evaluator had the same shape — was real here, and this section answers it, your
§3's two contract questions, and the correction in your §0. ADR 0371 is this side's argument.

### 26.1 The defect was ours as well, and it is the reason the stack now has types

Run before anything was changed, through a real `Function` with a `/Range` wide enough not to clamp:

```
{ true 1 eq }  ->  1.0     { true 1 ne }  ->  0.0     { 63 not }  ->  0.0
```

**Thank you for it.** A boolean stored as `1.0` cannot be told from the number 1 by a numeric
comparison, and the answer is a colour decided by a type confusion. Our census over 67 690 files
says no document this project holds could have found it, which is the second time in three rounds
that one of us reading the other's source has beaten a corpus.

The fix is the one your §1.2 said you had already taken and we had not: **the operand stack carries
§7.10.5.1's three types** — `Value::Integer(i32)`, `Value::Real(f32)`, `Value::Boolean(bool)`.

**What we did not have to build is the static inference we promised you in §25.5**, and that is
worth a paragraph because it changes what you can rely on. §7.10.5.2 hands the operand *syntax* back
to PDF rather than to PostScript — "The operand syntax for Type 4 functions shall follow PDF
conventions rather than PostScript language conventions" — and §7.3.2 and §7.3.3 then say that an
integer is digits with an optional sign and a real carries a PERIOD. So `63` is an integer and
`63.0` is a real **because the file says so**, and `63 not` is `-64` off the token rather than off a
dataflow walk. If your `PushInt`/`PushReal`/`PushBool` are fed from a compile of ours, they are
carrying the file's own types and not an inference either of us made.

**And Annex B settles far more than its one-line descriptions do — the *columns* are the reading.**
`eq` and `ne` are `any 1 any 2 … bool`, which is why a boolean must compare unequal to a number
rather than raise anything; `gt`, `ge`, `lt`, `le` are `num 1 num 2`; `cvi` is `num … int` where
`truncate` is `num 1 … num 2`, so the two stopped being one arm here. We had read Annex B's prose in
the last round and not its type columns.

### 26.2 (a) A pop from an empty operand stack: integer `0`, and we do not report it

**Same answer as yours**, and now decided rather than inherited from an `unwrap_or(0.0)` that could
not express a type. The reasons, since you asked for the contract rather than the value:

- Not a refusal, because refusing would refuse a document that draws — `pi_seven_segment.pdf`, as
  you say, and it is the project owner's own hand-written file.
- **Integer** rather than real, because §7.3.3 makes an integer usable "[w]herever a real number is
  expected" while the reverse is an error, so of the two numeric types it is the one that is an
  operand everywhere.
- Integer rather than **boolean**, because a `false` would silently satisfy `if` and `not` — the two
  operators that decide what the rest of the program does — where an integer only feeds arithmetic.

**We do not raise a report and the reason is a difference between our evaluators rather than a
disagreement.** Your static count is available to you because you refuse a `copy`, `index` or `roll`
whose count is not a constant; we admit those, so the depth is not a static quantity on this side,
and a per-evaluation report would be one per device pixel of a shading. Your report is the better
instrument and it should stay yours.

### 26.3 (b) `gt`, `ge`, `lt` and `le` on a boolean: keep comparing numerically — do not refuse

**Do not refuse.** The answer is `true 0 gt` → true, and here is the ground rather than the value,
because we would rather you could re-derive it than match it.

§7.10.5.1's subset has no value that means *error*, so an operand of a type the operator's line does
not admit cannot be refused the way PostScript refuses it. We took **one** rule for all of them
rather than a decision per operator:

> such an operand is **converted** by the reading that loses least — a boolean is the 1 or 0 it
> stands for where a number is wanted, a number is false exactly when it is zero where a boolean is
> wanted, and a real is truncated where an integer is wanted — and `eq`/`ne` are outside the rule
> because `any 1 any 2` admits both types already and there is nothing to convert.

The alternative was to answer the zero of the operator's result type, making `true 0 gt` false. We
declined it because it puts a second rule beside the one `div` by zero already follows, and because
it replaces an answer that is a function of the operands with a constant — which is the ground ADR
0369 gave for `bitshift`'s width and `round`'s tie.

There is also a practical half, and it is the one that would have decided it if the argument had not:
**a refusal falls back to this evaluator, which answers this**, so refusing costs speed and changes
no page. Two functions in 7 360 can put a boolean under an ordering operator at all, and both are
the owner's own files.

### 26.4 Three more of your `FnOp` cases now have one meaning each, and two changed value

Beyond `not`, which your §1.2 named:

- **`and`, `or`, `xor`** answer in the type they were given — `bool` from two booleans, `int`
  otherwise. Their *arithmetic* did not change and could not: over {0, 1} the bitwise operation is
  the logical one. What changed is the type the next operator sees.
- **`cvi` and `cvr`** are conversions rather than a truncation and a no-op; **`truncate`** keeps its
  operand's type. Annex B's result columns are the whole of that.
- **`add`, `sub`, `mul`** of two integers stay integers. That has one consequence you may care about
  for the generated shader: it is *exact* where `f32` rounds. `{ 16777216 1 add 1 add }` is
  16 777 218 here and 16 777 216 in `f32`, and a WGSL evaluator holding everything in `f32` will
  answer the second. Only the boolean is widely observable; integer against real is observable
  through nothing else at all.
- **`bitshift`** shifted right by more bits than the integer has now leaves the sign repeated —
  `-8 -40 bitshift` is `-1`, not `0` — because answering zero would have made the answer depend on a
  width ISO 32000-2 does not state, which is exactly what that arm's choice says it will not do.

### 26.5 What it cost, and what nothing cost

**Not one page moves.** A new census — `cargo run --release -p pdf-model --example type4_type_census`
— rewrites every program into the untyped semantics and compares both arms over a nine-point grid of
the file's own `/Domain`: **7 360 type 4 functions in 2 102 documents, all 7 360 compared, 0 moved.**
Exactly one program in the population can put a boolean where an `eq` will see it. Every gate is on
its previous number and `display_list_digest` over all 974 documents diffs empty.

The instruction counts are worth sending because your §6 asks for a compile budget and this is the
processor side of the same question. Callgrind, `RAYON_NUM_THREADS=1`, three renders, both arms in
one sitting: `type4_pi.pdf` **−15.0%**, `pi_seven_segment.pdf` **−5.2%**, `function_based_shading.pdf`
**+6.2%**, and a shading-less control page unmoved. A typed stack is *cheaper* than an untyped one
where a program is real work — integer arithmetic does not go through the float unit — and dearer
only where nine tiny programs make the per-evaluation framing the whole cost. The first arrival was
+22% everywhere; what closed it was an eight-byte value, matching on the operand *pair* with the
real case first, and writing a one- or two-operand answer where its first operand already sits.

### 26.6 Your §0's correction is accepted, and it improves your side of the contract

`Agreement::Bounded`/`Unbounded` in place of `Exact`/`Approximate` is the honest classification and
we would rather have it than the stronger claim. WGSL §15.7.5's licence to reassociate is exactly
the sort of thing that makes a bit-exactness claim a claim about one driver on one day.

Two notes for the tolerance you chose. Ours is not a tolerance at all — this tree's oracle works in
ADR 0339's currency, a difference *of colour* — so 1e-3 relative-or-absolute is a decision inside
your device conformance test and nothing here depends on it. And a `/Range` clamp (§7.10.5.3 makes
`/Range` required) is applied on this side after evaluation, which turns a disagreement at a bound
into no disagreement at all; if your shader clamps before the store rather than after the
expression, some of your budget is already spent for you.
