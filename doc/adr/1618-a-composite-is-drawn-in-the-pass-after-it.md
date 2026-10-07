# 1618 — A composite is drawn in the pass of the marks after it, and a frame's end is stamped on its last pass

Status: accepted. Session 1391. Answers ADR 1606 section 4 ("[f]ewer passes a group render is the
lever"). Supersedes nothing; amends ADR 0038 (raster's) in one respect: a composite is a draw
carried to the next pass onto its accumulator, not a pass of its own.
Context: ISO 32000-2 §11.3.6, §11.4.4, §11.4.5; `CLAUDE.md` principle 2; ADRs 1471, 1529, 1594,
1606, 1607; `doc/habits/measuring.md` 63; traps 50, 109, 114, 118.
Code: `raster/crates/raster-gpu/src/compose.rs` (`Executor::render_plan`'s pending composite,
`Executor::closing`, `pass_stamp`, `scissor_rect`), `compose/child.rs` (`Composite`,
`composite_child`, `draw_composite`, `composite_pass`), `compose/draw.rs` (`draw_pass`'s
`composite`, `PassLoad::op`), `compose/blit.rs` (`copy_pass` of a source nothing has written),
`device/record.rs` (the closing pass of each route), `layers.rs` (the reuse argument).
Tests: `raster/crates/raster-gpu/tests/a_composite_shares_the_pass_after_it.rs` (new, three tests,
each watched failing); the corpus gate's per-page digests on six arms.

## 1. Counted first (habit 63): what ends a pass on the §3e pages

Every `begin_render_pass` of `raster-gpu`'s frame logged by label (scratch, removed since), the
first frame of `zoom_frame` on HEAD. `bug1721218_reduced.pdf`'s frame is three raster frames: the
chromatic and the black frame of its four-component group (ADRs 1471, 1529) and the page frame
that places the result back. A group render records 107 passes, 108 where the timestamp query is
home, and the page frame 7 — **222 in all**, where ADR 1606's count of the same frame read 221:

| what ends the pass before it | a group render | why it is a pass |
|---|---:|---|
| a child plan's own content (29: the group and its 28 children) and a soft mask's group (3) | 32 | another attachment, the child's texture |
| the backdrop copy of each composite | 29 | another attachment, the copy (ADR 0038) |
| the composite itself | 29 | back onto the accumulator |
| **the parent's run of marks after a composite** | **12** | **none: the same accumulator, loaded** |
| **an empty pass clearing an accumulator whose first op is a child** | **1** | **none: the copy then reads zeros** |
| a mask's reduction to R8 | 3 | another attachment, the mask |
| the root onto the target | 1 | another attachment, the target |
| **the end timestamp** | **1** | **none: the target again, drawing nothing** |

No pass on the page ends for a blend mode (modes 8 and 9 are `composite.wgsl`'s), a knockout
stage (erase and add interleave inside one pass, ADR 0010), a clip (a scissor or a coverage tile
inside the pass) or a barrier (`wgpu` inserts those between passes; none asks for one). The other
§3e pages: the text page, the photograph, the `/Matte` page, the two stroke pages, `issue19802.pdf`
and the Type 3 page are flat, **2 passes** — the content and the end stamp; `images.pdf` 6 (one
child, its run first); `personwithdog.pdf` 28 (eight children, one run after a composite);
`22060_A1_01_Plans.pdf` 42 (ten children, nine runs after a composite).

## 2. What is merged, and why each merge keeps the bytes

The three bold rows are passes whose boundary has no cause: the pass before and the pass after
write the same attachment and nothing between them reads it.

- **A composite and the run after it are one pass.** `composite_child` records the backdrop's copy
  and hands back a `Composite` — its pipeline, its bind group, `onto`, and the textures it reads —
  which `render_plan` gives to the next `draw_pass` onto the accumulator, drawn there first under
  its own scissor and followed by the marks under the pass's; a child or the plan's end records it
  in a pass of its own, because the next child's copy reads what it wrote. **Why the bytes are the
  two passes'**: a pass's draws are rasterised and blended in recording order, as consecutive
  passes are; the composite reads the copy, the child, the group alpha, a mask and the scratch
  sheet, and the marks read the atlas, the scratch sheet, masks, ramps and images — never the
  accumulator, which `wgpu` would refuse as a binding of its own pass; and an `Rgba8Unorm`
  attachment stored by one pass and loaded by the next carries its bytes unchanged. So each pixel
  meets the same writes in the same order from the same value. The scissor is set before each
  draw as each pass set it: the composite's `onto ∩ damage`, then the marks' damage box or the
  whole attachment, which is the default a pass begins with.
- **An accumulator nothing has written is cleared by its composite's pass.** Where an isolated
  child is a plan's first op, the copy pass clears its texture and draws nothing — the zeros a
  blit of a cleared accumulator would copy — and the pass carrying the composite loads with
  `Clear`, which clears the whole attachment whatever the scissor, as the empty pass did. A
  non-isolated child's backdrop is read by its seed copy before any composite, so that case keeps
  its empty pass; a child that meets its parent nowhere leaves the accumulator unwritten, for the
  next run or the plan's end to clear.
- **The end timestamp is written by the frame's last pass.** `Executor::closing` is set before the
  root's blit, the damage patch or the flat route's one pass, and `pass_stamp` gives that pass
  `end_of_pass_write_index`; `execute` still spans the first pass's beginning to the last pass's
  end. Only a frame whose every damage rectangle falls outside the target, which records no pass,
  keeps the empty one.

After: a group render records **94**, the page frame 5, the frame **193 of 222**; every flat page
**1** of 2; `personwithdog.pdf` 26, `22060_A1_01_Plans.pdf` 32, `images.pdf` 5. Each removed pass
is two of `wgpu`'s command buffers and a `vkCmdBeginRenderPass` (ADR 1606, trap 118).

## 3. Priced and not built

- **The backdrop copy as `copy_texture_to_texture`**, 29 passes a group render: a transfer is
  recorded into the open HAL command buffer, one where a pass takes two, and copies exactly. It
  is not a merge — the copy's attachment is its own — and it needs `COPY_SRC | COPY_DST` on every
  layer texture and the damage scissor stated as the copy's rectangle, which `blit.rs` gives as
  its reasons for the blit; what those usages cost on this driver is unmeasured. Left priced.
- **Consecutive composites whose `onto` rectangles stand apart**, one pass: on this page 16 pairs
  of composites follow each other onto the group with no run between, 11 of them disjoint, so a
  later child's copy could be taken before the earlier composite is drawn and the two drawn
  together. It keeps the bytes for the same reason as above, and it keeps the earlier child's
  texture and copy alive while the later one renders — two textures past the peak
  `peak_layer_bytes` prices, which the frame budget is refused against. Not built: it moves the
  budget's arithmetic for up to 22 passes a frame.

## 4. Measured

Exports of HEAD and of HEAD with this change, each in its own target directory, sources touched
before building, every binary `md5sum`-distinct (traps 50, 114); behind the lock, 2026-10-07.

**Corpus.** `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×, the
sandbox worker rebuilt inside the lock (trap 109), per-page digests compared by name: **0 pages
moved on any of the six arms** (968, 964, 968, 963, 968, 964 compared), verdicts equal arm for arm,
one-versus-many 0.

**Clock**, pinned to the performance cores, load 2.0 to 2.6, three runs a tree interleaved:

| `bug1721218_reduced.pdf` | HEAD | this change |
|---|---:|---:|
| `frame_budget` turn, minima of 5 | 153.10–154.33 ms | **151.86–153.06 ms** |
| `frame_budget` seventh | 152.04–153.14 ms | **151.16–152.37 ms** |
| `frame_budget` step | 84.81–85.59 ms | **84.08–84.30 ms** |
| `zoom_frame` 1× frame, fresh device | 78.7–79.5 ms | 78.6–80.4 ms |
| `zoom_frame` 1.25× step, fresh device | 76.9–77.6 ms | 76.3–77.8 ms |
| `zoom_frame`, one device's seventh frame | 69.5–70.5 ms | 68.8–70.8 ms |

Every `frame_budget` run of the change is quicker than the HEAD run beside it, by 0.6 to 1.4 ms,
all of it in `scene`, where the group's two renders are recorded: 20 to 45 µs a pass on this
driver, the 29 passes a frame no longer begun. `zoom_frame`'s pair moves inside its spread. It is a
small saving taken because it costs nothing — no byte, no texture, no pipeline — and it is the
measure ADR 1619 prices the rest of the page against.
