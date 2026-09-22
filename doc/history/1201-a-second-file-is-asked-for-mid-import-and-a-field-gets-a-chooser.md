# 1201 — A second file is asked for mid-import, and a file-select field gets a chooser

## What moved

**§12.7.8.3.2, `partial` → `departed`** and **§12.7.8.3.3, `partial` → `departed`** (ADR 1239).
ADR 1235 named the missing piece exactly and every factual claim in it held: `forms_data::carry`
and `named_page::page_as_form` both existed, and what was missing was the **hop**. `ViewState` now
holds `AwaitedPage`s, `file_awaited` names the next file, `supply_named_pages` applies every
reference into it and `decline_named_pages` words what went without; `viewer-core` raises
`Purpose::NamedPage` while §12.7.6.4's own import is being applied, which is §12.6.4.4's suspended
walk with a second question inside it. Table 249's `/APRef` becomes the button's appearance through
the entry a stated `/AP` fills; Table 252's `/TRef` becomes an `AppendedPage::Carried` — §7.8.2's
concatenation of the other page's own `/Contents`, §7.7.3.4's inherited `/Resources` copied, its
boxes and `/Rotate` — built by `Pages::detached`, so no second `pdf_syntax::Document` reaches the
interpreter. `/Annots` do not cross and are named. What each row now stands on: `/RV`'s XFA rich
text, and `/Rename`'s `true`.

**§12.6.4.7 stays `implemented`, and its note stops naming a capability.** The refusal read "a
thread in another file, which this reader has no filesystem to open" — ADR 1227's expired shape.
`ThreadJump` carries Table 209's `/F` as the same `TargetRoot` Table 203's is read into, and
`resume_threaded` reads the `/Threads` array, the title and the bead index in the document that
arrived. One refusal is left and the *table* states it: `/D` and `/B` by reference "shall be in the
current file", so a reference beside a `/F` names an object of the wrong file.

**§12.7.5.3, `partial` → `departed`** (ADR 1240). `viewer_host::policy::may_choose_file` existed in
`Edit::ChooseFile`'s doc comment and nowhere in the tree; it exists now, and `form::edit_of` asks it
instead of matching Table 231 bit 21 itself, so the window that offers a chooser and the code that
reads the result cannot disagree. `viewer-gtk` puts a `gtk4::FileDialog` behind an entry icon,
`viewer-qt` a `QFileDialog` behind a trailing `QAction` — inside the widget's own §12.5.2 rectangle,
because that rectangle is the document's (trap 19). `quorra` has no dialogue toolkit and takes the
pathname typed, which is what the clause says the field's text is. The row's residue is bit 26.

**The leftover ADR 1228 section 5 named**: `enter_imported` carries `separation_simulation` into a
reference XObject's own `ViewState` beside `magnification` and `purpose`, calibrated by removing the
line and watching `a_readers_separation_request_crosses_into_an_imported_page` fail.

## Left for somebody

`--remote-documents=` now decides three acts rather than one; whether the *ask* level should
distinguish "parse this PDF into my page" from "open it instead of mine" is `doc/todo/38`'s.
