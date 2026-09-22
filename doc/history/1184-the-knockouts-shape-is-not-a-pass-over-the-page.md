# 1184 — The knockout's shape is an input, not a pass over the page

**The headline build was the wrong build, and the clause says so.** ADR 1113 declined a per-pixel
shape channel on a premise session 1148 removed by building `pdf_render::transfer_channel` (ADR
1125), so it was re-taken, and is upheld on another reason: §11.7.5.3's NOTE puts the transfer
function after all compositing, so that channel resolves over a *finished* raster, while §11.4.6's
weighted average `Pᵢ = (1 − fᵢ) × Pᵢ₋₁ + fᵢ × Eᵢ(B)` multiplies the accumulation as it stood under
element `i` — which a finished page no longer holds. The carrier could move, the resolution cannot,
and `Command::Shaped` already rasterises each element's shape at most once, inside a knockout group
alone. ADR 1205.

**Two of §11.4.6's four residues were mis-named; re-testing each against the code found it.**
- *A non-isolated group used as an element* was never a shape gap. §11.3.7.2: "The shape of a group
  object shall be the union" of the shapes of the objects it contains — no backdrop in it. The
  isolation condition came off `shape_the_alpha_already_is`'s arm, and the refusal string that named
  it was **unreachable**: a group nested inside a knockout group is emitted `isolated: true`.
- *`/AIS` both ways* narrowed by one element kind. `settled_over` borrowed `group_alpha_is_shape`'s
  predicate, which declines a `Command::Shaped` for that function's own reason — §8.5.4's exact
  intersection. A stated pair arrives with its shape already stated, so §11.6.4.3's flag has nothing
  of it left to reinterpret; `flag_reinterprets_nothing` asks this clause's question instead.
- *`SampleAlpha::Both`* is the one true shape gap, and ADR 1022 §5's price is corrected: the blocker
  is `Picture::source` multiplying the stencil into its `/SMask` before a command exists, not a
  second raster per command — one `Picture` arm in `pdf-model::image`, not this round's file. And
  §11.7.4.4's "a non-isolated group used as a part under the shape reading" is wrong twice: that
  guard is unconditional, and its quantity is NOTE 6's backdrop.

**Fixtures**, each calibrated by planting its construction away (trap 13):
`a_stated_pair_leaves_mixed_content_a_reading` draws the overlap at `(128, 128, 0)` — stage b) at a
shape of 1.0 over the red page — its control, the pair replaced by its object, staying refused; and
`a_non_isolated_groups_shape_is_the_union_accumulated_on_transparency`. **Cost**, 50 interpretations
of page 101, two arms: 1 252 899 002 → 1 252 899 128, **+0.00001%**; `raster_golden` held 974/974.

**ADR 1107's population was 1188's; the pricing is here** (ADR 1206; a duplicate census of mine
was deleted rather than landed). `issue12798_page1_reduced.pdf` page 1 is the whole curated
population — 2 groups, both §11.7.4's under `/Multiply`. `callgrind_rasterise`, 20 draws, one
sitting, `remove_the_backdrop`'s call planted away in the second arm: 6 225 323 395 →
6 668 515 460, **+443 192 065 instructions, +7.12%**, 22.2 M a draw — the page's own marks rather
than a constant, and no other curated page reaches the branch. Rows §11.4.6, §11.6.4.3 and
§11.7.4.4 corrected and §11.4.4 priced, all still `partial`; `doc/QUORRA_FEEDBACK.md` section 50
says what a scene would need for the own-backdrop group.
