# 1660 — The list types count as their drafts, a leader fills its room, a tab reads the paragraph's way, a width takes the nearest face, a missing character takes a machine face, and a link is formatting

Session 1412. Status: **accepted**. Builds five of the properties ADR 1649 section 6 left named by
`Owed::RichTextUnapplied` and `Owed::RichTextOneStyle`, and decides two more as choices; amends ADR
1649 (sections 4, 5 and 6) and ADR 1634's reading of `font-stretch`; none is edited.
Context: `crates/pdf-model/src/rich_text/markup.rs` (`ListStyle`, `additive`, `longhand`,
`cjk_decimal`, the `a` and `xfa:embed` arms), `crates/pdf-model/src/rich_text/style.rs` (`TabStop`,
`TabAlign::After`, `Leader`, `read_leader`), `crates/pdf-model/src/rich_text/layout.rs`
(`reached_stop`, `reached_stop_leftward`, `tab_line`, `place_groups`, `write_leader`,
`encode_tag`, `nearest_width`, `Faces::ask_machine`), `crates/pdf-model/src/variable_text.rs`
(`machine_faces_for`, `Order::from_visual`); `crates/pdf-model/tests/rich_text.rs`. ISO 32000-2 §12.7.4.3, Table 228; XFA
3.3 chapter 27 (*Summary*, page 1187; *Hyperlink Support*, page 1189; *List Support*, pages 1211 to
1219; *Embedded Object Specifications*, page 1222), chapter 2 (*Tab Stops* and *Tab Leader
Pattern*, pages 61 to 65) and chapter 5 (*Rich Text That Contains External Objects* and
*Displaying and Printing Rich Text*, pages 221 and 222); W3C *CSS Lists and Counters Module Level
3* (Working Draft, 24 May 2011, sections 8.1.3, 8.1.6, 8.5, 9.3, 9.6 and 10.3), *CSS3 module:
Lists* (Working Draft, 7 November 2002, section 4.4), CSS2 (REC 1998, section 15.5). All cited and
paraphrased, none quoted.

## 1. What §12.7.4.3 brings in, and what that decides

The clause's fields with bit 26 set "specify formatting information as described in" XFA 3.3, and
Table 228's `/RV` is a rich text string as XFA describes it. **Formatting is what is delegated.**
Two things chapter 27 names are not formatting, and each becomes a choice under ADR 1622's rule —
the row is `implemented`, the choice named:

- **Following an `<a href>`.** The chapter has an interactive processor highlight the link and
  hand its destination to the operating system on a click. The highlight is formatting and is
  drawn (the recommended blue and underline, as before). The click is behaviour: what a click on a
  widget does is §12.7's (focus, edit) and §12.6's (`/A`, `/AA`), and a URI leaving this program
  goes through §12.6.4.8's action under the reader's levels, never from inside a field's value.
  The report is withdrawn, because the drawing is complete.
- **`xfa:embed`.** A SOM expression names a node of an XFA object model a field without XFA does
  not have, and a URI is data from outside the file, which the renderer — with no network, by
  principle 3 — does not fetch. The attribute is not in chapter 27's minimum set, and the chapter
  has a processor ignore markup it does not understand. So the span is drawn as what it holds,
  and **the report stays**: the text the reference would have inserted is missing from the page,
  and that is trap 5's case even when the omission is chosen.

## 2. The list types *List Support* requires

Every value of the chapter's table is generated, each as its [CSS3-Lists] definition states
(the chapter's bibliography names *CSS Lists and Counters Module Level 3*): the four Kana sets
from the 2011 draft's section 9.3, the two Hangul sets — which that draft no longer names — from
the 2002 draft's section 4.4, all by the alphabetic algorithm (section 8.1.3, bijective base *n*);
Hebrew and the two Japanese styles by the additive algorithm (section 8.1.6) over section 9.6's
weights, Hebrew's 15 and 16 as the draft's own pairs; and the four Chinese styles by section
10.3's longhand algorithm. Outside a style's range (section 8.5) the fallback is taken —
`cjk-decimal` for the CJK styles, as the draft states, `decimal` for the rest. **Two choices**:
an additive representation longer than twenty characters takes the fallback, which section 8.1.6
permits and which bounds Hebrew's unbounded range; and the suffix is the full stop, because
chapter 27's *List Layout* fixes it for every numeric type (page 1219) where the drafts give the
CJK styles another. The tables were checked against the drafts' text by script, and the unit test
runs a worked number through each algorithm. The drafts are W3C documents fetched for this round
and not held; their URLs are in the comments.

## 3. Tab leaders

Chapter 2's leader grammar is read whole: `dots()`, `rule(style [thickness])` with `solid`,
`dashed`, `dotted` (and `double`, `groove`, `ridge` as solid, which the chapter permits), `space()`
and `none` as no leader, and `use-content(…)`, then the optional `leaderAlignment` and
`leaderPatternWidth`. The leader fills the room a tab advances — the region before the text its
stop aligns — at the baseline, in the paragraph style's colour. **Choices, each the chapter's to
leave**: dots are the run's own full stop (the chapter allows text or graphics); a rule's default
thickness is the underline's, a dash is half of a four-thickness cycle and a dot one thickness in
two; cycles are laid on a grid from the paragraph's left margin, so leaders on different lines line
up, which is `none`'s alignment chosen; and a partial cycle is left blank, as the chapter says.
`page` alignment is laid on the same grid and **said**, because a field's appearance does not know
the page's edge.

## 4. A tab in a paragraph read right to left

Chapter 2's *Tab Stops* makes the next stop the one on the left where the text flows right to left
(page 61), and chapter 27 has a default stop right-align what follows it there (page 1205). So a
line holding a tab and text read right to left is placed **group by group**: the text between two
tabs is a group, shown in UAX #9's rule L2 order over its own glyphs, and the groups follow one
another from the edge the paragraph starts at — the right one where UAX #9's rules P2 and P3 give
the paragraph an odd level, each tab moving to the nearest stated stop on the left, then to a
default one. That is UAX #9's own answer as well: a tab is a segment separator, which rule L1
resets to the paragraph's level, so no reordering carries text across one. `after` and `before`
are now the edges the text starts and ends at in its paragraph's direction rather than fixed as
left and right. **Choices**: chapter 27 puts the default stops in effect beyond the stated ones,
read here in the direction the text flows, so left of the leftmost in such a paragraph; and the
paragraph's alignment moves a placed line as it moves one read left to right, mirrored. A line of
a left-to-right paragraph holding a right-to-left run and a tab is placed the same way from the
left, where it used to advance nothing.

A **list tag** is generated text and is now set in its own display order by the same rules, with
its paragraph's direction: a Hebrew numeral reads right to left with its full stop after it, and a
tag whose characters two faces draw is written a face at a time.

## 5. A width no face states

CSS2 section 15.5 gives no matching criterion for `font-stretch` and leaves the best match it can
find to the processor, and chapter 5 makes rendering heuristics application-dependent. So a width
the family's `/DR` faces do not state is set in the nearest one they do: narrower first for a
condensed or normal request and wider first for an expanded one, nearest of each first — CSS
Fonts Level 3's order, taken as this tree's choice — and where the family has none in `/DR`, in its
normal width as before. The width stays in the report, now saying where it was set.

## 6. A character no run's face draws

A path holding a face this program chose now ends in faces from this machine covering the
characters it lacked (ADR 1414's rule, asked per string rather than per value), and the string is
encoded again — so every run keeps its style and only the missing characters are set in the
machine's face. The question is asked once for all of them, and where no one face covers them, once
per set — the CJK ideographs, kana and full-width forms together, Hangul together, and otherwise
each 256-code-point block — since a string can mix scripts no face holds; at most eight sets,
because each first question of a set reads the font catalogue (the four-field fixture of this
round interprets in 379 ms against 130 ms with the catalogue read once). `Owed::RichTextOneStyle`
remains for a machine with no such face. A `/DR` face is the document's choice and is not searched
past, and its shortfall is now said as `CharactersNotInFont` beside the runs rather than sending
the whole string to the one-style layout, which only a face this program chose falling short does.

## 7. What stays

`kerning-mode:pair`: no kerning data is held — the fourteen's widths come from pdf.js and carry no
pairs, Adobe's AFM pair tables are not in the tree, and reading a program's `kern` or `GPOS` is
`pdf_font`'s build. `xfa:embed`'s text and a leader's page alignment are said.
