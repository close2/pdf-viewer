# 527 — The window a bomb fits in

**Finding.** `doc/todo/14` — road D, the project owner's first of three — says a round taking it
owes **the measurement before the rewrite**, and this round took it. A `Lexer` fed from a fixed
window, refilled by ADR 0343's inflate pump, **holds Bomb B at 6 MB of peak resident where the
shipped path spends 1030 MB and produces not one token**, and reads the witness's page to the
same 20 834 587 tokens and 3 185 295 operators for **+4.10% instructions** and **98 MB instead of
380 MB**. Page one of every pdf.js document through a deliberately small 512-byte window: **948
agree, 0 disagree**. The verdict is yes and the rewrite is the next round's.

**And the prize is bigger than the file predicted.** `doc/todo/14` said the witness would fall
from 381 MB to "about 315 MB, the display list and the raster". `massif` names the four blocks
alive at the peak and the display list is not among them: two copies of the same 141 MiB of
decoded content are, plus the file and a second copy of the *encoded* stream. The display list is
about 99 MB and arrives after those are freed. The road leaves about **193 MB where 446 MB of
heap stands**. The same profile corrected a figure two todo files carried: the witness's content
stream is **147 972 263 bytes**, not 66 MB.

**Date.** 2026-08-14.
**ADR.** [0362](../adr/0362-the-window-a-bomb-fits-in.md).
**Touched.** `crates/pdf-model/examples/window_lexer_spike.rs` (new — the experiment),
`crates/pdf-model/examples/token_window_census.rs` (new — the census),
`crates/pdf-model/Cargo.toml` (`flate2` as a dev-dependency, for the spike's pump),
`doc/conformance/ledger.toml` (§7.8.2, §8.9.7), `doc/todo/14` (the measurement, the four
decisions the rewrite owes, the corrected arithmetic), `doc/todo/10` (§1's residue, §5's table and
its D rows), `doc/adr/0362-*` (new), this file. **No shipped code path moved.**

## The two censuses, which answer the two open design questions

39 976 documents opened of 40 388 found, 78 844 pages, 225 775 555 content tokens — one root,
`doc`, which holds pdf.js, the four corpora and this project's own.

- **The largest single lexical object** is **390.16 KiB**, a string on `219789.pdf` page 9. 233
  tokens pass 4 KiB, **2** pass 64 KiB, none passes 1 MiB. `max_string_len` is 2²⁶, 168 times
  anything measured.
- **Inline images**: 93 930 read; **90 304 state or imply their length before their data is
  read** (336 by `/L`, 89 968 by §8.9.3's arithmetic) and **3 455 need the forward `EI` search**,
  whose largest witness is **2.99 KiB** against a largest image of 9.01 MiB. So §8.9.7's one guess
  costs a bounded window a small lookahead buffer and a refusal it already has.

## The instrument that lied, and the one that did not

**`ru_maxrss` from `wait4` has a floor equal to the *spawning* process's resident set**, because
`posix_spawn` shares the parent's address space until the `exec` and the child inherits its
high-water mark. A Python harness measuring a 4 MB program reported 13–14 MB. `/proc`'s `VmHWM`
reads 4 MB for the same run and agrees with `ru_maxrss` exactly wherever the number is large — 765
against 765, 1030 against 1030, 379 against 379 — which is why ADR 0354's figures are unaffected
and why the small ones here are `VmHWM`.

Both bombs were rebuilt from `doc/todo/10` §2's description for the fourth time and came out
**389 317 and 1 847 467 bytes**, the sizes that file records, to the byte. The baseline through
`pdf-retrieve` reproduced ADR 0354's table to the megabyte: Bomb A 768 MB / VmPeak 777, Bomb B
1031 / 1041, the witness 381 / 390.

## Gates

`fmt` clean. `clippy --workspace --all-targets` silent of lints (the `viewer-qt` `cargo:warning=`
lines are gcc's on a cold build, `doc/todo/02` §2) — two pedantic lints on the new examples were
fixed rather than allowed. `nextest --workspace` **1926 tests run: 1926 passed, 15 skipped**.
Doctests pass. Conformance **875 subclauses — 427 implemented, 230 partial — 8013 citations, 772
quotations**, 0 unreviewed in every clause. Corpus **974 documents in 5.2s: 0 unopenable, 8
locked, 2 encrypted beyond us, 6 pageless, 61 incomplete, 0 slow**, run for cheap confirmation.

**The oracle is not owed**: nothing shipped this round is reachable from library code — two
dev-only examples and a dev-dependency — so no gate that renders can see it, and the spike's own
equality check over 974 documents is the reading that stands in its place.

**Four `pdf-font` tests fail in a bare worktree and it is the checkout rather than the code.**
`loading.rs`'s `corpus()` reads `<manifest>/../../doc` for `*.pdf`, so a worktree without the spec
documents unpacked (or symlinked) tells four tests that *no font in the corpus* has the property
they are written about, and they assert rather than skip. Worth knowing before reading them as a
regression: they pass the moment `doc/*.pdf` exists.
