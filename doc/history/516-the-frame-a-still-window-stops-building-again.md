# 516 — The frame a still window stops building again

**Finding.** quorra's retained frame, taken. Upstream's head moved eight commits past the pinned
`87898c69` to `580fa4ac`, carrying `RetainedScene` and `Device::render_retained` (their ADR 0048)
— the first quorra release this tree has had to *adopt* rather than merely take, and the bump
itself still cost no line: it compiled and linted clean before anything was reached for.
`render-quorra`'s frame path now keeps the frame's scene across frames in one `FrameSlot`, keyed
on the page display list's `Arc` identity and placement, the window, the medium, and the chrome
**by value**; a frame that changed in none of those builds no `SceneBuilder`, walks no `Encoder`,
uploads nothing, releases nothing and evicts nothing, and quorra replays the device commands it
made earlier. On the owner's `tmp/Entwurf.pdf` a still window's median frame goes from `scene`
14–17 + `encode` 82 + `settle` 0.8 + `execute` 26–28 to **`execute` and nothing else**: `scene`,
`encode` and `settle` all zero at the median, 40 uploads a frame going to none, 24 of 25 frames
replaying, `execute` unmoved — which is exactly the residue `doc/todo/44` §3 computed and exactly
quorra's own llvmpipe finding that only the host term moves. The launch table does not move,
because the first frame reuses nothing.

**Three things the migration document did not fully have, and one it asked back.** Two of them
would have shipped a viewer that stops drawing rather than one that draws slowly, and
`Frame::encode_source` is what found both. (1) The page's identity had to become *pinnable* —
`PresentFrame::page` now carries `&Arc<DisplayList>` — because a page turn drops one display list
and allocates the next at the same window size and the same placement, so only the allocator stood
between an address key and a stale page. (2) A rebuild frame must **not** release its own
transients: the retained handle names them until something replaces its scene, and the next
re-encode of a scene whose resources are gone is refused by name. (3) The caches' frame clock now
counts **scenes** rather than frames, because a reused frame looks nothing up and would otherwise
make the live page's own outlines evictable — the same refusal reached from the other side. And
quorra's own question — do the overlays change on frames where the page does not? — is answered in
`QUORRA_FEEDBACK.md` §23.3: one real case, a dragged selection, bounded by a person's hand, and
this tree is not asking for fragment composition until somebody puts a number on it.

**Date.** 2026-08-14.
**ADR.** [0351](../adr/0351-the-frame-a-still-window-stops-building-again.md).

**Code.** `crates/render-quorra/src/present.rs` (`SceneKey`, `Retained`, `FrameSlot`; `present`
is now three lines around one call), `crates/render-quorra/src/lib.rs` (`rasterize_frame` uses
the same slot, so the two window paths cannot disagree about when a frame is the same frame;
`rasterize` discards it first because it evicts), and one new test file,
`crates/render-quorra/tests/retained_frame.rs` — eight tests on the real device: byte identity
against the encode replaced *and* against a cold device that retained nothing, then one test per
key entry, each asserting the miss **and** that what is drawn after the change is the new page.
A miss that drew the right page for the wrong reason passes the first half alone.

**Touched.** `doc/adr/0351-*` (new), `doc/QUORRA_UPGRADE.md` (the `580fa4ac` section and its
eight-commit range — two of them move pixels and neither moves this tree's, which upstream
measured on this corpus before publishing), `doc/QUORRA_FEEDBACK.md` §23 (one correction to their
§3, one declined checklist item with its reason, and the answer their §5 asked for),
`doc/todo/44` (status: taken; §3.2 is what it did), `doc/todo/45` (item 3's "what is still ours"
is done — and its own 38-page-turn witness is untouched by this, because a page turn is a rebuild
by construction), `crates/viewer-ui/src/bin/pdf-viewer/timing.rs` (`encoded`/`replayed` in the
frame line, the replay count and the retained bytes in the summary), `.../presentation.rs` and
`.../surface.rs` and `crates/viewer-ui/tests/transition_frames.rs` (the `Arc` the pin needs),
`Cargo.lock`, this file.

**What it does not do.** The CPU raster stand-in still never replays, and that is a refusal with a
reason rather than an omission: `PresentFrame::raster` is bytes the processor made for *this*
frame and handed over by reference, so there is no allocation to pin and an address would be the
ABA bug the rest of this change exists to refuse. The reuse worth having on that path is of the
rasterisation itself, which is `doc/todo/45`'s open item about `render-cpu` having no per-frame
resource cache. And `QuorraRasterizer::rasterize` — the corpus gate's path — keeps
`Device::render`: it draws each page once, retention buys it nothing, and leaving it alone leaves
every cross-backend verdict comparable with the rounds before this one.
