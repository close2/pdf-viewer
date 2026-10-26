# 1768 — The `inapplicable` and `out-of-scope` rows, re-read for scope

Session 1768, a clause round run beside batch 116; 2026-10-26. ADR 2370; ADR 2371 unused; Q631–Q634.

**Contract.** The owner's request of 2026-10-26: re-read every `inapplicable` (34) and `out-of-scope`
(84) row for whether its reason holds, what blocks moving it, and which moves are a good idea. The
deliverables are review 1768, four questions, ADR 2370's standing rule and this record.

**Premise.** Held for the population (`tools/state.sh ledger`: 34 and 84). **Did not hold** for the
one move the brief allowed: `partial` or `unreviewed`. Every decayed row is cited in code, so
`unreviewed` fails `CitedButUnreviewed`, and `partial` on a leaf drags its heading through
`Ledger::derived_status` (§12.6.4.10 moves §12.6.4 and §12.6; §14.11.2.2 moves §14.11.2 and §14.11)
while claiming an execution or a debt the row does not have. **No status moved.**

**Found.** Classes A 33, B 56, C 28, D 1, E 0. Twenty-one §13.2 rows are executed by
`pdf_model::media` for §13.2.2 and still say "a media engine"; no sweep reads `out-of-scope` rows.
§14.11.2.2 is owed since A254 adopted the guidelines in the print panel. A movie's and a sound's file
are the media file the amended clause 13 entry admits, and nothing hands them over. Both clause 13
triggers are met by letter on crates.io: BSD-2 and Apache-2.0 H.264 and AAC decoders (four of them
published before RFC 0009 said none existed) and `u3d` 0.1.0. XFA and the deprecated families hold.
§10.6's notes owe A254 question 8's sentence. `accesskit` 0.24.1 carries what §14.8.5.4.4 states.

**Questions.** Q631: are a movie, a sound and rich media assets handed over as a clip is, and has
Tier B's demand come? Q632: is Tier C or Tier D reopened now their triggers are met by letter? Q633:
when is XFA's "later", and is its first step the specification and an RFC? Q634: does "as far as
accessibility needs it" take Table 380's inline attributes into the accessibility tree?

**Candidates, top five.** §14.11.2.2's guidelines (a round, no blocker); the §13.2 family read
against `media.rs` (a clause round, no blocker); the movie hand-off (Q631); §10.6's sentence
(minutes); §14.8.5.4.4 into the accessibility tree (Q634).

**Files.** New: `doc/reviews/1768-the-inapplicable-and-out-of-scope-rows-re-read-for-scope.md`,
ADR 2370, Q631–Q634, this record. Nothing else edited; the ledger and `doc/todo/65` are untouched.
I worked in `scratchpad/r1768/`.

**Gates.** `cargo test -p conformance --no-fail-fast`: exit 101, 463 passed, one red, not mine —
`records`, on 1764's 47-line record, red before my files existed too. `cargo run -p conformance
--bin quotations`: exit 0, none of my files diverging. `--bin inapplicable`: exit 0, 34 rows read.
No status moved, so no `tools/state.sh ledger` after and no status checker. No code touched, so no
tier 1. Duration about 1 900 s, about 70 tool uses.
