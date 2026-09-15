# 1065 — The constraints beside Table 197, and a location only a host knows

Date: 2026-09-15. Branch: `batch-1062-1067`, worktree `/home/AI/pdf-viewer-rounds`, shared with five
sibling rounds. ADR: [1079](../adr/1079-a-uri-is-resolved-against-a-location-only-a-host-knows.md).
§12.6's six `partial` rows: one moved, one advanced, one narrowed.

## §12.6.3 — `partial` → `implemented`

Table 197 defines *when* each mouse event happens; the paragraph after Table 200 states four
constraints on *when it may not*, and two were broken. "An E (enter) event may occur only when the
mouse button is up" — a cursor dragged into an annotation raised `/E` with the button held down, so
`viewer::pointer` now enters on no press and no drag and does not record the region as entered,
which keeps "[a]n X (exit) event may not occur without a preceding E event" true when the cursor
later leaves it; the entry is postponed to the first message that finds the button up. "A U (up)
event may not occur without preceding E and D events" — a release inside an annotation raised `/U`
whether or not it had ever been pressed, so a press on the page dragged into a widget and let go
there performed that widget's `/U`; `Open::inside` and `Open::pressed_on` are both required now. The
comment that shipped that one took Table 197's sentence for the whole condition; it is the whole of
Table 197 and not of §12.6.3. The fourth holds by construction: `over` is topmost, not a set. Gate:
`headless.rs::a_press_may_not_enter_and_a_release_needs_its_own_press`, over a fixture whose four
`/AA` entries are four *different* URI actions — `with_triggers`'s layer switch is one bit and
cannot tell an `/E` from a `/D`; trap 13, each half planted back and named by its own assertion.
`implemented` because the debt the old note kept — ten of Table 201's twenty types performed — is
§12.6.4's, carried by that row and the rows under it; a ledger in which a clause inherits the debt
of every clause it cites has no `implemented` rows at all.

## §12.6.4.8 — half a `shall` executed, and a refusal that can now become an ask

Table 211: with no `/Base`, partial URIs "shall be interpreted relative to the location of the
document itself". The row said the crate "does not know" that location — true of `pdf-model` and of
`viewer-core`, whose rule 2 keeps paths out of the core, and false of all four windows, each of
which opened the file by path. `viewer_host::policy::resolve_uri` carries it out against the `file`
URL of that path, percent-encoded because a path is not a URI: a `#` in a directory name would else
become the *base's* fragment and section 5.3 would merge from the wrong place. `may_open_uri` and
`uri_note` replace the `println!` each window wrote for itself — principle 3's "refusal that cannot
become an 'ask'" exactly. `partial` for one act: the fetch.

**And two more answers.** §12.6.4.4 narrows to Table 204's `/F`: a `/R /P` from a root document and
a `/R /C` naming a child the file does not embed are errors in the *file*, so refusing each by name
is correct rather than owed. And all ten of Table 197's triggers are dispatched — `/E` `/X` `/D`
`/U` from the pointer, `/Fo` `/Bl` from a press and a page turn, and `/PO` `/PC` `/PV` `/PI` from
`Viewer::page_events`.
