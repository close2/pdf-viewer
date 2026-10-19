# 1990 — A short ICC profile is given the operator's text, or written as its alternate

Session 1577, the archive slot of batch ninety-two. Status: **accepted**, **built**. It builds
ADR 1979's decision for `graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires`
(ISO 19005-2 section 6.2.4.2) and settles what that decision left to the building round.
Code: `crates/pdf-transform/src/archive/icc.rs`; the reading is `pdf_archive::icc_tag_shortfall`,
so the converter acts on exactly the list the row reports.

## 1. Which profile takes which answer

A profile is *drawn through* where `pdf_model::colour` draws through it: the stream decodes whole
(no `Decoded::damage`) and `Profile::parse` accepts it. A damaged profile is drawn in its alternate
there, so it is an unreadable one here too — otherwise a supply would re-encode undamaged bytes and
the output would be drawn through a profile the input was not.

- **Drawn through, short only of `desc` and `cprt`**: a `supply` (`Rewrite::ProfileTextSupplied`).
- **Not drawn through**: a statement (`Rewrite::ProfileAlternateWritten`), §8.6.5.5's Table 65 —
  the `/Alternate` as written, else the device space of `N`.
- **Drawn through, short of a transform tag**: refused, as ADR 1979 section 3.3 says.

Four further refusals, each by name: a file holding the first two kinds together (two answers, one
the operator's and one the converter's, and this version carries out one per requirement); an
unreadable profile that is a `DefaultGray`/`DefaultRGB`/`DefaultCMYK` value (a device space
substituting for itself is not what §8.6.5.6 describes); one stating neither `/Alternate` nor an
`N` of 1, 3 or 4 (the reader's RGB there is a guess, not the file's statement); and a failure the
walk did not reach.

## 2. The supply, refined from ADR 1979

- **Two keys, `description` and `copyright`**, rather than ADR 1979's one `text`: a profile missing
  `desc` given the copyright sentence would describe itself wrongly. A tag whose key the row does
  not state keeps the refusal. Both are 7-bit ASCII, refused at the configuration otherwise,
  because ICC.1:1998-09 sections 6.5.16 and 6.5.17 hold a version 2 profile's text to it.
- **The tag types are the edition's**: `textType` and `textDescriptionType` (with section 6.5.16's
  own guidance for an unlocalised description: Unicode and ScriptCode counts zero) under major
  version 2, ICC.1:2001-12 section 6.5.14's `multiLocalizedUnicodeType` above it. Its one record
  is `en`/`US`, **a documented choice**: the clause asks for an ISO 639 and an ISO 3166 code and
  the configuration states neither.
- **The table grows, nothing else moves**: one entry per tag, every existing offset moved by the
  twelve bytes an entry is (a multiple of four, so ICC.1 section 6.2.2's alignment holds), the new
  data after the old on a 4-byte boundary, the size field restated, a stated profile ID recomputed.
  A table naming data inside itself or past the end is not moved.
- **Shipped answers**: `as-shown`, `as-if-printed` and `keep-everything` state both keys, each a
  sentence saying the profile stated none and the text was supplied at archiving. The report and
  `xmpMM:History` name every profile and its text (`doc/rfc/0007` section 5b.1).

## 3. The alternate brings device colour, and its output intent

Table 65's alternate is usually a device space, and ISO 19005-2 section 6.2.4.3 licenses one only
through a PDF/A output intent of its family (grey: any). The input did not use that device space,
so no device row failed and no intent would be prepared. So `Prepared::of` adds the device rows
the alternates bring to the set its preparations read, and the conversion wants
`Rewrite::OutputIntent` beside the alternate; an intent of another family (or none) refuses by
name. The intent's own statement then applies: DeviceRGB drawn through the shipped sRGB profile
comes back within a 255th (`doc/adr/0948`'s class, the same for every DeviceRGB file).

## 4. Pixels (trap 182)

`issue14814.pdf` → 2b under `as-shown`: both pages byte-identical at 72 and 150 dpi before and
after, veraPDF 1.30.3 PASS. `435321.pdf` is answered at this row (six arrays, two profiles,
DeviceRGB alternates) and stays refused for a fenced operator and a font. The fixtures in
`tests/archive.rs` hold the rest: display lists identical through a CalRGB alternate and through a
supplied `cprt`, within a 255th through DeviceRGB under the added intent.

## 5. Left

The part 4 row (`…-their-version-requires`) and the other two ICC rows stay `not-built-yet`; the
part 4 row could take the same two answers once a document reaches it.

## 6. The census's next site: the role map, on the operator's table

With the ICC row built, the census ranks no `not-built-yet` site at 2b; at 2a the first is
`logical-structure/role-map-terminates-at-a-standard-type` (ISO 19005-2 section 6.7.3.4), whose
catalogue entry has always been a `supply`. Built (`archive/roles.rs`, `Rewrite::RoleMapSupplied`):
an element's `/S` is followed through the producer's own `/RoleMap` as `Tree::role` follows it, and
the type the chain ends at, neither standard nor mapped, gains the entry the operator's `role-map`
names — every entry the producer wrote kept, written into the root's direct map or into the map
object it names. Refused by name: an element with a `/NS` (§14.8.6.2 gives it its namespace's own
map), a chain ending at a type the map already names (a cycle, or an array target), a type the
table does not name, a target that is not a standard type, a truncated walk. `keep-everything`
keeps its empty table, which stops; no shipped profile can know a producer's vocabulary.

## 7. The premise under that site: the validator's vocabulary was PDF 2.0's alone

The row judged an element standard by `pdf_model::structure::StandardType`, which is ISO 32000-2
§14.8.4's PDF 2.0 vocabulary. But §14.8.6.1 makes the standard structure namespace for PDF 1.7
the *default* one — an element stating no `/NS` is in it — and Annex M lists the eleven types it
holds beyond PDF 2.0's: `Art`, `BlockQuote`, `TOC`, `TOCI`, `Index`, `Private`, `Quote`, `Note`,
`Reference`, `BibEntry`, `Code`. Those are also ISO 32000-1's own, the base document part 2 names.
So `TOC` or `Note` in a Word export was reported unmapped. `pdf_archive::standard_structure_element`
now accepts them in the default namespace (and the converter asks the same function); an element
naming PDF 2.0's namespace is still held to PDF 2.0's vocabulary. `pdf_model`'s own reading of the
default namespace is the reader's, not this slot's, and is left as a finding (`standard_role`
answers `None` for a `Note` with no `/NS`).
