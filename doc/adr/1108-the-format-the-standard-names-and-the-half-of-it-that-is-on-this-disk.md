# ADR 1108 — The format the standard names, and the half of it that is on this disk

## Status

Accepted, 2026-09-15. Session 1094. Closes the debt ADRs 1062, 1066 and 1070 left named in three
ledger rows as "bought rather than built": ISO 19444-1's XFDF, on both sides of §12.7.6.

`§N` is ISO 32000-2 and nothing else. ISO 19444-1's divisions are cited as section 5.6.3, and
**nothing below quotes that standard**: it is a licensed ISO text like the ISO 16684-1 and ISO
15076-1 previews this project holds, and the rule for all three is cite the section and paraphrase.

## Context

§12.7.6.2 lists four submission formats and §12.7.6.4's Table 243 names two import formats, and
XFDF is in both lists: "XFDF, a version of FDF based on XML as defined by ISO 19444-1". That
standard is paid, so §12.7.6.2's Table 240 bit 6 was a runtime refusal and §12.7.6.4's `.xfdf` a
declined extension — not for want of a parser (`xmlparser` has been `pdf-model`'s dependency since
ADR 0186) but because principle 5 makes a grammar taken from another reader or from sample files
not a reading of a specification at all.

**Adobe's own XFDF 3.0 document, which ISO 19444-1 adopted, is no longer published**:
`adobe.com/go/xfdf_spec` answers `302` to a 404 page, and every mirror tried was gone. What is
free is ISO's own preview of ISO 19444-1:2019 — cover, foreword, the complete table of contents
and body pages 1 to 9 — `doc/md/ISO-19444-1-2019-preview.md`. Those nine pages carry sections 5.4
to 5.7.1: the namespace and encoding `shall`s, the XFDF-to-FDF key mapping, the flat and
hierarchical form examples, and the one sentence that separates forms from annotations. They do
not carry sections 6.3 to 6.7, which are the element and attribute reference and the PDF-to-XFDF
mapping tables.

That split in the *text* is the decision this ADR records, because it is also a split in the
*format*.

## Decision

**Field data is implemented, in both directions, from the preview's own sections.** Section 5.5.2's
prologue — UTF-8, the namespace, `xml:space="preserve"` — is written as the format's bytes and
required on the way in; section 5.6.2's `<f href>`, `<ids original modified>` and `<fields>` are
read and (bar `<f>`) written; section 5.6.3's nested `<field>` elements are §12.7.4.2's dot
notation, joined on the way in and split on the way out.
`pdf_model::xfdf::read` returns a **`FormsData`** — the very type an FDF file is read into — so
§12.7.6.4's two named formats meet at one value and `ViewState::import` applies one meaning.
`pdf_model::submission` writes the same grammar from the same field tree an FDF is written from.

**Annotations are counted and named, not read.** Section 5.7.1 makes them owed: unlike a form
import, an XFDF import may create an annotation the target document does not have. That is a
departure, stated as one on `FormsData::owed`. What reading them would take is sections 6.4 and 6.6: one element per subtype
and a per-attribute mapping onto PDF's dictionaries, several dozen pages this tree does not hold.
Composing them from another reader's output is exactly what principle 5 forbids, and it would be
*worse* here than a refusal, because a wrong attribute mapping draws a plausible annotation in the
wrong place rather than failing. **A later round must not build `<annots>` from samples**; what it
needs is sections 6.4 to 6.7, and when they arrive the count and the sentence are already where the
work goes.

**`<f>` is not written and the submission says so.** Section 5.6.2 explains its `href` as pointing
at the PDF document holding the form fields, which is a file name principle 3 gives this process
none of; Table 240 bit 14's route for carrying the document is one the table restricts to FDF.
Section 5.6.3 lays out a form with `<fields>` and no `<f>`, so a file without one is a shape that
standard sets out. It is read on the way in, into `FormsData::source`.

**Table 240's bits are ranked by the table and every one it puts aside is named.** Bit 6 beats bit
3, because bit 3 is "[m]eaningful only if the SubmitPDF and XFDF flags are clear"; bits 4 and 5
follow bit 3 down; bits 7, 8, 11, 12 and 14 each say "shall be used only when the form is being
submitted in Forms Data Format (that is, when both the XFDF and ExportFormat flags are clear)".
Nine sentences on `Submission::owed`, one per bit a document set against the table's own condition
— a flag that changes *what is sent* is applied, and a flag the table forbids in this format is
carried to the host as a sentence rather than obeyed or dropped.

**The media type is `application/xfdf`**, registered 2022-04-29 by the same ISO TC 171/SC 2 that
registered `application/fdf`, which the registration says outright and which makes
`application/vnd.adobe.xfdf` a "[d]eprecated alias".

## Alternatives

**Go on refusing until ISO 19444-1 is bought.** Rejected: the preview is the specification's own
text and its nine pages state every rule the field half needs, each as a requirement or a laid-out
example. Refusing what a held specification defines is not principle 5, it is the opposite.

**Read `<annots>` from the pdf.js or Apryse descriptions of it.** Rejected above.

**Return an `Xfdf` type of this module's own rather than a `FormsData`.** Rejected: §12.7.6.4
states one import, and two result types would be two chances to disagree about what a fully
qualified name is — which is the one string the whole operation turns on.

**Accept a truncated file as a short one.** Rejected: `xmlparser` is a tokenizer and hands back the
tokens of an unclosed document without complaint, so `read` checks the element balance itself. A
partial import is a form saying something nobody wrote, which is worse than an error.

## Consequences

`Format` gains `Xfdf` and `Refusal` loses its `Xfdf` variant; the enumerator crosses
`viewer-confined`'s protocol as `3`. §12.7.6.2, §12.7.6.4 and §12.7.8 each lose the sentence they
called bought rather than built; §12.7.6.4 is `implemented` for the two formats Table 243 names,
and the submission row's remainder is Table 240 bits 10 and 11 alone. The XFDF annotation departure
is named on the §12.7.8.3.4 row, beside the FDF one it resembles: in both, an annotation the file
carries is read or counted and drawn by nothing. `doc/ISO-19444-1-2019-preview.pdf` is a fetched
text like ETSI's and X.690's — obtained free, not redistributable, converted into `doc/md/` by
`tools/spec-md.py`, and committed nowhere.
