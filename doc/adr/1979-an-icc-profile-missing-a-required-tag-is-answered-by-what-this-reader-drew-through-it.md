# 1979 — An ICC profile missing a required tag is answered by what this reader drew through it

Session 1571, the archive slot of batch ninety-one, on `doc/todo/72` section 2 item 9.
Status: **accepted**, **not built** — the reading and the two answers, for the archive round that
builds them. Context: ADRs 0948, 0950, 1952; `doc/pdf-a-mitigations.md` section 4.2.

## 1. The site, its witnesses, and which tag

`graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires` (ISO 19005-2 section 6.2.4.2:
an `ICCBased` space's profile conforms to one of four ICC editions) is checked against the two this
tree holds, ICC.1:1998-09 and ICC.1:2001-12, each of whose clause 6.3 lists a class's required tags.
**The brief's premise named ISO 15076-1; the check reads neither it nor ICC.1:2003-09**, which is
the unchecked row beside this one. Two documents reach it at 2b, 2u and 2a, and they are two cases:

- `issue14814.pdf`: a version 2.3 `mntr` RGB profile by lcms carrying `desc`, `wtpt`, the six matrix
  and curve tags and `chrm`, and **no `cprt`** — ICC.1:1998-09 section 6.4.11's copyrightTag, "the 7
  bit ASCII text copyright information for the profile". Nothing else is missing.
- `435321.pdf`: two version 2.1 `mntr` RGB profiles whose tag table is unreadable past its first
  entry — seventeen entries, one signature (`cprt`) and sixteen of bytes that name no tag — so the
  colour data's own tags are absent as far as any reader can tell.

## 2. What supplying the tag would change

`pdf_colour::icc` reads `cprt` only into `Identification::copyright`, for a converter's report;
`Profile::parse` builds the transform without it. So for `issue14814.pdf` **no colour this reader
computes depends on the tag**, and the brief's hypothesis — a tag whose absence the engine already
reads as its default — holds in the stronger form: the engine does not read it at all. For
`435321.pdf` the missing tags are the transform, `Profile::parse` returns `None`, and the space is
drawn through §8.6.5.5's Table 65 `/Alternate`, "used in case the one specified in the stream data
is not supported" — or, with none, the device space of its `N`.

## 3. Decision

1. **A profile this reader draws through, missing only text tags (`desc`, `cprt`)**: the tag is
   written into the profile, a `textType` in a version 2 profile and a `multiLocalizedUnicodeType`
   in a version 4 one, every byte of the colour data kept and the header's size and tag table moved
   by the entry; a non-zero profile ID is recomputed by ISO 15076-1 section 7.2.18's method. What
   the text says is not the file's to give, so it is a **supply**: the operator's sentence
   (`text = "…"`), the shipped profiles saying that none was stated, and the report and
   `xmpMM:History` naming the profile and the tag. No colour moves.
2. **A profile this reader does not draw through** (`Profile::parse` refuses it): the space is
   written as its `/Alternate`, or the device space of its `N`, which is Table 65's own answer and
   what the page already showed — a statement, ADR 0948's class. The array is reached where the
   finding names its holder, and a holder this conversion cannot site is refused by name, as the
   duplicated-profile rows are.
3. **A profile this reader draws through, missing a tag its transform needs**: refused by name —
   supplying a white point or a curve would be inventing the producer's colour.
