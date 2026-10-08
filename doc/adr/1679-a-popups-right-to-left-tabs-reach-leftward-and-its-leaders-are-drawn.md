# 1679 — A popup's right-to-left tabs reach leftward and its leaders are drawn, in `quorra`

Session 1421. Status: **accepted** and **built** in `quorra`; **said** in `quorra-gtk` and
`quorra-qt`. Builds on ADRs 1666 (a popup's tab stops), 1660 (a field's leaders) and 1634 (a field's
tab stops). Context: ISO 32000-2 §12.5.6.2 (Table 172's `/RC`), §12.7.4.3; XFA 3.3 chapter 27's
*Tab Stops* (pages 1205 to 1207) and chapter 2's *Tab Stops* and *Tab Leader Pattern* (pages 61 to
65), held, cited and never quoted.
Code: `crates/pdf-model/src/popup/rich.rs` (`RichTabStop::leader`, `RichLeader`,
`RichLeaderPattern`, `RichRuleStyle`; the leader sentence removed); `crates/pdf-model/src/rich_text.rs`
(`parts` names the leader types); `crates/viewer-host/src/popup.rs` (`TabStop::leader`, `Leader`,
`LeaderPattern`, the leftward default stops, `toolkit_unapplied`); `crates/viewer-ui/src/chrome/rich.rs`
(`tab_advance`, `lines`, `draw_leader`); `crates/viewer-confined/src/protocol/panels.rs`
(`encode_leader`, `decode_leader`); `crates/viewer-ffi/src/answers.rs` (`abi_note`). Tests: `viewer-ui`'s `panel.rs`
(`a_right_to_left_tab_reaches_leftward_to_its_stop`, `a_tab_leader_fills_the_room_before_its_stop`),
`viewer-host`'s `a_right_to_left_paragraphs_default_stops_lie_left_of_its_stated_ones`,
`viewer-ffi`'s `a_leader_the_abi_does_not_hand_over_is_said`.

## 1. A tab in a right-to-left paragraph

Chapter 2's *Tab Stops* has the next stop be the one on the left where the text flows that way
(page 61), and chapter 27 puts the default stops beyond the stated ones (page 1205) — beyond in the
direction the text flows. `viewer_host::popup::tab_stops` now places a right-to-left paragraph's
default stops at every `tab-interval` left of its leftmost stated stop (never at the margin, which
no tab reaches leftward), as the field's `reached_stop_leftward` does. `quorra` starts a
right-to-left line's cursor at its right edge, which is the line's width from the left margin the
stops are measured from, and each tab moves it to the nearest stop on its left, the text after it
standing there by the stop's side; the line is then ordered by UAX #9 as before, which keeps each
group's place because a tab is a segment separator and returns to the paragraph's level.

## 2. A leader

A stop's leader now crosses with it (it was read in `pdf-model` and turned into a sentence there).
`quorra` draws it across the tab's advance as a field's appearance draws it (ADR 1660): cycles on a
grid from the paragraph's left margin, a partial cycle left blank, a cycle the larger of
`leaderPatternWidth` and the pattern's own width; dots and content in the tab's run's face and
colour; a rule centred on the baseline, the underline's thickness where none is stated, dashed and
dotted as ADR 1660's pieces.

## 3. What the toolkits cannot do, and what was not measured

Neither Pango's `TabArray` nor Qt's `QTextOption::Tab` has a fill: a leader is said in both
("a tab leader in a popup window"). The C ABI hands a stop's position and alignment and not its fill, so a note it hands over carries
the same sentence among its unapplied phrases (`abi_note`), which a C caller read from `pdf-model`
before the leader crossed. A right-to-left tab stays said in both ("a tab in a
right-to-left paragraph"): Pango was measured to place a stop from a right-to-left line's start
edge — a stop at 100 in a layout 400 wide put the text after the tab at 300 — so the chapter's stop
from the left margin would be Pango's from a right edge the label does not know until GTK allocates
it, and handing tabs per allocation is a change to how the note is a label. How Qt places a stop in
a right-to-left block was not measured, and the case is said there until it is.
