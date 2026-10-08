# 1455 — The decoders and the wire that moved are campaigned, and every stop is a named class

Slot 6 of batch seventy-one, 2026-10-08, a fuzz round. ADR 1746; no row moved, no question.

**Premise.** Held: hayro at `ea9c81dc` and `zune-jpeg` at the fork's `1c7d01b8` in `fuzz/Cargo.lock`;
`wire::VERSION` was 10 at HEAD and round 1450 moved it to 11, so `script_wire`'s disk seeds read 44
edges to fresh seeds' 840 and were re-seeded. Which targets reach `zune-jpeg` was read with `nm`:
`jpeg_bands` and `page` take the fuzzer's bytes to a decode, three more only link it.

**Campaigns**, fork mode past every stop, about 30 min each: `jbig2`, `jpx`, `page` in one
`--tree 12` hold; `jpeg_bands` twice — on HEAD (2 616 s, 890 s of it loading 192 419 files), then on
round 1452's lifted band plan (ADR 1740) from the first run's finds; `script` and `script_wire` at
version 11. **No crash anywhere**, none at the DC multiply the fork fixed. Edges, census before →
disk plus finds after: `jbig2` 3 391 → 4 627, `jpx` 2 447 → 2 885, `page` 33 830 → 34 862,
`jpeg_bands` 1 892 → 2 587 and 2 527 → 2 624, `script` 13 630 → 20 430, `script_wire` 840 → 2 687.

**Stops, classed (ADR 1746 section 3).** `jbig2`'s 16 timeouts: re-run alone under `ulimit -t 45`,
14 past 45 s, one 31.2 s, one 14.4 s (a loaded-machine timeout); every stack, read under `gdb` at
libFuzzer's first alarm, is in `hayro-jbig2`'s symbol dictionary decode — ADR 1424's class, held by
`each_symbol_dictionary_of_minutes_is_ended_at_the_deadline`. The owner's pending bounds patch, built
into a copy of the target outside the tree, ends all 24 (these and ADR 1694's 8) in 0 to 3.6 s.
`script`'s one OOM is `s += s` in a loop the mutator made endless, ADR 1590's operator growth, held
by the worker's `growth_past_the_ceiling_ends_the_worker_and_is_named`. Slow units: `jpx`'s is 0.8 s
alone, `page`'s is `ContentStreamCycleType3insideType3.pdf` itself (ADR 0793's budget). No test owed.

**Seeder.** `fuzz/seed_script.py` spelled none of the members ADRs 1688–1737 bridged; 16 scripts
added. The census after read `script` stale (13 630 / 14 189, the new seeds leading); re-seeded, it
reads 14 191 / 14 194 with 7 296 seeds. `doc/verify.md`: the `jpeg_bands` sentence about the
unfixed multiply replaced, and how a timeout and an OOM are classed. `doc/state-of-play.md`: one
clause on the patch.

**Unfinished.** `cargo fuzz cmin jpeg_bands` in the main checkout (writes the owner's corpus).
The 16 new JBIG2 inputs stay in `scratchpad/r1455/run/jbig2/artefacts`, finds in `run/*/corpus`.

**Gates.** `cargo test -p conformance --test fuzz_workspace`: 4 passed, exit 0. `cargo check
--manifest-path fuzz/Cargo.toml`: exit 0. `cargo test -p conformance`: 397 passed, 1 failed —
`records.rs` on `doc/history/1453-*.md`, slot 4's record mid-write, not this round's.
`fuzz/seeds.sh check`: before, 5 current and `script_wire` stale; after, `script_wire` 840 / 840
and `script` 14 191 / 14 194 current. Seven campaigns, libFuzzer exit 0 each (`jbig2` 70, its
timeouts); `tools/bounded.sh` exit 0 on all eight holds. No `.rs` touched, so no rustfmt or clippy.
