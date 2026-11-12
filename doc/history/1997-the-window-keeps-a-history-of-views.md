# 1997 — The window keeps a history of views

Session 1997, a single ledger round (§12.3.2.1, JavaScript `app`), the ledger lane. ADR 2805.

**The slice, and why.** `doc/todo/76` section 3 orders rows 4 and 5 next. Row 4 is the larger and
the one a person meets: no window kept a history of views, so neither a person nor a script's
`app.goBack` had a previous view. Built whole; row 5 (a page's text on the confined pipe) is left
named, because the walks row 4 owes filled the budget at a load of 130–273.

**The premise.** Held. `grep` named only the census and the refusal; ADR 1736 section 1's gate was
"until a host round builds a view history", effort only. No `doc/questions/A*` answer cites it.

**Built.** `viewer_core::history::History`, per open document: a jump (a destination followed in
`Viewer::apply`, `PageTarget::Index`/`First`/`Last`, a script's `this.pageNum` and `gotoNamedDest`)
records the `Viewing` it leaves; steps (next, previous, relative, scroll, zoom) and a presentation's
turns do not. `Command::History(Retrace)` steps back or forward and restores through
`Command::View`'s path; a step with nowhere to go is said. Each stack holds 256. It crosses the
confined frame as command kind 52; `viewer_host::keys` binds Shift with Left and Right in every
host. `app.goBack`/`goForward` leave `surface::REFUSED`; a run's calls cross as one
`ViewChange::History(steps)` (wire version 15).

**Tests.** `history_of_views.rs` (four), `history::tests` (two), `window_view.rs`'s two, the wire and
confined round trips, `keys`' shift test. **Plant**: the jump in `apply` not recorded, a next-page
turn recorded, `goForward` a step back, Shift+Left a page turn. Each fails its test; reversed, pass.
**Found by the script test**: a `gotoNamedDest` and an `app.goBack` in one run went back, and then
`settle` applied the destination's still-pending view to the restored page. A step now clears the
pending views (a restored view is whole). **Habit**: a nextest filter for a plant names tests by
the words in their names; `test(/history/)` missed every `history_of_views.rs` test but one.

**Gates.** fmt `--check` over the nine crates: exit 0. Clippy `-D warnings --all-targets`: exit 0.
Nextest of the nine: 3777 of 3781 on the first pass, two failing (the two tests fixed above) and the
two `viewer-ffi` C-program tests stopped unfinished at a load of 136–273 (at the merge, `viewer-ffi`: 77
of 77); the touched binaries after the fix: 70 of 70. Conformance after `git add -N`: exit 0 at the
merge. `fuzz cargo check --bins`: exit 0. Tier 2: `script_corpus` model 0, engine 101 as round 1977's
run before this change (2 runs over the wall budget, 13833 runs under the 21101 floor); both censuses and
`awkward_classes` 0; `corpus` and `raster_golden` 101, HEAD's 62 and 16 exactly. `state.sh docs`: 0.

**Left of the rows.** §12.3.2.1 stays `implemented`, its note naming the history. No step across
§12.6.4.3's or §12.6.4.4's change of document; no `Query` for the stacks' depths. `doc/todo/76`
row 5 is next in that file's order.
