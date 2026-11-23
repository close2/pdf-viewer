# 2193 — ASCII data cut before its marker written as the bytes decoded

Session 2193, an archive round in the `lane3` worktree: ADR 3462 built, and two documents' stops
named. ADRs 3473 to 3475. Ledger rows 7.4.2 and 7.4.3 (`implemented`), before and after.

**ADR 3462 built (ADR 3473).** `pdf-archive`'s row
`file-structure/ascii-data-ends-at-its-end-of-data-marker`, at section 5.1 of both parts through
§7.4.2 and §7.4.3, fails a stream whose `ASCIIHexDecode` or `ASCII85Decode` stage reports
`Damage::Truncated`; `ascii_stage_cut_short` is the one question both crates ask. The converter
writes each such stream as the bytes decoded, re-encoded by `LZWDecode`'s construction, now shared
as `reencoded_through_flate`, under the loss `ascii-data-as-decoded`, which `as-shown`,
`as-if-printed` and `state-every-loss` grant; the report names each stream, its stage and its
length. `PDFBOX-3148-2-fuzzed` converts at 2b and 4 under both profiles with two `A85` streams
restated; veraPDF passes all four, and page 1 renders identical to the source. The first census run
found `xobject-image` refused at the new row over `/DecodeParms [ ]`; a short array now reads as
no parameters, as `pdf_syntax` reads it, and its four outputs pass and render identical.

**020747 (ADR 3474).** The bound is `MAX_DASHES`, 500 000 dashes a path, on page 7, whose stream
fails its own Adler-32: periods read as digits make four dashed paths 0.6 to 1.2 × 10⁹ units long
on a 592 by 807 page, 1.1 to 2.1 × 10⁸ dashes each. The page's; the refusal stays. The bound counts
length no pixel shows, which is `pdf-render`'s finding to take.

**issue13316_reduced (ADR 3475).** R11 restates over the face the substitution embeds, and the
substitution stops at ADR 2773 on five `/Differences` names no face reaches (ADR 2880). Refused by
name; the build is ADR 3125's empty glyph for a program the window refused.

**Census.** With and without the pack, every line is 2190's; ranking sections 2 and 9 regenerate
unchanged. The kept outputs are 2190's path for path; sizes change only where the row restated a
stream (`PDFBOX-3148-2-fuzzed`, `xobject-image`, and under `as-if-printed` `210260`,
`poppler-395-0-fuzzed`, `poppler-90-0-fuzzed`). veraPDF: 5520 written as conforming, 47 failing
outputs of 15 documents over 14 rules, 20 unfinished; the list is 2190's less `PDFBOX-3148-2-fuzzed`.

**Gates.** Over `-p pdf-archive -p pdf-transform -p viewer-qt`: fmt 0; clippy 0; `pdf-sandbox`
build 0; nextest 1738 passed, 0 failed (manifest difference none); rustdoc 0; `fuzz` check 0;
conformance 478 passed, 0 failed.
