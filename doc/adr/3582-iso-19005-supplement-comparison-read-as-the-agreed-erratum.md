# 3582 — ISO 19005's supplement comparison read as the agreed erratum, and the converter's answer

Session 2220. Status: **accepted**. Context: ADR 3581, ADR 2717 (the collection restated), ADR
3447 (the four departures drafted), veraPDF-library #1668, pdf-association/pdf-issues #77; ISO
19005-2 6.2.11.3.1, ISO 19005-4 6.2.10.3.1 (paraphrased, `doc/third-party-data.md`); ISO 32000-2
§9.7.3, Table 114, Table 118.

## The correction

Both clauses, as printed, have a `CIDFont`'s `Supplement` at least its `CMap`'s. The PDF/A TWG
agreed on 2022-02-03 that this was wrong (#77, petervwyatt's comment of that date) and the site
publishes the correction to 6.2.10.3.1 — *less than or equal to* — in `doc/errata/site/docs/
19005-4-2020/clause06.md`. Part 2 has no errata page; it prints the same sentence and #77 names
"all PDF/A specifications starting from PDF/A-2", so the correction is read into both. veraPDF had
followed it all along, which is what #1668 asked it to undo; MaximPlusov closed #1668 on
2026-11-24 pointing at the erratum. Under ADR 3581 the corrected sentence governs.

The reason #77 gives is a `CMap`'s lifetime: supplements only add CIDs, so a `CMap` of a later
supplement still means every CID of its font's, and a producer states for its font the least
supplement covering its glyphs.

## What changed

1. **`fonts/cid-system-info-agrees-with-the-cmap`** fails a `CIDFont` whose supplement is
   *above* the `CMap`'s. The four veraPDF test-suite files `pdf-archive`'s corpus test had ruled
   against the corpus (`6-2-10-3-1-t01-pass-a`, `-fail-c`, `6-2-11-3-1-t01-pass-d`, `-fail-c`) now
   agree with this crate and their adjudications are deleted.
2. **A predefined `CMap` is compared too**, by the collection its own program states
   (`pdf_font::predefined::collection`, new). The row used to pass predefined names over because
   reporting them under the printed direction would have failed "a great many conforming files";
   under the corrected direction that population conforms, and the reason is gone. #77's other
   question — whether a validator should take a predefined `CMap`'s supplement from the newest
   collection a processor must support — stays open there; the program's own statement is the
   lower bound, so a font meeting it meets either reading.
3. **The converter's answer** (`pdf-transform`'s `collections`, `Rewrite::CollectionRestated`):
   where the `CIDFont` and the `CMap` name one registry and ordering and only the supplement is
   above, the descendant is given the `CMap`'s supplement, for every font, embedded or not, and for
   a `Type0` font written directly in a resource dictionary when its descendant is an object of its
   own (`pdf_archive::cid_fonts_above_their_cmaps`, new). Nothing reads a `CIDFont`'s supplement:
   Table 114 says it shall not be used to decide compatibility, and `pdf_font::composite` does not
   read it. Every CID the `CMap` reaches is in the `CMap`'s own supplement, which is the least
   supplement #77 has a producer state. The other constructions weighed: a predefined `CMap` of the
   font's supplement with the same mapping — `data/cmaps` names none for `EUC-H`, `H` or
   `90ms-RKSJ-V`, and proving two programs' mappings equal is work a supplement does not need; and embedding Adobe's program with its stated supplement raised,
   which edits a published program under its published name to say the same thing less directly.
   A predefined `CMap` of another registry or ordering is refused by name (`PREDEFINED_ELSEWHERE`).
4. **`cmaps::collections_agree`**, which decides whether a published unlisted `CMap` may be
   embedded under a font, compares registry and ordering only; a supplement above the program's
   is the finding of item 1 and answered by item 3.

## The three documents

`noembed-eucjp` (`EUC-H`, Japan1-1, under a Japan1-2 descendant), `noembed-jis7` (`H`, Japan1-1,
under Japan1-2) and `issue11555` (`90ms-RKSJ-V`, Japan1-2, under Japan1-6, the `Type0` font written
directly in the page's resources) convert under `as-shown` and `as-if-printed` at 2b and 4 with the
descendant's supplement restated to the `CMap`'s; veraPDF 1.30.3 passes all twelve outputs.
