# 2803 — What the converter wrote that its own check refused is traced to four readers that disagreed

Session 1996, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/page_tree.rs`:
`refuse_a_page_tree_reaching_none_of_the_pages_drawn`; `archive/mod.rs`: the declining checks and the second pass's third
condition; `archive/fenced.rs`: `reached_from_annotations`; `archive/as_drawn.rs`: `Drawn::unread`, `Drawn::left`;
`archive/settle.rs`, `archive/report.rs`) and `pdf-archive` (`table/document_level.rs`: `embedded_file_is_itself_pdfa`).
It answers five lines of `doc/pdfa-blocking-issues.md` section 8 (`D-defect`).
Context: ADRs 0816, 2502, 2516, 2534, 2638, 2795.
Clauses: ISO 19005-2 sections 6.2.2, 6.2.5, 6.8; ISO 19005-4 sections 6.2.2, 6.2.5, 6.9; ISO 32000-2 §7.4.3, §7.5.5
(Table 15), §7.6.6, §7.7.3.1, §12.5.5.

## 1. The causes, traced

Each is a place where two readers in this tree disagree about the same bytes, so the source's check passed what the
output's check then refused.

- **`T02-02_002_page-tree-kids-links-recursive` (every profile, both parts).** The root's `/Kids` is `[1 0 R]`, the
  root itself, with `/Count 1`. `pdf_model::Pages` walks no page from it and draws the page it finds by its own `/Type`
  (`found_by_scanning`), so the source holds a page and meets `page-tree-holds-a-page-it-counts`; the conversion
  copies the closure from the catalog, which reaches no page object, and the output holds none. The declining checks of
  ADR 2502 asked only whether `/Kids` was an array.
- **`hesr13613-sup-0001-Authormatrix` (`as-if-printed`, `state-every-loss`).** The undefined operator is `>` in a text
  field widget's appearance stream (`> Tj`). The survey names the page the widget is on, so the page was photographed,
  and the photograph replaces `/Contents` and `/Resources` and keeps `/Annots`: the appearance stayed with its operator.
- **`poppler-90-0-fuzzed` (`as-if-printed`, `state-every-loss`).** Page 4's run `0..9172` is one this reader draws as
  0 (ADR 2638), so its page was left to the run writer; the stream's `ASCII85Decode` data holds a group past 2^32
  (§7.4.3), which `decoded_stream_data` refuses as `Corrupt` while the content reader draws the bytes before it. The
  writer skipped the stream in silence and the run stayed.
- **`027613` (`as-if-printed`, 4).** Object 3516, a graphics state with `/TR /Identity`, lies past the damaged
  cross-reference table and is found by its header; the check enumerates the table's objects and saw only object 4330.
  The first pass removed `/TR` there, and the output's fresh table listed 3516. The source failed the row, so ADR
  2795's condition (a requirement the source met) did not fire.
- **`auth-event-ef-open` (`no-data-loss`, 4).** The attachment's crypt filter states `/AuthEvent /EFOpen` (§7.6.6) and
  the file supplies no password; the embedded-file row skipped a stream it could not decode, so the source met it, and
  the first pass copied the stream with `/Filter [/Crypt]`. The second pass removed it under the three profiles that
  discard such a file; under `no-data-loss` the output stood refused by its own check rather than by name.

## 2. The decision

1. **A page tree reaching none of the pages drawn is declined** (`refuse_a_page_tree_reaching_none_of_the_pages_drawn`),
   beside ADR 2502's two checks: refused by name where `recovered-structure` is not granted, and otherwise written with a
   new root over the pages drawn. §7.7.3.1 makes the tree what "defines the ordering of pages in the document"; one that
   reaches no page object defines none, and the scan's order is the reader's choice the loss already names.
2. **An operator in a stream an annotation's appearance reaches keeps the fence's refusal** (`fenced::pages`): a
   photograph cannot answer it, so the row is refused by name (ADR 0816) instead of writing a file the second verdict
   refuses. The walk follows §12.5.5's appearance streams and, through their resources, forms, patterns, Type 3 glyph
   procedures and soft mask groups.
3. **A run in a stream whose filters do not decode whole is not written, and its page is photographed** where the
   profile allows it (`Drawn::unread`, `Drawn::left`). Writing the number would mean re-encoding the bytes the window
   drew before the damage, which drops the rest of the producer's stream; the photograph is the profile's existing
   answer for a page whose content the converter cannot edit.
4. **The second pass also runs where the output still fails a requirement the first pass changed** (`changed > 0`). A
   requirement changed at every place the source's check enumerated, and still failed, failed at a place that check
   never reached; the same one extra pass answers it.
5. **An embedded file whose data the file's own encryption withholds fails ISO 19005-2 section 6.8 and ISO 19005-4
   section 6.9** in the check, where any other undecodable stream is still passed over (that one is a fact about this
   program's filters; this one is a fact about the file, which holds no bytes any reader of it alone can judge). The
   first pass then answers it by the profile's embedded-file answer: removed and named under `as-shown`,
   `as-if-printed` and `state-every-loss`, refused by name under `no-data-loss`.

Each profile: 1 is the loss `recovered-structure`, granted by all four main profiles (`append-statement` under
`no-data-loss` and `state-every-loss`); 2 and 5's refusals are the fence's and `no-data-loss`'s own; 3 adds pages to the
photograph `as-if-printed` and `state-every-loss` already take (`page-as-image`, `text-as-image`); 4 changes no answer.

## 3. What it costs

- A conversion whose output still fails a requirement it changed is converted twice before it is refused; one whose
  output conforms pays nothing.
- `hesr13613` was a wrong output and is now a refusal: the widget's `> Tj` is the producer's content stream, and no
  remedy edits it.
- `poppler-90-0-fuzzed` has more pages photographed than before under the two printing profiles, each named in the
  report and the history.

## 4. Not built

- The check's enumeration of objects a cut or damaged table leaves to their headers (ADR 2795 section 4); a
  `pdf-syntax` API, outside this round.
- The rows of ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.5 still resolve a `uniXXXX` name through the reader's lenient
  mapping (ADR 2540 section 5): a `pdf-font` and `pdf-archive` change owing tier 2, and no corpus document reaches it.

## 5. Evidence

Tests (each fails under its plant, `doc/history/1996-*.md`): `recovered::a_page_tree_whose_kids_name_their_own_root_is_written_from_the_page_drawn`,
`photographed::an_operator_inside_an_annotation_appearance_keeps_the_refusal_a_photograph_cannot_answer`,
`photographed::a_run_in_a_stream_whose_filter_does_not_decode_whole_is_photographed`,
`a_requirement_changed_where_the_check_looked_and_failed_where_it_did_not_is_converted_again`,
`recovered::an_embedded_file_the_file_s_own_encryption_withholds_is_decided_in_the_first_pass`.
