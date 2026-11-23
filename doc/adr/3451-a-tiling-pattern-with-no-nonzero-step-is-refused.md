# 3451 — A tiling pattern that states no nonzero step is refused, and said

Session 2187. Status: **accepted and built.** In `pdf-model`: `content::pattern::tiling`'s step reading;
`lattice::cell_extent` removed. Finding 2184-D02. Clauses: ISO 32000-2 §8.7.3.1, Table 74.

## Context

Table 74 makes `/XStep` and `/YStep` "(Required)" and says of each that it "shall not be zero". A zero or absent
step was replaced, silently, by the `/BBox`'s width or height, on the reason that it is "what other readers assume" —
a reason principle 5 does not admit, and a silence trap 5 names.

## Options

1. Space the cells by the `/BBox`, reported. The box is the cell's clip, not its spacing; Table 74 states the two
   separately so that cells may overlap or leave gaps, and a file that states no spacing has not stated adjacency.
   The lattice drawn would be this reader's invention, presented as the file's.
2. Refuse the pattern, reported. The paint stays §8.6.8's initial pattern, "a pattern object that causes nothing to
   be painted", and the page names the pattern and the missing entry.

## Decision

Option 2. The step decides where every cell is, and no clause supplies one in its place; what is lost is said, under
`Unsupported::Shading` ("a shading or pattern (…) was not drawn"), whose sentence is true of it. A negative step stays
a legal step and tiles the same lattice as its magnitude.

## Consequences

A file with a zero or absent step loses that pattern's marks where it had drawn them at the box's extent; the page is
no longer reported complete, so the oracle no longer judges it.
