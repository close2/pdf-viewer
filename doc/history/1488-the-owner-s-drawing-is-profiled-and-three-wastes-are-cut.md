# 1488 — The owner's drawing is profiled, and three wastes are cut

Slot 3 of batch seventy-seven, 2026-10-10, a measure round asked for by the project owner ("if we
find a way to improve loading, we should improve this as well"). ADR 1813; no ADR 1814, no question.
No ledger row moved: §8.6.5.5, §8.6.8, §8.4.3.2, §7.8.2 stay `implemented`, §10.7.4 `departed`.

**Premise.** Held: 278 579 415 inflated bytes, 4 400 060 `S`, 1 288 210 `CS` on
`/Cs1 [/ICCBased 5 0 R]`; HEAD's `quorra-transform render` takes 15.37 s pinned, 1.34 GiB.

**Hypothesis.** Did not hold. Callgrind at 150 dpi: 217.6 G instructions, of which rasterising is
148.3 G (68 %) and interpreting 66.7 G (31 %); inflating is 2.2 G (1 %). Top five: exact-area
`mask_fill` 25.1 %, `Path::stroke` 18.1 % (twice per stroke), `fill_rect` through the mask 9.0 %,
the ICC profile re-parsed per `CS` 8.1 %, the lexer 5.1 %.

**Built.** The cuts are waste removed with each answer handed back, so the pixels are the same by
construction. `render_cpu::SolidOutline` builds a stroke's undashed outline once for
`draw_rule_as_bands` and `draw_stroked_outline`. `Interpreter::icc_spaces` now also remembers an
`[/ICCBased s]` array written in place, keyed by the stream (`Stated`). `may_open_a_string` sends
only `(`, `<`, `%` or the buffer's end on to `step_over_a_string`. `callgrind_rasterise` takes
`--scale=`. New test `a_profile_selected_again_starts_at_its_initial_colour_…`: a planted memo hit
that skips §8.6.8's initial colour fails it (trap 13). At slot 1's request, the stale
`MAX_OPERATIONS` comments in `text.rs` and `document.rs` now give the reason that holds today
(ADR 1809).

**Moved.** The whole page goes from 217.6 G to 178.0 G instructions (−18.2 %) before the string
check, which saves another 2.39 G of 48.0 G interpreting. Wall time, pinned, minimum of three
interleaved runs at load 3.8: 15.37 s → 11.24 s (−26.9 %). The PNG has one MD5 across all three arms.

**Not built.** The page is drawn serially: wall time equals user time at four threads, because
`pdf_render::strips` marks a stroke's whole extent unsplittable and the page frame's `re S` spans
every row. That is about three times on the raster, and it needs its own ADR on strip invariance
under `render_cpu::area`. The cause comes from reading the code; it was not isolated by a run.

**Gates.** `rustfmt --check --edition 2024` on the seven Rust files exit 0; `RUSTFLAGS="-D warnings"
cargo clippy -p pdf-syntax -p pdf-model -p render-cpu --all-targets` exit 0; `cargo nextest run`
`-p pdf-syntax` 294 passed, `-p pdf-model` 2036 passed, `-p render-cpu` 176 passed, each exit 0;
`cargo test -p conformance` exit 0. Tier 2, behind the lock: `raster_golden` exit 0 (held 974, moved
0), queued 1 002 s; the six arms against `/home/AI/arms-1486/`, each exit 0, 0 pages moved by digest
or mean error in any, 1 320 s held, queued 0 s; the `--clock` A/B exit 0. `turn_path` was not owed,
because it drives `render-raster`, which no change reaches. Duration 6 900 s.
