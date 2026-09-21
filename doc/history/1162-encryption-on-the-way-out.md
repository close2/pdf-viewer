# 1162 — Encryption on the way out

Date: 2026-09-21. Branch: `batch-1159-1164`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
ADRs: [1161](../adr/1161-encryption-on-the-way-out-is-asked-for-never-inherited.md) and
[1162](../adr/1162-a-derived-file-is-protected-where-the-caller-supplies-the-means.md).

## The contract

§7.6.4.4.7, §7.6.4.4.8 and §7.6.4.4.9 — Algorithms 8, 9 and 10 — out of `writer-side`, by building
what `CLAUDE.md` made their condition: §7.6 emitted by RFC 0002 section 10's serializer; plus the
consumers, on the owner's revisit note against ADR 1124, every factual claim of which held.

## Two things found by reading rather than by a gate

**Errata Collection 3's Issue #439 binds this writer for the first time.** It appends an encrypted
document's catalog to §7.5.7's shall-not list; `packable`'s comment had recorded it as satisfied
"the same way and only that way" — by there being no encrypted output — and it became a live
constraint the moment there was one.

**A `/Crypt` filter carried from a source would have produced a silently unreadable stream.** The
serializer copies `/Filter` arrays through; our reader resolves such an entry's `/DecodeParms
/Name` against `/CF` and falls back to `Identity`, so a stream encrypted under `/StmF` anyway is
ciphertext a reader passes through untouched. `Protected::method` asks the output's own dictionary
the questions `Document::stream_method` asks; `Written::cleartext` counts what stays in the clear.

## The tests, and why they are not a round trip

Eleven cases in `crates/pdf-syntax/tests/serialize_encrypted.rs`, read back by Algorithms 11, 12,
13 and 2.A — code that has opened seven real producers' files for hundreds of sessions. Beyond the
trip: the clause's structural statements about `/U`, `/O`, `/UE`, `/OE` and `/Perms`; determinism
under a fixed randomness source; a refusing source stopping the write; and for `/Perms` a
**tamper** — `/P -4` rewritten to `/P -8` in the bytes, the reader still reporting the block's
permission, because a correct file's would have been answered by `/P` alone (trap 27). Two in
`crates/pdf-transform/tests/redact.rs` take an encrypted document through the verb.

## Files touched

`crates/pdf-syntax/src/{crypt,serialize,lib,document}.rs` and `tests/serialize_encrypted.rs`;
`crates/pdf-transform/src/{lib,split,merge,pages,optimize,redact}.rs`, `src/bin/quorra-transform.rs`
and `tests/redact.rs`; `doc/conformance/ledger.toml` (six rows), `doc/state-of-play.md`, the ADRs.
