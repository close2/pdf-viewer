# Icons for `Stamp`

Status: **`FileAttachment` and `Sound` are drawn (two-hundred-and-sixty-sixth session); `Stamp` is
not, and the reason is what the names *are*.**
Priority: 26
Corpus: 1 document
Clauses: §12.5.6.12, with §12.5.6.15 and §12.5.6.16 closed
Code: `crates/pdf-model/src/icon.rs`, `crates/pdf-model/src/appearance.rs`

§12.5.6.4 says a processor **shall** provide predefined icon appearances for the `Text`
annotation's standard names, and draws none of them — so `icon.rs` is this processor's own artwork,
and it says so: it is the one module in the tree that is pure invention (ADR 0109).

The three tables covering `Stamp`, `FileAttachment` and `Sound` say **should**. This file set its
own condition for taking them: *worth doing only if the artwork can be argued from the clause's own
descriptions.* **Two of the three met it and one does not**, and the line between them is the
finding rather than the drawing:

- §12.5.6.15's `Graph`, `PushPin`, `Paperclip`, `Tag` and §12.5.6.16's `Speaker`, `Mic` name
  **objects**. A clause that says "Paperclip" has described the artwork more completely than
  §12.5.6.4 describes `NewParagraph`, whose seven mandatory shapes had to be invented out of a
  typographer's convention. Drawn, in `icon.rs`'s unit square, and recorded as a choice under a
  recommendation.
- §12.5.6.12's Table 186 names `Approved`, `Experimental`, `NotApproved`, `AsIs`, `Expired`,
  `NotForPublicRelease`, `Confidential`, `Final`, `Sold`, `Departmental`, `ForComment`,
  `TopSecret`, `Draft`, `ForPublicRelease`. **Legends rather than symbols.** Drawing one means
  choosing a typeface, a size, a border and a rotation, and what a reader would see is a *word this
  program picked* — an invention of a different kind from the one the name names, on the strength
  of a recommendation. That is not restraint about difficulty; a stamp is the easiest of the four
  to draw badly.

What would change it: a clause, an erratum, or an argument that the legend's typography is
derivable. Table 166's `/C` is not one — it is the background of a *text* annotation's icon, and
§12.5.6.12 says nothing of the kind.
