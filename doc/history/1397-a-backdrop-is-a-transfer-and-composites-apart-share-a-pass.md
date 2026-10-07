# 1397: a backdrop is a transfer, and composites apart share a pass

Raster slot, batch sixty-two. ADRs 1630 and 1631; no ledger row moved, no question. Threads of
user AI beside the first heavy run: 137.

**Measured first** (habit 68): a layer texture with and without `COPY_SRC | COPY_DST` on RADV, two
`md5sum`-distinct exports of a scratch probe interleaved, 8 × 300 iterations — allocation, first
pass, later pass, sampling pass, `finish` and `submit` all agree to 0.1 µs at 1600 × 1000 and
160 × 120; callgrind 225 462 against 224 857 instructions an iteration. The usages cost nothing.

**Counted** (habit 63), passes and transfers by site on scratch exports: `bug1721218_reduced.pdf`'s
turn is 193 passes, 59 of them backdrop copies (57 with a source, 2 cleared), 35 composites in a
pass of their own; no page section 3e times seeds a group.

**Built.** ADR 1630: a backdrop with a source is `copy_texture_to_texture` of `onto` into a copy
of exactly its size — 136 passes. ADR 1631: priced first against `peak_layer_bytes` — on the page
the peak holds (578 736 B with pairs or runs waiting), on `personwithdog.pdf` it does not (+896 B
pairs, +6 144 runs) — so a composite waits past the next child only where their rectangles stand
apart and the bytes it keeps alive fit under the plan's priced heaviest child (`Prices`,
`may_wait`): 114 passes, the peak unmoved by construction. Four new tests, each watched failing
(the transfer's origin at zero; the disjointness test removed; the budget test removed).

**Measured**, pinned, eight interleaved runs of five rounds on exports of HEAD, ADR 1630 alone and
both, load 1.0–3.2: turn 151.68–153.85 → 149.77–151.95 → 149.10–151.18 ms, seventh 150.66–152.01
→ 148.62–150.76 → 147.23–151.15, step 83.05–84.49 → 82.73–84.10 → 81.61–83.04; both together
quicker than HEAD in every pair of every row (turn by 0.57 to 3.49 ms). `zoom_frame` 1× 1.63× →
1.58× the CPU backend, step 1.33× → 1.30×: `doc/QUORRA_FEEDBACK.md` section 70. section 3e's three rows
re-taken; `turn-path.toml`'s bands by its rule (turn 126.7–187.5, seventh 125.1–187.0, step
69.3–101.6), `doc/todo/36` row kept true. ADR 1632's interpretation saving is not in these figures.

**Gates.** `rustfmt --check` on the eleven source files: 0. Clippy `-D warnings`, `raster-gpu` and
`render-raster`, all targets: 0. `cargo nextest run -p raster-gpu -p render-raster`: 0, 804 passed.
`cargo test -p conformance`: 0. Behind the lock, exports of HEAD and of the change in their own
target directories, sandbox rebuilt inside: `render-raster --test corpus`, six arms each, all exit
0, digests by name 0 moved of 968 / 968 / 968 / 964 / 963 / 964, verdicts equal, 1-vs-N 0, in
1843 s and 1438 s; `raster_golden` on the change: 0, held 974, moved 0, in 21 s;
`tools/batch.sh raster-examples` on the change: 0, 14 passed, in 158 s; `turn_path` on the old
bands: 0, 27 of 33 judged (load), 0 outside, in 185 s; on the new bands twice: 0 and 0, 33 of 33
judged, 0 outside, in 83 and 94 s.
