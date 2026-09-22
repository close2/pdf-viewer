# 1214 — One field for two backdrops, and the last two paints

Date: 2026-09-22. Branch: `batch-1213-1218`, worktree `/home/AI/pdf-viewer-rounds`, five sibling
rounds (1213, 1215–1218). ADRs 1265 and 1266.

## The one field three rows were each half-reading

`Interpreter::inside_knockout` was a `bool`, so §11.4.6's "[a] knockout group may be isolated or
non-isolated" could not be asked of it, and three rows carried one residue from three sides:
§11.4.6's NOTE 6, §11.6.7's tiling cell, §11.7.4.4's overprinting pair. The field is now
`enclosing_knockout: Option<KnockoutKind>`, scoped to a **direct** element the way NOTE 6 is, and
it absorbs `transparent_initial_backdrop` — that sentence asked of one kind.

Each site answers for itself: the own-backdrop construction is refused under an *isolated*
enclosing group alone; a tiling cell states §11.6.7's non-isolated group inside a non-isolated
knockout group; §11.7.4.4's first bullet is built in both. `implicit_group_statable` and its two
reports are gone, and so is `pattern.rs`'s. Three fixtures, each derived from the clause and
each failing when the boolean is planted back: `a_cell_takes_the_backdrop_its_enclosing_knockout_group_has`
(160 against 128), `a_pairs_group_takes_the_backdrop_its_enclosing_knockout_group_has`, and NOTE 6's
`a_group_inside_an_isolated_knockout_group_takes_the_transparency_note_6_gives_it`, whose third arm
is drawn rather than reported now.

## The last two paints

§11.7.5.3's NOTE puts §10.5's values "only when all colour compositing has been completed", which
the channel does, so ADR 0479's reason for sampling a ramp under the function and ADR 0430's for
baking a cell's colours both stop reaching. Neither paint keeps a transfer: a shading's ramp is raw
and rides its function on the mark (`shading::Cache` keys it again, a type 1 shading keeps its
device program), and a tiling's function is the one in force at the mark that paints it, put on the
finished tiling as one run. §11.7.5.2's sixth condition settles which object that is; a `/TR` in a
cell's own `/ExtGState` therefore decides nothing, which §11.6.7's row had said and the code
contradicted.

## Numbers
Rows: §11.6.7 `partial` → `implemented`; §11.4.6, §11.7.4.4, §11.7.5.2 narrowed, still `partial`.
`raster_golden`: 974 held, **0 moved** — no corpus page reaches any of this, which the census
behind those rows already said. `pdf-model --test corpus`, `render-raster --test corpus`,
`render-gpu --test headless_gpu` and the oracle sit at their ceilings. `callgrind_interpret`,
page 101, fifty interpretations: the new report cost 0.12% asked in `draw_mark` and 0.002% asked
in `draw_image`, which is where it went (ADR 1266 section 6).
