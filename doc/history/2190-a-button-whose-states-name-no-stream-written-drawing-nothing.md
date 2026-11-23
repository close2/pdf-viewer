# 2190 — A button whose states name no stream written drawing nothing, and what the rows leave

Session 2190, an archive round in the `lane3` worktree: the ranking's `builds only` rows that still
stop documents, most documents first, and round 2186's two leftovers. ADRs 3461 to 3463; Q1011.
Ledger rows 12.5.2 and 12.5.5 (`departed`), 12.7.5.2.3 and 9.7.4.2 (`implemented`), before and after.

**`issue15557` (ADR 3461), R16.** The one `builds only` stop under both shipped profiles: a check box
whose `/N` maps `/Yes` and `/Off` to the name `null` and states no `/AS`. §12.5.5's subdictionary
"shall define multiple appearance streams"; this one defines none, so every state draws nothing.
`sites::appearance_states` writes each state the file names as one form drawing nothing, leaves the
`/AS` absent as stated (A48), and the shape row answers the loss `field-appearances` per document.
All four outputs pass veraPDF; page 1 renders identical.

**What the rows leave (ADR 3463).** Under `as-shown`: five tagged documents whose forbidden
annotations' marks would need a structure type only the producer knew (R15, R16; Q1011 asks for
A707's answer applied to an appended page); `poppler-395-0-fuzzed`'s `stand-in-face`; R14's two,
each the file's own content (`241573_ftp`'s packet states no filter over deflate bytes;
`balloon_a1b_jp2k`'s packet holds a NUL where its byte-order mark goes, and the step that leaves a
control off the page is `as-if-printed`'s); R09's twelve `unauthorised` and `issue13316_reduced`,
which R11 refuses first.

**Leftovers.** `cid_keyed_by_map.rs` builds a three-glyph name-keyed CFF under a `CIDFontType0`
whose `/CIDToGIDMap` swaps CIDs 1 and 2, and holds `CidKeyedProgramWritten` end to end at 2b and 4:
mechanical, conforming, the map gone, `CIDFontType0C`, each CID the outline and width the map chose.
`PDFBOX-3148-2-fuzzed` (ADR 3462): a remedy, decided and not built — a section 5.1 row for §7.4.3's
end-of-data marker, the stream restated as the bytes decoded, under a loss of its own word,
`ascii-data-as-decoded`.

**Census.** With and without the pack, `issue15557` moves from refused to converted on each of
pdf.js's six lines (`as-if-printed` 2b 943, 4 957; `as-shown` 2b 909, 4 922, with the pack) and
nothing else moves; the kept outputs are 2186's path for path and size for size, and four more.
veraPDF: 5520 written as conforming, 51 failing outputs of 16 documents over 16 rules, 20
unfinished, the list 2186's; `issue15557`'s four pass.

**Gates.** Over `-p pdf-archive -p pdf-transform -p viewer-qt`: fmt 0; clippy 0; `pdf-sandbox`
build 0; nextest 1732 passed, 0 failed (manifest difference none); rustdoc 0; `fuzz` check 0;
conformance 478 passed, 0 failed.
