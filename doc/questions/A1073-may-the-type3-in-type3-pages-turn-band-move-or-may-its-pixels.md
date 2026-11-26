Status: complete
Given: 2026-11-26, in conversation — transcribed by the round
Owes: move ContentStreamNoCycleType3insideType3.pdf p1's turn band to match the true 12,224-command exact construction (option 1); then run a measurement/pricing round for option 3 (device-pixel-only flattening below a small device size, relaxing RELATIVE_FLATTEN_TOLERANCE's 16-segments-per-turn floor on sub-pixel curves) reporting speedup, pixel deltas, and golden impact before committing pixel changes

> Q1073: agreed: 1 now + price 3 next.

Reading:
Option 1 now, and price Option 3 next:
1. Move `ContentStreamNoCycleType3insideType3.pdf` p1's turn band to what the exact 12,224-command construction costs (the 20–33 ms band was measured against the pre-ADR-3653 buggy 152-command render), keeping all pixels unchanged for now.
2. Next, price Option 3 (flattening curves by device-pixel error alone below a small device size, rather than forcing `RELATIVE_FLATTEN_TOLERANCE`'s 16 pieces per turn on sub-pixel curves that expand a two-pixel glyph into ~1,200 pieces) — measuring the speedup, visual/channel pixel deltas, and golden impact before deciding whether to land it (rather than pricing Option 2's area-meet approximation).
