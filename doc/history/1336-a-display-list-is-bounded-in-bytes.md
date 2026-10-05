# 1336 — A display list is bounded in bytes, and the fax target was never stuck

Robustness slot of batch fifty-two. ADRs 1507, 1508. No ledger row moved; no question written.

**The 3.16 GiB page (ADR 1507).** Not the two mutated bytes: the unmutated
`ContentStreamCycleType3insideType3.pdf` builds 3 929 387 commands and 2 047 756 clips (peak
1.8 GiB, open 0.8 MiB, 7–8.5 s), and the `page` target holds two interpretations. Each tiling copy
carries the cell's clip and no budget counted a clip. `MAX_LIST_BYTES` (512 MiB,
`content/list_budget.rs`) is asked inside the comparison the loop already made against
`MAX_OPERATIONS`, every 64 operators: new commands, the dash in force and new clips with their
paths; tiling copies are charged as made. `Unsupported::ListBytes { charged, bound }` once;
`Interpretation::list_bytes`. Census (`examples/display_list_census`, 90 150 first pages, twice):
largest inside every other bound `poppler-12206-0.pdf` 214.3 MiB (bound 2.4×); refuses no page but
the cycle. The cycle stops at 0.82 GiB, 2.5 s; the fuzz binary 1.43 GiB, 13.4 s. Drawn and reported
in `quorra` and `quorra-confined` under Xvfb. **No clock in `interpret`**: it would make the prefix
depend on load. Fixture: `hostile_budgets.rs`, generated, with its no-cycle control.

**Launch gate.** A per-mark charge cost 2.0 k on `xfa_filled_imm1344e.pdf`'s open; the kept build
costs nothing (1821.46 k either way). The overrun was a 49th `/proc/self/maps` line parsed by std's
stack guard (base 1819.4 k); ceiling 1820 → 1825 k with the reason (`doc/checks/launch-path.toml`).

**Drive.** Step 27 now opens `drive-coverage.pdf` (the device draws the cycle now); step 28 retakes
its photograph until the processor's page lands — its one blank run of six was that race, and
`drive-coverage.pdf` charges 0.1 MiB. Both in `tools/drive-windows.sh`, 1333's file.

**`ccitt` (ADR 1508).** Premise wrong: six head bytes already choose Table 11's parameters and
`seed_streams.py` prefixes each stream's own. Coverage: `lib.rs` 93.4% of regions, `bits.rs`
99.4%; the misses are `Display`, `from_k` and two unreachable returns. 600 s: 223 → 223, 113 M runs.

**Slow units, first ten of 67.** None is the cycle: four take 43–131 ms now, one is 16.7 M
font-less text shows, five are ordinary image-and-font pages at 6–14 s instrumented.

**Gates.** rustfmt `--check` 0; clippy `-D warnings` pdf-model/pdf-ccitt/viewer-core 0; nextest the
three 2080 passed; `cargo test -p conformance` 373 passed, 0 failed; fuzz build exit 0. Behind the
lock: `pdf-model --test corpus` exit 0 (59/59); `raster_golden` 974 held after the cycle's list
hashes were regenerated (raster hash unchanged), exit 0; `launch_path` exit 0 at 1821.4 k; `page`
600 s 31 617 → 31 864, peak 1.59 GiB, no OOM; drive `--window quorra` 31 works 0 wrong,
`--window quorra-confined` 3 of 3 works.
