# 799 — The array that states no children

The errata selection rule's thirteenth use, and the first whose *arithmetic* had to be repaired
before its head could be believed. Step 3 attributes an `emit` heading to the nearest ledger row at
or above it, and the ledger has rows for the technical clauses and the normative annexes alone — so
Annex A's four annotations were sitting on §14.13.10 and made it the head at six. A family guard
takes them off, and both rankings then top out at three: over live rows §12.7.4.1, over every row
seven rows including that one. Step 4 prefers the settled row, the third use's tie-break picks the
one whose carets put requirements into a table's cells, and **§7.7.3.2 — `implemented` — paid
code**, which no settled head had done in five uses. Issue #271 inserts three rules into Table 30:
`/Kids` shall hold no null entries and shall be at least one entry long, `/Count` shall be 1 or
greater. Two of the three are this reader's own construction given the cell's words. The third
found a guard one degree weaker than the contradiction it was written for — `Pages::new` declines a
`/Count` on a root with *no* `/Kids` and tested that with `as_array().is_some()`, which `[]` passes
— so a root writing `/Kids [] /Count 3` reported three pages, produced none of them in silence, and
never reached the recovery scan that would have found its own page object; and a child node in that
shape had `find_leaf` skip pages it does not have, shifting every page after it. Two issues left
the population; the code moved.

Date: 2026-08-28.
ADR: [0732](../adr/0732-the-array-that-states-no-children.md), the number the briefing reserved.

Touched: `crates/pdf-model/src/page.rs` (the two `/Kids` tests and three erratum annotations),
`crates/pdf-model/tests/page_tree_nodes.rs` (three new tests, one of them a corpus witness),
`crates/pdf-model/examples/kidless_node_census.rs` (the empty array counted beside the absent
entry), `doc/conformance/ledger.toml` (§7.7.2 and §7.7.3.2, reformatted by its own binary),
`doc/errata-read.md` (thirteenth-use section), `doc/todo/01` (the section, the family guard in step
3, and the baseline-checkout footgun below), the ADR and this file.

## What the rule gave

Under the recipe's own single-issue line parse, 302 issue numbers in
`doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret and **73 were named nowhere** at this
round's base — the twelfth use's closing arithmetic (85 less its twelve verdicts) reproduced by the
greps, the fourth consecutive use at which base and derived closing figure agree. The multi-issue
parse counts 310 and 75, the twelfth's own figures less the same twelve. Two issues gain verdicts;
the closing population is 71 single-parse and 73 multi-parse, re-derived by the same greps after
the records were written.

**The ranking needed a repair first.** Twelve annotations fall under headings whose family has no
ledger row — six under the front matter, whose clauses the ledger starts after, four under Annex A
and two under Annex H — and the unguarded arithmetic dropped the first six in silence while
crediting Annex A's four to §14.13.10 and Annex H's two to Annex F's last row. Six of noise beat
three of signal on the first use whose signal was three, which is a decay detector's own decay:
the noise floats up exactly as the real heads are read down.

## What paid

- **Issue #271 (§7.7.3.2, Table 30)**: two carets with no strikeout — `check`'s fourth blindness,
  met the round after ADR 0728 named it. `[493.48 610.979 501.433 617.459]`, centre x 497.5 where
  `-bbox` ends the `nodes.` of the `/Kids` cell at 499.33, writes *(null entries shall not be
  present). The length of the array shall be at least one*; `[323.298 574.979 331.251 581.459]`,
  centre 327.3 where the `tree.` of the `/Count` cell ends at 329.47, writes *which shall be 1 or
  greater*. `Node::of` already answers `None` for a null entry and both places that trust `/Count`
  already require it positive, so two of the three insertions are vindication. The empty array is
  the defect, in two places — the root's page count and a subtree skip's countdown — and both ask
  for a non-empty array now.
- **Issue #614 (§7.7.2, Table 29's `/DPartRoot`)**: a caret at
  `[285.9199 481.29234 294.99595 488.68763]` writing *; shall be an indirect reference* at the end
  of `(Optional; PDF 2.0)`. Filed by the outline under §7.7.3.2 because page 117 reaches that
  heading — ADR 0712's placement rule, applied before the verdict. Third member of the family the
  row records for `/Extensions` and `/StructTreeRoot`; the same reader's tolerance answers it.

## The population, and why the corpus is quiet

`examples/kidless_node_census` counted the absent `/Kids` and not the empty one; it counts both
now. Over `doc/pdf.js` and `doc/corpora`, 1231 documents open and **1 states an empty `/Kids`** —
`doc/pdf.js/test/pdfs/issue8088.pdf`, whose empty node writes `/Count 0` beside it, which is the
value the same erratum outlaws. That zero is why the file reads correctly under either version:
the subtree skip is taken only on a positive count. It is pinned as a third witness, and it is what
says the change is not an over-correction on a real file.

Calibrated per trap 13, both ways, above a commit: with the empty array read as children again the
two new fixture tests fail and the seven older ones pass; with `/Count` never believed at all the
new tests pass and the pre-existing `a_count_without_kids_is_not_believed` fails on *a stated
/Count over a real tree is believed*.

## Gates

Full §2 sequence — the change is in `pdf-model`, which the map puts under everything. `fmt` clean
after one reflow; `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` silent apart
from gcc's `viewer-qt` bridge warnings on the cold build, which §2 names. nextest 2758 passed, 18
skipped. Doctests: 24 result lines, all ok. Fuzz `check` clean; both trap-10 binaries built before
the gates that need them. Corpus: 974 documents in 17.5 s — 0 unopenable, 8 locked, 2 encrypted
beyond us, 6 pageless, 67 incomplete, 0 slow. Oracle: 1945 pages in 98.5 s — 983 agree, 61
contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render;
exit 0, no ratchet moved, every verdict count identical to the round before's. Text extraction: 4
passed in 33.0 s, 10971/11163 words in bounds (98.28%), 487 of 508 documents fully in bounds.
Selection census 8.1 s, 1000/1011 words selected (98.91%). Accessibility census 33.7 s, no ratchet
moved, 0 untagged pages given structure. Dates, XMP, JPEG 2000 green. Quorra: 957 pages in 32.7 s —
932 agree, 22 differ, 3 refused, 17 not comparable. `fixed_documents`: 41 checked, 0 absent.
`cargo test -p conformance`: 23 result lines, all ok, re-run after the last document edit.

`Pages-tree-refs.pdf page 2: no render — no page 2` is in the oracle's output and is **not** this
round's: its tree is a `/Kids` cycle (3→4→5→3) under a root claiming two pages, which the walk's
own bounds stop. Every verdict count matches the previous round's.

## Sweeps

Fourteen sweeps plus the three errata ones, run against the pristine baseline — the four changed
files checked out at `46289075`, the binaries being the same ones — and again after every edit.
Every delta is the round's own work, and no sweep gained a hit line.

- `blockers`, `callers`, `capabilities`, `entries`, `owed`, `overstated`, `parts`, `unread`:
  identical.
- `counts`: +50 sentences governing a row word and +3 attributed counts, **all three in the
  "clause with no rows below it" bucket**; the agreeing count stays 151 and the contradictions 4.
- `tables`: +15 sentences naming a table and +6 attributed key citations, **all six agreeing** —
  Table 30's `/Kids` and `/Count`, Table 29's `/DPartRoot`; absent stays 101 and denials 6.
- `pointers`: +22 paths (+10 live, +6 unrooted, +6 not carried, the last being
  `doc/pdf.js/…/issue8088.pdf`, which this tree reaches through a submodule); absent stays 98 and
  undefined 13.
- `quotations`: +3 quotations in +2 documents — the ADR's own restatement of the rule as a Markdown
  blockquote and two spans this file quotes from a sweep's own vocabulary, all three in the
  "sharing too little to be a quotation of one" bucket; verbatim stays 2801 and diverging 38.
- `inapplicable`: the shared words *Issue* and *Collection* gain the three source files this round
  annotated; both sort to the noise end and no cousin is new.
- `overtaken`: one more decision record, no change to the 48 overtaken notes.
- `check` unchanged but for one line number; `applied` +7 places read and +11 naming an erratum,
  read-first list unchanged — the round's sentences write an erratum's replacement in italics,
  which is the convention that keeps them off it. `moved` identical.
- `quoted` and `unpriced` not run: no page-list note was touched.

## What contradicts the briefing

- Nothing does. The briefing's population figure — 73 after the twelfth use's twelve verdicts — is
  confirmed by the greps rather than trusted, and the multi-issue parse reproduces the same way at
  310 and 75.
- `tools/worktree.sh` reported `6/6 skip-worktree` for this worktree, as round 794's repair
  intended.
- The briefing offered the fourth blind spot as an alternative target if the head did not obviously
  outrank it. The head did — and it arrived *by* that blind spot, both of its carets striking
  nothing, so the round paid the debt the blind spot names rather than instrumenting it. What the
  round found about the instrument instead is a defect in the recipe's own step 3, which no
  blindness in `check` could have produced.
- §5's binaries were not rebuilt: this is not a fifth round, and nothing outside the gates was
  measured. The corpus census was run from this worktree's own build directory (trap 15).
- CI on `origin/main` is red for the pre-existing reason `round.sh` names.

## One thing the round nearly lost

The before-half of a sweep run is taken by checking the changed files out at the base commit and
back — the binaries resolve the tree from `CARGO_MANIFEST_DIR`, so running them inside a copy of
the base commit measures the worktree and prints *identical* for everything, which this round saw
first and did not believe. `git checkout HEAD -- <paths>` restores the last **commit**, and the
round's `cargo fmt` reflow and clippy repair had been made after its checkpoint, so the last dance
threw both away. They were found by re-running `fmt --check` and `clippy` before the commit, which
is `doc/todo/02` §6's rule — check the file, not the command's exit status. `doc/todo/01`'s sweep
section now carries it beside the invocation. The gates had already run against the repaired
sources and the restored text is byte-identical to what they saw.
