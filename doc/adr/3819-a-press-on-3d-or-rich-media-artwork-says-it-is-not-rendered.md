# 3819 — A press on a 3D or rich media annotation says its artwork is not rendered

Session 2279. Status: **accepted and built** in `viewer-core` (`interact::deprecated_media::exhibited`,
`artwork_not_rendered`). Clauses: ISO 32000-2 §13.6.2, Table 310, §13.7.2.1, Table 335. Finding:
2278-B04.

## Context

§13.6.2 gives a 3D annotation two states, "Inactive (the default initial state): the annotation
displays the annotation's normal appearance" and active, which shows the artwork; Table 310's `/A`
defaults to `XA`, activation by the user, and §13.7.2.1 gives a rich media annotation the same. This
program renders no U3D or PRC artwork (the clause 13 exclusion), so the page shows the normal
appearance — which the clause permits — and a press said nothing (trap 5).

## Decision

A press and release on a 3D annotation reports that its artwork is not shown, that rendering U3D and
PRC artwork is excluded, and that the annotation keeps its normal appearance, §13.6.2's inactive
state. On a rich media annotation it reports that the 3D artwork is excluded and the `Sound` and
`Video` configurations are not built yet (ADR 3820), and that the press hands the assets over as
files to save, which it already did (ADR 2399). It is a note (`Event::Reported`), not a refusal: no
restriction stands in front of it and no level can turn it into a play. The sentence is said on
every press, whatever Table 310's `/A` or Table 335's `/Condition` states, because each press is a
person asking.
