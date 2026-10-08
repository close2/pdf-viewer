# 1431 — A round's number in a comment is counted, and a state section walks behind the lock

Slot 6 of batch sixty-seven, 2026-10-08, an instruments round. ADR 1698; no row moved, no question.

**Premise.** Held in substance, not in its figures. The brief's grep gives 55 lines in 21 files; the
lexed sweep finds 59 in 21, the grep missing two sites split across a wrapped line and the
capitalised and plural ones. `state.sh` has 12 grep lines, of which nine are walks; twenty-six more
walks ran under no wrapper at all. ADR 1680's map gave round 911 one ADR where it wrote two (0864,
0865). Records 1420–1425 run 37–40 lines, each with `**Gates.**`, so none needed an edit.

**The sweep.** `tools/conformance/tests/round_numbers.rs` lexes every `.rs` under `crates/`,
`tools/`, `raster/` and `fuzz/` (1 461 files), reads `//` and nested `/* */` comments, skips every
string and character literal, and names `round`/`rounds` then three or four digits, across a line
break inside a comment. Calibrated by a planted source (six named, ten passed) and by the tree:
59 named before the rewrite, 19 after, the held-count check failing on a guessed 22 against 17.

**The rewrite.** 40 lines in 18 unowned files now cite the ADR the round wrote, a sentence whose
subject was the round rewritten to the present reason; `pdf-vfs/tests/confined.rs` cited ADR 0864
for 0865's arena. Held for owners: `launch_path.rs` 17 and `viewer-confined` 1 (slot 2),
`image_phase.rs` 1 (slot 3).

**The walks.** A section may not walk unlocked. `state.sh` gains `walk small|large|clock --`
(four threads, `--round` names the session) and all 35 walks use it: 22 small (merge peak ≤ 3 GiB),
4 large, 9 clock (`clock_gates`, launch, frame, the ratchets loop). A new `bounded.rs` test holds
each walk to the helper and each of the 30 gates the merge shares to its clock-ness; against the old
script it named all 35. `read_only.rs` reads past `walk` and the wrapper's options. Live: `state.sh
--round 1431 dates` queued 204.0 s behind two lanes, held 15.5 s, `kind=small`, exit 0.
`substitution_census.rs` declares `--tree 12` and leaves the held list.

**Unfinished.** The 19 held sites; the `--bins` builds stay outside the lock (ADR 1698 section 4).

**Gates.** `rustfmt --check --edition 2024` on my 22 `.rs` files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy` over `conformance`, `confined-transport`, `pdf-fuse`, `pdf-vfs`, `pdf-vfs-ffi`,
`pdf-transform`, `pdf-model`, `viewer-core` `--all-targets`: exit 0. `cargo nextest run` over the
same eight: 3 240 + 107 passed, 0 failed. `cargo test -p conformance`: 408 passed, 0 failed.
`bash -n` on `bounded.sh`, `batch.sh`, `state.sh`: exit 0 each. `tools/batch.sh check`: exit 0.
`tools/bounded.sh --self-test`: exit 0, every case holds. `state.sh` installed by rename after the
arms' `done` line (trap 130); `bounded.sh` and `batch.sh` untouched.
