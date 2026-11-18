Status: complete
Given: 2026-11-18, in conversation — transcribed by the round
Owes: honour a shading dictionary's /AntiAlias true across render-cpu, render-raster, and render-gpu by filtering the shading function over sub-pixel samples of the pixel's footprint in shading space (Types 1 to 3, and mesh Types 4 to 7 where a function is present), recording the chosen filter in an ADR, marking ledger row 8.7.4.3 partial until built, and adding a stepped-function test that fails if /AntiAlias true is ignored (a render round)

> Q899: agreed

Reading: adopts option (a) — `/AntiAlias true` in Table 77 (§8.7.4.3) is honoured across `render-cpu`, `render-raster`, and `render-gpu` by filtering the shading function over sub-pixel samples of the pixel's footprint in shading space (for Types 1 to 3, and for mesh Types 4 to 7 where a function is present), with the filter kernel documented as a choice in an ADR, ledger row 8.7.4.3 marked `partial` until built, and a stepped-function test proving the flag is honoured.
