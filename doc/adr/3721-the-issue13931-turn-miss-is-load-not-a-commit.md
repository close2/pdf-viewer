# 3721 — `issue13931.pdf`'s turn miss is the machine's load, not a commit

Session 2255. Status: accepted. Context: the turn gate's `issue13931.pdf` p1, outside its band
(turn 129.54 ms against 57.40 .. 125.00) on round 2251's tree and inside it on main (81.66 ms);
ADR 3689 (round 2247's §7.3 repairs in the lexer and the parser), ADR 3709 (round 2252's three
readers), ADR 1481 and ADR 1519 (how this page's image is decoded); `doc/habits/measuring.md`'s
"count per render, not per process".

## The question

Round 2251 found the page's interpretation stage at 89 to 109 ms on trees carrying df311ff7f,
against about 66 ms on 153f4e7d6, with the encode unchanged. ADR 3689 had measured the launch
bands after moving its checks to cold paths, but no turn page. A turn miss is a regression until a
page-by-page comparison clears it, so the interpretation was bisected over the three commits that
touch code it can reach: 153f4e7d6, df311ff7f and b9fdeef38. The two after them (0aa95f49d, tests;
f007f167d, `raster-gpu`'s `meet.rs`) change no source the interpretation links.

## What was measured

`render-raster`'s `turn_interpret` example, profile `gates`, one build directory, each commit's
binary copied aside. Instructions are callgrind's, a run of two interpretations less a run of one:

| commit | per interpretation, `RAYON_NUM_THREADS=1` | per interpretation, the full pool |
|---|---|---|
| 153f4e7d6 | 322 116 339 | 456 169 971 |
| df311ff7f | 321 978 226 (−0.04 %) | 455 600 157 (−0.1 %) |
| b9fdeef38 | 321 988 461 (−0.04 %) | 461 856 883 (+1.2 %) |

With one thread the profiles agree function by function, but for one closure's name moving
between two rayon frames as inlining chose; the pool's spread is `crossbeam`'s `steal` and
`wait_until_cold`, a scheduler's spinning, which no commit touched. The lexer's and the parser's
functions are not in the first twenty: the page is one photograph and its `/Matte` mask — JPEG
decoding, `mask_plane` and the inversion are three quarters of the instructions.

Wall clock, eleven interpretations per process, the three binaries alternated eight times at a
load of 27 to 31 on 32 processors: the medians range 62 to 88 ms on 153f4e7d6, 62 to 98 ms on
df311ff7f and 63 to 102 ms on b9fdeef38; the fastest single interpretations are 56.4, 55.6 and
58.5 ms.

The full turn test on main's tree drew the page at 73.47 ms, inside its band. The one other figure
past 20 % of main's log, `ISO_32000-2_sponsored_EC3.pdf` p101's seventh (39.69 ms against 26.68,
band 20.80 .. 33.00), came from children at a load of 14 to 129, and its interpretation counts
190 775 109 instructions against 153f4e7d6's 191 642 148, one thread.

## Decision

No commit moved the page, and nothing is changed. Its interpretation decodes on the whole rayon
pool (ADR 1481), so its wall clock follows how many of the 32 processors other users hold: round
2251's three children ran at a load of 23.7 to 26.7 and drew 85 to 189 ms, main's at 17.7 to 21.7
and drew 82 to 90 ms. The band is not moved either: it was taken on this machine and holds the
page whenever the load leaves the pool its processors.

## Cost

A miss on this page under a load in the twenties will recur, and each time it is to be cleared
the way this one was — by instructions, which do not depend on the load — before any code is read.
