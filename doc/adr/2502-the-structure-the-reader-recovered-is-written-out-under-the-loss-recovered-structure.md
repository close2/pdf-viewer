# 2502 — The structure the reader recovered is written out under the loss `recovered-structure`

Status: implemented. Session 1834, the R01 slot of batch one hundred and twenty-four (Sonnet, high).
Builds ADR 2490's row R01. Context: ADRs 2444 and 2457 (the refusal this reopens for one verb),
1014 and 2394 (what a fallback may be), 0816 (no producer's content stream is edited), 1187 (the
removed encryption's claim); `doc/questions/A681` item 1; `crates/pdf-transform/src/archive/recovered.rs`.
ADR 2503 holds the measured outcome. Q700 is unused.

## The reading

ISO 32000-2 §7.5.5's Table 15 makes the trailer's `/Root` "( Required; shall be an indirect
reference )" and §7.7.2's Table 29 makes the catalog's `/Pages` the same, and §7.7.3.2's Table 30
makes a page tree node's `/Kids` required. The 18 documents of ADR 2490's R01 that this reader
draws a page of fail one of them, on four causes: the catalog's `/Pages` resolves to no node (10
documents), `/Root` names an object that is no dictionary (4), the trailer states no `/Root` (3),
and a node states no `/Kids` (1). The reader draws them anyway: `pdf_model::Pages` asks every object
whether it declares itself a page (Table 31's required `/Type`) and draws those in ascending object
number; the objects it asks are the ones the file's cross-reference information names. The verb declined them as sources because rewriting would state a structure no producer
wrote (ADR 2444).

**The recovery is this reader's choice, not a clause's.** `doc/pdfa-blocking-issues.md`, eight
comments in `pdf-transform` and two runtime strings of `optimize.rs` (the sentences that decline a
source, which `recovered::rebuild` prefixes to its own) cite it as "§C.4's recovery". Annex C.4 (informative, titled "File Limits")
says only that a processor reading a damaged or missing cross-reference table "may attempt to
rebuild the table by scanning all the objects in the file". That is the table's reconstruction;
nothing in the standard defines how a missing catalog or page tree is recovered, and Annex C does not
bind. The wording is corrected at every one of those sites. §7.7.3.2 does say one thing that bears on
the second choice, writing the pages as a flat tree: "PDF processors shall not be required to
preserve the existing structure of the page tree."

## The decision

1. **What is rebuilt.** A catalog, and one `/Pages` node whose `/Kids` lists every page this reader
   draws, in the order it draws them, with `/Count` stating their number. The catalog is the
   producer's dictionary where `/Root` resolves to one (every entry but `/Type` and `/Pages`
   carried, references mapped), otherwise a catalog stating those two entries and nothing else.
2. **What is kept.** Every page dictionary, content stream and resource is carried as the source
   states it (ADR 0816 holds: no byte of a content stream is edited). Two kinds of entry are
   restated because a page owes them to its tree: `/Parent` names the new root, and the four
   entries §7.7.3.4 makes inheritable (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`) are copied
   onto a page that states none, from the nearest ancestor its old `/Parent` chain reaches,
   because the new root states none of them. No mark, page or stream is composed; this is not ADR
   1014's appended page, which composes a page.
3. **What is restated, and where.** The rebuilt file is serialised, reopened under the default
   limits, and held to the three checks that declined the source (the catalog, the recovery-only
   reading, the node with no children), plus: it must walk to as many pages as the reader drew, by
   its page tree and not by scanning. Only then does the conversion run on it. The report carries a
   `recovered_structure` entry (pages, how many were found by their own declaration, whether the
   catalog was kept or written, and the sentence that declined the source), the text report one
   line, and the output's own `xmpMM:History` one `converted` event. A rebuilt file that already
   conforms is not copied as it stands, because the copy would record nothing.
4. **The loss.** `recovered-structure`, authorised by `[profile] recovered-structure = "discard"`
   in both shipped profiles. ADR 2490's row R01 names the loss and the profile "both" and states no
   action word; `discard` in both is **this round's reading** of that row, taken from R02's row,
   which does state it, and not a sentence of the ADR. It is `discard` and not `derive` because
   `derive` is "a new representation ... made from content the document already has" (`config.rs`'s
   `Kind`, owner answer A55 that a derive is never a default), and this writes no representation of
   a content: it states a structure the source does not, which is the loss the word names. It is a key of the `[profile]` table and
   not of a site because no requirement of the table declines the document: the refusal comes from
   the source's structure, before stage 1.
5. **How `optimize` and `redact` part from the archive verb.** They keep the decline. They carry
   the producer's objects as they stand, a closure copy that rewrites no object (ADR 2457), so a
   rebuilt tree would be the one thing they wrote that no producer did, for a request that asked
   for no such thing. The archive verb already writes a catalog of its own (metadata, output
   intent) and restates page entries, and its request is a conversion to a form that needs a
   walkable tree. ADR 2444's sentence that the pages a reader finds without a `/Kids` array "are
   not the producer's" is answered the same way: each object is the producer's; their membership
   and order are the reader's, and that is the loss.
6. **Refused by name, as today**: a source that is encrypted or signed (the output's bytes are not
   the bytes the signature covers, and the encryption's decision belongs to its own requirement,
   ADR 1187), one that states more pages than this reader reached, one whose header states no version, one with a page that is no indirect dictionary
   this reader parses whole, and one this reader draws no page of (ADR 2490 section 7.2's five).
   The photograph (ADR 2394) is not R01's fallback beyond the gate that ADR states, which Q694
   item 1 holds open.
7. **A stated count is compared, never trusted.** The reader finds pages through what the file's
   cross-reference information names, so a source whose later sections it does not reach would be
   written as the pages it reached and nothing said of the rest. Two statements of a count are read
   and refuse the source when they exceed the pages drawn: a linearization parameter dictionary's
   `/N` (ISO 32000-2 §F.3.3, Table F.1) and the `/Count` of the page tree root where the catalog
   still names it (Table 30). Nothing else in a file states a page count. `027613` is the case:
   `/N 429`, one page reached; it is refused by name and the reader's short reach (the later
   cross-reference sections of a linearized file) is a robustness-lane candidate, not repaired here.
8. **Bounds.** The page dictionary is parsed by the parser, which admits a value nested to 255
   arrays and no deeper (a deeper page does not parse whole and is refused by name); `carry`'s own
   depth check, 257, is the backstop for a value no parser built. The walk up a page's `/Parent`
   chain ends at 64 ancestors and at the first repeat. **A value an ancestor states directly is
   placed once as an object of its own and every page that inherits it names that object**, so the
   output is not the page count times the value; §7.7.3.4 inherits values "as-is", and a reference
   to the object that holds a value changes no meaning. The page list is the reader's own count,
   bounded by the object limits the document was opened under. (`Pages::every()` itself holds a
   page's data for every page at once; its cost on a large valid tree predates this round and is
   found, not fixed.)

## What remains, and what would be elegant

The owner's word of 2026-10-27 is that where a refusal still stands for a document the viewer
draws, the round names the elegant remedy and that, failing it, the loss can be stated on an
appended page under the profile (ADR 1014's construction). For the six documents of this row that
are still refused:

- **An encrypted source** (2): the elegant remedy is to run the rebuild over the decrypted objects
  with the encryption's own decision taken as well (`Loss::Encryption`, ADR 1187), the claim read
  from the source and not from the rebuilt file. Failing that, the claim and the structure's
  recovery are stated on an appended page. Not built: it needs the decision for a requirement the
  rebuilt document no longer fails.
- **A signed source** (1): the elegant remedy is the signature route that already exists for a
  non-conforming signed source, run over the source's bytes before the rebuild so each signature's
  verdict is read against the bytes it covered. Failing that, ADR 1014's own case, the statement
  the signature made to its reader on an appended page.
- **A source stating more pages than the reader reaches** (1, `027613`): the elegant remedy is the
  reader following a linearized file's later cross-reference sections, so it reaches the pages the
  source states; failing that, an appended page (ADR 1014) stating that the file states 429 pages
  and 1 was written. Not built: the reader is not this slot's.
- **A page that is a direct object** (1): the elegant remedy is to give the page an object of its
  own; its inheritance chain then starts at no identifier and is read from its position in the
  tree. Not built here.
- **A page whose dictionary does not parse whole** (1): nothing can be carried, and an appended page
  has nothing of the page to state. Only ADR 2394's photograph reaches it, inside its gate.

Build only what the slot asks: none of the five is built.

## Consequences

- `Loss::RecoveredStructure` is a new variant of `Loss` (inserted before `TextAsImage`, so
  `Loss::ALL` grows by one), and `Authorisations` gains one field; every literal of it in the tests
  states it.
- `optimize` and `redact` are untouched by this ADR; their tests still hold the decline.
- Tests: `crates/pdf-transform/tests/archive/recovered.rs` plants all four causes, the withheld
  loss, the two profiles, the refusals by name (on corpus documents, skipped where the submodule is
  absent), the stated counts, the cycle and the chain past 64 ancestors, a value nested to the
  parser's last depth and one deeper, and `a_direct_value_every_page_inherits_is_written_once_and_named_by_each`.
- veraPDF at both targets on every output, under both profiles, in ADR 2503.
