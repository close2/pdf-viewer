# 967 — The index grew back into paragraphs, and a line count could never have shown it

Date: 2026-09-11. ADR: 0974.
Files: `doc/todo/README.md`, `doc/todo/02-every-round.md`, `doc/HANDOVER.md`, `doc/environment.md`,
`doc/habits.md`, `doc/state-of-play.md`, all five of `doc/traps/`, `tools/round.sh`. `CLAUDE.md` was
read and not changed.

The owner asked for a compaction of the twelve documents every round reads. This is the fourth
(ADRs 0232, 0281, 0428), and the first whose interesting output is a list of things it decided **not**
to touch.

**6317 → 6286 lines, 87 806 → 83 940 words.** The line count is the finding. 2527 of the 3866 words
removed were inside four Markdown table cells, so every diff statistic this project looks at said
nothing had happened while `doc/todo/README.md`'s index cells grew to 1663 characters apiece — with
ADR lists, percentages, page counts and per-round chronicles in them — forty rounds after ADR 0281
§4 made that index one line per item and wrote the reason into the file's own preamble. **The file
argues against its own body**, which is what made it the safest cut in the corpus and the one worth
recording: a compaction should be measured in words, because an index doubling is invisible in
lines.

The second-largest was the same shape one file over. `doc/todo/02` §2's change → gate map exists to
say *a change in X is under Y so run Z*, and two of its cells had become 991 and 407 words
restating what `pdf-vfs`'s and `pdf-transform`'s corpus walks assert, clause by clause — while every
one of those walks opens with a `//!` header saying the same thing at greater length beside the
code. That is ADR 0232 §4's archetype, and the two had already drifted in the way that archetype
predicts.

Four decayed claims came out, each corrected rather than dropped, and three of the four are this
project's own instruments failing to carry a finding home:

- `doc/environment.md` taught the baked-`CARGO_MANIFEST_DIR` hazard by naming `tools/conformance`'s
  build script, which has never existed in any commit — trap 25's own finding, recorded in
  `tools/round.sh` and never carried back to the document that teaches the hazard.
- `tools/round.sh` listed each trap group's numbers and **all five lists were short**; it names the
  group's subject now, with a comment saying why a second copy of somebody else's numbers is a copy
  nothing checks.
- `doc/HANDOVER.md`'s trap index was missing **trap 39** — `doc/todo/02` §6 warns about exactly this
  and records traps 14, 30 and 31 reaching the index and not the group table. This was the mirror,
  and it was found by cross-checking the two tables against the five files mechanically rather than
  by reading.
- `doc/todo/02` §5 still called a binary `pdf-retrieve`, in the paragraph *below* the one about an
  `install` loop that went on copying three pre-rename names.

And the trap-numbering rule was written out in full six times — the handover and all five trap
headers. One owner now, five pointers. The `git stash` incident was written twice and the two copies
already dated it to different sessions, which is the drift ADR 0232 §4 named and the reason a
duplicate is worth removing even while both copies are true.

**What was kept is in ADR 0974 and is the half that matters next time**: `doc/habits.md` almost
whole, trap 9's nine mechanisms and their follow-on paragraphs, the four separate false-failure
witnesses under §2's quiet-machine rule, every deliberately-vague duration, and `doc/HANDOVER.md`'s
repetition of the trap titles — because an index that does not repeat is not an index.

The instruments say nothing got worse: `--bin pointers` 185 absent before and after, 14 undefined
symbols before and after; `--bin quotations` 3708 verbatim and 49 diverging before and after, 1867
and 5 in the ledger. `cargo test -p conformance` exits 0, `sandbox_gates.rs` included, which is the
one that reads §2's own command block. The core four could not be completed: `cargo fmt --all
--check` and `clippy --workspace` fail in `crates/pdf-archive`, `pdf-font`, `pdf-model` and
`pdf-transform`, all of them three sibling rounds' uncommitted work in this shared tree, and no file
under `crates/` was touched here.
