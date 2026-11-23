# Round 2199 — audit 2197's fixes: chains in order, `/SD` on every route, outline ink and style

A fix round in the `audit` worktree for review 2197 (§12.3, §12.4, §12.6): its four B findings,
five failing tests and two D findings, in the four groups the review suggested.

## What changed

- B03, D02 (ADR 3497): `interact::resolve` folds a `/Next` chain's page requests in order; a
  named `NextPage`/`PrevPage` counts from the page the previous request reached; a movie's `/T`
  is looked for on the page reached when it came. A §12.6.4.15 transition a later page request
  moves away from is played into the chain's end and said out loud (one face per command).
- B01, D01: `outline::destination` reads the go-to through `Destination::of_go_to`.
- B02 (ADR 3498): `RemoteGoTo` carries `/SD`'s Table 149 view; `RemoteGoTo::page_in` answers
  page and view from the entry that decided, page 0 for an element found without content.
- B04 (ADR 3499): `PanelRow` gains `style` (`ItemStyle`: ink, italic, bold); GTK draws Pango
  attributes, Qt `ForegroundRole` and `FontRole`, `viewer-ui` already drew `Item`; the C ABI gains
  `quorra_outline_style` (header, .NET declaration). Black is carried, not left to the theme.
- Manifest: 2197's five entries removed; 2197's findings marked fixed.

## Ledger

12.3.3, 12.6.4.2, 12.3.2.3, 12.6.4.3, 12.6.2, 12.6.4.12 re-read: `implemented` before and after;
notes restated and the audit tests added to each row.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets -p pdf-model -p viewer-core -p
viewer-host -p viewer-qt -p viewer-gtk -p viewer-ui -p viewer-ffi` exit 0. `pdf-sandbox` 0.
nextest `--lib --bins --tests` of the seven (`xvfb-run -a`): 4408 run, 4406 passed, 2 failed —
viewer-ffi's two entry-point counts, raised for `quorra_outline_style`, then 8 of 8 passed;
`audit-diff.py` against the emptied manifest: nothing else. rustdoc `-D warnings
--document-private-items` of the seven, private target, exit 0. `fuzz` `cargo check --bins`
exit 0. `cargo test -p conformance --no-fail-fast` exit 0.

## Habit

A clause about the *order* of a sequence is a clause about every request in it, including the
one (a transition) that draws instead of moving.
