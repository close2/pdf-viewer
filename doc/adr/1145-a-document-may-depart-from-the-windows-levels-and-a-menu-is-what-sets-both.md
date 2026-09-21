# 1145 — A document may depart from the window's levels, and a menu is what sets both

Session 1155. Status: **accepted**.
Context: `crates/viewer-core/src/command.rs` (`RestrictionScope`, `RestrictionOverride`,
`RestrictionPolicy::under`), `crates/viewer-core/src/open.rs`, `crates/viewer-core/src/viewer.rs`,
`crates/viewer-host/src/restriction.rs` (new), `crates/viewer-host/src/keys.rs`,
`crates/viewer-gtk/src/host.rs`, `crates/viewer-qt/{src/host.rs,cpp/window.cpp}`,
`crates/viewer-ui/src/chrome.rs`, `crates/viewer-confined/src/protocol.rs`, `crates/viewer-ffi`.
Builds: ADR 1144 (a level per operation), ADR 0814 (the four levels and the event), ADR 0604 (a
host-supplied value is the reader's), ADR 0526 (one key table, three toolkits), `doc/todo/38`.
Clauses: ISO 32000-2 §7.6.4.2 (Table 22), §12.2 (Table 147), §12.8.2.2.

## 1. The scope a viewer-wide policy could not express

`Command::Restrict` carried a policy that applies to "every open document and to every one opened
afterwards", which is right: a host-supplied value is a statement about the **reader** (ADR 0604).
That is also exactly what it cannot say — *for this document, ask before copying*. A level set to
catch one suspicious file catches every file the window opens afterwards, and a reader who then set
it back has changed the policy for documents they were not thinking about.

So the command carries a scope: `RestrictionScope::Window(RestrictionPolicy)` is what every document
inherits, and `RestrictionScope::Document(RestrictionOverride)` is what the focused document departs
from it in. **A variant's shape changed rather than a message added** — `doc/ui-boundary.md`'s own
preference, and every consumer failed to compile.

**The override is one `Option<RestrictionLevel>` per operation, not a second whole policy**, and that
is the decision rather than a convenience. A menu sets one operation at a time, so an override
carrying all six levels would freeze the other five at whatever the window held when the first was
chosen: a reader who asked to be questioned before copying *this* document would have detached its
annotating from the window's as well, and nothing on the screen would say so. `None` is not a fifth
level — `CLAUDE.md` names four — it is the absence of one, which is what makes *use the window's
level* an entry a menu can offer. `RestrictionPolicy::under` is the whole of the layering, asked in
`Viewer::standing` so that no consumer composes the two for itself.

**It lives on `Open`, which is what makes it end when the document does.** ADR 0604's rule read the
other way round: the window keeps its policy for its whole life, and a departure keeps the
document's. `crates/viewer-core/tests/restriction_levels.rs` pins both halves — window `On`,
document `Ask`, the copy asks; the next document opened is refused again.

## 2. The menu is one decision and three bindings

48 entries about wording, order, what a tick means and what each one sends, written three times,
would be three answers to every one of those questions — `viewer_host::keys`' argument for the third
time. `viewer_host::restriction` holds the rows, the two scopes, the state a menu edits (because
`Command::Restrict` carries a policy and an entry sets an operation), the question the *ask* level
puts and the sentence a decline gets. GTK nests it into a `gio::Menu` behind
`set_create_popup_func`, Qt into a `QMenuBar` refilled on `aboutToShow`, and `viewer-ui` draws the
same rows flat on a card. **Every one of them builds it when it is opened**: nothing on the launch
path, and a menu built once would tick the levels the program launched with.

`Print` and `Assemble` are shown and settable with `INERT` beside them, which is ADR 1144's "carried
for the day those operations exist" made visible rather than a switch that does nothing.

## 3. §12.2's `/HideMenubar` is read, answered and not obeyed

Table 147's entry is "[a] flag specifying whether to hide the interactive PDF processor's menu bar
when the document is active", and the only menu bar these windows have is the one holding the
reader's levels. Obeying it there would let a file take away the control over what that file is
allowed to do — against principle 3's "it shall always be possible to turn them off". So it is
answered in words (`NOT_THE_DOCUMENTS_TO_HIDE`, naming clause, entry and reason) rather than in
silence; `/HideToolbar` and `/HideWindowUI` are obeyed as before, and Table 29's full screen still
takes the bar, because that sentence is the reader asking rather than the document.

## 4. Consequences

- The wire spells a scope byte and six level bytes, `NO_DEPARTURE` being the value that is not a
  level; the C ABI gains `quorra_restrict_document_operation` and `QUORRA_RESTRICT_INHERIT`.
  `QUORRA_ABI_VERSION` does not move — no struct crosses by value.
- `viewer_host::unanswerable` keeps one face: `quorra-confined`, which performs no restricted
  operation at all, so the event cannot reach it.
- `Key::R` and `WindowAct::Restrictions` are the menu's key in all three windows.
- `launch_path` and `raster_golden` are unmoved: no menu is built before a page and no pixel of a
  page is decided here.
