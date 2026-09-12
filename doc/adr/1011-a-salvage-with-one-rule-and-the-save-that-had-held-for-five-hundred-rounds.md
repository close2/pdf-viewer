# 1011 — A salvage with one rule, and the save that had held for five hundred rounds

Session 990. Status: **accepted**. Two leads from the direction review's first attempt (session
984), both principle-level and both in this round's corridor, and a third question the leads
answered on the way. Amends §7.3.3's and §7.5.6's rows; corrects `doc/todo/05` item 2; adds
`tools/conformance/tests/state_sections.rs` and `tools/state.sh save`.

## 1. The salvage, arm by arm

`pdf-syntax/src/lexer.rs::salvage_number` reads a number out of a run ISO 32000-2 §7.3.3 does
not spell. Its doc comment said, of `--5`:

> producers emit `--5` by prepending a minus to an already-negative value, and both Acrobat and
> pdf.js read it as -5. Reading it as +5 would silently mirror geometry.

and the unit test beside it said "[m]alformed numbers occur in real files; other viewers accept
them, so we must." Both sentences are in the lexer's first commit (`f26bbcb1`, 2026-07-26) and
neither had been challenged since — the same commit's *third* such sentence, "matching what other
viewers do" over the digit-less run read as zero, is the one ADR 0303 took out in session 468.
Principle 5 states the direction of inference once and it runs one way: agreement with another
reader is evidence that we read a clause right, never the reason for a behaviour. So each arm of
the function was read with one question — *what clause makes this the reading?* — and answered
with a clause, an honest choice, or a hack.

| arm | run | read as | clause | verdict |
|---|---|---|---|---|
| leading signs collapse to the first | `--5`, `-+5`, `+-5` | −5, −5, +5 | none: "optionally preceded by **a** sign" | **hack** — a value invented for an empty prefix, justified by two references; now the keyword `--5` |
| a sign after the digits ends the run | `1-2`, `1.5-2` | 1, 1.5 | none makes a value; §7.2.3 makes it one token | **choice** — the grammar's prefix; kept, stated as a choice |
| a second point ends the run | `1.2.3` | 1.2 | same | **choice** — same, kept |
| any other byte ends the run | `12pt`, `5f`, `1e` | 12, 5, 1 | same; §7.8.2 decides the tail (ADR 1004) | **choice** — kept; the operator line is ADR 1004's |
| a digit somewhere, none in the prefix | `.-1`, `..5`, `-x1` | 0 | none | **hack** — the zero ADR 0303 took out of `.`, surviving one condition below it; now the keyword `.-1` |

The three middle arms are one rule, and the two hacks are what the rule says nothing about. So the
function is now that one rule and states it as the choice it is: **the number §7.3.3's own grammar
spells off the front of a run it does not spell whole** — one optional sign, decimal digits, at
most one PERIOD, stopping at the first byte outside that, which is `fixed_format_number`'s scan
without its digit bound — and **a run the grammar reads nothing of is the keyword §7.2.3 makes
it**, whether it states a digit further along or not. The parser refuses the keyword where an
object was expected, exactly as it refuses `.` since ADR 0303; the interpreter reports it as an
operator it does not know and drops the operands before it, ADR 0302's rule. `Salvage::dropped`
is never empty now and never the whole run, and the doc comment says so; `Lexer::salvaged` is
`None` for a keyword, so `pdf-model`'s content reader has nothing to ask about one.

**Why refuse rather than keep the leniency with an honest comment.** The prompt allowed either,
and the difference is what the two hacks *are*. `12pt` → 12 reads a value the run's front spells
and drops a spelling; `--5` → −5 reads a value nothing in the run spells, because the grammar's
prefix of `--5` is empty and −5 is one of two plausible intentions (a redundant sign, or a
double negation), chosen by asking Acrobat. `.-1` → 0 is not even plausible — it is the value
trap 5 names as the one that draws in the wrong place and says nothing. A malformed file is
outside the standard and robustness is the second denominator, so the question is what a damaged
stream is *owed*, and the answer this tree has given since ADR 0303 is a report naming the run,
not a mark placed by a guess.

### The population, counted before the choice was believed

A temporary test — not committed — lexed every page content stream of `doc/pdf.js`'s 974
documents and `doc/corpora/`'s 275 through `Lexer::salvaged` and classified each salvage by the
arm that produced it (4689 pages, 17.6 million tokens; form XObjects, patterns and glyph
descriptions are outside it):

| arm | tokens | documents | where |
|---|---|---|---|
| leading signs collapse (`--5`) | 6 | 4 | all `format-corpus/govdocs1-error-pdfs` |
| digit somewhere, none in the prefix (`.-1`) | 264 | 11 | all `govdocs1-error-pdfs` |
| a sign after the digits (`1-2`) | 3 696 | 19 | 18 `govdocs1-error-pdfs` + `poppler-90-0-fuzzed.pdf` |
| a second point (`1.2.3`) | 68 824 | 22 | 21 `govdocs1-error-pdfs` + `poppler-90-0-fuzzed.pdf` |
| any other byte (`12pt`, `5f`) | 52 126 | 29 | 27 `govdocs1-error-pdfs` + `issue19484_2.pdf`, `poppler-90-0-fuzzed.pdf` |

Every witness of both hacks is in a deliberately damaged file, in a stream whose surrounding bytes
are already garbage — `)]TJ\n--7.44 -1.15 TD` beside `M.SoL)10.3oler`, `-d5TJ` four times in
one line — and **no file a producer wrote carries either shape**. The sentence "producers emit
`--5` by prepending a minus" had no witness on this disk, and this is the disk ADR 1004's
SafeDocs sample was taken from, which found the same: every salvage in the crawl is a stream
that lost its white space.

`examples/display_list_digest` over all 1249 first pages, both arms, moves **one document**:
`507676.pdf`, the wholly damaged page ADR 0303 already named, 33 854 commands both ways, two
reports gained (`--7` inside an array and as an operand — both were `-7` into a `cm`). Nothing in
`doc/pdf.js` moves.

**And two documents moved that were not mine, which is the round's habit.** The first before/after
pair showed `issue18032.pdf` gaining a report and `issue20513.pdf` moving ten kilobytes of
parameters with *no* report — trap 1's shape, and a sentence away from being written up as a
silent defect of this change. The census could not see either document, which was the first
clue. The bisection ADR 1004 recommends — `git diff <file> > x.patch`, `git apply -R`, digest,
`git apply` — put `issue20513.pdf` wholly on the neighbours' side (two siblings were editing
`colour.rs`, `image.rs` and `transparency.rs` in the same working tree), and `issue18032.pdf`'s
report turned out to be a neighbour's *transient*: the same document digested with and without
the patch, back to back at one neighbour state, is byte-identical three times over. The rule is
the one ADR 1004 wrote and it has a sharper form: **a before and an after taken ten minutes apart
in a shared tree are two neighbour states, and a document that moved is attributed only by the
two arms run back to back.**

### Calibration

`a_run_the_grammar_reads_nothing_of_is_a_keyword` holds nine spellings as keywords with no
salvage recorded; `malformed_numbers_salvage_a_leading_value` holds the prefix rule on six, with
a doc comment that no longer names another reader as its reason; the overflow test's salvage
arm now reaches `f64::MAX` through a second point rather than through a doubled sign; and
`numbers_without_digits.rs::a_run_whose_front_spells_no_number_is_reported_rather_than_read`
is the page-level pair — `-12 Tf` and `.5 Tf` draw in silence, `--12`, `.-12`, `-+12` and `..5`
are each reported by name with the show they cost. The old assertions were the calibration in the
other direction: `--5 → Integer(-5)` and `.-1 → Integer(0)` were green on the old code and are
the negation of the new ones. `content/reader.rs`'s test of the same five runs has its two
expectations changed — outside this round's corridor, two lines, and said so.

### What the sweep for the shape found

`grep -rn -i -E "other (viewers|readers|renderers|implementations) (accept|do|read|treat)"` over
`crates/`, `tools/` and `fuzz/` names the two first-commit sentences and one more that is live:
`render-cpu/src/convert.rs:122`, "clamping matches what other viewers do", in a crate another
round owns this block. It is reported here rather than touched. `pdf-font/src/loading.rs:862`
("ADR 0433 measures what the other renderers do") is evidence-shaped and stays.

## 2. The save round-trip: why ignored, whether it passes, what it costs, and the line

`crates/pdf-model/tests/save_round_trip.rs` is `#[ignore]`d for the reason its attribute states —
corpus-scale, two reference processes per document — and it is in no §2 line for the reason ADR
0334 states: `doc/todo/05`'s standing rule that an instrument's numbers enter §2 only after they
have held across rounds, and a binary of its own so that it does not ride another gate's
`-- --ignored`. It was built and run in session 499, and **no history file since records a run**;
the file was touched three times (sessions 568, 872, 967) for the renames and the restriction
census, never for its numbers. So the rule that kept it out had no way to let it in.

**It passes today, and the distribution is session 499's to the document**, with one movement:

| | session 499 | session 990 |
|---|---|---|
| refused open | 2 | 2 |
| no page to annotate | 7 | 6 |
| documents with a fillable text field | 80 | 80 |
| refused by policy under `Restrict(On)` | 9 | 9 |
| saved under `Restrict(On)` (free texts, fields) | 935 (933, 80) | 935 (933, 80) |
| nothing to save | 7 | 7 |
| save refused by construction | 23 | 24 |
| prefix / readback / panic failures | 0 / 0 / 0 | 0 / 0 / 0 |
| reference exclusions (On, Off) | 9, 4 | 9, 4 |
| disagreements, reference errors | 0, 0 | 0, 0 |
| `Restrict(Off)`: saved, refused | 8, 1 | 8, 1 |

The movement is one document that had no reachable page one in 499 and has one now — a page-tree
recovery landed between — and is refused for its rebuilt cross-reference table like the other
twenty-three. **Its cost**: 12.5 s on the first run and 10.9 s on the third, on a machine with
one round's build running, 0.97 GiB peak over the process tree under `tools/bounded.sh`, no
clock judged anywhere — the only duration in the instrument is the thirty-second budget on each
reference call, so a loaded machine can cost it a `reference would not answer` line and nothing
else, and it needs no re-run alone to be believed. It is cheaper than `dates`.

**So the counts ratchet**, on the file's own promise. Every capability count has a floor (935
saved, 933 free texts, 80 fields, 80 fillable, 8 and 2 under `Off`) and every population that
shrinks the judged set is a set of **names** checked in both directions — refused open, pageless,
policy-refused, nothing-to-save, save-refused under each level, and reference exclusions as
(document, reference) pairs — because a name that leaves is a document the writer began to chain
to or a reference began to read, and either is examined rather than quietly enjoyed (ADR 0282's
rule for the oracle's lists). The ratchet runs only over the 974-document tracked population and
prints why where it does not (ADR 0970's rule). Calibrated by breaking it twice before it was
believed: the `saved` floor raised to 936 fails naming the count; `scan-bad.pdf` deleted from
the refused list fails naming it as *joined*. The test carries the `// no sandbox worker:` line
`sandbox_gates.rs` asks of every gate, with its reason — the save path interprets no page and
decodes no image — and `tools/state.sh save` runs it.

**The line, for the owner to add to `doc/todo/02` §2** — the instruction documents are not this
round's to edit:

```sh
tools/bounded.sh -- cargo test --profile gates -p pdf-model --test save_round_trip -- --ignored --nocapture   # §7.5.6's update over the corpus, read back by this tree, poppler and mupdf; a walk, ~12 s
```

It belongs after `xmp` and before `jpeg2000`, where `state.sh` already runs it. The argument for
its being a gate at all is `CLAUDE.md`'s: §7.5.6's incremental update is the one form of writing
the project places this tree to get right, it is the path every annotation a person adds goes
through, and for four hundred and ninety-one sessions its only independent judge was an
instrument nobody ran.

## 3. The population question: what else measures itself against a denominator it produces

Three instances met this round, one of them new to the tree.

- **The salvage census measures the lexer through the lexer.** Its denominator is what
  `Lexer::salvaged` records, so a run the lexer misfiles as a clean number or a keyword is outside
  it, and it walks page content streams only. That is why the digest — first pages, every object
  the interpreter reaches, a different denominator — saw two documents the census could not, and
  why the bisection was needed to read the digest. A census whose predicate is the thing being
  checked is trap 8's third shape; what makes it usable is a second instrument with a different
  denominator beside it.
- **The save round-trip's denominator is "every corpus document that opens", decided by the
  reader under test** — which is why its refusals were printed by reason from the first run and
  are now held by name: a reader that stopped opening a document would appear as a name joining
  `REFUSED_OPEN`, and the arithmetic 974 = 2 + 6 + 935 + 7 + 24 is checkable off the printed
  lists.
- **`doc/todo/02` §2 and `tools/state.sh` are two hand-written copies of the gate sequence**, and
  the document's claim that the script "runs the same sequence" was checked by nothing. Measured
  by hand this round they agree line for line — and then disagree by exactly the line this round
  added to the script ahead of its §2 line. `tools/conformance/tests/state_sections.rs` derives
  both lists from the two files and fails on a §2 line the script does not run; a script section
  §2 does not list is printed, because an instrument may honestly run ahead of the sequence while
  its numbers are watched, and that is the state this round leaves. Calibrated by removing a
  section and watching it fail by name.

The general form, for `doc/traps/instruments-and-reports.md` if a later round takes it there:
**an instrument whose population is produced by the code under test is answerable only beside a
second instrument whose population is not** — the digest beside the census, the references beside
the readback, the document beside the script.

## 4. Gates

`pdf-syntax` moved, so the whole of `doc/todo/02` §2 ran; `doc/history/990` has the figures as
printed. The workspace lint line was red on two neighbours' files (`content/transparency.rs`,
`shading.rs`) at the time of this round's first run and green on this round's own files; a
build error in a crate this round may not touch is a neighbour mid-edit (ADR 0992), and the
history file says which lines were re-run and when.
