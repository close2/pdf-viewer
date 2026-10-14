# 1915 — The strip floor holds the witness to ten strips, and its memory is its display list

Session 1539, the owner's target of 2026-10-14. Status: **accepted** as two findings; §2's change
was built, measured and **taken back**, and §3's cut is named and not built. Context: ADRs 0139,
0219, 1758, 1837, 1859, 1873 §6, 1882, 1896 §4, 1900 §4. Witness: `demo_heavy_engineering_A1.pdf`
(MD5 `166bb8be…`), 4 408 405 commands. No clause is read anew and no ledger row moves.

## 1. The premise, and the lever it did not name

ADR 1896 §4 found the planner granting ten strips on twelve threads and on twenty-four, and asked
for the pool's width at a 1 000-row target. The premise held: HEAD's planner cuts the witness at a
1 416 × 1 000 window into ten strips at both widths. **The ask is not the lever.** It is already
the pool's width on twelve threads, and on twenty-four it is fifteen, the rows over the floor.
What makes ten is the floor itself: `strip_boundaries_avoiding` balances the rows' costs and may
not cut a strip under `MIN_STRIP_ROWS`, and the witness's marks crowd into a few hundred rows. A
scratch build asked the planner: a floor of 40 grants 12 and 13 strips, and 32 grants 12 and 16,
at `MAX_STRIPS` of 16, 24 or 32 alike.

## 2. A floor of thirty-two rows: measured, and taken back

**What it buys.** One `--clock` hold: `viewer-host/examples/pooled_draw` at 1 416 × 1 000, ADR
1914's split (floor 64) against it plus `MIN_STRIP_ROWS = 32`, alternated three times, three draws
a process, load 1.4–4.4. The page drawn whole, ms: on 12 threads 10 strips 318–340 (median 326)
against 12 strips **276–309 (293)**; on 24 threads 10 strips 317–342 (322) against 16 strips
**289–314 (303)**. The plan itself stayed at 38–42 ms. That is 6–10 % of the draw, not the fifth
ADR 1896's even cuts suggested from a sitting on the second lane.

**What it costs**, from its own export walked against `/home/AI/arms-1534/`: `raster_golden`'s
pages, drawn in one strip, held 974; its division test failed. `issue20324.pdf` p1 moves 1 044 of
501 832 pixels at two strips — past ADR 1758's one in a thousand, on a page that bound held — and
three named pages pass their ceilings (`issue1350` 1 078 → 1 083 pixels and 1 → 2 levels, `issue7014`
329 → 334, `pdfjs_wikipedia` 713 → 781). The oracle's frame moved under the three 1× arms: their
mean errors, printed to four places, moved on 12, 13 and 15 of 968 first pages, and no lane's own
frame did. The 4× arms were not walked. Two fixtures assumed the old floor, and the culled window cut in four meets ADR
0219's residual on one row of dots, 15 of 60 000 pixels by one level.

**Why it is taken back.** A strip draws its marks at its own origin, so each new cut moves pixels
by ADR 0219's residual, and the division bound is the claim that the picture does not depend on
the machine. Raising its ceilings and naming a thirteenth page past it for 20–33 ms on one document
is the trade the bound exists to refuse. `MIN_STRIP_ROWS` stays 64. **What would make the floor
free** is the residual gone: a mark that `area` converts row by row from the device coordinates
moves nothing at any cut (ADR 1819), so each kind of mark moved off `tiny-skia`'s converter lowers
what a smaller floor costs.

## 3. The census: where the witness's bytes go

**One instrument**: `valgrind --tool=massif` over `quorra-transform render --dpi 42.75 --format
ppm` (ADR 1900's headless figure), built `--profile bench` — release code with its symbols — from
the split's export, on twenty-four threads; each allocation attributed to the first frame in this
tree (`scratchpad/r1539/attrib.py`). Its frame is the native run's, byte for byte. The heap peaks
at **1 534.5 MiB** useful (1 669 MiB with the allocator's own) at the join that ends the divided
read. The same binary's `wait4` peak is 1 648–1 717 MiB on twenty-four threads and 1 557–1 561 MiB
on four.

| at the peak | MiB | share |
|---|---|---|
| paths: `Arc<Path>` 369.3 and their segments 235.1 (`Interpreter::end_path`) | 604.3 | 39.4 % |
| the pieces' commands, still held (`Interpreter::draw`) | 398.4 | 26.0 % |
| the inflated content stream, still held (the pipeline's `produce`) | 273.9 | 17.8 % |
| the joined list's commands, part-copied (`Interpreter::join`) | 214.1 | 14.0 % |
| the stream's own bytes (`Parser::parse_stream_data`) | 37.7 | 2.5 % |
| everything else, the frame among it | 6.0 | 0.4 % |

Through the draw the heap is 1 238 MiB: the paths, the joined commands (504.5 MiB, 120 bytes a
command), the bounds `MarkBounds` keeps (84.1 MiB) and the stream. The frame, its strips and their
mask caches are each under massif's half-percent threshold. The hand-over belongs to a window, and
`quorra-transform` has none.

**The hypothesis held in two parts of three.** The display list is 79 % of the peak and the frame
is under one percent. But the peak is the read's, before the drawer starts. So a list that drops
what the drawer finished with lowers the draw's plateau and not the peak. **The first cut is the
inflated stream**: dead once the last piece is read, held through the join, 274 MiB of the peak for
no time. The second is the join's copy (ADR 1873 §6). Both are `pdf-model`'s read, not this crate,
and both lower a figure rather than state a bound, so neither is built here.
