# 626 — The threshold a clause stated and three documents denied

The band `doc/todo/01` named at the bottom of its blame ordering, read with the rule session 620
derived. Nine rows, six of them wrong, and one of the six was a computation the standard defines
and this tree had recorded three times as undefined.

Date: 2026-08-20.
ADR: [0460](../adr/0460-the-threshold-a-clause-stated-and-three-documents-denied.md).

Touched: `crates/pdf-model/src/requirements.rs` (`penalty_total`, `PENALTY_LIMIT`, a unit test and
the module header), `crates/viewer-core/src/notes.rs` (the note), `crates/viewer-core/tests/headless.rs`
(two tests), `crates/pdf-model/src/action.rs` (module header),
`crates/pdf-model/src/attachment.rs` (module header), `doc/conformance/ledger.toml` (§12.11.3,
§12.11.6, §12.6.4.6, §12.6.4.9, §12.6.4.10, §14.8.2.2.1, §14.8.2.2.2, §14.13, §14.13.2, §14.13.9),
`doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## How the band was ordered, and whether blame agreed with 620

Re-derived rather than taken, which is 616's lesson and 620's: `git blame --line-porcelain
doc/conformance/ledger.toml`, each `partial` or `reported` row's own `note = ` line, ranked by
where its commit falls in `git log --reverse`.

**It agreed with 620 on all nine.** Three parallel rounds were running beside this one and none had
merged a row out of the band. The band is *nine* rather than 620's eight because §12.6.4.6, §12.6.4.9
and §12.6.4.10 are three rows sharing one commit, and the boundary is unmissable: ranks 1 to 9 sit
at four commits, and rank 10 is five hundred commits later.

620's rule — rank by blame, then read the row whose stated reason is a claim about this codebase
rather than about the standard — put §12.11.3 and §12.11.6 at the top, and that pair was the round's
work.

## §12.11.3's threshold, and why three documents said it did not exist

The two rows and `requirements.rs` all said the standard states no threshold on a document's
requirement penalties, so §12.11.6's "cannot be met" was "never made decidable" and the program's
choice to draw the document anyway rested on that.

§12.11.3's fourth paragraph states one:

> In the situation where the penalty values are being used to evaluate the presentation of the
> base PDF document, and there exist no other alternates, if the penalty value exceeds 100 then
> the PDF processor should not attempt to display or process the document.

The computation is a **sum over the requirements that cannot be met**, and neither half is an
inference: Table 273 bounds one entry at "between 0 and 100 (inclusive)", so a threshold on a single
entry could never fire; the paragraph above the threshold names "the total penalty points"; and
Table 273 says a penalty is "the penalty value to be applied when this requirement cannot be met by
a PDF processor", which is exactly `requirements::unmet`'s population.

`requirements::penalty_total` performs it, `PENALTY_LIMIT` is the number, and `viewer_core::notes`
says the total and that it passed. The clause is still not obeyed — but it says `should`, and the
departure now rests on that word rather than on a silence. The decision is taken in the host's
direction rather than in `pdf-model`, which is `CLAUDE.md` principle 3's shape for a restriction a
document asserts over its reader: the four levels can be added where the host is.

**Nothing on the disk could have found this.** 0 of the 974 corpus documents state a `/Requirements`
array. Both witnesses are built and mutation-checked: sum → max fails the unit test, `>` → `>=`
fails the headless one.

## §12.6.4.9, and two rows reading one sentence two ways

§12.6.4.9's row said `Sound` was "[a]djacent to clause 13's exclusion and not covered by it, because
§12.6.4.9 is in clause 12". The clause opens with the sentence §12.6.4.10 opens with, word for word
— "The features described in this subclause are deprecated with PDF 2.0. They are superseded by the
general multimedia framework described in 13.2, 'Multimedia'." — so the two neighbours had held
opposite readings of one sentence since the ledger was written, and the one that quoted it was
right. It matters because principle 5's exclusions are a *closed list*, and a refusal justified by
stretching one across a clause boundary is how a closed list stops being closed.

## The three action rows' evidence, which reached none of them

620's newest shape, three times over. All three cited
`action.rs::a_name_the_table_does_not_hold_is_not_an_action`, which asserts that `/Teleport` yields
**no** action — the one path that never calls `action::refused`. `Launch` was reached obliquely by
`a_next_chain_is_flattened_in_execution_order`, which splits the sentence off at the colon; `Sound`
and `Movie` were reached by nothing in the tree.

The witness is built, because the corpus has almost none: of the 974, exactly one states a
`/S /Launch` action, **none states a `/S /Sound` or a `/S /Movie` one**, and
`multimedia_annotations.pdf`'s three `/Sound` names are §13.6.2's annotation subtype and §13.3's
sound object rather than §12.6.4.9's action. `a_click_on_an_action_this_program_will_not_perform_says_which_and_why`
clicks a link on each of the three and asserts the exact sentence reaches `Event::Reported`, plus
the negative case — an action this program performs says nothing about declining. Mutation-checked:
deleting the `Movie` arm fails it.

The rows' `code` array named `crates/viewer-ui/src/bin/pdf-viewer.rs`, which contains none of this;
the printing is `dispatch.rs`, and the sentence becomes a note in `viewer_core::interact`. The
eighth sweep passed the pointer because the file exists.

## §14.13's eight objects, and the one that is not a dictionary entry

§14.13's row justified reading `/AF` off any dictionary by saying the clause "lists seven objects
that may carry one and says the same sentence about every one". §14.13.1 lists **eight** — the
eighth is a metadata stream dictionary, which gets no subclause here and about which §14.3.2 states
nothing — and the sentence is not the same for the third: a graphics object's array lives in a
marked-content property list under `/MCAF`, which is why `attachment.rs` has two readers rather than
one. A reader who believed the row would state seven sites and find six. Both the row and
`attachment.rs`'s module header now say so, and §14.13.9's "the clause's seven places" with them.

The row also stopped restating its children's counts, which did not add up against them: "6 on
catalogs, 30 on structure elements" beside §14.13.6's 37 arrays. It keeps the one number that is its
own, with the command.

## §14.8.2.2.1's sentence, which is §14.8.2.2.2's

The row named "the clause's other test" and quoted a sentence that is two paragraphs under Table 363
in the *next* subclause. §14.8.2.2.1's own `shall` — "[w]here artifacts are to be included in the
structure tree, they shall be included through the Artifact structure element type, and shall not be
considered real content" — had never been quoted by either row, and both halves are implemented.

The confirmation on §14.8.2.2.2 is the paragraph after Table 363: `Alt`, `ActualText`, `E` and
`Lang` "may also be used as entries in the property list of an artifact", and `Interpreter::accessibility`
asks *every* `BDC`'s property list for those four, so an artifact whose `ActualText` carries a page
number replaces the readback the way §14.9.4 says.

## What `spec-errata emit` found, run before writing

Three errata the rows did not carry:

- **#484** turns §14.8.2.2.2's `shall` about EXAMPLE 1 and EXAMPLE 2 into NOTE 2. Nothing moves —
  a reader that accepts both forms accepts what either text describes — but the row's *ground*
  moves from obedience to a note.
- **#568** replaces §14.13.1's designation sentence with the first statement of both `/AF` forms as
  a `shall` each, and this tree already reads both.
- **#86** puts a UTF-8 `shall` on every name key, stated on §14.13.2's page; `Name::as_str` is
  already the fallible answer.

## What was priced and left

**An artifact census.** §14.8.2.2.2's "30 of the 953 corpus first pages" is a one-off run against a
corpus that is now 974, and no command re-derives it — `witness_census` counts names, and
artifact-hood is a `BDC` tag the interpreter reads. About sixty lines in `file_attachment_census`'s
shape, one corpus pass, and the same pass settles §14.13's per-site `/AF` counts, which three rows
state and no two of which agree. In `doc/todo/01`.

## A gate that failed for a reason that was not in the tree

The sequence was started in the background and the ledger sweeps were run beside it. The oracle
section came back with **38 not comparable, 873 agreeing, in 218 seconds**, and `tools/state.sh`
exited 101. Re-run alone on the same tree it reported **13 and 907 in 57 seconds** and exited 0 —
the numbers this project has been printing for rounds.

Nothing had changed but the machine's load. Three gate lines spawn other programs with wall-clock
budgets — poppler, mupdf and ghostscript for the oracle, `pdftotext` for the text line, the device
for quorra — so a reference that would have finished loses to a `cargo` build in another terminal,
and the loss arrives as *this tree failing to compare*. That is the worst shape a false result can
take, and it is now written into `doc/todo/02` §2 beside the sequence it is about. Every figure
reported for this round is from a run with nothing else going.

## The sharpening `doc/todo/01` gained

620's rule ranks by what kind of claim a reason makes. This round adds where the answer would be:

> **A note that says the standard states nothing has to name where it looked.**

All three of this round's denials were of a sentence in the *final paragraph* of the clause, which
is where a standard puts the consequence after it has finished defining the terms. It is a rule for
writing a row rather than a sweep, and it is the only kind that makes the next re-read cheaper.
