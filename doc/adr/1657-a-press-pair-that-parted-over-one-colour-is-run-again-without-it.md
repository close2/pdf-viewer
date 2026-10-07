# 1657 — A press pair that parted over one solid colour is run again without it

Session 1410. Status: **accepted**. Found while pricing ADR 1656; amends ADR 0272's pair and ADR
0857's mask pair where the two runs part. Supersedes nothing.
Context: ISO 32000-2 §11.3.4, §11.4.6, §11.4.7, §11.7.2, §11.7.4.4; ADRs 0272, 0857, 1103, 1256,
1656; traps 1, 5 and 8.
Code: `crates/pdf-model/src/content/transparency.rs` (new: `OneColour`, `Interpreter::run_group_in`,
`Interpreter::mask_run_in`; `blend_at_the_do`, `implicit_knockout_group`, `knockout_construction`,
`Interpreter::black_half`, `Interpreter::mask_halves`), `content.rs` (the field), `content/path.rs`
and `content/text.rs` (the three implicit groups' calls).
Tests: `pdf-model`'s `transparency_groups::a_b_whose_portions_differ_only_in_black_keeps_its_press_pair`
and `soft_masks::a_b_in_a_four_component_mask_group_keeps_the_pair_when_its_portions_differ_only_in_black`,
each watched failing with its retry planted out.

## 1. The defect

A group naming four components is one content stream run once per plane (§11.4.7, ADR 0272), and the
two lists are a pair only if they have one structure. §11.7.4.4 makes a `B`'s fill and stroke "a
non-isolated knockout group", and the construction that moves the mode to the group's `Do`
(`blend_at_the_do`) is exact where the blend function is affine in its source or the parts are one
colour — the derivation's `C₁ = C₂`, a statement about colours in the group's space. Each run
compared only its own plane's channels, so `0 0 0 0.3 k` with `0 0 0 0.7 K` under `/BM /Darken` was
one colour to the chromatic run and two to the black one; the runs took different constructions,
`paired` failed, and the group was drawn outside its `/DeviceCMYK` space with the report `blending
colour space /DeviceCMYK` — a soft mask's group with its own. `black_half` said no valid content stream does this; this one is valid. Loud, so not trap
5, but wrong: §11.7.2 says "all blending and compositing computations shall be done in that space".

## 2. Decided: count the admission, and run a parted pair again with it refused

`OneColour` counts each time one solid colour admits the move and can refuse it. Where a pair's runs
part and an admission was made since its first run began, both runs are made again with it refused,
so both take the construction the derivation holds for whatever the colours are (§11.4.6's own
backdrop, ADR 1256), and the pair is kept if they now agree. The first chromatic run's readback
stands and the reruns' is rewound: what a reader gets out of a page does not depend on which exact
construction drew it. Nothing else changes: a pair that pairs is never run again, a pair parted for
any other reason is given up as before, and a group outside a press never reads the count.

**Taken over the alternative of refusing the admission on every press plane**, which needs no
rerun but moves every pair whose parts are one colour on both planes to another exact construction
— pages that draw correctly today, whose bytes would move for nothing. The rerun is paid only where
HEAD gave up a pair, which today costs a third run (`rerun_inheriting`) anyway; refused, it costs
two more and the third.

## 3. What it costs and what it moved

Callgrind on the change's export against HEAD's, `pdf_model`'s functions: ISO 32000-2 page 101's
turn +148 instructions of 174.3 M, the six launch rows' page one −11 to +169, `bug1721218_reduced.pdf`'s
turn +10.5 k of 1 260.1 M, under +0.004% on every one (ADR 1656 section 2's floor: the totals move with
the allocator). The six corpus arms, `raster_golden`
and `turn_path` are the record's. On the fragments the parted groups draw in their space — the
`B` over a cyan backdrop is the backdrop with 30% and 70% black added, as `Darken` on colorant
amounts is — and the one-colour twins are byte-identical to HEAD's rasters.
