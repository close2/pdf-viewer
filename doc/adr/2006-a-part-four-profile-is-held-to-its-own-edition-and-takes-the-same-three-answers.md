# 2006 — A part 4 profile is held to its own edition and takes the same three answers

Session 1585, the archive slot of batch ninety-three. Status: **accepted**, **built**. It carries
ADR 1990's three answers to the part 4 row `graphics/icc-profiles-carry-the-tags-their-version-
requires` (ISO 19005-4 section 6.2.4.2). Code: `crates/pdf-transform/src/archive/icc.rs`
(`Reading`); the reading is `pdf_archive::icc_version_tag_shortfall`.

## 1. The two parts choose the edition differently, and nothing else differs

ISO 19005-2 section 6.2.4.2 names four ICC editions and a profile need satisfy one; part 4 sends
the profile to ISO 32000-2 §8.6.5.5, whose last sentence holds it to the edition its own header
names. The validator already keeps the two as two rows, and the part 4 row judges a profile only
where its version names an edition this tree holds (2.2.0, 4.0.0, 4.4.0.0, 5.0.0.0); a 2.1.0
profile is not judged by it at all. So the converter's population at part 4 is that row's, read
by `icc_version_tag_shortfall` — the predicate's own three calls answered for one profile's
bytes, as `icc_tag_shortfall` is for part 2.

What a converter may do about a short profile depends on what this reader drew through it, not
on which text found a tag missing, so ADR 1990's answers apply unchanged: a profile drawn through
and short only of `desc` or `cprt` takes the operator's `supply`; one not drawn through is written
as its Table 65 alternate, with the output intent part 4's section 6.2.4.3 licenses a device
alternate by; one drawn through and short of a transform tag is refused. `Reading::of(target)`
chooses the population, and the text types ADR 1990 chose by version cover every held edition:
`textDescriptionType` below version 4, `multiLocalizedUnicodeType` from 4 — ICC.2-2023 section
9.2.106 gives `desc` that type alone.

## 2. What the profiles say

`as-shown`, `as-if-printed` and `keep-everything` state the part 4 row with the same two texts
as the part 2 row. `keep-everything` stated `preserve` with `via = "alternate"` and
`keep-rejected = "attach"`, which this version does not carry out; the alternate route needs no
answer at all (it is a statement), so the row now says what is built.

## 3. Evidence

No corpus document reaches the part 4 row (the veraPDF and the pdf.js census both count 0 at 4,
4f and 4e), so the witnesses are fixtures (`tests/archive.rs`): the shipped sRGB profile without
`cprt`, restated as 2.2.0 and as 4.0.0, converts to PDF/A-4 under the operator's text and its
display list is the source's; a profile with no transform, stated 2.2.0, is written as its CalRGB
alternate and draws the same; the same profile at 2.0.0 is not the row's to answer.
