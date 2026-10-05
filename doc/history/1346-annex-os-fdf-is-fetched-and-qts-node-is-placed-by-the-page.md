# 1346 — Annex O's `fdf` on a server is fetched, and Qt's document node is placed by the page

The HOST-UI round of batch fifty-four. ADRs 1527 and 1528. No status moved; O.2.2's note is updated.

**The fetch (ADR 1527).** Table Annex O.4: "Open the document and then import the data from the
specified FDF or XFDF file". `viewer_host::import_is_fetched` sends an absolute URI to
`may_fetch_import`, which follows `may_submit`'s order. No scheme, a scheme outside
`SUBMIT_SCHEMES`, or a URL `check_url` refuses is refused at every level. After that the
`Submissions` level decides. The act's name is now "sending a form or fetching its data", and
`--submissions=` sets the level in all three windows. `submit::fetch` is the submit client's GET:
`TIMEOUT`, no redirect, the body bounded at `RESPONSE_LIMIT` (64 MiB), and a larger one is refused
by name. The format still comes from the URI's name. **One departure from the design:** the bytes
cross as `Command::Respond { answers: Answered::Import }` rather than as `Supply`, because `Supply`
answers whichever tab is in front and a fetch arrives later. Its sentences read "import-data", the
same words the relative route uses. `quorra-confined` refuses at every level and says it has no
network. Tests: `viewer-host/tests/fetch_import.rs`, 7, on a loopback listener (each level, three
refused URLs, an answer over the bound, an HTML body, a 404).

**Qt (ADR 1528).** I measured it first. The drive's probe found Qt's document node for character 1
at 83, 367 and the `QLineEdit` at 454, 428, so the node sat 371 × 61 px off. `reportPageArea` sends
`page_->mapTo(window)` as `page_placed`, and `Reading::at` carries it. After the change the node is
at 464, 439, inside the field. `29-field-extents` now reads the document node in Qt and holds it to
the field.

**Driven** (release, Xvfb :146, all four windows): `31-fragment-fdf` imports "Fetched" into field A
at `send`. `31-fragment-fdf-refused` reports the refusal at `refuse`, and the server logs no request.

**Gates.** fmt on my 19 files: 0. clippy `-D warnings` on `viewer-core` and `viewer-host`: 0. For
`viewer-ui`, `viewer-gtk` and `viewer-qt` it stops at a sibling's `render-raster` line
(`own_space.rs:175`, `TMPFRAME`); without `-D`, my crates give 0 warnings. nextest `viewer-core` +
`viewer-host`: 502/502, exit 0. `viewer-ui` + `viewer-gtk` + `viewer-qt`: 195/195, exit 0.
`conformance`: 0. `selection_census`: exit 0, find 998/1010. `accessibility_census`: exit 0.
`launch_path`: exit 101. `bug1815476.pdf`'s peak_anon is 50.73 MiB against a 50.0 band. My changes
are off the launch path, and a sibling's `launch_path.rs` and `pdf-model` edits are in the tree.
Drive: exit 0 in 633 s, 105 works, 0 wrong, 3 not offered.

**Left.** Nothing of `doc/todo/39`. The `ask` level is reachable in all three windows but not
driven, because answering a question card under Xvfb is a step of its own. Files outside the brief:
`doc/conformance/ledger.toml` (row O.2.2: two sentences of the note, one test entry; 1350's file),
`doc/state-of-play.md` (one sentence).
