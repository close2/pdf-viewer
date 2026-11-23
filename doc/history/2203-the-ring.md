# Round 2203 — a stream's large items cross in a sealed ring

Video's second round (RFC 0015 section 6), in the `audit` worktree: the ring, and `Profile::Media`.

## What changed

- ADR 3513: `confined_transport::ring`, `Host::attach_ring`, `Carriage::Slot`. A `memfd` sealed
  `SHRINK | GROW | SEAL`, at most half the worker's address-space ceiling and sixteen slots, passed
  once beside a `RING` frame; item `q` in slot `q mod slots`, grants clamped to the slots, the slot
  copied out before the next grant; a slot not held, or a length past one, ends the worker.
- ADR 3514: `Profile::Media` — `RLIMIT_NOFILE` 4 (one slot, number 3), `recvmsg` on descriptor 0,
  `mmap` of descriptor 3 shared and exactly read-write, the allocator's `mmap` narrowed to private
  anonymous. No `close`: the kept descriptor fills the slot, so a second is dropped by the kernel.
  `Profile::Script` unchanged.
- ADR 3515: the two copies through the mapping's pointer are `inherited_descriptors::shared_mapping`,
  pinned by `tests/callers.rs`; the sweep stays at five names. Q1024 asks about the crate's name.
- ADR 3516: seven witness tests, three unit tests and the seals' test; `pdf-sandbox`'s ring probe;
  `confined_stream` fuzzes the slot carriage, seeded with a ring run; the copy saved, measured.

## Ledger

No row moved: the 51 `clause-13-multimedia` rows are `out-of-scope` before and after
(`grep -c 'exclusion = "clause-13-multimedia"' doc/conformance/ledger.toml`); a row video unblocks
moves in the round that decodes and presents it.

**Measured** (ADR 3516): 120 frames of 8 294 400 bytes, dev profile, load 29 to 50: inline 28.7 /
35.9 / 29.6 frames per second, ring 159.3 / 227.2 / 189.9.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets` of confined-transport, stream-witness,
pdf-sandbox, pdf-media-worker, viewer-host, viewer-qt, inherited-descriptors exit 0; Windows clippy
of the transport, witness, mapping and sandbox exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib
--bins --tests` of the seven under `xvfb-run -a`: 666 passed, 6 skipped; `audit-diff.py`: failed 0,
expected 0. rustdoc exit 0; `cargo deny check` exit 0; `fuzz` `cargo check --bins` exit 0;
`confined_stream` 120 s from six seeds; `cargo test -p conformance --no-fail-fast` exit 0.

## Habit

A probe on the harness's thread inherits an arena past a small address-space ceiling, so every
mapping fails with `ENOMEM` and passes as "refused": run it with `MALLOC_ARENA_MAX=1` and assert
the filter's `SIGSYS` itself (trap 13).
