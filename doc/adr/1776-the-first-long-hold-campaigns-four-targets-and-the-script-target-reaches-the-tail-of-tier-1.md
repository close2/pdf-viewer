# 1776 — The first long hold campaigns four targets, and the script target reaches the tail of Tier 1

Session 1471. Status: **accepted**. Context: ADRs 1424, 1590, 1694, 1717, 1746, 1756, 1762; traps
13, 107, 111, 112; `doc/verify.md`'s fuzz lines; `fuzz/seeds.sh`; `fuzz/seed_script.py`.
Code: `fuzz/fuzz_targets/script.rs` (the input's fourth part, `keys_and_rich_value`, the pages'
words), `fuzz/seed_script.py` (`KEYED_SCRIPTS`, `EXTRAS`, eighteen scripts). No test owed: no crash.

## 1. The target and the seeder reach ADR 1762's members

ADR 1762 bridged twenty-one names; the target told the realm every key up, no rich value and no
page's words, so `event.shift`, `modifier`, `keyDown`, `richValue`, `richChange` and the word pair's
answer were unreachable whatever the seeds spelled. An input may now carry a fourth part after a
third NUL — a byte whose low three bits are shift, the modifier and an arrow-key selection, then a
rich text field's `/RV` — and the realm's first page has four words, its second none read (the
refusal), its third an empty list. An input without the part runs as before. The seeder writes the
five scripts that read the part with each of four extras in turn, and eighteen more for the members
the file answers: `lineWidth`, `textSize`, `textFont`, the `font` constants, `console`'s three, the
word pair, `getIntent`, and `util`'s seven (`scand`, `crackURL`, `spansToXML`, `xmlToSpans`,
`streamFromString`, `stringFromStream`, `iconStreamFromIcon`). `event.keyDown` leaves the
"refused by name" block, since ADR 1762 bridged it. Every earlier seed is written byte for byte.

**The part's edges, measured by taking it away** (trap 13): the 240 seeds that carry it reach 5 775
edges, the same 240 with the part cut off 4 991.

## 2. The census before, and the one re-seed

`fuzz/seeds.sh check`, the four side by side in one `--long` hold (asked 22:57:57, 182.0 s queued,
1 170.8 s held, peak 1.78 GiB), disk `INITED cov` against fresh seeds':

| target | disk | fresh | verdict |
|---|---|---|---|
| `script` | 14 240 (7 296 seeds) | 15 461 (8 704) | **stale**, the new seeds leading by 1 221 |
| `script` re-seeded | 15 459 (8 752) | 15 462 | current |
| `script_wire` (wire 13) | 893 (32) | 893 (32) | current — round 1463 re-seeded it |
| `page` | 33 106 (10 826) | 31 808 (15 234) | current |
| `jpeg_bands` | 1 770 (192 419) | 1 583 (1 125) | current |

## 3. The campaign: one `--long` hold, four targets

Asked 23:20:55, granted after 28.3 s, held **3 340.0 s** to 00:17:04, peak **2.15 GiB** — the first
`kind=long` campaign line in `/home/AI/heavy-walk.log`. Built `-O -s none`, run by path from a copy
taken when built, `-fork=1` past every stop, a scratch corpus first and the disk's second, `TMPDIR`
in scratch. Each line's own limits except memory: four targets side by side under the second lane's
6 GiB ran `script`, `script_wire` and `jpeg_bands` at `-rss_limit_mb=1024` and `page` at 3072, which
the peak shows was room to spare. A fork parent runs its whole corpus before its first mutation, so
the time fuzzed is the time ran less the load:

| target | ran | load | executions | cov | crash / timeout / oom | after: disk + finds |
|---|---|---|---|---|---|---|
| `script` | 2 444 s | 105 s | 1 683 705 | 21 913 | 0 / 0 / 1 | 21 934 |
| `script_wire` | 2 416 s | 2 s | 1 248 296 867 | 2 966 | 0 / 0 / 0 | 2 964 |
| `page` | 2 748 s | 345 s | 133 746 | 34 057 | 0 / 0 / 0 | 34 085 |
| `jpeg_bands` | 3 339 s | 1 116 s | 1 304 149 | 2 499 | 0 / 0 / 0 | 2 491 |

The pass after is a `-runs=0` over the disk corpus with the finds beside it, the four side by side
with the census of `script` in one `--long` hold (00:17:22 to 00:29:26, 724 s, peak 1.81 GiB); the
census after reads `script` 15 462 / 15 462, current. **`jpeg_bands` fuzzed 2 223 s, not forty
minutes**: its load grew from ADR 1746's 890 s to 1 116 s, and `-max_total_time` counts from the
parent's start. `cargo fuzz cmin jpeg_bands` in the main checkout stays the remedy ADR 1746 named
(the fork kept 1 562 of the 192 419 files) and writes the owner's corpus, so it is named, not done.

## 4. Every stop, classed

- **`script`'s one memory refusal is ADR 1590's class.** The input runs the seeder's
  `for (var i = 0; i < 20; i++) s += s;` twice over one `s`, the mutator having turned the second
  loop's reset into an assignment to another name, so the string doubles toward 2^40 units inside
  one operator, where no per-call budget looks; the confined worker bounds it by its address space,
  and `pdf-script-worker`'s `growth_past_the_ceiling_ends_the_worker_and_is_named` holds that.
- **No timeout on any target**, so ADR 1424's classes were not reached.
- **`page`'s two slow units are one seed and its one-byte mutation**, a 164 694-byte document in the
  disk corpus (`021ace25`, `8b242bdd`). Alone, each spends 7.7 s on a processor in the campaign
  build, under the line's 60 s, and 2.1 s in a release `callgrind_interpret`: 40.7 G instructions,
  99.8 per cent inside `show_text` running nested content streams through `run` — a Type 3 glyph's
  description — with `NestedContent::of` 20.6 and the lexer 13.9. Interpretation ends inside every
  budget, so it is not a finding; it is a page-cost lead for a round that profiles §9.6.4's glyph
  descriptions.

The finds stay in `scratchpad/r1471/run/<target>/corpus`, the stops in `run/<target>/art`; nothing
reached the owner's corpus but the `script` re-seed (ADR 1423).
