# 1522 — One page is read on many threads, divided where `Q` restores the state

Slot 1 of batch eighty-three, 2026-10-12, a parsers round on the owner's performance target. ADR
1880; no ADR 1881, no question. No ledger row moved: the partial-row word yields to the target.

**Premise.** Held: `content/` and `pdf-syntax/src` unchanged since `c6fb18fb`; HEAD read the witness
in 1 768–2 394 ms headless this round under five siblings' load. **Hypothesis.** Did not hold in its
letter: a part is a copy of its commands' headers and an `Interpretation` is one list, so handing
pieces to the drawer adds a copy rather than sparing the join. The join is built instead — one
reservation, each piece copied in as it arrives while the later ones are still being read.

**Built** (`content/divide.rs`, ADR 1880 §2): chunks lexed at once, their stack effects composed,
cuts after a `Q` with only the state open and before the first `BT`, each piece run from its spine's
own statements, the pieces joined in order; what the join does not carry is read whole and says why.

**Measured** (exploration on the second lane, not a clock hold). Witness, 12 threads, 48 pieces:
**585–697 ms against 1 833–1 902 ms whole, 2.6–3.3×**; inflate 145–158 ms, plan 98–117, pieces about
300, join 150–164 before it overlapped. Digest `07eb7445…68e3` (776 097 064 bytes) at 4 and 12
threads. Corpus, four pieces of every stream: 50 of 974 first pages divided, all equal to the whole
reading, 49 drawn while read equal; none divided as shipped, so no arms are owed (ADR 1863). The
drawer sees piece one's parts only; the first frame is the whole draw after the read.

**Unfinished.** The clock hold (24 threads, `launch_path`, `quorra` on Xvfb and the owner's display,
HEAD `d8f0cff8…` against `db072273…`, staged in `scratchpad/r1522/hold.sh`, `tmp/r1522/owner.sh`)
queued 19:23–22:28 behind round 1524's arms on lane 1, withdrawn at the budget. Not taken: ADR
1888's pipelined inflate and byte-scan plan. Still declining: `gs`, marked content, inline images.

**Files.** `pdf-model`: `src/content/divide.rs` (new), `src/content.rs`, `src/content/{run,reader,
list_budget,report,transparency,pattern}.rs`, `src/progress.rs`, `tests/drawn_while_read.rs`,
`examples/read_split_price.rs`; `pdf-render/src/display_list.rs`; ADR 1880; `doc/state-of-play.md`;
`doc/todo/69`; this record.

**Gates.** `rustfmt --check --edition 2024` on every file exit 0; `RUSTFLAGS="-D warnings" cargo
clippy -p pdf-model -p pdf-render --all-targets` exit 0; `cargo nextest run -p pdf-model` 2 046
passed, 21 skipped, exit 0; `-p pdf-render` 282 passed, exit 0. `cargo test -p conformance` one
failure, `names.rs` on `raster/crates/raster-gpu/src/encode/instance.rs:250` (slot 3's file
mid-edit), none in this round's. `t2-drawn_while_read` exit 0 (910 finished; the new sweep 50
divided, 0 different); `t2-corpus` exit 0, ratchets at their ceilings; `t2-raster_golden` held 974,
moved 0, exit 0; `callgrind_interpret --digest` holds. Clock hold: not run. Duration 18 334 s.
