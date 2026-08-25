# 0660 — The quotation that hid by being wrong

Status: accepted.
Context: the successor selection rule's fourth use, on §7.7.4 — and what the rename at its head
turned out to reach.

## The rule, run

ADR 0627's rule, with ADR 0637's repair to its second step and ADR 0653's tie-break:

> Rank each live ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Reassemble the issue from every clause `emit` files it under, and read the issue
> whole. Where rows tie, read the one whose errata strike a cell ahead of the one whose errata
> substitute a word in prose.

§12.10.2 is gone from the ranking, which is the decay working for the second use running. What is
left at the head is the rest of the plateau ADR 0653 described: **§7.7.4 and §14.8.5.3, seven
annotations apiece**, the two rows that lost the last tie-break. §7.7.4 wins the rerun of the same
rule — #672 appends *; deprecated in PDF 2.0* to two of Table 32's cells, where §14.8.5.3's four
carets swap *version* for *level* in the name of a referenced CSS specification.

(ADR 0653 wrote "five of Table 31's rows" for #214's strikes; the table is **32**, the name
dictionary's, and page 124's `-bbox` places all five. Recorded here rather than edited there.)

## What the two issues turned out to be

`doc/errata-read.md` has all five with the rectangle that places each.

- **#672 deprecates the catalogue's whole Web Capture surface with one hand.** Three bare `Caret`s,
  each landing on the `1.3)` at the end of an `(Optional; PDF 1.3)` cell: Table 32's `/IDS` and
  `/URLS`, and Table 29's `/SpiderInfo` one clause up. §7.7.4's row had `/IDS` and `/URLS` as two of
  six trees *owed to a feature*; a deprecated entry is a different kind of silence, and it is the
  reason the same row already gives `/AlternatePresentations`. Reassembling the issue across its two
  clause headings is what turned two cells into three.
- **#214 is the standard's rename, not a table's.** One `Text` note on page 10 — *all occurrences of
  the term "name string" are replaced by just "string" throughout ISO 32000-2:2020* — with ten
  illustrative strikes on Table 32 and two more, four clauses along, on §7.9.6. The term is not one
  §7.9.2 defines: that clause states a text string, a PDFDocEncoded string and a byte string. So the
  erratum withdraws a type ISO 32000-2 never had, and nothing in this tree behaves differently under
  it.

## Decision 1: a quotation is corrected against the clause even when no erratum struck it

Following #214 to §7.9.6 found `pdf_syntax::tree::name_pairs` citing that clause for the phrase
**"by unsigned character code"**, which appears in no clause of ISO 32000-2, in no annex, and in none
of the technical specifications under `doc/md/`. It was standing in for the ordering rule, on the
sentence the erratum amends.

**No instrument in this project was placed to reach it**, and the three misses are worth naming
because each is a different reason:

- `spec-errata check` compares a quotation against text an erratum **struck**. Nothing is struck
  under an invented phrase, so there is nothing for it to match.
- `--bin quotations` reads every Markdown document under `doc/` and `ledger.toml`'s notes. It does
  not read `crates/`.
- The conformance gate `every_quotation_is_the_standards_own_words` verifies rustdoc **blockquotes**.
  This was a quotation inside a sentence of prose, which `CLAUDE.md` permits and nothing checks.

The sentence now carries §7.9.6's own words, and `pdf_model::named_page`'s "which §7.9.6 makes
lexical by key" — the one word #214 takes out of that clause — is prose about bytes.

## Decision 2: a misquotation is invisible to the instrument built to catch misquoting

`viewer_core::command`'s `Command::Extract` put *the tree shall map name strings to file
specifications* in quotation marks against §7.11.4.1. The clause opens that sentence with *the
associated name tree*, so the quoted words were never the standard's — **and because they were not,
`spec-errata check`'s comparison matched nothing**. The same sentence was struck out by Issue #481
with the two bullets around it; `pdf_model::attachment` was corrected for it in the
four-hundred-and-eighteenth session and `viewer_host::panel` in the four-hundred-and-twenty-ninth,
each because `check` landed on their *accurate* quotations. This third copy outlived both by being
wrong.

That is a general argument for principle 5's rule rather than a fact about one comment: **the cost of
a paraphrase inside quotation marks is not only that it is wrong, but that it stops being
checkable.** A verbatim quotation of retired text is found by the tool the first time it runs. A
paraphrase of retired text is found by a person, four hundred sessions later, or not at all.

## Decision 3: #307 is a requirement, and the test asserts the reader's half of it

Issue #307 is two `Caret`s with nothing struck beneath them, adding *Keys shall not be the null
object.* to §7.9.6's Table 36 `/Names` row and to §7.9.7's Table 37 `/Nums` row. It is the fifth
caret-with-no-strikeout this collection has yielded, and `check` cannot see one by construction.

It is addressed to whoever writes the file, so what a reader owes is not a refusal but a defined
behaviour on a file that breaks it — and the behaviour that matters is not "the null key answers
nothing" but **"every pair after it still holds the value the file put beside it"**. Both walks in
`tree.rs` chunk the pairs array in twos, so a null in a key position costs exactly its own pair; a
reader that dropped the null *element* before pairing would re-pair the whole remainder against
itself and hand back the wrong value for every key past the fault. That plant is the calibration
(trap 13), and the second plant — a null key admitted as an empty key — fails the other assertion.
`a_null_key_yields_nothing_and_leaves_its_neighbours_paired` asserts both trees, because the erratum
states the sentence twice.

Nothing is *reported*. A `shall not` addressed to a writer is a malformed file to a reader, and this
module has no report channel of its own; the population is unmeasured and a report on an unmeasured
condition is trap 11.

## Decision 4: the rule ranks what is owed, so a round runs it twice

Step 3 keeps rows whose status is live. That is what makes it a ranking of *debt* — and an erratum's
whole point is that it can add a requirement to a clause this tree calls complete, which is the
falsest row the ledger can hold. #307 landed on two `implemented` rows and the ranking could not see
either.

Measured at this base with `implemented` admitted, the top of the list is not the top the rule
prints: **§9.6.4 carries 11 unread annotations under four issues and §7.4.1 carries 8**, both above
the live head's seven, with §7.9.2.4, §7.5.4, §7.6.4.4.3, §7.10.5.3, §12.5.6.1 and §12.10.4 also in
the top twenty. `doc/todo/01`'s recipe gains a fourth step: rank twice, and say which list the row
came from. This round took its row from the live list and met the other list's shape by accident,
which is exactly the evidence for adding the step rather than for replacing the rule.

## Decision 5: an erratum read only far enough to rank it stays in the population

The reason §7.7.4 was still at the head is that ADR 0653 read #214 and #672 far enough to break a tie
and recorded them as bolded bare numbers — `**#214**` — in an ADR and in `doc/todo/01`. **Neither of
step 2's greps can see that form**: the first wants the `Issue #` prefix, and the second reads
`doc/errata-read.md` alone. ADR 0637 found the same failure in a different costume and repaired it
with a second grep; a third grep is not the repair here, because the bare-number search over the tree
collides with `doc/HAYRO_ISSUES.md`'s other-tracker numbers, which is exactly why step 2 is two greps
and not one.

So the repair is a rule about writing, and it has two halves that must both hold:

- **An erratum read to a verdict is recorded in `doc/errata-read.md`**, in the table whose column
  both greps see.
- **An erratum read only far enough to rank a row is deliberately left in the population**, and the
  round says so. Recording it would take unread ground off the list, which is worse than re-offering
  it.

Neither half is new policy so much as the two halves being stated together for the first time: this
round is the second in a row whose head was decided by where a previous round wrote a number down.
