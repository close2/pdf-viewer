# Round 2200 — a stream is one request answered by items the host grants

Video's first round (RFC 0015 section 6, round 1; A1006), in the `audit` worktree: the streaming
arm in `confined-transport`, and the clause 13 exclusion's video half restated.

## What changed

- ADR 3501: `confined_transport::stream` and `Host::stream`. `OPEN` (the protocol's kind, a first
  grant, its request), `CREDIT`, `STOP`, `ITEM` (sequence, carriage, body), `END` (finished or
  stopped, a closing). Credit is granted only once the last grant has arrived, and a worker with
  none reads before it writes, so no control frame is ever left stale. A stop drains the grant and
  keeps the worker; a broken contract or a stream dropped open ends it. Budgets: frames bound every
  grant, bytes are checked at the item's header and read past, wall clock is a watch thread that
  ends a stalled worker. The carriage byte is round 2's seam for the ring.
- ADR 3502: `tools/stream-witness`, a worker confined under `Profile::Media`, and nine tests
  through it (window, exact last grant, stop, frames, bytes, over-send, stall, malformed item,
  dropped stream); nine unit tests of the byte contract; fuzz target `confined_stream`, seeded by
  `fuzz/seed_confined_stream.py`, with its `doc/verify.md` line and `seeds.sh` arm.
- ADR 3503: the sound path stays request by request — each run's units cross *in* its request, so
  one request answered by many replies is not simpler, and so it was not measured.
- ADR 3504: `CLAUDE.md`'s clause 13 bullet, `doc/scope-ratifications.md` and `ledger.rs`'s
  `Exclusion::Multimedia` put video in scope (A1006), interlaced H.264 on the hand-off by name.

## Ledger

No row moved: the 51 `clause-13-multimedia` rows are `out-of-scope` before and after
(`grep -c 'exclusion = "clause-13-multimedia"' doc/conformance/ledger.toml`); a row video unblocks
moves in the round that builds it.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets` of confined-transport, stream-witness,
pdf-media-worker, viewer-host, viewer-qt, conformance exit 0; Windows clippy of the transport and
witness exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` of the five under
`xvfb-run -a`: 575 run, 575 passed; `audit-diff.py`: failed 0, expected 0. rustdoc `-D warnings
--document-private-items` of the transport, witness and conformance exit 0; `fuzz` `cargo check
--bins` exit 0; `confined_stream` 120 s from its 5 seeds, exit 0, no crash; `cargo test -p
conformance --no-fail-fast` exit 0.

## Habit

Planting `count_item` saturating failed the unit test and the witness test both (trap 13).
