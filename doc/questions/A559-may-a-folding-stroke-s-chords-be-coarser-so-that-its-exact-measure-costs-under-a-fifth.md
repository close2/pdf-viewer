Status: complete
Given: 2026-10-21, in conversation — transcribed by the round
Owes: the build — the exact measure routed with chords at a twentieth of a device pixel (`FOLD_FLATNESS` = 1/20 in `crates/render-cpu/src/stroke/folding.rs`), the correctness gate restated as the flattened set's (the pieces drawn sixteen times finer hold the measure to the flattening's own bound, √2/20, against the curve), and one boundary per mark shared between the strips that cross it so the twenty-four-thread figures are under the fifth too (acting round)

> I agree with 1/20 chords.  Please answer.

Reading: the recommendation adopted is the question's own, quoted and attributed: *"Take chords
at 1/20 of a pixel with the exact measure, and restate the gate as the flattened set's."* The
word came after the owner saw the impact rendered — the comparison sheets built from the patched
measure at both chord widths, the paperclip's witness pixel and the ink pages' fold rims — so the
adoption is with the pictures in view, both the page that visibly changes and the one that does
not.

What the answer settles:

- **Chords at 1/20, exact measure.** Every measured page lands under the fifth — +14.5 % on
  `inks.pdf`, +7.7 % on `rotated_ink.pdf`, +7.8 % on `issue14415.pdf`, by the same instrument
  ADR 2122 priced with — where the sixty-fourth is +145.3 % on the worst page.
- **The rim band is bounded for the first time.** A folding stroke's rim pixel is within the
  chords' own error, √2/20 of its area (18 levels), where the sixteenths' band reached 50 and
  stated no bound; the paperclip's witness pixel reads 245 against the curve's 242.4 where the
  sixteenths read 255, and at the fold edges of the ink pages the whole-pixel misses the
  sixteenths left (a rim pixel white that the union fills) come out.
- **The gate holds the flattened set, the way §10.7.4 measures it.** §10.7.2 leaves the flatness
  to the processor in so many words, and §10.7.4 measures the flattened set; the ladder holds
  the measure to the flattening's bound against the curve rather than to the curve itself, and
  the paperclip to the same bound.
- **The second lever is in the owed build**: at pool width each strip finds a mark's boundary
  again (+21.8 %, +63.6 %, +6.9 % at 1/20), and sharing one boundary per mark between the strips
  that cross it is what puts the twenty-four-thread figures under the fifth, so the answer ships
  as the A538 wait promised rather than as a single-thread number.