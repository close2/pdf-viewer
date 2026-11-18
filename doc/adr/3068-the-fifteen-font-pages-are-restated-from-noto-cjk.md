# 3068 — The golden's fifteen held font pages are restated from this machine's Noto CJK

Session 2091. Status: **accepted.** Context: ADR 1016 (the golden and its update), ADR 3051 §3 (the fifteen held
pages), ADRs 3066 and 3067 (the two defects, fixed), `crates/pdf-model/tests/raster_golden.tsv`. Clauses: ISO
32000-2 §9.7.4.2, §9.7.5, §9.10.2.

## Decision

The fifteen pages ADR 3051 held because the face under them was Droid Sans Fallback are restated by ADR 1016's
update, each line taken from one update run of this tree on this machine (`PDFVIEWER_RASTER_GOLDEN=update`) and
nothing else of that run kept. §9.10.2 leaves the substitute face to the processor; what the clauses ask of it is
that the shown characters be drawn from a face that draws them, and every one of these pages now is: the two that
the corpus ratchet listed (`issue15053`, `mixedfonts`) draw whole, and their reports equal the lines the golden
already held. The face named is the one the substitution chose for each font, read from its `name` table.

| page | font (encoding) | face that now draws it |
|---|---|---|
| `90ms_rksj_h_sample` | `HeiseiMin-W3` (`90ms-RKSJ-H`) | Noto Sans CJK JP |
| `issue11555` | `KozMinPro6N-Regular` (`90ms-RKSJ-V`) | Noto Sans CJK JP |
| `issue13343` | `Ryumin-Light` (`90ms-RKSJ-H`) | Noto Sans CJK JP |
| `issue15053` | `KozMinPr6N-Regular` (`UniJIS-UTF16-H`) | Noto Sans CJK JP, its Latin its own |
| `issue19182` | `PMingLiU` (`UniCNS-UTF16-H`) | Noto Sans CJK JP |
| `issue2128r` | an unembedded GBK `CID TrueType`, bold | Noto Sans CJK JP Bold |
| `issue3521` | four `GBKp-EUC-H` fonts | Noto Sans CJK JP and Noto Sans CJK JP Bold |
| `issue6286` | `KozMinProVI-Regular` (`UniJIS-UCS2-H`) | Noto Sans CJK JP |
| `issue8372` | `AdobeHeitiStd-Regular` (`UniGB-UTF16-H`) | Noto Sans CJK JP |
| `mixedfonts` | `ArialUnicodeMS` (`UniJIS-UTF16-H`) | Noto Sans CJK JP, its Latin its own |
| `noembed-eucjp` | `MS-Gothic` (`EUC-H`) | Noto Sans CJK JP |
| `noembed-identity-2` | `MS-PGothic` (`Identity-H`) | Noto Sans CJK JP |
| `noembed-identity` | `Ryumin-Light` (`Identity-H`) | Noto Sans CJK JP |
| `noembed-jis7` | `Ryumin-Light` (`H`) | Noto Sans CJK JP |
| `noembed-sjis` | `MS-Gothic` (`90ms-RKSJ-H`) | Noto Sans CJK JP |

## What these lines hold, said plainly

- **The JP face for Chinese.** Noto's region faces map the same characters and state the same style and code pages,
  so ADR 3066's tie-break by index gives face 0, the JP face, to `Adobe-GB1` and `Adobe-CNS1` fonts too; their
  ideographs are drawn in Japanese forms. A region chosen by §9.7.3's ordering would move `issue19182`,
  `issue2128r`, `issue3521` and `issue8372` again, by design.
- **Sans for Mincho.** `HeiseiMin`, `KozMin` and `Ryumin` are serifed designs, and the ranking puts the wider
  repertoire before the generic family (ADR 1441), so Noto Sans CJK, which maps more characters than Noto Serif
  CJK, draws them.

## Not restated

The same run moved 63 other pages, and none is this change's. Fifty-seven moved identically in the post-merge
train's run at 66c33750 (`postmerge/20261118-1626-66c33750.log`), before this change and before round 2087's merge:
list-only moves on shading and pattern pages and raster moves on `issue17065` and `issue18529`. The other six
arrived with round 2087's descriptor-less advances (fb3699c3): five move in their reports only, which carry that
round's wording (`issue18099_reduced`, `issue6069`, `issue6106`, `issue6108`, `issue2391-2`), and
`poppler-90-0-fuzzed` holds only standard Type 1 and Type 3 fonts, which no machine collection reaches. Each is for
its owner to read by ADR 3051 §1's method before its line is restated. `issue19389`'s line (ADR 3051 §5) holds.
