# 1509 — The last walk-held rows take fixtures of their own construct, and the ratchet is at zero

Session 1337. Status: accepted. Amends ADR 1497 section 6 (the five rows it left). Code:
`crates/pdf-model/tests/tagged_pdf_fixtures.rs`, `crates/viewer-core/tests/objects_not_understood.rs`,
`tools/conformance/tests/conformance.rs` (`ONLY_WALKS_CEILING`).

## 1. What the brief said and what the text says

The brief described the five rows by guesses at their clauses. `doc/md/` disagrees on four of them,
and the fixtures follow the text: §14.8.2.5.2 is *Sequencing of annotations*, not the hierarchy in
general; §14.8.2.6.2 is *Identifying word breaks*, not `/Alt`; §14.8.6 is *Standard structure
namespaces*, not attributes; and §I.1 is *General*, whose one instruction is "ignore or inform",
while the `/Version` precedence the brief named is §I.2's (that row is not walk-held).

## 2. The fixtures, and where each expected value comes from

- **§14.8.2.5.2**: one page whose `/Annots` order, content-stream order and structure order all
  differ. The expected logical order is the tree's depth-first one, item for item, from "[t]he
  position of an annotation in the logical content order is determined from the document's logical
  structure"; the logical text holds no annotation's text, from "not interleaved within the page's
  content stream".
- **§14.8.2.6**: the clause's own soft-hyphen example by both routes it names, a `ToUnicode` entry
  and an element's `/ActualText` reached through the parent tree; each reads back U+00AD.
- **§14.8.2.6.2**: two show strings meeting inside a word read as one word, a SPACE at a string's
  end is the one break, and a word read across `/ActualText` takes the replacement — all with
  `inferred_separators` zero, NOTE 1's processor that need not guess.
- **§14.8.6**: §14.8.6.1's default namespace for an element stating no `/NS`, the PDF 2.0 name, and a
  foreign namespace counted alone as outside the standard.
- **§I.1**: the per-feature decision on one page. An image under a filter no clause of §7.4 defines
  is *informed* (a report names `Im0`, the fill beside it is drawn); an unknown page-dictionary entry
  is *ignored*, I.3's "behave as if they were not there". The assertion is on the object named, not
  on the sentence's reason, which today says "malformed image: stream did not decode" — a new
  filter is not damage under I.3, and the wording is a defect of `Document::image_stream`'s
  `Option`, which drops the reason; it is left for the round that owns `image.rs`.

Each fixture was calibrated by a mutation of its input (structure order swapped, `ToUnicode`
removed, a gap between the show strings, the PDF 2.0 element pointed at the foreign namespace),
and each failed.

## 3. The ratchet

`ONLY_WALKS_CEILING` falls from 5 to 0 and keeps its `==`: a new `implemented` row held only by
walks or witnesses now fails the build. §7.4.9, the one `partial` row in the same holding, named
its fixture files rather than their functions; it now names the functions (ADR 1510).

## 4. A blind spot found on the way

§14.8.6's notes test builds its own document, yet was classified a corpus witness: the
classifier reads a corpus root anywhere in a body, and `viewer_core::notes::about` names
`doc/pdf.js` in a comment, so every test calling `about` is a witness. Excluding comment lines
from the corpus match is the fix, in `tools/conformance/src/ledger.rs`, another round's file.
