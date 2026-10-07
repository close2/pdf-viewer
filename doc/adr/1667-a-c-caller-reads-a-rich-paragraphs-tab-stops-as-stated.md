# 1667 — A C caller reads a rich paragraph's tab stops as the paragraph states them

Status: accepted and **built**. Session 1415.
Builds on ADR 1655 (a rich note crosses the C ABI as runs, lengths as the caller resolves them),
ADR 1666 (a popup's tab stops), ADR 0737 (a struct added does not move `QUORRA_ABI_VERSION`).
Code: `crates/viewer-ffi/src/abi.rs` (`quorra_popup_rich_tabs`, `quorra_popup_rich_tab`,
`PdfvRichTabs`, `PdfvRichTab`, `QUORRA_RICH_TAB_*`), `include/quorra.h`, `c/open_a_page.c`. Tests:
`a_c_program_drives_the_abi` (the form fixture's note states `tab-interval:36pt;tab-stops:right
72pt` and a tab; the C program reads both), `header_and_library_agree`, `unsafe_position`.

## The question

ADR 1666 carries a paragraph's tab stops to the three windows, and a run's characters now hold a
`'\t'` per stop a tab advances. A C caller reading the same runs would see the tab character and
nothing to advance it to. Two shapes were open: the stops as `viewer_host::popup::tab_stops` places
them — resolved against a text size, a window width and a direction — or the stops as the paragraph
states them.

## The decision: as stated, and two entry points beside the paragraph

**What crosses is what the producer specified (ADR 1655), and the placing is the caller's.** The
placed list needs the window's base size, its width (how many default stops) and its direction,
all three the caller's; so a paragraph answers how many stops it states and its `tab-interval`
(`quorra_popup_rich_tabs`), and each stop its alignment and its position as so many of the
caller's text size plus so many points (`quorra_popup_rich_tab`), exactly as a run's lengths cross.
The alignment crosses as chapter 2 names it, `AFTER` and `BEFORE` included, because which edge
those are turns with the paragraph's direction, which the caller lays out. The header states the
rule a caller follows: next stop past the cursor, default stops every interval past the last stated,
**no advance where no stop lies past it**.

**`quorra_rich_paragraph` is not widened.** The struct is written into caller memory, so a caller
compiled against the older header hands over fewer bytes than a wider struct writes; new entry
points with structs of their own are a shape no older caller calls, so `QUORRA_ABI_VERSION` stays
(ADR 0737's reasoning). Entry points 220 → 222.
