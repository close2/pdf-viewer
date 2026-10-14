# 1924 — The divided read lets each segment go once its piece has read it, and the join copies once

Session 1544, the owner's target of 2026-10-14 (code improvement). Status: **accepted**, both cuts
built. Context: ADRs 1873 §6, 1880, 1894, 1902, 1915 §3. Witness: `demo_heavy_engineering_A1.pdf`,
4 408 405 commands, 278 579 415 bytes inflated. No clause is read anew and no ledger row moves:
§8.4.2's row stays `implemented`, and what the pieces state is unchanged.

## 1. The two cuts ADR 1915 §3 named

**The stream.** The pipeline (ADR 1894) kept every segment in the composer until the scope ended,
for two readers: a later spine, whose kept statements were ranges into the segments, and the
fallback, which concatenated the segments for the plan after the inflate. Both are now served
without the stream:

1. **A spine states copies.** When a segment's effect is composed, the composer copies out the
   bytes of every statement the stack still keeps that lies in that segment, and drops the copies
   of what the stack no longer keeps (`Composer::keep_statements`). The witness keeps a handful.
2. **A segment is let go once its piece has read it and its effect is composed**, unless it is
   one the tail read from the first text object needs. Nothing is let go before the first piece
   has been read: most refusals fall there, and a refusal before any segment is gone still finds
   the stream whole.
3. **A fallback that finds a segment gone reads the stream again** from the file
   (`Outcome::Released`) and divides it after the inflate, as before. **The cost, written down**:
   a page the pipeline gives up after its first piece inflates its stream twice. The witness and
   every page the pipeline settles pay nothing.

**The copy.** The join took a piece's commands with `split_off_commands(0)`, and `Vec::split_off(0)`
copies every element into a new buffer and keeps the old one — measured on this toolchain, the
returned vector's buffer is not the original's. So each piece's list was copied twice, once out
of the piece and once into the page's list. `DisplayList::split_off_commands` now hands the vector
itself over when it is asked for everything; the join's one copy, into the page's list, is left.

## 2. What was not built, and why

**Handing the drawer the pieces' lists in order** would remove the last copy, but the display list
is one `Vec<Command>` read as `&[Command]` at 453 call sites across the workspace, the drawer's
strips and every census among them; a list in pieces is a change to all of them, not to the read.
**A reservation the pieces write into directly** needs each piece's command count before it is
read, which nothing knows, or writes into uninitialised memory, which `#![forbid(unsafe_code)]`
refuses. The copy left is on the joining thread beside the pieces still being read, so it is off
the read's critical path except for the last piece's.

## 3. Measured

Three exports of HEAD `70eda087`: HEAD, the stream cut alone, both cuts; `--profile bench`.

| | HEAD | stream cut | both cuts |
|---|---|---|---|
| display list digest (`callgrind_interpret --digest`) | `07eb7445…68e3` | the same | the same |
| read, pipelined, 24 threads, ms (two interleaved runs of five) | 429–456, 392–419 | 433–462, 386–425 | 388–431, 406–476 |
| read, pipelined, 12 threads, ms | 438–506, 439–480 | 436–515, 453–472 | 435–482, 441–500 |
| `wait4` peak, headless render, huge pages off, 4 threads, MiB (three runs) | 1 556.1–1 556.5 | 1 439.2–1 440.1 | 1 438.6–1 442.2 |
| the same, 24 threads, MiB | 1 710.3–1 888.6 | 1 576.0–1 682.9 | 1 574.2–1 634.7 |
| massif's peak, `--fair-sched=yes`, 24 threads, MiB | 1 442.5 | 1 389.3 | 1 389.3 |

**The read is no slower and no quicker**: every spread overlaps HEAD's. **The read's peak is gone**:
under HEAD the heap peaks at the join with the stream's 273.9 MiB held; with the cut the stream
falls segment by segment through the read (113.4, 80.6, 65.7, 35.8, 0 MiB at successive snapshots)
and the heap's peak moves into the draw, at 1 379.5 MiB.

**The hypothesis that the peak falls under 1.1 GiB did not hold, and could not have**: ADR 1915 §3
measured the draw's plateau at 1 238 MiB — the joined list, its paths and their bounds — and the
draw's peak stands on it: the list's 504.5 MiB, the paths' 604.4, and two 84.1 MiB vectors of
`Option<Rect>` at once, `DisplayList::mark_bounds`'s and the strip planner's
(`pdf_render::strips::strip_inputs`). That pair is `pdf-render`'s, and the next cut.

**Massif under its default scheduler is not this pipeline's instrument.** It runs one thread at a
time and starves the calling thread: under it the first piece finished last, so nothing was let go
until the read ended and the cut's peak read 1 590.2 MiB against HEAD's 1 581.8. With
`--fair-sched=yes` the threads take turns and the segments go as they do natively, where a scratch
build printed 91 of the witness's segments let go during the read.

**The copy's cut** moves no wall figure, because the joining thread is not the critical path, and
no peak, because each piece's second buffer lived only for its own join. It is kept because it is
work removed with nothing added. Callgrind, `--fair-sched=yes`, 24 threads, the witness read once
by `callgrind_interpret`: the 91 joins cost 232.5 M instructions inclusive under HEAD, 232.7 M with
the stream cut, and 178.2 M with both, `split_off_commands`'s 56.8 M gone; the whole read is
58.513 G, 58.512 G and 58.457 G.
