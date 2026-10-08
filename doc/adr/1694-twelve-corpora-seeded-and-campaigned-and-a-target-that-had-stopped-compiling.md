# 1694 — Twelve corpora seeded and campaigned, and a target that had stopped compiling

Session 1429. Status: **accepted**. Context: traps 107, 111, 112; ADRs 1423, 1559, 1571, 1664,
1688, 1689; `fuzz/seeds.sh`, `fuzz/fuzz_targets/script.rs`, `doc/verify.md`'s fuzz lines,
`tools/main-checkout.py`.

## 1. What was seeded, and what the census said after

`tools/main-checkout.py` named ten of 32 targets with no corpus on disk and two stale by ADR 1559's
census. Each was seeded with its `fuzz/seeds.sh` arm, one hold of the lock's second lane each
(`--tree 6`; `jbig2`'s arm writes `jpx`'s corpus in the same pass), then `fuzz/seeds.sh check
<target>` in a hold of its own. Seeds, and `INITED cov` disk against fresh, after the seeding:

| target | seeds | disk | fresh | | target | seeds | disk | fresh |
|---|---|---|---|---|---|---|---|---|
| `shaping` | 1 317 | 830 | 830 | | `vfs_write` | 4 010 | 5 968 | 5 968 |
| `find` | 920 | 627 | 627 | | `script_wire` | 32 | 711 | 711 |
| `aform` | 350 | 1 369 | 1 369 | | `linearize` | 13 285 | 6 939 | 6 939 |
| `script` | 6 272 | 13 468 | 13 472 | | `jbig2` | 8 701 | 3 363 | 3 363 |
| `embed` | 3 540 | 1 257 | 1 257 | | `jpx` | 612 | 2 439 | 2 439 |
| `forms_data` | 1 365 | 1 387 | 1 285 | | `display_list` | 2 020 | 1 008 | 956 |

Every one is **current**. The two re-seeded corpora now lead their fresh seeds, as a fuzzed corpus
should (ADR 1559 section 1); their figures before the re-seed were not re-measured, so the 528 and
272 of ADR 1559 are that census's and not this one's. The jbig2/jpx arm peaked at 5.34 GiB over its
process tree, which is under the second lane's ceiling with little room: a seeder that grows past
6 GiB takes the first lane.

## 2. The `script` target had stopped compiling at HEAD

ADR 1664 moved a field's widget members — display, colours, border, alignment, rectangle,
captions — out of `FieldState` into `widgets: Vec<WidgetState>`, and `fuzz/fuzz_targets/script.rs`
still built a `FieldState` with them, so `cargo fuzz build` failed with eight E0560 errors and no
campaign on `script` could have run since. Nothing a round's tier 1 runs builds `fuzz/`: it is its
own workspace, and the two lines `doc/todo/02` section 2 gives it are tier 2 for a change *to*
`fuzz/`, not for a change to a type a target constructs. CI's `cargo fuzz build --dev` would have
named it at the next push. The target now builds one widget with the members it had, and the
`on_state: None` slot 1's ADR 1689 adds this batch; it compiles against the batch's tree and not
against HEAD alone, so the two land together.

## 3. A wire version bump stales a seeded corpus within the hour

`script_wire` was seeded at 32 seeds from `wire_seeds` and its check, forty minutes later, said
**STALE**: disk 44 edges against fresh 594. Slot 1 had raised `wire::VERSION` from 6 to 7 between
the two (ADRs 1688, 1689), so every seed on disk was refused at its first byte. `wire_seeds` names
its files by what they carry, so the re-seed overwrote the 32 rather than adding beside them, and
the check after it reads 711 against 711. `doc/verify.md` says so under the target's line. Until
the merge the owner's corpus holds version-7 seeds that HEAD's decoder refuses.

## 4. The campaigns

Each seeded target ran 600 s as `doc/verify.md`'s campaign paragraph states it: built `-O -s none`,
run by path, a scratch corpus first and `fuzz/corpus/<target>` second, artefacts to scratch, under
the line's own `-rss_limit_mb`, `-timeout` and `-max_len`, `--data 4 --tree 4` (`display_list`
`6`/`6` for its 4096 MB), `jbig2` with its line's `-fork=1 -ignore_timeouts=1`. `INITED → DONE`
edges, and executions:

| target | edges | executions | | target | edges | executions |
|---|---|---|---|---|---|---|
| `script_wire` | 711 → 1 912 | 346 433 135 | | `embed` | 1 257 → 2 632 | 5 432 775 |
| `script` | 13 479 → 20 819 | 425 139 | | `vfs_write` | 5 968 → 8 401 | 639 093 |
| `aform` | 1 369 → 2 084 | 14 749 147 | | `linearize` | 6 939 → 7 868 | 231 618 |
| `shaping` | 830 → 1 276 | 2 273 814 | | `jpx` | 2 439 → 2 848 | 20 231 |
| `find` | 627 → 1 294 | 4 939 085 | | `jbig2` | 3 363 → 4 199 | 5 268 |
| `forms_data` | 1 387 → 2 610 | 10 656 624 | | `display_list` | 1 008 → 1 649 | 21 850 225 |

**No crash and no memory refusal on any target, and no timeout but `jbig2`'s.** Eleven artefact
directories are empty and those eleven libFuzzer exits were 0. `jbig2`, in fork mode, left eight
timeouts (fork-mode exit 70): `2009ce2c`, `27c870d5`, `5f9f0c44`, `ba172b14`, `bb0e491a`,
`cee75808`, `d8905028` and `ff5139bc`, 13 887 to 213 915 bytes, every one still decoding past
40 s without the sanitiser. A stack taken under `gdb` after 8 s puts all eight inside
`hayro-jbig2`'s symbol dictionary decode (`decode/symbol.rs`, under its integer decoder and, once,
its generic region), which is ADR 1424 section 3's class: the confined worker's `REQUEST_TIMEOUT`
is the bound, and that ADR's test holds it on an input of the class. So none is a new defect and
none takes a new test; the eight are kept in `fuzz/artifacts/jbig2/` and named here, which is what
reads them.

## 5. What this does not say

A clean ten minutes is a statement about ten minutes. `script` and `vfs_write` gained 54 and 41 per
cent of their edges inside the run and were still finding at its end, so their corpora are far from
saturated and a longer campaign is the next thing to spend on them; `jpx` ran 33 executions a
second, so its 20 231 are a sample and not a search. The campaigns' finds went to scratch, as ADR
1423 chose, and were not merged into the owner's corpus.
