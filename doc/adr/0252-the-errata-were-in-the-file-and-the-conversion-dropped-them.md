# ADR 0252 — The errata were in the file all along, and the conversion dropped them

Status: accepted, 2026-08-09 (session 416).

## Context

`CLAUDE.md` principle 5 makes ISO 32000-2 the only source of truth, and this project reads it
through `doc/md/` — a Markdown conversion of the fourteen PDFs under `doc/`, made by a converter
nobody here wrote. The conformance gate verifies **6051 citations and 577 quotations** against that
conversion, which makes it the instrument the whole principle rests on.

**The conversion ignored annotations.** `doc/todo/48` proposed a census before anything was built,
on the argument that the answer decides the item: 882 link annotations would be worth nothing, and
882 review comments would change how every clause in the tree is read.

## What the census found

`crates/pdf-model/examples/spec_annotation_census.rs`, over all fourteen documents, every page —
not a prefix, because an erratum on page 900 is the case it exists to find:

| | pages | annotations | subtypes |
|---|---|---|---|
| `ISO_32000-2_sponsored_EC3.pdf` | 1023 | **11 462** | Link 6031, Popup 2553, Text 1876, Caret 522, StrikeOut 435, Widget 38, FileAttachment 6, Highlight 1 |
| `ISO_TS_32002-2022_sponsored_EC3.pdf` | 14 | 75 | Link 62, Text 8, StrikeOut 5 |
| `ISO_TS_32001-2022_sponsored_EC3.pdf` | 14 | 69 | Link 48, Popup 9, Text 8, StrikeOut 3, Caret 1 |
| the other eleven | 5–72 | 6–210 each | **Link, and nothing else** |
| **total** | 1382 | **12 545** | Link 7080, Popup 2562, Text 1892, Caret 523, StrikeOut 443, Widget 38, FileAttachment 6, Highlight 1 |

**The premise's own number was wrong and low by a factor of thirteen.** `doc/todo/48` recorded 882
`/Annots` in ISO 32000-2; there are 11 462 annotations in 882 arrays. The item was written from a
count of pages that state the entry.

Eleven of the fourteen carry links and nothing else, which is the outcome the todo file said would
close the item. The other three answer the question the other way, and the answer is worse than the
proposal guessed:

- **360 distinct `/Subj` values in ISO 32000-2, of the form `Issue #NNN`**, plus `EDITOR NOTE`.
- **434 strikeouts covering 4038 words of the standard's own text**, across **252 sections**.
- Each correction is a group in §12.5.6.2's sense: a `StrikeOut` over the retired words with
  `/IT /StrikeOutTextEdit`, a `Caret` with `/IT /Replace` whose `/Contents` is the replacement, and
  `/RT /Group` tying them together.
- **1752 of the notes are replies**, which is §12.5.6.4's state mechanism: "[t]he state is not
  specified in the annotation itself but in a separate text annotation that refers to the original
  annotation by means of its IRT ("in reply to") entry". Their `/State` is Table 174's, and the
  errata this round acted on all carry `Completed` — "[t]he change has been completed".

**The body text under the strikeouts is the unamended 2020 text.** So `doc/md/` presents retired
sentences as the standard's current words, with nothing marking them, and the gate verifies a
quotation of one without complaint. That is the failure principle 5 exists to prevent, and it had
already happened — see below.

The proposal's second argument, that a tagged document needs no layout inference, holds for
thirteen of the fourteen and **fails on the one that matters**. All fourteen state a
`/StructTreeRoot`; twelve state `/MarkInfo /Marked true`. But `structure::Tree::walk` is bounded at
`MAX_CHILDREN = 65 536`, and ISO 32000-2's tree hits it — 71 371 items and still going, so
`logical_order`, `logical_text` and `logical_range` see the front of the document and stop. The
per-page route does not have that problem: 1021 of its 1023 pages state `/StructParents`, which is
what `ParentTree::for_page` reads. A future substrate would have to go page by page, and the
handover's claim that the structure tree "gives reading order outright" is true only in that form.

## Decision

**The item does not close. Step 2 is built, steps 3 and 4 are not, and nothing this project
generates goes anywhere near the gate.**

`tools/spec-errata` is a sidecar with three commands and no test, no gate and no consumer:

- `census` — the counts above, per document, which are facts about the documents and may be written
  down.
- `emit` — the notes as Markdown, keyed to page and to the section §12.3.3's outline puts that page
  in, which in these documents *is* the clause number. Its output is the specification's text and
  goes under `.gitignore` beside `doc/md/`, by ADR 0187's rule and for ADR 0187's reason.
- `check` — the two questions the census raised, asked of this tree.

**The dependency runs one way and that is the whole design.** `spec-errata` uses
`conformance::quote::normalise` so that "does this doc comment quote retired text" is asked with
the gate's own comparison; `conformance` does not know this crate exists and must not. If the
checker read a conversion this project generated, a defect in our extractor would become a defect
in the standard we check ourselves against, and a wrong quotation would verify against our own
error. `pdftotext` plays the independent-second-opinion role in `tests/text_extraction.rs`; the
Markdown conversion plays it for quotations, and it keeps playing it.

It is not run by any gate, deliberately: `doc/todo/48` is explicit that a round must not parse a
1023-page PDF on every run, and this parses fourteen of them in 6.2 s (`census`) to 6.4 s (`check`).

## What `check` found, which is why this is a defect report and not a tool announcement

**79 struck passages of four words or more are still in `doc/md/`, presented as current text.**

**Three rustdoc quotations in this tree quote a passage struck out of the very clause they cite**,
all three `/State Completed`:

| quotation | clause | issue | struck |
|---|---|---|---|
| `crates/pdf-syntax/src/date.rs:30` | §7.9.4 | #251 | "Regardless of whether the time zone is specified, the rest of the date shall be specified in local time." — deleted, no replacement |
| `crates/pdf-syntax/src/write.rs:506` | §7.5.5 | #101 | "in the decoded stream" — struck, and the sentence gains "or the beginning of the previous cross-reference stream" |
| `crates/pdf-model/src/structure.rs:1365` | §14.7.6.1 | #354 | "conforming product that owns" → "owner of" |

All three are annotated in place this round and their ledger rows say what happened. **No behaviour
changes, and that is a finding rather than a relief**: §7.5.5's erratum is the one with teeth —
`startxref` may name a cross-reference stream rather than the `xref` keyword — and `xref::read_at`
has read both for hundreds of sessions, so the code was right for a reason nobody in this tree had
found. §7.9.4's replacement says in `/Offset`'s own units what the struck sentence said in prose;
the *second* strikeout there deletes outright, and what it carried has to be re-derived from the
grammar rather than assumed.

**Seven more quotations match a phrase struck out of a different clause, and none of them is a
finding** — six are "; shall be an indirect reference", which EC3 struck out of seven other
tables and not out of the Table 192 row `appearance.rs` quotes. That is why `Landing::in_clause` exists: the length
of a passage does not separate a finding from a coincidence, and the clause it was struck in does.
Measured at four, six and eight words the check answers ten landings, seven and one — so a
threshold high enough to hide the seven coincidences hides two of the three real ones.

## Consequences

- **Every quotation and citation in the tree is now checkable against the errata**, in 6.4 s,
  by a command a round can run. `doc/todo/48` carries the other 76 passages as the next step.
- **`doc/md/` is unchanged, and all 6628 verified references with it.** Zero migration cost was the
  property that made the sidecar the right shape, and it is kept.
- **"The conversion is lossy" is now a measured claim rather than three accidents.** Sessions 401,
  413 and 414 each found a loss by tripping over it; this is the fourth and the first found by
  looking.
- **A number in a todo file is a claim like any other.** The premise here was off by a factor of
  thirteen because somebody counted `/Annots` arrays and wrote "annotations". The census that the
  file demanded before any building is what caught it, which is the rule justifying itself.
