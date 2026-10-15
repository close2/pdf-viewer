# 1941 — A whole page's cull is answered by its bounds' edges, and a window's list is copied in place

Session 1552, under the project owner's "switch to code improvements … until I say otherwise" of
2026-10-14: ADR 1930 §4's next cut, the window's cull. Status: **accepted** and **built** in
`pdf-render`; ADR 1930's half stands, re-measured. Context: ADRs 1841, 1882, 1930. Code:
`pdf_render::MarkBounds::every_command_reaches`, `DisplayList::reaching`, `DisplayList`'s `Drop`.
Test: `reach::tests::the_edges_say_every_command_reaches_only_where_none_misses`. Instrument:
`viewer-host`'s `examples/window_against_whole`, which now draws the largest window at every
magnification and prints both drawings' digests. Cites no clause and moves no ledger row.

## 1. What was found

**The premise did not hold in its mechanism.** The brief had the cull walking every command; it
does, and the walk is 7–12 ms of the witness's frame on four to twenty-four threads, the bounds
being kept per magnification already (ADR 1841). What a window that is most of its page pays is
elsewhere, measured by a scratch probe on a quiet machine: a window of 83 % of the page at 1.7× the
fit keeps 4 333 950 of the 4 408 405 commands, and **copying them** — 520 MB of commands, a
reference count apiece — took 77–158 ms on one thread; a `filter_map` on the pool was no quicker,
81–98 ms, because an unindexed parallel iterator gathers pieces and joins them on one thread, a
second copy of every command. **Letting them go** took 31–34 ms on one thread, on the frame's path.
And a window's translation is not the one the bounds were measured at, so the strip planner
measured each kept command's bound again, twice (ADR 1940 §2.3).

**The hypothesis did not hold, and could not have.** A culled list that keeps 98 % of the commands
draws in what the whole page does — 491–526 ms against 525–533 on four threads, 323–335 against
344–368 on twenty-four — because at these sizes the draw's cost is per command, not per pixel. So a
window of nine tenths cannot land *before* the page with a cull of no cost at all; it can tie.

## 2. Decision

1. **A whole page's cull costs nothing after the first.** `every_command_reaches` keeps the bounds'
   innermost edges (the smallest right and bottom edges, the largest left and top), taken once per
   magnification on the pool, and answers for every command at once where they are inside the
   target. It never says so where a command misses: each of `reach`'s four tests is monotone in
   the edge it reads, because a bound is moved by a sum and grown by a sum of something not
   negative and an `f32` sum rounds monotonically. A NaN, or a bound whose corners are out of
   order, is outside that argument and is answered by the walk.
2. **The kept commands are copied by an indexed iterator**, which writes each into the list in
   place on the pool: 43–49 ms where the walk's filtered copy took 81–98.
3. **A list of 65 536 commands or more is let go of on the pool** (`DisplayList`'s `Drop`): 18–21
   ms where one thread took 31–34. Smaller lists are dropped where they are.
4. **ADR 1930's half stands**, because the crossing is still about there (§3).

## 3. Measured

Two exports of HEAD `7bf264b3` (the example's own change in both), one `--clock` hold, the witness
at 1200 × 900, three rounds a magnification, two passes on four threads and one on twenty-four;
milliseconds from the ask to the landing. The window is the largest one (the viewport and a quarter
of it round it), centred. Load 3.0–4.9, rising to 10.4 at the end.

| × fit | window | HEAD, 4: whole / window | this, 4 | HEAD, 24 | this, 24 |
|---|---|---|---|---|---|
| 1.25 | 100 % | 453–668 / 461–604 | 442–530 / 427–463 | 353–372 / 351–366 | 323–372 / 342–353 |
| 1.5625 | 96 % | 526–684 / 568–693 | 495–537 / 526–538 | 350–387 / 369–404 | 341–352 / 373–381 |
| 1.7 | 83 % | 531–730 / 689–903 | 520–567 / 628–722 | 374–387 / 506–520 | 353–390 / 459–484 |
| 1.85 | 70 % | 557–760 / 665–846 | 546–583 / 614–631 | 373–421 / 476–502 | 370–395 / 448–486 |
| 1.953 | 63 % | 574–615 / 621–748 | 560–604 / 579–596 | 379–433 / 447–471 | 366–407 / 420–432 |
| 2.2 | 49 % | 623–657 / 515–533 | 597–690 / 477–500 | 405–432 / 360–387 | 379–428 / 343–353 |
| 2.441 | 40 % | 672–702 / 479–500 | 654–698 / 451–468 | 410–446 / 340–344 | 404–446 / 306–318 |
| 3.052 | 26 % | 805–890 / 315–327 | 796–827 / 288–387 | 483–536 / 228–253 | 456–494 / 225–235 |

**Where the crossing is now**: between 63 % and 49 % of the page on both pools — a window of 63 %
ties the page on four threads and is 13–66 ms behind it on twenty-four; one of 49 % is 97–213 ms
ahead on four and 26–85 on twenty-four. A window of 83 % is 61–181 ms quicker than at HEAD on four
threads and 22–61 on twenty-four, and still 61–202 ms behind its page on four and 69–131 on
twenty-four. Every drawing, whole and
window, has HEAD's digest at every magnification and thread count. An earlier hold at a load of
4.9–8.2 read the same digests and noisier clocks.

## 4. What this does not do

**A culled list that copies nothing** — the window as a selection of the whole list's commands —
is what would make a window cost its window. It reaches every reader of `&DisplayList` the draw
has: the strip loop and its glimpses, the planner, the soft masks' reach, the reduced images,
`check_group_blit`, and the device's encoder; and even then a window that keeps most of the
commands draws as long as the page does. What would let ADR 1930's half fall is that, with the
bound restated in commands kept rather than pixels shown, which the cull already counts.
