# 2173 — The audience reaches a C window on the panels record and its frames, not a kind of its own

Session 1668, batch one hundred and three's slot 6, a host round. Status: **accepted** and
**built**. Context: ADRs 1101 and 1106 (§8.11.4.4's audience), 2104 (the panels record), 2156
(the reader's words cross the handle, and its section 4's cost).

## 1. The question

ADR 2156 section 4 left the audience reaching a C window "shown by the unit tests' equality with
the typed words and by the directory's refusals, not by a page drawn under it", and the brief for
this round asked for an owed record carrying the layer a named reader would see.

## 2. What a named reader sees is already owed twice

§8.11.4.4's `User` category decides a group's state — "[i]f there is an exact match, the ON state
shall be used; otherwise OFF shall be used" — and nothing else. A window shows that state in two
places, and the C window already owes both: the layer row `quorra_window_layers_read` hands over,
read again on `QUORRA_OWED_PANELS` with its `on` (Table 99's state after `/AS`, which
`Query::Layers` answers), and the page area's frames, drawn under the same state. A kind carrying
the layer again would be a second answer to `Query::Layers`, which `WindowState`'s rule refuses
for every piece of chrome a query rebuilds.

## 3. Decision

No new owed kind. `drive_a_window.c`'s `the_audience` opens one layer whose `/Usage /User` names
Ada, managed by `/AS` at `View`, under `--reader-name Ada`, `--reader-name Grace` and nobody, and
judges the row and the ink: on and drawn for Ada, off and undrawn for Grace, and the
configuration's own state where nobody is named — the comparison with "the user's
identification" is not made where there is none, which is `pdf_model::optional_content`'s
reading (`Recommendation::Unanswerable`), seen here from C for the first time.

## 4. Costs

One C function, one fixture; no entry point, no constant, no C# declaration. A host that wants
to say *for whom* a layer is drawn reads the audience it stated itself.
