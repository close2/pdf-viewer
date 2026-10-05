# 1339 — Three turns bisected, a scan without its EOI is cut at its restarts, and the table is a gate

Extra round of batch fifty-two. ADR 1513 (ADR 1514 not needed: the launch band was round 1336's,
ADR 1507 section 2). No ledger row moved; no question written. I did not open `crates/render-cpu/`.

**The premise was off.** Batch fifty-one moved none of the three. Exports of six commits, each its
own target directory, pinned and interleaved: the photograph is ADR 1495's no-`EOI` decline (69.4 →
32.5 with it out); `issue14415.pdf` is ADR 1443's two chords a flat piece (`29df1ce9`; 16.1 → 11.6
with the line out — the +50% ADR 1443 priced and ADR 1455 kept); the Type 3 page is the same line
(about 4.5 ms) plus ADR 1467's mark edges built on the walk's thread (`8566f66e`, +3.6 ms; callgrind:
the commit's meet 17 M → 109 M instructions, 87.6 M of it the mark's `RowEdges::of`). ADR 1491's
generic fill cost nothing (225 M → 181 M instructions by name).

**Fixed.** The restart plan admits a scan with no `EOI`: T.81 sections E.2.3 and E.2.4 end a scan on
its MCU count, and the last band reads the tail as the whole decoder does, strict, refused where the
data runs out before a row; the cut plan keeps the decline. A fan-out job above 1 024 point-rows
builds its mark's edges over its tile (`RowEdges::of_charged`), the meet asking the bound its own
rows would (`charge`). Photograph turn 69.40 → 32.45, Type 3 12.24 → 10.50, the rest inside spread.

**The gate.** `crates/render-raster/tests/turn_path.rs` over `tests/support/frame_cost.rs` (the
method, now `frame_budget`'s too) and `doc/checks/turn-path.toml`: twenty figures, three pinned
children of five rounds, the launch gate's calibration probe and load ceiling, a command-count
witness, bands min × 0.85 .. max × 1.20 over eleven runs. `tools/batch.sh gates` runs it as
`t2-turn_path`; `doc/verify.md`, `doc/todo/02`, `doc/performance.md` 3e (re-taken), `doc/todo/36`.

**Gates.** `rustfmt --check` on my eleven files exit 0; clippy `-D warnings --all-targets`:
`pdf-model`, `raster-gpu`, `render-raster` exit 0. nextest: `raster-gpu` 670 passed, `render-raster`
100, `pdf-model` 1 763, all exit 0; `cargo test -p conformance` 373 passed, exit 0. `jpeg_bands`
`-runs=0` on 320 seeds exit 0, then 600 s: 561 135 runs, 2 461 edges, clean. Behind the lock:
`raster_golden` held 974, moved 0, exit 0; `pdf-model --test corpus` exit 0; `render-raster --test
corpus` 1× 967/0/0/7 and 4× 963/0/3/8, no thread-count move at either, both exit 101 on
`ContentStreamCycleType3insideType3.pdf` leaving the device-refused lists — round 1336's list bound,
as round 1335 found; `headless_gpu` 39 passed; `launch_path` 26 banded, 0 outside, exit 0;
`turn_path` 20 of 20 inside, exit 0, 20 s — with the decline put back it failed at 68.83 (watched).

**Left.** The corpus gate's two refusal lists are 1336's to update. The whole decoder still
128-fills a complete last MCU row its lookahead overreads (ADR 1495's fixture), a `zune-jpeg` fix
beside Q227's. The text page's turn is bimodal (7.2 / 8.9 ms, pinned); its band spans both.

**Proposed habit.** A table re-taken by hand is held by a gate before the round that re-took it ends.
