# 1784 — The annotation rows' claims, read against their lines, and an ending no longer takes its line

Session 1474. Status: **accepted** — an audit, with two defects fixed and thirteen stale sentences
corrected. Context: ADRs 1754, 1765 and 1772, which did the same for clauses 8 and 9. Rows: the 24
`implemented` rows under `12.5`, and the `partial` aggregates §12.5 and §12.5.6. Code:
`crates/pdf-model/src/appearance.rs`, `crates/pdf-model/tests/annotations.rs`.

## 1. Why this is coverage work on `implemented` rows

Every `partial` leaf waits on an owner's answer (Q308, Q271, Q348, A66's trigger, a policy syntax no
signature names, §7.4.7's patch on the owner's fork); §12.5's own, §12.5.6.6, waits on Q308. A false
claim in an `implemented` row is a false `implemented` (ADR 1754 §1).

## 2. The method

ADR 1772's: the notes split on `[[clause]]`, `\b(is|are) (read|applied|executed|drawn|honoured|obeyed)\b`
counted over the `implemented` rows whose clause starts `12.5` — 71 claims, in 21 of the 24 rows
(`scratchpad/r1474/claims.py`, `ctx.py`) — each matched to the function that makes it true, each
cited reader asked for a caller outside the tests, and every backticked name checked to exist
(`names.py`). Line numbers are this session's tree.

## 3. The claims

| row | claim | where it holds | verdict |
|---|---|---|---|
| 12.5.3 | Hidden, NoView, Invisible; Print; NoZoom, NoRotate; ReadOnly, ToggleNoView; bits 8 and 10 (nine) | `annotation::decided` 1019, `displayed` 631, `view_flags` 468, `interacts` 868; `restriction::asserted` 497 | holds |
| 12.5.4 | B and I in relief, only where `/BS` is a border; Table 166's precedence (four) | `Border::draw` from `link`, `widget`, `free_text_border` alone; `Border::geometry` | holds |
| 12.5.6.2 | `/RC` drawn in the window; `/CA`; the paragraph rule; `/Subj`, `/CreationDate`; `/DS` by the group (eight) | `popup/rich.rs` `note`; `annotation.rs` 1338; `chrome.rs` 3076; `popup.rs` 590–610 | holds; **two sentences still called the formatting owed**, and a free text border reads no `/C` |
| 12.5.6.3 | the walk executed; states "handed over" for "a host" to show | `annotation_state::states` 197 | the walk holds; **no host calls it** |
| 12.5.6.4 | the subtype's flags; the icon's square; seven names; the window; `/Open` by the group (six) | `view_flags`; `text_icon`; `opens_with_the_page` | holds |
| 12.5.6.5 | `/QuadPoints`; `/H`; the border; `/BS` (four) | `link.rs` 159; `annotation::highlight` | holds; **the launch and URI rows' statuses were stale** |
| 12.5.6.7 | `/LL 0`; endings; `/Cap`, `/CP`, `/CO`; `/IT`, `/Measure` "read whatever is drawn" (nine) | `line`; `caption`; `appearance::intent` 339 | holds but **`intent` and `annotation_measurement` have no caller outside the tests**, and an unknown ending took the line (§4) |
| 12.5.6.8, 12.5.6.13, 12.5.6.16, 12.5.6.18 | the mark; the ink; both icons; `/AP` drawn | `square_or_circle`, `differences`; `ink`; `symbol_icon`; `annotation::construct` 1321 | holds |
| 12.5.6.9 | `/IC`; a cloudy `/BE`; `/IT` and `/Measure` (five) | `polygon` | holds; **a polygon's `/LE` was reported** (§5) |
| 12.5.6.10 | drawn and added; `text_under` with two callers | `ViewState::add_markup`; `tools/spec-errata`, `tools/pdf-retrieve` | holds |
| 12.5.6.11 | `/RD` read by `appearance::insets` | no such function; the caret arm refuses before reading an entry | **false** |
| 12.5.6.14 | the window drawn; `/F` from the popup; a window with no area left out | `popup::read` 573; `viewer_host::popup` 112 | holds; **"none of the formatting" predates ADR 1642** |
| 12.5.6.15 | four icons; `/FS` read | `symbol_icon`; `attachment::of_annotation` 582; `viewer_core` `exhibit` | holds |
| 12.5.6.19 | the value; `/R`; `/TP`; `/RI` to `/AC`; `/H`'s five modes and default (eight) | `field_text` 3939, `CAPTION_SHARE` 3494, `icon_entries`, `down_overridden` | holds; **`Owed` has fourteen variants, not eight** |
| 12.5.6.20, .21, .22, .24 | drawn like any other; `/FixedPrint` | `decided`; `fixed_print` 1765, `target_media` 1006 | holds; **§14.11.3 is `implemented`, not `partial`** |
| 12.5, 12.5.6 | the aggregates | — | **named §12.5.6.2 as owing** |

The brief's hypothesis held in kind: what was false were reach claims, a reader that does not exist
and three that exist with no caller outside the tests. The defects were not behind a ledger claim.
They were behind a doc comment and a table cell nobody had read whole.

## 4. Defect one: an unknown line ending erased the line

`UNKNOWN_LINE_ENDING`'s doc says the name is reported "beside a drawn line rather than instead of
one" (ADR 0106), and the callout of §12.5.6.6 does that. `line` and `polygon` read `/LE` through
`line_endings(…)?`, so a line or polyline naming a style outside Table 179 returned a refusal and
drew nothing, although its `/L` or `/Vertices` is required and the ending optional. `line_endings`
now leaves that slot without an ending and returns the refusal beside the endings. A line whose
ending and caption both owe states both (`Refusal::Both`, so neither report hides the other).
`a_line_ending_style_the_table_does_not_have_is_reported_beside_the_line` holds both subtypes. With
the old `?` planted back, it fails on the line half.

## 5. Defect two: a polygon's `/LE` was reported

Table 181: `/LE` is "(Optional; meaningful only for polyline annotations)". The same table's `/BE`,
"meaningful only for polygon annotations", is unread on a polyline and reports nothing (trap 11), and
ADR 0192 reported a polygon's `/LE` on the right inference without reading that cell. A polygon's
`/LE` is now neither read nor reported, so a malformed one no longer refuses the polygon either. A
`/Path` polyline's `/LE` is still named: the cell places the endings on "the first and last pairs of
coordinates in the Vertices array", which a `/Path` replaces, so the table states no endpoint. The
refusal's sentence now says that rather than "gives no two points". The test is
`a_polygons_line_endings_are_not_meaningful_and_a_path_polylines_are_named`, which fails with the
polygon's reading planted back.

## 6. What no host reads

`annotation_state::states`, `appearance::intent` and `measurement::annotation_measurement` are
public and called only by tests and a census. No clause they serve asks a processor to display what
they return, so the rows stay `implemented` and the notes now say that no program path calls them.
A later round that gives a host a comments pane or a measuring tool has its readers ready.
