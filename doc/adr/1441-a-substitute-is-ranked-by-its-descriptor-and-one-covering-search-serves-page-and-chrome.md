# 1441 — A substitute is ranked by its descriptor, and one covering search serves the page and the chrome

Session 1303. Status: **accepted**.
Context: `crates/pdf-font/src/substitute.rs` (`Style`, `FaceStyle`, `find_styled`,
`installed_styled`, `installed_covering_styled`, `family_members`, `best_covering`,
`weight_from_stem`), `crates/pdf-font/src/provider.rs` (the description's version 2),
`crates/pdf-font/src/substituted.rs` (`substitute_face`), `crates/pdf-font/src/loading.rs`
(`composite_substitute`, `class_faces`, `LoadedFont::substitute_program`),
`crates/viewer-host/src/machine_faces.rs`, `crates/pdf-model/examples/substitution_census.rs`.
Builds: ADRs 0007, 0086, 0133, 0152, 0358, 1154, 1430. Supersedes ADR 1430's section 1 ranking
(style before repertoire) for the one search both callers now share.
Clauses: ISO 32000-2 §9.8.1 (Table 120), §9.8.2 (Table 121), §9.6.2.2.

## 1. The clause, and three premises the brief had wrong

§9.8.1 gives a descriptor its role: "These font metrics provide information that enables a PDF
processor to synthesise a substitute font or select a similar font when the font program is
unavailable." §9.5's NOTE 5 leaves "font substitution" implementation-dependent, so what follows is
a reading of what "similar" can be measured by, and every ranking below is a documented choice.
The brief named §9.6.6.4 (ISO 32000-1's TrueType encodings; ISO 32000-2 has no such subclause),
Table 122 for the common entries (they are Table 120; 122 is the CIDFont additions), and Table 111
for the standard 14's aliases (Table 111 is the Type 3 operators; the standard states no alias
table — the clones are `names_a_standard_font`'s own list).

## 2. What the search did, measured

Before: `Request::derive` read the name, PANOSE and `/Flags` into a family and a bold/italic pair
(`/FontWeight` ≥ 600, ForceBold); the machine's faces were matched by file-name suffix, so no light,
semibold, black or narrow member was ever reachable, `/StemV` and `/FontStretch` were read by
nothing, and the covering search's catalogue stage ranked by repertoire alone.
`substitution_census` over `doc/pdf.js/test/pdfs` and `doc/corpora` (1459 documents, 3216
substituted fonts, 867 with a descriptor, 2789 in a compiled-in face): weight more than one class
off 11 (2 where `/FontWeight` is stated), slope 3, spacing 30, serif-ness 44, any 68. Every one is
either the machine lacking the face (`HelveticaNeue-Heavy` 900 in Nimbus Sans Bold; five Mincho and
Song fonts in the only CJK face installed, Droid Sans Fallback) or the descriptor contradicting its
own name (Courier New with no FixedPitch bit, Times New Roman with no Serif bit, `Arial-ItalicMT`
with a zero angle), where the name keeps deciding. After, the counts are unchanged and six fonts
change face: every non-embedded Arial Narrow goes from Nimbus Sans to Nimbus Sans Narrow.

## 3. The rule

- **`Style`** — weight, slope, width — is derived from the document: the weight is the first of a
  weight word in the name (one of §9.6.2.2's fourteen states its weight by its name), `/FontWeight`
  read as the nearest of Table 120's nine hundreds, PANOSE's bold, and `/StemV`; ForceBold makes it
  at least 700. `Request.bold` is now `Style::is_bold`, so the compiled-in fallback and the machine
  agree. Width is `/FontStretch`, then a width word (`Narrow`) in the name.
- **`/StemV` to weight is this tree's**: a stem of 120 thousandths or more is 700, above zero and
  below it 400, zero is Table 120's unknown. The line sits between the regular and bold stems of
  the standard Latin families; the census shows producers write bold-named stems mostly 121–212 and
  plain ones mostly 40–109, and **no corpus font is decided by the rule** — a name word, a
  `/FontWeight` or PANOSE always spoke first.
- **Within a preferred family**, its members (a file name that adds only style words) are ranked by
  their own `OS/2` weight class, slope and width class, nearest first; family stays the outer loop.
- **The covering search is one**, asked by a page's composite font and by the chrome: repertoire
  first, then weight, slope, generic family (FixedPitch and Serif against `post` and `OS/2`), width.
  ADR 1430 put style first; on this machine that hands a bold Chinese font to
  `NotoTraditionalNushu-Bold`, a Nüshu face stating a handful of Han characters, while every weight
  and width of `NotoSansArabic` states the same 1 250 characters — so repertoire first loses no
  character and still picks `NotoSansArabic-Regular` for the chrome's word.
- **The broker's description carries the style** (version 2, weight and width bytes), so a confined
  worker's family ranking is the unconfined one's.

## 4. What moved

`raster_golden`: one row, `bug1671312_ArialNarrow.pdf` p1, raster and list — drawn in Nimbus Sans
Narrow at stretch 1.0 instead of Nimbus Sans condensed to 0.82. Looked at beside `poppler` and
`mupdf` at 144 and 576 dpi: page ink at 576 dpi 15.45 → 15.69 against `poppler`'s 15.67 and
`mupdf`'s 15.42, modal x-height stem 11 → 12 device pixels, `poppler`'s and `mupdf`'s both 12.
Toward the references. The oracle passes; the page stays `ambiguous` (§9.8.1 states no `shall`).
`pdf-model --test substituted_shapes` asks ADR 0358's control only of a normal-width stand-in, read
off the face's `OS/2`. Cost: the first lookup, catalogue walk included, 2.2 ms; a second family
88 µs; style reads are remembered per file and faces not chosen are not held.

## 5. Left

No serif CJK face is installed here, so a Mincho descriptor's Serif flag cannot yet choose one;
`/AvgWidth`, `/CapHeight` and `/XHeight` are still evidence nothing weighs (ADR 0267).
