# 888 — A reconciliation per clause, and a refusal by name

2026-09-03. Argued in [ADR 0821](../adr/0821-a-merge-is-a-reconciliation-per-clause-and-a-refusal-by-name.md).
The seventh implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`. `main` had not moved since session 886's merge, so nothing was
merged in.

Session 886 left `merge` deliberately, on the ground that the hard part is not the machinery.
That reading held: cross-file renumbering was `pdf_syntax::serialize::Assembly`'s already, and the
round's whole substance is RFC §6.2's document-level reconciliations — one per clause, each
derived from a sentence, each with its refusal path.

Touched: **`crates/pdf-transform/src/merge.rs`** (new), `src/lib.rs` (`Plan::Merge`,
`Plan::sources`, three refusals, `Origin::Merged`, `apply` over several documents),
`src/bin/pdf-transform.rs` (the verb, `a.pdf:1-5`, `--collate`);
**`crates/pdf-transform/tests/merge.rs`** and **`tests/merge_corpus.rs`** (new),
`tests/split.rs` (trap 30); `crates/pdf-model/src/content/colour.rs` (§14.11.5's second home),
`src/content.rs` and `src/content/transparency.rs` (the page threaded to it);
`doc/conformance/ledger.toml` (eleven rows), `doc/todo/02-every-round.md` (the new gate line),
`tools/state.sh`; `doc/traps/instruments-and-reports.md` and `doc/HANDOVER.md` (trap 30);
`doc/state-of-play.md`, `doc/crate-map.md`, `doc/todo/57-…`; ADR 0821, this file.

## 1. The one reading finding, and it was found by needing a writer's answer

§14.11.5's first sentence puts `/OutputIntents` "in the document catalog dictionary … or a Page
dictionary", and two paragraphs later it says that a processor that chooses to respect output
intents **shall** use a page's own where a page has one. This tree does choose to respect them —
that is what `output_intent_space` is for — so the `shall` bound it, and only the catalog was read.

It came up because the merge needed an answer: several documents' catalog arrays cannot all be one
document's, and a catalog array claims *every* page. The clause's second home is exactly the
construction, so the reader was taught it and the writer uses it. A scan of all 974 corpus
documents found **no** page-level array and 17 catalog ones, so the reader change is provably inert
on everything this project measures — which is why it was taken rather than deferred.

## 2. What each reconciliation rests on

Each is in ADR 0821 with its sentence; the short list is §8.11's groups and one default
configuration out of several (§8.11.4.3's parenthesis about `/BaseState`), §7.9.6's colliding keys
renamed with `/Dests`' references chased through §12.3.2.4's two homes, §12.3.3's outlines spliced
rather than parented under an item Table 151 would make this program invent a `/Title` for,
§12.4.2's labels one entry per page, §14.11.5 above, §12.7's Table 224 entry by entry, and
§12.8.1's signature crossing without its `/V` — which settles §12.8.2.2's "only one signature field
… DocMDP" by construction.

**Two refusals, and the argument for the shape.** §12.7.4.2's "actual field dictionaries with the
same fully qualified field name shall have the same field type ( FT ), value ( V ), and default
value ( DV )" is refused by name across sources, and Table 31's single `/Parent` refuses a page
named twice. The rename that would resolve the first exists in §12.7.4.2's own hierarchy and is
recorded as not taken, because it changes what §12.7.6.2's submit-form action exports — a change to
the document's meaning invisible on the page.

## 3. Two defects the corpus walk found, both real, neither in the reading

- **§8.11.4.3's arrays were read direct.** `issue18823.pdf` states `/OFF`, `/Order` and
  `/RBGroups` as indirect references, so the merged default configuration turned nothing off and
  the page drew with layers the document had hidden. §7.3.10 makes a reference equivalent to what
  it names; a reader that accepts only the direct form is reading a different file. Every array of
  Table 98 and Table 99, and Table 224's `/Fields` and `/CO`, go through one helper now.
- **An `/AcroForm` with an empty `/Fields` was carried.** Table 224 makes `/Fields` "( Required )
  An array of references to the document's root fields" and §12.7.3 makes the dictionary what a
  document's interactive form "shall be defined by", so one with no field states no form. Carrying
  it out of one source changed how a *different* source's annotation drew, which the raster oracle
  caught — an entry reaching across sources is precisely what these reconciliations exist to
  prevent.

A third finding was not a defect in the verb: `issue15096.pdf` states one fully qualified field
name twice with two values *inside itself*, and the first implementation refused to merge it. The
clause binds the document that holds both fields, that document already held them, and carrying
what the producer wrote is RFC §11.1's premise — so a within-source collision is warned about and a
cross-source one is refused.

## 4. What was looked at

`qpdf --qdf` on the sources and the merged file; the two rasters' differing byte counts with the
offset of the first, which is what said "a whole region" rather than "an antialiasing level"; the
merged document's outline chain printed item by item with its `/Prev`, `/Next`, `/Parent` and
`/Dest`, which is how the splice and the destination rewriting were confirmed before any assertion
was written; and Table 96, Table 98, Table 99, Table 150, Table 151, Table 161, Table 224, Table
225, Table 226 and Table 401 read whole.

## 5. A gate that was measuring the scheduler

`tests/split.rs` failed once, on `left: 1, right: 2`, and the cause was not this round's code:
`MemorySinks` hands its outputs back "in the order the outputs were opened", `split` opens them
inside a `rayon` map, and three assertions indexed the vector by position. It is **trap 30** now,
and every one of them looks its output up by name.

## 6. Gates

`pdf-model`, `pdf-transform`, the ledger and four documents changed, so the whole `doc/todo/02` §2
sequence was run in this worktree — the walking lines under `tools/bounded.sh`, one at a time, with
a wait for a neighbouring round's quorra walk to finish and for the load to fall before the oracle.
The results are in the round's report and not here.

## 7. What the next transform round does first

`doc/todo/57`'s order: `pages`, which `merge` with one input mostly subsumes; `optimize`, where
§7.5.7's producer half is owed; `split --at-bookmarks`; and **the structure tree**, which is now
the largest single thing the suite owes — neither verb carries §14.7's `/StructTreeRoot`, and every
carried page still states a `/StructParents` that names nothing. The foreign-readback gap has a
third writer in it, and the merged files are the ones most worth showing a foreign reader.
