# 2058 — The lifted gates swept against the tree, and none left to build here

Session 2058, an audit round in the audit worktree. `doc/todo/76`'s rows; no ledger row read to a move.
ADR 2930; ADR 2931 and Q886 were assigned and are not used: nothing was built and no owner's word was needed.

**The slice.** The brief's two items in order: the sweep of `doc/todo/76` section 2's rows not marked built, then
the first open, effort-only row in section 3's order outside row 21 and the signatures lane's rows.

**The sweep.** Every row checked by its subject, ADR and witness against `doc/adr`, `doc/history`, the ledger and
the code (ADR 2930 section 2). Row 7 was built (ADR 2185, which supersedes ADR 2137); row 21 was built (ADRs 2356,
2404). Rows 2, 3, 28 and 29 were further on than written (ADRs 2904; 2560, 2604, 2781, 2874, 2922, 2923; 2454,
2700, 2642; 2905, 2927). Row 10 is measured and scheduled (ADRs 2837, 2876, A840); rows 11–15, 22, 23 were kept
by A828; rows 16–20 hold. `doc/todo/72`'s "owed to a writer that does not exist (ADR 2519)" was ADR 2700's.

**Item 2's premise.** Failed. What is open and effort-only is row 6's and row 7's WinUI print dialogue (N4),
which only CI compiles and nothing here runs (ADR 2136), and row 9, whose spool path does not exist: the GTK
and Qt windows print through their toolkits' RGB contexts, RFC 0009's `ipp` submission and spool container
(A254 item 2, `doc/todo/65` section 1) are unbuilt, and no print system is installed here. A254 adopted
"no, until a spool path exists"; A800 lifted only the document half. Neither needs the owner, so no Q886.

**Tests and plant.** No code and no test. **Plant**: this record's gates paragraph first said "see the report";
`records`' `every_record_since_the_rule_states_its_gates` failed on it by name, and the figures below replaced it.

**Gates.** No crate touched, so fmt, clippy, nextest, rustdoc and the fuzz check are not owed. Conformance after
`git add -N`: exit 101 planted (473 passed, 1 failed, the record), then exit 0 (474 passed).

**Habit.** A row's "effort only" is checked against the ADRs that cite its own ADR number (`grep -l <n>
doc/adr/*.md`, read the later ones' status lines) before a brief is written from it; three of this file's
rows were answered that way in under a minute each.

**Left.** A254 item 2's spool path before row 9; N4 on a Windows machine; brainpoolP512r1 (signatures lane);
ADR 2630's Noto pack (PDF/A lane).
