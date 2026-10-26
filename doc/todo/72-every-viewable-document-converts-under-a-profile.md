# Every viewable document converts under a profile

Status: **open.** The project owner, 2026-10-15: "The pdf-a converter should be able to convert
*all* pdfs which are viewable", under a profile that is a lower bound on what may be lost, never an
instruction to lose it (ADR 1942). This file is the work that sentence still needs, in order.
Priority: the archive slot's standing list — an archive slot takes the top open item.
Instrument: `cargo run --release -p pdf-transform --example archive_unbuilt -- doc/pdf.js/test/pdfs
doc/corpora` ranks every site `--remedy-sites` counts as not built by the documents failing it,
per target, with its kind (`the-fence`, `not-built-yet`, `not-this-target`). The order below is
that ranking's; re-run it rather than trusting it, and the veraPDF corpus (`doc/veraPDF-corpus`)
separately, because its files are built to fail one rule each.
Catalogue: `doc/pdf-a-mitigations.md`, keyed by the same site identifiers.

## 1. Where a person chooses a profile

- **The window converts, chosen at launch** (ADR 1954): `--archive <target>:<profile>` and Shift+A
  in `quorra`, `quorra-gtk` and `quorra-qt`; `Command::Archive` asks Table 22's bit 11, the host runs
  the verb on a thread through `pdf_transform::profiles` and the one executor, says the report's
  lines and writes `<name>.pdfa-<target>.pdf` beside the document; `quorra_archive` in the C ABI.
  The JSON report carries `profile`. **Chosen after launch** (ADR 1966): Shift+A with nothing
  chosen offers the six targets and the shipped profiles with each one's sentence — `quorra`'s
  card, a GTK and a Qt dialogue — and the window keeps the choice. **Under a password** (ADR 1967):
  a document opened with one converts under it, from the window's `password::Kept`. **Owed**:
  choosing again in a window that has chosen; the confined window converts nothing, by argument.

## 2. The sites with no remedy, by how many documents reach them

Each line: the site, and what its remedy would be. `the-fence` rows are refused by argument
(ADR 0816) and leave this list only by an argued amendment or a departure the owner allows.

1. `optional-content/configuration-names` — **built as a `supply`** (ADR 1952): the three shipped
   lower-bound profiles name a configuration that states none `Configuration {n}`. Left: a name
   two configurations share (no corpus document does).
2. `graphics/only-operators-the-base-standard-defines` (fence) — an operator the standard does
   not define. §7.8.2 makes one an error outside a `BX`/`EX` section and has it ignored inside
   one (Table 33), so removing one *inside* such a section changes nothing a conforming reader
   draws; outside, what the page means is this reader's tolerance. Argue the first half.
3. `fonts/embedded-programs-define-every-glyph-shown` (fence) — a shown code the program has no
   glyph for: 29 pdf.js and corpora documents at 2b, 22 of the veraPDF corpus. The row asks
   `pdf_font`'s own `glyph_index`, which carries ADR 1943's step, so every code it counts is one
   that route already failed to reach; it cannot move the count further.
4. `graphics/no-transfer-function-in-a-graphics-state` — **built**: every `/TR` `Identity` is a
   statement (26 of 27 documents; ADR 1952 section 3), any other function the loss
   `transfer-function`, kept in `xmpMM:History` before it goes. Both shipped profiles say `stop`:
   a printer applies the function as a screen does, and the witness (`issue6931_reduced.pdf`)
   turns to a negative without it (ADR 1964). `/TR2` other than `Default` — **built** (ADR 1978
   section 6): written `Default`, the same statement or the same loss; veraPDF's
   `6-2-5-t02-fail-a.pdf` is its witness. Left: a function past the record's bound, refused.
5. `graphics/named-resources-are-defined` (fence) — a resource name the page uses and the file
   does not define. Read what ISO 32000-2 has a processor do with one before arguing a remedy;
   the catalogue's "none" rests on supplying an object being invention.
6. `file-structure/stream-filters-are-standard` — **built for Table 92's abbreviations** (ADR
   1952: the name written in full, the bytes kept). Left: `XXXDecode`, `BrotliDecode`, `DCTecode`,
   names this tree does not decode.
7. `implementation-limits/values-written-in-content-streams` (not this target) — part 4 lifts it.
8. `fonts/non-symbolic-truetype-differences-are-listed-names` (fence) — a `/Differences` name
   outside the Adobe Glyph List; ADR 1942's construction names the glyph by its `/ToUnicode`
   character instead, proved the same way — the same build widened.
9. `graphics/no-halftone-name` — **built** as the loss `halftone-name`, `discard` under `as-shown`
   (ADR 1965): `endchar.pdf` converts to 2b. `graphics/halftone-type-is-one-or-five` and part 4's
   `…-no-halftone-origin-…` — **built** as the loss `halftone` (ADR 2034): a threshold halftone's
   graphics states set `Default`, and `HTO` goes. Part 2's `…-no-halftone-phase-…` — **built** as
   a statement (ADR 2035): no base standard defines `HTP`. `graphics/no-overprint-mode-one-under-icc-cmyk` —
   **built** (ADR 1978): the states that set mode 1 for `ICCBased` marks state 0, a statement where
   they govern no `DeviceCMYK` mark (`2018-Lee-Fatherinclusiveperinatal.pdf`, whose next refusal
   is a standard-14 font) and the loss `overprint-mode` where they do (`102952.pdf`, which
   converts to 2b under `as-shown`); a mark in a four-component blending space is refused.
   `graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires` — **built** (ADRs 1979,
   1990): a profile drawn through that lacks only `desc` or `cprt` takes the operator's
   `description`/`copyright` (three shipped profiles state both; `issue14814.pdf` converts to 2b,
   pages identical), and one not drawn through is written as its Table 65 alternate with the
   output intent its device space needs (`435321.pdf`, refused elsewhere). Its part 4 row
   (`…-their-version-requires`) — **built** (ADR 2006): the same three answers, the profile held
   to the edition its header names; no corpus document reaches it, fixtures are its witnesses.
   The XMP edition (ADR 1991): part 2 identifies the XMP Specification only as its September 2005
   edition, so a predefined namespace's undefined name needs a container; `102952.pdf`'s 2b
   output now passes veraPDF. Then
   `file-structure/bound-names-are-valid-utf8`,
   `fonts/symbolic-truetype-program-has-a-usable-cmap`, `fonts/cid-system-info-agrees-with-the-cmap`,
   `fonts/non-symbolic-truetype-program-maps-every-code`, `graphics/inline-image-interpolation-is-off`
   — each reached by a handful; the catalogue entry says what each would be.
10. Part 4 only: `annotations/file-attachment-only-in-embedded-file-files` and
    `annotations/three-dimensional-only-in-engineering-files` (a retarget to 4f or 4e is the
    answer), `fonts/type3-glyph-procedures-state-their-width`, `fonts/actual-text-states-no-private-use`.
11. Level A only: `logical-structure/structure-tree-root` (fence — auto-tagging; 2u is the answer)
    and `logical-structure/role-map-terminates-at-a-standard-type` — **built as a `supply`** (ADR
    1990 section 6): the operator's `role-map` table; the eleven PDF 1.7 namespace types are
    standard (section 7), which the census's 2a count measured as unmapped.

**Over the veraPDF corpus** no pdf.js or `doc/corpora` document reaches a `not-built-yet` site
at 2b, so that corpus ranks them: `graphics/destination-profile-class-and-colour-space` (9) —
**built** where this reader does not draw through the profile and the slot is RGB (ADR 2007; four
witnesses convert, two drawn-through ones stay refused) — and
`graphics/halftone-transfer-function-only-where-required` (3) — **built** as `/TR`'s third row
(ADR 2007; its witnesses are PDF 2.0 files, refused at 2b by their header) — and
`…-carries-the-tags-its-class-requires` with `…-states-a-correct-profile-id` (2 each, one witness
failing both) — **built** where the header alone fails them (ADR 2021): a version 5 header on a
profile written in ICC.1:1998-09's tags and types is restated 2.2.0, a stale `Profile ID` is
written as the digest; `6-2-3-t01-fail-d` converts to 2b and 4 — and
`graphics/halftone-type-is-one-or-five` (2) — **built** as ADR 2034's loss, its witness
`6-2-5-t03-fail-a` holding one threshold halftone where the validator had counted three: two were
graphics states its `ExtGState` table names, now not halftones — and
`graphics/no-halftone-phase-in-a-graphics-state` and
`graphics/pdfa-output-intent-states-a-destination-profile` (1 each) — **built** as ADR 2035's
statements, a `Custom` PDF/A entry stating no profile given its construction's — and
`metadata/extension-schema-structure-fields-are-described` (1, `6-6-2-3-3-t03-fail-b`) — **built**
as ADR 2050's statement, the field described in its own value type; no `not-built-yet` site the
veraPDF corpus reaches is left at 2b or 2u. A part 4 output now writes the identification
namespace erratum #123 states, `http` (ADR 2051), which was the intent witness's one veraPDF
failure. `annotations/three-dimensional-only-in-engineering-files` (19 at 4) — **built** as the
subtype rows' loss and their `preserve` (ADR 2051 section 4), `6-3-1-t01-fail-d` converting to 4
with its poster drawn where it was under `as-shown`. At 2a,
`logical-structure/element-and-property-list-language-identifiers` (41 with the veraPDF corpus)
and `…/catalog-language-identifier` (22) — **built** as ADR 2066's loss, the entry stating
§14.9.2.2's unknown language, and its `supply` of the operator's tag, an inline property list
respelled in its content stream (ADR 2067); 32 of the 63 witnesses convert, veraPDF-compliant, and
the rest stop elsewhere. `annotations/file-attachment-only-in-embedded-file-files` (16 at 4) —
**built** as the subtype rows' removal with the file its `/FS` names filed in `/EmbeddedFiles`
(ADR 2067 section 4); `6-3-1-t01-fail-f` converts to 4 and 4e, veraPDF-compliant, its icon kept
under `as-shown`. The field description is also the operator's `supply` now
(`field-descriptions`, ADR 2082), the converter's sentence for any field it does not name.
At 2b, 2u and 2a the census ranks no `not-built-yet` site with a document. At part 4,
`graphics/no-destination-profile-reference` (1, `6-2-3-t04-fail-a`) — **built** where the intent
embeds its profile (ADR 2083), converting to 4 and 4e. The two duplicate-profile rows (6 and 2,
veraPDF's `6-2-4-2-t03-fail-a` to `-f` and `6-2-4-4-t01-fail-i`, `-j`) — **built** as ADR 2098's
statement, an `ICCBased` array over the profile in force at every use written `DeviceCMYK` in a file
that turns overprinting on nowhere; all eight convert to 4, veraPDF-compliant, pages byte-identical.
`graphics/icc-profiles-conform-to-the-base-standard` (2, `ISO 32000-1/6-2-3-2-t01-fail-b` and
`Undefined/6-2-3-2-t01-undefined-a`, both its `N` sentence) — **built** as ADR 2099's statement, `N`
written from the profile's header. At part 4 the census now ranks no `not-built-yet` site with a
document.

**The validation that did not end, ends** (ADR 1953): Isartor's `6-1-12-t01-fail-a.pdf` (ten
thousand pages in one `/Kids` array) cost 269 s per target in quadratic page lookups and costs
1.5 s with `Pages::every`'s one walk. No gate walks `doc/veraPDF-corpus`; the census over it is
the instrument (`examples/archive_unbuilt -- doc/veraPDF-corpus`).

## 3. What the two shipped lower-bound profiles still owe

**Not owed, by the owner's word**: an annotation stating no flags is not given Print by a run
without a profile (`doc/questions/A448`, which adopts the question's recommendation to keep it
as it is). The site,
`annotations/flags-entry-present`, asks for `--authorise annotation-printing`; the three keeping
profiles answer it with `preserve` (ADR 1942).

- **`as-shown`'s `printable-and-visible` split is built** (ADRs 2210–2211).
  `hidden-at-rest` discards Hidden/NoView with Invisible/ToggleNoView clear;
  `visible-nonprinting` preserves missing Print with forbidden flags clear.
  Mixed objects retain separate report decisions. Negative/malformed flags,
  Invisible/ToggleNoView and discarded widgets remain refused. Closed reciprocal
  hidden markup/Popup pairs may leave together only with independent discard answers,
  once-only same-page membership and no extra relationships (ADRs 2218–2219);
  other linked structures remain refused. Missing F keeps the independent
  flags-entry-present policy.
- **"As close as possible" at the forbidden subtypes.** `as-if-printed` discards them; the owner's
  rule prefers `preserve` (ADR 1123) with `discard` as the one `on-failure` alternative.
- **A font this machine draws from a face of its own** (ADR 0270), the reading (ADR 2162). The file
  carries no face and §9.5's NOTE 5 leaves the face of such a font to the processor, so embedding
  the shipped face loses nothing the file states. What it loses is what this viewer's window
  showed on this machine: the machine's shapes. A printed copy carries whatever face the machine
  that printed it had, so `as-if-printed` answers the loss `machine-face` with `discard`. `as-shown`
  keeps `stop`, since the window drew those shapes. No owner's word is needed: an operator who
  wants the machine's shapes kept names that face with `--font`, which is already the statement
  §9.9.1 asks for. Where the page shows a code the shipped face lacks, a mark would be lost, and
  both profiles refuse. A font that already states a descriptor and whose dictionary needs no
  restating — on this machine, every TrueType font whose name `substitute::find` matches to an
  installed face — is the same loss and is named by it (ADR 2279, which has the census).
- **The `DefaultCMYK` row's sentence is false for this renderer.** Pol.pdf's DeviceCMYK text moves
  by up to 34 of 255 under §10.4.2.5's conversion the converter writes; the sentence says this
  renderer already drew those colours so. Either draw DeviceCMYK the §10.4.2.5 way without an
  output intent (§10.4.2.1's order is the question) or say what changes.
- **Pol.pdf at 2u**: its Type 3 font names glyphs `/0`…`/7`, outside the Adobe Glyph List, and
  carries no `/ToUnicode` (section 6.2.11.7.2). **At 4**: DeviceRGB and DeviceCMYK on one page,
  and part 4 states no `DeviceN` half for CMYK, so a CMYK output intent is needed beside RGB.

## 4. What the two lower-bound profiles still refuse over the veraPDF corpus, by group

The instrument: `cargo test --profile gates -p pdf-transform --test archive_corpus --
--ignored --nocapture the_shipped_profiles` lists, per profile and target, every requirement that
refused a document, the kind of *no* (or `output-fails`, the second verdict's), the sentence's first
clause and the witnesses. Its counts are the record's (`doc/history/1639`), not this file's. The
groups, in the order a slot should take them:

1. **Remedies RFC 0007's shapes can still build.** (`metadata/extension-schema-container-fields`
   with nothing in the file to complete it was the largest group at 2b: its `discard` is built,
   and takes only the incomplete descriptions where the container is one bag holding complete
   ones beside them — ADR 2131.)
   `annotations/subtype-defined-in-iso-32000-1`
   under `as-shown`, whose `preserve` "by keeping" is section 3's owed item. The named part 4
   `6-2-4-3-t04-fail-n` case is **built** (ADR 2230): it has no catalog intent, a CMYK
   page intent and a neutral Gray page group. A closed all-context proof permits the
   existing `/DefaultRGB` writer beside the actually prepared shipped sRGB profile.
   Other independent targets, arbitrary consumers and producer/supplied RGB profiles
   remain outside that proof; this is not a general page-intent remedy.
2. **Converter defects, owed a fix and a test.** None is open. (`6-6-1-t01-fail-m`'s `Trans`
   action in a widget's `/AA` was the last: the edit was tagged with the `/AA` rule nobody failed,
   so the rewriter skipped it, and it is now the type rule's. A PostScript XObject the page invokes
   was the other: ADR 2115 writes it as a form that draws nothing.)
3. **The colour family the profile in hand cannot answer.** `DeviceCMYK`, or a group blending in
   it, beside an RGB intent at part 4, and such a group at part 2: no profile ships a press, so
   these stay refused under both profiles — part 4's `DeviceCMYK` row with the sentence naming the
   flag, the group, base and alternate rows by the output's verdict; the operator's
   `--output-intent-profile` with the press's CMYK profile answers every one, as an `ICCBased`
   `/DefaultCMYK` beside the file's own intent (ADR 2130). `a_supplied_press_profile_answers_the_
   colour_family_refusals` is the census with one supplied. `DeviceRGB` inside a group blending in
   a CMYK or grey `ICCBased` space remains item 1's context-proof obligation, except
   the narrow page-intent case already handled there.
4. **Refusals the standard forces.** The content-stream fence (ADR 0816): a `.notdef` glyph or a
   code the embedded program lacks, an undefined operator, an inline image's filter or intent, a
   hexadecimal string's digits, a content stream naming resources it has no dictionary for, an
   annotation with no appearance whose subtype clause states no artwork. Section 6.1.13's limits
   at part 2 (`not-this-target`; part 4 lifts them). Data outside the file nobody handed over.
   A PDF 2.0 header at part 2. A part 4f file with no attachment, a 3D stream neither U3D nor PRC.
5. **Losses both profiles leave at `stop` by their yardstick.** Transfer functions, halftones under
   `as-if-printed`, image interpolation: section 3's argument stands, and an operator's
   `--authorise` answers each.
6. **`not-built-yet` sites each reached by one to three witnesses**: the JPEG 2000 rows, the
   composite font rows (`cid-to-gid-map-present` for a file labelled PDF 2.0, ADR 2246 writing it
   for a 1.x label, `embedded-cmap-states-its-own-write-mode`,
   `font-programs-embedded`'s shapes, the TrueType route), `spot-colourants-appear-in-the-colorants-dictionary`
   with no Separation of its own, `graphics-state-blend-modes-are-defined`, `no-reference-xobjects`,
   `normal-appearance-shape`, the `to-unicode` rows at 2u, `role-map-terminates-at-a-standard-type`
   at 2a. The catalogue entry of each says what its remedy would be.

The other corpora — pdf.js's test documents and `doc/corpora`'s four collections, files real
producers wrote — are swept at each part's lowest level by
`the_shipped_profiles_list_what_real_producers_files_refuse_on`, which prints the same census and
names as `FAILED` any document the verb errors or panics on. Its counts are `doc/history/1655`'s.
Its largest refusal, a standard font stating no descriptor, is built (ADR 2146), and so is the
Table 124 pairing a shipped face of the other kind met (ADR 2147). `fonts/font-programs-embedded`
no longer refuses over the viewer's horizontal fitting: removing it is the loss
`substitute-stretch` on every route that embeds the shipped face (A566, ADR 2262), `as-if-printed`
discards it and `as-shown` stops on it, and one decision names it beside `machine-face` where a
document needs both. Of ADR 2179's eight, four convert; the other three failed the output's second
verdict on a font rule the fitting refusal had hidden, each the converter's, and now convert under
`as-if-printed` (ADR 2278): a `Differences` array naming `.notdef` is restated once the dictionary
is `TrueType`, and a descriptor flagged Symbolic beside the Latin encoding it is given says
Nonsymbolic. The two Symbol refusals had a missing
code-to-name table when TrueType became CFF; stating the source selections clears that font
refusal, while their independent content-stream fences remain. The direct-resource and
machine-face cases were addressed in ADR 2162; `machine-face` remains a named loss, accepted by
`as-if-printed` and stopped by `as-shown`.
Its largest refusal at part 4 over the two corpora together, an embedded Type 2 CIDFont stating
no `/CIDToGIDMap` in a file labelled PDF 1.x, is built as a statement under both profiles (ADR
2246). Its largest in pdf.js alone, a composite font nobody named a program for, stays refused by
name: every shipped face is a simple font's, and `doc/history/1706` says what a remedy would need.
The next two are answered (ADR 2294). `fonts/to-unicode-values-are-usable` at part 4 — 32 pdf.js
composite fonts whose `CMap` states U+0000, U+FEFF or U+FFFE, nothing derivable from a CID — is a
restatement of the producer's `CMap` without those statements, which part 4 admits because it only
recommends a mapping; part 2 at Level U and A takes it only where no shown code loses its mapping.
The content-stream fence's 33 `doc/corpora` documents are damaged streams (§7.8.2 makes the page an
error), named in the mitigations entry and refused. The five part 4 outputs veraPDF failed at
6.2.10.4.1 were none of them a `FontFile3` with no `Subtype`: three hold a program filed under a key
or name its bytes contradict, which `fonts/font-programs-filed-by-their-format` refiles
(`bug921409`, `issue5751`) or refuses (`issue9949`, a `glyf` program under `CIDFontType0`), and two
hold a TrueType collection under `FontFile2` (`issue13193`, `issue9262_reduced`), which the
TrueType Reference Manual's font file — one font, opening with its own font directory — is not, and
which no row of Table 124 admits: both are refused by a sentence naming the collection (ADR 2327).
**Owed** for them: the face this reader draws, chosen by the descriptor's `/FontName` (ADR 0141),
written out of the collection as a TrueType program of its own. The row cost pdf.js four documents
(ADR 2307 section 3): `issue9949`, `issue11740_reduced` (a Type 1 program under `CIDFontType0`,
veraPDF agrees), and two CFF-flavoured `OTTO` programs under `/FontFile2` in a `CIDFontType2`. Of
those two, `issue5084`'s `/CIDToGIDMap` is `Identity` over a name-keyed CFF, so the dictionary is
retyped `CIDFontType0` with every CID selecting the glyph it selected (§9.7.4.2, ADR 2326), and it
converts; `bug1186827`'s map is a stream sending CID *c* to glyph *c* − 31, so under `CIDFontType0`
its CIDs would select other glyphs — **owed** for it: re-keying the encoding and the widths to the
glyph indices.

veraPDF's two disagreements on `me450f12project10_report` were this validator's (ADR 2295): an
`xmpMM` structure is now held to the September 2005 edition's `ResourceEvent`, `ResourceRef` and
`Version` fields, and a stream with no profile file signature is short of it at part 2. The
edition's other structured types are held to their fields the same way, and part 4's tag row finds
a stream that is no profile short of the signature as part 2's does (ADR 2318); neither corpus holds
a structure of the new types outside its fields.

The next refusal by count under `as-if-printed`, `annotations/normal-appearance-shape`'s pdf.js
documents, was a push button's single normal appearance stream in every one, which the census's
sentence had named as a subdictionary with no `/AS`: a push button's stream is now the one entry of a
subdictionary its new `/AS` selects (ADR 2319), and the check box and radio button half keeps a
sentence of its own. The one larger, a composite font nobody named a program for, waits on Q570.

Table 109's widths-array count is checked independently for simple fonts. A nonempty short
array narrows LastChar; a long array loses only entries beyond it. This `Stated` rewrite keeps
all effective advances and does not replace dictionary widths with a font program's metrics
(ADR 2178). Empty arrays, missing arrays, malformed byte ranges and Type 3's separate rule are
not invented into conformance. Fonts sharing a descriptor now retain their own proved
programs through separate descriptor copies rather than overwriting each other's metrics
(ADR 2179).

Owner answer A550 is implemented by ADRs 2194–2195: the archive font emitters attach
`LICENSE_FOXIT` as exact default-language XMP UsageTerms on bundled Foxit program
streams. Modified Liberation programs replace reserved active identities with
QuorraText while preserving legal records and all glyph tables. Unmodified copies
remain unchanged. Existing different usage terms or unreadable name storage refuse
rather than discarding information.
