# 1447 — `zune-jpeg` is taken from the owner's fork, and both guards go

Slot 4 of batch seventy, 2026-10-08, a dependency round. ADR 1730; ADR 1731 and Q343 not used.
No ledger row changed status. **Premises.** The fork held: `close2/zune-image` branch
`pdf-viewer/0.5.15-with-fixes` is at `1c7d01b8` (ls-remote), both patches on `31d81fed` give it with a
0-byte diff, and its `zune-core` `src/` is crates.io's 0.5.1. Three did not hold. ADR 1589's "nothing
else in the tree takes `zune-core`": `hayro-syntax` takes both crates from crates.io, so the lock now
names each twice. The worker reaches no `zune-*` package, so `worker_features.rs` has nothing of
this one to read; JPEG decodes in the host. And no gate is called `jpeg`. The JPEG gates run here
are `corpus`, `raster_golden` and a census of both decoders.

**The move (ADR 1730), made once at 11:59:38.** That was ten minutes in, not late. It was tried
first in an export, and no cargo was running in the worktree. The `Cargo.toml` stanza at that `rev`, `deny.toml`'s third `allow-git`, `cargo update -p
zune-jpeg` on both locks (root +2 packages; fuzz 2 swapped). In `banded_decodes.rs` the DC test
lost its ignore. The grey-row guard is now `a_complete_last_row_without_its_eoi_is_decoded_as_the_frame`:
the two decodes are equal, and a control checks that the last row is not grey. HEAD's guard held
0.5.15's grey row, so equality fails there. `[patch.crates-io]` was declined: the reference renderer
keeps the release's decoder (trap 9). The patches' preambles, `main-checkout.py`'s docstrings,
`doc/stack.md`'s codec row and `doc/state-of-play.md` now say the fork carries both patches.
`main-checkout.py`'s `patches` already counts a pinned `Fork:` as applied: on this tree it prints
"3 owed …, 2 whose base it no longer pins, 0 for a fork the owner is to create".

**Census.** Both decoders in one release binary: the tracked corpus has 179 DCT codestreams (172
the same, 7 refused by both). A one-in-thirty crawl sample has 31 016 over 1 428 documents, all
the same. The planted reproducer differs at byte 31 200.

**Unfinished.** Stale texts in files other slots own: `image/cut.rs:344–348` ("even where the data
holds every MCU"), §7.4.7's ledger note ("wait on the manifest pinning … `close2/zune-image`"),
`doc/environment.md` around line 637 ("the stanza is written out in a comment"), `doc/todo/65`:257,
and `doc/HANDOVER.md`:55. Neither upstream report is filed yet; filing them is the owner's job.

**Gates.** `rustfmt --check` on `banded_decodes.rs`: exit 0. `cargo deny check`: advisories, bans,
licences and sources ok, 38 duplicate warnings (36 before). `RUSTFLAGS="-D warnings" cargo clippy -p
pdf-model --all-targets`: exit 101 at first on a sibling's mid-edit `scripts.rs`, then exit 0.
`cargo nextest run -p pdf-model`: 2003 passed, 18 skipped. Fuzz `cargo fmt --check`: exit 0. Fuzz
clippy: exit 101 on `fuzz_targets/script.rs` (slot 1's new `DocumentState`/`FieldState` fields),
and exit 0 on the other 31 targets. Behind the lock as `--tree 6`: `corpus` exit 0, 41 s, 1.48 GiB;
`raster_golden` exit 0, 974 held and 0 moved, 31 s, 3.35 GiB; extraction exit 0, 102 s; census
exit 0, 600 s, 0.15 GiB. `cargo test -p conformance`: exit 0, 417 passed, `worker_features` among
them.
