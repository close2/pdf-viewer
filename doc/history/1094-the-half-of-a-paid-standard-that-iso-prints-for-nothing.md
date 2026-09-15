# 1094 — The half of a paid standard that ISO prints for nothing

2026-09-15. ADR 1108. Contract: §12.7's XFDF debt, which three ledger rows called "bought rather
than built". Files: `pdf-model`'s new `xfdf.rs` and six more of its own, three of `viewer-core`'s,
`viewer-confined`'s `protocol.rs`, the ledger, `doc/third-party-data.md`.

**The fetch the brief named is dead; ISO's own preview is not.** Eleven URLs tried and gone:
`adobe.com/go/xfdf_spec` answers 302 to `/404.html`, six Adobe paths 404, the PDF Association links
XFDF to the paywall while hosting XFA 3.3 itself, the Archive was offline. Free previews differ in
length: `https://www.sis.se/api/document/preview/80014076/` reaches body page **9** against
`cdn.standards.iteh.ai`'s 5 and 7. 6 410 287 bytes, title and contents verified,
`doc/ISO-19444-1-2019-preview.pdf` in the main checkout, `tools/spec-md.py` into `doc/md/`, nothing
committed. Its nine pages, sections 5.4 to 5.7.1, are the whole field half; 6.2–6.7's elements,
attributes and mapping tables are in no preview. **It is a licensed ISO text and this tree quotes
none of it** — cite the section, paraphrase — which is where ISO 16684-1's and ISO 15076-1's
previews already stand, now written down beside it.

**That split in the text is a split in the format (ADR 1108).** `pdf_model::xfdf::read` returns a
**`FormsData`**, the type an FDF is read into, so §12.7.6.4's two formats meet at one value and
`ViewState::import` applies one meaning of §12.7.8.3.2; `submission::xfdf` writes the same grammar
from the field tree `fdf` writes from (`tree` is now shared), nesting §12.7.4.2's dot notation and
splitting it back. **`<annots>` is counted and named on `owed`, not read**: 5.7.1 makes it owed and
6.4 and 6.6 say how, and a mapping from another reader would fail worse than a refusal.

**Table 240 ranks its own bits and nine are now named by number.** Every bit but 10 and 11 is
applied on the path the table makes it meaningful on, and named on `owed` by number where a document
set it against the table's own condition — 3 only where 6 is clear, 4 and 5 only where 3 is set, 7,
8, 11, 12 and 14 only in Forms Data Format. **Still only reported: 10 and 11**, not the five the
§12.7.6 row claimed. Rows: **§12.7.6.4 `partial` → `implemented`**, the other four keeping status.

**Census, calibrated first (trap 13).** The action census gained the two places a *document* asks
for XFDF and, on four planted files, named exactly the two. Over **1 450 of 1 477 documents
opened**: **none**; one `/S /SubmitForm`, performed, FDF, no `/S /ImportData`.

Gates, in a worktree five siblings edit, which is why clippy is scoped. `fmt` over the three crates
**0**; `clippy` under `-D warnings` **0** over the four `pdf-model` targets touched and over `-p
viewer-core -p viewer-confined --all-targets`, against **101** workspace-wide on three siblings'
files. `nextest -p pdf-model -p viewer-core --lib --tests` **0**, 1697; `--doc` **0**; `conformance`
**101** on three sibling sites. Tier 2: `--test corpus -- --ignored` **0**, five ratchets at slack
0; `--test actions` and `--test save_round_trip` **0**.
