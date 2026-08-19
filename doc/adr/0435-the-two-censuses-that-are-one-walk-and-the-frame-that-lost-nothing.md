# 0435 — The two censuses that are one walk, and the frame that lost nothing

Status: accepted
Date: 2026-08-19
Session: 600

## Context

`doc/todo/54` held the last two residues of what quorra's own account of where it stands asked of
this tree. Both had been recorded as ours in `doc/QUORRA_FEEDBACK.md` since §25 — five sessions —
and both had the same reason for standing: each is a question about a *corpus*, and nobody had
spent the walk.

- **The two censuses.** How many of this corpus's fills are axis-aligned rectangles, and how the
  `(clip_residue_regions, clip_residue_tiles)` pair is distributed over its pages. §25.4 had
  already decided they were one piece of work rather than two, and §28.9 had said which of them
  matters: upstream's `artwork` archetype gains 1.2× from their divided encode where their drawing
  gains 6.6×, residue-clipped marks are what separates the two, and nothing in this tree could say
  how much of this corpus is which.
- **The fifth-frame tile-cache loss.** ADR 0368's session — the owner's 49.7 MB drawing, `+`, `+`,
  `-`, `-` — where the fourth frame returned to the second's magnification and paid almost no
  geometry while the fifth returned to the first's and paid full geometry again. That ADR left it
  open between two causes: the atlas's own capacity, or a transform differing where the round did
  not look. Upstream's answer named the instrument: `Counters::atlas_repacked`, wired here since
  the five-hundred-and-thirty-second session, is true exactly on the frame whose atlas layout was
  thrown away.

## Decision

**1. One committed example walks the corpus and answers both, and it uses the shipping predicate
rather than a copy of it.** `crates/render-quorra/examples/rect_and_residue_census.rs` walks the
pdf.js corpus's first pages at 1× — `tests/corpus.rs`'s own population and scale, so that what it
says about "this corpus" is what every other statement in this project about this corpus is made
over. `pdf_render::crop::whole_rectangle` was `crop.rs`'s private `the_whole_rectangle` and is now
public: a census that carried its own rectangle test would be measuring a second implementation of
the test, which is the same argument `examples/function_paint_census.rs` makes about driving the
shipping path.

**2. The residue pair is exposed by `QuorraRasterizer::last_clip_residue` and deliberately not by
`FrameCost`.** This is the position `QUORRA_FEEDBACK.md` §25.3 stated to the quorra team and it is
kept: a field read into a struct and never printed is dead weight, and a field printed on every
frame of a window to answer a question about a corpus is the wrong instrument in the wrong place.
An accessor a census can ask is the right one. It is cleared at the start of a render and by the
window path, because a pair left over from another page reads as this page's.

**3. `zoom_frame`'s two-frame pair is generalised to a session, and its frame line carries
`repacked`.** `ZOOM_FRAME_SEQUENCE` names magnifications in order against one device; the default
is the pair, at the same two targets, so every table taken with this example before today is still
comparable. That is what makes a five-frame session measurable without a new probe, which is what
the item asked for.

## What the runs said

Numbers are in `doc/QUORRA_FEEDBACK.md` §34, which is where they go — the owner carries that
document back. The three findings, and only the findings:

- **Ninety-seven per cent of this corpus states no residue at all**: 925 of the 955 pages a frame
  was counted for report `(0, 0)`, and 30 report a region or a tile. So the population that gets
  upstream's 6.6× is nearly all of this corpus and the 1.2× one is thirty pages — which is the
  number §25.3 asked for, and it is also this tree's reason for *not* asking them to divide the
  residue rasterisation.
- **6.30% of this corpus's 223 532 fills are one axis-aligned rectangle and nothing else**, and
  5.81% reach the device as one. The gap between the two rows is 1 095 rectangles under a transform
  that does not preserve the axes, and printing one number without the other would have been a
  claim about a device made from a fact about a path.
- **The fifth frame's loss is not a repack and does not reproduce.** `atlas_repacked` is false on
  every frame of the five-frame session, and frames 4 and 5 both cost about 92 ms against 465–727
  for the first three. The atlas holds the fit view's tiles across three intervening
  magnifications. What the counter establishes is that the one named cause is not it; what the
  clock adds is that there is no longer a loss to attribute.

**Run twice, identical.** The census's own output is byte-for-byte the same across two runs — five,
in the end — and the five-frame session was run twice. This is not ceremony: the
five-hundred-and-eighty-first session found a census whose answer changed between runs because a
`static` table was a process budget, and an instrument that cannot answer the same thing twice
establishes nothing.

**And one fill moved between two builds of the same session, which is left named rather than
chased.** The first pair of runs reported 223 531 fills and 14 071 rectangles where every later run
reports one more of each. It is **not this round's code**: the count was measured again with
`collection.rs` reverted and it stayed at the higher number, so the delta is machine state between
the morning's run and the afternoon's rather than a source change. The likely mechanism is worth
recording because it is not obvious — a glyph is a `Command::Fill` in this display list, so a page
whose font is substituted from the *system* draws a different number of fills, and the gates ran
between the two measurements. One fill in 223 532 moves no share by more than a hundredth of a point
and no conclusion at all; what it does say is that a page's display list is not a function of the
file alone, which is a question for a round with a corpus instrument rather than for this one.

## Consequences

- `doc/todo/54` is deleted and its line comes out of the index: both items are closed, and the
  argument is here.
- **A closed item is not a closed question.** The residue distribution is a fact about *this*
  corpus at *this* revision; a corpus that grew by a hundred drawings would move it, and the
  instrument is committed so that asking again costs thirteen seconds.
- `QuorraRasterizer` gains one two-word accessor and `pdf-render` one public function. Both are on
  the census's account, and both are documented as such — a reader who deletes the census should
  delete them.
- The fifth-frame item leaves nothing owed upstream and nothing owed here. It is recorded rather
  than dropped because "we looked and there is nothing there" and "nobody looked" are two states,
  and this file is the difference.
