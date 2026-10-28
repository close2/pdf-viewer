# 1865 — Every answered question checked against the tree for what it owes

Slot 5 of batch 127, 2026-10-28, Opus 5.5, one session, a reading round; no code. ADR 2564 used, ADR
2565 unused; Q731, Q737 and Q738 asked, Q739–Q746 unused. Ledger: no row moved.

**What was done.** The 121 `A` files at base 2d54febf, read whole by five fresh Opus subagents through
one brief at the commit alone, one row an answer (owes, built?, evidence, owner); every `partly` and
`unbuilt` row re-checked against the answer and the tree, and five `built` rows (A01, A02, A03, A130,
A691). The table and its reading are ADR 2564.

**Premise.** A01, A02 and A23 sat unbuilt for weeks; held — A23 at 0c34fee4 is unbuilt by the same
brief. **Hypothesis** (a handful unbuilt, one or two unowned): did not hold — 68 built, 25 owe
nothing, 5 deferred on their own trigger, 15 partly, 8 unbuilt, and twelve answers carry work nobody
owns: A08, A09, A11, A27, A28, A31 with A39, A54, A288, A694's CFF and Type 1 half, A706's
`derive:<tool>:append` as a `try` step, A254's Route A. Five answers that say `Owes: none` command work in their body
(A26, A27, A28, A31, A39).

**Asked.** Q731 (does A662's hardware lane stand after ADR 2512 took its first item back), Q737 (A47's
download shape and its non-CJK family against A569), Q738 (A35's host option "if it is wanted at all").

**Plant.** The brief given A23 at 0c34fee4 and A714 at HEAD answered `unbuilt` and `built`: both right.

**Owed in `CLAUDE.md`, the owner's file, named in the report.** A26: Q26's "No port changes the
renderer's system-call set" for principle 3's "system-call set does not change". A706 item 2's
sentence is already in the amendment (round 1851).

**Found, not fixed.** `doc/HANDOVER.md` line 39 calls RFC 0009 built in its section 12's order, and
item 2 (the print path) is not; `crates/pdf-model/src/integer_entry.rs` line 57 calls Q31 open;
`doc/todo/58-the-file-system-faces.md` line 431 calls reorder-by-rename open against A09.

**Files.** `doc/adr/2564-every-answered-question-checked-against-the-tree-for-what-it-owes.md`;
`doc/questions/Q731-does-a662s-hardware-lane-still-stand-now-that-its-first-item-was-taken-back.md`,
`doc/questions/Q737-does-a569-replace-a47s-download-shape-and-is-a-wide-coverage-non-cjk-family-still-owed.md`,
`doc/questions/Q738-is-a35s-host-supplied-substitute-for-a-null-font-owed-or-only-permitted.md`; this record. I worked in `scratchpad/r1865/`.

**Gates.** `cargo test -p conformance --test questions` exit 0 (2 passed); `--bin pointers` exit 0,
`--bin cited` exit 0, `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 101 on
`--test records` alone, this record's own placeholder, then `--test records` exit 0 once it was written.
