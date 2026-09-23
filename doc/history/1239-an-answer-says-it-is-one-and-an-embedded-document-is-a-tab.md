# 1239 — An answer says it is one, a layout waits for no key, and an embedded document is a tab

The HOST-UI round of batch thirty-seven. ADR 1316. No ledger row was briefed and none moved.

## Driven under Xvfb, two documents each time

- **GTK's first transition**, with `full.pdf` (`/FullScreen`, a `/Wipe` per page) first and alone:
  `--trace=launch,frames` showed the faces at 509x1019, then `Resize 890x1090` at the first key.
  Opened after `plain.pdf` it hid, because the second tab's arrival re-laid the window. Fixed, both
  transitions are 890x1090 in every order.
- **Qt's first heading**: fifty launches, click after map, at three delays, with the pointer parked
  over *copy → off*, and with the full-screen document alone. No level was chosen; the detector was
  checked against a deliberate choice. Not reproduced.
- **§O.2.1's `ef`** in all three windows, `'issue17056.pdf#ef=destination-doc.pdf&page=3'` first
  and after `plain.pdf`. The embedded document opens at its page 3 in its own tab: in front where
  its holder was, behind where its holder was. Ctrl + Tab walks all three.

## Findings and fixes

1. A server's answer was worded "import-data:". `interact::Arrival` now words it "submit-form
   answer"; the import-data action keeps its own words.
2. GTK hid the chrome inside the allocation that opened page one, and nothing re-laid the window
   until a key. `apply_chrome` now queues a resize from the idle queue.
3. Qt's stray level: not reproduced, count above. ADR 1303's record stands.
4. `quorra-confined` still holds one document (ADR 0713 Decision 2, ADR 1190's rule): it performs
   no open beside. `doc/todo/30` says so and names the one-worker question.
5. `doc/todo/38`: the attach gestures wait on the mockups and the descriptor route has no sender,
   so the `ef` entry's path was taken. GTK and Qt never opened the file; `quorra` replaced its first
   document with it. All three use `Arrivals::wait_held` now, and `quorra`'s `embedded` field is gone.

## Tests

`an_answer_to_a_submission_is_worded_as_one_and_an_import_as_an_import` (viewer-core),
`an_embedded_document_opens_from_its_bytes_under_a_name_beside_its_holder` (viewer-host), and
`submit.rs` reworded. The relayout and the tabs are toolkit event handling no gate drives (ADR 1240).
## Left

- The four levels over `ef`: `may_open_extracted` and the menu's third group (`doc/todo/38`).
- Read, not driven: in GTK and Qt a GoToR document opened beside that asks for a password is
  retried by `open_document` against the front document's bytes.
