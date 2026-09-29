# 1272 — A tagged page's omitted field is a control, a held button shows no rollover, and a CJK tab is words

The HOST-UI round, batch forty-two. ADRs 1381, 1382. No ledger row moved: the contract named none.

## A tagged page's widget no element names (ADR 1381)

- The brief's Table 368 for `/OBJR` is Table 358 (§14.7.5.3); Table 368's `Form` `shall` binds the
  producer. `PageStructure::widgets` now holds, on every page, the interactable widgets no published
  element names (by §14.7.5.3's `/Obj` or Table 357's `/StmOwn`), published after the structure's
  nodes in `/Tabs` order, named from `/TU`.
- Census: new floors 7 pages / 82 widgets tracked, 44 / 120 whole population (issue16500.pdf states
  no `/OBJR` at all); a widget both an element and in the list is held at zero, replacing ADR 1369's
  "tagged page with a list" class. No existing floor moved.
- Driven: `quorra-gtk` on AT-SPI shows the `Document` subtree then `push button 'Place the order'`
  with `click`, the `Form` element's `entry` not repeated.

## The rollover without pressing (§12.5.5)

- "The rollover appearance shall be used when the user moves the cursor into the annotation's
  active area without pressing the mouse button." `Viewer::button_down` records the button whatever
  the press landed on; a held `Moved` and a `Dragged` show `/D` only over the pressed annotation.
- The drive in `quorra` found a drag off a pressed button kept its `/D` (winit reports `Dragged`);
  now it ends there and returns on coming back. `a_move_with_the_button_down_shows_no_rollover`.
- Driven mid-press in `quorra-gtk` and `quorra` by pixel: nothing-then-onto is `/N`, the release
  `/R`; held from one button to the other shows `/N` on both.

## The chrome's CJK (ADR 1382)

- `doc/todo/27` had refused a machine fallback; ADR 1382 revisits it: `Chrome::new` asks
  `installed_covering` for a character both compiled-in routes lack, loaded through
  `LoadedFont::load`; `Chrome::compiled_in_only` keeps the old chrome. Test skips without a face.
- Driven: `quorra rollover.pdf 多边形批注.pdf` — the tab reads 多边形批注.pdf, front and behind.

## Left

- A tagged page whose structure reaches nothing on it says ADR 0214's untagged sentence.
- A tagged `Form` element with no text of its own is named `''` on the bus, not from `/TU`.
