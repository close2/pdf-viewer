# 0749 — Two delimiters that only delimit inside a type 4 program, and a ranking that ran out a second time

Status: accepted.
Context: the errata selection rule's seventeenth use — the eighth consecutive use whose base count
reproduces the previous use's closing arithmetic, and the third run under step 5.

**0750 to 0753 are sibling rounds'.** This number was taken one above the tip on that reservation.

## The rule, unchanged, and what its third run under step 5 says

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step, ADR
0691's writing rule, ADR 0712's placement rule, ADR 0732's family guard and ADR 0743's fifth step.
Nothing in it is amended here. What is now known is what a third outing could tell and a second
could not.

**Step 5 went flat one use after it was adopted.** Both row rankings top out at two annotations
again — six rows tied there over the live rows, 30 over every row with 17 more at one — and the
ranking by *issue*, which gave the sixteenth use a head of four against a floor of one, tops out at
**two with 33 issues tied there** and 23 at one. The reason is not that the unit was a bad idea. It
is that this rule decays fastest exactly where it is read: the sixteenth use read both issues at
four annotations and both at three, and those four were the whole of the shape the issue unit had.

**So the answer is not a third unit.** What chose the reading here is ADR 0653's tie-break,
unchanged — *a cell ahead of a word in prose*, and among cells the requirement level first — which
has now settled ties of 39 rows, of two issues and of 33 issues. Written plainly: **the rankings are
a filter and the tie-break is the selector.** A step 6 that ranked by some third cardinal would be
answering a question the last three uses have not been failing at.

**The base population.** Of the 302 issue numbers carrying a strike or a caret under the recipe's
own single-issue line parse, **56 were named nowhere at this round's base** — the sixteenth use's 60
less its four verdicts, reproduced by the greps rather than quoted from the record. The multi-issue
parse counts 310 and 58, and the eight numbers that appear only as the second number of a two-issue
line are the same eight. The parse the sixteenth use wrote into the recipe needed no re-derivation,
which is what writing it down was for.

**Of the 56, 39 touch only a settled row, 10 touch a live one and seven land on no row at all**,
against 42, 11 and seven one use ago — a plain subtraction of the sixteenth use's four verdicts, and
the first of these three uses where it is one. The ratio step 4's preference rests on has not moved.

## The head: Issue #139, which cites and re-prices a guard

Two bare carets — ADR 0691's fourth blindness, since `check` compares the tree's quotations against
*struck* text and nothing here is struck — insert into Table 151's `/Prev` and `/Next` cells the
words *shall not be present on the first item at each level;* and *shall not be present on the last
item at each level;*, beside the requirement each already stated. `doc/errata-read.md` has both
rectangles against `pdftotext -bbox`.

**Nothing in the tree moves and one thing in it is re-priced.** `outline.rs` walks `/First` and
`/Next` alone — `/Prev`, `/Last` and `/Parent` are redundant with what the walk already has — and
stops where `/Next` is absent, which the published cell made merely the likely case and the amended
cell makes a prohibition. What changes is the standing of the walk's refusal to visit an object
twice: under the published text a producer was free to close the list into a ring and the guard was
this reader's tolerance; under the amended text the ring is non-conforming and the guard is a repair
of a file the standard says shall not exist. That is §8.4.3.5's miter floor exactly — a clause that
*says* a thing being a stronger warrant than one that implies it (`CLAUDE.md` principle 5).

## The payment: Issue #365, and a delimiter set two characters too wide

§7.2.3's sentence introducing Table 2 has always scoped two of its ten rows:

> The delimiter characters { and } (LEFT CURLY BRACE (7Bh) and RIGHT CURLY BRACE (7Dh)) are
> additional delimiter characters within Type 4 PostScript calculator functions (see 7.10.5 "Type 4
> (PostScript calculator) functions").

Table 2 then lists all ten with no qualification on any row. Issue #365 puts the qualification into
the table, as a footnote on the `{` and `}` rows with the word *only* in it. **The published text
and the amended text therefore agree**, and what the erratum removes is the reading that an
unqualified table invites — which is the reading this tree had.

`pdf_syntax::lexer::class::is_delimiter` held all ten unconditionally for the whole of its life, and
§7.2.3's ledger row said so in as many words. Outside a type 4 program `{` and `}` are **regular**
characters, so the two readings disagree about a token boundary: `/A{B}` is one name of four bytes
under the clause and was a two-byte name followed by three tokens here — and §7.3.5 makes names
distinct by their bytes, so that is a dictionary key found or not found rather than a cosmetic
difference.

**The fix costs nothing, which is the argument for making it rather than documenting a divergence.**
The one place in a document where the braces do delimit is a type 4 program, and
`pdf_model::function::compile_postscript` tokenises one **itself** — it spaces the braces apart and
splits on white space, and has never used this lexer. So the predicate becomes the eight general
delimiters, `Token::BraceOpen` and `Token::BraceClose` go with it as unconstructible, and no
tokenisation of a type 4 program changes at all.

### The writer keeps escaping both, and that is a choice

§7.3.5 rule b) — "Any character in a name that is a regular character (other than NUMBER SIGN) shall
be written as itself or by using its 2-digit hexadecimal code, preceded by the NUMBER SIGN" — leaves
a writer both spellings for a regular character. `Name::escaped` writes `#7B` and `#7D`.

- **What it buys**: a name this program writes into an incremental update is read as the same name
  by a reader still taking Table 2's ten delimiters unconditionally, which is every reader that has
  not read this footnote — including this one, until this round.
- **What it costs**: two bytes in a name almost no document has.
- **Why it is not a repair**: the raw spelling is legal and this tree's own reader answers the same
  name for it. `a_brace_in_a_name_is_written_as_its_hexadecimal_code_by_choice` asserts both halves,
  because a choice that is not reversible is a repair wearing a choice's words.

### Calibration

Trap 13, above the commit that makes the change and in both directions. With the two braces put back
into `is_delimiter`, the lexer's test fails with `left: [Name([65]), Keyword([66]), Integer(1)]`
against `right: [Name([65, 123, 66, 125]), Integer(1)]`. With the exception taken out of
`Name::escaped`, the writer's test fails with `left: "curly{braces}"` against
`right: "curly#7Bbraces#7D"`.

### Two earlier records count the old set, and they stay

ADR 0370 argues the `REGULAR` table from what the predicates compile to and says "the ten delimiters
run from 37 to 125"; ADR 0453 lists what `Name::escaped` sends out as `#xx` and says "the nine
delimiters". Both are records of a decision made at the time and neither is edited (ADR 0232 §2) —
the first counted the set the tree then had, and the second miscounted it, since the predicate it
described listed ten. The live comments are corrected: `class::is_delimiter` now says eight and names
the condition, `Name::escaped` says eight and names the two it escapes by choice, and the `REGULAR`
table's own comment says in the past tense what the delimiter range was when ADR 0370 measured it,
because dropping the braces narrows it from 37–125 to 37–93 and the table stays on the measurement
that exists rather than on an argument about masks.

## Two things about the instruments

- **`emit` filed the erratum one clause late**, under §7.2.4 *Comments*, because page 38 opens that
  heading and ADR 0712's rule attributes an annotation by its page's outline section. Table 2 is
  §7.2.3's. Known shape; the recipe already calls the bucket a sort order.
- **A caret's contents can be a cross-reference rather than replacement text**, which had not
  happened before in seventeen uses. Both of #365's say only *See new table footnote (a)*; the
  footnote is a third annotation, a `Text` under the same `/Subj` on the same page. The issue ranks
  correctly, since it carries carets — but the amendment is not in the ranked annotations, and an
  erratum whose *whole* substance were a `Text` annotation would be dropped by step 3's "neither a
  StrikeOut nor a Caret" filter. The rule that follows is cheap: read the `Text` lines `emit` prints
  beside a ranked pair before believing the pair is the amendment.

## A correction to `doc/errata-read.md`'s own *Owed* list

Its last bullet named two populations "nothing reads at all": a quotation in a Markdown file under
`doc/`, and a quotation of a table cell rather than of prose. **Both are read.** `--bin quotations`
names the first in its own opening sentence, sweeps it, and prints the count of documents it read at
the end of every run; the second is read by construction, because `conformance::prose::Conversion`
joins all fourteen conversions into one body of normalised text in which `doc/md/`'s tables are
Markdown rows, so a quotation lying inside one cell is looked for exactly as a quotation of prose is.
What remains outside the comparison is a quotation that *spans* two cells, since the `|` between them
survives the fold.

Two rounds before this one found the bullet stale and neither corrected it, each judging it another
round's file. It is this round's, and the general point is the one `CLAUDE.md` makes about a ledger
row: **a claim about the instruments decays exactly like a claim about the tree, and nothing sweeps
the file that holds this one.**

## What this round did not take

Issue #700's renumbering of Annex O's two tables — the sixteenth use's finding — stands on 75 lines
across 27 files, and that repair is a sibling round's. Nothing here touches those lines.
