# 1767 — `Vfs` is continued in seven modules by concern

Session 1767, slot 8 of batch 116, the standing Sonnet 5.5 code-quality slot; ADR 2368; ADR 2369 unused; no question.
No bug fixed; no model change; ledger statuses unchanged (seven rows name the new files beside `lib.rs`).

**Choice.** `tools/state.sh gods` named `Vfs` in `crates/pdf-vfs/src/lib.rs` (one impl block of about 1 180 lines, 38
methods, in a file of 2 415). It is not the protocol's one `match`, so `Vfs` was taken and `Carry` left. The methods are
in `mount.rs`, `directory.rs`, `reading.rs` and `writing.rs`, one `impl Vfs` each; the free functions that answer what
the document has are `inventory.rs`; `error.rs` holds `VfsError` and `Refused` and `consent.rs` the seam every worker
question crosses. `lib.rs` fell from 2 415 to 566 lines; the hypothesis of under 500 did not hold (`Config`, the data
types, the generation's structs and the crate documentation are the root).

**Proof.** Token multiset of the non-comment lines, `pub(crate) ` stripped, HEAD's file against the root and its seven
children (`proof.py`, from the script `split.py` whose anchors are asserted): nothing lost; after `use` and `mod`
statements are removed the additions are four `impl Vfs {` heads with braces and two trailing commas from `rustfmt`.
Twenty intra-doc links that resolved through the root's imports are `crate::`-qualified. No method was renamed; no
teaching diagram names `Vfs`; `doc/crate-map.md`, `doc/todo/74` (section 18) and seven ledger rows name the modules.

**Gates.** `cargo fmt --check -p pdf-vfs` exit 0 and `RUSTFLAGS="-D warnings" cargo clippy -p pdf-vfs --all-targets`
exit 0; `RUSTDOCFLAGS="-D warnings" cargo doc -p pdf-vfs --no-deps` exit 0, with and without `--document-private-items`;
`cargo test -p pdf-vfs --no-fail-fast` exit 0 (21, 21, 22 and 17 tests passed, 2 ignored corpus gates); on an export of
this tree, `write_corpus` exit 0 in 37.51 s of test time and `read_corpus` exit 0 in 318.86 s (the `vfs_write` and
`vfs_read` gates); `cargo check -p pdf-fuse -p pdf-vfs-ffi` exit 0. `cargo tree -i pdf-vfs` prints `pdf-fuse` and
`pdf-vfs-ffi` only, so the six arms' binaries do not link the crate and need no re-run. `cargo test -p conformance
--no-fail-fast` exit 0 (142 s of the lock; no red).

**Found, not fixed.** `Vfs::flush` is still about 100 lines and `Vfs::entries_of` about 100; `Consenting::held`
and `Vfs::held` are two lock accessors with one name on two types; `Totals::twice` is a `pub(crate)` field a
method could return instead (`Vfs::asked_twice` clones it).
