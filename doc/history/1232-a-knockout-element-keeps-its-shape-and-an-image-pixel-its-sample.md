# 1232 — A knockout element keeps its shape, and an image pixel its sample

Date: 2026-09-23. Branch `batch-1227-1232`, shared worktree. ADRs 1301, 1302.

## What the brief said, and what the tree said

ADR 1279 had closed the product residue for an *image*. It had not closed it for a stencil painted
through a pattern, which becomes a mask on a fill. Reading the other remainders against the tree
turned up two more wrong pixels, both silent: a `/AIS` record that `Q` did not restore, and one
that could forget a reading.

## Built (ADR 1301)

- A stencil through a tiling pattern keeps its mask in the group's shape. A stencil under its own
  `/SMask` through a pattern is drawn through the product and knocks out through the stencil alone
  (`ShapeMasks::record_apart`).
- §11.7.4.4's group inside an isolated knockout group goes on NOTE 6's transparent backdrop with
  its blend modes dropped (§11.3.6). It was refused before.
- `Q` restores §11.6.4.3's reading. A record of both readings is never replaced. A `B`'s portions
  are read where they are painted.

## Built (ADR 1302), the one-pixel shift

The CPU oracle was right by §10.7.4 and §8.9.4 and did not change. The device was wrong twice. Its
copy of `smoothed` lacked the native branch, so it filtered at one pixel per sample (128 levels).
Its nearest lookup broke ties by float error (239 levels, magnified). The fix is the native branch
plus a device → texel transform composed in f64.

## Rows

§11.3.7.2, §11.3.7.3 and §11.4.4 went `partial` → `implemented`, and §11.3.7 followed. §11.4.6 and
§11.7.4.4 stay `partial`, narrowed (ADR 1301, Consequences). §11.4 stays `partial` because of
§11.4.3, §11.4.6 and §11.7.4.4.

## Gates

`raster_golden` holds 974 and moved 0. See the report for the rest.
