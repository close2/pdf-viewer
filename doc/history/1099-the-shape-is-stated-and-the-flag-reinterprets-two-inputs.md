# 1099 — The shape is stated, and the flag reinterprets two inputs

2026-09-15. ADR 1113. Files: `pdf-model/src/content/{transparency,path,pattern,text}.rs`, `content.rs`,
`tests/transparency_groups.rs`, `examples/group_shape_census.rs`, five ledger rows. `render-cpu`, `pdf-render`
and `raster` needed no line of this.

**The distinction is already carried, so no channel was added.** §11.4.6 names itself the only reader of a
shape apart from an opacity, and it reads a *stated* one: `Command::Shaped` (ADR 0234) is a per-pixel `f`, per
element, materialised only inside a knockout group — at a fraction of ADR 1022 §7's price for a channel.
§11.7.5.2's transfer identity is that cost for another quantity, population still zero, named on `doc/todo/13`
§3.

**Census first (trap 8), and it is why nothing was built.** `group_shape_census` gained §11.4.6's half.
`doc/pdf.js`, 963 opened first pages: **33 knockout groups on 18 pages, 5 holding a stated shape on 5 pages, 0
on the group's own backdrop, 0 refusals**; `doc/corpora`'s 487: 6 on 5 pages, 0 and 0. Calibrated by planting
`stated_shape`'s image arm away: the sweep names `knockout_groups_test.pdf` as a stencil under its own
`/SMask`, and 33 becomes 31. A crawl run was stopped (a `pdf-sandbox` zombie leak) and is unsummable.

**What moved is a guard, not a construction.** `/AIS` is a graphics state parameter, so a group's run can
paint under both readings, and `AlphaSourcesSeen::Mixed` was read as "no reading describes this group".
§11.6.4.3 and §11.6.4.4 give the flag exactly two inputs to reinterpret and §11.6.4.2 fixes the third at 1.0,
so a group stating neither has one shape per element under **both** and is described by both. `settled_over`
asks the elements with `group_alpha_is_shape`'s predicate under `Opacity`: ADR 0554's shape one level down.

**The fixture's numbers are the formulas'.** Two opaque fills, `/AIS` both ways, the upper under `/BM
/Multiply` so §11.4.6 can show at all. §11.3.6 gives no blend effect against §11.4.5's transparent initial
backdrop, so the overlap is **(0, 0, 255)** against the flat **(0, 0, 0)**; planted back to `settled` it reads
(0, 0, 0) and reports, and its control, the same group with `ca 0.5` below, stays refused. **Cost**: `or_else`
is unevaluated where the record is settled, so `callgrind_interpret` reads ISO page 101 flat (1 232 459 426
against 1 232 459 681) and `issue18032.pdf` page 1 **0.025% cheaper**.

**Movers: none.** `raster_golden` held 974 and moved 0, the proof no page of mine moved; `render-raster --test
corpus` 943/8/7, `pdf-model --test corpus`'s five ratchets at their ceilings, oracle 1012/46/835 against
1093's 1010/46/835 (the +2 a sibling's, by that proof). Those five and `pdf-transform --test gate` exit 0;
tier 1 is red on a sibling's file alone (clippy on `variable_text.rs`, and record 1100 over budget).

**Rows: none moved status; three lost a decayed claim.** §11.7.4.4 and §11.3.7 said a non-opaque shading and
an ambiguous image are *reported*, and ADRs 1017/1022 had stated both; §11.6.4.3 narrows to the group the flag
splits. Left: `SampleAlpha::Both`, a non-isolated group as an element, route 3 on two backends, a per-element
reading, §11.7.5.2's transfer identity.
