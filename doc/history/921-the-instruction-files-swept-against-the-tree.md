# 921 — The instruction files swept against the tree, and a phrase six rounds each wrote once

Date: 2026-09-04.
ADR: 0882 (the sweep and what it changed); `doc/questions/Q25` is the half a round may not decide.
Files: `doc/HANDOVER.md`, `doc/environment.md`, `doc/habits.md`, `doc/history/README.md`,
`doc/state-of-play.md`, `doc/todo/README.md`, `doc/todo/01-ledger-partial-rows.md`,
`doc/todo/02-every-round.md`, `doc/questions/Q25-principle-2s-perf-gates.md`, `tools/state.sh`,
`tools/round.sh`, `crates/viewer-qt/src/bridge.rs`.

The owner asked for the instruction files to be gone through and compacted, on the expectation
that fifty-odd sessions had left obsolete claims in them. They had. What follows is what was
false, because that is the half worth recording; the compaction is in the diff.

## The finding that generalises

**Six bullets of `doc/todo/02` §4 each opened "and it is the newest", and five of them were
wrong** — `overstated`, `overtaken`, `quoted`, `unpriced` and `parts`, the eighteenth to the
twenty-second sweeps, each written by the round that built it and none revisited by the round
after. `doc/todo/01` carried the same phrase five times over and `doc/todo/README.md`'s index line
for item `01` once more — three files, one phrase, and a `grep` for it found every instance after
the first. No instrument could see it: the
sentence is true on the day it is written, it is about *this project's own list* rather than about
the standard or the tree, and `--bin parts` reads cardinals about parts rather than superlatives
about rounds. It is the exact shape `02` §4's own bullets exist to catch, happening inside the
section that catches it.

The rule that comes out of it, and it is why the six became a table: **a superlative is a claim
with no anchor.** An ordinal (`the twenty-second sweep`) is checkable against `doc/todo/01`'s
catalogue in a second; *the newest* is checkable only against every neighbour, which is the check
nobody runs. Where a round is tempted to write "newest", "latest" or "the last one added", the
honest form is the ordinal and the pointer.

## The other false claims, each checked rather than assumed

- **`doc/HANDOVER.md`'s trap group table had lost traps.** The pixels row listed 1, 2, 6, 12b and
  12c while `doc/traps/pixels-and-rasterisers.md` holds **14** as well; the instruments row stopped
  at 29 while that file holds **30**. Both traps are in the same file's index below the table, with
  their group named — so the two halves of one page disagreed, and a round opening the group it was
  told to open would have read one trap short. **The merge added a third instance in the same
  sitting**: main's trap **31** had reached the index and not the table. That recurrence is why
  `doc/todo/02` §6's rule about adding a trap now names *both* entries rather than one — the index
  and the group's row — because three rounds in a row updated the half they were looking at.
- **Three cardinals counting this tree's parts were wrong**, all found by `--bin parts`:
  `doc/HANDOVER.md`'s "either rasteriser" and `doc/state-of-play.md`'s "so that both backends
  draw it", of the sidebar's own display list, where this workspace states three rasterisers and
  the window presents with the third; and `doc/environment.md`'s "the four submodule paths under
  it" of `git checkout -- doc`, where six submodules live under `doc/`. The rest of that sweep's
  hits in `doc/` are the QUORRA and HAYRO correspondence, which is not an instruction to a round.
- **`tools/state.sh` did not run the sequence it claims to.** `doc/todo/02` §2 says the script
  "runs the same sequence"; it had no line for `pdf-transform`'s `optimize_corpus` and none at all
  for `pdf-vfs`'s `write_corpus`, the latter added to §2 in session 909. Fixed on the instrument
  rather than in the sentence — a `vfs` section with its own `--bins` build (trap 10), and
  `optimize` beside the other four writer walks. Its foreign-readback line also said "the four
  writers'" where the test's own header says five. **The new section then failed on trap 10 itself
  the first time it was run**, which is trap 13 doing its job: `section_vfs` built `pdf-vfs`'s bins
  and not the sandbox worker, so all three of its walks refused to start. `gate_binaries` is
  memoised and now heads that section — and `section_writer`, which had the same hole and was
  covered only by `transform` happening to run before it in the `all` order.
- **`doc/habits.md` gave `ISO_32000-2_sponsored_EC3.md` 860 `##` headings.** It has 1216, of which
  922 begin with a digit. Replaced by the command, which is what ADR 0281 asks for.
- **`doc/state-of-play.md` claimed "one hand-written `unsafe` token in the tree".** True of
  `viewer-qt`, which is where the sentence came from; false of the tree, because `viewer_ffi::abi`
  writes its entry points as `pub unsafe extern "C" fn` by the hundred. The same sentence said "no
  other crate lifts the denial" while the test it cites is named for the crates that do.
  `crates/viewer-qt/src/bridge.rs` carried the same widening and is corrected with it. **Both
  corrections were then falsified by the merge**, which brought `pdf-vfs-ffi` and renamed that test
  from `only_the_two_…` to `only_the_three_…` — so the sentences now *name the test* and let it
  carry the number, which is the ADR 0281 move applied to a cardinal rather than to a count.
- **`doc/state-of-play.md` counts three windows and there are four.** `pdf-viewer-confined` is
  `viewer-ui`'s second window; `tools/state.sh windows` excludes it deliberately and says so, and
  the prose did not carry the qualifier. The population is now stated once where the consumer list
  begins rather than guessed at eight times.
- **`doc/HANDOVER.md` and `tools/round.sh` both said "the four things a round has got wrong
  before"** over a list of five — the fifth being ADR 0450's check of CI's last run on `main`.
  `round.sh`'s own header introduces five items as four in one sentence.
- **`doc/todo/README.md`'s index was stale about the two live streams.** Item 57 said `merge`,
  `pages` and `optimize` were what is left; `doc/todo/57`'s own header says all five writing verbs
  are done and names three different remainders. Item 58 said the write side and the FUSE face were
  what is left; both landed in sessions 906 and 909. This is precisely the hazard that file's own
  preamble names — "a summary here that restates them is a second copy to keep in sync".
- **`doc/todo/02` §2 said "Five notes bind here" over ten bullets**, and "this sentence said 'the
  first four' while five were owed" over six core lines. Both are the superlative's cousin: a count
  of a list, written by a round that had just read it and not by the round that grew it.

## What was not touched, and why

**`CLAUDE.md` is unedited.** Three of principle 2's sentences are false against the tree — the CI
perf gates, the deferral to a Spike A that finished long ago on a different subject, and the
parenthetical naming vello as the GPU offload where the product presents with quorra — and a
principle reading false is a `Q` file rather than an edit. `Q25` states all three with the
evidence, what the tree does meanwhile, and a recommendation for each. `Q24` is left for round 920,
which owes `doc/todo/59` §5's amendment to principle 3.

`doc/adr/` and `doc/history/` were out of scope by the owner's own rule and nothing there was
opened for editing. `doc/todo/README.md`'s index was corrected where it was false and **not**
shortened: each line is the item's own remainder plus its ADR citations, `--bin pointers` reads
those citations, and the file's authority for every line is the item file it links to — so a
shortening round would be trading checkable pointers for a smaller file. The QUORRA and HAYRO
documents under `doc/` are correspondence with another project rather than instructions to a
round, and `--bin parts`'s dozens of hits in them were left alone for that reason.
