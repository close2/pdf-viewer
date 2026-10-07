# Q308 — May the fourteen's pair kerning come from Adobe's Core 14 AFM files?

Asked by round 1412, the third rich text round (ADRs 1660, 1661). §12.7.4.3's row and the two rows
that share its build are `partial` on one property now: XFA 3.3 chapter 27's `kerning-mode:pair`
(*Kerning*, pages 1203 and 1204), which the chapter lists among the attributes its minimum set
supports. It needs the pairs of the face a run is set in, and for the faces a rich text field
mostly names — §9.6.2.2's fourteen, `Helv` above all — this tree holds none.

## What the tree has, and why it is not enough

- **Widths come from pdf.js** (`crates/pdf-font/src/standard_metrics.rs`, ADR 0007), chosen because
  they are Apache-2.0 where the URW metric files on this machine are AGPL-3.0. pdf.js carries no
  kerning pairs.
- **The compiled-in programs** (`crates/pdf-font/src/standard.rs`) are ten bare CFF programs, which
  have no kerning table, and four Liberation Sans faces, whose pairs are Liberation's and not the
  ones the fourteen's metrics were published with — so taking them would kern Helvetica by
  another typeface's numbers and leave Times unkerned.
- **Adobe's own pairs are public**: the Core 14 AFM files state them as `KPX` lines — 2 705 for
  Helvetica and 2 073 for Times-Roman, none for the Courier faces — and Apache PDFBox ships them
  under `pdfbox/src/main/resources/org/apache/pdfbox/resources/afm/` with Adobe's `MustRead.html`.

## The licence, paraphrased

Adobe's notice lets the files be used, copied and distributed for any purpose without charge, with
or without modification, on four conditions: the copyright notices are kept, the AFM files are not
distributed without the notice, modifications are noted prominently, and the notice paragraph is
not modified.

## The question

May `pdf_font` carry a table of the fourteen's `KPX` pairs generated from those files, the way
`standard_metrics.rs` carries widths generated from pdf.js — with `MustRead.html`'s paragraph
added to `/NOTICE`, `data/standard-fonts/PROVENANCE.md` naming the files and their source, and the
generated table marked as a modification of them?

## Recommendation

**Yes.** The pairs are the published metrics of the typefaces §9.6.2.2 names, the same kind of fact
as the widths this tree already takes from Adobe's numbers by way of pdf.js, and the conditions
are attribution conditions a NOTICE file meets. The build that follows is `pdf_font`'s table and a
`kern` lookup for an embedded TrueType or OpenType program's own pairs, then the rich text layout
applying them as §9.4.3's `TJ` adjustments.

## What the tree does meanwhile

A run stating `kerning-mode:pair` is laid out unkerned and `Owed::RichTextUnapplied` names the
property where it happens; the three rows stay `partial` on it alone.
