# 1398: a group's composed grid is sampled across the pool, and a uniform run of stops is not asked again

Performance slot, batch sixty-two. ADR 1632; no ADR 1633, no ledger row moved, no question.
Threads of user AI beside the first heavy run: 152, and 173 beside the last.

**Counted** (habit 63), callgrind by function on `bug1721218_reduced.pdf`'s interpretation alone
(`examples/turn_interpret`, new: a run of two less a run of one, gates profile, one thread,
pinned): the turn is 1 388.4 M instructions. The brief's premise is not the tree's: nothing is
flattened during interpretation (a `W n` puts §8.5.4's path in the list, 7 229 clips a run at 5.0
thousand instructions; the 37-gon is raster's). The `/DeviceCMYK` group's content is run twice,
chromatic then black, and the second run is 569.5 M (41%), 26 to 33 ms by a probe. Also in
`interp`: the composed conversion grid, 83 521 points, 136.4 M, about 5 ms; 137 ramps at 3.05 M.

**Built** (ADR 1632), exact by construction: the composed grid sampled across rayon's pool, and
`simplify` answering a uniform run of stops without asking each middle (26 of 137 ramps).
**Built, measured and taken out**: the black run handed the tokens the first run lent — 34 M net
on this page and 0.19 to 0.38% on every ordinary page in every shape tried; ADR 1632 section 4.

**Measured** (exports of HEAD and the change, md5-distinct, `frame_budget` interleaved, three runs
of five rounds, pinned, load 2.3–2.9): turn 152.43–157.97 → 146.32–147.45 ms, `interp`
68.21–69.96 → 62.96–63.90, seventh 152.17–155.10 → 147.72–148.58, step 83.34–85.40 → 84.57–85.42.
Callgrind: the turn 1 388.4 → 1 369.1 M (79.8 M of it on the pool); ISO 32000-2 page 101 fifty
times 1 323.45 → 1 318.46 M, all of it `memcpy` (habit 45). `doc/performance.md` 3e's rows and
`turn-path.toml` are round 1397's this batch: the figures are handed over in the report.

**Launch**: no lever reaches it. Page one of each of the six rows, under callgrind, within 0.08%
of HEAD; `launch_path` with clocks on the change, 53 figures banded, 0 outside (HEAD the same
sitting: 0 outside, 1 not judged). `script_open` left as ADR 1620 banded it.

**Gates.** `rustfmt --check` on the three source files: 0. Clippy `-D warnings`, `pdf-model`,
`pdf-render`, `render-raster`, all targets: 0. `cargo nextest run` on those three: 0, 2 232
passed. `cargo test -p conformance`: 0, 396 passed.
Behind the lock, exports in their own target directories, sandbox rebuilt
inside: `render-raster --test corpus` six arms each, all exit 0, digests by name 0 moved of 968 /
964 / 968 / 964 / 968 / 963, verdicts equal, in 1 496 s and 1 443 s of arms; `pdf-model
raster_golden`: 0, held 974, moved 0, in 32 s; `pdf-model --test corpus`: 0, ratchets at their
ceilings, in 14 s; `turn_path` on the change: 0, 33 of 33 judged, 0 outside (HEAD the same sitting:
1 outside, `issue14415.pdf`'s step at 10.73); `launch_path` with clocks: 0. The oracle is the
merge's (tier 3); no pixel moved for it to see.
