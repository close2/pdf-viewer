# 827 — A quantity the clause hands over

Date: 2026-08-29. A general-improvement round in a worktree from `main` at `3c259925`, branch
`round-827`. One subject, one ADR (0756), one instrument changed and no pixel touched. `doc/rfc/`
and `doc/todo/56` were not touched: both await the owner. Not merged.

## Choosing, and what the instruments said first

The demand side was asked before anything was written, and most of it turned out to be quiet or
somebody else's.

- **The corpus gate's own classification names exactly one mechanism as this reader's**:
  `a /DA font stood in for that cannot draw the value's script`, one document,
  `freetext_no_appearance.pdf` — which is `doc/todo/22`'s single remaining entry, read, priced and
  pinned by ADR 0348 on an Arabic glyph source this binary does not have. So the corpus's
  reader's-fault row is not a round's work; it is a project's, and it is already argued.
- **The oracle's contradicted pool, the errata ranking and the CMS corpus were the three sibling
  lanes** and were left alone.
- **The text gate's second instrument was not.** `the_word_boxes_we_place_agree_with_the_references`
  printed twenty-one documents not fully in bounds, and six of them shared one constant: every
  matched word's left *and* right edge exactly 1.00 pt from `pdftotext`'s, on a page whose text is a
  form field's value.

## What the six were

`/NeedAppearances true` text fields, plus one `/FreeText` with no `/AP`. Both readers construct the
appearance, and §12.7.4.3 hands the position over — the `/BBox` is fixed at the origin and the
annotation rectangle's dimensions, and the positions are "values it determines to be appropriate,
based on the field value, the quadding ( Q ) attribute, and any layout rules it employs". This
tree's rule is §12.5.4's border width; `pdftotext`'s is the `/BS` width plus two points. The delta
is exactly their difference, on every word.

**The diagnosis was already in the tree**, written into the list's own note by the
seven-hundred-and-ninety-first session under the heading *§12.7.4.3's layout hand-off* (ADR 0726),
and every sentence of it is right. What no round had done is tell the instrument, so six documents
sat pinned in an equality-checked ratchet, the tail's printed summary counted them as positioning
errors, and the verdict's denominator carried words no bound in the file is about.

ADR 0756 is the argument and the refusal that goes with it: moving this tree's inset to two points
would take all six inside the bound and is curve-fitting a quantity the standard delegates.

## What changed

`crates/pdf-model/tests/text_extraction.rs` only. A matched word whose **centre** lies inside the
`/Rect` of an annotation whose appearance the file does not fix — Table 224's `/NeedAppearances`,
or no `/AP` at all — is set aside rather than judged, counted, and printed. Five documents whose
every unique match was a field value become a named refusal; `JUDGED_FLOOR` falls with the reason
beside it and `SELECTION_BELOW_FLOOR` loses the seven names that left. The `/Rect` is carried into
the reference frame through `pdf_model::content::page_transform`, which is the transform the
interpreter itself applied, rather than a rotation rebuilt here.

The centre rather than the whole box was measured both ways: whole-box containment left
`issue12750.pdf` and `issue19389.pdf` in the list, because §12.7.4.3 clips a value rather than
shrinking it and their glyph quads clear their own rectangles by 0.53 pt and 3.4 pt.

**One finding the note could not have made from the measure**: `issue16021.pdf` was classified under
a font-metric convention and is a `/FreeText` with no `/AP` on a page whose `/Resources` are empty,
so §12.7.4.3 places the whole of its text. A hand-off is a property of *who placed the word*.

`doc/conformance/ledger.toml`'s §12.7.4.3 row carries the reading; the row stays `partial` and the
ledger binary reformatted it.

## Calibration

Trap 13, above the commit, both ways, both reverted: with the set-aside disabled the gate fails
naming seven documents newly out of bounds; with its condition removed so every widget rectangle
sets its words aside, the judged set falls to 480 and the gate fails on `JUDGED_FLOOR`.

## Gates and sweeps

Run on this tree; the figures are the runs' own and are not repeated in any instruction file.
`tools/state.sh` prints them.

## What this round did not take

`doc/todo/22`'s Arabic free text (ADR 0348's list, unchanged), the six sweeps' reading lists, and
the vertical-centre bound's own defect — which was found while diagnosing this one, is written down
in ADR 0756's closing section with its two witnesses, and was left because every obvious repair
lands them within a thousandth of the bound.
