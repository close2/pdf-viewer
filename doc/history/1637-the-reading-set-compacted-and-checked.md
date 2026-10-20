# 1637 — The reading set compacted, and each claim in it checked

Session 1637, slot 7 of batch ninety-nine, a docs round; 2026-10-20. ADRs 2110 and 2111. The
owner's words: "go through the md files, which are read by every subagent. Can we compact them?
Is all information still valid?"

**Done.** `CLAUDE.md`, `doc/todo/02` section 0, the rule block of `doc/environment.md`, both
every-round files, `doc/HANDOVER.md` and the brief template read whole and checked against the
tree here; the rest of `doc/environment.md`, `doc/traps/` and `doc/habits/` read by four parallel
readers under the same rules. Repeats kept once and pointed to (the wait rule lives in the rule
block, section 0 names it); printed counts made commands; corrections applied, not appended.
Eight traps folded into survivors and one habit into another, every incident and number kept; four
ledger-reason habits moved to their file as ADR 0983 left owed. Every deletion is in ADR 2110 §4.

**Measured.** The every-round set, by the growth of the next request's input when each version is
printed whole: 18 953 → 16 286 tokens (−14 %), `CLAUDE.md` alone 12 582 → 10 454 (−17 %). Words over
the whole set named in the brief: 115 869 → 108 303 (−6.5 %); `CLAUDE.md` 5 385 → 4 688,
`doc/traps/README.md` 16 986 → 13 168, `doc/environment.md` 8 941 → 6 449. The first request of
rounds 1631–1633 was 40 963–41 087 tokens; the after figure needs a round launched from the merge.

**The premises that did not hold.** A round does not read the trap README or the group files at
launch (ADR 1639), so the largest files are not the largest part of the launch; and a third of the
words were not restatements — the incidents are most of them and stay.

**Found false, corrected.** Principle 2's six sentences A25 ruled on in 2026-09 and nobody applied
(ADR 2111); two misquotations in `CLAUDE.md` (§8.6.4.4, §10.7.3); `every-round.md`'s "verbatim"
rows and its gate claim; "four-fifths"; `PYTHONDONTWRITEBYTECODE` under `-I`; and forty further
claims in environment.md, the trap files and the habit files (ADR 2110 §2).

**Outside this slot.** Handed off: four comments quoting retired principle-2 wording (`page.rs`,
`callgrind_open.rs`, `turn_path.rs`, the §7.7.3.2 note), as a `-U0` patch. Found: `tools/state.sh traps` prints "cited with no row in the index: 0 632";
habit 141 appended to `measuring.md` mid-batch by a sibling; this batch's brief has an 81-line common
part and 33-line slots against the template's 60 and 25; CI's `fuzz` job red on `main`.

**Gates.** `cargo test -p conformance --no-fail-fast`: 101 twice, on three failures naming slot 4's
moved host files, none of this slot's; the third run 0 but for this record's own shape, fixed, then
`--test records` 0 (443 passed). `--test traps`: 0. `--bin quotations`: 0 and `--bin pointers`: 0,
neither naming a file of this slot. `tools/state.sh --round 1637 docs`: 101 after 624 s, on
`render-cpu`'s rustdoc (`crate::scan::Exact::Pieces` unresolved, a sibling's `scan.rs`). Duration:
about 2 300 s.
