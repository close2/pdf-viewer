# ADR 0276 — A mask group is not a group the page composites in

Date: 2026-08-11 (session 440)
Status: accepted

## Context

After ADR 0272 closed the largest condition `doc/todo/23` had, the standing row was **§11.6.6's
group-level conversion at 85 of 65 944 web documents** — a page that states §11.4.7's
`/Group << /S /Transparency /CS /DeviceCMYK >>`, and a group inside it that composites in some
other space, so the page is drawn on the device's three components and reported by name.

`doc/todo/02` §1 and this chain's own method say to read the clause before pricing the
construction, and four rounds running the clause's algebra has collapsed something (ADRs 0220,
0234, 0237, 0262). This round asked the prior question instead: **what are the 85?** The survey's
label is this tree's report string, and sessions 415, 426, 427 and 436 each found a label
describing something narrower or wider than the code's condition.

## The measurement

`Interpreter::blending_changed` is the flag that fires the report. One `eprintln!` at the place it
is set — the two spaces, the group's `/I`, and the interpreter's `soft_mask_depth` — over all 85
documents, 1320 changes:

| where every change on the page happens | documents |
|---|---|
| **inside a soft mask's group** | **77** |
| on the page itself | 8 |

Not one document had changes in both places. The three corpus documents are the same shape:
`bug1703683_page2_reduced.pdf`, `bug1755507.pdf` and `issue13520.pdf` each have four to six
changes and every one of them is at `soft_mask_depth = 1`.

The eight real ones are 32 sites of three kinds: an isolated group with a three-component `/CS`
inside a `/DeviceCMYK` page (20), an isolated `/DeviceCMYK` group inside an isolated
three-component one (10), and an isolated `/DeviceGray` group inside a `/DeviceCMYK` page (2).

## Why 77 of them are not this clause

`build_soft_mask` already clears the blending space in force for a mask group's content, and ADR
0220 is why: §11.5.3 composites such a group against a backdrop of its own and reduces the result
to one number —

> The second method of deriving a soft mask from a transparency group shall begin by compositing
> the group with a fully opaque backdrop of a specified colour. The mask value at any given point
> shall then be defined to be the luminosity of the resulting colour.

— which §11.6.5.1 uses as the mask's alpha. §10.4.2.3's conversion to that luminosity is linear in
the components, so the compositing this tree performs inside such a group *is* the compositing the
clause asks for, whatever space the group names.

**The flag that records a change of space was not part of that scope.** It was added in ADR 0262
for a different question — may the *page* be composited in the four components §11.4.7 gives it? —
and because the line above makes every group inside a mask compare its space against `None`, an
isolated group declared inside a mask counted as a group the page composites in. A mask group's
result never reaches the page as a colour at all; it reaches it as an alpha.

So the fix is two lines in the scope that already existed, and it is the clause's own boundary
rather than a narrowing of the condition (trap 5): a page whose only space change is inside a mask
now composites in ink, and a page with one on the page itself still reports.

```rust
let saved_blending = self.blending.take();
let saved_change = std::mem::replace(&mut self.blending_changed, false);
self.run(&content, &resources, &inner, 0);
self.blending_changed = saved_change;
self.blending = saved_blending;
```

## The fixtures, and the arithmetic they are held to

`transparency_groups.rs` gains one fixture that puts one isolated `/DeviceGray` group in either of
two places — inside §11.6.5.1's `/G`, or on the page — with everything else held fixed, and two
tests over it.

- `a_group_declared_inside_a_soft_mask_leaves_the_pages_blending_space_alone` asserts the page is
  complete and reads the pixel where an opaque `0 0 0 0 k` is covered by a half-opaque
  `1 1 1 1 k`. §11.3.4 applies the compositing formula per component, so that pixel holds ½ of
  each of the four inks, and the assumed press's conversion out of the cube at (½, ½, ½, ½) is the
  mean of its sixteen corners: **(76.0, 66.1, 63.9) of 255**. Putting the old route back — deleting
  the two lines above — fails this test and nothing else.
- `the_same_group_on_the_page_still_reports_and_still_draws_on_the_device` moves the same group to
  the page and asserts both halves of the other answer: the report fires, and the pixel is
  **127.5**, because converting each colour first and averaging on the device is 51 of 255 from
  what the clause composites (ADR 0251).

## What the gates said

The six gates a changed raster can reach — corpus, oracle, quorra, both text gates and the
conformance counts — were run **on the stashed tree and again after**, in this round, rather than
compared with the table. `dates`, `xmp` and `jpeg2000` were run after only, and reproduce what
`doc/HANDOVER.md` records; nothing this round touched can reach them.

| | before | after |
|---|---|---|
| corpus, 974 documents | 68 incomplete | **65** |
| oracle verdicts | 905 / 68 / 786 | **identical** |
| oracle complete / incomplete | 1690 / 104 | **1693 / 101** |
| quorra | 917 / 35 / 5 / 17 | **identical, name for name** |
| text vs `pdftotext` | 99.2% (24003/24187), 65 not gated | 99.2% (**24007/24191**), **62** not gated |
| text vs PDFBox | 99.8% (14257/14281) | identical |
| dates, XMP, JPEG 2000 | 1514/1545, 318/1, 14 | identical |
| tests | 1609 | **1611** |
| citations / quotations | 6490 / 617 | **6505 / 618** |
| ledger | 875 rows, 406/248/18/82/8/113 | identical |

**The two conformance figures in `doc/HANDOVER.md` were behind and this round's before-run is what
found it**: the table said 6469 and 612, which are session 438's, while session 439's own history row
records 6490 and 617. Running the gate on the stashed tree is the only way that shows, and it is
`doc/todo/02` §2's own rule about this table restated — it carries the gate's number rather than the
last round's.

**quorra being identical name for name is the sharpest of these**: it compares this backend's
raster page by page, it draws §11.4.7's two rasters since ADR 0275, and the three pages whose CPU
raster changed still agree with it.

**`doc/todo/00`'s step 7, over all 786 ambiguous pages, before and after: two lines differ and they
are the two pages this round moved** — `bug1703683_page2_reduced.pdf` +0.141 → +0.146 and
`issue13520.pdf` +0.695 → +0.507, each losing its `[incomplete]` label. Every other line of 786 is
byte-identical, twenty names sit at or past −1, and they are the same twenty. The alarm holds for
the fifteenth consecutive run.

## The two pages that arrived at the oracle's judgement

A page this tree reports is excused the oracle's demand for a diagnosis, so two of the three
arrived undiagnosed and the gate failed until they were read. Neither is ambiguous *because* of the
blending space, and that is measurable rather than arguable — the same page can be drawn both ways:

| RMSE | ours in ink vs ours on the device | poppler vs ghostscript |
|---|---|---|
| `issue13520.pdf` | 0.0144 | 0.0736 |
| `bug1703683_page2_reduced.pdf` | 0.0018 | 0.0229 |

A fifth and a thirteenth of the references' own disagreement. Each therefore has its own reading in
`AMBIGUOUS_PAGE_DRAWN_IN_INK`: on `bug1703683_page2_reduced.pdf` two ladders agree at the limit and
`mupdf` is 0.144 of 255 below both, with every lit pixel of the difference image on the outline of a
photograph or a glyph; on `issue13520.pdf` **no two of the five draw the same picture** —
`poppler` omits every white highlight, `hayro` paints a dark disc over the right-hand bulge — and
the two ladders end 0.74 apart moving in opposite directions.

## The web population, before and after

65 944 documents, 145 archives, one process apiece, both passes with no failure of any kind:

| | before | after |
|---|---|---|
| **incomplete** | 905 | **851** |
| blending-space reports, all conditions | 157 over 156 documents | **83 over 82** |
| a group inside the page composites in a different space (§11.6.6) | 85 | **8** |
| a non-separable blend mode (§11.3.5.3) | 28 | **31** |
| an `/ExtGState` states Table 57's `/BG` or `/UCR` (§11.7.5.3) | 9 | 9 |
| four components that are not four this tree can sample | 5 | 5 |
| a group *introduces* a space on a page that states none | 30 | 30 |

**54 of the 77 become complete and 23 keep a report they already had** — 21 of §11.4.4's
non-isolated group, one knockout, and **three that join the non-separable row**, which is why that
row rises by exactly three. A population narrowing until the next condition each document meets
fires is trap 5's honest direction, and all three are in the 77 by name (`0792036.pdf`,
`4100373.pdf`, `6696551.pdf`).

**The largest transparency row the web has is now §11.3.5.3's non-separable blend at 31**, and
§11.6.6's is fourth.

The `/BG` row was measured the same way before it was left alone: all nine documents state Table
57's black generation at `soft_mask_depth = 0`, so `black_generation_stated` being monotone for the
page — which is deliberate, and documented where it is set — costs nothing here.

## Consequences

- **`doc/todo/23`'s standing item falls from 85 web documents to 8**, and what is left of it is one
  construction rather than four: a group *on the page* that introduces a space of its own needs its
  own pair of rasters and a conversion between two presses at its `Do`. Both halves of that
  conversion exist (`Press::blending_space` out, `colour::rgb_to_ink` in); what does not exist is a
  display-list vocabulary for a group in a space of its own, which is a round of its own and is
  priced in that file.
- **The lesson is about scopes rather than about clauses.** `blending`, `compositing` and
  `uncoloured` are saved and restored across a mask group; `blending_changed` was added later, for a
  question about the page, and joined none of them. A flag that answers a question about one scope
  has to be scoped, and the way to find out that it is not is to ask the population what it contains
  before believing its label.
