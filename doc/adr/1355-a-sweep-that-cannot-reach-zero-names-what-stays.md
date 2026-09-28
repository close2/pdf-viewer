# 1355 — A sweep that cannot reach zero names what stays, by class or by name

Session 1259. Status: accepted and **built**.
Context: `tools/conformance/src/cited.rs` (`Unrowed`, `why_no_row`), `tools/conformance/src/overtaken.rs`
(`READ`, `sweep`'s third argument), `tools/state.sh` (`section_cited`), `doc/todo/01`'s nineteenth
and twenty-seventh sweeps.
Builds on: ADR 1274 (`--bin cited`), ADR 0491 (`--bin overtaken`), ADR 1273 (a named population is
keyed by what does not move).

## 1. The problem both sweeps had

A count that can only stay where it is or rise is not an instrument: a real finding arriving beside
the standing population moves a number nobody expected to be zero. `--bin cited` ended "34 with no
row at all", and every one of the 34 was right — ISO 32000-2's informative Annexes B and C, and the
whole clauses 2 and 11, none of which the ledger rows by design (`ledger::INFORMATIVE_ANNEXES`,
`ledger::NORMATIVE_CLAUSES`). `--bin overtaken` printed 69 notes, and a note that a later decision
names only as an example of some other property of the same page had no way to say it had been
read short of citing that decision inside the note.

## 2. The decision

- **`cited::Unrowed` is a class with a reason.** An informative annex's number, and a top-level
  clause's, are counted per class with the numbers each holds, and are not listed. Every other
  row-less pair is listed and read first, and that count is zero on a clean tree. A normative
  annex's number and a front-matter subclause are in no class: the first is rowed to the leaf, and
  the second is where a `§` that meant an ADR's or an RFC's section most often lands. What a class
  cannot see is a misread `§` that spells a class's number; the printed numbers are how a new one
  shows, and the conformance gate already refuses a number the standard does not print.
- **`overtaken::READ` is a named population keyed by list name**, each entry the newest decision a
  reading reached. A note is compared only against decisions numbered above both its own newest
  citation and its entry, so a decision taken later brings it back. The list lives in the sweep and
  not in the notes, because a diagnosis a student reads should not carry ADR numbers about other
  properties of the same page. An entry whose list the tree no longer declares is printed.

## 3. What it cost and what stays

The reading behind `READ` was every one of the 69 notes against every later decision naming its
pages (458 pairs); it corrected sentences in eighteen notes and left seven on the list, whose
correction needs a measurement rather than a sentence. `doc/todo/01` names them. An entry added
without that reading is the failure this record exists to prevent: the list is a claim that a
person read the note, and it decays exactly as a citation does.
