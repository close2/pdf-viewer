# 1839 — An image's samples held on the device are measured against HEAD and taken back

Slot 8 of batch one hundred and twenty-four, 2026-10-27, a "measure" round (Opus), continued from its
first agent's handover; ADR 2512 written, ADR 2513 and Q705 left unused. I worked in `scratchpad/r1839/`.
No ledger status moves and no source file differs from HEAD at the end.
**Premise** (the plan sheet's step is its samples crossing again, which a texture per image removes):
it does not hold as stated. raster-gpu already keeps a texture per image and per reduction (ADR 1493);
HEAD's step sends 13 534 128 bytes in no upload, the host's reduction of the cells the new factor needs.
**Built, then taken back.** A compute shader re-implementing the host's area averaging and premultiply in
integers, the samples as a storage buffer charged to the device's residency budget with a public
refusal, the pipeline compiled on the warm-up thread only (last in the warm set where the device did
not ask for the compute lane, which `encode_threads_nested`'s counters needed: compiled after the set,
the first frame's path was a race), and two tests — byte-equal frames at six extents and both filters,
and a refused image drawn by the host; a truncating division planted fails both.
**Measured** in one `--clock --nice 0` hold, 2 049 s, HEAD and the tree interleaved, the load beside
each arm (2.84 to 16.32 at the starts), no render-node holder of this account, arms compared on
`pp_dpm_sclk`'s middle line: the plan sheet's turn gate reads turn 65.51 and step 20.79 ms (both outside
their bands) against HEAD's 53.09 / 15.97 and 54.62 / 16.31; `frame_budget` step 29.65 against 21.25.
A stage probe places the cost: the samples' 104 360 320 bytes cross on the turn in 16.4 to 33.7 ms,
the shader takes 2.3 to 6.3 ms of device time, and an exploratory run after the hold puts the step's
new textures above HEAD's whole step transfer. `images.pdf` and `issue13931.pdf` steps fell by about 4 ms; every image turn rose.
**Decision** (ADR 2512): taken back, the figures moving the wrong way on the page the slot names; the
ADR prices what would make the lever pay (the crossing off the turn, the step's textures at HEAD's
cost, a per-image lane by a measured prediction).
**Found, not fixed:** the turn gate failed on HEAD in both of HEAD's arms, one figure outside each
(`personwithdog.pdf` step 14.91 ms against 7.10 .. 12.50 at load 2.84; `bug1743245.pdf` seventh 49.11
against 29.70 .. 49.10 at load 16.32) — HEAD's own, at the loads beside them. **Proposed habit:** before
pricing a residency lever, check what an RFC's "sent again on every step" means against the cache the
code already keeps.
**Gates.** Hold: `frame_budget` 4 arms exit 0, probe 2 arms exit 0; turn gate HEAD exit 101 twice (1
outside each), the tree exit 0 (not judged, load 9.39 rising to 31) and exit 101 (8 outside); launch
gate exit 0 on both, 0 outside, cold bring-up 18.5 ms each. Built tree, before the take-back: `cargo
test -p raster-gpu` exit 101, 1 red (`encode_threads_nested`), then after the warm-set fix its lib 291
passed and that test 4 passed in 6 of 6 runs, clippy and fmt 0;
`cargo test -p render-raster` exit 0, 112 passed; the six arms (`render-raster`'s corpus test under each lane and scale) against HEAD's six arms exit 0 each, 0 of 968 (1×) and 0 of 965 (4×) pages differing in every arm.
Final tree: `cargo fmt --check -p raster-gpu` 0, `RUSTFLAGS="-D warnings" cargo clippy -p raster-gpu
--all-targets` 0, rustdoc `-D warnings --document-private-items -p raster-gpu` 0; `raster_golden` not
run, `cargo tree -p pdf-model -i raster-gpu` finding no path; `--bin cited` exit 0; `cargo test -p
conformance --no-fail-fast` exit 0 on the last run (an earlier one had two reds, record 1837's and
`pdf-transform`'s, neither mine).
