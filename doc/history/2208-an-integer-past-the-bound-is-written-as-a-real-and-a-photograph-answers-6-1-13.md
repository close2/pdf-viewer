# 2208 — An integer past the bound is written as a real, and a photograph answers 6.1.13

Session 2208, an archive round in the `lane3` worktree: ADR 3519's readings of the thirteen 6.1.13
refusals at 2b, built. ADRs 3533, 3534, 3535. Ledger: 7.3.3 stays `implemented`, its code and test
lists gaining `integer_reals`.

**The period (ADR 3533).** `integer_reals` walks every content stream for the values section 6.1.13
bounds; an integer past ±2^31, within the real bound, as a direct operand of an operator whose
operands are numbers, is written with a period after its last digit and no other byte moves
(`Mechanical(IntegerWrittenAsReal)`, §7.3.3). The objects' row `integer-values`, which the reader's
log fails at the same page, is answered by the same rewrite where the page's dictionary states no
such value. Anything else keeps the target's sentence. `475121` converts at 2b under `as-shown` and
`as-if-printed`, 40 integers restated; byte proof: its decoded stream is the source's 311254 bytes
plus 40 periods, each after a 34-digit run; pixel proof: this tree's and mupdf's renderings are
byte-identical, poppler differs by 805 pixels between the two spellings of the same number
(evidence about poppler). Section 5.1 (ADR 3521) counts its `integer-past-i64` repairs answered where this rewrite writes the
page's content.

**040669 and 427669 (ADR 3534).** Not misreadings: `040669` states `62e545` and `238.e285`, the
exponential form the lexer reads (ADR 0733) and the scan counted as no number; `427669`'s object 209
ends inside a string that §7.7.3.3's concatenation runs on for 62170 bytes. `note_real`'s comment
corrected; no survey change.

**The ten (ADR 3535).** `content-as-drawn` answers none: each value past a bound is a token whose
drawn value is itself. Where every stream stating one is taken off its page by a photograph, the
photograph answers both rows. Under `as-if-printed` at 2b `020747`, `020806`, `033689`, `268346`,
`477139`, `492484`, `507676` convert, with `040669` and `427669`; `150277` stays refused by name
(object 629, page 25, not photographed); `027613` and `219789` pass 6.1.13 and fail the output's
section 5.1, refused by name for a content repair no rewrite writes out (ADR 3521). `as-shown` keeps all ten refused.

**Census.** With and without the pack, on main after round 2205: under `as-if-printed` at 2b ten
of the thirteen are written (pdf.js 940, corpora 451 converted with the pack); `D-defect` is 4
documents. `rank.py` regenerated sections 2 and 9. veraPDF: 5515 written as conforming, 46 failing
outputs of 14 documents; none of the eleven new 2b outputs among them.

**Gates.** Over `-p pdf-archive -p pdf-transform -p viewer-qt`: fmt 0; clippy 0 (after two
findings fixed); rustdoc 0; `fuzz` 0; nextest 1772 passed, 9 failed, the manifest's 9 (after rebuilding the sandbox and media
workers, whose stale builds failed eight more); conformance
478 passed, 0 failed.
