# 1209 — The backdrop a knockout element is handed, and the shape the channel reads

Date: 2026-09-22. Branch: `batch-1207-1212`, worktree `/home/AI/pdf-viewer-rounds`, shared with
five sibling rounds. ADRs: [1255](../adr/1255-the-transfer-channels-shape-is-the-clauses-and-not-the-marks-region.md),
[1256](../adr/1256-a-non-isolated-group-is-an-element-of-a-knockout-group-where-the-backdrop-is-kept.md).

## §11.4.6's NOTE 6 was answered by a sentence four paragraphs above it

The refusal in `implicit_knockout_group` said a non-isolated group cannot be an element of a
knockout group, because the note gives it the *outer* group's initial backdrop and a command is
seeded from its immediate one. But the clause composites "each individual element ... with the
group's initial backdrop rather than with the stack of preceding elements in the group", so for a
**direct** element those two backdrops are one raster — the note's own "consequence of the group
compositing formulas when b = 0" — and `render-cpu`'s `knockout_on_backdrop` had been handing each
element a private clone of the initial backdrop all along. The refusal now guards only the two
constructions that draw the elements on transparency; the third takes such an element, with no flag
and no display-list change. What is left is an enclosing knockout group that is *isolated*.

## The channel was choosing by the mark's region rather than by its shape

`pdf_render::shape_of` painted every shape solid white. §11.6.4.2 states a shading's shape as its
painting geometry and an image's as its whole rectangle, and both were wrong in opposite
directions: a non-extending shading occluded pixels it never painted, and an image with an
`/SMask` stopped occluding pixels it covers. Both are a wrong *function* rather than a wrong
colour, and both now read the clause. That unblocks the first of §11.7.5.2's two remaining paints —
ADR 0479's chord argument is about mapping a ramp and does not reach a channel that maps the
finished pixel — which ADR 1255 designs as two hunks in `content/pattern.rs`, not this round's file.

## Attribution, because the corpus moved and it was not this round

`raster_golden` named `issue17671.pdf` and `issue20513.pdf` as moved; planting both of this round's
changes back moved the same two, so they are a sibling's and no corpus page reaches either
construction. `pdf-model --test corpus` holds every ratchet at its ceiling.

## Files touched

`crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/tests/{transparency_groups,transfer_functions}.rs`,
`crates/pdf-render/src/transfer_channel.rs`, `doc/conformance/ledger.toml` (§11.4.6, §11.7.5.2),
`doc/todo/{13,23,65}`, the two ADRs, this file.
