# 950 — The edition a profile names decides which text judges it

Date: 2026-09-10. ADR: 0950.
Files: `crates/pdf-model/src/icc.rs`, `crates/pdf-archive/src/table/graphics.rs`,
`doc/third-party-data.md`, `data/icc/PROVENANCE.md`.

The owner supplied ICC.1:2022, ICC.2:2023 and then ICC.1:1998-09 and ICC.1:2001-12 — **two of the
four texts ISO 19005-2 section 6.2.4.2 actually names**. PDF/A-4 reached zero corpus misses and
`over` stayed zero on all six targets.

**The correction the two named editions forced is the finding of the round.** The `Profile ID`
calculation is *not* edition-invariant: ICC.1:2001-12 zeroes the rendering intent, the **device
attributes** and the field, where ICC.1:2022 and ICC.2:2023 zero the profile **flags** in the
attributes' place — and 2001-12 does not state the method at all, pointing at a web technical
note. So no 4.0.0 profile is judged by a calculation its own edition does not describe. **Had the
texts arrived in the other order, this round would have shipped that mistake with every gate
green.**

`6-2-3-t01-fail-d` closed against the file: it states version 5.0.0.0, ICC.2 makes that iccMAX's
own, §8.6.5.5 makes the header's version name the text that judges it, and it carries none of the
sixteen A-to-B and B-to-A tags iccMAX requires of a display profile — only v2 matrix/TRC tags
iccMAX does not define.

**The shipped profile is admissible, established from the text rather than asserted.**
`data/icc/sRGB2014.icc` states 2.0.0.0 where ICC.1:1998-09 calls itself 2.2.0, which looked like a
problem. Five things in the held text say otherwise, the load-bearing one being that neither part
2 nor that edition requires a profile to *state* the number of the edition it conforms to.
`PROVENANCE.md` carries the argument, because the sentence it already had was unverifiable when
written.

**One row deliberately reports less than it could.** A 4.4.0.0 profile is not reported as
inadmissible, because part 2's ISO 15076-1 reference is *undated* — whether a 4.4-based edition of
it exists is a fact about ISO's catalogue rather than about any document held here. Concluding
from an absence is the thing this project keeps catching itself doing.
