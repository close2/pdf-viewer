# PDF/A: what still blocks a document, and what a company would do with it

Status: **a census record**, taken at `7d6ed87f` on 2026-10-27 by session 1828 for the owner's
`doc/questions/A681` item 1, and re-run by the commands in section 1. Section 1's census lines and
section 2's table are session 1939's run, section 2 being `rank.py --table`'s output; the figures in
sections 3 to 9 are session 1828's, each entry's built parts named where a round built them, and
none is kept current by hand. ADR 2490 is the decision that reopens the blocks; this file
is the list it reopens them over. It does not repeat `doc/pdf-a-conversion-limits.md` or
`doc/pdf-a-mitigations.md`: each remedy points at their entry. ISO 19005 is cited by section and
paraphrased (`doc/third-party-data.md`); quotation marks are ISO 32000-2's own words.

## 0. The question, and how to read an entry

The owner's question: *what would a company do if it received such a document by mail and needed to
archive it?* — and the owner's rule that if we can see the document we will usually find a way to
archive it, with genuine exceptions such as an embedded office file. A company in that position does
one of three things: it keeps the file as it came, it prints it and files the paper, or it scans the
print. The two shipped profiles are the first two of those (`as-shown`: keep what the window showed;
`as-if-printed`: keep what a print would have carried), and ADR 2394's photographed page is the third.
So every remedy below names which of the three it is, the loss it takes, and the profile it falls
under. None invents a mark: each test is CLAUDE.md's — does the operation invent marks?

- **documents/alone**: documents a reason (or remedy) stops, and of those the ones it is the *only*
  stop of, so the ones building it alone would free.
- A refusal can hide one behind it: a document is counted under every reason its conversion
  reported, and a reason only a remedy would reach is not in the log. "Frees" is therefore an upper
  bound the remedy's own round measures.
- Real producers' files only: pdf.js's test documents and `doc/corpora`, 1478 files. The veraPDF
  corpus (files built to fail one clause each) is not ranked here; its census is
  `the_shipped_profiles_list_every_refusal_by_site_and_reason`.

## 1. The census, and the commands that re-count it

In an export of HEAD, once without and once with a copy of the CJK pack (`QUORRA_CJK_PACK`, the
census's first line says which, ADR 2460):

```sh
ulimit -u 8192; tools/bounded.sh --lock --round <n> --tree 6 --build '--profile gates -p pdf-sandbox --bins' -- \
  cargo test --profile gates -p pdf-transform --test archive_corpus \
  the_shipped_profiles_list_what_real_producers_files_refuse_on -- --ignored --nocapture --exact > census.log
python3 -I -B doc/pdfa-blocking-issues/rank.py census.log --repository . \
  --render <target>/gates/quorra-transform > rank.txt
```

`rank.py` prints the census lines, every reason with documents/alone per line, every remedy of this
file with documents/alone per profile and target, the order in which building them frees the most,
and the documents behind each remedy (section 9 below is that last table). The attachment census of
section 6 is `quorra-transform attachments <file> --list` over the same 1478 files.

With `QUORRA_CENSUS_OUTPUTS=<empty directory>` the census keeps every output it writes, and
`python3 -I -B tools/verapdf-census.py <directory>` puts each through veraPDF at the flavour it was
written for and prints every rule veraPDF fails on an output this tree wrote as conforming, by rule
and by document (ADR 2652); `tools/state.sh archive` prints its two count lines.
Two disagreements found outside it, to weigh in batch 132 (ADR 2652 section 2):
- 6.2.2: veraPDF reads the run `33600.272.58` as no operator where this tree's fence names it one; §7.3.3 makes it an operator token, so the standard reads for this tree, provisionally.
- 6.2.11.4.1 on `178360`'s `ICLJGJ+Glypha` was no disagreement: the 1009-byte `Type1C` program holds no `0x00` byte where a CFF's header and Name INDEX hold several, its Name INDEX counts 8193 names in the bytes of one, and this tree now fails it as veraPDF does and substitutes it (ADR 2636 decision 5); `178360` passes veraPDF at 2b and 4.

| census line, with the pack | converted | conforming | refused | unreadable | declined as a source |
|---|---|---|---|---|---|
| pdf.js `as-if-printed` 2b | 907 | 2 | 51 | 11 | 5 |
| corpora `as-if-printed` 2b | 429 | 6 | 49 | 16 | 3 |
| pdf.js `as-if-printed` 4 | 917 | 0 | 43 | 11 | 5 |
| corpora `as-if-printed` 4 | 446 | 0 | 39 | 16 | 2 |
| pdf.js `as-shown` 2b | 841 | 2 | 117 | 11 | 5 |
| corpora `as-shown` 2b | 344 | 6 | 133 | 16 | 4 |
| pdf.js `as-shown` 4 | 845 | 0 | 115 | 11 | 5 |
| corpora `as-shown` 4 | 348 | 0 | 135 | 16 | 4 |

In session 1828's run, without the pack, pdf.js `as-if-printed` was eleven documents worse at each
part, which is why the pack is taken as installed. `as-shown` is the same
either way (its `machine-face` stop holds the pack's documents).

Two deep JPEG 2000 shapes that A691 wrote (a deep grey image with opacity; a colour image with an
opacity channel of another depth) fail veraPDF at ISO 19005-2 6.2.8.3 and -4 6.2.7.3 on veraPDF's
reading of the codestream header, not the standard's; the draft of the upstream issue, its two
reproducers and the clauses it cites (cited, not quoted) are indexed here:
[`pdfa-blocking-issues/verapdf-jpeg2000-issue.md`](pdfa-blocking-issues/verapdf-jpeg2000-issue.md),
files under `pdfa-blocking-issues/verapdf-jpeg2000/`.

**Under `as-if-printed`, 120 documents are refused or declined at 2b and 108 at part 4; 91 at
both parts, 137 at one at least. Under `as-shown`, 248 at both, 292 at one at least.** Of the
`as-if-printed` population, everything but a few documents falls under a remedy below; those are
the exceptions of section 7 (four `X1`, six `X2`) and the documents the converter itself fails
(section 8; seven at 2b, six at 4).

## 2. The ranking, and what each row still needs

The table below is the output of

```sh
python3 -I -B doc/pdfa-blocking-issues/rank.py census.log --repository . \
  --render <target>/gates/quorra-transform --table
```

over section 1's census, and is replaced by it whole, never edited by hand: the order is the one in
which building the remedies frees the most documents under `as-if-printed` at 2b, then those only
part 4 reaches; each "frees" is given the rows above it; "what is left needs" and the last column
are `rank.py`'s `STATUS`, the one judgement in it, kept beside the rule that files each reason. A
`built` row leaves what it is meant to leave (R06b's figures are what a request for one part
refuses, which the two-target request of ADR 2498 answers); a `builds only` row needs a build and
no owner's word; no row is `owner-blocked` whole, and the parts that are say so in their last
column. R02 is built (ADR 2498) and no document reaches it.

| order | remedy | frees at 2b | frees at 4 | stops under `as-shown`, 2b / 4 | what is left needs | built by, and what is left |
|---|---|---|---|---|---|---|
| 1 | R06b-the-other-part | 37 | 0 | 42 / 0 | built | ADR 2498: the two-target request; the figures are what a one-part request refuses; a `%PDF-3.x` source is written at 4 (ADR 2692) |
| 2 | R03-photograph | 7 | 10 | 55 / 57 | builds only | built under `as-if-printed`: A694, ADRs 2516, 2618, 2638, 2760; left, a page this reader does not draw whole, by its cause (ADR 2760); `as-shown` keeps ADR 2394's gate (A694 item 1) |
| 3 | R08-unembedded-font-photograph | 8 | 8 | 34 / 34 | builds only | built under `as-if-printed`: A694, ADR 2516; `as-shown` keeps its stop (A694 item 1) |
| 4 | R10-font-refiling | 4 | 4 | 6 / 6 | builds only | built: a cut program (ADR 2636), a Type 1 program's widths (ADR 2685), a standard font's stated widths (ADR 2686), a mode 3 font's descriptor (ADR 2688), a glyf program under a CIDFontType0 (ADR 2716), a collection nothing reads (ADR 2717), a font written inside another object (ADR 2722), a code Annex D encodes twice in a glyf face (ADR 2723) |
| 5 | R11-truetype-route | 5 | 5 | 6 / 6 | builds only | built: A694 item 3, ADR 2520 |
| 6 | R14-appended-page-composition | 3 | 2 | 3 / 3 | builds only | built: ADRs 2678, 2682 |
| 7 | R03a-notdef-named | 2 | 2 | 6 / 3 | builds only | built: ADRs 2543, 2556, 2602, 2620, 2650, 2753; left, a code an embedded CMap reads as invalid in a CJK collection without ToUnicode |
| 8 | R16-annotations-and-forms | 2 | 2 | 14 / 17 | builds only | built: a push button with no appearance dictionary (ADR 2751), an appearance dictionary stating no entry (ADR 2752); left, named per site |
| 9 | R01-recovered-structure | 1 | 1 | 3 / 3 | builds only | built: ADRs 2502, 2503, 2551, 2596, 2670; each left is refused by its own sentence |
| 10 | R07-undecodable-stream | 1 | 2 | 1 / 2 | builds only | built: ADR 2689; the `UnknownFilter` files' `%PDF-3.x` header, ADR 2692; left, an ICC profile written as its alternate |
| 11 | R12-spot-colourant | 1 | 1 | 1 / 1 | builds only | built for a PostScript calculator function: ADR 2690; left, a sampled transform, refused until its loss is named (ADR 2490) |
| 12 | R05-press | 0 | 2 | 0 / 2 | builds only | A694 item 2: (a) built, ADR 2518; (b) built, ADR 2747; (c) built, ADR 2750: overprint mode 1 on a DeviceCMYK mark is ADR 1978's loss under the two shipped profiles; left, a mark blended in four components, ADR 1978's refusal |
| 13 | R15-embedded-files | 0 | 2 | 3 / 7 | builds only | built: ADR 2536 |
| 14 | R09-as-shown-font-losses | 0 | 0 | 26 / 26 | builds only | `as-shown` only; ADR 2490: (c) decided; (b) built for a `glyf` face, ADR 2769, and an OpenType CFF face, ADR 2789; (a) built for a simple font, whole, ADR 2773; left, (a) for a composite font and a subset, and (b) for a bare CFF program |
| 15 | X1-dynamic-xfa | — | — | 4 / 4 | exception | CLAUDE.md's exclusion; §K.1 |
| 16 | X2-no-page-to-see | — | — | 6 / 6 | exception | the reader draws no page of it |

Under `as-shown` the largest rows are R03 and R08, each held by an answer of the owner's (A694 item
1) rather than by a build, then R09 (26 documents at each part, its simple fonts with a descriptor
embedding the machine's face the window drew, ADR 2773, fitted where the window fitted it, ADRs
2769 and 2789).

## 3. Fonts

**R08 — a font the document does not embed and no route of this tree can carry** (39 documents;
alone in 27 at 2b and 25 at 4). Eleven sentences, the largest: a font naming a face outside the 14
with no descriptor and no `/Widths` (10, Q687's — listed, not decided here), an `Adobe-Identity`
font whose page also shows extractable text (5, plus 1 whose `/ToUnicode` names a character no face
draws), a standard-14 font with no descriptor and a code the shipped face advances differently (3),
a composite font with no program named (3), a code the encoding sends to `.notdef` (6, fenced; 8
beside a descriptor). *A company* would print it: the print carries the glyphs the screen drew and
no text layer. **Remedy**: ADR 2394's photographed page, under `as-if-printed`, for *any* page
showing such a font, not only one whose text was never readable — the gate of ADR 2394 item 2 is the
owner's (A570) and is what would be widened; Q694 asks. Loss `text-as-image` for that page; the page's text is
named in the report so a reader knows what was lost. `as-shown` keeps its stop. Q687's question
stays open: a photograph states no widths as the producer's, so it is the route that does not
depend on Q687's answer. Mitigations section 5, `fonts/font-programs-embedded`; limits 2.1, 4.9.

**R09 — the `as-shown` font losses** (26 documents at each part; per part, `machine-face` 13,
with `glyphs-from-to-unicode` 4, with `layout-from-this-viewer` 4, `layout-from-this-viewer` alone 4, a machine's face lacking a shown code 1). `as-if-printed` converts
every one. *A company* that must keep what it saw keeps what its screen showed. **Remedy**: (a) for `machine-face`, embed *the machine's face the
window drew*, where the face's own `fsType` permits: **built for a simple font with a descriptor**
(ADR 2773), whole rather than a subset, the bytes the window loaded, a name-keyed `CFF ` OpenType
face under a `Type1` dictionary (a `TrueType` one restated), held to the window's outline code by
code, and only where the profile withholds the loss (ADR 2774), so `as-if-printed` keeps the shipped
face. It freed 25 documents at 2b and 24 at 4 on this VM, whose drawn faces are URW's Nimbus faces
(`fsType` preview-and-print, the permission §9.9.1 names) and, for composite fonts,
`DroidSansFallback`; a machine with other faces converts other documents. A glyph whose width a
subroutine carries behind a nested call or a hint mask is restated since ADR 2779, which freed 6
more at each part (every glyph of the 35 URW faces is restatable); of the other 19 it reached, 11
convert once their fitted CFF face is written (below) and 8 left for R08 and the rows they already
stood in. Left:
a standard font stated by name alone (`described`) and the composite routes (`rekeyed`, the CJK
pack), which still embed a shipped or pack face over the machine's; a subset, which needs a
compacting CFF writer; (b) for `substitute-stretch`, **built** (ADR 2769): the shipped `glyf` face
goes in with the window's horizontal fitting (ADR 0358) written into its outlines, wherever the
profile withholds the loss; a profile that authorises it keeps A566's unmodified face (ADR 2770);
and **built for a machine's OpenType CFF face** (ADR 2789): each charstring run and stated again
with its x scaled, which freed the 18 documents at each part that stopped on it — 14 convert under
`as-shown` at both parts, and 020747, 064034, 492484 and 504076 leave for the content-stream and
string rows they already stood in; a bare CFF program is still refused, with no witness;
(c) `as-shown` may still authorise `machine-face` by name, which is what reopening its stop means
(ADR 2490). Mitigations section 5; ADRs 0270, 0358, 2162.

**R03a — a code that reaches `.notdef` in the program the file embeds** (30 documents, the fence
rows `fonts/embedded-programs-define-every-glyph-shown` 29 and `fonts/no-notdef-glyph-shown` 23;
alone in 16 and 15). ISO 19005-2 section 6.2.11.8 forbids a reference to `.notdef`; the fence holds
because the code is in the content stream. **Remedy**: leave the content stream as it is and give
the embedded program a glyph for that code whose outline is the program's own `.notdef` outline
(empty, or the box the producer drew): the page draws exactly what this reader drew, and no glyph
shown is `.notdef`. A694 item 3 adopts it. **Built for a simple TrueType program** (ADR 2543,
`Rewrite::NotdefGlyphGiven`): one copy of glyph 0 for each distinct width among the codes on it, the
restated `cmap` mapping each code to its copy; the ten documents of this row whose fonts are simple
TrueType convert in six at 2b and five at part 4. **Built for a `CIDFontType2`** (ADR 2556): the same
copy by CID, one per `/W` width, the `CIDToGIDMap` restated as a stream; `issue10519_reduced.pdf`
converts at both targets under both profiles and passes veraPDF at both. **Built for a shown CID 0**,
which ISO 32000-2 §9.7.4.2 makes the CIDFont's own `.notdef` whatever glyph it is sent to (ADR 2557;
veraPDF agrees), over either program (ADR 2622): `Identity-H` restated as an embedded CMap that sends the
code to a fresh CID drawing CID 0's glyph, `/W` given its width; `4387ba48….pdf`,
`emi15514-sup-0002-SupinfoS2.pdf`, `issue7544.pdf`, `issue4650.pdf`, `issue11718_reduced.pdf` and
`PDFBOX-3062-005717-p1.pdf` convert at both targets and pass veraPDF at both; `issue11768_reduced.pdf`
(an `Adobe-Japan1` font under a CMap of its own, no `ToUnicode`) keeps the refusal. The fonts are named from the whole population the
requirement failed over, not the places the report keeps (ADR 2602). **Built for a simple
font's bare CFF program** (`/FontFile3 /Type1C`, ADR 2578): the program is written, its `CharStrings`
INDEX grown by a copy of `.notdef`'s charstring per width (or a glyph that draws nothing where the code
reached no glyph), the charset and String INDEX restated, and the encoding restated to name each copy
where it named none; the rebuilt program is read back by `read-fonts` before the font is proved.
`issue5256.pdf`, `issue7439.pdf` and `10531_2004_Article_5096977.pdf` convert at both targets under
both profiles and pass veraPDF at both; six more lose this row and stay refused on others. **Built
for a `CIDFontType0`'s CID-keyed CFF program** (`/FontFile3 /CIDFontType0C`, ADR 2598): a copy of
`.notdef`'s charstring per shown CID the charset does not hold, under glyph 0's Font DICT, the charset
restated with the CIDs, `FDSelect` restated in the source's format 0 or 3 and the `FDArray` with it,
`CIDCount` raised, every table read back before the font is proved; every `Type0` font over one
descendant is answered together. Of the four documents of this row over such a program,
`issue4650.pdf`, `issue11718_reduced.pdf` and `PDFBOX-3062-005717-p1.pdf` show their own CID 0, which
ADR 2622 answers; `issue12963.pdf`, with 199 places on glyph 0, converts at
both targets and passes veraPDF at both. **Built for a simple font's Type 1 program** (`/FontFile`,
ADR 2620): the `eexec` section decrypted, the `CharStrings` dictionary grown by a copy of `.notdef`'s
charstring per width (or a glyph that draws nothing) under Type 1 charstring encryption at the program's
`lenIV`, the encoding restated to name each copy, `/Length1`–`/Length3` restated, and the section read
back byte for byte and by `read-fonts` before the font is proved. `issue11477_reduced.pdf`,
`issue6721_reduced.pdf` and `issue6901.pdf` convert at both targets under both profiles and pass veraPDF
at both; `504076.pdf` loses this row and stays refused on others. **Built for a shown code a simple
TrueType program reaches no glyph for** (ADR 2753), which section 6.2.11.4.1 finds the program does
not define and this reader draws as nothing: a glyph with no outline at the code's width, beside ADR
2543's copies; `issue14821.pdf` and `issue17333.pdf` convert at both targets under every profile and
pass veraPDF at both. Left: `issue11768_reduced.pdf` (above), and documents refused on other rows as
well. Mitigations `fonts/no-notdef-glyph-shown`; limits 2.2.

**R11 — a TrueType code that reaches its glyph by a route ISO 19005 does not admit** (built; 13
documents left of the 30 at 2b and 28 at 4, alone in 3 under `as-if-printed` and 1 under
`as-shown`). **Remedy, as built** (ADR 2520, `Rewrite::TrueTypeCmapRestated`): the font becomes
symbolic, its `/Encoding` is removed, and a new descriptor with a new `FontFile2` carries a `cmap` of
a (1, 0) format 6 and a (3, 0) format 4 subtable at 0xF000 plus the code, built from the code-to-glyph
map this reader drew with (§9.6.5.4 leaves that choice to the processor, so the file now states it);
the content stream is untouched, and the proof reloads the font from the in-memory objects and finds
all 256 codes on the same glyph. The file's `xmpMM:History` names the clause and the fonts (A48's
condition). A code on glyph 0 is left out and held only to "still no other glyph" (R03a's `.notdef`
row is untouched). Four rows answer by it: `non-symbolic-truetype-differences-are-listed-names`,
`-differences-need-the-unicode-cmap`, `-program-maps-every-code` and
`symbolic-truetype-program-has-a-usable-cmap`; the three others (`truetype-codes-reach-glyphs-by-the-standard-route`,
`symbolic-truetype-states-no-encoding`, `non-symbolic-truetype-uses-a-standard-encoding`) take it as
their second answer where their own narrower rewrite refuses (`Answer::OrStated`). A font the
file did not embed is restated over the face the substitution embeds for it, from that face's own
code table, and one the substitution makes a `Type1` font is passed over (ADR 2744), which frees
`issue6127`, `issue7580`, `issue17492` and `PDFBOX-3067-negativeTf` under `as-if-printed` at both
parts. **Left**: a `/FontFile2` whose RFC 1950 check value disagrees, which this reader does not
draw from (`507676`, `issue13316_reduced`); fonts the substitution photographs the page around
(`sample_fonts_solidconvertor`); and three documents the UTF-8 name fence refuses first
(`SimFang-variant`, `XiaoBiaoSong`, `issue20489`). A conversion
copies at most 32 MiB of font program and copies none for a file another requirement refuses by
name. Probe of the 32 named documents: exit 0 rose from 2 to 14 under each of the four arms, veraPDF
at the arm's own target passing 13 and failing `pdf.js:issue6961` (a checker defect, section 8).
Census, the pack present, failing documents converted (pdf.js then corpora): `as-if-printed` 2b 838 to
848 and 386 to 388, part 4 841 to 849 and 385 to 387; `as-shown` 2b 797 to 807 and 333 to 335, part 4
801 to 809 and 331 to 333; `issue6961` is among them and does not conform. Mitigations
`fonts/truetype-codes-reach-glyphs-by-the-standard-route` and the four TrueType rows; ADRs 1942, 2520.

**R10 — a font the converter could re-file** (7 documents): a CFF-flavoured OpenType program
under a simple `TrueType` dictionary (`issue215`) and a Type 1 program under a `CIDFontType0`
(`issue11740_reduced`), which no row of Table 124 files under their dictionaries and which a
retyping would reinterpret; `499039`'s two codes reaching one glyph at two widths in a CFF face
and in an embedded subset `TrueType` program (each needs its own copy of the glyph, as ADR 2723
makes in a `glyf` face); a descriptor written inside a font that has no program
(`SimFang-variant`, `XiaoBiaoSong` — the substitution needs an object to add `/FontFile2` to, and
both would still stop at R03 and R11); a CFF width in a subroutine (`issue_cff_unsigned_bbox`);
and a Type 0 font with no descendant (`issue12823`, the photograph). Builds only; no owner word.

**A code Annex D encodes twice** (built, ADR 2723): `WinAnsiEncoding`'s 173 and 160 and
`MacRomanEncoding`'s 202, shown at a width other than their twin's, are named `softhyphen` or
`nonbreakingspace` — the character ISO 32000-1's Annex D says each means — where that edition
describes the file, and a `glyf` face that gives the character its twin's glyph gains a copy of it
at the code's own width; `486832` converts under `as-if-printed` at 2b and 4 (under `as-shown` its
substituted faces' `substitute-stretch` is R09's). **A font written inside another object** (built,
ADR 2722): a program behind a direct descriptor is restated where it stands, and `/CIDToGIDMap` is
written into a `CIDFont` inside a `Type0` written in any object; `hesr13613` passes both rows and
now stops at the output's `graphics/only-operators-the-base-standard-defines`, which they hid.

**A `glyf` program under a `CIDFontType0`** (built, ADR 2716): the descendant becomes the
`CIDFontType2` that selects the same glyphs, its map kept or written as the `Identity` this reader
read, the program renamed `/OpenType` under the key it is filed by; `issue9949` converts under both
shipped profiles at 2b and 4. **A `CIDSystemInfo` that disagrees with its `CMap`** (built, ADR
2717): restated as the `CMap`'s where this reader reads the `CIDFont`'s collection for nothing;
`issue18117` and `issue9534_reduced` convert under both at both parts, and `issue11768_reduced`'s
remaining stop is R03a's.

**A Type 1 advance in encrypted charstrings** (built, ADR 2685): the `hsbw` or `sbw` width operand
is restated inside the decrypted charstring and the program is encrypted again, its three lengths
stated again, the width being the horizontal component the program's `/FontMatrix` gives it.
`bug1157493`, `bug1539074`, `bug1539074.1`, `issue1905` and `issue2833` convert under both shipped
profiles at 2b and 4, and `ESUjulyaugust2002` under `as-shown`
(under `as-if-printed` its `halftone-name` is R17's); `issue_cff_unsigned_bbox`'s width is in a CFF
subroutine and stays refused. `issue11150_reduced`'s `Symbol` states a descriptor and no widths,
which ISO 32000-1 Table 111 allows a standard 14 font only with all four absent and ISO 32000-2
Table 109 allows no font; the checker fails it by name (`fonts/simple-font-states-its-widths`),
and the three entries are written as the published metrics this reader lays it out by (ADRs 2686,
2687). A font drawn only in rendering mode 3 that states none of the four, which no route embeds,
is given the three and a descriptor with no program, read off the face this reader substitutes for
it and proved to advance and read every code as before (ADR 2688): at part 4 for one of the
fourteen, whose exemption ISO 32000-2 removes (`42828.0001.001`, `issue12963`, `issue2074`,
`issue2956`, `multiple-filters-length-zero`, `scan-bad`), and at both parts for `0000490`'s
TrueType `Arial`, `TimesNewRoman` and `CourierNew`. All 16 outputs of each census pass veraPDF
1.30.3, and their extracted text and its positions are those of the outputs round 1932 wrote.

**A cut font program** (built, ADR 2636): a program whose decode stops short and that this reader
cannot load takes the unembedded route; `issue11651`, `435321` (2b and 4) and `268346` (4) are freed
and pass veraPDF. A `FlateDecode` refused outright or disowned by its Adler-32 is cut too;
bytes that decode whole and are not a program in the format they are filed under are too (decision
5): `178360` converts with `ICLJGJ+Glypha` substituted as well and passes veraPDF at 2b and 4; `F001u_3_7j`'s front loads whole and is
refused by name — writing the stream whole is the remedy it waits for.

## 4. Colour and graphics

**R02 — `image-smoothing`** (43 documents at every census line; alone in 41 at 2b, 39 at 4 — the
largest single reason). An image dictionary's `/Interpolate true`; both parts require it false; both
shipped profiles stop. ISO 32000-2 §8.9.5.3 says of the entry: "However, this is only a hint, and a
PDF processor may ignore it." **Remedy**: `as-if-printed` discards it — a printer was free to
ignore it, so a print need not have carried it; `as-shown` discards it too with the loss named,
because this reader honours the hint (`render-cpu`'s image filter) and a magnified image is drawn
nearest-neighbour instead of bilinear in the output: a real loss, small and stated, which the window
shows only at magnification. Mitigations `graphics/image-interpolation-is-off`, limits 4.7.

**R05 — `DeviceCMYK` beside an RGB intent, part 4 only**. All but `digitally_signed_3D_Portfolio`
convert at 2b, through part 2's `DeviceN` default (that one is a `D-defect` there). ADR 2130 lets the
operator name the press; ADR 2114 section 4 refused a shipped `/DefaultCMYK` because this reader drew
`DeviceCMYK` through its assumed press, which no ICC profile stated. **Remedy**: (a) `as-if-printed`
uses the shipped press profile as the default when none is supplied, loss `press-assumed`, the
profile named in the report and `xmpMM:History` (ADR 2518); (b) **built** (ADR 2747): every other
profile writes *this reader's* assumed inks (`data/icc/quorra-assumed-inks.icc`, ADR 2700) as the
`/DefaultCMYK`, a statement and no loss, because the window drew those colours through those inks;
(c) **built** (ADR 2750, deciding ADR 2749): the default makes a `DeviceCMYK` mark overprinting under
mode 1 an `ICCBased` one, which section 6.2.4.2 forbids, so under a profile that accepts ADR 1978's
`overprint-mode` (both shipped ones) the governing states are written `/OPM 0` beside it, the loss
named in the report and in its own history entry. What is left is ADR 1978's refusal: a governed
mark blended in a `DeviceCMYK` group, where this reader composites by the mode (`363_Risk…`,
`response-to-fiber-concerns[1]`; each converts with its group's space written `DeviceRGB`, the
control). Mitigations section 4.2; ADRs 2114, 2130, 2518, 2747, 2749, 2750.

**R12 — a DeviceN colourant with no Separation to name it** (7 pdf.js documents, each alone).
**Remedy**: write the colourant's Separation from the producer's own DeviceN space — its alternate
space, and its tint transform restricted to that one colourant with the others at zero. For a
PostScript calculator function the restriction is exact (the program is prefixed with the zeros);
for a sampled or exponential one it is a resampled function, and that loss is named. This derives,
rather than invents, the transform: it is what the producer's space paints for that colourant alone.
ADR 2490 decides it; mitigations `graphics/spot-colourants-appear-in-the-colorants-dictionary`.
**Built for a PostScript calculator function** (ADR 2690, `archive/spot_separation.rs`): six of the
seven convert under every shipped profile at both parts. `devicen.pdf`'s transform is sampled
(FunctionType 0) and stays refused by that name; an `NChannel` space is refused too, since there
the entry changes what the page draws.

**R17 — two press statements `as-if-printed` keeps** (4 documents): `halftone-name` (3) and overprint
mode 1 under an ICC CMYK space (1). `as-shown` already discards both (ADR 1965). *A company*
printing on its office printer uses that printer's screens. **Remedy**: `as-if-printed` takes the
same discards, each loss named. Mitigations sections 4.2, 4.3. **Built** (ADR 2693): both entries,
and every profile that takes either loss names each halftone and the name it stated, or each
graphics state and the pages of the marks it governed, in the output's `xmpMM:History`. All four
convert under `as-if-printed` at 2b and 4, byte for byte the `as-shown` outputs once dates are
masked; `no-data-loss` keeps refusing `102952.pdf` by `overprint-mode`.

**R13 — a JPEG 2000 image outside the baseline** (3 documents, each alone). **Built** (ADRs 2694,
2695): each of the three is a bare codestream under a stated `/ColorSpace`, which `preserve` now
gives the JP2 file structure §7.4.9 expects, the codestream byte for byte; any other shape with a
stated space is ADR 1400's lossless transcode. `as-if-printed`, `as-shown` and `only-metadata-loss`
answer the four JPEG 2000 sites with it, as the stating profiles did. Mitigations
`graphics/jpeg2000-uses-the-baseline-feature-set`.

## 5. The content-stream fences

ADR 0816's fence is that the converter never edits a producer's content stream. A681 asks for it to
be opened behind the remedy profiles; the answers below open it without editing a byte of one.

**R03 — the fenced rows the photograph carries** (52 documents at 2b, 54 at 4; alone in 26 and
29): an operator the base standard does not define (41 — the second-largest single reason), a
UTF-8-invalid bound name (6), a MediaBox
outside part 2's range (2), an inline image's `/I` (2), a hexadecimal string with a non-digit (1),
a Type 3 width its `d0`/`d1` contradicts (3), an `ActualText` holding a Private Use character (2, part
4), a transfer function (1, `issue6931_reduced`, which draws a near-black negative without it). *A
company* would print and scan. **Remedy**: the photographed page (ADR 2394) for the page that holds
the fault, under `as-if-printed`, loss `text-as-image`; the other pages keep their text. `as-shown`
stops, except where the page's readback names no character (ADR 2394's gate as it stands). Finer
remedies exist for some: the bound names a content stream never uses can be renamed outright; a
MediaBox past 14 400 units becomes a smaller box under `/UserUnit` (ADR 1123's bracket around the
producer's operators); part 4 has none of part 2's limits (R06b). Q694 asks for the widening.
A photographed page is one the window drew whole, its own bytes' faults apart (ADR 2516 decision
5); ADR 2618 adds the page's own damaged `/Contents`, a text object the stream ends inside and a
keyword inside an array, and names what stays refused: a part of `/Contents` drawn of nothing, an
image or shading this reader cannot build, a drawing stopped at a bound. ADR 2759 has the reader name
the faults a stream that fails its own check value most often decodes to as the stream's own (a
colour given a count no space takes, an undefined `sh` name, an inline image not bracketed as §8.9.7
writes one, a stray `EI`, a nameless `Do`, a clip the stream ends on, a bare `true`), and ADR 2760 has
the refusal say which of the three kept causes holds a page. Of the census's seven, `504076` and
`492484` are freed, `150277` and `219789` now wait on the pixels one conversion photographs,
`poppler-90-0-fuzzed` on the output check, and `427669` (a bound) and `507676` (images whose data does
not decode) stay refused by name: every stream they name fails `zlib` too, and neither a bound nor an
undecodable stream tells the file's fault from this reader's.
A page photographed in a document that states optional content is drawn in the configuration this
viewer shows and carries `optional-content-of-photographed-pages`, the groups it fixed named with
the state drawn, under `as-if-printed` and `state-every-loss` at 2b and part 4; Level A and Level U
keep the refusal (A717, ADR 2576). Where that page also holds content this reader does not draw
whole, the refusal is that one's, and the photograph waits for the fence to name it.
An operator that is a run of regular characters §7.3.3 spells as no number, and that this reader
draws as the number on its front (`33600.272.58`, `454` and a damaged byte; ADR 1004), is the one
answer here that edits a content stream, and only the run: where every unlisted operator in the
document is such a run, each is written as the number the window draws, loss `content-as-drawn`,
under `as-if-printed`, `as-shown` and `state-every-loss` (whose page names each run), and the page
keeps its text (ADR 2638). `509284` keeps one page photographed of five: its page 2 holds a stray
`)`, which is no run.
Mitigations section 4.7; limits 5.2.

**R04 — a resource the content stream names and its dictionary does not hold** (22 documents;
alone in 12): a name nothing defines (17) and a form with no `/Resources` of its own (7). This
reader draws a form with no `/Resources` from the page's resources (ADR 0255), and a name nothing
defines draws nothing and is reported (`note_missing_resource`). **Remedy**: write into the form's
dictionary the resources the reader used — a copy of the form per page where two pages differ — and
for a name nothing defines, an empty resource of its category (an empty form, an empty graphics
state), which draws the nothing the reader drew. No byte of a content stream changes and no mark is
added. A font name nothing defines is answered only where the reader advanced nothing for it, and is
otherwise R03. ADR 2490; mitigations `graphics/named-resources-are-defined`. **Built** by ADR 2500; the standard-fourteen name's embedding is ADR 2522's, and the copy of the form per differing page, bounded at 256 copies and 32 MiB, is ADR 2523's. No census document is refused for the differing-pages shape, so that item moves tests and no count. ADR 2746 built the last two: a Type 3 font stating no `/Resources` is given the object its pages state, or the union of their bindings where that draws each page as the reader did (`pr4922`); and a font that moved is the empty font wherever no glyph of a defined font is placed from the position its spacing left before the text matrix is set again (the five govdocs files, which `TD` after each such show). Still refused by name: a Type 3 font whose pages bind a glyph's name differently (a copy per page, owed; no census document), and a show whose spacing a later glyph stands on, or that clips.

## 6. Files, structure, annotations and attachments

**R06b — the other part** (39 documents at 2b, 8 at 4; alone in 21 and 1): a PDF 2.0 document asked
for at 2b (22, plus one declined as a source), an implementation limit part 4 does not set (18), and
the eight `UnknownFilter-*` files at part 4, whose `%PDF-3.x` header ADR 2692 writes there as 2.0
(R06a). PDF/A-2 is built on ISO 32000-1, so a PDF 2.0 file is not a 2b file by design. *A company*
whose rule does not pin the part takes PDF/A-4. **Remedy**: the verb takes "2b, or 4 where 2b refuses" as one request and says which part it wrote. No loss.

**R06a — a header that states no version this tree reads** (7 documents, each alone; five are
`T01_*` test files). The output is a new file this program writes; its header states the version of
the file written (§7.5.2), as every converted output's does. **Remedy**: write the target's header
and record the source's header bytes in `xmpMM:History`. ADR 2490. **Built** by ADR 2558: the
header is the target's (1.7 for part 2, 2.0 for part 4), the source's first line goes escaped into
`xmpMM:History` and the report, and a rebuilt structure states the target's version and `/Type /Page`.
A page tree root that states `/Parent`, or a kid that names no dictionary, declines the source to
`recovered-structure`. Five `T01_*` files and `bug1606566` convert, and `issue9105_other` and
`PDFBOX-6040-nodeloop` under `recovered-structure`; all pass veraPDF at both targets. A header
stating a version later than any §7.5.2 defines (`%PDF-3.n`, the `UnknownFilter-*` files) is ADR
2692's: written as 2.0 at part 4, the source's first line and version in `xmpMM:History`, and
refused at part 2 as a 2.0 source is, by a sentence that names part 2.

**R01 — a document only this reader's recovery reads** (the standard defines none: its Annex C.4 advises on a damaged cross-reference table only, ADR 2502; 23 declined as a source; the reader draws a page of
18 of them, 4 of those blank; five draw no page, section 7.2). *A company* would open it, see it,
and print it. **Remedy**: write the structure the reader recovered — the catalog and page tree it
walked — loss `recovered-structure` (new), the report naming what was rebuilt; the photograph is the
fallback where the recovered objects do not convert. ADR 2490 reopens the declining sentence ("would
state a structure no producer wrote"): the output states the structure this reader drew from. **Built** by ADR 2502, its outcome in ADR 2503; the encrypted and the signed source (the claim and the signatures are the source's) and the direct page, parented or not, are ADR 2523's, which removed `refuse_what_is_not_rebuilt`; the stated-count refusal of ADR 2502 decision 7 stands. Of the six ADR 2503 left, `poppler-742-0-fuzzed` (its page's dictionary stops being readable at byte 678) is rebuilt from the seven entries the reader drew the page from, named as read in part (ADR 2551), and refused by other rows; `027613` states `/N 429` and the reader now reaches all 429 pages (ADR 2526), so its refusal is an ordinary requirement's and not this row's; `issue19484_1`, `issue19484_2` and `poppler-395-0-fuzzed` get past the rebuild and are refused by other rows, and `issue9105_other` by the file-header row. The stated-count refusal is now the named loss `page-not-held` where a profile takes it (A727, ADR 2596): a `/Kids` entry that names no object is §7.3.10's null and is subtracted from the root's `/Count` under `recovered-structure`, and one the cross-reference information places with no dictionary there is a page not held, the root's count and the count carried stated in the report, the JSON and `xmpMM:History`. `as-if-printed` converts `A Defence of the New Tenseless Theory of Time`, `T02-02_003`, `10856087`, `14342427` and `66307685`; `as-shown` converts three and refuses the Defence document and `14342427` by `page-not-held`; `T02-03_006` holds none of the one page it states and is refused under every profile by the sentence the checker fails it with (A775, ADR 2670). `state-every-loss` and `no-data-loss` take `recovered-structure` and `page-not-held`, each stated on the appended page (A757, ADR 2669). The Defence document converts under both at both targets: its flat label tree's range at the thirteenth page, the one not held, labels none of the pages carried and is taken out, and the appended page takes a range of its own (ADR 2671).

**R07 — a filter no reader decodes** (the seven `UnknownFilter-*` files; never alone — each also
needs R06b or a font route). **Remedy**: the stream is what the reader treated it as, absent — an
image or form that drew nothing is removed, a font program becomes an unembedded font (R08's routes),
an ICC stream's alternate is stated — loss `undecodable-stream` (new). **Built** by ADR 2689 under
both shipped profiles and `state-every-loss` (`no-data-loss` refuses by the loss's name): an image
or form becomes a form drawing nothing, a content stream holds no operator, a font program is left
out, an object or cross-reference stream is not written, each named in the report and
`xmpMM:History`; an ICC profile and an image used as a mask stay refused by name. No R07 stream is
a decoder defect: seven state the invented `/XXXDecode`, `poppler-742-0-fuzzed` a fuzzed
`/DCTecode` (ADR 2679). The seven files stay refused by their `%PDF-3.x` header, which no part
admits; with it patched to `%PDF-1.7`, every one but `UnknownFilter-ICC` converts.

**R18 — a PieceInfo that keeps `ModDate` in the information dictionary at part 4** (3 documents,
each alone under `as-if-printed`): section 6.1.3 keeps `/ModDate` beside the catalog's PieceInfo, and
§14.3.4 asks it to be equivalent to the packet's `xmp:ModifyDate`. **Built** (ADR 2748): the two are
compared as instants, which frees `225188` (one instant, the packet stopping at the minutes) under
every profile; where they differ (`20789_ftp`, `UMTRI-2008-80`), §14.5 compares `LastModified` with
the dictionary's entry, so it stays and the packet's is restated from it, the former value recorded
in `xmpMM:History` (`modification-date`), under `as-shown`, `as-if-printed` and `only-metadata-loss`
(`disagreeing = "discard"`). Left: the stating profiles, whose page would have to state a packet
property.

**R16 — annotations and forms** (4 documents under `as-if-printed`, 14 at 2b and 17 at 4 under
`as-shown`). Built: a push button with no appearance dictionary is given the constructed one under
ADR 2319's state (ADR 2751, `160F-2019`), and an appearance dictionary stating no entry, or only null
ones, is taken away (ADR 2752, `sakula`). Left: a check box with no appearance dictionary (1,
`checkbox_no_appearance` — §12.7.5.2.3 makes its state names its values, and the reader draws
nothing for it, so the remedy left is its removal, which touches its field), a hidden widget whose
removal could break its field (3), a Popup pair the linked removal cannot take (2), an appearance kept on an appended page of a
document with a structure tree (10 over four sites, `as-shown` — at 2b and 4 no structure element is required, and
under a Level A target ADR 1014 already places the element inside the permission), a field whose
appearance the converter could not build (1), a signature widget that does not print (2, both XFA
forms — section 7.1). Builds only. Mitigations sections 6 and 7.

**R14 — the appended page could not be composed** (2 documents, each alone): `balloon_a1b_jp2k`'s
packet header holds U+0000 where the byte-order mark goes, which no face draws. Under
`state-every-loss` the packet rows' second step leaves a C0 control character other than a line
break or a tab off the page that keeps the packet and lists it on the statement page by its code
point, count and byte offsets (ADR 2682, `doc/questions/A778`), so it converts under that profile at
2b and 4; `no-data-loss` and the profiles naming no second step keep the face search's refusal. No
shipped face covers U+2400–U+243F, the control pictures A778's option (a) would set; and
`JPXTestGrey` at 2b, whose refusal is the extension-schema container
(a producer's container that does not describe every schema it uses), not the page. `241573_ftp`'s
catalog packet is Flate data its stream states no filter for, so through the filters it states it
is not text, and no other filter is tried (ADR 2679): `state-every-loss`'s packet rows now take the
second step, removing a packet no page can lay out and listing it on the statement page (ADR 2678),
so it converts under that profile at 2b. At part 4 the two section 6.1.3 document-information rows
carry a custom `/Info` key Table 349 names no property for (`/WPS-ARTICLEDOI` and others) into the
packet unchanged, under a namespace stating only where the value came from (ADR 2680,
`doc/questions/Q779`), under both stating profiles. A value that is not a text string, such as its
`/SaveStreams` dictionary, is removed by the rows' second step under `state-every-loss` and stated on
the appended page (ADR 2681), so it converts under that profile at 4 too; `no-data-loss` refuses it
by name.
ADR 1014; `crates/pdf-transform/src/archive/preserve.rs`. A packet page in a file stating no intent
is given one, a replaced packet answers the property row, and one that will not decode owes no page
(ADR 2676); a character `/WinAnsiEncoding` does not name is set through a `/Differences` array of
listed glyph names (ADR 2677) — so `breast+cancer+immunotherapy` and `PDFBOX-3148-2-fuzzed` convert
under both stating profiles at 2b and 4, and `issue20453` under `state-every-loss` at both. A tagged document whose
`/ParentTree` holds `/Kids` gave its appended page no place in the structure tree; that tree now
takes the pages' entries in a new leaf at any depth (ADR 2672), and `064034`, `316267`, `459011`
and `UMTRI-2008-80` convert under `state-every-loss` at 2b (`064034` at part 4 too). A source whose parent tree loses a content item
the structure tree claims has the entries rebuilt from that tree (ADR 2674), so `issue15340`,
`issue20516` and `pdfjs_wikipedia` convert under both stating profiles at 2b and 4.

**R15 — embedded files, the owner's example** (3 documents blocked at 2b, 5 at 4). The attachment
census: 22 of the 1478 documents carry 64 attachments — 13 PDFs (8 documents), 11 text or HTML (7),
17 MathML and other XML (4), 18 copies of a `.joboptions` settings file (1, text), 3 Word `.docx`
(3) and 2 Lotus and Quattro spreadsheets (2). Today `as-if-printed` discards an attachment ("paper
had no attachments"), and blocks where an annotation or a GoTo names it. *A company* would print
the attachment and file it behind the letter. **Remedy**: append it as pages, which ADR 1014's
permission already covers (content the file holds): an embedded PDF's own pages, each converted
under the same profile with its own refusals; an image on a page of its own; a text, HTML or XML
file as its text, laid out in a shipped face (HTML and MathML as written, not rendered — no engine
here renders them, and the loss says so); the annotation or GoTo pointing at the first appended page.
Loss `attachment-as-pages` (new). At PDF/A-4f the file is kept as it is (Annex A), so 4f is the
answer for the office files. Part 4's missing media type (2) is §14.13.2's
`application/octet-stream`, or the type the bytes' own header names. Mitigations section 11;
limits 3.1. **Built** (ADR 2536): a text, PDF, PNG or JPEG attachment appended as pages under a
`preserve` row with `on-failure` per file, a PDF converted one level deep; HTML and XML as text.
Still blocked: an attachment in a tagged document (its pages need structure elements) and office
files below 4f.

## 7. The exceptions, each with its reason

**7.1 A dynamic XFA form** (4: `issue14130`, `xfa_filled_imm1344e`, `xfa_issue14315`,
`MOZILLA-1671648-0`). Table 29's `/NeedsRendering true` says the pages in the file are not the
document; laying out the template is CLAUDE.md's exclusion (§K.1's permission). Photographing the
file's own pages would archive a placeholder as the document.

**7.2 A document the reader draws no page of** (5: `Brotli-Prototype-FileA`, `REDHAT-1531897-0`,
`bug1020226`, `poppler-937-0-fuzzed`, `PDFBOX-6041-example`): the render verb answers that the
document has no pages. Nothing was seen, so nothing can be archived.

**7.3 The unreadable** (27, counted in the census's last column): 20 need a password — *a company*
asks the sender, and with it the document converts (`on-missing-password = "stop"` is the input
asked for, not an exception); 5 use the public-key security handler §7.6.5 defines (`adbe.pkcs7.s5`,
and `PDFBOX-4352-0` whose `/Encrypt` resolves to null) — §7.6.5 is in scope and owed, not an
exception; 2 are not PDF files at all (no header, no object).

**7.4 An embedded office file at 2b or plain 4** (the 3 `.docx` and 2 spreadsheets): no reader in
this tree draws them, so they cannot become pages; the exception holds at 2b and plain 4 and is
answered at 4f.

## 8. What the converter itself gets wrong

Documents refused by the converter's second verdict — every decision carried out and the output still failing, kind
`output-fails` in `rank.py`'s section 2 — are the converter's own defects, and so are five signed sources at part 4
(`bug854315`, `issue16553`, `issue17069`, `Montenegro2021`, `signed_verified`) that the census harness printed as
`no-decision`. The five are not refused without a sentence: a signed source is refused by the signature decision
(`--authorise signature-assertion` allows it), which is not a row of `decided`; the harness reads it, and the five count
under `R16` as `unauthorised signatures/signature-assertion`. How many remain `output-fails` is the `D-defect`
row of section 2's table, which `rank.py` counts and leaves out where none remains (ADRs 2534 and 2535). The families, by cause:

| cause | documents | state |
|---|---|---|
| An appearance the conversion constructed (a free-text or widget stream built from the field's variable text) is content no pass saw: it names an unembedded Helvetica, selects a device colour no default licenses, or sets a blend mode | `annotation-tx`, `bug1538111`, `bug1865341`, `bug1871353`, `freetext_no_appearance`, `issue12750`, `issue16021` | fixed: a second conversion over the first output, only where the first would be refused (ADR 2534) |
| The validator keeps 32 places of a requirement and counts the rest; the rendering-intent rewrite was gated on the kept prefix | `AIAA-2002-4016-606` (114 places) | fixed: the validator names every place up to 4 096 and says when it stopped short; `sites::objects_named` refuses a requirement whose places it cut short rather than act on a prefix, and the rendering-intent rewrite walks every graphics state only then (ADRs 2535, 2540) |
| An annotation dictionary written directly in a page's `/Annots` array has no object number and was never given `/F` | `CLLASS Report for ER&L 2020`, `Wiki SI 563 2021` | fixed (ADR 2535) |
| A font with a descriptor and no `/Widths` (Table 109 makes the fourteen all-or-none) was embedded as a face whose dictionary states none, so shown codes were 0 wide | `NeilJairath_3`, `issue2099-1`, `issue5470` | fixed: the widths the viewer draws it with are stated (ADR 2535); `NeilJairath_3` converts at part 4 under both shipped profiles (ADR 2750) |
| The signature decision is not in `decided` | the five above | counted; at part 4 no `file-structure/document-signature-states-no-digest` row exists, so a profile's `discard` there authorises nothing (`config.rs` `authorisations`), while §5 above keys the loss `signature-assertion` by loss and lists both shipped profiles as `discard/stop` |
| Page 9 names `/Im0 99 0 R` and object 99 does not exist, which §7.3.9 makes null and so undefined; the survey counted the entry present without resolving it (trap 258), so the output's own check found the name undefined and no output was written. The object headers agree with both cross-reference sections, and the writer restates every `/Length` from the bytes it writes | `veraPDFHiResWrongObjectID` | fixed: the survey resolves the entry, and the name is defined empty in its place; it converts at 2b and 4 under both profiles (ADR 2550) |
| The output's page 1 has a `/Font` dictionary without the `/F3` its content names, which the source met | `AIAA-2005-6173-712` | gone: at round 1992's premise check it converts at 2b and 4 under `as-shown`, `as-if-printed`, `no-data-loss` and `state-every-loss`, so an earlier round's change removed it untraced |
| veraPDF 1.30.3 fails 6.2.11.5 / 6.2.10.5 at code 45 (`/minus`, 584) of fonts sharing one substituted Liberation Sans with a `/WinAnsiEncoding` font, where it reads `hyphen` (333) | `AIAA-2005-6173-712` (both stating profiles, 2b and 4) | not ours: §9.6.5.4 maps each font through its own encoding and both widths agree; veraPDF caches one code-to-name table per program for every direct `/Encoding` (ADR 2683) |
| An annotation appearance opens `/OC /MC0 BDC` and its resources hold no `/Properties`; the survey read a property list name without recording a miss, so the checker passed it | `issue14438` (`state-every-loss`, 4), `issue17679` (both stating profiles, 4) | fixed: the name is a `/Properties` miss, and the converter defines it as `<< /Type /OCMD >>`, which hides nothing (ADR 2684) |
| A symbolic TrueType font with no program meets ISO 19005-2 section 6.2.11.6's `cmap` paragraph, which reads a program; the face substituted for it carries (0,3), (1,0) and (3,1), which part 2 refuses, and the restating rewrite was never asked because only the output failed | `bug864847` (2b) | fixed: an output failing a requirement the source met is converted once more (ADR 2534's second pass), and that pass restates the program's `cmap` (ADR 2795) |
| The first-page trailer states `/Prev 576066 0 R`, a reference where ISO 32000-2 Table 15 asks for an offset; the reader finds the main section's objects by their headers, and the check enumerates only the first-page section, so the source meets section 6.1.7.2 while its thumbnails are `LZWDecode`, and the output's fresh table lists them | `JA1-1996-PDF`, `JA3-1998-PDF` (both parts, every profile) | fixed by the same second pass (ADR 2795); the check's enumeration is not widened, so such a source is still judged on its first section alone |
| A transparency group in `DeviceCMYK` with no CMYK intent and no `/DefaultCMYK`; the group rows took the stand-in press only where an operator supplied one, and part 2 prepared none, its `DeviceN` default being one §11.6.6's Table 145 forbids as a group's space | `issue14200`, `issue14297` | fixed: the stand-in is the part 4 `DeviceCMYK` row's (the inks' profile, or the shipped press as `press-assumed`), at part 2 only for such a group (ADR 2795). `issue14200` at 2b, whose soft mask's `/G` form states no `/Resources` and names none: given an empty dictionary of its own, which ISO 32000-2 §7.8.3 asks a writer to state, and the default written into it (ADR 2804) |
| The output still fails the `DeviceCMYK` row after the decision wrote its default, at 2b (the `DeviceN` default) and, since ADR 2750 lets the default be written, at 4; the pages 2 and 3 and the image the conversion lays out from the file's embedded PDFs are content no pass over the source saw | `digitally_signed_3D_Portfolio` | fixed: an output still failing a requirement the first pass changed is converted once more (ADR 2803); it converts under `as-shown` and `as-if-printed` at both parts |
| The checker accepted `Differences` names `uniXXXX`, `_` ligatures and `.suffix` variants through `encoding::text_for`, the Adobe Glyph List Specification's *algorithm*, where ISO 19005-2 6.2.11.6 asks for names the 2002 list itself lists | `issue6961`, `bug1132849`, `issue6068`, `issue6889`, `issue6894` | fixed in the checker: `encoding::is_listed` is the row test, and the five are refused under ADR 2520's restatement (ADR 2540) |
| The rows of 6.2.11.4.1 (every glyph shown is in the program) and 6.2.11.5 (its width agrees) still reach a `uniXXXX` glyph through the reader's lenient name mapping, where the clause's own mapping of a non-symbolic TrueType font (6.2.11.6's) finds none; veraPDF fails such a file at all three | a planted `/uni00A7` in place of `/section`: this tree fails it at 6.2.11.6 alone | open, owed: the two rows to resolve a name as 6.2.11.6 does (ADR 2540 section 5); a `pdf-font` reading owing tier 2, which round 1996 left (ADR 2803 section 4) |
| The embedded file is encrypted under a crypt filter with `/AuthEvent /EFOpen` this reader cannot open without its password; the first pass drops the encryption and copies the stream with `/Filter [/Crypt]`, so the output fails the crypt-filter row and the embedded-file row the source met | `auth-event-ef-open` (part 4) | fixed: the check fails ISO 19005-4 section 6.9 at an embedded file the file's own encryption withholds, where it passed it unread, so the first pass answers it by the profile's embedded-file answer (removed under `as-shown`, `as-if-printed`, `state-every-loss`; refused by name under `no-data-loss`) and nothing is copied with `/Crypt` (ADR 2803) |
| A page tree whose `/Kids` names its own root reaches no page; the reader draws the page it finds by `/Type`, and the closure copied from the catalog held none | `T02-02_002_page-tree-kids-links-recursive` (every profile, both parts) | fixed: declined beside ADR 2502's checks and rebuilt over the pages drawn under `recovered-structure`, refused by name elsewhere (ADR 2803) |
| An undefined operator (`> Tj`) in a widget's appearance stream: the page was photographed, and the photograph keeps `/Annots` | `hesr13613-sup-0001-Authormatrix` (`as-if-printed`, `state-every-loss`) | refused by name: an operator in a stream an annotation's appearance reaches keeps the fence's refusal (ADRs 0816, 2803) |
| A run drawn as a number (`0..9172`) in a stream whose `ASCII85Decode` data holds a group past 2^32: the content reader drew the bytes before it, and the run writer, which needs the whole decode, skipped the stream in silence | `poppler-90-0-fuzzed` (`as-if-printed`, `state-every-loss`) | fixed: such a stream's runs are not written and its page is photographed (ADR 2803) |
| A graphics state with `/TR` found only by its header past the damaged table: the first pass changed the place the check saw, and the output's fresh table listed the other | `027613` (`as-if-printed`, 4) | fixed: an output still failing a requirement the first pass changed is converted once more (ADR 2803) |
| A part 2 stand-in `/DefaultCMYK` built for a page's `/Group` on a page the photograph replaces, which is written without its group: about 1.3 to 2.2 MB changing no mark | `bug1365930` (`as-if-printed`, `state-every-loss`) | fixed: the photograph answers a group row whose every place is a photographed page's group, and no default is prepared or recorded for it (ADR 2804) |
| A photographed page's `/Contents` names the object the final revision reuses as the catalog's `/Metadata`, and the hexadecimal-digit repair and the `/Lang` respelling rewrote it as a content stream, breaking the packet and the identification edit with it | `064034` | fixed: a stream only photographed pages name is no content stream of the output, and both rewrites leave it (ADR 2640 section 1) |
| Two fonts share one descriptor and one CFF program whose widths only one of them agrees with, and the restatement for the other was written over the shared program | `507618` | fixed: the program is restated in a copy, with a copied descriptor naming it, where another font's widths would break in place (ADR 2640 section 2) |
| The second conversion's statement page was not in the report, which named only the first pass's | none in the corpora: no document reaches a second-pass photograph | fixed: the report's one statement names both passes' pages (ADR 2640 section 3) |
| An output intent the conversion added, over a source drawn without one, with the shipped sRGB profile: its colorants are sRGB rounded to s15Fixed16, and a grey of 188 draws 189 in red | `colorspace_cos`, `colorspace_sin` (266 pixels, one level, every profile and part) | required, not a defect: ISO 19005-2 and ISO 19005-4 section 6.2.4.3 admit `DeviceRGB` only through a profile, and with the intent removed the output draws pixel-identically (ADR 2691) |
| The shipped CMYK press written as the output intent (ADR 1153) where this renderer drew the source's `DeviceCMYK` through its assumed inks, which no profile states; the report said "this renderer already showed those colours that way" | `issue9940` (5 172 pixels, at most 7), `postscript_type4_many_outputs` (every pixel, at most 76, and an edge where GRACoL's cyan leaves sRGB); 52 documents' outputs carry the press under the shipped profiles and 34 under the stating ones | fixed: the owner chose (A780). Every profile but `as-if-printed` states this renderer's assumed inks as an ICC profile, within one level of the source (ADRs 2700, 2701); `as-if-printed` keeps the shipped press and ADR 2691's sentence that the drawing moves |
| A grey inside a soft mask, under the assumed-inks intent: a `/Luminosity` group stating `/CS /DeviceCMYK` paints `DeviceGray` images, fills or shadings, or a `/DeviceGray` mask group draws an isolated `/DeviceCMYK` group; the source converts between the two by ISO 32000-2 §10.4.2.3's black alone, the archive by the profile's own separation (ADR 0796), and the report blamed the page's blending space | `bug1755507` (25), `issue9017_reduced` (25), `bug1703683_page2_reduced` (23), `issue18032` (22), `issue12798_page1_reduced` (16), `issue13520` (9), at both parts under `as-shown` | fixed: the owner chose (A788). The reader follows section 6.2.4.3, a `DeviceGray` colour in a PDF/A file under a CMYK intent being the intent's black ink alone, so each of the six now draws within three levels of its source under `as-shown` (from 9 to 25), and every grey of such a file draws through the press's black (ADRs 2712, 2713) |

`0004-637X_751_2_90` is a CFF witness (`/FontFile3 /Type1C`, a CMR10 subset whose glyph 61 is 776.89 wide against
`/Widths` 778): the CFF width reader rounded to whole units and read 777, inside the tolerance, while veraPDF fails
ISO 19005-2 6.2.11.5. The reader now keeps the charstring's fraction, and its private `nominalWidthX`, written as a real
(495 after its floor here), is floored as the evaluator adds it rather than read as 0, so the converter restates the
width and the output passes both (ADR 2540). A bare Type 1 program's width is read from `hsbw`/`sbw`; where it
disagrees with `/Widths` the refusal's sentence is `PROGRAM_NOT_RESTATABLE`.

## 9. The documents behind each remedy

`rank.py`'s section 4, with the pack, verbatim:

- R01-recovered-structure (3): corpora:143424271191706722106594310729442158189.pdf, corpora:A Defence of the New Tenseless Theory of Time.pdf, corpora:T02-03_006_page-object-missing.pdf
- R03-photograph (59): corpora:020747.pdf, corpora:020806.pdf, corpora:027613.pdf, corpora:033689.pdf, corpora:040669.pdf, corpora:050734.pdf, corpora:064034.pdf, corpora:073439.pdf, corpora:150277.pdf, corpora:178360.pdf, corpora:189478.pdf, corpora:210260.pdf, corpora:219789.pdf, corpora:226387.pdf, corpora:268346.pdf, corpora:303226.pdf, corpora:367594.pdf, corpora:379049.pdf, corpora:392154.pdf, corpora:427669.pdf, corpora:435321.pdf, corpora:455746.pdf, corpora:477139.pdf, corpora:492484.pdf, corpora:498264.pdf, corpora:504076.pdf, corpora:507618.pdf, corpora:507676.pdf, corpora:509284.pdf, corpora:T02-05-01_006_font-size-operator-missing.pdf, corpora:T02-05-01_009_Missing_open_paranthesis.pdf, corpora:T02-05-01_011_paranthesis-substituted-with-brackets.pdf, corpora:T02-05-01_014_endstream-keyword-missing.pdf, corpora:hesr13613-sup-0001-Authormatrix.pdf, corpora:sakula.pdf, pdf.js:SimFang-variant.pdf, pdf.js:TAMReview.pdf, pdf.js:XiaoBiaoSong.pdf, pdf.js:boundingBox_invalid.pdf, pdf.js:bug1065245.pdf, pdf.js:bug1953099.pdf, pdf.js:issue17554.pdf, pdf.js:issue19484_1.pdf, pdf.js:issue19484_2.pdf, pdf.js:issue19517.pdf, pdf.js:issue19634.pdf, pdf.js:issue19971.pdf, pdf.js:issue20489.pdf, pdf.js:issue2128r.pdf, pdf.js:issue2391-1.pdf, pdf.js:issue3521.pdf, pdf.js:issue5010.pdf, pdf.js:issue5039.pdf, pdf.js:issue6342.pdf, pdf.js:issue6931_reduced.pdf, pdf.js:issue9252.pdf, pdf.js:poppler-90-0-fuzzed.pdf, pdf.js:sci-notation.pdf, pdf.js:smaskdim.pdf
- R03a-notdef-named (6): corpora:027613.pdf, corpora:268346.pdf, corpora:477139.pdf, corpora:504076.pdf, corpora:507676.pdf, pdf.js:issue11768_reduced.pdf
- R05-press (2): corpora:363_Risk-Based Management of Groundwater Contamination _Paper.pdf, corpora:response-to-fiber-concerns[1].pdf
- R06b-the-other-part (42): corpora:020747.pdf, corpora:020806.pdf, corpora:027613.pdf, corpora:033689.pdf, corpora:040669.pdf, corpora:150277.pdf, corpora:219789.pdf, corpora:268346.pdf, corpora:427669.pdf, corpora:477139.pdf, corpora:492484.pdf, corpora:507676.pdf, corpora:PDF 2.0 UTF-8 string and annotation.pdf, corpora:PDF 2.0 image with BPC.pdf, corpora:PDF 2.0 via incremental save.pdf, corpora:PDF 2.0 with offset start.pdf, corpora:PDF 2.0 with page level output intent.pdf, corpora:Simple PDF 2.0 file.pdf, corpora:T01_001_header-invalid-major-version.pdf, corpora:UnknownFilter-Font.pdf, corpora:UnknownFilter-FormXObject.pdf, corpora:UnknownFilter-ICC.pdf, corpora:UnknownFilter-ImageXObject.pdf, corpora:UnknownFilter-Linearized.pdf, corpora:UnknownFilter-OutlineObjStm.pdf, corpora:UnknownFilter-PageContentStream.pdf, corpora:UnknownFilter-objstm.pdf, corpora:UnknownFilter-xrefstm.pdf, corpora:pdf20-utf8-test.pdf, pdf.js:auth-event-ef-open.pdf, pdf.js:bug1937438_af_from_latex.pdf, pdf.js:bug1937438_mml_from_latex.pdf, pdf.js:bug1997343.pdf, pdf.js:bug2009627.pdf, pdf.js:bug2025674.pdf, pdf.js:bug854315.pdf, pdf.js:issue14755.pdf, pdf.js:issue17069.pdf, pdf.js:issue19176.pdf, pdf.js:issue3928.pdf, pdf.js:issue6151.pdf, pdf.js:poppler-67295-0.pdf
- R07-undecodable-stream (2): corpora:UnknownFilter-ICC.pdf, pdf.js:poppler-742-0-fuzzed.pdf
- R08-unembedded-font-photograph (35): corpora:040669.pdf, corpora:150277.pdf, corpora:178360.pdf, corpora:189478.pdf, corpora:210260.pdf, corpora:219789.pdf, corpora:303226.pdf, corpora:392154.pdf, corpora:427669.pdf, corpora:435321.pdf, corpora:443752.pdf, corpora:455746.pdf, corpora:507676.pdf, corpora:ControlCharacters.pdf, corpora:F001u_3_7j.pdf, corpora:T02-04-01_006_resources_subtype_wrong.pdf, corpora:UnknownFilter-Font.pdf, corpora:VerticalText.pdf, corpora:embedded_zip.pdf, corpora:mattgib_1.pdf, pdf.js:ThuluthFeatures.pdf, pdf.js:bug1365930.pdf, pdf.js:bug1815476.pdf, pdf.js:issue11403_reduced.pdf, pdf.js:issue13916.pdf, pdf.js:issue17929.pdf, pdf.js:issue18059.pdf, pdf.js:issue20489.pdf, pdf.js:issue5244.pdf, pdf.js:issue6127.pdf, pdf.js:issue7835.pdf, pdf.js:noembed-identity-2.pdf, pdf.js:noembed-identity.pdf, pdf.js:poppler-90-0-fuzzed.pdf, pdf.js:standard_fonts.pdf
- R09-as-shown-font-losses (26): corpora:All Quiet on the Third Coast.pdf, corpora:T02-04-01_001_resources-basefont-missing.pdf, corpora:T02-04-01_002_resources-basefont-wrong-font.pdf, pdf.js:90ms_rksj_h_sample.pdf, pdf.js:issue11555.pdf, pdf.js:issue11651.pdf, pdf.js:issue13316_reduced.pdf, pdf.js:issue13343.pdf, pdf.js:issue15053.pdf, pdf.js:issue18099_reduced.pdf, pdf.js:issue19182.pdf, pdf.js:issue20065.pdf, pdf.js:issue2128r.pdf, pdf.js:issue2391-2.pdf, pdf.js:issue2840.pdf, pdf.js:issue3521.pdf, pdf.js:issue6069.pdf, pdf.js:issue6106.pdf, pdf.js:issue6108.pdf, pdf.js:issue6286.pdf, pdf.js:issue8372.pdf, pdf.js:issue9084.pdf, pdf.js:mixedfonts.pdf, pdf.js:noembed-eucjp.pdf, pdf.js:noembed-jis7.pdf, pdf.js:noembed-sjis.pdf
- R10-font-refiling (6): corpora:499039.pdf, pdf.js:SimFang-variant.pdf, pdf.js:XiaoBiaoSong.pdf, pdf.js:issue11740_reduced.pdf, pdf.js:issue12823.pdf, pdf.js:issue215.pdf
- R11-truetype-route (7): corpora:507676.pdf, corpora:sample_fonts_solidconvertor.pdf, pdf.js:SimFang-variant.pdf, pdf.js:XiaoBiaoSong.pdf, pdf.js:issue13316_reduced.pdf, pdf.js:issue20489.pdf, pdf.js:issue6127.pdf
- R12-spot-colourant (1): pdf.js:devicen.pdf
- R14-appended-page-composition (4): corpora:241573_ftp.pdf, corpora:JPXTestGrey.pdf, corpora:balloon_a1b_jp2k.pdf, pdf.js:poppler-742-0-fuzzed.pdf
- R15-embedded-files (7): corpora:PDF-Sample-Document-Fully-Featured-Layout_Redacted.pdf, corpora:embedded_video_avi.pdf, corpora:embedded_video_quicktime.pdf, corpora:fileAttachment_fileAttachmentAnnotation.pdf, pdf.js:annotation-fileattachment.pdf, pdf.js:issue17056.pdf, pdf.js:issue18030.pdf
- R16-annotations-and-forms (17): corpora:MOZILLA-1671648-0.pdf, corpora:PDF-Sample-Document-Fully-Featured-Layout_Redacted.pdf, corpora:PDFBOX-6018-099267-p9-OrphanPopups.pdf, corpora:embedded_video_avi.pdf, corpora:embedded_video_quicktime.pdf, corpora:fileAttachment_fileAttachmentAnnotation.pdf, pdf.js:160F-2019.pdf, pdf.js:annotation-fileattachment.pdf, pdf.js:annotation_hidden_noview.pdf, pdf.js:annotation_hidden_print.pdf, pdf.js:checkbox_no_appearance.pdf, pdf.js:issue7446.pdf, pdf.js:multimedia_annotations.pdf, pdf.js:poppler-395-0-fuzzed.pdf, pdf.js:pr7352.pdf, pdf.js:widget_hidden_print.pdf, pdf.js:xfa_filled_imm1344e.pdf
- X1-dynamic-xfa (4): corpora:MOZILLA-1671648-0.pdf, pdf.js:issue14130.pdf, pdf.js:xfa_filled_imm1344e.pdf, pdf.js:xfa_issue14315.pdf
- X2-no-page-to-see (6): corpora:PDFBOX-6041-example.pdf, pdf.js:Brotli-Prototype-FileA.pdf, pdf.js:REDHAT-1531897-0.pdf, pdf.js:bug1020226.pdf, pdf.js:poppler-85140-0.pdf, pdf.js:poppler-937-0-fuzzed.pdf
