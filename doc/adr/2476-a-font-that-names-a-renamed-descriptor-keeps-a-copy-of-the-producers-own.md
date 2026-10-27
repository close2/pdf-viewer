# 2476 — A font that names a descriptor the conversion renamed keeps a copy of the producer's own

Status: implemented. Session 1821, the archive slot of batch one hundred and twenty-two. Context:
ADR 2461 (the CJK descendant), ADR 2378 (re-keying), ADR 2179 (simple fonts sharing a descriptor
keep their own), `doc/history/1813` (the defect, found and left), ADR 2477 (why this and not the
ranking's first row).

## The defect

ISO 32000-2 Table 120 makes a descriptor's `/FontName` the name its fonts state:

> This name shall be the same as the value of BaseFont in the font or CIDFont dictionary that
> refers to this font descriptor.

The CJK pack's route and the re-keying route embed a face under the descriptor the font names and
rename both, `/BaseFont` and `/FontName`, for the face. A second font dictionary that names the same
descriptor object and that no route touched keeps the producer's `/BaseFont`, so its descriptor
then names a font it does not describe. `issue13343.pdf` has it: three descendants share one
descriptor; two are rendered and each is given a descriptor of its own (ADR 2461), and the third,
which no page shows, kept the shared object, whose `/FontName` had become the first subset's name
(`ETBXNA+QuorraCJK-Japan1` against `/Ryumin-Light`).

## The decision

`fonts::bystanders::keep_their_own_descriptors` runs once after every font is in. For every
descriptor a route gave a `/FontName`, every font dictionary in the document that names it and to
which the conversion gave neither a `/BaseFont` nor a descriptor is pointed at **a copy of the
producer's descriptor, as the producer wrote it** (one copy for all the bystanders of one object).
It embeds nothing: ISO 19005-2 section 6.2.11.4.1 asks a program of the fonts a document renders, and
no page renders this one. The walk is over every object, because Table 120 speaks of every dictionary
that refers to the descriptor, and it runs only where a route renamed one.

- **Not built**: a bystander written directly inside another dictionary has no object number to
  point a copy from, and keeps the renamed descriptor (the limit `fonts::gains` meets for a font that
  is not an object). No corpus document has one.
- **The fix is for both routes**, found by the descriptor's `/FontName` rather than by the pack's
  bookkeeping; the re-keying route renames only where its face is a Compact Font Format one.

## Proof

- `tests/archive/rekeyed.rs::a_font_that_shares_a_rekeyed_descriptor_keeps_the_name_it_states`: a
  serif Identity font and a sibling never shown naming its descriptor. With the call disabled the
  output's sibling reads `JZBHUQ+ChromSerifOTF` against `TimesNewRoman` (the test fails on Table 120);
  with it, it passes with the sibling's descriptor unembedded and the shown font's embedded.
- `tests/archive/cjk_pack.rs::a_descendant_nothing_shows_keeps_a_descriptor_stating_its_own_name`
  (CI's `cjk-pack` job; run here with a copy of the pack): failed with the call disabled, passed with it.
- `issue13343.pdf` at 2b and 4 under `as-if-printed` with the pack: the third descendant is
  `/BaseFont /Ryumin-Light` over a descriptor `/FontName /Ryumin-Light`; veraPDF 1.30.3 PASS at both.

## Consequences

- No census count moves: the sibling was already converting. Only the unrendered font's descriptor
  differs, in the documents that have one.
