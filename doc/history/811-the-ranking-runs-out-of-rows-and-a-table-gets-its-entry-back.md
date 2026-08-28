# 811 — The ranking runs out of rows, and a table gets its third entry back

Finding: **the errata selection rule's row ranking has plateaued to a head of two annotations with
thirty-nine rows tied at it, and the same population ranked by *issue* still has a head three
times its next tier** — which took the round to Issue #346, five table cells, and a ledger row
that had deleted an entry the standard states.

Date: 2026-08-28. Argued in ADR 0743.

Touched: `doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`,
`doc/conformance/ledger.toml` (§14.8.5, §14.8.5.3, §14.8.5.5, §14.8.5.8),
`crates/pdf-model/src/structure.rs`, `doc/adr/0743-…`.

## What the rule was asked and what it answered

The fifteenth use. `spec-errata emit` over `doc/ISO_32000-2_sponsored_EC3.pdf`, step 2's two
greps, step 3's attribution with ADR 0732's family guard, step 4's two rankings. The base
population reproduces the fourteenth use's closing arithmetic exactly — **302 issue numbers carry
a strike or a caret under the recipe's own single-issue line parse and 61 were named nowhere**,
which is 804's 71 less its ten verdicts; the multi-issue parse's 310 and 63 reproduce the same
way. That is the sixth consecutive use whose base count derives rather than quotes.

Twelve annotations fall outside any family the ledger has a row for — 3 under clause 2, 3 under
clause 3, 4 under Annex A, 2 under Annex H — which reproduces the split the thirteenth and
fourteenth uses recorded, so the guard is doing the same thing on a third run.

**And then the ranking had nothing to say.** Over live rows eight rows tie at two annotations and
four hold one. Over every row **thirty-nine** tie at two — twenty `implemented`, eight
`out-of-scope`, six `partial`, two `reported`, two `inapplicable`, one `writer-side` — and
nineteen hold one. ADR 0653's tie-break was written to choose among three rows and cannot choose
among thirty-nine; reading the plateau out would be reading most of what the rule has left.

## The decay reading the round was asked for

The base population, off the uses whose record carries it — the third, then the ninth through this
one: **133, 111, 104, 99, 85, 73, 71, 61**. Six or seven a use, steady, with about a fifth of the
collection's strike-or-caret issues still unread. The *head* has fallen much faster than the population — fifteen at the fifth use, seven at
the third and fourth, three at the thirteenth and fourteenth, two here — because the rule takes
the head each time and the head is where the mass was.

So the two curves say different things, and reading only the second would have retired the rule on
the wrong evidence:

- **the population's curve says the rule is not finished.** Sixty-one specific sentences of the
  standard that nobody here has read is not a tail;
- **the head's curve says the *unit* is finished.** A row count that can only be one or two is not
  a ranking. What collapsed is step 3's resolution, not the yield.

The repair is in the recipe as **step 5**: where the row counts go flat, rank the same annotations
by issue. It is not a new instrument — step 3's own last sentence already says to read one issue
whole across every heading it appears under, so the issue was always the *reading* unit and the
ranking simply was not asked in it. By issue the population still has a shape: one at six
annotations, two at four, two at three, thirty-three at two, twenty-five at one.

One more number, and it is step 4's argument turning into arithmetic: **43 of the 61 unread issues
touch only a settled row, 12 touch a live one, 1 touches both.** What this rule has left to find
sits almost entirely on rows claiming to owe nothing.

When it retires: not on a flat row ranking, and not on a count. On the day an issue read whole
stops changing anything — which did not happen this round and has not happened in six.

## The head, and what it was worth

**Issue #346**, the only issue in the population reaching three ledger rows, six annotations, five
of them bare carets. Every one adds the same two words to a standard structure attribute's
requirement cell: Table 382's `/ContinuedList` and `/ContinuedFrom` and Table 385's `/Type`,
`/BBox` and `/Subtype` all become *not inheritable*. `doc/errata-read.md` has the rectangles
against `pdftotext -bbox`; the outline filed the Table 382 pair one clause late under §14.8.5.6
and the Table 385 strike one clause late under §14.8.6.2, which is ADR 0712's placement rule doing
the round's work before a verdict was written.

**§14.8.5.8's row said Table 385 states two entries, and its own parenthesis recorded the session
that made that true.** The three-hundred-and-eighty-seventh corrected the row's table number and,
in the same sentence, deleted `/Subtype` as belonging to Table 363. The table states three: the
`/Subtype` row straddles a page break and is printed under §14.8.6.2's heading in the conversion,
which is the same artefact that misfiled the erratum — one conversion misleading two rounds four
hundred sessions apart. **Issue #346's only strikeout is over the requirement cell of the entry
that sentence deleted.** The same sentence also said Table 385's four type names are Table 363's:
Table 363's fourth is `Background` and Table 385's is `Inline`, and their `/Subtype` conditions
differ with them.

**And the row was `inapplicable` while one of its three entries had a reader.** `Tree::bounds`
applies §14.8.5.3's priority over every PDF-native owner, so an `Artifact`-owned `/BBox` has
always been answered — under a note saying the table was read and dismissed. That is the
eighteenth sweep's shape with both of its sides inside one note, which is why no sweep printed it.
The status stood on "it describes rather than draws", word for word the reason §14.8.5.7 next door
records being taken off `inapplicable` for.

**§14.8.5.5 was the second row the same erratum opened**, and its note named one of Table 382's
three entries. `/ContinuedList` and `/ContinuedFrom` are PDF 2.0's, and the clause says they
control the interpretation of an `L` element against other `L` elements — which this program
interprets. Nothing reads either and nothing reports it.

## What moved

- §14.8.5.8 `inapplicable` → `partial`, with the third entry restored and the two tables' cells
  separated.
- §14.8.5.5 `inapplicable` → **`silent`**, which is the only `silent` row this ledger carries. It
  had none, and `CLAUDE.md` says why that is not the same as having nothing hidden: what ships is
  the gap inside a feature that is already there.
- §14.8.5's parent row loses a count of its own family that had gone stale by four, and gains the
  two rows the family's own test was never applied to.
- §14.8.5.3 gains a question, recorded rather than paid: priority 1 names the `NSO` owner, whose
  condition this program does not meet, and `Tree::attribute` admits `Owner::Namespace` beside the
  five PDF-native owners with no argument written anywhere.
- `Tree::bounds`'s doc comment stops naming one of the two tables it reads.
- `an_artifact_owned_bounding_box_is_the_same_rectangle_as_a_layout_owned_one`, calibrated per
  trap 13 both ways above the change: dropping `Artifact` from `Owner::is_pdf_native` fails the
  first assertion, and giving the discriminating element the `Artifact` owner it must not have
  fails the third.

## Gates and sweeps

Full §2 sequence, all green: formatting for both workspaces, `RUSTFLAGS="-D warnings"` clippy over
the workspace and all targets (the only output was `viewer-qt`'s build script's gcc
`-Wmaybe-uninitialized` lines on a cold build, which are documented), nextest, the doctests, the
`fuzz/` check, the sandbox build, corpus, oracle, text extraction and its two neighbours, both
censuses, dates, xmp, jpeg2000, quorra, fixed documents, and `cargo test -p conformance`. The
ledger binary was run and its own output committed.

§4's sweeps before and after, against a pristine checkout of the base commit with its own build
directory (`doc/todo/01`'s second method). `blockers`, `callers` and `parts` identical;
`capabilities`, `entries`, `pointers`, `tables`, `owed`, `quotations`, `overstated` and `overtaken`
differ only in line numbers and in the counts the new prose adds. Two substantive changes, both
expected: `inapplicable` loses §14.8.5.5 and §14.8.5.8, and **`unread` gains one hit** — §14.8.5.8
`partial`, `/Type` and `/Subtype` "claimed unread that the tree quotes", witnessed by
`structure.rs`. That is the sweep's own dominant noise shape, a short key the standard's shared
vocabulary reaches everywhere: what `structure.rs` quotes those two names for is Table 363's
property list, which is the other table entirely — and it is the hit the row's new note is about.
`spec-errata check` prints the same 453 landings before and after, so the round's own quotations
add none.

Not run: `quoted` and `unpriced`, which take an oracle log and read the oracle's page-list notes,
none of which this round touched; §5's binaries, since this is not a fifth round and nothing was
measured.
