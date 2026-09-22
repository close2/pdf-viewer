# 1195 — A file a document names gets a level, and a page for paper gets its own boxes

## What moved

**§12.6.4.3, `reported` → `implemented`** (ADR 1227). The refusal read "a destination in another
file, which this reader has no filesystem to open", and the filesystem has been there since
ADR 0244. `Action::GoToR` carries Table 203 whole through the same §7.11 reader Table 204's `/F`
uses, `Purpose::RemoteDocument` is a third value of a vocabulary that needed no new message, and
`RemoteGoTo::page_in` reads the destination in the document a host supplied — `/SD` through
§14.7.2's `/IDTree` and §12.3.2.3's algorithm on Table 203's own `should`, `/D` behind it as
§12.3.2.2's page number *there*. Which files a document may name is
`--remote-documents=refuse|ask|warn|open`, default `ask`, in three windows: a value of its own on
ADR 1155's division, `resolve_import`'s path rule refused at every level including `open`, and
`/NewWindow` obeyed as written on the clause: no `shall` in its three sentences.

**§12.2, `partial` → `departed`** (ADR 1227, on ADR 1145's departure). `Pages` carries two pairs of
§14.11.2 boundaries, `Page::print_box` and `Page::print_clip_box` stand beside `display_box` and
`clip_box`, and `Page::render_for_printing` selects in `viewer_core::open::page` under the stated
`Purpose::Print`. `/PrintScaling`'s dialogue-suppressed sentence was re-read: the C ABI *is* such a
path, and honours it by applying no page scaling, which is `None` exactly. What is left is
`/HideMenubar`, decided against.

**§10.8.3's control** (ADR 1228). `ViewState::separation_simulation` is the input the algorithm
needed, `Command::Separations` the tenth host-supplied policy value, `--separations=on|off` and
shifted `S` in three windows, one bit on the confined wire, `quorra_separations` for a C caller —
a preference and not one of the four levels (ADR 1189 section 2). Round 1196 consumes it.

## What it caught

- Six tests in `remote_go_to.rs`, one in `print_preferences.rs` that prints the two documents it
  shows, two in `separations.rs`, two in `host_mappings.rs`; the C ABI is 202 entry points now,
  with the reason above the constant.
- **A defect on the existing `/GoToE` path**: `quorra-confined` said a sentence about a file a
  document asked for and never sent `Command::Supply`, so the worker held the action for ever.

## What is left

No window opens a second document beside the first; ADR 1227 section 5 says what tabs would change.
`content/xobject.rs`'s `enter_imported` does not yet carry this answer into a reference
`XObject`'s document (ADR 1228 section 5).
