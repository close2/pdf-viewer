# 3589 — `spec-errata` reads pdf-issues.pdfa.org's source as a second input

Session 2222. Status: **accepted**. Context: ADR 3581 (step 3 of the owner's plan of 2026-11-24),
ADR 3583 (`doc/errata/site/`), ADR 0252 (the tool, which read only Collection 3's annotations).

## Decision

`tools/spec-errata/src/site.rs` reads every page of `doc/errata/site/docs/` into the same `Note`
the PDF reader produces, with `Origin::Site` and the source line in place of a page. Each `<del>`
or `<ins>` run (a replacement joined into one change) under the page's heading and its
`<p class="location">` instruction is one note; `data-issue` becomes its subject, `data-iso` and
publication its states. `check` and `moved` read the site wherever the tree holds it, with or
without PDFs; `emit`, `renumbered` and `applied` stay the PDFs' alone.

**What a site change retires** is the printed text at the change: the struck words with three
printed words each side, since most strikes are one word. An insertion retires text only inside a
clause — not at a block's edge, after a sentence, or ahead of a clause's closing mark — and a
struck passage the same standard's pages insert again is a move and retires nothing.

**New populations.** `check` compares the PDF/A notes with `doc/pdfa/`'s conversions as it does
the others with `doc/md/`, and every note with `pdf-archive`'s rule texts (`Quoted::Rule`, each
`asks` of the table, cited at its part 4 clause). `moved` reports a part 4 clause's ground as the
table rows that cite it, not the ISO 32000-2 ledger.

**Reconciliation where both speak** (ISO 32000-2): a quotation that a Collection 3 note and a site
note of the same issue both land on is reported once, under the PDF's note, whose struck text is
the page's own glyphs; each source's other landings stand. The site carries corrections published
after the collection and the collection carries annotations never published, so neither replaces
the other.

## Consequences

`check` names more than it did, most of it errata the collection never carried, and none of it is
read yet: the count is `spec-errata check doc/ISO_32000-2_sponsored_EC3.pdf`'s, and reading them is
owed in `doc/errata-read.md`'s way. ISO 19005-4's page lands on nothing in this tree.
