# 0351 — The frame a still window stops building again, and the three things that were undoing it

**Status.** Accepted.

## Context

`doc/todo/44` §3 priced what a person looking at the project owner's own document pays for
looking at it: 28 frames of `tmp/Entwurf.pdf` at one view, the display list unchanged after the
first, a median frame of 393.1 ms of which `scene` is 50.2 and quorra's `encode` 233.8. Every
one of those frames re-translated the same display lists into the same scene and re-walked the
same 58 009 commands to produce the same device commands, for an answer that could not differ.

The upstream half of the fix arrived on 2026-08-14. quorra's ADR 0048 built `RetainedScene` and
`Device::render_retained`: a handle the caller holds, owning the `Scene` and the encode of its
last frame, replaying that encode when nothing an encode reads has moved. The project owner
wrote `doc/QUORRA_RETAINED_FRAME.md` from that side, and it is explicit that **nothing happens
automatically** — `Device::render` is untouched, and a caller that ignores the addition sees
today's library exactly. It also names the part that costs work, and names it in advance:

> Today `present` rebuilds everything on every frame: a `SceneBuilder`, the background
> rectangle, the page's display list through `Encoder`, then each overlay list. Under
> `render_retained` that rebuild is exactly what has to stop happening when nothing changed.

and three things `present` does on every frame that would defeat it even after the rebuild
stops — the per-frame `device.release` of the transient list, the CPU raster stand-in uploaded
and released per frame, and a `set_scene` on every frame, which is the same thing as not
adopting any of it.

## Decision

**Take upstream's head, `580fa4ac`, and adopt the retained frame in `render-quorra`'s frame
path — both of them, the presenter's and the rasterizer's.** `doc/QUORRA_UPGRADE.md` carries the
range; this ADR is the adoption and the four judgements inside it.

The shape is the owner's document's, with one structure added that its sketch left to the
migration: `FrameSlot` in `crates/render-quorra/src/present.rs` holds the retained scene, the
key it is valid under, and — the part that turned out to be load-bearing — everything that scene
still *names*. `QuorraPresenter::present` and `QuorraRasterizer::rasterize_frame` are both three
lines around one call to it, so the two window paths cannot come to disagree about when a frame
is the same frame.

### What `present` now does per frame

| the next frame | what happens |
|---|---|
| **unchanged** — same page `Arc`, placement, window size, medium, chrome | no `SceneBuilder`, no `Encoder` walk, no upload, no release, no eviction. quorra replays the encode: `FrameCost::encode_source` reads `Replayed`, `uploads` reads 0, `settle` reads zero |
| **the page changed** (a page turn, a re-interpretation) | the slot rebuilds: the *previous* scene's transients are released, the caches settle, `begin_frame` ticks, the lists are translated, the new scene is retained and encoded |
| **a zoom step, a scroll, a resize** | a rebuild, for the same reason — and quorra's own table says no design makes a zoom survive, because the transform's linear part is inside every atlas key and the sub-pixel phase is its fractional translation |
| **the chrome changed** (a selection dragged, the sidebar opened) | a rebuild. Chrome that was merely *rebuilt* — the host does that every frame — is not a change |
| **the CPU raster stands in for the page** | a rebuild, always. See below |

### Judgement 1 — the page is keyed by a *pinned* `Arc`, so `PresentFrame::page` changed type

The owner's sketch keys on "`Arc::as_ptr` of the `DisplayList`", and `PresentFrame::page` did not
carry an `Arc`: it was `Option<(&DisplayList, TargetSpec)>`. An address alone is the ABA bug
`crates/render-quorra/src/cache.rs` already refuses to have — drop a display list, let the
allocator hand the same address to the next page, and the key matches for a page that is not the
one it was taken from. This is not theoretical here: a page turn drops one `Arc<DisplayList>` and
allocates the next, at the same window size, the same placement and usually the same (absent)
chrome. Every other condition for a false hit is already met; only the allocator decides.

So the field is `Option<(&'a Arc<DisplayList>, TargetSpec)>` and `Retained` keeps a clone of it.
This is the crate's existing rule rather than a new one — `cache::Entry` holds "a clone of the
`Arc` it is keyed by" for the identical reason, and its module comment is the argument. Two
callers moved: `presentation.rs::frame_to_draw` hands its transition frame over in an `Arc`
(a fresh one per animation frame, which rebuilds, which is what a moving picture is), and
`transition_frames.rs` wraps its fixture. The two `rasterize_frame` users in `viewer-ui` already
held `Arc<DisplayList>` and did not move at all.

### Judgement 2 — the chrome is compared *by value*, and holding it is what keeps the reuse alive

The overlays have no identity to key on: `Overlays::of` rebuilds every one of them from the
host's own state on every frame, so their allocations are that frame's. An address key would
miss on every frame — which is exactly why quorra declined to put this cache inside the device —
or, worse, match a different list that landed on the same address.

They are therefore held by value in `Retained::overlays` and compared with `PartialEq`, which is
cheap: chrome is a selection's rectangles and a sidebar, not a page. Float equality can only err
towards rebuilding (`NaN` compares unequal), with the single exception of `-0.0 == 0.0`, which
draws the same pixels because every coordinate it produces differs only in the sign of a zero.

**The second reason is the one that decides it.** `cache::ResourceCaches::drop_unreachable`
releases an entry whose pin is the only reference left to that allocation. Overlays dropped at
the end of their frame would take their uploaded outlines with them — and a release bumps the
resource generation quorra keys every retained encode on, so the next frame would re-encode, and
the frame after that would do it again. The reuse would have defeated itself through the cache it
shares with the page. Holding the lists holds their outlines reachable, and the steady state
releases nothing at all.

### Judgement 3 — the transients belong to the *scene*, not to the frame

This is the correction the migration turns on and it is stronger than the owner's document's
item 1 states. That document says of a reused frame that "there is nothing to release — the
`transient` vector is empty, because nothing was uploaded — so the loop must simply not run,
which follows for free once the scene build is skipped." True, and not sufficient: it is the
**rebuild** frame's own release that is the problem. `Encoder` puts a resource on the transient
list for every clip outline it splits, every dashed stroke it flattens, every soft mask and every
image; releasing them at the end of the frame that built them leaves the retained handle holding
a scene whose resources are gone. The next re-encode of that scene would then be *refused by
name* — which is quorra's correct behaviour and a viewer that stops drawing.

So `Retained::transient` holds them, and they go back when the scene they belong to is replaced.
That also removes the wasted encode the naive order costs: releasing after the render bumps the
generation the just-stored encode was keyed under, so the frame after every rebuild would encode
again before anything could settle.

### Judgement 4 — the cache's frame clock counts *scenes*, and the eviction moved with it

`ResourceCaches` protects an entry that "this frame" looked up. A reused frame looks nothing up.
Advancing the clock on one would make the live page's own outlines evictable, and evicting them
would leave the retained scene naming released ids — the same refusal as judgement 3, arrived at
from the other side.

`begin_frame` and `evict_settled` therefore run on the rebuild path and on no other, which is
exactly what they meant back when every frame was a rebuild: the clock counts the scenes a
device has been asked to draw. The eviction also moved to the *front* of the rebuild, before
`begin_frame`, so that it settles with the clock still on the scene being replaced — identical
semantics to the end-of-frame call it replaces, and it leaves the resource generation still by
the time the new encode is keyed under it. `FrameCost::settle` is what it always was and is zero
on a frame that reused its scene.

The one cost of this is worth naming rather than discovering: a window that sits perfectly still
never evicts. That is correct — eviction exists to make room for resources a *new* scene needs,
and a window drawing the same scene needs none — but it means a budget is reclaimed on the next
page turn rather than on the next frame.

### The raster stand-in does not get an identity, and that is a deliberate refusal

The owner's checklist asks for the CPU raster to be cached by identity "like any other
resource". It cannot be, and the reason is upstream of this crate: `PresentFrame::raster` is
`&Raster` — bytes `on_the_processor` produced with `CpuRasterizer::rasterize` for *this* frame
and handed over by reference. There is no allocation to pin and no `Arc` to key on, and an
address would be the ABA bug judgement 1 exists to refuse, on a `Vec<u8>` that is freed and
reallocated at the same size every frame.

So `SceneKey::raster` is a serial rather than an identity: a frame carrying a stand-in is always
a new key and never replays. What that costs is one upload and one encode per fallback frame —
against a full CPU rasterisation of the whole page on the same frame, which is what the fallback
path *is*. The reuse worth having there is of the rasterisation, and that is `doc/todo/45`'s
open item about `render-cpu` having no per-frame resource cache; when a raster arrives with an
identity, this key is where it goes. Recorded as a choice with its cost, not taken silently.

### What was not done

**`QuorraRasterizer::rasterize` keeps `Device::render`.** The corpus gate draws each page once
and retention buys it nothing; leaving it alone also leaves every cross-backend verdict
comparable with the rounds before this one. It does now discard the retained window frame first,
because it evicts and an eviction could otherwise release ids that frame's scene still names —
three lines against a class of reasoning about an instance used both ways.

**`Options::instrument_encode` stays unused**, as ADR 0347 left it: a replayed frame's encode
subdivision is zero by construction, and there is nothing to attribute.

## Evidence

### It is byte-identical, and it misses on everything it must

`crates/render-quorra/tests/retained_frame.rs`, eight tests, on the real device rather than on a
model of it. quorra proved byte identity on its own archetypes; this proves it on the scene *this
crate* builds — medium, placed page, chrome — because that is the scene a window presents and
quorra's fixtures are not it.

- **byte identity**: a replayed frame against the encode it replaced, and against a cold device
  that retained nothing, `0` of the window's bytes differing in both.
- **one test per input, each asserting the miss *and* that what is drawn afterwards is the new
  page**: a different display list; the same picture at a new `Arc` (identity, not content); a
  resized window; a zoom step; chrome that changed. A miss that drew the right page for the
  wrong reason passes the first half alone, which is why the second half is there.
- **chrome merely rebuilt still hits**, which is the positive half of judgement 2 and the one an
  address key would fail.
- **a raster stand-in never replays**, three rounds of it, and the page **settles again** after a
  fallback frame rather than being poisoned by it.

### The frame's structure on `tmp/Entwurf.pdf`

Under `Xvfb` on `llvmpipe`, `--trace`, the release binaries of this tree with and without the
adoption — the same quorra revision in both arms, so the only variable is this change.
Structure only: the machine is shared and no wall clock from it is a claim.

The recipe: a 900×1100 `Xvfb`, the document opened, then 24 presses of `Up` with the page
already at the top — a clamped scroll, which is a frame the host asks for and which changes
nothing at all. 25 frames each, three runs an arm, alternating so drift falls on both.

**Run 1 of the `before` arm was a load spike and is kept rather than dropped**, because it is the
reason the rest is legible: its `execute` reads 106.6 ms against 26–28 in its own arm's other two
runs, which is the machine and not the change. The quiet runs are the two that follow it.

| median, ms | `before`, runs 2 / 3 | `after`, runs 2 / 3 |
|---|---:|---:|
| frame | 128.8 / 126.9 | **29.3 / 31.3** |
| — `scene` | 14.2 / 17.3 | **0.0 / 0.0** |
| — `device` | 113.6 / 109.6 | 29.3 / 31.3 |
| — — `encode` | 81.6 / 81.9 | **0.0 / 0.0** |
| — — `transfer` | 0.3 / 0.3 | 0.3 / 0.3 |
| — — `execute` | 28.0 / 26.2 | 28.0 / 29.9 |
| — — `elsewhere` | 2.4 / 2.3 | 0.8 / 0.9 |
| — `settle` | 0.8 / 0.7 | **0.0 / 0.0** |

**The prediction and the shape it predicted.** `doc/todo/44` §3 said that under full reuse "what
would remain is `execute` + `elsewhere` + `settle`", and that is exactly and only what remains:
`scene`, `encode` and `settle` are zero at the median, `execute` is unmoved, and the frame *is*
`execute` plus a fraction. Its ≈56–60 ms was arithmetic on the owner's RADV trace and this is
`llvmpipe`, so the number is a different machine's; the structure is the one it computed.
`execute` being unmoved is also quorra's own llvmpipe finding — "only the host term moves" — which
is what makes the residue believable rather than a measurement that lost something.

Three counters say the same thing without a clock:

- **`24 of 25 frame(s) replayed a retained encode`**, identically in all three `after` runs. The
  one that did not is the first.
- **uploads: 58 989 → 58 029**, which is 40 per steady frame going to none. The 58 029 both arms
  share is the first frame's, and it is the page.
- **the handle held at most 3 830 032 bytes** — this page's price for the reuse, and the number
  `doc/QUORRA_RETAINED_FRAME.md` section 6 asks a host to budget with. One handle, one visible
  frame.

**The launch table does not move, which the first frame reusing nothing requires.** Quiet runs of
each arm, the deltas in ms: `interpreted, 58009 cmd` 960 / 1008 before against 964 after,
`first scene built` +319 / +280 against +324, `first present` +657 / +641 against +640. Two of the
`after` runs were taken while the machine was loaded and say so in every row at once (one reads
`interpreted` at 4567 ms), which is why the comparison is between the quiet ones.

**One note on provenance.** The `after` binary of the table above predates a representation-only
change made afterwards — the page's address was held both in the key and in the pin, and the key's
copy went, so `Retained::page` is now the identity as well as the thing that makes it one. A
fourth `after` run on the committed binary reproduces every structural figure and every counter
exactly: `scene`, `encode` and `settle` zero at the median, 24 of 25 replayed, 58 029 uploads,
3 830 032 bytes retained. Its wall clock is a loaded machine's and is not quoted, which is the
same reason `before` run 1 is not.

### The four quorra lanes, which a round taking a release owes (ADR 0283)

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| scale 1, `cpu` | 934 | 20 | 2 | 18 |
| scale 1, `gpu` | 933 | 21 | 2 | 18 |
| scale 4, `cpu` | 936 | 10 | 5 | 23 |
| scale 4, `gpu` | 937 | 9 | 5 | 23 |

**Every cell is ADR 0347's at `87898c69`**, which is what the two pixel-moving commits in the
range predicted from their own side: `6b75e00`'s knockout-stroke fix reaches a shape this corpus
does not contain (upstream had to build the fixture rather than find it), and `a85cc47`'s
rectangle lane moves five pages in 951 by a mean of 0.0001–0.0021 with every worst tile unchanged
— below this gate's own resolution. The lanes were run because a release can live entirely inside
one of them, not because a movement was expected.

The corpus gate here draws each page through `QuorraRasterizer::rasterize`, which still uses
`Device::render`, so these four rows are also the control: they say the *rest* of the crate did
not move while the frame path did.

### What did not move

`cargo fmt --all --check`, `clippy --workspace --all-targets` silent, `cargo nextest run
--workspace` (every test, the eight new ones included), `cargo test --workspace --doc`,
`conformance` — which earned its place in this round by catching a `§` pointed at
`QUORRA_RETAINED_FRAME.md` instead of at the standard — and the corpus, oracle, both text gates,
dates, XMP and JPEG 2000 gates, all green.

## Consequences

The window now has a state that can be stale, where before it had none — `FrameSlot` is the only
place in this crate that can draw a page other than the one it was handed. That is why the key
is enumerated rather than discovered, why every entry of it has its own test, and why
`FrameCost::encode_source` is in the frame line of the trace: a frame loop that means to reuse
and does not is legible as itself, and so is one that reuses when it should not have.

`doc/todo/44` §3 closes with this. What is left of that file is its §2, which is already taken,
and the note that the remaining `scene` cost across zoom steps is the page-space construction —
still available, still needing nothing from upstream, and still not the case the trace is full
of.
