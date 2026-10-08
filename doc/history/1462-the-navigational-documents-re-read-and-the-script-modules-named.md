# 1462 — The navigational documents re-read, and the script modules named

Slot 1 of batch seventy-three, 2026-10-08, a docs round. ADR 1760; ADR 1761 and Q358 not used. No
ledger row moved.

**Premise.** Held. `wc -l` printed 756, 64, 1 052 and 88. The last re-read is `1423`'s; `1436`'s hit
is a title about blockers, and `1456`'s says the re-read was skipped.

**Hypothesis.** Half held (ADR 1760 section 3). Seven of the fourteen false claims are what batches
67 to 72 built, each an omission: the script modules of `pdf-script`, `pdf-model`'s `view/`,
`viewer-host` and `viewer-core`, the popup module, the lock's `--long` kind, the golden's divided
draw, `planckian.rs`, the rich text kerning and the signature policy. No sentence named a
construction those batches retired. The three false constructions were older: "no moc" while
`viewer-qt`'s build script runs it; `pdf-sandbox/build.rs` said to bake the worker's path, which
stamps a build identity (ADR 0458); and §8.9.5.2 given as `partial` where the row is `implemented`.

**The re-read.** `cargo metadata`: 43 members, 43 rows. `tools/state.sh navigation`: one absent
pointer, the owner's `A170`, and zero history hits in the four. The extractor was calibrated on a
planted file (3 of 3 named), then resolved every backticked path and identifier. A census of the
source files added since `fe5d66eb`, and of each row's unnamed modules, chose what to read closely.
ADR 1760 section 2 lists each correction and the command that checked it.

**Lines.** `doc/PLAN.md` 756 → 765 (9 115 → 9 248 words), `doc/crate-map.md` 64 → 64 (10 524 →
10 902), `doc/state-of-play.md` 1 052 → 1 061 (16 629 → 16 769), `doc/HANDOVER.md` 88 → 88 (1 618, no
correction). No document lost lines; four chronology phrases went. Section 0 gained the `--long`
kind. `traps/every-round.md`'s "the other hundred-odd are cited once each or never" was false and
was replaced. `habits/every-round.md`'s "about seventy habits" is now the `grep -c` that counts them.

**Left to slot 5.** The rule block's `RAYON_NUM_THREADS=4` overrides `bounded.sh`'s own share; the
lock lines are slot 5's, so the rule block has no hunk of this round's. `PLAN.md` section 5a's
`comment-history.py` sentence stays true under slot 5's ADR 1767, which keeps the script.

**Gates.** `cargo test -p conformance --no-fail-fast`: exit 101, 424 passed and 1 failed:
`batch.rs`'s `check_compiles_the_fuzz_workspace…`, in slot 5's file mid-edit (it asks for the small
lane, and HEAD's `batch.sh` takes the large one). `records`, `documents`, `traps` and `names` pass.
`--bin quotations`: exit 0, no divergence in a file touched. `--bin pointers`: exit 0, 20 absent,
as before; in the four, only `A170`. `tools/batch.sh check`: exit 1, on siblings' files only: the
fuzz `script` target lacks `Layer`'s new `intent` field, and `cargo fmt` differs on `popup.rs`,
`panel.rs` and `pdf-model`'s `lib.rs`. No Rust file is this round's: no `rustfmt` or `clippy` owed.
Duration 2081 s.
