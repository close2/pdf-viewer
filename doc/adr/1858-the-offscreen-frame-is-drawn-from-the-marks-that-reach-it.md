# 1858 — The offscreen frame is drawn from the marks that reach it

Session 1510. Status: **accepted** and **built**. ADR 1841 §3 left open why a device frame
depends on marks that draw nothing in it, and called it raster's question; this reads and answers
it. Context: ADRs 0082, 1841, 1857; raster's ADRs 0015, 0028, 0029; traps 13, 66. Code:
`crates/render-raster/src/lib.rs` (`render_whole`). Instrument:
`crates/render-raster/examples/offscreen_cull.rs`. Clause: §10.7.4. No ledger row moves.

## 1. The hypothesis, and what held instead

ADR 1841 culled the offscreen path. One corpus page then moved on the gpu lane, `bug946506.pdf`, 254
of whose 462 commands lie off its page: oracle error 0.1101 → 0.1141 at 1× and 0.0324 → 0.0351 at
4×. So the cull was taken back out. The brief's hypothesis was that marks drawn beyond the page
edge conflate into the same tiles. **It did not hold.** Raster already drops those 254 commands
before any geometry (`commands_culled` reads 254 for the whole list), so no tile ever meets them.
`offscreen_cull` draws the page's whole list and its culled list on fresh devices, lane by lane.
`Cpu` and `Compute` give the same bytes. `Gpu` differs in 802 pixels by up to 22 levels at 1×,
and in 16 550 by up to 38 at 4×. Every one lies on the edge of a glyph that is still on the page.

**What decides it is raster's census** (`census.rs`, raster's ADR 0029). Before choosing a lane
for a solid fill, the encoder asks whether the scene places that shape exactly once, so that an
atlas entry would be written and read one time each. The count runs over the scene's commands,
culled or not. A glyph drawn once on the page and once beyond its edge counts two. It goes to the
atlas, whose tile is the processor's exact coverage. Culled first, it counts one and is drawn on the
gpu lane's own coverage, which is the lane's stated difference (ADR 0082). At the differing pixels
the whole list's frame is 0.07 levels from the `Cpu` lane's and the culled one 6.3. Planted:
`placed_once` made to answer `false` gives the whole and culled frames the same bytes at 1× and
at 4×. Reverted. So no pixel came from the off-page marks. §10.7.4 paints a pixel only where the
shape's area meets it, and a mark that meets no pixel of the target leaves it unmarked under every
reading. What moved was which lane drew the glyphs the cull left.

## 2. The decision

**`render_whole` draws from `DisplayList::reaching(target)`**, as the window's scene already does
(`present.rs`'s `culls`). The census then counts the placements this frame will draw. That is the
quantity the atlas question is about, so the count is truer, not merely different. The frame drawn
offscreen and the frame a window draws of the same marks now ask the census the same question.
Every list a page is drawn from has the page's geometry: its §11.4.7 plane of black, its §10.8.3
spot planes, its §11.7.5.2 shape lists. Each loses the same commands, so the passes stay in step.
A list `reaching` declines is drawn whole, as before. So does a page whose every command reaches.

The census itself is left counting a scene's commands. Counting only those that reach the
viewport is the same answer for a window, whose scene is culled already. Only a page-space scene
that reads no view differs, and there the next view may need what this one culled.

## 3. Evidence

The six corpus arms against HEAD's (`/home/AI/arms-1509/`), by page digest: see this session's
record. The only page the cull moves is `bug946506.pdf` on the gpu lane, by the figures in §1. The
others that move are ADR 1857's dots. `offscreen_cull` is the instrument; its planted census is the
control (trap 13).

## 4. What is left

The group bound (`check_group_blit`) still reads the whole list, so a page whose costly group lies
wholly off the target is refused as before. A tiled frame (`tiles.rs`) now culls
each tile through this path, and nothing has timed it.
