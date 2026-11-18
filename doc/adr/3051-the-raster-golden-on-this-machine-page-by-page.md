# 3051 — The raster golden's moved pages on this machine, page by page, after `fonts-noto-cjk`

Session 2088. Status: **accepted.** Context: `doc/questions/A879`, ADR 1016 (the golden), ADRs 2812 and 2818 (the
audit round 2000's readings), `crates/pdf-model/tests/raster_golden.rs`, `crates/pdf-model/tests/corpus.rs`,
`crates/pdf-font/src/substitute.rs`, `crates/pdf-font/src/loading.rs`. Clauses: ISO 32000-2 §7.10.4, §9.6.4
(Table 111), §9.7.4.2, §9.10.2, §12.5.6.10 (Table 182), §12.7.4.3.

## 1. The premise, and what held of it

A879 said: once a Japanese face is installed, a golden page that still moves is a real finding. The owner installed
`fonts-noto-cjk` on 2026-11-17, and both walks still failed as before: `raster_golden` held 951 and moved 23,
`corpus` failed its incomplete-drawing ratchet with `issue15053.pdf` and `mixedfonts.pdf` joined.

The premise that all 23 were font pages did not hold. Seven of them move for no reason a font gives. The method
that separated them is a dump of each first page's display list and reports (the golden's own `interpret`, digested
by the golden's SHA-256 over the `Debug` text), taken five ways: HEAD as the machine stands; HEAD with a standalone
copy of face 0 of `NotoSansCJK-Regular.ttc` in the user's own font directory; HEAD with `no_machine_fonts()`; a build of
`82b0b11d` (main before the audit merge, 2026-11-11) on this machine; and a build of `44e42f63` (the last commit
that wrote the golden on the owner's notebook, 2026-10-28) on this machine.

## 2. The finding behind sixteen pages: the installed face is never read

`fonts-noto-cjk` installs four `.ttc` collections and no single-font file. `substitute.rs`'s catalogue admits
`ttc` and `otc` by extension, but `covers`, `qualifying` and the loader open every file with
`skrifa::FontRef::new`, which read-fonts documents as refusing a collection ("The data must be a single font (not a
font collection)"; `FontRef::from_index` is the reader for one). So no Noto CJK face qualifies, and this machine's
only Japanese face is still `DroidSansFallbackFull.ttf`. With face 0 extracted into a standalone OpenType file, the
reports of `issue15053.pdf` and `mixedfonts.pdf` become the golden's own (the notebook's) and both draw completely.

The second defect is what makes those two incomplete rather than merely different. Both show **Latin** text through
a CID font of the Adobe-Japan1 collection (`issue15053`: "This Message can be toggled." in `KozMinPr6N-Regular`,
`UniJIS-UTF16-H`; `mixedfonts`: `a`–`z`, `A`–`Z`, `0`–`9` in `ArialUnicodeMS`, `UniJIS-UTF16-H`).
`loading.rs::composite_substitute` asks `installed_covering_styled` for the collection's **script sample** — the
characters §9.7.4.2's ordering implies — and not for the characters the page shows. Droid Sans Fallback covers the
Japanese sample and has no Basic Latin (`fc-list ':charset=41'` lists no Droid file), so it wins and draws none of
the 29 and 74 characters, while DejaVu, Nimbus and Noto Mono, each of which covers them, are never asked. §9.10.2
makes the substitution a processor's choice; drawing nothing where a face on the machine draws every character is
not a choice the clause offers. Both defects are in `pdf-font`, which round 2087 owns; neither is fixed here.

## 3. The page-by-page table

`machine` means: the list is the same at `44e42f63` and at HEAD on this machine, and differs from the line
`44e42f63` wrote on the notebook, so no code change since lies under it and the whole move is the faces.
`(b)` means a merged round moved the page on purpose and left its golden line unrestated.

| page | layer | cause | class | done |
|---|---|---|---|---|
| `90ms_rksj_h_sample` | raster, list | `HeiseiMin-W3` (`90ms-RKSJ-H`) drawn from Droid; the face is a machine fact | machine, §2 | held |
| `issue11555` | raster, list | `KozMinPro6N-Regular` (`90ms-RKSJ-V`) from Droid | machine, §2 | held |
| `issue13343` | raster, list | `Ryumin-Light` (`90ms-RKSJ-H`) from Droid | machine, §2 | held |
| `issue15053` | raster, list, reports | §2's two defects: draws none of 29 Latin characters | (c), left to 2087 | held |
| `issue19182` | raster, list | `PMingLiU` (`UniCNS-UTF16-H`) and two unembedded Arial | machine, §2 | held |
| `issue2128r` | raster, list | an unembedded GBK `CID TrueType` | machine, §2 | held |
| `issue3521` | raster, list | four unembedded `GBKp-EUC-H` fonts | machine, §2 | held |
| `issue6286` | raster, list | `KozMinProVI-Regular` (`UniJIS-UCS2-H`) | machine, §2 | held |
| `issue8372` | raster, list | `AdobeHeitiStd-Regular` (`UniGB-UTF16-H`) | machine, §2 | held |
| `mixedfonts` | raster, list, reports | §2's two defects: draws none of 74 Latin characters | (c), left to 2087 | held |
| `noembed-eucjp` | raster, list | `MS-Gothic` (`EUC-H`) | machine, §2 | held |
| `noembed-identity-2` | raster, list | `MS-PGothic` (`Identity-H`) | machine, §2 | held |
| `noembed-identity` | raster, list | `Ryumin-Light` (`Identity-H`) | machine, §2 | held |
| `noembed-jis7` | raster, list | `Ryumin-Light` (`H`) | machine, §2 | held |
| `noembed-sjis` | raster, list | `MS-Gothic` (`90ms-RKSJ-H`) | machine, §2 | held |
| `issue11150_reduced` | raster, list | an embedded `/Symbol` with no `/Widths` laid out by Symbol's published metrics (§9.6.2.1), ADR 2687 (593a8e2d), which names this page | (b) | restated |
| `annotation-underline-without-appearance` | raster, list | Table 182's bar centred on the stated bottom edge, ADR 2818 §4 (79d0c6c4) | (b) | restated |
| `annotation-squiggly-without-appearance` | raster, list | the same quadrilateral reading, ADR 2818 §4 | (b) | restated |
| `bug1538111` | raster, list | four text markups, ADR 2818 §4, which names this page | (b) | restated |
| `issue14565` | raster, list | an underline at its quadrilateral's edge, ADR 2818 §4, which names this page | (b) | restated |
| `issue19389` | raster, list, reports | a regenerated widget's `/DA` font is read from the stream's own `/Resources` before `/DR` (§12.7.4.3), ADR 2818 §5; the "/DR does not define /F1" report goes | (b) | restated |
| `issue19634` | raster, list | a `d1` glyph's `gs` sets no soft mask (Table 111), ADR 2812 §3, which names this page | (b) | restated |
| `bug1703683_page2_reduced` | raster, list | §7.10.4's degenerate first interval: `/Bounds [0.0 …]` equal to `Domain 0` gives x′ = `Encode 0` of the first function (79d0c6c4's stitching change); the first stop's grey is now function 432's at 0, not 433's. Its 43 `/AntiAlias` are all `false`, so ADR 3024 (round 2083) leaves it alone | (b) | restated |

The fifteen font pages are not restated. Restating them now would hold Droid Sans Fallback's drawing, which is
Q879's rejected option (c), and §2's first defect would then move them all again the day it is fixed. The round
that fixes it restates them, by ADR 1016's update, from this machine's Noto CJK.

## 4. Why the font pages are trusted as machine and the others are not

The two builds settle what one could not. At `44e42f63` on this machine the eight `(b)` pages reproduce the
notebook's list and reports digests exactly, so this machine draws them as the notebook did and only a later
commit moved them: seven at `79d0c6c4` (they also match at `82b0b11d`), and `issue11150_reduced` between the two
builds, at `593a8e2d`. It had been read as a font page since round 1941's first run here on 2026-11-05, because
rounds on this machine kept the sixteen notebook lines while restating the rest; that is how a deliberate change
went unnamed for a fortnight. The fifteen font pages, by contrast, are byte-identical in their lists from
`44e42f63` to HEAD here, and every one changes when the Noto face is offered,
so the faces are the whole of their move.

## 5. Pages a font with no descriptor reaches

Round 2087 (ADRs 3046 to 3048) gives a descriptor-less substituted font's unnamed or out-of-range codes the drawn
face's advance instead of 0. Five of the 23 hold such a font: `issue19389` (`Helvetica`), restated here, and four
held font pages, `90ms_rksj_h_sample` (`Helvetica`, `HeiseiMin-W3`), `issue15053` and `issue19182` (`Helvetica`,
`ZapfDingbats`) and `mixedfonts` (`Helvetica`, `Helvetica-Bold`, `Times-Roman`). Whether a code of theirs takes
that path is for the merge's golden run to show; `issue19389`'s line is the one to re-check.
