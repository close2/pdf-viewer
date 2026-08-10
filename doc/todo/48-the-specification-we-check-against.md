# The copy of the standard this project checks itself against is lossy

Status: **census run, step 2 built, all 120 known passages read, the ledger's 977 spans swept** —
sessions 416 (ADR 0252), 417 (ADR 0253) and 418 (ADR 0254). The census answered the question the
item turned on and answered it the bad way: the annotations are the **errata**, and `doc/md/`
presents struck-out passages as the standard's current text. Reading all of them found **three
clauses this tree implemented differently** — §12.5.2's `/BM`, §14.13.5's `/MCAF`, and §7.8.3's
Type 3 glyph resources — and one it still does, §8.9.5.4.
Priority: 48 — kept, by the convention that the `40`–`49` band is *the project's own instruments*.
**Its real weight is higher than the number**: this is a list of known-wrong passages in the file
principle 5 rests on.
Corpus: —, the subject is `doc/md/` and the fourteen documents under `doc/`
Code: `tools/spec-errata` (built), `crates/pdf-model/examples/spec_annotation_census.rs` (the census)
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

`tools/spec-errata`, with `census`, `emit` and `check`. Seven seconds over all fourteen documents.
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
  doc comment, which nothing reads; and **`ledger.toml` notes**, ADR 0249's 977 spans, which nothing
  reads either. 25 land in the clause they cite and every one is now a *correction* quoting the
  wording it retired; 51 more match a phrase struck out of another clause and all of them have been
  looked at.

## What is still owed

1. **§8.9.5.4** — the one clause this tree knowingly implements a retired version of, declined for
   the reason ADR 0253 states: the amended step a) reads as terminal and would leave the amended d)
   unreachable, so a rewrite trades one contradiction for another. No corpus document states
   `/Alternates`.
2. **§14.8.6.3's enclosure requirement** — EC3 requires a `math` element under a `Formula` structure
   element and the namespace on every MathML type *and attribute*. `Tree::role` checks neither.
3. **The ledger's single-quoted spans.** `quoted_spans` collects `"` … `"` only, because an
   apostrophe would make every possessive an opening mark; §12.7.5.2.2's stale quotation was in
   single quotes and was found through the source rather than by the sweep.
3a. **Quotations of the standard in Markdown**, named in the four-hundred-and-nineteenth and
   uncounted: `doc/errata-read.md`, `doc/HANDOVER.md`, `doc/todo/` and the 255 ADRs quote the
   standard constantly and no instrument compares a word of it. It is named here rather than done
   because the reason to expect something in it is inductive — each of the five populations swept so
   far produced a finding on its first run — and because it is the largest of them.
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

The replacement condition, for whoever picks it up: **the reading half is done and the correctness
question it was asked to answer is answered.** What is left is items 1 to 3 above, which are two
named clauses and one known gap in a sweep, plus the two steps this file has carried since it was
written. If items 1 to 3 close and nothing new arrives, what remains is a documentation debt with a
known size and it belongs in the notes. The rates are worth carrying: 66 passages gave two findings,
the next 54 gave one, and the *quotation* sweep — a different question over the same errata — gave
nine in one round and six in the next, over two populations nobody had counted.
