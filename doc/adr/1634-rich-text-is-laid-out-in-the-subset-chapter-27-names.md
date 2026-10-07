# 1634 — Rich text is laid out in the subset XFA 3.3's chapter 27 names, each run in its own face

Session 1399. Status: **accepted**. Builds what ADR 1623 found owed; amends ADRs 1122 and 1197 (the
formatting they reported rather than applied is applied) and ADR 1224 (Table 177's `/DS` is applied
rather than reported); none is edited.
Context: `crates/pdf-model/src/rich_text.rs` and `rich_text/` (`style`, `markup`, `layout`);
`crates/pdf-model/src/appearance.rs` (`field_text`, `free_text_layout`, `rich_laid_out`);
`crates/pdf-model/src/variable_text.rs` (`Face`, the `Owed` variants); `crates/pdf-model/tests/rich_text.rs`;
`doc/third-party-data.md`'s XFA 3.3 section. ISO 32000-2 §12.7.4.3 (Table 228), §12.7.5.3 (Table 231),
§12.5.6.6 (Table 177), §12.5.6.2 (Table 172), §9.6.2.2, §9.6.3; XFA 3.3 section 27, pages 1187 to 1223,
and chapter 2's *Measurements*, page 36. XFA 3.3 is cited by section and page and never quoted.

## 1. What is read, and what is built

ISO 32000-2 names one text for every rich text string: Table 228's `/RV` is "A rich text string, as
described in Adobe XML Architecture, XML Forms Architecture (XFA) Specification, version 3.3 ." and its
`/DS` "A default style string, as described in Adobe XML Architecture, XML Forms Architecture (XFA)
Specification, version 3.3 ."; Table 177 says the same of a free text annotation's pair. XFA 3.3's
chapter 27 lists the XHTML elements and the CSS2 and XFA properties a processor supports, sends a
reader to XHTML and CSS2 for their values, and says a processor ignores what it does not understand —
an unrecognised element with its whole content (page 1187).

**Built, every one with a fixture in `tests/rich_text.rs` naming the chapter's example:** the
containers `html` and `body`; `p`, `br`; `span`, `b`, `i`, `sub` and `sup` at the chapter's own 15%,
31% and 66%; `a` in the style the chapter recommends; `ol`, `ul`, `li` with the default tag per level,
`start`, `type`, `value`, `list-style-type` for the numeric, Latin, Greek and Roman types, compound tags,
and the half-inch list indent and six-point tag gap *List Layout* fixes; `font` and its longhands
`font-family` (a search path), `font-size`, `font-weight`, `font-style`; `color`; `text-align` with
`justify` and `justify-all`; `text-valign`; `text-indent`; `line-height`; `margin` and its four sides;
`vertical-align`; `text-decoration`'s four underlines and `line-through`; `letter-spacing`;
`xfa-font-horizontal-scale` and `xfa-font-vertical-scale`; `xfa-spacerun`; the `xfa:spec` version gate
on lists (*Legacy Handling of List Content*, page 1217). A `/DS` is the same declaration grammar and
sits beneath the markup.

**Read, and nothing to act on:** `orphans`, `widows` and the three `page-break` properties govern text
flowing from one content region to the next, and an annotation's appearance is one box.

**Named, stated by a file, and not carried out — reported** as `Owed::RichTextUnapplied`, the rest of
the string drawn: a `font-stretch` other than `normal` (§9.6.2.2 and §9.6.3 name no face by width),
`kerning-mode:pair`, the tab properties, following an `a href`, `xfa:embed`, and the algorithmic and
Kana, Hangul and Iroha list types.

## 2. Which face a run is set in

Chapter 27's `font-family` is a search path, character by character (page 1201). Each family resolves,
with the run's weight and posture, to the first of: **the `/DA`'s own face**, where all three are its
own, so a string restating the field's default draws exactly as the plain value; **a `/DR` face** whose
Table 120 `/FontFamily`, `/FontWeight` and `/Flags` — or failing those its `/BaseFont` — say so;
**§9.6.2.2's fourteen** for Helvetica, Times and Courier and the two faces without a family, which this
binary carries; and **a stand-in** named in §9.6.3's `Family,Bold` spelling that `pdf_font` places as
it places any unembedded font a page names. The `/DA`'s face ends every path. CSS2's generic families
map to the fourteen's three, a choice that keeps them machine-independent. The cascade's root weight
and posture are the `/DA` face's own. A character no face draws, in a face this program chose, falls
back to the one-style layout and its machine face (ADR 1414), rather than drawing a scatter.

## 3. Lengths, and where chapter 2 and CSS2 disagree

Chapter 2 makes a unitless measurement inches; CSS2 section 4.3.2 requires a unit after every length
but zero, and section 4.2 ignores an invalid declaration. Chapter 27 makes CSS2 normative for values, so
CSS2's rule is applied and `font-size:12` is dropped. CSS2 section 15.7 measures `font-size`'s `em`
against the parent; every other `em` is the element's own size, chapter 2's `%` the width of a space.
A size is held as `per_root × root + points`, so §12.7.4.3's auto-size moves every relative length with
it and no absolute one.

## 4. Colour

Chapter 27's two forms are sRGB, and the stream writes them as `DeviceRGB` `rg`. **A choice with a
cost**: a device whose `DeviceRGB` is not sRGB shows the colour in its own sense of the numbers. An
`ICCBased` sRGB space would need a profile in every appearance's resources for a difference this
program's screen output does not show; the next round that writes an output intent is where to revisit.

## 5. Positions

Both texts hand a processor the positions — §12.7.4.3's "positioning values it determines to be
appropriate", and chapter 27's examples are informative — so they are chosen, and chosen to be the
plain layout's: a single line centred, several from the top, the first baseline one ascent down, a
line 13/12 of its tallest em below the last, CSS2 section 8.3.1's collapsed paragraph margins between.
`a_string_with_no_style_of_its_own_lands_where_the_plain_value_does` holds the two equal. The underline
a tenth of an em below the baseline, the line through 0.28 em above it, each 0.05 em thick, are choices
CSS2 section 16.3.1 also leaves to the reader. The `/DA`'s `Tm` is not applied to rich text: it is one
of the conventions §12.7.4.3 sets aside for these fields (ADR 1635).

## 6. What the runs do not reach yet

Three constructions take the one-style layout, each reported as `Owed::RichTextOneStyle`: a comb field;
a value holding a right-to-left run, whose UAX #9 order lives there (ADR 1413); and a host's question
about a caret, a point or a range. The popup window's formatting is a host's to draw (§12.5.6.14), and
an import does not carry Table 249's `/RV` yet. Each is the next build's, named in `doc/todo/65`.
