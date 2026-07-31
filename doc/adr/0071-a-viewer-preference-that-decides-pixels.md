# ADR 0071 — A viewer preference that decides pixels

Status: accepted, 2026-07-31.

## Context

§12.2 was a `silent` row, and its ledger note described it as a window's business: "[m]ost
describe a window this program does not have — it shows one page, with no panels and no chrome
to hide". That is true of eleven of Table 147's eighteen entries and false of four, and the four
it is false about were not named in the note at all.

`/ViewArea`, `/ViewClip`, `/PrintArea` and `/PrintClip` each hold "the key designating the
relevant page boundary in the page object". `/ViewArea` is "the name of the page boundary
representing the area of a page that shall be displayed when viewing the document on the screen";
`/ViewClip` is "the name of the page boundary to which the contents of a page shall be clipped
when viewing the document on the screen". Both are normative sentences about what is on a screen,
and this program is a thing that puts pages on a screen.

They also depend on a clause that was itself half read. §14.11.2.1 defines five boundaries and
this tree read two: the media box and the crop box. The other three had a `partial` row saying
they "describe where a press cuts and are read by nobody" — correct about their purpose and
incomplete as a reason, because §12.2 can name any of the five as the one to display.

## Decision

**Read all of Table 147, and apply the two entries that decide what is on the screen.**

- `viewer_preferences.rs` reads every entry with Table 147's own defaults, including Table 148's
  `/Enforce`. It holds no policy: a caller with a window decides what to do with `/HideToolbar`.
- `page.rs` gains §14.11.2's five boundaries as `Boundary`, with `Page::boundary` the rectangle.
  The three new ones default to the crop box, are read from the page object alone — §7.7.3.4
  makes only four entries inheritable and none of these is one — and are each treated as "its
  intersection with the media box", which is §14.11.2.1's rule for all four non-media boxes and
  had been implemented for the crop box only.
- `Page` gains `display_box` and `clip_box`: the `/ViewArea` and `/ViewClip` answers. Everything
  that used to read `crop_box` as "the visible region" now reads `display_box`, and `crop_box`
  goes back to meaning what the file says.
- `content.rs` builds one clipping path when `clip_box` does not cover `display_box`, and it is
  the initial clip for both the page's content stream and every annotation drawn over it — an
  annotation is drawn *on* the page and is not exempt from what the page is clipped to.

## Why implement something deprecated, with no corpus witness

Two questions, and they have different answers.

**Deprecated in PDF 2.0.** Deprecation is an instruction to writers. §8.6.5.1's withdrawn
`CalCMYK` is already read here for the same reason: a reader's job is to draw the file it was
given, and a file written in 2005 is not improved by a 2020 decision.

**Zero corpus documents state any of the four.** Measured, not assumed: 58 of the 974 state a
`/ViewerPreferences` at all — `/Direction` 29, `/DisplayDocTitle` 22, `/PrintScaling` 2, one each
of five window flags, and three that write a `/PageDirection` that is not in Table 147 — and not
one names a boundary. This is the specification track working exactly as `CLAUDE.md` principle 5
describes it: a demand curve cannot rank a requirement no file exercises, and the cost of being
wrong here is a page displayed at the wrong size for the one document that turns up tomorrow.

The consequence for the gates is the honest one: **nothing moves**, and nothing was expected to.
The tests are the clause's own two questions asked of a fixture — where the raster ends, and
where the ink stops — and each was confirmed to fail when its half is removed.

## What is still owed, and it has a corpus behind it

`/DisplayDocTitle`, which 22 documents set. The title is "taken from the `dc:title` element of the
XMP metadata stream (see 14.3.2)", and §14.3.2 is a `partial` row whose note says no XMP reaches
anything. So the entry is read, its meaning is not available, and the row says which of the two
it is. Nothing pretends otherwise by falling back to the file name silently — that *is* the
entry's `false` case, which is why the gap is invisible in a window and is written down here.

## Consequences

- `silent` falls 172 → 171; §12.2 becomes `partial` and §14.11.2 and §14.11.2.1 lose their
  "read by nobody" clauses.
- `Pages` reads the preferences once, from the catalog it already holds, and carries the two
  boundary keys. That is one dictionary lookup at open time and none per page — the startup rule
  in `CLAUDE.md` is about work proportional to the document, and this is not.
- `Page` grew five fields. Every one of them is a rectangle the file states or the clause
  defaults, which is what a page *is*; the alternative — computing them at each use — would have
  put §14.11.2.1's intersection rule in more than one place.
