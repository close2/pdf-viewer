# 819 — Two delimiters that only delimit inside one function, and a ranking that ran out a second time

Finding: **`pdf-syntax`'s lexer held all ten of Table 2's delimiter characters unconditionally, and
§7.2.3 makes two of them delimiters only inside a type 4 PostScript calculator function** — so
`/A{B}` lexed as a two-byte name and three tokens where the clause makes it one name of four, which
by §7.3.5 is a different object and therefore a dictionary key found or not found. The one place the
braces do delimit tokenises itself and never reaches that lexer. Beside it, and about the rule
rather than the standard: **step 5 went flat on its third outing**, one use after it was adopted.

Date: 2026-08-29. Argued in ADR 0749. (0750 to 0753 are sibling rounds'; this number was taken one
above the tip on that reservation.)

Touched: `crates/pdf-syntax/src/lexer.rs`, `crates/pdf-syntax/src/object.rs`,
`crates/pdf-syntax/src/parser.rs`, `crates/pdf-syntax/tests/name_escaping.rs`,
`crates/pdf-model/src/function.rs`, `doc/conformance/ledger.toml` (§7.2.3, §7.3.5, §7.10.5.2,
§12.3.3), `doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, `doc/adr/0749-…`.

## What the rule was asked and what it answered

The errata selection rule's seventeenth use. `spec-errata emit` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, step 2's two greps, step 3's attribution with the family guard,
step 4's two rankings, step 5's ranking by issue where they went flat.

The base population reproduces the sixteenth use's closing arithmetic exactly — **302 issue numbers
carry a strike or a caret under the recipe's own single-issue line parse and 56 were named
nowhere**, which is 815's 60 less its four verdicts; the multi-issue parse's 310 and 58 reproduce the
same way, over the same eight numbers that appear only as the second number of a two-issue line.
Eighth consecutive use, and the first that needed no re-derivation of the parse, which 815 wrote
into the recipe.

Of the 56, **39 touch only a settled row, 10 touch a live one and seven land on no row at all**,
against 42, 11 and seven at 815's base — a plain subtraction of its four verdicts, and the first of
three uses where it is one.

## Step 5 went flat too, and the tie-break is what has been choosing all along

Both row rankings top out at two annotations: six rows tied there over the live rows, 30 over every
row with 17 more at one. The ranking by **issue** — added at the fifteenth use because the row unit
had run out, and giving the sixteenth a head of four against a floor of one — tops out at **two,
with 33 issues tied there** and 23 at one.

That is not the unit failing; it is the rule decaying fastest where it is read. 815 read both issues
at four annotations and both at three, and those four were the whole of the shape the issue unit had.
**What chose this round's head is ADR 0653's tie-break** — a cell ahead of a word in prose, and among
cells the requirement level first — which has now settled ties of 39 rows, of two issues and of 33
issues. The rankings are a filter; the tie-break is the selector. No step 6 was added, and the ADR
says why one would answer a question the rule has not been failing at.

## The head cited and the walk down paid

**Issue #139 — §12.3.3, Table 151's `/Prev` and `/Next`.** Two bare carets add *shall not be present
on the first / last item at each level;* to cells that already required the entry for every other
item. `outline.rs` walks `/First` and `/Next` alone and stops where `/Next` is absent, so no line
moves — and its refusal to visit an object twice stops being this reader's tolerance for a ring a
producer was free to write and becomes the repair of a file the standard says shall not exist. The
shape §8.4.3.5's miter floor took.

**Issue #365 — §7.2.3, Table 2's `{` and `}` rows.** Two carets add a footnote reference to those
two rows and a `Text` annotation states the footnote: they are additional delimiter characters *only*
within type 4 PostScript calculator functions. The clause's own sentence introducing the table
already said as much without the word *only*; the tree, and §7.2.3's ledger row, took the table's
unqualified list. `class::is_delimiter` is the eight general delimiters now, `Token::BraceOpen` and
`Token::BraceClose` went with them as unconstructible, and `pdf_model::function::compile_postscript`
— which spaces the braces apart and splits on white space itself — is unaffected, being the one
place they do delimit.

`Name::escaped` still writes both as `#7B` and `#7D`, which §7.3.5 rule b) permits for a regular
character, so that a name this program writes is read alike by a reader still holding Table 2's ten.
A choice, with its cost written down, and pinned by a test that also asserts the raw spelling reads
back as the same name.

Calibrated per trap 13 above the commit that makes the change, both ways: the braces put back into
`is_delimiter` fails the lexer's test on the four-byte name, and the exception taken out of
`Name::escaped` fails the writer's test on the spelling.

## The two standing items

- **Taken.** `doc/errata-read.md`'s *Owed* list said nothing reads a quotation in a Markdown file
  under `doc/`, or a quotation of a table cell. `--bin quotations` reads the first population by
  name and prints the count of documents it read on every run, and the second is read by
  construction, since the conversions are joined into one body of text in which the standard's
  tables are Markdown rows. Only a quotation spanning two cells is still outside it. Two rounds had
  found the bullet stale and neither corrected it; it is this file's own.
- **Left.** Issue #700's 75 lines across 27 files standing on Annex O's retired table numbers are
  round 820's, by the briefing's own split. Nothing here touches them.

## A third thing the collection had not done in seventeen uses

**A caret can point instead of saying.** Both of #365's read only *See new table footnote (a)*, and
the amendment is in a `Text` annotation under the same `/Subj`. The issue ranks correctly because it
carries carets — and an erratum whose whole substance were a `Text` annotation would be dropped by
step 3's "neither a StrikeOut nor a Caret" filter. `emit` prints the `Text` lines beside the ranked
pair, so the reading costs nothing; the assumption it costs is that a ranked annotation carries the
amendment.

## Gates and sweeps

The full §2 sequence — a change in `pdf-syntax` is under everything by the change→gate map, whatever
the diff looked like. §4's sweeps before and after, against a pristine checkout at the base commit
with its own build directory, closed with it afterwards.
