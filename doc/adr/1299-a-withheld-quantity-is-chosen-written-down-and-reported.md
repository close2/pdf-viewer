# 1299 — A withheld quantity is chosen, written down and reported

Status: accepted and **built**.
Context: `crates/viewer-core/src/transition.rs` (`BLINDS`, `DISSOLVE_CELLS`, `GLITTER_BAND`,
`Faces`, `fly`, `note`), `crates/viewer-host/src/clock.rs` (`Playing` holds `Faces`),
`crates/pdf-model/src/appearance.rs` (`CAPTION_SHARE`, `CaptionPosition::split`,
`Refusal::CaptionShareChosen`), `crates/pdf-model/src/requirements.rs` (`Transitions` met).
Answers: `doc/questions/A72` — "choose, and bound it".
Supersedes: ADR 0230's refusal of `Blinds`, `Glitter`, `Dissolve` and `Fly` (the rest of 0230, a
frame as two pages and a fraction, stands); ADR 0239's refusal of Table 192's `/TP` codes 2 to 5.

## The ruling, and its bound

Where a clause names the **kind** of mark and withholds only a **quantity**, this program chooses the
quantity, writes it in the code as a named constant whose comment says it is a choice, and says in
the report that the quantity is its own. Where a clause names no mark at all, nothing is drawn —
which keeps §12.5.6.11's caret, §12.5.6.12's stamp legends, §12.5.6.23's overlay and the watermark
where A72, A64 and A65 put them.

## The choices

| where | the clause's words | the quantity withheld | chosen |
|---|---|---|---|
| Table 164 `Blinds` | "Multiple lines, evenly spaced across the screen, synchronously sweep in the same direction" | how many | 8 |
| Table 164 `Dissolve` | "The old page dissolves gradually to reveal the new one." | grain and order | square cells, 40 along the longer side; one fixed SplitMix64 order |
| Table 164 `Glitter` | "Similar to Dissolve , except that the effect sweeps across the page in a wide band" | band width | 0.25 of the distance swept, over Dissolve's cells |
| Table 164 `Fly` | "Changes are flown out or in (as specified by M )" | what the changes are | the pixels in which the two pages differ |
| Table 192 `/TP` 2–5 | "2 Caption below the icon 3 Caption above the icon 4 Caption to the right of the icon 5 Caption to the left of the icon" | the caption's share | a third of the rectangle on its side |

**Table 192 is the right table.** A72's "Table 192's `/TP` codes 2 to 5" names the appearance
characteristics dictionary under §12.5.6.19, which is where `/TP` is; Table 191 is the widget's own
entries. No slip.

**The dissolve order is fixed, not random**, because a frame must be a pure function of transition,
view and fraction — so a test with no display sees the window's frame and the two backends can be
compared — and because a cell once replaced must stay replaced for the page to dissolve gradually,
which every frame agreeing on one order gives and a clock-seeded generator would not.

**Table 164 decides which entries apply to which style**, and each is obeyed there: `/Dm` is
"(Optional; Split and Blinds transition styles only)"; `/M` "(Optional; Split, Box and Fly
transition styles only)"; `/Di` covers `Glitter` and `Fly` among others; `/SS` and `/B` are "Fly
transition style only". `/B`'s "the area that shall be flown in is rectangular and opaque" makes the
flown picture the changes' bounding rectangle, opaque. `Fly`'s offscreen end is where a picture at
that end's scale is wholly off the view, which follows from "to or from a location that is
offscreen"; the scale's centre, the view's, is a second choice and is written down beside `scale`.
`/Di /None` with `/SS` 1 moves nothing, and the table makes `None` "relevant only for the Fly
transition when the value of SS is not 1.0", so that one is reported rather than drawn.

## What moved

§12.4.4, §12.4.4.1, §12.5.6.19 and §12.6.4.15 (which rested on the same four styles) leave
`departed` for `implemented`; §12.4's and §12.11.2's notes, and `requirements::Kind::Transitions`,
now say the styles are drawn. The report sentence is `transition::note`'s for the styles and
`Refusal::CaptionShareChosen`'s for the widget.
