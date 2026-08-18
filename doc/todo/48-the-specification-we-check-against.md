# The copy of the standard this project checks itself against is lossy

Status: **census run, step 2 built, all 120 known passages read, and items 1 to 3 closed in the
five-hundred-and-fortieth (ADR 0375)** — sessions 416 (ADR 0252), 417 (ADR 0253) and 418 (ADR
0254). The census answered the question the item turned on and answered it the bad way: the
annotations are the **errata**, and `doc/md/` presents struck-out passages as the standard's
current text. Reading all of them found **four clauses this tree implemented differently** —
§12.5.2's `/BM`, §14.13.5's `/MCAF`, §7.8.3's Type 3 glyph resources and §8.9.5.4, the last of
which is implemented as the erratum states it since the five-hundred-and-fortieth.
**What is left is steps 3b, 4 and 5 below** — and 3b is new rather than old: the
five-hundred-and-ninety-first repaired the comparison and 27 more struck passages became visible
with it.
Priority: 48 — kept, by the convention that the `40`–`49` band is *the project's own instruments*.
**Its real weight is higher than the number**: this is a list of known-wrong passages in the file
principle 5 rests on.
Corpus: —, the subject is `doc/md/` and the fourteen documents under `doc/`
Code: `tools/spec-errata` (built), `conformance::prose` and its `quotations` binary (the sixth
population, built in the four-hundred-and-seventy-fourth),
`crates/pdf-model/examples/spec_annotation_census.rs` (the census)
Read: `doc/errata-read.md` — all 120, their verdicts, and what is owed

## What the census found, and what it corrected in this file

The table below **replaces this file's original premise, which was wrong by a factor of thirteen**:
it recorded "882 `/Annots`" in ISO 32000-2 and called them annotations. There are 882 `/Annots`
*arrays* and **11 462 annotations** in them. The item's own rule — census before building — is what
caught it.

| | annotations | what they are |
|---|---|---|
| `ISO_32000-2_sponsored_EC3.pdf` | **11 462** | Link 6031, Popup 2553, **Text 1876, Caret 522, StrikeOut 435**, Widget 38, FileAttachment 6, Highlight 1 |
| `ISO_TS_32002-2022_sponsored_EC3.pdf` | 75 | Link 62, Text 8, StrikeOut 5 |
| `ISO_TS_32001-2022_sponsored_EC3.pdf` | 69 | Link 48, Popup 9, Text 8, StrikeOut 3, Caret 1 |
| the other eleven | 6 to 210 each | **Link, and nothing else** |

The three `_EC3` and `-2022` files record their errata as review markup and apply them to nothing:
**360 distinct `Issue #NNN` subjects**, **434 strikeouts over 4038 words** across **252 sections**,
each with a `Caret` carrying the replacement, and 1752 §12.5.6.4 state annotations whose `/State`
says how far the change got. The body text underneath is unamended.

`/StructTreeRoot` in all fourteen, `/MarkInfo /Marked` in twelve. **This paragraph used to end with
a bound and the bound is gone**: `Tree::walk` stopped at 65 536 items, so the 71 371 recorded here as
the size of ISO 32000-2's structure tree was the *bound overshooting* rather than the tree, which is
**129 389**. Session 421 raised it to 2²⁰, made it report through `Reading::truncated`, and found the
walk quadratic besides — 16.8 s to **151 ms** (ADR 0257). So `logical_order` and everything on it now
see the whole document, and a substrate built on this need not go page by page; 1021 of its 1023
pages state `/StructParents`, so `ParentTree::for_page` remains the cheaper route where only one
page is wanted.

## What is built

`tools/spec-errata`, with `census`, `emit`, `check`, `moved` and — since the
five-hundred-and-ninety-first — `applied`, which asks whether a place that *records* an erratum has
applied it (ADR 0426, `doc/todo/01`'s seventeenth sweep). Seven seconds over all fourteen documents.
Not a gate and not a test, and `conformance` does not know it exists — the gate must keep checking
quotations against a conversion this project did not make, or a defect in our extractor becomes a
defect in the standard. ADR 0252 has the argument.

`check` reports two things, and **neither list contains the other**:

- **151 struck passages of four words or more that `doc/md/` still carries as current text**, 120
  of them distinct, over 92 sections. All 120 are read, in `doc/errata-read.md`. The comparison
  drops every space before comparing, because both sides are extractions of the same glyphs by
  different programs (ADR 0253); it took the first number from 79 to 151. **One class of false
  positive is known and cannot be removed**: where the standard prints a sentence twice and the
  erratum deletes one copy, the survivor is current text and this reports it as retired — §7.5.4's
  Issue #113 is the witness.
- **Quotations in this tree that overlap struck text.** Three populations since the
  four-hundred-and-eighteenth session, because only one of them has a gate: rustdoc **blockquotes**,
  which `cargo test -p conformance` verifies; rustdoc **prose**, a pair of quotation marks inside a
  doc comment, which nothing reads; and **`ledger.toml` notes**, ADR 0249's spans, which
  `--bin quotations` reads against the standard since the five-hundred-and-fortieth and which
  nothing gates. 25 land in the clause they cite and every one is now a *correction* quoting the
  wording it retired; 51 more match a phrase struck out of another clause and all of them have been
  looked at. **A fifth and a sixth were added later** — a `"` in an ordinary `//` comment (ADR 0255)
  and, in the four-hundred-and-seventy-fourth, every quotation in this project's own Markdown
  documents (`spec_errata::document_landings`, ADR 0309). The sixth's landings are almost all
  correct writing by construction, because `doc/errata-read.md`'s whole subject is the struck text;
  two were not, and both are corrected.

## What is still owed

1. ~~**§8.9.5.4**~~ — **implemented in the five-hundred-and-fortieth, and the reason it had been
   declined was wrong.** ADR 0253 said the amended step a) "reads as terminal and would leave the
   amended d) unreachable". It is terminal and d) is unreachable *for a hidden base image*, which is
   the amendment rather than a defect in it: a) and b) dispose of every base image that states an
   `/OC`, and c) and d) open at "Otherwise", so they belong to one that states none. `doc/md/` was
   checked against the PDF here and is faithful — the conversion is not the problem in this clause,
   the tree's reading of the carets was. ADR 0375.
2. ~~**§14.8.6.3's enclosure requirement**~~ — **read and declined by argument.** The amended
   sentence opens "[w]hen including mathematics structured as MathML", so the enclosure under
   `Formula` and the namespace on every MathML type and attribute are `shall`s on a producer, which
   `CLAUDE.md`'s closed exclusion covers; the reader's half is done and what stays owed is a
   validator's report. `structure::Namespace::is_standard` got its caller anyway and not the one
   this entry predicted: §14.8.6.2's rule decides whether a type *name* is §14.8.4's word or a
   foreign vocabulary's homonym, so `Tree::standard_role` refuses a name that ends outside a
   standard structure namespace. **And the clause carried the round's real finding**, which is the
   next entry's: `doc/md/` writes its namespace name in single quotes with spaces inside them where
   the PDF sets one double quotation mark.
3. ~~**The ledger's single-quoted spans.**~~ **Read since the five-hundred-and-fortieth**, and there
   are 106 of them. `conformance::quote::quoted_spans` is the shared rule — an opening `'` needs a
   space or a bracket before it, a closing one needs a space or ordinary punctuation after it, and a
   double quotation mark ends the search — and the ledger's notes are a population of
   `--bin quotations` rather than a script somebody retypes. Its first committed run found three
   misquotations. ADR 0375.
3a. ~~**Quotations of the standard in Markdown**~~ — **built and run, and the induction held.**
   `conformance::prose` and `cargo run --release -p conformance --bin quotations` read every
   Markdown document this project wrote under `doc/` against all fourteen specifications, with ADR
   0249's discriminator and the standard's own continuation printed under each divergence. Thirteen
   corrections on the first run, three of them sentences ISO 32000-2 does not contain, two of them
   spread across four files and two files where the gate could not see them, and one a wrong *table*
   number. Two of the suspects turned out to be `doc/md/` truncating a row and shifting a table's
   columns — the caveat above, now with witnesses. ADR 0309. **A gate is still refused for ADR
   0249's reason** and the price has gone up: the syntax it needs would have to be migrated onto
   1401 spans rather than 417.
3b. **The 27 struck passages the repaired comparison made visible**, new in the
   five-hundred-and-ninety-first (ADR 0426). `squeezed` kept square brackets and dash shapes, so it
   could not find a passage quoted in `CLAUDE.md`'s own `"[e]ncloses"` spelling of an altered first
   letter, nor one carrying a table caption `doc/md/` writes with a hyphen where the standard sets
   an em dash. It is `conformance::prose::folded` now, and `check`'s struck-passage list went from
   151 lines to **178**. Three of the new ones are in Annex A and four in clause 13, both outside
   scope; **the other twenty are unread**, and `doc/errata-read.md` names their clauses. This is the
   same reading task items 1 to 3 were, at the rates that file records, and it is the one thing this
   item gained rather than closed.
4. **The disagreement sweep** (the old step 3), unchanged: compare our extraction against `doc/md/`
   and report every span where they differ. The annotations were one loss of four; three others are
   recorded in this file's history and none has been swept for.
5. **Only then** the substrate question (the old step 4) — and **the cost is measured now rather than
   feared**, by `tools/pdf-retrieve/examples/substitution_cost.rs`, which asks the gate's own two
   questions of both substrates (session 421, ADR 0257). **Clause existence is free**: all 506
   distinct clauses this tree cites are among the 946 numbered items of §12.3.3's outline, resolved
   in 23 ms with nothing interpreted, against `doc/md/`'s 1034 headings. **The quotation half is 29
   readings**: of 582 blockquotes, 40 are found in this reader's extraction by the gate's own
   comparison, 523 with the spaces taken out and **553** with the dashes folded together as well,
   leaving 29. The gap is typography rather than words — `doc/md/` writes `Table 87 -Additional
   entries` where the standard prints `Table 87 — Additional entries`, so 59 of this tree's
   quotations carry the converter's dash. **Still a separate round from the API**, for the reason
   session 413's decision gives.

## What would make this item wrong

It was written with the closing condition "if the annotations are links and bookmarks, steps 2 to 4
buy nothing". Eleven of the fourteen documents met it and the three that matter did not, so the
condition was spent, and session 416 replaced it: **if reading the remaining passages turns up no
clause this tree implements differently, the errata are a documentation concern and not a
correctness one, and this item drops into the notes.**

**That test has been run three times and the item passes it every time.** Three clauses were
implemented differently — §12.5.2's `/BM`, §14.13.5's `/MCAF` and §7.8.3's Type 3 glyph resources —
and a fourth, §8.9.5.4, still is. So the item stays where it is. **The four-hundred-and-nineteenth
ran it a fourth time and it passed again**, this time from the other direction: reading §7.8.3 for
an unrelated clause found `content.rs` quoting the struck fourth bullet, which found two holes in
the instrument — a `"` inside an ordinary `//` comment and a quotation with an ellipsis in it — and
six more stale quotations behind them (ADR 0255). One of the six, §8.9.7's NOTE 3, is a clause whose
*code* was already right and whose comment was two years behind it.

**The four-hundred-and-seventy-fourth ran it a fifth time, over the population item 3a named, and it
passed again** — three sentences quoted as ISO 32000-2's that ISO 32000-2 does not contain, two of
them also standing in `crates/` where the gate exists and could not see them because neither was a
blockquote. ADR 0309. It also produced the first *evidence* for this file's own warning about the
conversion rather than more advice: two suspects were `doc/md/` truncating Table 29's `/OpenAction`
row and shifting Table 179's columns, both acquitted by `pdftotext -layout` over the PDF.

**The five-hundred-and-fortieth ran it a sixth time, over items 1 to 3, and it passed again** — the
last clause this tree knowingly implemented a retired version of is implemented as the erratum
states it, and the reason it had been declined turned out to be a misreading of the amended steps'
own ordering rather than a defect in them. ADR 0375.

The replacement condition, for whoever picks it up: **the reading half is done for the passages the
instrument could see, the correctness question it was asked to answer is answered, and items 1 to 3
are closed.** What is left is step 3b — twenty passages the five-hundred-and-ninety-first's repair
of the comparison made visible, which is reading of exactly the kind items 1 to 3 were — and steps 4
and 5, the disagreement sweep and the substrate question, which this file has carried since it was
written and which are *work* rather than documentation debt. The item stays where it is with that
scope and no other. If a round closes step 4 and it turns up nothing, what remains is one
API question and this file should become a paragraph in `doc/todo/01` rather than a file. The rates
are worth carrying: 66 passages gave two findings, the next 54 gave one, and the *quotation* sweep —
a different question over the same errata — gave nine in one round, six in the next, three in the
one that read the Markdown documents and three more in the one that read the ledger's notes, over
three populations nobody had counted.
