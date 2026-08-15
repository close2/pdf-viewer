# 0372 — The eighth sweep becomes a program, and the transform method nothing named

**Status.** Accepted.

## Context

`doc/todo/01`'s binding rule for a sweep round: commit one more prose sweep as a program before
running any of them. Seven of the fifteen were commands — `conformance --bin entries`,
`--bin quotations`, `--bin unread`, `--bin blockers`, `--bin capabilities`, `--bin retired`,
`--bin callers` — and eight were descriptions.

The **eighth** sweep is the one that reads what a note *points at* rather than what it claims.
Every other sweep here asks whether a sentence is still true; this one asks whether the file it
names still exists. A pointer decays faster than a claim, because deleting a file is a thing rounds
do on purpose: §8.9.6.1's note sent a reader to `doc/todo/20` for a refusal ADR 0169 had
implemented, and the session that made the sentence false deleted the file in the same commit;
`doc/todo/12` stood in six places under `crates/` after the item was done and gone.

**It is the most mechanical of the eight left, which is the argument for building it and was very
nearly the argument against.** The five-hundred-and-twenty-fifth passed over it and built the
caller sweep instead, on a reason that was about the caller sweep rather than about this one — "it
is the sweep whose *number* is the finding", and its level had been session-local on every run. That
reason does not transfer. What decided it here is that the by-hand runs of this sweep have been
*losing* findings to their own looseness: the three-hundred-and-ninety-fourth recorded that "the
sweep's own file globbing has to know where a file lives — an `examples/foo` under
`crates/<crate>/examples/` read as dead until the glob was fixed", which is a session-local repair
of a session-local script, and the same repair was owed again the next time. A glob written afresh
each round is a different instrument each round.

## Decision

**`cargo run --release -p conformance --bin pointers`** is the eighth sweep as a program
(`tools/conformance/src/pointers.rs`, fourteen unit tests, 0.33 s over the whole tree). Four
decisions are worth their lines.

**It is `pointers` and not `citations`, because `citation.rs` in the same crate already means a
citation of the *standard*.** Two populations, two questions: that module checks a `§` against ISO
32000-2's clause index, and this one checks a path — and the symbol half of a path — against this
tree. The ledger gate covers a third, the `code` and `test` arrays, whose sites it resolves
including the `::function` half. What was unchecked, and what this reads, is every pointer written
in **prose**: a note's sentence, a doc comment, an ADR's paragraph.

**A pointer is resolved from where it is written.** `tests/import_policy.rs` in a doc comment under
`crates/viewer-host/src/` means that crate's `tests/` directory; the same words in
`doc/QUORRA_FEEDBACK.md` mean a directory in the render library's tree. So a fragment whose head is
`src`, `tests`, `examples` or `benches` is joined to the crate the mentioning file belongs to — the
crate being the nearest directory above it with a `Cargo.toml`, which is the manifest rule
`--bin callers` already uses — and is `Unrooted` where the mentioning file is in no crate. That is
the honest answer for a document addressed to another project, and it keeps a whole population out
of the reading list without a hand-maintained list of documents to skip. **Both of this run's path
defects were found by that rule and by nothing else**: they are unrooted fragments naming a file of
their own crate that has never existed.

**The noise is classified rather than filtered, and each shape gets a rung.** A fragment that
resolves in *another* crate (`pdf-syntax` naming `examples/callgrind_interpret`, which is
`pdf-model`'s) is neither dead nor exactly right — it resolves for a reader who searches and does
not say where. A **form** rather than a citation (`doc/todo/NN-slug.md`, `crates/foo.rs`, a glob) is
a metavariable and not a pointer. A path the tree deliberately does not **carry** — a submodule
nobody checked out, a fuzz corpus a run builds, the specifications unpacked from
`doc/specifications.zip` — is absent for a reason that is not decay. What is left is the reading
list, and it is classified once more by `retired::kind_of`: a **correction quoting the pointer it
retired** is this sweep's oldest false positive, standing on every run since the
three-hundred-and-seventy-fifth, and a **standing** dead pointer is the finding.

**The symbol half is in, because it is the same sentence's other end.** `doc/todo/01` names it as a
sweep somebody could build — "a note that cites `crates/foo.rs::some_test` is making the same kind
of claim, and the checker only verifies the ones in the `test` array". A symbol is looked for as a
*definition* (`fn`, `const`, `static`, `struct`, `enum`, `trait`, `type`, `mod`) rather than as a
bare identifier, because a comment naming a neighbour is how this tree explains itself and is not
evidence the item is there. Its cost is that a citation of a field or a macro reads as undefined —
a hit to read rather than a claim missed — and its finding this round is exactly the case it was
predicted for: a function that moved when a module was split, with the file it left still named.

**Not a gate**, for ADR 0249's ratio reason and one of its own: a dead pointer is sometimes the
right thing to write. A round may cite the file it is about to add, and a correction *has* to quote
the pointer it retired. A build that failed on either would teach rounds to write around the
checker.

## Consequence

First run: **4609 path pointers — 2525 live, 104 absent, 14 in another crate, 1616 unrooted, 106 a
form, 244 not carried — and 48 symbol pointers, 9 of which no file defines.** Of the 104 absent, 84
are in `doc/adr/` and in `doc/todo/01`'s own records of earlier runs, which is the dominant shape
over the ADRs and is **not** a defect: an ADR saying the remaining question is "recorded in
`doc/todo/47`" is a true statement about the day it was written, and the file being closed since
does not falsify it. **Three defects, and all three are the sweep's own subject.**

- **`crates/viewer-host/src/policy.rs`** explained `resolve_import`'s purity by "which is what
  `tests/import_policy.rs` does" — a file that has never existed in any commit of this tree. The
  policy *is* tested, one file over, in `tests/host_mappings.rs`.
- **`crates/viewer-accessibility/tests/tree.rs`** said what the bus does with the tree "is
  `tests/atspi.rs`'s question" — also never written. The half of that question which can be asked
  without a session bus is `src/bridge.rs`'s own test, that a build with no adapter names its
  shortfall; nothing in this tree drives a real bus, and the comment now says so.
- **`doc/errata-read.md`** quoted the alternate-image divergences as being in
  `content.rs::alternate_image`. The file exists and the function is not in it: it moved to
  `content/image.rs` when the module was split. The *symbol* half found this and the path half
  could not.

**And the round's sharpest finding is not this sweep's.** §12.8.2's note said "`FieldMDP` and `UR`
are recognised where a `/Reference` names them"; **nothing in this tree names the string
`FieldMDP`**. `has_transform` takes a method name and `DocMDP` is its only caller, so a document
stating a FieldMDP transform reaches a reader as an ordinary signature. §12.8.2.4 was `partial` on
the strength of that recognition — `doc/todo/01`'s fourth failure shape, where the half of the note
saying what *is* done is the wrong half — and is `reported` now: none of the clause is executed,
and what a person is told is `notes.rs`'s sentence on every signature, that this program answers
two of the three questions a signature asks and names the third. The protection the clause exists
for is not missing from the program — §12.7.5.5's signature field lock, which Table 259 says a
writer *copies* into these parameters, is read and enforced — it is simply not this transform doing
it. Building the transform is `has_transform(document, dict, b"FieldMDP")` plus Table 259's
`/Action` and `/Fields`, and it is named rather than done.

**The ninth sweep paid in the source again, in a block.** Seven mis-attributed table numbers, six
of them in `pdf-model`: an embedded font stream's `/Length1` under Table 126 (predefined spot
functions) instead of Table 125, an ICC profile stream's `/N` under Table 66 instead of Table 65, a
`/ShadingType` under Table 78 instead of the common Table 77, a Type 0 font's `/Encoding` under
Table 122 instead of Table 119, a widget's normal appearance `/N` under Table 168 (border style)
instead of Table 170 — in `view.rs`, which is where the five-hundred-and-twenty-fifth's single
finding was — a parent tree's `/Nums` under Table 354 instead of Table 37, and `/Enforce` under
Table 148 (the names defined for it) instead of Table 147 (the dictionary that states it). **A
block of consecutive wrong numbers written in one sitting is this sweep's oldest shape**, and five
of these seven are in one `enum`'s doc comments.

**The tenth sweep's finding was written by a sweep round.** §14.8.2 said "[o]f the twelve rows
below" over a family of thirteen, and all thirteen have been in the ledger since it was generated —
so the count was wrong when it was written, in the five-hundred-and-first, by a round whose whole
subject was rows that had stopped being true.

## What this does not do

It does not read a pointer written with spaces in the file's name beyond the first — `doc/hayro vs
this project.md` resolves because `holds` treats a space as a separator after the stem, which is a
repair rather than a parse. It does not follow a fragment out of the workspace: a citation of
another project's `crates/quorra-gpu/examples/zoom.rs` is collected and reported absent, because
this tree has a `crates/` and that is where the token points. And it says nothing about whether the
file it found is the *right* file — `tests/host_mappings.rs` is now named by `policy.rs` because a
person read it, not because the program checked that the test tests the policy.
