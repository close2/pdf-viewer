# 1337 — The last walk-held rows take fixtures, and §7.4.9's undrawn spaces are counted

Ledger slot of batch fifty-two. ADRs 1509, 1510. No status moved; no question written.

**Fixtures (ADR 1509).** `crates/pdf-model/tests/tagged_pdf_fixtures.rs`: §14.8.2.5.2 (a page whose
`/Annots`, stream and tree orders differ; the logical order is the tree's), §14.8.2.6 (the soft
hyphen by `ToUnicode` and by an element's `/ActualText`), §14.8.2.6.2 (show-string boundaries carry
no meaning; `inferred_separators` 0), §14.8.6 (§14.8.6.1's default namespace).
`crates/viewer-core/tests/objects_not_understood.rs`: §I.1, an unknown filter informed and an unknown
entry ignored on one page. Each calibrated by a mutation of its input that made it fail.
`ONLY_WALKS_CEILING` 5 → 0. The brief's clause descriptions were wrong for four rows (§14.8.2.5.2
is annotations, §14.8.2.6.2 word breaks, §14.8.6 namespaces, the `/Version` precedence is §I.2's);
the fixtures follow `doc/md/`.

**§7.4.9 (ADR 1510).** Row stays `partial`; it now names its fixtures by function, so it is held by
fixtures. `examples/jpx_colour_census` over 90 801 files (1 789 with `JPXDecode`, 105 108 images,
103 868 beside a `/ColorSpace`): no `colr` box states 19, 20, 21 or 24; CIE Lab under D65 in 2
documents, never deciding; deciding boxes 1 210 sRGB, 14 grey, 8 CMYK, 2 sYCC. The note says so, and
says 18 and 21 are the codec's conversions whose texts are not held either. sRGB's fixture already
existed (`srgb_is_drawn_as_its_samples`), so no codestream was generated.

**Found.** The classifier reads a corpus root in a comment: `viewer_core::notes::about` names
`doc/pdf.js` in one, so every test calling it is a witness. And the §I.1 report for an unknown
filter says "malformed image: stream did not decode" — `Document::image_stream`'s `Option` drops the
reason. Both are in other rounds' files and are left.

**Also touched.** `doc/todo/65`'s eighth shape (re-derived: no row), `doc/verify.md` (the census
line), `doc/habits/the-ledger-and-claims-about-this-tree.md` (one sentence that said "five").

**Gates.** `cargo test -p conformance` exit 0 (`bounded` self-test failed once at load 55, passed
on rerun); `cargo nextest run -p pdf-model` 1759 passed, `-p viewer-core` 298 passed; rustfmt on my
four files 0; clippy `-D warnings` conformance 0; pdf-model and viewer-core lint clean in my files,
but `-D warnings` stops at siblings' `list_budget.rs`, `image.rs`, `variable_text.rs`. Census behind
the lock: exit 0, 112 s, peak 0.81 GiB.
