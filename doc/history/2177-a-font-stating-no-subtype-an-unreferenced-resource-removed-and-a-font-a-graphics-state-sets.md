# 2177 — A font stating no subtype, an unreferenced resource removed, and a font a graphics state sets

Session 2177, an archive round in the `lane3` worktree: session 2173's veraPDF failures that are this
tree's, in its order. ADRs 3409 to 3412; no question. Ledger rows 7.8.3, 8.4.5, 9.6.2.1 and 9.9
(`implemented` before and after).

**`T02-04-01_005` (ADR 3409).** No row read a font stating no `/Subtype`, and
`font-programs-embedded` skipped it, so the part 4 output kept it unembedded; veraPDF fails it on
6.2.2-3. New row `fonts/font-dictionary-states-its-subtype` (Tables 109, 110, 115, 119) and
`Rewrite::FontSubtypeStated`: the kind the embedded program's Table 124 filing decides, or the kind
of the face the substitution embeds, under this viewer's layout; refused by name otherwise. Converts
at 2b and 4 under `as-if-printed` (`/Subtype /Type1`), refused under `as-shown` on the layout loss;
veraPDF passes both outputs.

**`PDFBOX-4532-reduced` (ADR 3410, ADR 3394's remedy).** `Judgement::withdrawn` counts what section
6.2.2's exemption withdrew, `Exempt::removable_entries` names the cut entries no content stream
names anywhere, and `Rewrite::UnreferencedResourceRemoved` removes them where a finding was
withdrawn: `/MC0` goes with its metadata; all four outputs pass veraPDF.

**`extgstate` (ADR 3411).** The survey now records a font a `gs` sets through Table 57's `/Font`, so
`font-programs-embedded` binds it and the substitution embeds a face; all four outputs pass veraPDF.

**Left (ADR 3412).** `issue7901` (a `CIDToGIDMap` on a `CIDFontType0`, two readings; re-key the
program), `bug920426` (`beginbfchar` in an Encoding CMap; restate as `cidchar`), the empty CFF reals
(`bug1068432`, `bug1308536`; restate the DICT), `bug866395` (undetermined), and the exemption
reaching past a form stating no `/Resources` (`issue6541`).

**Census.** With and without the pack, one line moves: corpora `as-shown` refuses one more at 2b
and at 4 (412 and 432 converted), `T02-04-01_005` on the layout loss; every other kept output is
2173's, path for path. A first census refused five more (a catalog and a page stating `/Type /Font`,
`/Type7`, `poppler-742-0-fuzzed`'s `/Subype`, and `issue6541`, whose removed entries a form stating
no `/Resources` drew): the row reads fonts by position, an undrawn standard font is given `Type1`, a
photographed page defers to the re-check, and the removal reads every content stream's names.
veraPDF: 5516 written as conforming, 79 failing outputs of 23 documents over 18 rules (was 87, 26,
21), 20 unfinished; only `T02-04-01_005`, `PDFBOX-4532-reduced` and `extgstate` left the list.

**Gates.** Over `-p pdf-archive -p pdf-transform -p viewer-qt`: fmt 0; clippy 0; `pdf-sandbox`
build 0; nextest 1719 passed, 0 failed (the manifest lists none; difference none); rustdoc 0;
`fuzz` check 0; conformance 478 passed, 0 failed.
