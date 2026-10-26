# 1765 — A run is tested eight pixels at a time, and the decoder's time is split by stage for the fork

Session 1765, slot 6 of batch 116, a measure round; 2026-10-26. ADRs 2364, 2365; Q628 unused. No
ledger row moves: the conversion keeps every byte and the decoder is not touched, so no clause's
reading changes.

**Premise** held: the memo is 455.4 M instructions, 17.4 a pixel, on `22060_A1_01_Plans.pdf`.
**Hypotheses.** A row-wise exact form halving it held, 17.4 → 7.4 a pixel. One codec stage worth
more than half held for a stage the brief's list did not name: entropy decoding, 56 %; upsampling,
the inverse DCT and colour conversion are 3, 14 and 13 % (ADR 2365).

**Built.** `convert_three` tests the run a block of eight pixels at a time and copies a block wholly
inside it with vector stores; the memo holds the converted pixel's word and is keyed by the samples'
own word (ADR 2364); the disassembly shows two 128-bit compares a block (habit 188). `convert_three`
455.4 → 193.4 M on the plan, 249.5 → 206.4 M on `issue19971.pdf`; digests equal on eight pages; the
turn gate does not see it on the plan. **Priced, not built**: skipping
the `ICCBased` conversion (not exact), the 2²⁴ table (24 M of misses for 64 MB a band),
`convert_four`'s run (unmeasured); three levers in the fork, at most about 211 M on the plan — a
scalar DC-only inverse DCT, a `Result` dropped out of line a block, a `memset` call a block (ADR 2365,
`deferred`).

**Files.** New: ADRs 2364, 2365, this record. Edited: `pdf-model/src/image/{convert.rs, tests.rs}`,
`doc/performance.md` section 3e. I worked in `scratchpad/r1765/`.

**Gates.** Tier 1 for `pdf-model` in the worktree: `rustfmt --check` on both files exit 0, `cargo
fmt --check` exit 0, clippy (`-D warnings`, all targets) exit 0 on its second run (the first, 101,
was a sibling's `render-cpu` mid-edit), `cargo nextest run` 2 202 of 2 202, rustdoc with warnings
denied exit 0; four planted defects each fail a route test (2 run, 1 or 2 failed each). On the
export: `raster_golden --ignored` exit 0, held 974, moved 0; against HEAD's six arms, cpu, gpu and
compute at 1x and 4x each exit 0, 968 pages at 1x and 965 at 4x, and no page differs in frame digest
or mean error (1 734 s under the lock; the comparison named a planted digest). Clock, one hold,
H F F H and H F, load 2.90 to 3.38: `t2-turn_path` exit 101 on all four arms — HEAD's first 4
outside of 12 judged, then 1 of 33, 1 of 27 and 1 of 33, `issue19802.pdf`'s step on each alike;
`t2-launch_path` exit 101 on both, 5 outside on HEAD and 6 on the change, none on an image page,
`open_anon_mib` within 0.08 MiB and `peak_anon_mib` within 1.0 MiB of HEAD's on all six documents
(ADR 2364 section 3). `tools/state.sh docs` exit 0. `cargo test -p conformance --no-fail-fast`
with this record and both ADRs: exit 0, 69 test binaries.

**Unfinished.** The Huffman loop, the photograph page's stage worth more than half, is the fork's
design question and is not priced; the three fork levers wait for the owner (A227).
