# 3679 — A colour operator is written as the device operator this reader drew; a section left open is not closed

Session 2244. Status: **accepted and built**; extends ADR 3558 and ADR 3631. Context: the census
refusals by `model/colour-operands-do-not-fit-the-space` (`issue18894`, the `MICOM` poster, three
govdocs files) and `model/marked-content-unbalanced` (`050734`, `504076`). Clauses: ISO 32000-2
§8.6.8, §8.6.5.6, §14.6.1 (Table 352); ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

ADR 3558 wrote a colour operator whose operands do not fit the current space out with its
operands, and the proof left it wherever the reader had drawn something for it: given one, three
or four numbers, `pdf_model::content::colour` draws them in the device space with that many
components, as the page defines it (ADR 3541). That is exactly what `G`/`g`, `RG`/`rg` and
`K`/`k` draw, `Default` spaces included (§8.6.5.6). ADR 3631 wrote a stray `EMC` out, and left a
section open at the content's end, which Table 352's `BMC` and `BDC` say is "terminated by a
balancing EMC operator".

## Decision

1. **A colour operator** (`SC`, `SCN`, `sc`, `scn`) whose only class is this one and whose operands
   number one, three or four keeps its operands and is written as the device operator of that
   count and side. Any other count, which the reader draws nothing for, is written out as before.
2. **A section left open is not closed.** Built and withdrawn in this round: an `EMC` at the
   last stream's end drew the same, but the write then counted the page as answered, so under
   `as-if-printed` `050734`'s page — whose other operators the write left — was no longer
   photographed and its output failed section 5.1 (a census regression of one document). Where a
   content ends is this reader's reading; the class stays refused by its sentence and photographed.
3. The colour write stands under ADR 3558's proof — both files drawn, the write standing only
   where every page draws the same — and the loss is `content-as-drawn`: the device space is this reader's reading
   of operands §8.6.8 says do not fit.

## Consequences

`issue18894` converts under `as-shown`. The `MICOM` poster does not: its colour operators stand in
a form that inherits the invoking page's `ICCBased` space (§8.10.1), which the walk, starting each
form at §8.4.1's initial state, does not follow, so the respelled form draws differently and the
proof leaves it. `050734` and `504076` stay refused by this class and others (ADR 3680).
