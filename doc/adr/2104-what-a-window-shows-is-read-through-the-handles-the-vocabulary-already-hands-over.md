# 2104 — What a window shows is read through the handles the vocabulary already hands over, and what changed is a record

Session 1634, batch ninety-nine's slot 4, RFC 0010 section 6's second N2 round, a host round.
Status: **accepted** and **built**. Context: ADRs 0246 (`viewer-qt`'s update flags and getters),
0247 (the C ABI, and why it refused callbacks), 0244 finding 5 (refresh after every drain), 2088
(`quorra_window`, its first round).

## 1. What was missing

After ADR 2088 a C caller could run the window's loop and draw its pictures, and could not build
anything around them: the panels' rows, the controls placed over the page, the find bar's matches,
Annex O's highlight, the selection and §12.5.1's focus were each readable only through a
`quorra_viewer *`, which a window does not lend (ADR 2088 section 3), and nothing a person does to a
row or a control could reach the window's viewer.

## 2. Decisions

1. **What changed is a record; what it changed to is read.** `viewer-qt` answers this question with
   a flag beside a getter, and the window has the same shape: `PANELS` already said "the rows
   again", and three more kinds say "again" — `CONTROLS` (10), the set of controls over the page
   changed, so the widgets are rebuilt rather than moved and a control being typed into keeps the
   keyboard while it stays; `POPUPS` (11); `STRIP` (12). The marks drawn over the frames need no
   kind of their own: a `REPAINT` is owed after every drain, as Qt's `chrome` is raised beside its
   `frame` on every refresh, and a repaint draws the frames and then the marks.
2. **The two sets are compared, not inferred**: after every drain the window queries the fields and
   the popups and owes a record only where the set differs from the one last told — the field's
   qualified name and its widget for a control, as Qt's `Placement::key` names one (ADR 0244
   finding 5: `Event::Damage` is a bound, not a list).
3. **The rows cross as the handles the vocabulary layer already hands over** — `quorra_outline`,
   `quorra_panel`, `quorra_fields`, `quorra_popups`, `quorra_quads`, `quorra_matches`,
   `quorra_thumbnail` — read and freed by their own accessors. Fourteen `Session` readers moved,
   line for line, into `viewer_ffi::Reads`, a borrow of a viewer that both handles lend
   (`Session::reads`, `Window::reads`), so the two families of entry points answer from one body.
4. **A commit is addressed as a toolkit window addresses it**: a field by its qualified name (and
   `QUORRA_NO_ANSWER` where the page shown has no widget of it), a check box or radio button by its
   widget through `viewer_host::toggling`, a text through `viewer_host::form::edit_of` so §12.7.5.3's
   file-select field takes its file, a layer's switch with Table 99's lock said as Qt says it, an
   attachment by its key, and a row's or a push-button's object activated. A refusal the
   document's flags state is a `SAID` record, not a status.
5. **The panels themselves are counted**, which RFC 0010 section 5 listed as owed: `QUORRA_TAB_*`
   is `viewer_host::Tab::ALL`'s order, `QUORRA_TAB_COUNT` against `quorra_tab_count()` says at
   startup that a panel was added, and `quorra_tab_label` hands over the one wording every window
   shows, so a C# window neither numbers its tabs nor words them.
6. Twenty-four entry points, declared in `quorra.h` and `Native.Window.cs`; `drive_a_window.c`
   reads the tabs, the rows, the marks before and after the find bar closes, activates a row,
   refuses a toggle on a text field and an edit to a field the page lacks, and commits typed text.

## 3. Considered and refused

- **A record carrying the rows themselves**, as the brief's hypothesis priced it (eight kinds).
  A record is read once and freed; the rows are read whenever the caller's model asks, which is
  why Qt keeps getters, and a second copy of every panel accessor on `quorra_owed` would be a
  second ABI for one answer.
- **Lending a `quorra_viewer *`**: refused in ADR 2088 section 3, for the same ownership reason.

## 4. Costs, and what is not built

`abi.rs` gains twenty-four shims and one safe helper. Still owed by RFC 0010 section 3.2a: the
archive chooser, a script's dialogue and a print grant, and the reader's policy flags, which wait
for RFC 0010 section 5's shared parser.
