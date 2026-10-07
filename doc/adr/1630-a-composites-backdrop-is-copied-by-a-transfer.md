# 1630 — A composite's backdrop is copied by a transfer, and the copy usages cost nothing on RADV

Status: accepted. Session 1397. Builds what ADR 1618 section 3 priced and left ("[t]he backdrop
copy as `copy_texture_to_texture`"), after measuring the condition it named. Supersedes nothing;
amends ADR 0038 (raster's) in one respect: the backdrop a composite reads is copied out of the
accumulator by a transfer, not by a `blit.wgsl` pass.
Context: ISO 32000-2 §11.3.6, §11.4.5; `CLAUDE.md` principle 2; ADRs 1606, 1618, 1619;
`doc/habits/measuring.md` 63 and 68; traps 50, 109, 114, 118, 121.
Code: `raster/crates/raster-gpu/src/layers.rs` (`LAYER_USAGES`), `device/textures.rs`
(`create_internal_texture` takes its caller's usages), `compose/child.rs` (`composite_child`,
`transfer_backdrop`), `compose/blit.rs` (`copy_pass`: the seed, and the clear of a backdrop that
has no source), `shaders/blit.wgsl`, `device/binds.rs`.
Tests: `raster/crates/raster-gpu/tests/a_backdrop_is_copied_by_a_transfer.rs` (new, two tests,
each watched failing with the transfer's source origin set to zero); the corpus gate's per-page
digests on six arms.

## 1. The condition, measured first: what the two copy usages cost

ADR 1618 left the lever on one unknown: a transfer needs `COPY_SRC` on the accumulator and
`COPY_DST` on the copy, and the pool hands one texture out for either role, so every layer texture
carries both. On some drivers a transfer usage changes how an image is laid out or compressed.
A probe (scratch, removed) made a `Rgba8Unorm` texture with and without the two usages and drew
into it as a frame does: a pass clearing it and drawing a full-screen triangle, a later pass
loading it and blending another, and a pass onto another target sampling it by `textureLoad`.
Two exports, identical but for the usage constant, each in its own target directory and
`md5sum`-distinct (traps 50, 114), run interleaved, eight runs of 300 iterations each, pinned to the
performance cores, on the 890M through RADV; medians of the per-run medians, microseconds:

| 1600 × 1000 | without | with |
|---|---:|---:|
| `create_texture` and its view, host | 1.92 | 1.95 |
| the first pass, device (timestamps) | 38.63 | 38.71 |
| a later pass, device | 40.92 | 41.04 |
| the sampling pass, device | 48.25 | 47.97 |
| `CommandEncoder::finish`, host | 10.42 | 10.54 |
| `Queue::submit`, host | 11.70 | 11.78 |

At 160 × 120 every row agrees to 0.04 µs. Callgrind on the same two binaries, differencing 100
and 400 iterations: **225 462 and 224 857 host instructions an iteration**, and
`Device::create_texture` inclusive 4 991 929 against 4 990 991 over 411 textures. And on the page:
`frame_budget` on an export with the usages and nothing else, interleaved with HEAD six times,
read turn 150.81–154.15 ms against HEAD's 152.10–159.64, the pairs from +0.49 to −2.41 ms and one
at −5.49 where HEAD's run was the loaded one, load 5.1 to 7.7. **The usages cost nothing this adapter can show**, which is the
measurement ADR 1618 asked for.

## 2. What is built, and why the transfer sees the bytes the blit saw

`composite_child` copies `onto` (`child ∩ parent`, in device space) out of the accumulator with
`copy_texture_to_texture`: from the texel `onto − region` of the accumulator, to the origin of a
texture acquired at exactly `onto`'s size, `onto`'s extent. A backdrop whose accumulator nothing
has written keeps ADR 1618's cleared copy, a pass that draws nothing.

- **The same bytes.** The blit's fragment at `p` loaded the accumulator at `p + (onto − region)`
  and stored it unchanged into an `Rgba8Unorm` attachment, which is the identity on 8-bit unorm;
  a transfer copies texels without conversion by definition. Both read after the accumulator's
  last pass, which ends with `StoreOp::Store`. Between that pass and the transfer `wgpu`-core
  moves the accumulator from colour attachment to copy source, and between the transfer and the
  pass that samples the copy it moves the copy from copy destination to sampled texture
  (`set_single` and `transition_textures` in `command/transfer.rs`), each a barrier that makes
  the earlier writes visible to the later reads. Passes and transfers in one encoder execute in
  recording order, so the order of every read and write is the blit's.
- **The same rectangle.** `onto` lies inside the accumulator (it is `region ∩ child`) and the
  copy is `onto`'s size, so the transfer is the blit's rectangle with no scissor: where a frame
  patches a damage list (raster's ADR 0012) the blit wrote `onto ∩ damage` and cleared the rest, the
  transfer writes all of `onto`, and the composite that reads the copy is scissored to
  `onto ∩ damage` and reads at its own fragment — so no texel outside it is read. The texels
  outside are the accumulator's own, cleared by its first pass. A transfer of less than the whole
  copy would cost a clear: `wgpu` tracks initialisation per subresource, and
  `handle_dst_texture_init` clears a destination a transfer only partly covers.
- **What the frame no longer pays for each copy**: a render pass (two of `wgpu`'s command buffers,
  trap 118), a uniform buffer and its write, and a bind group.

## 3. Counted (habit 63): what the page's turn records

Every `begin_render_pass` and every transfer logged by site (scratch exports, removed), the turn
on `frame_budget`'s lane after the warm-up page:

| `bug1721218_reduced.pdf`, the turn | HEAD | this change |
|---|---:|---:|
| render passes | 193 | **136** |
| backdrop copies with a source | 57 passes | 57 transfers |
| backdrop copies of an unwritten accumulator | 2 passes | 2 passes |

ADR 1618 counted 29 copies a group render; the turn has 59, 29 for each of the group's two
renders and one where the page frame places the group back. `personwithdog.pdf` 26 → 18,
`22060_A1_01_Plans.pdf` 26 → 18, `images.pdf` 5 → 4; the seven flat pages stay at one.

## 4. Priced and not built: the seed

§11.4.4's seed (raster's ADR 0019) is the other blit between two layer textures, and a transfer would keep
its bytes by the argument of section 2: the seeded plan takes its parent's region, so the copy is
the whole texture. It is not built because **no page `doc/performance.md` section 3e times seeds a
group** — counted 0 on all eleven — so its saving is unmeasured; the usages it needs are on every
layer texture now, which makes it a change of one call for the round that times such a page.

## 5. Measured

Exports of HEAD, of HEAD with this change, and of HEAD with ADR 1631 beside it, each in its own
target directory, sources touched before building, every binary `md5sum`-distinct (traps 50, 114);
behind the lock, 2026-10-07.

**Corpus.** `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×, on the
export of HEAD and on the export with ADRs 1630 and 1631 together, the sandbox worker rebuilt
inside the lock (trap 109), per-page digests compared by name: **0 pages moved on any of the six
arms** (968, 968, 968, 964, 963, 964 compared), verdicts equal arm for arm, one-versus-many 0.

**Clock**, `frame_budget` on the page, eight runs of five rounds a tree interleaved, pinned to the
performance cores with eight rayon threads, load 1.0 to 3.2, minima, ms:

| `bug1721218_reduced.pdf` | HEAD | this change | the pair, this change less HEAD |
|---|---:|---:|---:|
| turn | 151.68–153.85 | **149.77–151.95** | −0.29 to −2.44 |
| seventh | 150.66–152.01 | **148.62–150.76** | −0.22 to −2.83 |
| step | 83.05–84.49 | 82.73–84.10 | +0.13 to −1.72 |

Read per interleaved pair (trap 121): the turn and the seventh frame are quicker in all eight, the
step in seven, all of it in `scene`, where the group's two renders are recorded. **Callgrind** on
the turn (collection inside `rasterize_frame`, the warm-up's one flat pass and the turn): `Executor::render_plan` 23 351 133 → 22 402 956
instructions, `CommandEncoder::finish` 17 782 809 → 17 002 589 — of which the HAL's
`begin_render_pass` 2 459 394 → 1 741 314, against 1 547 786 for the 57 transfers RADV records
(about 27 000 instructions each, twice a pass's beginning) — and `Queue::submit` 4 606 843 →
4 080 209, the command buffers of 57 passes fewer. The transfer is not free on the host; it is
cheaper than the pass, the blit's uniform and bind group, and the command buffers it replaces.
