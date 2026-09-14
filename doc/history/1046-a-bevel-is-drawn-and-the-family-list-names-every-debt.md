# 1046 — A bevel is drawn, and the family's list names every debt it has

Ledger slot of batch 1044–1049. Contract: §12.5's eleven `partial` rows and the decision 1040 named
and left. **One row closed, the parent's list corrected, one ADR, three calibrated tests, no corpus
pixel moves.** A dead agent's uncommitted work on this contract was inherited; its reading was
re-derived from the clause before anything was built on it.

| row | was | now | what moved it |
|---|---|---|---|
| §12.5.4 Border styles | `partial` | **`implemented`** | Table 168's `B` and `I` drawn in relief, `Border::relief` |
| §12.5 Annotations | `partial` | `partial`, seven debts | its list named five children and omitted four rows' own |

**1. The bevel is the cloud's construction one table row up.** §12.5.4 states the obligation twice
— "[i]f present, the border shall be drawn completely inside the annotation rectangle" and "[i]f
neither the Border nor the BS entry is present, the border shall be drawn as a solid line with a
width of 1 point" — and Table 168's `/S` then says which border, in one grammar for all five
values. Three were drawn; two were refused because the table names no colour for the relief — and
that is ADR 0192's and ADR 1057's shape, a `shall` behind a silence about artwork. ADR 1061 chooses
once: two bands inside the line, each as wide as it, light from the upper left, `I` the same relief
exchanged, greys rather than tints of the line's colour, `Border::thickness` twice the width.

**2. Looked at rather than counted** (trap 1). Ten links, both styles at five widths, at scale 3:
at one and two points the shadow alone carries the illusion, at four and eight the frame reads as
raised or recessed at a glance, and at twenty a cloud's own clamp is what is on the page.

**3. The corpus cannot rank it, counted rather than assumed** (trap 8). `border_precedence_census`:
**0 of 1452** curated documents and **0 of pdf.js's 963** state a `B` or an `I` among 45 909 and
33 781 constructed borders; over the crawl, the 84 `B` and 86 `I` this row already carried sit in
**six documents of 65 720**, which the census now names.

**4. The other ten rows were read against the clause and nine hold for a sentence that is exact.**
§12.5 and §12.5.6 are aggregates; §12.5.1 owes handlers for the subtypes `CLAUDE.md` excludes;
§12.5.2 owes `/AF` and `/Lang`; §12.5.3 owes `Print` and `Locked`, which needs a reader that moves
or deletes an annotation; §12.5.5 owes two thirds of one sentence about `/ca` and `/CA` that Table
166 and §12.5.2's closing rule contradict. The parent's list named none of those four.

**Calibration (trap 13) and gates.** `relief` made to return early: both relief tests fail, the
layout one passes; `thickness` made to answer `self.width`: the layout test fails at 25 for 28.
Tier 1 and `raster_golden` — held 974, moved 0. Three lines carry a sibling's failures, in
`pdf-model/src/submission.rs`, `pdf-signature` and `pdf-archive/src/editions.rs`.
