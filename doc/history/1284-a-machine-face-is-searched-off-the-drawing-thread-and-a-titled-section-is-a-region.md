# 1284 — A machine face is searched off the drawing thread, and a titled section is a region

The HOST-UI round, batch forty-four. ADRs 1405, 1406. No ledger row moved: the contract named none,
and §14.7.2, §14.8.4.4, §14.9.3 and §9.7.4.2 were already `implemented`.

## The launch-path cost of ADR 1382 (ADR 1406)

- `launch_path` cannot see it: its first-page figure leaves the window and the chrome out by design.
  `quorra --trace=frames,launch` under Xvfb, fonts evicted from the page cache for "cold":
  a Chinese tab label cost the frame after first present 1024.6 / 1032.2 ms cold (100 / 120 warm);
  a document opening on an outline panel with one Chinese item presented first at 1136.9 / 1017.9 ms
  cold (205 warm) against ~100. One Chinese-named document alone: no walk (a strip of one is hidden).
- `viewer_host::machine_faces`: `ask` never searches; one thread, faces found first, wakes the loop.
  After: outline case 92.6 to 104.4 ms cold; tab case 114 / 123 ms, the face lands at 0.98 s and that
  frame's host step is 0.3 ms. Screenshots: box at 0.3 s, characters at 3 s, outline drawn.
- `launch_path` after (release, load 2.35): 26 figures banded, 0 outside. Not run before the change;
  its measured path holds none of the changed code.

## Qt's tab strip (ADR 1406)

- Reproduced: a standalone Qt program draws the label as boxes, and correctly once
  `addApplicationFontFromData` is given `DroidSansFallbackFull.ttf` (family `Droid Sans Fallback`).
- `quorra-qt` asks `machine_faces` for what `compiled_in_lacks`, `pumpFaces` registers each file on a
  timer armed only while a search is out. Driven: `latin.pdf 多边形批注.pdf` and
  `多边形批注.pdf icc.pdf outline-cjk.pdf` draw the Chinese tab. `quorra-gtk` unchanged (Pango).

## A titled element (ADR 1405)

- Table 355's `/T` is read for every element; `AccessibilityNode::titled` crosses (and the confined
  wire); a titled `Sect` is `Role::Region`, AT-SPI `landmark`. `Div`, `Part` keep their roles, named.
- Census: 20 elements now named by `/T` (5 `Part`, 10 `TOCI`, 2 `TOC`, 3 `Formula`), 0 before by
  construction; new floor 20. No corpus `Sect` is titled without text: the fixture carries it.
- AT-SPI (own bus + registry, gi `Atspi`), `titled-sect.pdf` in `quorra`, `quorra-gtk`, `quorra-qt`:
  `landmark 'Chapter 1'` > `paragraph`, `section 'A division'` > `paragraph`.

## Left

- The box shows for up to a second cold before the face; a label's width changes when it lands.
- `raster-gpu`'s `fill_solid` is over clippy's line limit (a neighbour's file) and stopped
  `-D warnings` clippy of the viewer crates; run without it, the touched crates are clean.
