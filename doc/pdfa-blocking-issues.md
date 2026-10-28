# PDF/A: what still blocks a document, and what a company would do with it

Status: **a census record**, taken at `7d6ed87f` on 2026-10-27 by session 1828 for the owner's
`doc/questions/A681` item 1, and re-run by the commands in section 1 — the figures below are that
run's and are not kept current by hand. ADR 2490 is the decision that reopens the blocks; this file
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

| census line, with the pack | converted | conforming | refused | unreadable | declined as a source |
|---|---|---|---|---|---|
| pdf.js `as-if-printed` 2b | 804 | 2 | 149 | 11 | 10 |
| corpora `as-if-printed` 2b | 351 | 6 | 116 | 16 | 14 |
| pdf.js `as-if-printed` 4 | 807 | 0 | 148 | 11 | 10 |
| corpora `as-if-printed` 4 | 353 | 0 | 121 | 16 | 13 |
| pdf.js `as-shown` 2b | 768 | 2 | 185 | 11 | 10 |
| corpora `as-shown` 2b | 304 | 6 | 164 | 16 | 13 |
| pdf.js `as-shown` 4 | 772 | 0 | 183 | 11 | 10 |
| corpora `as-shown` 4 | 304 | 0 | 170 | 16 | 13 |

Without the pack, pdf.js `as-if-printed` is 793/2/160/11 at 2b and 796/0/159/11 at 4: eleven
documents the pack frees, which is why the pack is taken as installed below. `as-shown` is the same
either way (its `machine-face` stop holds the pack's documents).

Two deep JPEG 2000 shapes that A691 wrote (a deep grey image with opacity; a colour image with an
opacity channel of another depth) fail veraPDF at ISO 19005-2 6.2.8.3 and -4 6.2.7.3 on veraPDF's
reading of the codestream header, not the standard's; the draft of the upstream issue, its two
reproducers and the clauses it cites (cited, not quoted) are indexed here:
[`pdfa-blocking-issues/verapdf-jpeg2000-issue.md`](pdfa-blocking-issues/verapdf-jpeg2000-issue.md),
files under `pdfa-blocking-issues/verapdf-jpeg2000/`.

**Under `as-if-printed`, 289 documents are refused or declined at 2b and 292 at part 4; 262 at
both parts, 319 at one at least. Under `as-shown`, 351 at both, 397 at one at least.** Of the
`as-if-printed` population, everything but a few documents falls under a remedy below; those are
four exceptions (sections 7.1, 7.2) and five documents the converter itself fails
(section 8).

## 2. The ranking, for batch 124's brief

The order `rank.py` builds them in under `as-if-printed`, by documents freed (each figure given the
remedies above it):

| order | remedy | frees at 2b | frees at 4 | the loss | needs |
|---|---|---|---|---|---|
| 1 | R02 image smoothing | 41 | 39 | `image-smoothing` (named today) | the profiles' entry; ADR 2490 |
| 2 | R08 unembedded font, photographed | 27 | 30 | `text-as-image` (named today) | the owner: ADR 2394's gate widened (Q694) |
| 3 | R03 content-stream fence, photographed | 30 | 29 | `text-as-image` | the owner: the same widening (Q694) |
| 4 | R06b the other part | 26 | 1 | none — the target changes | an option of the verb |
| 5 | R05 the press (part 4 only) | — | 25 | `press-assumed` (new) | the owner: ADR 2114 section 4 reopened (Q694) |
| 6 | R03a `.notdef` named in the program | 23 | 23 | none visible | ADR 2490; the owner may prefer R03 |
| 7 | R11 the TrueType route stated | 23 | 20 | none visible | the owner: A48's line (Q694) |
| 8 | R04 the resource the reader used | 22 | 22 | none | ADR 2490 |
| 9 | R01 the structure the reader recovered | 18 | 18 | `recovered-structure` (new) | ADR 2490 |
| 10 | R10 a font re-filed | 15 | 15 | none | builds only |
| 11 | R06a the header written | 7 | 7 | none | ADR 2490 |
| 12 | R12 a spot colourant named | 7 | 7 | none for a Type 4 function | ADR 2490 |
| 13 | R07 an undecodable stream | 7 | 7 | `undecodable-stream` (new) | ADR 2490 |
| 14 | R14 the appended page composed | 6 | 5 | none | builds only |
| 15 | R17 press statements | 4 | 4 | `halftone-name`, `overprint-mode` | the profile's entry |
| 16 | R15 an embedded file as pages | 3 | 5 | `attachment-as-pages` (new) | inside ADR 1014 |
| 17 | R13 a JPX transcode | 3 | 3 | none | builds only (slot 3's writer) |
| 18 | R16 annotations and forms | 3 | 3 | named per site | builds only |
| 19 | R18 PieceInfo's date | — | 3 | none | builds only |

Under `as-shown` the first row is R09 (the `as-shown` font losses, 81 at 2b and 79 at 4), and the
rest follow in nearly the same order. Rows 1, 4, 6, 8, 11 and 13 together free about a third of the
blocked population with no loss beyond one already named; rows 2, 3 and 5 are the large ones that
need the owner's word, because each changes what a previous answer of the owner's allowed.

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

**R09 — the `as-shown` font losses** (107 documents under `as-shown` only; alone in 81 at 2b, 79
at 4): `machine-face` 41, `substitute-stretch` 40, both 23, `machine-face` with
`glyphs-from-to-unicode` 3. `as-if-printed` converts every one. *A company* that must keep what it
saw keeps what its screen showed. **Remedy**: (a) for `machine-face`, embed *the machine's face the
window drew* — a subset, where the face's own embedding permission allows; then `as-shown` loses
nothing on the machine that converted it, and a face that forbids embedding keeps the stop by name;
(b) for `substitute-stretch`, embed the shipped face with each glyph's outline scaled to the width
the file states, which is what the window draws (ADR 0358) — a font-program rewrite like
`pdf-font`'s width restatement, no content stream touched; (c) until (a) and (b) are built,
`as-shown` may authorise both losses by name, which is what reopening its stop means (ADR 2490).
Mitigations section 5; ADRs 0270, 0358, 2162.

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
converts at both targets under both profiles and passes veraPDF at both. Never for a shown CID 0, which
ISO 32000-2 §9.7.4.2 makes the CIDFont's own `.notdef` whatever glyph it is sent to (ADR 2557; veraPDF
agrees): `4387ba48….pdf` and `emi15514-sup-0002-SupinfoS2.pdf` show one, and an embedded CMap sending
the code to a fresh CID is what they would take. Refused as well where the checker's list of places
is cut short (more than 32 places on glyph 0), since the fonts are named from it. A CFF or Type 1
program (a charstring writer, ADR 2556 section 2) is owed. Mitigations `fonts/no-notdef-glyph-shown`; limits 2.2.

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
their second answer where their own narrower rewrite refuses (`Answer::OrStated`). **Left**: 5,
5 and 3 documents of those three, where the restatement could not be proved either. A conversion
copies at most 32 MiB of font program and copies none for a file another requirement refuses by
name. Probe of the 32 named documents: exit 0 rose from 2 to 14 under each of the four arms, veraPDF
at the arm's own target passing 13 and failing `pdf.js:issue6961` (a checker defect, section 8).
Census, the pack present, failing documents converted (pdf.js then corpora): `as-if-printed` 2b 838 to
848 and 386 to 388, part 4 841 to 849 and 385 to 387; `as-shown` 2b 797 to 807 and 333 to 335, part 4
801 to 809 and 331 to 333; `issue6961` is among them and does not conform. Mitigations
`fonts/truetype-codes-reach-glyphs-by-the-standard-route` and the four TrueType rows; ADRs 1942, 2520.

**R10 — a font the converter could re-file** (15 documents; alone in 9): a `glyf` program under a
`CIDFontType0` dictionary (3), two codes reaching one glyph at two widths (3 — duplicate the glyph
under a second identifier so each code has its own width), a `CIDSystemInfo` that disagrees with its
CMap (3 — restate it where the reader's drawing does not read the entry), a descriptor or a CIDFont
written inside another object (3 — make it an object, as ADR 2162 does for fonts), a Type 1 advance
in encrypted charstrings (2 — decrypt, restate, re-encrypt), a restatement that did not read back
(1), a Type 0 font with no descendant (1 — the photograph), and one refusal naming no font to
restate (1, a defect). Builds only; no owner word.

## 4. Colour and graphics

**R02 — `image-smoothing`** (43 documents at every census line; alone in 41 at 2b, 39 at 4 — the
largest single reason). An image dictionary's `/Interpolate true`; both parts require it false; both
shipped profiles stop. ISO 32000-2 §8.9.5.3 says of the entry: "However, this is only a hint, and a
PDF processor may ignore it." **Remedy**: `as-if-printed` discards it — a printer was free to
ignore it, so a print need not have carried it; `as-shown` discards it too with the loss named,
because this reader honours the hint (`render-cpu`'s image filter) and a magnified image is drawn
nearest-neighbour instead of bilinear in the output: a real loss, small and stated, which the window
shows only at magnification. Mitigations `graphics/image-interpolation-is-off`, limits 4.7.

**R05 — `DeviceCMYK` beside an RGB intent, part 4 only** (29 documents; alone in 20). Every one
converts at 2b. ADR 2130 lets the operator name the press; ADR 2114 section 4 refused a shipped
`/DefaultCMYK` because this reader draws `DeviceCMYK` through its assumed press, which no ICC profile
states. *A company* prints on its own press, whichever it is. **Remedy**: (a) `as-if-printed` uses
the shipped press profile as the default when none is supplied, loss `press-assumed` (new), the
profile named in the report and `xmpMM:History` — the print it describes is a print on that press;
(b) `as-shown` writes *this reader's* assumed press as an ICC CMYK profile (a lookup table computed
from `pdf_colour`'s own conversion), so the file states what the window showed — this needs an ICC
table writer and the owner's word, since it states this reader's choice; (c) meanwhile, R06b's
other part. Q694 asks for (a) and (b). Mitigations section 4.2; ADRs 2114, 2130.

**R12 — a DeviceN colourant with no Separation to name it** (7 pdf.js documents, each alone).
**Remedy**: write the colourant's Separation from the producer's own DeviceN space — its alternate
space, and its tint transform restricted to that one colourant with the others at zero. For a
PostScript calculator function the restriction is exact (the program is prefixed with the zeros);
for a sampled or exponential one it is a resampled function, and that loss is named. This derives,
rather than invents, the transform: it is what the producer's space paints for that colourant alone.
ADR 2490 decides it; mitigations `graphics/spot-colourants-appear-in-the-colorants-dictionary`.

**R17 — two press statements `as-if-printed` keeps** (4 documents): `halftone-name` (3) and overprint
mode 1 under an ICC CMYK space (1). `as-shown` already discards both (ADR 1965). *A company*
printing on its office printer uses that printer's screens. **Remedy**: `as-if-printed` takes the
same discards, each loss named. Mitigations sections 4.2, 4.3.

**R13 — a JPEG 2000 image outside the baseline** (3 documents, each alone). **Remedy**: write the
decoded samples back losslessly (Flate, or `pdf-jpx-write`'s baseline writer, slot 3 of batch 123):
the pixels are equal; only the file grows. Mitigations `graphics/jpeg2000-uses-the-baseline-feature-set`.

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
Mitigations section 4.7; limits 5.2.

**R04 — a resource the content stream names and its dictionary does not hold** (22 documents;
alone in 12): a name nothing defines (17) and a form with no `/Resources` of its own (7). This
reader draws a form with no `/Resources` from the page's resources (ADR 0255), and a name nothing
defines draws nothing and is reported (`note_missing_resource`). **Remedy**: write into the form's
dictionary the resources the reader used — a copy of the form per page where two pages differ — and
for a name nothing defines, an empty resource of its category (an empty form, an empty graphics
state), which draws the nothing the reader drew. No byte of a content stream changes and no mark is
added. A font name nothing defines is answered only where the reader advanced nothing for it, and is
otherwise R03. ADR 2490; mitigations `graphics/named-resources-are-defined`. **Built** by ADR 2500; the standard-fourteen name's embedding is ADR 2522's, and the copy of the form per differing page, bounded at 256 copies and 32 MiB, is ADR 2523's. No census document is refused for the differing-pages shape, so that item moves tests and no count. Still refused by name: a Type 3 glyph procedure's resources, and a font that moved.

## 6. Files, structure, annotations and attachments

**R06b — the other part** (39 documents at 2b, 8 at 4; alone in 21 and 1): a PDF 2.0 document asked
for at 2b (22, plus one declined as a source), an implementation limit part 4 does not set (18), and
the eight `UnknownFilter-*` files at part 4. PDF/A-2 is built on ISO 32000-1, so a PDF 2.0 file is
not a 2b file by design. *A company* whose rule does not pin the part takes PDF/A-4. **Remedy**: the
verb takes "2b, or 4 where 2b refuses" as one request and says which part it wrote. No loss.

**R06a — a header that states no version this tree reads** (7 documents, each alone; five are
`T01_*` test files). The output is a new file this program writes; its header states the version of
the file written (§7.5.2), as every converted output's does. **Remedy**: write the target's header
and record the source's header bytes in `xmpMM:History`. ADR 2490. **Built** by ADR 2558: the
header is the target's (1.7 for part 2, 2.0 for part 4), the source's first line goes escaped into
`xmpMM:History` and the report, and a rebuilt structure states the target's version and `/Type /Page`.
A page tree root that states `/Parent`, or a kid that names no dictionary, declines the source to
`recovered-structure`. Five `T01_*` files and `bug1606566` convert, and `issue9105_other` and
`PDFBOX-6040-nodeloop` under `recovered-structure`; all pass veraPDF at both targets.

**R01 — a document only this reader's recovery reads** (the standard defines none: its Annex C.4 advises on a damaged cross-reference table only, ADR 2502; 23 declined as a source; the reader draws a page of
18 of them, 4 of those blank; five draw no page, section 7.2). *A company* would open it, see it,
and print it. **Remedy**: write the structure the reader recovered — the catalog and page tree it
walked — loss `recovered-structure` (new), the report naming what was rebuilt; the photograph is the
fallback where the recovered objects do not convert. ADR 2490 reopens the declining sentence ("would
state a structure no producer wrote"): the output states the structure this reader drew from. **Built** by ADR 2502, its outcome in ADR 2503; the encrypted and the signed source (the claim and the signatures are the source's) and the direct page, parented or not, are ADR 2523's, which removed `refuse_what_is_not_rebuilt`; the stated-count refusal of ADR 2502 decision 7 stands. Of the six ADR 2503 left, `poppler-742-0-fuzzed` (its page's dictionary stops being readable at byte 678) is rebuilt from the seven entries the reader drew the page from, named as read in part (ADR 2551), and refused by other rows; `027613` states `/N 429` and the reader now reaches all 429 pages (ADR 2526), so its refusal is an ordinary requirement's and not this row's; `issue19484_1`, `issue19484_2` and `poppler-395-0-fuzzed` get past the rebuild and are refused by other rows, and `issue9105_other` by the file-header row.

**R07 — a filter no reader decodes** (the seven `UnknownFilter-*` files; never alone — each also
needs R06b or a font route). **Remedy**: the stream is what the reader treated it as, absent — an
image or form that drew nothing is removed, a font program becomes an unembedded font (R08's routes),
an ICC stream's alternate is stated — loss `undecodable-stream` (new).

**R18 — a PieceInfo that keeps `ModDate` in the information dictionary at part 4** (3 documents,
alone in 2): the converter writes the date from both sources and they disagree. Builds only.

**R16 — annotations and forms** (5 documents under `as-if-printed`, 14 at 2b and 17 at 4 under
`as-shown`): a button widget with no appearance dictionary (2 — §12.7.5.2.3; where the reader draws
nothing for it, its removal loses no mark), a hidden widget whose removal could break its field
(4), a Popup pair the linked removal cannot take (2), an appearance kept on an appended page of a
document with a structure tree (10 over four sites, `as-shown` — at 2b and 4 no structure element is required, and
under a Level A target ADR 1014 already places the element inside the permission), a field whose
appearance the converter could not build (1), a signature widget that does not print (2, both XFA
forms — section 7.1). Builds only. Mitigations sections 6 and 7.

**R14 — the appended page could not be composed** (6 documents, each alone): no face with every
character (the shipped faces cover WinAnsi only — the CJK pack's faces would set the rest), the page's
text painted in the initial `DeviceGray` with no default (write its colour in the intent's family),
a packet that is not UTF-8 (1), and one case where no requirement asked for the page (a defect).
Builds only. ADR 1014; `crates/pdf-transform/src/archive/preserve.rs`.

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
under `R16` as `unauthorised signatures/signature-assertion`. Five documents remain `output-fails`, and the `D-defect`
row is 4 per profile and target (ADRs 2534 and 2535). The families, by cause:

| cause | documents | state |
|---|---|---|
| An appearance the conversion constructed (a free-text or widget stream built from the field's variable text) is content no pass saw: it names an unembedded Helvetica, selects a device colour no default licenses, or sets a blend mode | `annotation-tx`, `bug1538111`, `bug1865341`, `bug1871353`, `freetext_no_appearance`, `issue12750`, `issue16021` | fixed: a second conversion over the first output, only where the first would be refused (ADR 2534) |
| The validator keeps 32 places of a requirement and counts the rest; the rendering-intent rewrite was gated on the kept prefix | `AIAA-2002-4016-606` (114 places) | fixed: the validator names every place up to 4 096 and says when it stopped short; `sites::objects_named` refuses a requirement whose places it cut short rather than act on a prefix, and the rendering-intent rewrite walks every graphics state only then (ADRs 2535, 2540) |
| An annotation dictionary written directly in a page's `/Annots` array has no object number and was never given `/F` | `CLLASS Report for ER&L 2020`, `Wiki SI 563 2021` | fixed (ADR 2535) |
| A font with a descriptor and no `/Widths` (Table 109 makes the fourteen all-or-none) was embedded as a face whose dictionary states none, so shown codes were 0 wide | `NeilJairath_3`, `issue2099-1`, `issue5470` | fixed: the widths the viewer draws it with are stated (ADR 2535); `NeilJairath_3` at part 4 is now R05's press refusal |
| The signature decision is not in `decided` | the five above | counted; at part 4 no `file-structure/document-signature-states-no-digest` row exists, so a profile's `discard` there authorises nothing (`config.rs` `authorisations`), while §5 above keys the loss `signature-assertion` by loss and lists both shipped profiles as `discard/stop` |
| Page 9 names `/Im0 99 0 R` and object 99 does not exist, which §7.3.9 makes null and so undefined; the survey counted the entry present without resolving it (trap 258), so the output's own check found the name undefined and no output was written. The object headers agree with both cross-reference sections, and the writer restates every `/Length` from the bytes it writes | `veraPDFHiResWrongObjectID` | fixed: the survey resolves the entry, and the name is defined empty in its place; it converts at 2b and 4 under both profiles (ADR 2550) |
| The output's page 1 has a `/Font` dictionary without the `/F3` its content names, which the source met | `AIAA-2005-6173-712` | open: not traced |
| A symbolic TrueType program whose `cmap` has (0,3), (1,0) and (3,1) | `bug864847` (2b) | open: rule `fonts/symbolic-truetype-program-has-a-usable-cmap` |
| A transparency group in `DeviceCMYK` with no CMYK intent and no `/DefaultCMYK`; the decision says the output intent licensed it | `issue14200` | open |
| The checker accepted `Differences` names `uniXXXX`, `_` ligatures and `.suffix` variants through `encoding::text_for`, the Adobe Glyph List Specification's *algorithm*, where ISO 19005-2 6.2.11.6 asks for names the 2002 list itself lists | `issue6961`, `bug1132849`, `issue6068`, `issue6889`, `issue6894` | fixed in the checker: `encoding::is_listed` is the row test, and the five are refused under ADR 2520's restatement (ADR 2540) |
| The rows of 6.2.11.4.1 (every glyph shown is in the program) and 6.2.11.5 (its width agrees) still reach a `uniXXXX` glyph through the reader's lenient name mapping, where the clause's own mapping of a non-symbolic TrueType font (6.2.11.6's) finds none; veraPDF fails such a file at all three | a planted `/uni00A7` in place of `/section`: this tree fails it at 6.2.11.6 alone | open, owed: the two rows to resolve a name as 6.2.11.6 does (ADR 2540 section 5) |
| A crypt filter and an embedded file that is not itself PDF/A, both still failing after their decisions | `auth-event-ef-open` (part 4) | open |

`0004-637X_751_2_90` is a CFF witness (`/FontFile3 /Type1C`, a CMR10 subset whose glyph 61 is 776.89 wide against
`/Widths` 778): the CFF width reader rounded to whole units and read 777, inside the tolerance, while veraPDF fails
ISO 19005-2 6.2.11.5. The reader now keeps the charstring's fraction, and its private `nominalWidthX`, written as a real
(495 after its floor here), is floored as the evaluator adds it rather than read as 0, so the converter restates the
width and the output passes both (ADR 2540). A bare Type 1 program's width is read from `hsbw`/`sbw`; where it
disagrees with `/Widths` the refusal's sentence is `PROGRAM_NOT_RESTATABLE`.

## 9. The documents behind each remedy

`rank.py`'s section 4, with the pack, verbatim:

- D-defect (5): corpora:AIAA-2005-6173-712.pdf, corpora:veraPDFHiResWrongObjectID.pdf, pdf.js:auth-event-ef-open.pdf, pdf.js:bug864847.pdf, pdf.js:issue14200.pdf
- R01-recovered-structure (18): corpora:027613.pdf, corpora:T02-01_001_document-catalog-No-document-catalog.pdf, corpora:T02-01_002_document-catalog-wrong-object-number.pdf, corpora:T02-01_003_document-catalog-indirecte-pages-reference-missing.pdf, corpora:T02-01_004_document-catalog-incorrect-pages-reference.pdf, corpora:T02-01_005_document-catalog-type-key-missing.pdf, corpora:T02-02_001_no-page-tree-node.pdf, corpora:T02-02_005_page-tree-no-kids.pdf, corpora:T04_012_trailer-root-no-indirect-reference.pdf, corpora:T04_013_trailer-root-invalid-indirect-reference.pdf, corpora:T04_014_trailer-root-indirect-reference-to-wrong-object.pdf, corpora:UnknownFilter-xrefstm.pdf, pdf.js:issue19484_1.pdf, pdf.js:issue19484_2.pdf, pdf.js:issue9105_other.pdf, pdf.js:issue9418.pdf, pdf.js:poppler-395-0-fuzzed.pdf, pdf.js:poppler-742-0-fuzzed.pdf
- R02-image-smoothing (43): corpora:103199.pdf, corpora:10531_2004_Article_5096977.pdf, corpora:140611493383864950079938377607360675655.pdf, corpora:1742-6596_375_00_001001.pdf, corpora:363_Risk-Based Management of Groundwater Contamination _Paper.pdf, corpora:475121.pdf, corpora:6.2017-0960.pdf, corpora:CLLASS Report for ER&L 2020.pdf, corpora:EngHonorsCapstoneReport-Final.pdf, corpora:Godfrey Lubuulwa-Thesis-2023.pdf, corpora:ModRob_An_Inexpensive_and_Modular_Robotic_Arm_for_Hobbyists_Poster_-_Albert_Anwar.pdf, corpora:Montenegro2021.pdf, corpora:Wiki SI 563 2021.pdf, corpora:coleprintingislam.pdf, corpora:dweigel_1.pdf, corpora:kateking_presentation_WN23.pdf, corpora:lorem-ipsum-plus-image-updated.pdf, corpora:lorem-ipsum-plus-image.pdf, corpora:sakula.pdf, corpora:selederm.pdf, pdf.js:bigboundingbox.pdf, pdf.js:bug1771477.pdf, pdf.js:bug1868759.pdf, pdf.js:bug1992868.pdf, pdf.js:comments.pdf, pdf.js:compressed.tracemonkey-pldi-09.pdf, pdf.js:highlights.pdf, pdf.js:image-rotated-black-white-ratio.pdf, pdf.js:images.pdf, pdf.js:issue12337.pdf, pdf.js:issue18911.pdf, pdf.js:issue18956.pdf, pdf.js:issue19239.pdf, pdf.js:issue9713.pdf, pdf.js:issue9972-1.pdf, pdf.js:issue9972-2.pdf, pdf.js:issue9972-3.pdf, pdf.js:pr12564.pdf, pdf.js:tracemonkey.pdf, pdf.js:tracemonkey_annotation_on_page_8.pdf, pdf.js:tracemonkey_freetext.pdf, pdf.js:tracemonkey_with_annotations.pdf, pdf.js:tracemonkey_with_editable_annotations.pdf
- R03-photograph (56): corpora:020747.pdf, corpora:020806.pdf, corpora:033689.pdf, corpora:040669.pdf, corpora:050734.pdf, corpora:064034.pdf, corpora:073439.pdf, corpora:150277.pdf, corpora:178360.pdf, corpora:189478.pdf, corpora:210260.pdf, corpora:219789.pdf, corpora:226387.pdf, corpora:268346.pdf, corpora:303226.pdf, corpora:367594.pdf, corpora:379049.pdf, corpora:392154.pdf, corpora:427669.pdf, corpora:435321.pdf, corpora:455746.pdf, corpora:477139.pdf, corpora:492484.pdf, corpora:498264.pdf, corpora:504076.pdf, corpora:507618.pdf, corpora:507676.pdf, corpora:509284.pdf, corpora:T02-05-01_006_font-size-operator-missing.pdf, corpora:T02-05-01_009_Missing_open_paranthesis.pdf, corpora:T02-05-01_011_paranthesis-substituted-with-brackets.pdf, corpora:T02-05-01_014_endstream-keyword-missing.pdf, corpora:hesr13613-sup-0001-Authormatrix.pdf, corpora:sakula.pdf, pdf.js:SimFang-variant.pdf, pdf.js:TAMReview.pdf, pdf.js:XiaoBiaoSong.pdf, pdf.js:boundingBox_invalid.pdf, pdf.js:bug1065245.pdf, pdf.js:bug1953099.pdf, pdf.js:issue17554.pdf, pdf.js:issue19517.pdf, pdf.js:issue19634.pdf, pdf.js:issue19971.pdf, pdf.js:issue20489.pdf, pdf.js:issue2128r.pdf, pdf.js:issue2391-1.pdf, pdf.js:issue3521.pdf, pdf.js:issue5010.pdf, pdf.js:issue5039.pdf, pdf.js:issue6342.pdf, pdf.js:issue6931_reduced.pdf, pdf.js:issue9252.pdf, pdf.js:poppler-90-0-fuzzed.pdf, pdf.js:sci-notation.pdf, pdf.js:smaskdim.pdf
- R03a-notdef-named (30): corpora:032270.pdf, corpora:073439.pdf, corpora:10531_2004_Article_5096977.pdf, corpora:225188.pdf, corpora:226387.pdf, corpora:268346.pdf, corpora:427330.pdf, corpora:4387ba480c6f935b8e97749d40b3c001ad584b7bda66ca2c040be67337669497.pdf, corpora:477139.pdf, corpora:504076.pdf, corpora:507618.pdf, corpora:507676.pdf, corpora:509284.pdf, corpora:PDFBOX-3062-005717-p1.pdf, corpora:emi15514-sup-0002-SupinfoS2.pdf, pdf.js:issue10519_reduced.pdf, pdf.js:issue11477_reduced.pdf, pdf.js:issue11718_reduced.pdf, pdf.js:issue11768_reduced.pdf, pdf.js:issue12963.pdf, pdf.js:issue13107_reduced.pdf, pdf.js:issue14821.pdf, pdf.js:issue17333.pdf, pdf.js:issue4650.pdf, pdf.js:issue4684.pdf, pdf.js:issue5256.pdf, pdf.js:issue6721_reduced.pdf, pdf.js:issue6901.pdf, pdf.js:issue7439.pdf, pdf.js:issue8234.pdf
- R04-absent-resource (22): corpora:020747.pdf, corpora:189478.pdf, corpora:219789.pdf, corpora:226387.pdf, corpora:427669.pdf, corpora:492484.pdf, corpora:DefaultRGBColourSpaces-inherit.pdf, corpora:PDFBOX-4372-2DAYCLVOFG3FTVO4RMAJJL3VTPNYDFRO-p4_reduced.pdf, corpora:T02-03_007_page-object-resources-wrong-object.pdf, corpora:T02-03_012_page-object-resources-missing.pdf, corpora:T02-05-01_005_invalid-Font-name-operator.pdf, corpora:meeram_1.pdf, corpora:source.pdf, pdf.js:bug1669099.pdf, pdf.js:bug1873345.pdf, pdf.js:issue1055r.pdf, pdf.js:issue17492.pdf, pdf.js:issue5954.pdf, pdf.js:issue6541.pdf, pdf.js:operator_list_cycle.pdf, pdf.js:poppler-90-0-fuzzed.pdf, pdf.js:pr4922.pdf
- R05-press (29): corpora:0004-637X_751_2_90.pdf, corpora:0004-637X_752_1_11.pdf, corpora:1742-6596_375_00_001001.pdf, corpora:210260.pdf, corpora:219789.pdf, corpora:363_Risk-Based Management of Groundwater Contamination _Paper.pdf, corpora:382252277877c00a92f3bb54daef1413bafe5557e2f5f7c32c23e800cd9bdcd4.pdf, corpora:4387ba480c6f935b8e97749d40b3c001ad584b7bda66ca2c040be67337669497.pdf, corpora:443752.pdf, corpora:504076.pdf, corpora:E3_Mayhew_et_al_2009.pdf, corpora:LLMImplicationsforScience.pdf, corpora:NeilJairath_3.pdf, corpora:Orellana2022.pdf, corpora:PDF-Sample-Document-Fully-Featured-Layout_Redacted.pdf, corpora:carp2010-2.pdf, corpora:digitally_signed_3D_Portfolio.pdf, corpora:hae14831.pdf, corpora:jne5_4_l03.pdf, corpora:mattgib_1.pdf, corpora:me450f12project10_report.pdf, corpora:response-to-fiber-concerns[1].pdf, pdf.js:freeculture.pdf, pdf.js:issue10640.pdf, pdf.js:issue14627.pdf, pdf.js:issue269_2.pdf, pdf.js:issue6413.pdf, pdf.js:prefilled_f1040.pdf, pdf.js:stamps.pdf
- R06a-header-written (7): corpora:PDFBOX-6040-nodeloop.pdf, corpora:T01_003_header-no-minor-version.pdf, corpora:T01_004_header_invalid-syntax-no-dash.pdf, corpora:T01_005_header-invalid-syntax-replace-char.pdf, corpora:T01_006_header-invalid-syntax-no-pdf.pdf, corpora:T01_007_header-no-version-info.pdf, pdf.js:bug1606566.pdf
- R06b-the-other-part (40): corpora:020747.pdf, corpora:020806.pdf, corpora:033689.pdf, corpora:040669.pdf, corpora:150277.pdf, corpora:219789.pdf, corpora:268346.pdf, corpora:427669.pdf, corpora:477139.pdf, corpora:492484.pdf, corpora:507676.pdf, corpora:PDF 2.0 UTF-8 string and annotation.pdf, corpora:PDF 2.0 image with BPC.pdf, corpora:PDF 2.0 via incremental save.pdf, corpora:PDF 2.0 with offset start.pdf, corpora:PDF 2.0 with page level output intent.pdf, corpora:Simple PDF 2.0 file.pdf, corpora:T01_001_header-invalid-major-version.pdf, corpora:UnknownFilter-Font.pdf, corpora:UnknownFilter-FormXObject.pdf, corpora:UnknownFilter-ICC.pdf, corpora:UnknownFilter-ImageXObject.pdf, corpora:UnknownFilter-Linearized.pdf, corpora:UnknownFilter-OutlineObjStm.pdf, corpora:UnknownFilter-PageContentStream.pdf, corpora:UnknownFilter-objstm.pdf, corpora:pdf20-utf8-test.pdf, pdf.js:auth-event-ef-open.pdf, pdf.js:bug1937438_af_from_latex.pdf, pdf.js:bug1937438_mml_from_latex.pdf, pdf.js:bug1997343.pdf, pdf.js:bug2009627.pdf, pdf.js:bug2025674.pdf, pdf.js:bug854315.pdf, pdf.js:issue14755.pdf, pdf.js:issue17069.pdf, pdf.js:issue19176.pdf, pdf.js:issue3928.pdf, pdf.js:issue6151.pdf, pdf.js:poppler-67295-0.pdf
- R07-undecodable-stream (7): corpora:UnknownFilter-Font.pdf, corpora:UnknownFilter-FormXObject.pdf, corpora:UnknownFilter-ICC.pdf, corpora:UnknownFilter-ImageXObject.pdf, corpora:UnknownFilter-Linearized.pdf, corpora:UnknownFilter-OutlineObjStm.pdf, corpora:UnknownFilter-objstm.pdf
- R08-unembedded-font-photograph (39): corpora:040669.pdf, corpora:150277.pdf, corpora:189478.pdf, corpora:210260.pdf, corpora:219789.pdf, corpora:303226.pdf, corpora:392154.pdf, corpora:443752.pdf, corpora:455746.pdf, corpora:All Quiet on the Third Coast.pdf, corpora:ControlCharacters.pdf, corpora:Deficient inflammasome activation permits an exaggerated asthma phenotype in rhinovirus C-infected immature mice.pdf, corpora:T02-04-01_001_resources-basefont-missing.pdf, corpora:T02-04-01_002_resources-basefont-wrong-font.pdf, corpora:T02-04-01_006_resources_subtype_wrong.pdf, corpora:VerticalText.pdf, corpora:embedded_zip.pdf, corpora:mattgib_1.pdf, corpora:sample_fonts_solidconvertor.pdf, pdf.js:ThuluthFeatures.pdf, pdf.js:bug1365930.pdf, pdf.js:bug1815476.pdf, pdf.js:issue11403_reduced.pdf, pdf.js:issue13916.pdf, pdf.js:issue17929.pdf, pdf.js:issue18059.pdf, pdf.js:issue18099_reduced.pdf, pdf.js:issue20489.pdf, pdf.js:issue2391-2.pdf, pdf.js:issue5244.pdf, pdf.js:issue6069.pdf, pdf.js:issue6106.pdf, pdf.js:issue6108.pdf, pdf.js:issue6127.pdf, pdf.js:issue7835.pdf, pdf.js:noembed-identity-2.pdf, pdf.js:noembed-identity.pdf, pdf.js:poppler-90-0-fuzzed.pdf, pdf.js:standard_fonts.pdf
- R09-as-shown-font-losses (107): corpora:008677.pdf, corpora:029016.pdf, corpora:032270.pdf, corpora:033689.pdf, corpora:050734.pdf, corpora:064034.pdf, corpora:102924.pdf, corpora:125619.pdf, corpora:137036.pdf, corpora:160721.pdf, corpora:176446.pdf, corpora:195981.pdf, corpora:20789_ftp.pdf, corpora:225188.pdf, corpora:225189.pdf, corpora:226387.pdf, corpora:247224.pdf, corpora:316267.pdf, corpora:363_Risk-Based Management of Groundwater Contamination _Paper.pdf, corpora:379049.pdf, corpora:421197.pdf, corpora:427330.pdf, corpora:427669.pdf, corpora:435321.pdf, corpora:436840.pdf, corpora:436857.pdf, corpora:459011.pdf, corpora:492484.pdf, corpora:495712.pdf, corpora:503492.pdf, corpora:504076.pdf, corpora:AIAA-2002-4016-606.pdf, corpora:AcroFormsBasicFields.pdf, corpora:Acroform-PDFBOX-2333.pdf, corpora:AlignmentTests.pdf, corpora:E3_Mayhew_et_al_2009.pdf, corpora:Godfrey Lubuulwa-Thesis-2023.pdf, corpora:It's all about connecting Reasons for drug use among Latino gay men living in the San Francisco Bay Area.pdf, corpora:PDF 2.0 image with BPC.pdf, corpora:PDF 2.0 via incremental save.pdf, corpora:PDF 2.0 with offset start.pdf, corpora:PDF 2.0 with page level output intent.pdf, corpora:PDFBOX-2725-878725.pdf, corpora:PDFBOX-3127-RAU4G6QMOVRYBISJU7R6MOVZCRFUO7P4-VFont.pdf, corpora:PDFBOX-3195.pdf, corpora:PDFBOX-4417-001031.pdf, corpora:PDFBOX-5762-722238.pdf, corpora:PDFBOX-5792-240045.pdf, corpora:PDFBOX-5809-509329.pdf, corpora:PDFBox.GlobalResourceMergeTest.Doc01.decoded.pdf, corpora:PDFBox.GlobalResourceMergeTest.Doc01.pdf, corpora:PDFBox.GlobalResourceMergeTest.Doc02.decoded.pdf, corpora:PDFBox.GlobalResourceMergeTest.Doc02.pdf, corpora:Simple PDF 2.0 file.pdf, corpora:UMTRI-2008-80.pdf, corpora:ajpa23833_am.pdf, corpora:calistoMTNoFontsEmbedded.pdf, corpora:drafts_David-Cyberstructure_Washington(Brussels-5)+2figs.pdf, corpora:hesr13613-sup-0001-Authormatrix.pdf, corpora:hisn13025_am.pdf, corpora:me450f12project10_report.pdf, corpora:sakula.pdf, corpora:sampleForSpec.pdf, corpora:selederm.pdf, corpora:test_fontArialNotEmbedded.pdf, corpora:test_pagelabels.pdf, corpora:tjp14887-sup-0001-SuppMat.pdf, corpora:webCapture.pdf, corpora:wp375.pdf, corpora:wp376.pdf, pdf.js:90ms_rksj_h_sample.pdf, pdf.js:TAMReview.pdf, pdf.js:bug1671312_ArialNarrow.pdf, pdf.js:bug1671312_reduced.pdf, pdf.js:bug847420.pdf, pdf.js:freetexts.pdf, pdf.js:issue10301.pdf, pdf.js:issue11555.pdf, pdf.js:issue12295.pdf, pdf.js:issue12706.pdf, pdf.js:issue13343.pdf, pdf.js:issue13447.pdf, pdf.js:issue14438.pdf, pdf.js:issue15053.pdf, pdf.js:issue15262.pdf, pdf.js:issue16091.pdf, pdf.js:issue17492.pdf, pdf.js:issue19182.pdf, pdf.js:issue20065.pdf, pdf.js:issue2128r.pdf, pdf.js:issue2840.pdf, pdf.js:issue3521.pdf, pdf.js:issue4934.pdf, pdf.js:issue6286.pdf, pdf.js:issue7454.pdf, pdf.js:issue7580.pdf, pdf.js:issue8372.pdf, pdf.js:issue8697.pdf, pdf.js:issue9084.pdf, pdf.js:issue9291.pdf, pdf.js:mixedfonts.pdf, pdf.js:noembed-eucjp.pdf, pdf.js:noembed-jis7.pdf, pdf.js:noembed-sjis.pdf, pdf.js:non-embedded-NuptialScript.pdf, pdf.js:prefilled_f1040.pdf, pdf.js:smaskdim.pdf
- R10-font-refiling (15): corpora:486832.pdf, corpora:499039.pdf, corpora:hesr13613-sup-0001-Authormatrix.pdf, pdf.js:SimFang-variant.pdf, pdf.js:XiaoBiaoSong.pdf, pdf.js:issue11718_reduced.pdf, pdf.js:issue11740_reduced.pdf, pdf.js:issue11768_reduced.pdf, pdf.js:issue12823.pdf, pdf.js:issue18117.pdf, pdf.js:issue215.pdf, pdf.js:issue9534_reduced.pdf, pdf.js:issue9949.pdf, pdf.js:issue_cff_unsigned_bbox.pdf, pdf.js:javauninstall-7r.pdf
- R11-truetype-route (32): corpora:032270.pdf, corpora:12884_2006_Article_120.pdf, corpora:225188.pdf, corpora:226387.pdf, corpora:427330.pdf, corpora:507676.pdf, corpora:PDFBOX-3067-negativeTf.pdf, corpora:sample_fonts_solidconvertor.pdf, corpora:source.pdf, pdf.js:SimFang-variant.pdf, pdf.js:TrueType_without_cmap.pdf, pdf.js:XiaoBiaoSong.pdf, pdf.js:bug1057544.pdf, pdf.js:bug1337429.pdf, pdf.js:bug1669099.pdf, pdf.js:bug1727053.pdf, pdf.js:bug894572.pdf, pdf.js:issue13211.pdf, pdf.js:issue13316_reduced.pdf, pdf.js:issue14117.pdf, pdf.js:issue14297.pdf, pdf.js:issue15910.pdf, pdf.js:issue17492.pdf, pdf.js:issue17808.pdf, pdf.js:issue20489.pdf, pdf.js:issue215.pdf, pdf.js:issue4800.pdf, pdf.js:issue5501.pdf, pdf.js:issue5701.pdf, pdf.js:issue6127.pdf, pdf.js:issue6961.pdf, pdf.js:issue7580.pdf
- R12-spot-colourant (7): pdf.js:colorspace_atan.pdf, pdf.js:colorspace_cos.pdf, pdf.js:colorspace_sin.pdf, pdf.js:devicen.pdf, pdf.js:issue17065.pdf, pdf.js:issue9940.pdf, pdf.js:postscript_type4_many_outputs.pdf
- R13-jpx-transcode (3): pdf.js:bug_jpx.pdf, pdf.js:issue5567.pdf, pdf.js:red_stamp.pdf
- R14-appended-page-composition (6): corpora:241573_ftp.pdf, corpora:JPXTestGrey.pdf, corpora:balloon_a1b_jp2k.pdf, corpora:breast+cancer+immunotherapy.pdf, pdf.js:PDFBOX-3148-2-fuzzed.pdf, pdf.js:issue20453.pdf
- R15-embedded-files (5): corpora:PDF-Sample-Document-Fully-Featured-Layout_Redacted.pdf, corpora:fileAttachment_fileAttachmentAnnotation.pdf, pdf.js:annotation-fileattachment.pdf, pdf.js:issue17056.pdf, pdf.js:issue18030.pdf
- R16-annotations-and-forms (22): corpora:MOZILLA-1671648-0.pdf, corpora:Montenegro2021.pdf, corpora:PDF-Sample-Document-Fully-Featured-Layout_Redacted.pdf, corpora:PDFBOX-6018-099267-p9-OrphanPopups.pdf, corpora:embedded_video_avi.pdf, corpora:embedded_video_quicktime.pdf, corpora:fileAttachment_fileAttachmentAnnotation.pdf, corpora:sakula.pdf, pdf.js:160F-2019.pdf, pdf.js:annotation-fileattachment.pdf, pdf.js:annotation_hidden_noview.pdf, pdf.js:annotation_hidden_print.pdf, pdf.js:bug854315.pdf, pdf.js:checkbox_no_appearance.pdf, pdf.js:issue16553.pdf, pdf.js:issue17069.pdf, pdf.js:issue7446.pdf, pdf.js:multimedia_annotations.pdf, pdf.js:pr7352.pdf, pdf.js:signed_verified.pdf, pdf.js:widget_hidden_print.pdf, pdf.js:xfa_filled_imm1344e.pdf
- R17-press-statements (4): corpora:102952.pdf, corpora:ESUjulyaugust2002.pdf, corpora:reserve3.pdf, pdf.js:endchar.pdf
- R18-piece-info (3): corpora:20789_ftp.pdf, corpora:225188.pdf, corpora:UMTRI-2008-80.pdf
- X1-dynamic-xfa (4): corpora:MOZILLA-1671648-0.pdf, pdf.js:issue14130.pdf, pdf.js:xfa_filled_imm1344e.pdf, pdf.js:xfa_issue14315.pdf
- X2-no-page-to-see (5): corpora:PDFBOX-6041-example.pdf, pdf.js:Brotli-Prototype-FileA.pdf, pdf.js:REDHAT-1531897-0.pdf, pdf.js:bug1020226.pdf, pdf.js:poppler-937-0-fuzzed.pdf
