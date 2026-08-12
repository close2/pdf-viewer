# 447 — Two of the three gates never moved, and the third had a passenger

**Finding.** The "three gates that roughly doubled" between the three-hundred-and-ninety-eighth
session and the four-hundred-and-forty-fifth is **one** gate, and its cause is a self-declared
non-gate test sharing a binary with it, not anything this tree draws.

**Date.** 2026-08-12.
**ADR.** [0282](../adr/0282-a-gate-that-doubled-and-the-passenger-in-its-binary.md).
**Touched.** `crates/pdf-model/tests/oracle.rs`, `Cargo.lock`, `doc/todo/02-every-round.md`,
`doc/todo/43-the-projects-own-turnaround.md`, `doc/todo/README.md`, `doc/QUORRA_FEEDBACK.md`,
`doc/adr/0282-*`, this file.

## The question that started it

The project owner asked whether a new quorra release fixes the performance regression. It could not
have, and the reason is structural rather than a matter of measurement: `cargo tree -p pdf-model
--edges normal,dev | grep -c quorra` prints **0**, and two of the three gates said to have regressed
— corpus and oracle — are `crates/pdf-model/tests/`. Only the third runs quorra at all.

## What the bisect found

Session 398's own commit is `244b86a`. Checked out beside `351bfed` in one sitting on one machine
with the reference cache at 99.7% at both ends:

| gate | 398 reported | 445 reported | `244b86a` today | `351bfed` today |
|---|---:|---:|---|---|
| oracle | 51.5 s | 102.0 s | 50.3, 52.0 | 94.4, 96.1, 98.4 |
| quorra | 25.1 s | 39.0 s | 34.6, 34.5 | 34.1, 35.0 |
| corpus | 3.2 s | 5.0 s | 4.6, 4.6, 4.7 | 3.9, 4.0, 4.0 |

The corpus gate is **faster** at HEAD than at 398, and the quorra gate is level. So the handover's
sentence — "the three that moved are exactly the three that rasterise all 974 first pages" — had the
right observation about which gates *could* move and the wrong conclusion about which *did*, and its
page-group hypothesis is excluded by the gates it would have had to move.

`git bisect` over the remaining 53 commits, probe = the oracle's own summary line, good below 70 s:
`744472e` **46.5 s** → `92579c2` **87.2 s**, adjacent commits, 40.7 s apart against a ~2 s spread.

`92579c2` is the four-hundred-and-seventh session (ADR 0243). It added
`the_fixed_bounds_against_the_references_own_spread` to `oracle.rs` — a derivation whose own doc
comment says it "is not itself a gate" — and `doc/todo/02` §2 runs that binary with `--ignored`,
which un-ignores the whole binary rather than filtering it. Gate alone **47.9, 47.8 s**; derivation
alone **40.2 s**; both, which is what a round ran, **94.4, 96.1, 98.4 s**.

The four-hundred-and-seventh saw the symptom and wrote it down as bookkeeping: its history row ends
"Only the *skipped*-test count moved, 9 → 10."

## The fix, and the two things it also fixes

The derivation now declines unless `PDFVIEWER_ORACLE_SPREAD` is set, printing why. The guard is in
the test rather than in the invocation because an invocation can be copied without its guard and a
test cannot be run without itself — so `tools/state.sh`, `doc/todo/02` §2 and CI all get it with no
line to keep in step.

Interleaved, three samples each: **47.0 / 46.3 / 55.5 s → 24.4 / 25.1 / 30.2 s.**

Two things go with it. ADR 0222 measured this gate at **24.5–25.7 s** before `92579c2` existed and
`tools/state.sh` now prints **25.3 s**, which is independent evidence that sixty-two rounds of
rendering work cost the oracle nothing. And the gate's own `processor time` and `slowest pages` rows
had read a factor of two high for thirty-nine rounds — `22060_A1_01_Plans.pdf` page 1 at 93.2 s
against 18.0–19.7 for the same work — one of which `doc/todo/43` had quoted as *evidence for* the
regression it was a symptom of.

No verdict, bound or pixel moves. 905 / 68 / 786 / 1 / 2 / 14 / 18 either way.

## The pin

Upstream moved twice during the round: `595d8c87` (the release asked about — a hash avalanche fixing
a regression quorra introduced in `89d7dd77`, the revision we were on) and then `c1f6e2f4` (the GPU
coverage lane chosen per command by cost). A/B/A alternation, six samples each, rebuilt between arms:
`89d7dd77` 26.3–27.1 s, `595d8c87` 26.2–27.3 s, `c1f6e2f4` 26.9–29.0 s, with 917/35/5/17 at all
three. Every band overlaps; this gate cannot resolve a change at quorra's `encode` grain, which is
now the second independent time that has been shown rather than supposed. Pinned to `c1f6e2f4`.

`doc/QUORRA_FEEDBACK.md` gains **§19**: their `doc/corpus-profile.md` walked our 995 first pages and
found that not one emits a `Command::Rect` — a fact about our translation, confirmed here, and the
first time either side has had a count of what this viewer actually hands over.

## The gate numbers this round watched print

`tools/state.sh` end to end, **2 m 37 s**: tests 1619 passed / 11 skipped in 22.1 s; corpus 974
documents in 3.1 s (65 incomplete); oracle 1794 pages in 25.3 s (905/68/786/1/2/14/18); text 40
documents in 1.8 s at 99.8% and 974 in 30.9 s at 99.2%; quorra 957 pages in 26.0 s (917/35/5/17);
dates 1545 strings, 97.99%; XMP 319 documents, 318 read; JPEG 2000 14 byte-identical, 13 differing,
3 not comparable; conformance 631 quotations verbatim, 6559 citations, 0 clauses owing a review.
`cargo deny` clean on all four; five cross-target checks build under `-D warnings`.
