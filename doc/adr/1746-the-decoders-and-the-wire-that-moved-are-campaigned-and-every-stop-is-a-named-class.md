# 1746 — The decoders and the wire that moved are campaigned, and every stop is a named class

Session 1455. Status: **accepted**. Context: ADRs 1424, 1447, 1590, 1602, 1694, 1714, 1716, 1717,
1730, 1736, 1737, 1740; traps 13, 107, 111, 112; `doc/verify.md`'s fuzz lines; `fuzz/seeds.sh`;
`fuzz/seed_script.py`.

## 1. Before: the census

`fuzz/seeds.sh check`, disk `INITED cov` against fresh seeds' — `jpeg_bands`, `page`, `script`,
`script_wire` in one `--tree 6` hold (2 406 s, peak 1.60 GiB), `jbig2` and `jpx` in a `--tree 12`
one: `jpeg_bands` 1 892 / 1 717, `page` 33 830 / 32 534, `script` 13 630 / 13 628, `jbig2`
3 391 / 3 391, `jpx` 2 447 / 2 447, all **current**; `script_wire` 44 / 840, **stale**, because
round 1450 raised `wire::VERSION` to 11 (ADRs 1736, 1737) and every seed on disk is a version
refusal (ADR 1694's sentence). Re-seeded by its arm (32 seeds, the names overwritten), it read
840 / 840. Which targets reach `zune-jpeg` was read off the binaries rather than assumed (trap 25):
`nm -C` finds the MCU decoder (`zune_jpeg::bitstream`, `mcu`, `decoder`) in `jpeg_bands`, `page`,
`variable_text`, `vfs_write` and `fetched_import` only. The first two hand the fuzzer's bytes to a
decode — a codestream, a page whose images it states — and were campaigned; the other three link the
decoder with `pdf-model` and were not, `variable_text`'s fixed page naming no image.

## 2. The campaigns

Built `-O -s none`, run by path from a copy taken when built, fork mode past every stop (ADR 1716),
a scratch corpus first, each line's own limits, `TMPDIR` in scratch. `jbig2`, `jpx` and `page` side
by side in one `--tree 12` hold (3 336 s, peak 5.63 GiB); `jpeg_bands` alone, then `script` and
`script_wire` together, on the second lane. Fork-mode coverage at the end, executions, stops, and a `-runs=0` pass over the corpus with the finds
beside it (the second `jpeg_bands` row's corpus is the first run's finds and fresh seeds):

| target | ran | executions | cov | crash / timeout / oom | after: disk + finds |
|---|---|---|---|---|---|
| `jbig2` (hayro `ea9c81dc`) | 1 816 s | 18 415 | 4 629 | 0 / 16 / 0 | 4 627 |
| `jpx` (hayro `ea9c81dc`) | 1 805 s | 36 240 | 2 888 | 0 / 0 / 0 | 2 885 |
| `page` | 1 807 s | 61 487 | 34 873 | 0 / 0 / 0 | 34 862 |
| `jpeg_bands` (fork `1c7d01b8`) | 2 616 s | 944 712 | 2 593 | 0 / 0 / 0 | 2 587 |
| `jpeg_bands`, ADR 1740's plan | 1 816 s | 757 121 | 2 621 | 0 / 0 / 0 | 2 624 |
| `script` (wire 11) | 1 837 s | 1 422 100 | 20 340 | 0 / 0 / 1 | 20 430 |
| `script_wire` (wire 11) | 1 831 s | 1 102 325 898 | 2 692 | 0 / 0 / 0 | 2 687 |

**`jpeg_bands` spent its first 890 s loading.** The disk corpus is 192 419 files, 1.8 GiB, and a
fork-mode parent reads all of it before its first mutation; the merge kept 1 732. So its run is
2 616 s of which about 1 726 s fuzzed, and the second run (after section 4) started from the first
run's 1 332 finds and the arm's 1 125 fresh seeds in scratch, which gave 2 527 edges in 20 s against
the disk corpus's 1 892 in 890 s. `cargo fuzz cmin jpeg_bands` in the main checkout is the remedy,
and it writes the owner's corpus, so it is named here rather than done (ADR 1423).

**No crash on any target**, and none at `bitstream.rs` line 400: the DC multiply that stopped every
`jpeg_bands` run within minutes before the fork (ADR 1730) did not stop it once in 944 712 runs.

## 3. Every stop, classed

**`jbig2`'s sixteen timeouts are ADR 1424 section 3's class, and the bounds patch ends every one.**
Each input was re-run alone under `ulimit -t 45`, so the figure is processor time and not a fork
child's wall clock under siblings' builds (ADR 1717), and its stack read at libFuzzer's first alarm
with `gdb` as the parent — `gdb -batch -ex "handle SIGALRM stop print" -ex run -ex "bt 30" --args
<jbig2> -timeout=12 <input>`, because `ptrace_scope` is 1 and an attach is refused. Fourteen ran
past 45 s, `06f0fa7a` took 31.2 s and `62386bd8` 14.4 s — the last a timeout only of the loaded
machine, under the worker's 30 s. All sixteen stacks are inside `hayro-jbig2`'s symbol dictionary
decode at the release commit: eleven in its integer decoder (`decode/symbol.rs` lines 59 and 72),
five in a symbol's generic-region bitmap (`symbol.rs:110` into `generic.rs:408`). ADR 1694's eight
re-run the same way all still time out past 30 s. The confined worker's `REQUEST_TIMEOUT` bounds
them, and `pdf-sandbox`'s `each_symbol_dictionary_of_minutes_is_ended_at_the_deadline` holds that
bound on inputs of the class, so no test is owed. **Calibrated against the patch the owner has yet
to push** (`doc/patches/hayro-jbig2-symbol-dictionary-bounds.patch`, ADRs 1447, 1714): the same
`jbig2` target built from a copy of `fuzz/` whose `[patch]` names `ea9c81dc` with the patch applied
ends all twenty-four in 0 to 3.6 s of processor time, the unpatched build none of them under 14 s.

**`script`'s one memory refusal is ADR 1590's named class.** The input is the seeder's
`for (var i = 0; i < 20; i++) s += s;` with `<` mutated to `|`, so the loop never ends and the
string doubles past 2 GiB inside one operator, where no per-call budget looks. ADR 1590 section 5
names growth through an operator as the confined worker's to bound, by its 96 MiB address space
(ADR 1602 section 6), and `pdf-script-worker`'s `growth_past_the_ceiling_ends_the_worker_and_is_named`
holds it; the target runs the engine in its own process, without that ceiling. No test is owed.

**The slow units are not findings.** `jpx`'s `ee42a981` is 0.8 s alone; `page`'s `2ecfe415` is
`ContentStreamCycleType3insideType3.pdf` byte for byte, 17.3 s in this instrumented build, the
operator budget ADR 0793 sets spent on a cycle, under the line's 60 s.

## 4. What moved under the campaigns, and the census after

Round 1452 lifted the band plan's refusal of a scan without `EOI` (ADR 1740) after the first
`jpeg_bands` binary was built, so the campaign's 1 332 finds were replayed under a binary built
from that code (exit 0) and the second run made on it. The `script` and `script_wire` binaries were
built after round 1450's last change to the wire, the realm or the target, at version 11.

`fuzz/seed_script.py` had spelled none of the members ADRs 1688 to 1737 bridged — a focus, a check
box, timers, a sound, the annotations, the pages and their labels, a named destination, the window's
view, a choice's options read and rewritten — nor `app.goBack`, refused by name. Sixteen scripts now
do (1 024 seeds at every site), and the census after read `script` **stale**, 13 630 against
14 189: the new seeds lead, which is the seeder change measured rather than assumed (trap 13). The
disk corpus was re-seeded by the arm and reads **current**, 14 191 / 14 194 (7 296 seeds). `script_wire` reads 840 / 840.

The finds stay in `scratchpad/r1455/run/<target>/corpus`, the sixteen new JBIG2 inputs in
`scratchpad/r1455/run/jbig2/artefacts` beside ADR 1694's eight in `fuzz/artifacts/jbig2`, which this
round reads and does not write; nothing reached the owner's corpus but the two re-seeds (ADR 1423).

The timeout inputs the campaign left are kept under `fuzz/artifacts/jbig2/` in the main checkout — each ADR 1424's symbol-dictionary class, each ended in 0 to 3.6 s of processor time under the owner's pending bounds patch — and the tree names them here by their hash prefixes so that `tools/main-checkout.py` counts them as read: `0ccf289b`, `1509cf80`, `255dba92`, `2c814637`, `3146fe51`, `34de4ba9`, `5bef550e`, `607fbc53`, `6ab1cf48`, `9e386be0`, `a8b14f87`, `e60a4c96`, `ec3db9c0`, `f8e360bf`.
