# 3817 — A movie this program shows is turned by its `/Rotate`, and so is its poster

Session 2279. Status: **accepted and built** in `pdf-model` (`media::Playing::quarter_turns`,
`media/deprecated.rs`'s `quarter_turns`), `viewer-core` (`Event::Poster::quarter_turns`),
`viewer-confined` (both cross the protocol), `viewer-host` (`video::placed_turned`,
`Placed::quarter_turns`, `Placed::part_of`, `Placed::part_quad`, the poster's `Asked` and `Poster`)
and the quorra window (`viewer-ui`). Clauses: ISO 32000-2 §13.4, Table 306. Finding: 2278-B02.

## Context

Table 306's `/Rotate`: "The number of degrees by which the movie shall be rotated clockwise relative
to the page. The value shall be a multiple of 90." ADR 3594 said the entry among those this
program's player sets aside, and the pictures stood at the page's orientation. A player handed only
the file is still told nothing, and the hand-off still says the entry.

## Decision

1. **The turn is a play parameter.** A movie this program shows itself carries `/Rotate` as
   clockwise quarter turns, 0 to 3, on `Playing` (a negative multiple turns the other way, which is
   the clockwise turn completing it). A value that is no multiple of 90 is said, with the table's
   sentence, and the pictures stand unturned. The poster, which Table 306's `/Poster true` takes
   from the movie file, is the movie's first picture, so it carries the movie's turn
   (`Event::Poster::quarter_turns`).
2. **The placement turns, not the decoder.** `placed_turned` starts the rectangle's corners at the
   one the turn carries the picture's top-left to and fits the picture along the sides it now lies
   on, so `Placed::quad` and `Placed::affine` — what the graphics device and the quorra window draw
   by — need nothing else. A window that draws a part upright over `Placed::bounding_box` (GTK, Qt,
   the C ABI) gets `Placed::part_of` turned in samples, and `Placed::is_whole` is false for a turned
   placement so that no window hands on the unturned picture. A window drawing the part over a
   quadrilateral uses `Placed::part_quad`.

## Cost

A turned movie's part is copied once more per picture on the processor paths (one pass over its
samples); the device path and every unturned movie pay nothing. The confined protocol carries one
byte more per clip and per poster, held to 0 to 3 on the trusted side.
