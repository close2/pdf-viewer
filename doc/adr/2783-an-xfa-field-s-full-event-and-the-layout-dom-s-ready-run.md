# 2783 — An XFA field's `full` event and the Layout DOM's `ready` run

Session 1986, a ledger round (K.2). Status: implemented. Extends ADR 2606 (the Form DOM's `ready`
and the application's `docReady` at the open sequence's end) and ADR 2732 (a commit's `change`
reads what the window handed as `$event.fullText`); keeps ADR 1616's confined window at `off`,
where neither event runs a script.

## Context

Round 1983's record left four of XFA 3.3's events: a field's `full` (page 391), a drop-down choice
list's `preOpen` and `postOpen` (page 392), and the Layout DOM's `ready` (page 388), each thought to
need the window to report a moment it does not report. Read against the code, two of them do not.

- **`full`** "fires when the user has entered the maximum allowed amount of content into the field
  and tries to enter more content" (page 391), and page 406's second rule fires it "[w]hen the user
  types the last allowed character", `change` first and `full` second, for the same change of state.
  The window already hands `ViewState::set_field` the whole text a person typed, before the field's
  room cuts it (ADR 2732's `fullText`), and the field's room — Table 232's `/MaxLen` and Table 231's
  `DoNotScroll` — is what `Typed::of` already measures for Adobe's `event.fieldFull` (ADR 1626).
- **The Layout DOM's `ready`** fires "when the layout is complete but rendering has not yet begun"
  (page 388). A static form's layout is the pages the file holds — `CLAUDE.md`'s XFA entry and §K.1:
  the AcroForm of a conforming hybrid file *is* the form — so it is complete when the document opens,
  and the moment before rendering is the one `Stimulus::Ready` already marks for `docReady`.

So no host, no confined frame and no C ABI entry changes for either. Effort was the only obstacle.

## Decision

1. **`full` fires at the commit, after `change`, where the typing filled the field.** This reader
   raises a field's `change` once, at the commit, over the whole entry (ADR 2732); `full` follows it
   there, as page 406 orders the two. The field is full where more was offered than it takes (page
   391's "tries to enter more") or where it holds `/MaxLen` characters (page 406's "the last allowed
   character"); `/MaxLen` is the interactive form's statement of the template's `maxChars`, which
   §K.2 requires to be consistent with it. `xfa::Typing` carries the answer, computed by the view
   where the field's widgets are known (`view::scripts::fills`).
2. **`full`'s `$event` is page 399's row.** It sets `change`, `fullText`, `newContentType`,
   `newText`, `prevContentType`, `prevText` and `target`, and page 400 adds `selStart` and `selEnd`
   "[w]hen a change or full event occurs". The row does not list `name`, `keyDown`, `modifier` or
   `shift`, so a script that reads one is stopped with page 398's sentence.
3. **The Layout DOM's `ready` runs between the Form DOM's `ready` and `docReady`.** `$layout` and
   `xfa.layout` (page 91's table) are `Bound::Layout`. Page 388 puts `docReady` after the Form DOM's
   `ready` and both DOMs' `ready` before rendering, and does not order the layout's against
   `docReady`; **a choice**: the layout follows the merge, so its `ready` comes after the form's,
   and the application's event closes the sequence over a form whose DOMs are all built.
4. **It fires once.** Page 408 has it fire "whenever the layout engine determines that the layout
   needs updating", and says "it is very much implementation-dependent when exactly the layout
   processor runs". A static form's layout is changed by nothing a person or a script does here —
   a value changes a widget's appearance stream, not the page's placement — so it completes once.
5. **The Form DOM's `ready` no longer reaches the page background.** Page 388's note: when it fires
   "the layout process has not run, so page background has not yet been assigned to the form. This
   means that the form ready event cannot propagate to objects (such as fields and subforms) within
   page background. By contrast the layout ready event can propagate to all objects". So a container
   of a `pageSet` or `pageArea` hears the layout's `ready` and not the form's.

## Cost

One `bool` on `xfa::Typing` (a public struct: its two constructions in the tree and the fuzz target,
which passes no typing, are unchanged in kind), one `Bound` variant, one template walk per container
at the form's `ready`, and one more reading of a text field's dictionary per keystroke; the
`DoNotScroll` measure `set_field` makes is shared with the `full` test rather than made twice.
