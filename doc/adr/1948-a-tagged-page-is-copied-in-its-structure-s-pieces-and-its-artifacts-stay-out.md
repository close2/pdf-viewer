# 1948 — A tagged page is copied in its structure's pieces, its artifacts stay out, and a replacement keeps the break before it

Session 1556, slot 5 of batch eighty-eight. Status: **accepted, built**. Context: the owner's question
in the batch's brief ("do we use the information from tagged pdfs to improve for instance text
selection? … If not, use a round, to improve usage of the a11y layers"); ISO 32000-2 §14.8.2.2.1,
§14.8.2.2.2, §14.8.2.5.1, §14.8.3.3, §14.8.4.1, §14.8.4.5, §14.8.4.6, §14.8.4.7, §14.8.4.8.3, §14.9.4;
ADRs 0425, 0444, 1100, 1502, 1515.

## 1. What the tree already did, and where it stopped

The brief's premise was that the copy followed page content order. **It did not, and that part of the
premise did not hold.** `Command::Copy` has carried §14.8.2.5's logical order beside the page order
since ADR 0444: `Tree::logical_range` put a selection through the structure's depth-first traversal,
and `viewer_host::copying` takes it where it exists. Three things kept that order from being worth
much:

- **One artifact sent the whole copy back to page order.** `logical_range` answered only where the
  tree reached every byte. A running head or folio tagged `/Artifact` is reached by nothing, so a
  select-all on any such page was copied in page order. 21 of the corpus's 78 tagged first pages
  declare one.
- **The pieces were joined with nothing between them.** Two paragraphs the stream drew out of order
  came back as `First paragraph.Second paragraph.`, and a table's cells ran together.
- **An `/ActualText` cut away the break before it.** `separate_text` writes the separator it infers
  from a gap when the next glyph is shown. That glyph is inside the section, so the separator was
  cut with what the section read back, and `the` ran into a replaced `first`. §14.9.4's own EXAMPLE
  read back as `Druc\nker` where the clause says the word is "Drucker".

## 2. The structure states pieces and boundaries, and the viewer chooses the characters

`Tree::logical_range` now answers `Option<Vec<LogicalPiece>>`. Each piece is one sequence's share of
the range, in traversal order, and carries the `Boundary` between it and the piece before.
`Flow < Cell < Block`, and where several elements end between two pieces the strongest one stands.
The boundary follows §14.8.4.1's categories through `Category::of`. A block, grouping or internal
element ends with `Block`, and §14.8.3.3 says why: "BLSEs shall be stacked within a reference area in
block-progression order". `TH` and `TD` end with `Cell`, and an inline element ends with `Flow`.
**Three exceptions, each a choice.** `Link`, `Annot` and `Form` end with nothing, because Table 368
makes each one about an annotation and a link in a list item's body would otherwise break its
sentence. `Sub` ends a line, because every example Table 367 gives of it is a line. A type that is
neither standard nor mapped ends with nothing and takes its parent's category, because a boundary on
a name this reader cannot read would be a guess.

**Real content only.** Sequences under an `Artifact` element (§14.8.2.2.1: such artifacts "shall not
be considered real content") and ranges a producer declared with the marked-content method are
*accounted for*: they count toward the coverage, and they give no piece. **Artifacts by absence are
not.** §14.8.2.2.2's "[a]ny content that is not included in the structure tree is an artifact" is this
reader's inference. The corpus shows what that inference holds: untagged widget captions, signature
text, and `issue14627.pdf`'s "Please refresh your browser…", which is real content. So a byte nothing
claims still keeps the copy in page order. A range that is artifacts and nothing else answers `None`
too, because a person who dragged across a folio asked for the folio.

`viewer_core::select::logical_copy` joins the pieces. A block's end is one `\n`, a cell's end is a
tab, and inside a block the readback's own separators stand. Whitespace on either side of a boundary
gives way to the boundary's own character, because it is a separator inferred for a neighbour the
logical order no longer puts there. The copy neither starts nor ends on whitespace. No clause states
what a clipboard holds, so these characters are written down here as choices.

## 3. A replacement replaces the content and not the gap

`Interpreter::cut_replaced` replaces the bare cut. **The separator inferred at the section's first
byte stays.** `WordGaps::last_separator` is what tells it from a space the content showed. The
consecutive-replacement rule ("treated as if no word break is present between them") still removes
it, because it runs on the readback as the cut leaves it. **A replacement that removes the hyphen its
content ended on withholds the next inferred line break.** That is the EXAMPLE's case: the producer
replaced the line-end hyphen, which states that the word goes on. A replacement that keeps its hyphen
keeps the break.

## 4. What it measured, and what is left

`tests/logical_order.rs` counts it over the corpus. 49 of the 78 tagged first pages copied whole in
logical order before, and 58 do now. The nine between are pages whose declared artifacts used to
refuse the order. Every other page in the corpus keeps its readback apart from whitespace, so
`text_extraction` and `selection_census`, which compare with the whitespace removed, cannot move.
`viewer-core/tests/tagged_copy.rs` holds the four rules on one built page.

**Left, in order of worth:**
- **An annotation the tree claims through §14.7.5.3's object reference** reaches the readback without
  its object. `160F-2019.pdf` and `bug1868759.pdf` copy in page order for that reason alone. Placing
  it is §14.8.2.5.2's rule, and it needs `Interpretation` to carry which annotation each structural
  range belongs to.
- **Line breaks inside a block are the readback's**, so a pasted paragraph still breaks where the page
  did. Joining them is a reflow decision with `Sub`, verse and code against it, and it is not taken
  here.
- **A soft hyphen at a line's end** is folded by search (ADR 1100) and kept by the copy.
