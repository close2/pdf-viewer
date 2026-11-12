# 2008 — A reader's own anchor is locally valid, and a trusted list is read

Session 2008, a single ledger round, the signatures lane (`doc/questions/A800` item 3; §12.8.1,
§12.8.3.4.4, §12.8.3.4.5). ADRs 2834 and 2835; no question.

**The slice, and why.** Locally valid whole, and the core of the eIDAS half: an ETSI TS 119 612
list read and TS 119 615 clause 4.3 run, which every later step (authentication, QC statements,
clauses 4.4 and 4.5) starts from. The level is clause 4.4's fourteen pages and is `doc/todo/77`'s.

**The premise.** Held: one anchor set, every path to it *valid*; no list reader; ADRs 2725, 2782 call
it work. No `A*` after A800 cites it. TS 119 172-4 routes the level through TS 119 615 (unnamed).

**Built.** `verdict::Standing` (`Local`, `Listed`) on every `Valid`; `Anchored::listed`.
`trusted_list`: the reader (bounded, DTD refused), `matching` (PRO-4.3.4-01, -03, -03 b), -03A, -05
to -11), `listed_anchors` (TS 119 172-4 REQ-4.2-03 a)), `Lists`. `xml.rs`: the policy reader's tree,
shared. `TrustPolicy::lists`, `--trusted-lists`, the confined wire; `notes` words *locally valid*
and a listing, the list's unverified signature and the undetermined level said. `fuzz/`
`trusted_list`. `tools/drive-windows.sh` greps the new sentence.

**Tests.** Seven over written lists (clause 5's fields, a closed list, refusals, PRO-4.3.4-01,
-03 at its boundary instant, -03A, -05/-06/-11); two verdict tests (local, and a list naming the
fixture's root reaching `Listed`); the host word. **Plant**: `SI-at-Date-time`'s `>=` as `>` and
`Valid`'s standing forced `Local`: three tests fail; reversed, they pass.

**Gates.** fmt (five crates, `fuzz/`) and clippy `-D warnings --all-targets` (`pdf-signature`,
`viewer-core`, `viewer-host`, `viewer-confined`, `viewer-ffi`): exit 0. nextest of the five: 1542 of
1542, 9 skipped; doctests exit 0. `cd fuzz && cargo check --bins` and fuzz clippy
(`trusted_list`, `policy_document`): exit 0, once one `&raw const` was written. Conformance
after `git add -N`: 471 passed, once `trusted_list` had its `seeds.sh` arm and `verify.md` line.
Rustdoc `-D warnings` with private items on the five: exit 0, once a link lost to the moved
`use` was qualified. Tier 2: `corpus` 101, 974 documents, 61 incomplete, joined the pair round 2004
had (`issue15053.pdf`, `mixedfonts.pdf`), none left. `raster_golden` 101, held 951, moved 23: round
2004's 16 by name and seven more (two synthesised annotation appearances among them) that the audit
merge 79d0c6c4 reaches, which 2004's walk predates; no HEAD run was taken, and this change reaches
no drawing path. `selection_census` 0 (974 documents); `accessibility_census` 0 (1020).
`awkward_classes` 0. `state.sh --round 2008 docs` 0, re-run after the walks.

**Left of the rows.** §12.8.1, §12.8.3.4.5 stay `implemented` (neither requires the level);
§12.8.3.4.4 stays `partial`, `TAsInTrustedList` now its list residue. `doc/todo/77` is the order.
