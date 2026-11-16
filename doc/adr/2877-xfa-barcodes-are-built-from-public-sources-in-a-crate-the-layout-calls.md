# 2877 — XFA's barcodes are built from public sources, in a crate the layout calls

Session 2030. Status: **accepted**, carrying out `doc/questions/A840` item 3. Clauses: ISO 32000-2 §K.2 (XFA 3.3 as
the resource's format); XFA 3.3 chapter 12, *Using Barcodes*, pages 418–444, and the `barcode` element's template
reference, pages 610–620.

## What is built, from what

The owner asked for the symbologies XFA 3.3's `barcode` names to be built from public sources where that is
reasonable, the edge cases a missing ISO text leaves open written down, and a purchase asked for where it is not.
Three were looked for, the three a census of 53 dynamic forms found (`doc/todo/78` section 2):

- **Code 39 (`code3Of9`, `logmars`): built**, in `crates/xfa-barcode/src/code39.rs`, from MIL-STD-1189A — a US
  government standard, read as Appendix E of DTIC's ADA180561 (`doc/third-party-data.md`). Table E.1 is carried
  as a table; §4.1, §5.2, §5.2.2 and §5.2.3 fix the character set, the elements, the gap and the widths. It prints
  no check character and no full-ASCII table, so `checksum="auto"` and `code3Of9extended` are refused by name,
  not reconstructed from memory or from another program (principle 5).
- **PDF417 (`pdf417`): not built.** The patents (US 5,243,655, 5,304,786, 5,399,846) print no codeword tables, or
  tables and modes of a pre-standard PDF417 that differ from the standard's. The one complete public text found is
  ISO/IEC 15438:2006 as Public.Resource.Org publishes it, incorporated by reference into US federal regulation; its
  pages still carry ISO's copyright notice, and whether this project builds from that copy is the owner's call,
  not a round's. `doc/questions/Q857` item 1.
- **Data Matrix (`dataMatrix`): not built.** GS1's guideline and general specifications refused the download;
  US 5,612,524 is the pre-ECC 200 family; NASA-STD-6002D prints the size table only. The encodations, the
  Reed-Solomon field and the module placement are in ISO/IEC 16022 alone: `Q857` item 2 asks the owner to buy it.

Each other identifier the template reference defines (62, pages 616–620) is `Refusal::NotBuilt`, said by name.

## Where the symbol is drawn

Nowhere yet, and on purpose. A static form's barcode field draws what its widget's appearance stream holds: XFA's
static half keeps every mark the AcroForm widget's (`doc/scope-ratifications.md`), and a widget with no appearance
is §12.7.4.3's variable text, which knows no barcode — composing bars into it would be marks no clause of ISO
32000-2 asks for, the authoring line's far side. A dynamic form's barcode is a mark its template names, placed by
XFA 3.3's layout: the construction ADR 2876 accepted. So the encoder is a crate of its own, `xfa-barcode`, with no
dependencies and one responsibility — a symbology's data into its elements — which the drawing piece of
`doc/todo/78` section 4 calls. It takes the template's `type`, matched without case (page 421), `checksum`'s
keywords (page 613) and holds `moduleWidth` and `wideNarrowRatio` to page 440's limits.

## Why in-tree, and the decoder as a dev-dependency

A reviewed crate may stand in for in-tree code where the owner's bigint precedent says so: where writing it here is
the risk. Here a symbology is a table and a short rule, and the risk is the table — which a crate's own tests hold
against nothing this tree can read. So the tables are carried from the source, cited, and `rxing` (Apache-2.0,
`default-features = false`) is a dev-dependency only: every symbol is read back by an independent decoder, as
evidence that it is readable, never as the definition. `doc/stack.md` records it.

## And the checker reads XFA 3.3

XFA 3.3 now travels in `doc/specifications.zip`, and `tools/conformance/tests/xfa_citations.rs` reads its
conversion: every `XFA 3.3 … page N` in a comment names a page the text has, an italic section title beside it is
on the cited pages or the one before, and no blockquote is attributed to it — its notice permits no reproduction of
its prose (`doc/third-party-data.md`). It is not ISO 32000-2's quotation check: XFA 3.3 has no numbered clauses and
is never quoted, so the page is what can be checked.
