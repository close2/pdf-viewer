# 2916 — The counted open no longer raises `/PV`, and its two floors are restated

Session 2049. Status: **accepted.** It moves two `open_kinstructions` bands in
`doc/checks/launch-path.toml` and no ledger row. Context: ADR 2666 (the counted figure is this program's
own object), ADR 2910 section 4 (three figures outside at `0dd1127c` under 1.97.1).

## 1. What moved the two counted figures

The bands were set from four children of HEAD on 2026-11-03, and round 1964's six judged runs on
2026-11-09 (`77514286`) held them. That commit's tree was extracted and built in release beside this
round's, and the gate's counted child was run under `callgrind` from both on the same documents:

| document | `77514286` | this round | band |
|---|---|---|---|
| `bug1815476.pdf` | 1827.1 k | 1744.4 k | 1808 .. 1846 |
| `opt_demo.pdf` | 430.1 k | 396.5 k | 425 .. 436 |

The inclusive cost of `viewer_core::Viewer::raise`, called from `page_events`, fell from 285.0 to
188.4 thousand on the first and from 386.7 to 343.8 on the second, with half as many calls; the dictionary
clones and drops it made (`Object::clone` from `Document::get`, `BTreeMap` `clone_subtree` and
`dying_next`) are most of the rest. The cause is `79d0c6c4` ("Every finding of the specification audit
is answered"): `page_events` used to push Table 197's `/PV` and `/PI` for every annotation of the page
opened and closed, and `visibility_of` now raises them for the pages that enter or leave the viewport,
since §12.6.3 says `/PV` is performed "when the page containing the annotation becomes visible". The
counted child builds its viewer with `Viewer::new(0, 0, 1.0)`: with no viewport no page is visible,
`visibility_of` returns after 44 instructions, and no `/PV` is raised.

## 2. Not a regression, and not a saving the program has

The figure fell because the counted open stopped doing work that a window's open still does: a window has
a size, so `/PV` is raised for page one as before, now from `visibility_of`. Nothing is faster for a
reader. The two floors are restated by the band file's own rule (the observed range of four children,
1744.4 and 396.5 thousand each time, widened by one per cent and rounded outwards: `1726 .. 1762` and
`392 .. 401`), and each row's comment says what left the count, so a later round that moves the counted
child to a sized viewport knows the figure will rise by about this much. The first-page clocks still
include the `/PV` work, which is where the reader sees it.

## 3. The warm open, and the run

`bug1815476.pdf`'s warm open read 2.68 ms at `0dd1127c` in round 2047, against `0.95 .. 1.88`. This round's
first judged run read 1.85 ms and its second 1.38, inside, at one-minute loads of 1.9 to 6.9 and up to
14.8, with the row's instructions, bytes and calls inside their bands: the 2.68 was that run's machine.
Its band does not move. The first run put
`PDF20_AN001-BPC.pdf`'s warm open at 2.392 ms against a ceiling of 2.390, 0.067 ms of it waiting for a
processor already taken, with that row's 2192.4 thousand instructions unchanged from 2047's 2192.3. It
is the band file's carve-out case, a clock outside while every counted figure of the row sits inside, and
it does not move the band either; the second run read 1.42 ms. That second run had a train walk beside
it, and four other clocks fell outside in it (EC3's turn, three of `opt_demo`'s), none of them outside in
the first.
