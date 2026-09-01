# A wheel tick that interprets: §12.5.3's residue after the seam

Status: **open — the seam has landed (ADR 0777) and what is left is the clause's own pass.**
Priority: 46 — performance a person feels directly, on a bounded population of documents.
Corpus: `doc/ISO_32000-2_sponsored_EC3.pdf`. `examples/spec_annotation_census` is where the share
across corpora goes; §12.5.3's ledger row carries it.
Code: `crates/pdf-model/src/content.rs` (`interpret_replaceable`, `replace`, `Checkpoint`),
`crates/viewer-core/src/open.rs` (`OnScreen::replaceable`, `reinterpret`, `stale`).
Instrument: **`cargo run --profile gates -p viewer-core --example zoom_cost -- <file.pdf> [steps]
[page|all]`**, which times exactly what `viewer-ui`'s `--trace` line times — `Viewer::handle`, with
no host and no window in the number. `pdf-model/examples/replace_cost` prices one page's
interpretation against the clone the seam makes of its display list.

**Three of `zoom_cost`'s settings are load-bearing and each was wrong on the first attempt** (ADR
0775 §1): the resize arm reaches this clause only under a *fit* mode, because the ISO specification
opens at §12.3.2.1's stated magnification where a window drag changes nothing; the arrangement has
to be scrolled *across a page boundary*, because the document's own catalog says `/PageLayout
/OneColumn` and one page on the screen understates a per-page cost by half; and the plain-page arm
is what attributes the cost, at 0.3 µs against milliseconds.

## What has landed

- **The re-interpretation is applied to the pages the clause is about** rather than to Table 29's
  whole arrangement (ADR 0775). Worst notch 13.99 ms → 4.98 ms.
- **The pages it is about are re-*placed* rather than re-interpreted** (ADR 0777): `draw_annotations`
  runs last, so everything §12.5.3's pass contributes is a tail on every one of the interpreter's
  accumulators, and `content::interpret_replaceable` keeps the state at that seam for
  `content::replace` to run the tail again. Worst notch 5.04 ms → **1.035 ms**, page 407
  4.99 ms → 402 µs, page 1001 3.81 ms → 529 µs, page 10 566 µs → 96 µs, all in one sitting.

  The construction was chosen by measurement: the `DisplayList` clone the seam makes is 7–39 µs
  against interpretations of 0.73–12.7 ms, so the list is rebuilt by copying and the *transform
  node* alternative — which would have cost all three rasterisers — is not owed.
- **§11.4.7's subtractive pair is the documented exception.** A page drawn in its own
  four-component space is two interpretations merged by geometry digest, so it keeps no replacement
  and a zoom of it re-interprets whole, at the price it already pays twice over.
- **The seam's own condition found a second reader of the clause and it was wrong** — see
  §12.5.6.4's ledger row and ADR 0777 §3. `annotation::view_flags` is the one reading now.

## What is left

**The residue is the clause's own pass, and page 962 is where to look.** It improved by 1.7× where
the rest improved by 6 to 13, so what a notch costs there is now almost entirely
`draw_annotations` — resolving each annotation, deciding its appearance and running the appearance
stream — rather than anything this seam can keep. Nothing has attributed *that* yet: the obvious
suspects are `crate::annotation::decide` re-reading each dictionary and `draw_appearance` re-running
each stream, and both are re-derivable per notch by construction because the magnification is what
changed.

Two things a round taking this owes before writing code:

- **Attribute the millisecond.** `replace_cost` prices the clone against the interpretation; it does
  not price `draw_annotations` against its own parts. A page whose notch is a millisecond of
  annotation pass is a different item from one whose notch is a millisecond of anything else.
- **Ask whether it is worth taking at all.** One millisecond per notch is not a gesture a person
  feels. The condition that would change that is `46-the-kernel-floor.md`'s: if a zoom step drops
  toward the 30 µs the compute lane could reach, this becomes the visible fraction of the gesture
  again. Until then this file is a priced remainder rather than a queue entry.

The two shapes ADR 0775 rejected stay rejected, and for the reasons it gave: interpretation off the
event thread relocates work the seam removed, and debouncing to gesture-settle would draw a real
frame at a magnification §12.5.3 says the annotation never had.
