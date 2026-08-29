# 0760 — The table that resolved in another standard

Status: accepted.
Context: `doc/todo/01`'s standing item, named by ADR 0750 — a table designation that is not a
number is checked by nothing, and closing that needs the foreign-standard rule the `§` checker
already has. Taken as the eight-hundred-and-thirty-second round's general subject.

**0757, 0758 and 0759 are sibling rounds'.** This number was taken above that reservation.

## Why this rather than a page

`CLAUDE.md`'s "work is chosen from both, never one" ranks a subject with demand-side evidence
above an instrument, and both demand-side instruments were asked before this one was taken:

- the corpus gate's `whose defect it is` composition puts **one** document in the `this reader`
  row, and it is `doc/todo/22`'s Arabic free text — read, priced and pinned;
- the oracle's *ambiguous, undiagnosed, and furthest from the nearest reference* list is **empty**.

Neither names work. That is the "unless an instrument ranks otherwise" clause, and it is why the
standing item was takeable.

## What the gap was, and what looking at it found

The item as ADR 0750 wrote it was narrow: `Table A.19` is cited twice, ISO/IEC 15444-1 captions
it, ISO 32000-2 does not, and a gate over `Scan::designations` would report both sites unless it
learned `read_citations`'s foreign-document rule first.

**The rule was the item; the finding is what the rule caught one level down.** `TableReference` —
the *numbered* population, gated since the thirteenth session — had exactly the same hole, and
nobody had looked because the reference resolves:

> ISO/TS 32002 Table 3 pairs each curve with SHA-2 and SHA-3 digests

ISO/TS 32002's Table 3 is *Supported ECDSA elliptic curves*. ISO 32000-2's Table 3 is *Escape
sequences in literal strings*. There are **twenty-one** such references across `pdf-model`'s
`ecdsa`, `x509` and `signature`, `viewer-core`'s `notes` and two ledger notes, every one of them
correct writing, every one of them checked against the wrong document — and the gate's own listing,
which exists so that "Table 106 — Text-positioning operators" beside a file about rendering modes
catches the eye, printed *Escape sequences in literal strings* and *Examples of literal names* as
tables this tree stands on. It is `ForeignCitation`'s failure verbatim, one level down: a reference
that reads correctly to a person and resolves, in silence, in the wrong standard.

So `read_tables` classifies foreign first and `Scan::foreign_tables` holds the result. **They are
printed rather than reported**, and that asymmetry with the `§` arm is the point: a `§` in this
tree *means* a clause of ISO 32000-2, so another document's name in front of one is a finding;
the word `Table` means nothing of the kind, and naming the standard that captions a table is how
this tree writes one. What a checker owes is to say where it did not look.

## The rule is positional, so the writing had to meet it

`another_document` looks at the word *immediately* before, and it cannot do otherwise: `eddsa.rs`'s
own first line reads "the row ISO/TS 32002 adds to Table 260", where the foreign document and one
of ISO 32000-2's tables are eight words apart in one sentence. A rule that carried a document
across a doc comment the way an attribution carries a clause would lose that table and dozens like
it.

**Which made the finding bigger than the rule.** With the positional rule in, the listing still
printed both wrong titles, because the four modules about ISO/TS 32002 also write its tables
*bare* — `Table 4's second curve has no verification here` — with the document established a
paragraph earlier. Correct English, unreadable to any line-based checker, and wrong by the
convention `read_tables` states. Every one of them is named now, in `ecdsa`, `eddsa`, `x509`,
`signature` and two ledger notes, and the two references broken across a line are reflowed. The
listing's `Table 3 — Escape sequences in literal strings` and `Table 4 — Examples of literal names`
are `pdf-syntax`'s, which is what those tables are about.

**One is deliberately left**, and it is the residue: `SignatureAnswer::CurveNotVerifiable` quotes
ISO/TS 32002 verbatim — "elliptic curves not listed in Table 3 or Table 4" — and a quotation is not
edited to please a checker. `CLAUDE.md`'s quotation rule and this one meet here and the quotation
wins.

## Two more findings, and neither was the subject

**`another_document`'s two character tests admit strings that are neither a name nor a number.**
The sets are permissive because `ISO/IEC` needs the solidus and `32000-2` the hyphen, and `all`
over a permissive set is satisfied by a string made of nothing but its punctuation. So `///`
passed as an acronym and `-` passed as a number, and the first run of the new caller reported a
document called `/// -`. Latent on the `§` side for the whole of that arm's life — it needs a bare
number immediately before the sign to show — and reached twice on the first table run. A number
must contain a digit and an acronym a letter; both are stated now.

**An annex's table has two spellings and both are the standard's.** ISO 32000-2 captions
`Table Annex L.1` and, five lines above the caption, writes "Table L.1 provides a legend for use in
interpreting Table L.2". The ledger's §L row follows the standard's cross-reference and the checker
followed its caption, so the gate's first run called the standard's own spelling a table nobody
captions. `designated_table_title` takes both, one way only: a citation may omit an `Annex ` the
caption carries, never add one it does not — which is exactly how the standard's two spellings
differ, and which leaves ADR 0750's reading of Annex O intact.

## The five places that broke the rule were the places stating it

With the foreign rule in, everything the tree cites resolved except one designation, in five
places: the amended `Annex O.1` and `Annex O.2` that Issue #700 renumbers Annex O's two tables to
— written as `Table Annex O.1`, in `spec-errata::renumbered`'s module comment twice and in the
three Annex O ledger rows.

Those five sentences say, in as many words, that a tree citing the amended designation would be
citing a caption no reader can find. **They were right, and the gate that was supposed to make
them true did not exist**: it refused a `u16` ISO 32000-2 does not have, and a designation no
`u16` can hold was not checked at all, so the claim about the instrument was false for as long as
the sentence had existed. The fix is in the writing rather than in the checker, and the convention
was already there one sentence earlier in each of them: what the erratum states is a `StrikeOut`
over a **designation**, so the designation is what a sentence about it names — *Annex O.1*, bare,
the way the caret writes it. Only the published caption is written as a table.

The gate then caught this ADR's own first draft of two of those repairs, which had reached for
`Table Annex O.1` to say why nobody may write it.

## What is not fixed, and is named rather than left

- **The scanner reads one line at a time**, so a reference broken between a standard's name and
  the word `Table` cannot be attributed. Two were — `ecdsa::Curve::bits` and
  `SignatureAnswer::CurveNotVerifiable` — and both are reflowed, the first with the reason on the
  line so it stays that way. A multi-line scanner is a different module, and nothing in this tree
  would gain by it beyond those two.
- **A verbatim quotation of another standard names that standard's tables by number**, and there
  is no rule that can tell those apart from ours. One remains, above, and it is why `Table 3` and
  `Table 4` are not simply absent from the wrong side of the listing.
- **`--bin tables`, the ninth sweep, has its own `Table ` reader** and is not touched here. Whether
  it wants the same rule is left open in `doc/todo/01` rather than answered, because it is a sweep
  a person reads rather than a gate, and its population is keys rather than numbers.

## Calibration

Trap 13, both ways, above the commit and reverted:

- the foreign rule removed → `a_table_after_another_documents_name_is_not_one_of_the_standards`
  fails, printing `TableReference { table: 3 }` — the wrong document's table, by name;
- the foreign rule loosened to allow one word between the two names →
  `the_standards_own_table_survives_a_neighbouring_documents_name` fails on
  `ForeignTable { document: "ISO/TS 32002", designation: "21" }`;
- `Table D.99` planted in a scanned source → the designation gate fails at that site;
- `Table L.1` planted in the same place → the gate passes, on the annex fallback;
- `Table Annex Q.9` planted in a ledger note → the ledger's prose half fails.
