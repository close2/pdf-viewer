# 1540 — A question card is driven by its buttons' `Action` in the toolkits and by its keys in `quorra`

Status: accepted. Session 1352. Context: ADRs 1145, 1453, 1478, 1527; `tools/drive-windows.sh`;
Table Annex O.4's `fdf`. Supersedes nothing.

## Decision

The `ask` level's question is driven, in all three windows, as step `31-fragment-fdf-ask-yes` and
`-ask-no`: the window is launched on `drive-form.pdf#fdf=<the drive's server>` at
`--submissions=ask`; the step first checks that nothing was fetched and no import was said while
the question stands; it answers; and it reads the result off the log and the server — yes, one
request and `import-data: 1 field(s) from …, into 1 widget(s)`; no, no request and the decline's
sentence.

Where the card lives decides how it is answered:

- **`quorra-gtk` and `quorra-qt`**: a toolkit dialogue (a `GtkWindow`, a `QDialog`), a separate
  top-level that Xvfb with no window manager does not place over the page. Its buttons are found
  on the AT-SPI bus by role and name ("Go ahead", "Do not" — `viewer_host::restriction`'s words)
  and pressed through their `Action` interface (`press.py`), with no coordinates. A button found
  is also the witness that the card is up. This machine's AT-SPI names the role `button`, where
  older versions said `push button`; the script takes either.
- **`quorra`**: the card is drawn, not published, so it is answered by its two keys, Enter and
  Escape, and photographed before the answer (`…-card.png`) as the witness that it is up.

Rejected: clicking the dialogue's buttons by coordinates measured on a picture, which moves with
the font and the toolkit's layout and, with no window manager, with wherever the toolkit placed a
transient window.
