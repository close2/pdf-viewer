# 2670 — A document holding none of the pages it states is refused by name under every profile

Session 1921, a single archive round.
Status: accepted and built in `pdf-archive` (`pages_held.rs`; `table::file_structure`, the row
`file-structure/page-tree-holds-a-page-it-counts`; `coverage`, section 5.1 of both parts) and
`pdf-transform` (`archive/mod.rs`'s `refuse_a_document_holding_none_of_its_pages`, the row's answer
in `archive::decision`).
Context: `doc/questions/A775` (Reading 2, all profiles), ADR 2596 (the loss `page-not-held`), ADR
2583 (the two page tree rows), ADR 2456 (the `/Count` row's narrowing), ADR 2669 (the same round's
A757).

## The reading

ISO 32000-2 §7.7.3.2's Table 30 makes the root's `/Count` the number of leaf nodes beneath it, and
neither it nor Annex F.3.3's `/N` forbids zero; Q775 found no sentence of ISO 32000-2, ISO 19005-2 or
ISO 19005-4 requiring a document to hold a page. What a file stating one page and holding none
breaks is the count. `T02-03_006_page-object-missing` is the case: `/Count 1`, `/Kids [2 0 R]`, and
the cross-reference section places object 2 where object 3 sits.

Q775 recommended a split: an archive of the statement page alone under `append-statement`, a
refusal under `discard`. The owner overruled it ("This really is a broken pdf"): such a document
has archived nothing a reader saw, and it is refused under every profile.

## Decision

1. **One sentence, one place.** `pdf_archive::NoPageHeld` holds the count and where it is stated —
   the root's `/Count`, or a linearization parameter dictionary's `/N` where the root states none —
   and its `Display` is the sentence: `page-not-held: its page tree root's /Count states 1 page and
   the file holds 0 of them (ISO 32000-2 7.7.3.2, Table 30), so it holds nothing a reader sees to
   archive (doc/questions/A775)`. It lives in the checker's crate because the converter already
   depends on it and not the other way round.
2. **The checker: a row of its own.** `file-structure/page-tree-holds-a-page-it-counts`, section
   5.1 of both parts through Table 30's `/Count` row, narrowed to the one disagreement that leaves
   nothing to read: a root counting one page or more where this reader draws none. A count that
   disagrees with a tree holding some pages stays unjudged (ADR 2456): that is A727's loss, which a
   profile may take. Not a widening of the `/Kids` row, for ADR 2583's reason: a report naming each
   sentence of Table 30 by its own row says which one failed.
3. **The converter: before anything.** `run_pass` refuses such a source with
   `Refusal::Reconstructed` (exit 4) before the structure checks and the rebuild, so a profile taking
   `recovered-structure` and `page-not-held` meets it exactly as `refuse-any-loss` does. The
   decision table answers the row with `Because::TheFence`: writing pages for it would be composing
   them, and nothing later in the project's life closes that.
4. **"Held" is this reader's pages**, `pdf_model::Pages` asked one index at a time until one is
   found, so a document holding its first page costs one page's reading; the checker passes the
   examination's own pages. The search for a linearization dictionary, which reads every object the
   cross-reference information names, runs only for a document already found to hold no page.

## What it moves

Only refusal sentences. `T02-03_006` was refused before ("this reader draws no page of it" under the
two profiles that rebuild, the declined tree's sentence under the rest) and is refused now under all
nine shipped profiles with the shared sentence; the census of ADR 2669 shows no other document
changing state by it. A pure-`null` kid under a root counting one is refused by the same sentence,
where the rebuild would have refused it as drawing no page.

## Tests

`crates/pdf-archive/src/pages_held.rs` (the sentence), `table::file_structure`'s
`a_root_counting_a_page_where_none_is_held_is_failed_with_the_shared_sentence`, and in
`crates/pdf-transform/tests/archive/recovered.rs`: every shipped profile at both targets refuses a
planted one-counted, none-held tree with the checker's sentence; `T02-03_006` likewise, and through
the program with exit 4 and no output; and a tree holding twelve of thirteen is not refused for
holding none. `crates/pdf-archive/tests/witnesses.rs` holds `T02-03_006` to both page tree rows.
