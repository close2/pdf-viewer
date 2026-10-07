# 1619 — The regions floor stands on a long-lived device, and the page's next exact lever is its copies

Status: accepted. Session 1391. Re-asks ADR 1582 section 4 (the region fills' floor) now that
`turn_path` gates a long-lived device's seventh frame (ADR 1607); answers it by measurement and
builds nothing. Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("decided by measurement, never by assumption"); ADRs 1582,
1606, 1607, 1618; `doc/habits/measuring.md` 68 and 70.
Code: none.

## 1. The question

ADR 1582 ranked what is left of `bug1721218_reduced.pdf`'s walk by the clock on a fresh device —
the region fills 9.1 ms, the chain's edges 4.7, the links' flattening 4.4, the rest under 2 each —
and called the fills a floor: the one cheaper construction measured, a dot's coverage taken as a
polygon at 1.20 µs against the fill's 1.50, is not byte-identical by construction. A device a
window keeps has drawn other frames first, and a cost that depends on what the device holds would
rank differently there. Does it?

## 2. Measured: the seventh frame adds no stage

`frame_budget`, one sitting, the tree of ADR 1618, three runs of five rounds, pinned, load 2.0 to
2.6, minima, ms:

| | total | interp | scene | encode | transfer | elsewhere | execute |
|---|---:|---:|---:|---:|---:|---:|---:|
| turn, a fresh device | 151.86–153.06 | 67.69–68.46 | 81.09–81.60 | 1.38–1.39 | 0.44–0.46 | 0.78–0.93 | 0.27–0.52 |
| seventh, a device that drew six | 151.16–152.37 | 68.16–69.36 | 79.81–80.31 | 1.38–1.40 | 0.44–0.47 | 0.86–0.89 | 0.30–0.44 |

The seventh frame is the quicker by about 1.3 ms, all of it in `scene`: the first uses a fresh
device pays inside the group's two renders, ADR 1606's command-buffer allocations among them. Every
other stage is inside its runs' spread. Section 3e's two rows that stood 7 ms apart, the seventh
the slower, were taken in two sittings; ADR 1607's own sitting read the two ranges overlapping, and
this one reads the seventh frame the quicker.

**So the ranking does not change.** The region fills, the chain's edges and the flattening are the
walk's work on the frame's threads — the same edges in, the same coverage bytes out — and nothing
a device has drawn before reaches them; what does reach the frame through the device is the
first-use cost, which only the fresh device pays and which is smaller than any of them.

## 3. What is left, by the clock, and the decision

- **The polygonal fill is not built.** Its saving is a fifth of 9.1 ms, about 1.8 ms a render on
  either kind of device, and it is still not byte-identical by construction; ADR 1582's floor
  stands on the long-lived device as on the fresh one.
- **The passes are the exact lever left in raster's frame.** ADR 1618 measured 20 to 45 µs a pass
  on this driver; the turn still records 193, and 58 of them are the backdrop copies of its two
  group renders. A copy as `copy_texture_to_texture` is byte-identical by construction and costs
  one command buffer where a pass costs two and no render pass at all, so it is worth up to about
  2.6 ms a turn — in the polygonal fill's range, without its inexactness. It is priced in ADR 1618
  section 3 and owes one measurement first: what `COPY_SRC | COPY_DST` on every layer texture costs
  on RADV.
- **The page's largest stage is not raster's.** `interp` is 68 ms of the 152, this tree's
  interpretation of the page's 3 518 clipped shadings; a raster round has no lever on it.
