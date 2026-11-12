# 2812 — What the specification audit found in the model core is read again from the clauses

Round 2000, the audit-fix round (group MC, model core).
Status: **accepted and built** in `pdf-model` (`attachment/filing.rs`, `view/writing.rs`, `forms_data.rs`,
`content/marked.rs`, `content/run/marked_ops.rs`, `content/ext_gstate.rs`, `content.rs`,
`content/text/type3.rs`, `content/run/compat_type3_ops.rs`, `content/transparency/soft_mask.rs`,
`content/resources.rs`, `content/pattern.rs`, `image/dispatch.rs`, `image/mask.rs`, `image/space.rs`,
`named_page.rs`, `content/run/nested.rs`, `content/pattern/tiling.rs`) and `pdf-syntax` (`text_string.rs`).
It answers findings of `doc/reviews/1983-specification-audit.md`, the owner's audit of 2026-11-10, whose instruction
of 2026-11-11 — "Please use a round to fix all findings" — is the decision each section below records.
Supersedes, for the points named: ADRs 0343 and 0359's "only fewer marks" argument for drawing a damaged prefix
(the policy stands), ADR 0814 (an attachment's `/Desc` is optional in the tree), ADR 0091's named-page
diagnostic (a template's absent `/Type` unreported), and the reading in `image/space.rs` that a base image's `/Decode`
of the wrong length is honoured as far as it goes. Qualifies trap 11's missing-resource condition.
Clauses: ISO 32000-2 §7.9.2.2.2, §7.9.2.3, §7.11.3 (Table 43), §8.9.5.1 (Table 87), §8.11.3.1, §9.6.4 (Table 111),
§12.7.7, §12.7.8.3.1 (Table 246); Errata Collection 3 Issue #215.

## 1. A file specification in `EmbeddedFiles` carries `/Desc`

Table 43, `/Desc`: "It shall be used for file specification dictionaries referenced from the EmbeddedFiles name tree".
ADR 0814 wrote `/Desc` "where given", which left a document-level attachment with no description writing a
specification the table says shall carry one. The writer now supplies the filing's name as the description where the
person gave none (`filing::tree_description`): it is text the person supplied, so nothing is invented, and it is the
one description every filing has. A page-level attachment is unchanged — its description goes to the annotation's
`/Contents` (Table 187) and the table's sentence does not reach it. Refusing an undescribed filing was the other
compliant answer and was not taken, because it turns a missing nicety into a lost attachment.

## 2. A language escape in `/ActualText` is a language, and an FDF `/Status` is `PDFDocEncoding`

§7.9.2.2.2: escape sequences "indicate the language in which subsequent text shall be written", and §14.9.2.1 has
such a sequence override "the prevailing Lang entry". The escapes were stripped and their languages discarded.
`pdf_syntax::text_string_languages` now returns each escape's language with the offset in the decoded text from which
it governs, and a replacement text's runs are recorded as §14.9 spans nested inside the section's own, so the inner
statement wins as §14.9.2.3's hierarchy has it. `/Alt` and `/E` keep their current reading (an escape removed, the
section's language spoken); a phrase that changes language mid-string is not yet split.

Table 246 types `/Status` a *PDFDocEncoded string*, and §7.9.2.3 makes that "a character string in which the
characters shall be represented in a single byte using PDFDocEncoding". Bytes 254 255 there are `þÿ`, never a UTF-16BE
marker; `pdf_syntax::pdf_doc_encoded_string` decodes the type, and is the only entry ISO 32000-2 types so.

## 3. A `d1` glyph ignores transparency set by `gs`

Table 111, `d1`: the description "should not execute any operators that set the colour (or other colour-related
parameters including transparency) in the graphics state; any use of such operators shall be ignored and the glyph
stream continues to be processed without error". §8.6.8's list, which this tree already applied to `d1` and to an
uncoloured tiling pattern alike, names no transparency entry; Table 111 adds them for `d1` alone. A new interpreter
flag, `shape_only_glyph`, is raised by `d1` beside `uncoloured`, saved and restored with it, cleared inside a soft
mask's group as `uncoloured` is (ADR 0173), and `gs` skips `/CA`, `/ca`, `/BM`, `/SMask`, `/AIS` and `/TK` under it.
**`issue19634.pdf` changes**: its `d1` glyph's `gs` names a luminosity soft mask, and Table 111 now has that mask
ignored, so the glyph draws unmasked.

## 4. A missing resource that sets graphics state is reported inside hidden content

Trap 11 suppresses a missing-resource report inside hidden optional content, because a skipped `Do` "cannot have
failed". §8.11.3.1 keeps the other half: "Graphics state operations, such as setting the colour, transformation
matrix, and clipping, shall still be applied." A `gs` or a pattern `scn` naming nothing inside a hidden section leaves
a state the visible marks after `EMC` are painted with, so those two lookups report whether or not the content is
hidden (`note_missing_state_resource`). `Do`, `sh` and the inline image keep trap 11's condition.

## 5. An image dictionary whose entries disagree is refused, and an image mask's `/Mask` is ignored

§8.9.5.1: "Attempting to use an image dictionary whose entries are inconsistent with each other or with the current
colour space shall cause an error." Table 87 states two relationships this tree repaired in silence: a base image's
`/Decode` "length shall be twice the number of colour components required by ColorSpace" (an array short was padded
from Table 88, one long was truncated), and an image mask's `/BitsPerComponent`, "if specified, its value shall be 1"
(2 was read as 1). Both are now refused before a sample is read, naming the entry (`dispatch::inconsistent_entries`),
as is an image mask's `/Decode` that is neither `[0 1]` nor `[1 0]`. A `JPXDecode` image stating no `/ColorSpace` is
exempt, because the same row has its array "ignored unless ImageMask is true". A soft mask's or an alternate's array
is outside the clause, which speaks of "a base image", and keeps the old reading.

Errata Collection 3's Issue #215 has a reader ignore `/Mask` and `/ColorSpace` that a producer wrote beside
`/ImageMask true`. A stream-valued `/Mask` was applied, and suppressed the stencil's marks; it is now not read, and no
longer counts as an image mask overriding the graphics state's soft mask.

## 6. A template with no `/Type` is not a template

§12.7.7: "Such invisible pages shall have an object type of Template rather than Page". ADR 0091's diagnostic named a
template whose `/Type` was a name other than `Template` and passed one with no `/Type`, or one whose `/Type` is not a
name, as though it stated the type. Both are now named, each in its own words.

## 7. Drawing a damaged stream's prefix is a disclosed choice, not a subset of the page

ADRs 0343 and 0359 draw what a damaged filter delivered of a content stream and report the damage, and argued it from
the prefix being "a shorter sequence of the same kind, made of bytes the producer's own encoder emitted", so that the
drawing could only omit marks. The audit refuted that argument twice: corrupt compressed bytes need not decode to a
prefix of what the producer wrote, and a prefix of a sequence whose later paint covers its earlier marks shows marks
the whole would have hidden — its witness is a luminosity mask group whose intact suffix leaves the page white (255)
and whose loss leaves `/BC`'s value there (64), ink the producer's page does not have. The policy stands, as the audit
found it may: it is reported every time, and no clause of ISO 32000-2 asks a reader for any particular recovery. The
reason changes, in the comments that stated the old one (`content/run/nested.rs`, `content/pattern/tiling.rs`,
`content/transparency/soft_mask.rs`, `tests/damaged_content_streams.rs`): the drawing is a recovery this tree chooses
and discloses, and it is not a subset of the producer's marks.

## 8. A `Tf` label spelling a standard font's name is a guess, and a retired sentence is not a warrant

ADR 0183 drew a `Tf` whose label names nothing in `/Font` from the compiled-in fourteen when the label spells one of
§9.6.2.2's names, and argued that such a stream "named something the standard permits it to name". It did not: the
operand is a key in the resource dictionary, the fourteen names are `/BaseFont` values, and no clause reserves the one
as the other. The behaviour stands as a recovery heuristic — the guess needs only the one-entry dictionary Table 109
admits for the fourteen — and `content/font.rs::standard_font_named` now says so. In the same family,
`pdf_font::loading::opening::published_metrics` quoted §9.6.2.1's compatibility `shall`, which Errata Collection 3's
Issues #47 and #48 replaced by a cross-reference (ADR 2687 quoted it too); the warrant it now cites is Table 109's
"optional in PDF 1.0-1.7 for the standard 14 fonts" with §6.3.2.2, and the behaviour is unchanged.

## 9. An informative NOTE explains the anti-aliasing and area-averaging departures; it does not license them

ADR 0025 kept area averaging for a reduced image against §10.7.4's "There shall not be averaging over the pixel area",
and wrote that §10.7.1's NOTE — "The specifics of the scan conversion algorithm are not defined as part of PDF" —
"licenses all three" departures. A NOTE is informative and cannot set aside a `shall`; the audit qualified the claim
and kept the choice, and so does this ADR. `pdf-render`'s `paint.rs`, `edge.rs` and `sub_pixel.rs` now call both
departures documented choices that the NOTE explains, with ADR 0025's cost and benefit unchanged.

## What this costs

Each section changes what a malformed file draws or reports, never what a conforming one draws, except §3's: a `d1`
glyph that sets transparency is a file the table discourages, and it now draws as the table says. The image refusals
of §5 take pages off the oracle's judged set where a producer wrote such an array; a refusal names its entry, which is
the trade §8.9.5.1 makes.
