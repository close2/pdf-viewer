# 1369 — Clause 8's notes state what is, and each open row states one reason

Ledger slot of batch fifty-seven. ADRs 1573 and 1574; one row moved; no question written.
**The notes.** ADR 1547's rule, applied to clause 8: its 78 rows carrying a session ordinal (241 by
`tests/ledger_notes.rs`'s rule, not the 400 the brief guessed) were rewritten by eight forked readers
working from JSON copies. A checker read each output: every kept double-quoted quotation, backticked
name, ADR and `§` number had to be in the old note, no ordinal could remain, quotes had to balance
and no `\u` could appear. The chunks were applied one at a time under `flock /home/AI/ledger.lock`,
each with `assert s.count(old) == 1` on a fresh read and `cargo test -p conformance` after it.
Clause 8's notes went from 296 732 to 220 958 characters. The ledger's count went from 934 to 693,
and the constant holds 693. Five rows spot-read against their old notes: §8.3.4, §8.4.5, §8.6.5.8,
§8.9.7 and §8.11.2. No clause-8 status moved.

**The fifteen open rows**, each read against `main-checkout`, the code and the owner's A files
(ADR 1574):
- **§7.4.9 now has one reason.** It stays `partial` for the three spaces whose texts are not held.
  The thirteen irreversible-path codestreams are no longer debt. ISO/IEC 15444-1 sections E.1.1.2,
  F.3.8.2 and G.3 leave that path's reconstruction and precision to the decoder, so a one-level
  difference is not a departure.
- **§12.10 and §12.10.2 can be built.** The owner answered Q171 on 2026-10-05 (`A171`, uncommitted):
  build the projection, census first. The brief still called Q171 open. Both rows moved from bucket
  2 to bucket 6.
- **The rest hold.** §7.4.7 is the extended-template patch alone, of the five owed. A66's trigger is
  unlit, so §7.6, §7.6.5's four and §7.6.6 stay. Aggregates stay `partial` by ADR 1035's rule.
- **PAdES validation is already built.** §12.8.3.4.5 to §12.8.3.4.8 are `implemented` offline. A98's
  client is not what §12.8.3.4.4 lacks; a signature that states a policy is.

**One row moved: §12.1 `partial` → `implemented`** (ADR 1573). Its only text is a map, "This clause
describes the PDF features that allow a user to interact with a document on the screen". It has no
subclause rows, so it is not an aggregate, and ADR 1535 makes it vacuous. It is no longer in
`doc/todo/65`'s aggregate list.

**`doc/todo/01`** is now what is: 6217 lines to 541, `grep -c hundred-and-` 394 to 0, sweeps keeping
their numbers, and a section giving each open row's reason in one line.

**Gates.** `rustfmt --check --edition 2024 tools/conformance/tests/ledger_notes.rs`: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets --keep-going`: exit 0.
`cargo test -p conformance --no-fail-fast`: exit 101, every binary passes but `records`. That one
fails on a sibling's record still being written (`1370-*.md`). `ledger_notes` passes at 693 and
`the_frontier_map` passes. `--bin quotations`: 3 ledger divergences, all carried verbatim.
