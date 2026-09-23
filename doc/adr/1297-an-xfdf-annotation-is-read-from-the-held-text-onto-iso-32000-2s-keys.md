# 1297 — An XFDF annotation is read from the held text, onto ISO 32000-2's keys

Status: **accepted**, built.
Context: `crates/pdf-model/src/xfdf.rs`, `crates/pdf-model/src/xfdf/annotations.rs`,
`crates/pdf-model/src/forms_data.rs` (`FdfAnnotation::popup`, `FdfAnnotation::parent`,
`read_annotations`), `crates/pdf-model/src/view.rs` (`ViewState::place_file_annotations`,
`ViewState::link_reply`, `ViewState::write_additions`), `crates/pdf-model/tests/xfdf_annotations.rs`
and its fixtures under `crates/pdf-model/tests/xfdf/`.
Builds: ADR 1108 (the field half of XFDF), ADR 1223 (a value that crosses is copied), ADR 1224 (an
FDF annotation is an annotation), ADR 0187 (the specifications leave the repository).
Owner's answer: `doc/questions/A97`.
Clauses: ISO 32000-2 §12.7.8.3.4 (Table 254), §12.7.8.3.1 (Table 246), §12.5.2 (Table 166),
§12.5.3 (Table 167), §12.5.4 (Tables 168 and 169), §12.5.6.2 (Table 172), §12.5.6.4 to
§12.5.6.24 (Tables 175 to 195), §7.11.4 (Tables 44 and 45), §13.3 (Table 305), §7.3.8.1.

`§` is ISO 32000-2 alone. The XFDF text — Adobe's *XML Forms Data Format Specification* 3.0,
August 2009, `doc/XFDF_Spec_3.0.pdf` — numbers only its chapters, so it is cited as *chapter,
heading, page*, and paraphrased: it is licensed for reading and nothing of it is quoted here.

## 1. Which text decides what

The XFDF text supplies the **spelling**: which element is which subtype (chapter 2, *Annotation
Elements*, pages 37 to 54), which attribute or child is which key (*Annotation Subelements*, pages
55 to 68; *Annotation attributes*, pages 69 to 90; *Mapping Tables*, pages 91 to 99). Its own
introduction (chapter 1, page 17) sends the reader to the PDF reference for every key's meaning.
So **the key, its type and every rule about it are ISO 32000-2's**, and where the two disagree ISO
32000-2 wins. Each place it does is listed in section 4.

## 2. One application, not two

`xfdf::read` fills `FormsData::annotations` with the same `FdfAnnotation` an FDF file's `/Annots`
is read into, and `ViewState::import` places both. An XFDF annotation's dictionary is built in
the **FDF form** — Table 254's `/Page` beside the rest, Table 172's `/IRT` as the text string of
the replied-to `/NM`, which that table requires of an FDF file and the XFDF text states for its
`inreplyto` in the same terms (page 72) — so the one import turns both formats' FDF forms into
what a PDF file states.

Three things the import did not do before, which both formats needed:

- **`/Popup` and `/Parent` cross as positions**, `FdfAnnotation::popup` and `::parent`, and are
  written as references to the objects the import allocates. Table 172 and Table 186 make each an
  indirect reference, and in an FDF file the two name each other: `forms_data::carry` followed the
  pair until its depth bound refused the whole annotation, so an FDF annotation with a popup was
  never placed. A popup whose parent is refused is refused with it.
- **`/IRT` becomes a reference** to the annotation its name finds on the same page — this
  import's, then what was added before, then the page's own `/Annots` — which Table 172's "[b]oth
  annotations shall be on the same page of the document" bounds. Found nowhere, it is refused by
  name and `/RT` with it.
- **Every stream in a placed annotation becomes an indirect object** at save, not only `/AP`'s:
  Table 187's embedded file and Table 188's sound object are streams, and §7.3.8.1 allows a stream
  nowhere else.

## 3. The mapping

Every element of *Annotation Elements* reads Table 166's common entries (`rect` `/Rect`, `color`
`/C` from `#RRGGBB` over 255, `date` `/M`, `flags` `/F` by Table 167's first nine bits, `name`
`/NM`, `contents` `/Contents`) and, for Table 171's markup subtypes, Table 172's (`title` `/T`,
`creationdate`, `subject` `/Subj`, `intent` `/IT`, `inreplyto` `/IRT`, `replyType` `/RT` as `R` or
`Group`, `contents-richtext` `/RC`, `popup`) plus `opacity` `/CA`, now Table 166's. Per subtype:
`text` Table 175 (`icon` `/Name`, `state`, `statemodel`); the four text markups Table 182
(`coords` `/QuadPoints`); `line` Table 178 (`start`+`end` `/L`, `head`+`tail` `/LE`,
`interior-color` `/IC`, `leaderLength` `/LL`, `leaderExtend` `/LLE`, `leader-offset` `/LLO`,
`caption` `/Cap`, `caption-style` `/CP`, the two `caption-offset`s `/CO`); `circle`, `square`
Table 180 (`/IC`, `fringe` `/RD`); `caret` Table 183 (`/RD`, `symbol` `/Sy`); `polygon`,
`polyline` Table 181 (`vertices` `/Vertices`, `/IC`, `/LE`, `/IT`); `stamp` Table 184 (`/Name`);
`ink` Table 185 (`gesture`s `/InkList`); `freetext` Table 177 (`defaultappearance` `/DA`,
`defaultstyle` `/DS`, `justification` `/Q`, the legacy `border` as `/Border` and `/BS`);
`fileattachment` Table 187 (`/FS` with `/F`, `/UF` and `/EF`, the `data` as Table 44's stream with
Table 45's `size`, `creation`, `modification`, `checksum`); `sound` Table 188 (`/Sound`, Table
305's `rate` `/R`, `bits` `/B`, `channels` `/C`, `encoding` `/E`); `redact` Table 195
(`/QuadPoints`, `/IC`, `overlay-text`, `overlay-text-repeat` `/Repeat`, `/Q`, `/DA`); `projection`
§12.5.6.24 (Tables 166 and 172 only); `width`, `dashes`, `style` Table 168's `/BS`; `style="cloudy"`
and `intensity` Table 169's `/BE`; a child `popup` Table 186 (`open` `/Open`, and its own `/T`,
which Table 186's override sentence presumes).

`contents-richtext` holding markup is `/RC`, taken as the markup the file wrote; holding plain text
it is `/Contents` (chapter 1, page 30) unless `<contents>` states that itself. `data` is the
stream's stored bytes in the text's two pairings of `mode` and `encoding` (page 31): the text's
`length` and `filter` are the stream's own `/Length` and `/Filter` (pages 86 and 87), so
`filtered` is read as the escaping of an ASCII rendering and not as decoded data, and a length
that disagrees with what was decoded is refused. Byte strings (`/F` of the file specification,
`/DA`, `/CheckSum`) follow the text's Latin-1 convention with its octal escapes (page 28).

## 4. Where ISO 32000-2 overrides the text

| The text | ISO 32000-2 | Here |
|---|---|---|
| the polyline subtype spelled `Polyline` (page 94) | Table 171: `PolyLine` | `PolyLine` |
| `polygon-dimension`, `polyline-dimension` (page 79) | Table 181: `PolygonDimension`, `PolyLineDimension` | Table 181's spelling |
| `state`, `statemodel` are names (page 74) | Table 175: text strings | text strings |
| `rotation` is `/Rotate` on stamp, freetext, projection | no annotation table states `/Rotate` | refused by name |
| `defaultappearance` under `caret` (page 45) | Table 183 states no `/DA` | refused by name |
| border style on `text` (page 84) | Table 175 states no `/BS` | refused by name |
| border effect on `polyline` (page 84) | Table 181: `/BE` meaningful only for a polygon | refused by name |
| `redact` lists no `page` or `rect` (page 54) | Tables 254 and 166 require both | read for every element |
| `encoding="alaw"` (page 82) | Table 305 as held lists `Raw`, `Signed`, `muLaw` | refused by name |
| `link` is supported (page 23) | Table 246 excludes Link from an FDF `/Annots`, which `<annots>` is (page 37) | read, refused at placement as an FDF Link is; `Dest`, `OnActivation`, `BorderStyleAlt` named, not built |

## 5. What is refused because the text does not say enough

- **`appearance`** (page 56): base 64, and the text never says what the decoded bytes are. No
  `/AP` is built from bytes whose format is not stated.
- **`overlayappearance`** (page 65): Table 195's `/RO` is a form XObject; the text gives its
  content model as a text string.
- **`resource`** (page 66): Table 45's `/Mac`, deprecated in PDF 2.0, with none of its entries
  defined by ISO 32000-2.
- **`ex_data`** (appendix A): Table 173's `/ExData`, whose subtypes are clause 13's 3D, excluded.

An annotation missing an entry ISO 32000-2 requires of its subtype — `/Rect`, Table 182's
`/QuadPoints`, Table 178's `/L`, Table 181's `/Vertices`, Table 185's `/InkList`, Table 177's
`/DA`, Table 187's `/FS`, Table 188's `/Sound`, Table 254's `/Page` — is named and not placed. A
conditional entry whose condition is missing (`/State` without `/StateModel`, `/RT` without
`/IRT`, `/LLE` without `/LL`, `/OverlayText` without `/DA`, `/BE /I` without `/S /C`) is named and
left out, and the annotation is placed without it.

## Alternatives

**Build `<link>`'s destinations and actions.** Rejected: Table 246 excludes the subtype from the
array the text maps `<annots>` onto, so nothing built would ever be placed, and a local
destination's page is an ordinal only a placement can resolve.

**A second import path for XFDF annotations.** Rejected for ADR 1108's reason about fields: two
paths are two chances to disagree about where an annotation goes and what it replies to.

**Decode `<appearance>` as the XML Acrobat is known to write there.** Rejected: that is what
another program does, not what the text states — principle 5.
