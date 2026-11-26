# Round 2255 — `issue13931.pdf`'s turn miss is the machine's load, not a commit

A pixels round in the `lane3` worktree. Round 2251 measured `issue13931.pdf` p1's interpretation
at 89 to 109 ms on trees carrying df311ff7f (round 2247's §7.3 repairs) against about 66 ms on
153f4e7d6, and its turn outside the band. The round bisected the interpretation and changed no code.

## What it found

- `turn_interpret`, profile `gates`, one build directory, the binaries of 153f4e7d6, df311ff7f
  and b9fdeef38 side by side (0aa95f49d and f007f167d change nothing the interpretation links).
- Callgrind, two interpretations less one, one rayon thread: 322 116 339, 321 978 226 and
  321 988 461 instructions; on the full pool 456.2 M, 455.6 M and 461.9 M, the spread
  `crossbeam`'s `steal` and rayon's `wait_until_cold`. The page is a photograph under a `/Matte`
  mask: JPEG decoding, `mask_plane` and the inversion are three quarters of it, and no function
  of the lexer or the parser is in the first twenty.
- Wall clock, the three alternated eight times at a load of 27 to 31: medians 62 to 102 ms on
  every build alike, fastest 56.4, 55.6 and 58.5 ms. Round 2251's children ran at a load of 23.7
  to 26.7 and main's at 17.7 to 21.7; the page decodes on the whole pool (ADR 1481).
- ADR 3721: no commit moved the page, nothing changes, the band stays.

## Ledger

Rows before and after: unchanged; no clause was touched.

**Gates.** No source changed, so no crate's fmt, clippy, nextest or rustdoc is owed and the fuzz
check is not (no public type changed). `tools/state.sh --round 2255 launch` 0: 36 figures banded, 0
outside. The turn test 101: 5 outside, issue13931 inside at 73.47 ms (main's log 81.66); the Type 3
page's command count and three figures (ADR 3705's, 2293.82 ms) and EC3 p101's seventh, 39.69 ms
against 33.00, its children at a load of 14 to 129 and its interpretation 190.8 M instructions
against 153f4e7d6's 191.6 M. `cargo test -p conformance --no-fail-fast` 0 once this paragraph stated
its gates.

## Habit

A turn miss on a page that decodes on the whole pool is read first as instructions per
interpretation, one rayon thread, commit beside commit: the count does not depend on the load,
and here it cleared three commits in ten seconds of callgrind where wall clocks had disagreed.
