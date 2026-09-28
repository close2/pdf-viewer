# 1266 — An untagged page's fields are controls, and a held button stays down

The HOST-UI round, batch forty-one. ADRs 1369, 1370. No ledger row moved: the contract named none.

## Untagged widgets on AT-SPI (ADR 1369)

- §14.9 owes nothing about untagged files; §12.5.1's click, Table 226's `/TU` and Table 31's `/Tabs`
  decide it. `PageStructure::widgets` carries an untagged page's interactable widgets in tab order;
  `viewer-accessibility` publishes them after the untagged sentence. Confined wire carries the list.
- Census: 87 untagged pages publish 386 widgets (new floors, tracked and whole population); a
  tagged page with a widget list is held at zero. No existing floor moved.
- Driven in GTK, Qt and `quorra`: `button 'Go to page two'` + `entry 'Your name'`; `DoAction` turned
  the page in all three. `form_two_pages.pdf`, `two-buttons.pdf` (row order kept) also walked.

## GTK's down appearance (ADR 1370)

- A `Moved` during a press keeps §12.5.5's `/D`. The drive found GTK's zero-length `Dragged` turning
  a push-button click into a selection when `/D` has its own caption; a drag inside a pressed push
  button no longer moves the selection. Screenshots mid-press: `/D` shown; release turned the page.

## quorra's Space/Enter

- Driven: Tab + Enter and Tab + Space activate the focused push button; Tab Tab + Enter on a
  `/Tabs /R` page reaches the lower button and its `/Named /LastPage`. Nothing to fix.

## Path rule (ADR 1369 section 3)

- `resolve_import` admits §7.11.2.2's relative path below the document (its EXAMPLE 1 is a test),
  splits by §7.11.2.1, refuses `..`/`.`/empty/absolute. `issue17846.pdf` opened its `/UF` file two
  directories down, beside the source, at `--remote-documents=open`.

Tests: `viewer-core/tests/untagged_widgets.rs`, two in `headless.rs`, two in `tree.rs`, one in
`host_mappings.rs`, the census counts.

## Left

- A tagged page's widget no `Form` names reaches no node.
- A press on nothing then a `Moved` onto an annotation still shows its rollover.
- `quorra`'s tab label draws CJK file names as boxes (chrome font), seen on the witness.
