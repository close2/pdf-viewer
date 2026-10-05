# 1505 — A stroke fan-out's threads claim their jobs, and a page's frames do not divide the pool into cuts

Status: accepted. Session 1335. Amends ADR 1395's fan-out (the shares fixed by `Job::weight`
become claims; the weight keeps its other job, the floor). Keeps ADR 1481's `InFlight` signal as
it is, having measured its generalisation and declined it. Supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §7.4.8; ISO/IEC 10918-1 (ITU-T T.81, cited by section) sections
F.1.2.1, F.2.2; `CLAUDE.md` principle 2 and the rule on optimisations; ADRs 1321, 1375, 1395,
1421, 1431, 1445, 1457, 1481, 1495; `doc/habits/measuring.md` 48, 52, 56, 57, 59, 61; traps 50, 94.
Code: `raster/crates/raster-gpu/src/encode/parallel.rs` (`rasterise_all`, `makers_first`; the
`partition` function and its two tests are gone).
Tests: `parallel.rs`'s `every_mask_is_its_own_jobs_at_every_thread_count` (nine jobs of very
different sizes at 1, 2, 3, 7, 12 and 64 threads, each mask equal to the one-thread run's) and
`a_shared_expansion_is_made_before_its_later_placements_are_claimed`; `tests/encode_threads.rs`
unchanged and passing.

**I did not open `crates/render-cpu/`.** Every expected value here is raster against itself.

## 1. Plans: eight frames, not four, and the share rule measured a loss

The brief's premise was four photographs on eight cores. `22060_A1_01_Plans.pdf` draws **eight**
`DCTDecode` frames of 2 480 × 2 630 — four `ICCBased` photographs and their four `DeviceGray`
soft masks, each decoded beside its image (ADR 1469) — and six small ones. Pinned to the fast
class (eight logical CPUs, four physical cores), a timeline probe of one turn: all eight frames
start within 0–6.6 ms, the masks end at 17.7–29.6 ms, the pool's photographs at 21.4–33.0, and the
interpreter's own frame — started first, and the ninth runnable thread on eight CPUs — at 42.9.
Busy threads integrate to 66% of eight over the decode. So pinned, nothing idles until the masks
end, and the tail is the interpreter's frame.

**Built and measured: a frame's share of the pool.** `InFlight` became the pool's threads divided
by the large frames (image and `DCTDecode` `/SMask`, at least `BANDED_FLOOR`) that the page's
resource dictionaries reach, counted before any decode began — counted as decodes start, the first
finds the pool to itself because they start within a millisecond — and a frame was cut where its
share was two threads or more, masks included (a test held a grey frame's `Luma` bands to the whole
plane). Pinned, Plans' share is one, so nothing changes by construction. Unpinned (24 threads,
share three), interleaved, minimum of three runs of five rounds: Plans' interpretation **37.26 →
43.43** ms; `arsimon_1.pdf` p56 (two 4 961 × 3 508 frames, share twelve) **30.75 → 34.37**; pinned
p56 (share four) worse in all three runs. Reverted.

**The break-even, measured on p56's frame alone**: whole, 15.8 ms on one thread. The cut is the
pass, 4.6 ms, and then bands that cost **22.3 ms of work** on one thread — 1.41 times the whole
decode, since every band decodes an MCU row above and below what it keeps and starts a decoder of
its own — finishing in 9.8 ms on eight threads: 14.4 against 15.8 with eight idle threads, 17.1 on
two. The cut breaks even only with most of a pool idle, which a page of several frames never has;
ADR 1481's `Alone` is the right signal and stays.

## 2. Where `bug1743245.pdf`'s tiling goes now

Callgrind, one `frame_budget` round, four threads, a probe build with the tiling's steps not
inlined: 7 621 M in the process. The cutting — `Line::split` 1 279 M over 5.74 M calls,
`subtract_from` 628 M over 1.22 M, `sliver` 220 M over 1.81 M fragments — is the largest part; the
separation question is 610 M over 3.87 M pairs (2.59 M answered by an edge of the earlier piece,
0.06 M by an edge of the fragment, 1.22 M not separated and cut), `subtract_from_each`'s own loop
423 M, the scan over earlier boxes 939 M, `takes_part` 148 M. Of 612 tilings a round, 350 vouch for
the whole stroke; every one of the other 262 fails `the_rest_stand_apart` on a real overlap (217 a
tiled piece against an untiled one, 45 two untiled), so the fill's set question on them (980 M in
`complex_pixels`) is owed, not slack.

## 3. Five exact levers on the tiling, each dropped

| lever | instructions | wall, pinned |
|---|---|---|
| the scan finds the next meeting box and asks the bounds only after a cut | −370 M | +0.6 to +1.6 ms |
| the box test without branches | +163 M | — |
| eight box tests at a time, without branches | +282 M | — |
| `split` measures each side once into a buffer (ADR 1421 section 4 measured the carried form) | +510 M | — |
| dropped point lists recycled into the next split | — | none |

## 4. Kept: the threads claim their jobs

A probe of the fan-out on this page: eight shares of equal weight in one drain finished between
42.1 and 60.7 ms, and single jobs took up to 16 ms — a tight bend's tiling is quadratic in its
pieces, so a stroke's segments do not price it, and a share on a Zen 5c thread is not the time of
one on Zen 5. Each thread now claims the next unclaimed job and the masks are put back in job
order. Claimed in plain job order the Type 3 page went **12.7 → 18.4** ms: every fourth job there
is a placement of one of two glyph strokes whose expansion is shared (ADR 1445), and the threads
queued behind the two making them. Claiming the first placement of each shared expansion, and every
job with none, before the later placements fixes it; claims of a shrinking run of jobs instead
(`1 / (threads × 2)` of what is left) read 16.5 on the Type 3 page and were dropped.

Pinned, interleaved with an export of the base commit (`md5sum`-distinct), minimum of three runs of
five rounds, load 3.3–4.6, turn (step): `bug1743245.pdf` **43.57 → 41.70** (41.02 → 39.88),
`issue14415.pdf` 16.87 → 16.28 (7.96 → 7.63), Type 3 13.00 → 12.43 (12.89 → 11.45), text 7.82 →
7.53; the image and mesh pages inside their spread. The cost to a reader: a claim counter, a lock
taken once per thread, and a function that orders the jobs.

## 5. Exactness

`rasterise` is a pure function of its job and the one-thread path is the loop it was. The
`render-raster` corpus gate on the base commit with this change alone: 1× 966 agree, 0 differ, 1
refused, 7 not comparable; 4× 962 / 0 / 4 / 8; **one encode thread against many, 0 pages differ at
both**. `raster_golden` held 974, moved 0.

## 6. Left

The photograph's turn is 76 ms where ADR 1433's bands made it 32: its codestream has no `EOI`, and
ADR 1495 declines such a scan for both band plans on a counterexample in which the data ends
inside the scan; the same build without that decline interpreted it in 30.3 ms against 70.4. A
narrower admission needs its own argument for the bytes. The Type 3 page (5.98 → 12.4) and
`issue14415.pdf` (11.14 → 16.3) are dearer than `doc/performance.md`'s last figures on the commit
before this tree's base as well, and are not bisected here.
