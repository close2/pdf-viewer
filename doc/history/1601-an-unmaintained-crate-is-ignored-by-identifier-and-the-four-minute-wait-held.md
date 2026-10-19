# 1601 — An unmaintained crate is ignored by identifier, and the four-minute wait held

Slot 3 of batch ninety-five, 2026-10-19, a docs round under the owner's switch to code
improvement: CI's `deny` job, the cache census over batches 92–94, and the two navigational files.
ADRs 2038 and 2039. No Q file; no ledger row moves (the work cites no clause).

**Premise.** Held. CI's `deny` run failed on RUSTSEC-2026-0370, 0371 and 0372 alone (`advisories
FAILED`, the other three `ok`), and a local `cargo deny check advisories` (0.20.2) reproduced it,
exit 1. **Hypothesis.** Held: `accesskit_unix` 0.24.0 asks `zbus ^5.19`, `accesskit_atspi_common`
0.21.0 `zvariant ^5.15`, `atspi` 0.30.0 `zbus ^5.5` (crates.io, 2026-10-19), so no bump clears the
three and none is taken; and the cache rule held with the wait script in use.

**Built.** ADR 2038: the three ignored in `deny.toml` by identifier, with the shipped-graph reason,
the cost (no further releases; parsed from the session bus in the unconfined host, never a
document's bytes) and the revisit condition, which `cargo deny` reports itself as
`advisory-not-detected` — checked by planting an unmatched ignore in a copy (exit 0, the warning);
`doc/stack.md`'s accessibility row says so. ADR 2039: the census, and `tools/cache-census.py`'s
`--until` and its per-transcript count of gaps. `doc/state-of-play.md`: the XMP edition and the
`xmp:Rating` ruling (ADRs 1991, 2020), the short ICC profile at parts 2 and 4 (ADRs 1990, 2006),
the unreadable destination profile and a halftone's `TransferFunction` (ADR 2007), omitted since
batches 92–93. `doc/HANDOVER.md`: the performance row no longer calls performance the owner's
current target and names `doc/todo/75`; a row for the code-improvement target, `doc/todo/74`.

**The census** (ADR 2039 §1), `--since 2026-10-19T09:18:16Z --until 2026-10-19T20:07:16Z
--buckets` over rounds 1577–1598's 22 transcripts: 3 724 requests, 6.6 M tokens written, 813.6 M
read; 3 gaps over 300 s (round 1598's, 353–359 s), 0 misses; 184 requests in [240, 300) s, all
hits; cache cost 49.0 M equivalents. Batch 91 (1571–1576, `--since 2026-10-19T05:53:45Z`, owed by
1581): 1 gap, 308 s, 0 misses. Control: round 1565, before the rule, 14 of 14 gaps past 400 s missed.

**Gates.** `cargo test -p conformance --no-fail-fast`: exit 0, 442 passed, 0 failed. `--bin
pointers` exit 0 (9 absent, none mine); `--bin quotations` exit 0. `cargo deny check advisories`:
exit 1 before, exit 0 after (`advisories ok`). `tools/state.sh --round 1601 docs`: exit 0, 59 s.
The census runs: exit 0 each; an empty window exit 2, as it should. 1 450 s.

**Files.** `deny.toml`, `doc/stack.md`, `tools/cache-census.py`, `doc/state-of-play.md`,
`doc/HANDOVER.md`, ADRs 2038 and 2039, this record. `Cargo.lock` untouched.

**Unfinished.** None owed. Found, not mine: round 1598's three waits in one response.
