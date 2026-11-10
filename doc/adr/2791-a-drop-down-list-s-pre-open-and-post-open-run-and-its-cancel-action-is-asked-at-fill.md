# 2791 — A drop-down list's `preOpen` and `postOpen` run, and its `cancelAction` is asked at the reader's level for `fill`

Session 1990, a ledger round (K.2). Status: implemented. Builds ADR 2784's construction, with two
corrections recorded in ADR 2792; extends ADR 2763 (a save a `preSave` script declines is asked at
the reader's level) and ADR 2777 (a print alike) to a third operation; keeps ADR 1616's confined
window at `off`.

## Context

XFA 3.3 page 392: `preOpen` and `postOpen` "apply only to drop-down choice lists, or more
specifically choice lists for which open="userControl" or open="onEntry"". `preOpen` fires when a
person clicks the symbol that drops the list down or presses the key sequence that does, and "does
not fire in response to the user pressing the Enter key"; `postOpen` "fires after the combo box is
displayed. It also fires when the display of the combo box fails, to ensure that any cleanup code
is executed". Page 400 pairs them as a pre- and post-event (page 1340's list says so too, "preOpen
is matched by postOpen"), page 402 has an action clear `cancelAction` as it begins, and page 399 has
the flag "cancel whatever action fired the event". Page 398's table of what a primary event sets
names neither event. Page 653: `open` is `userControl` (listed first, the default), `onEntry`,
`always` or `multiSelect`, the last two displaying the list whenever the field is visible.

## Decision

1. **`Stimulus::PreOpen` and `Stimulus::PostOpen`** fire the field's own events (`Bound::Own`) on a
   field whose `ui` holds a `choiceList` whose `open` is absent, `userControl` or `onEntry`
   (`Container::drops_down`); any other field runs nothing and the report says why. Each runs under
   an event whose `cancelAction` is clear and which sets no primary property, so `$event.name`
   under either is refused by page 398's rule. `Activation::cancelled` carries a `preOpen` script's
   flag. `ViewState::activate_form_before_opening` and `activate_form_after_opening` are the view's
   two calls.
2. **The opening is a grant**: `Command::OpeningList { field }` runs `preOpen` and answers with
   `Event::ListOpens { document, field }`; a host displays the list then and not at the press, and
   sends `Command::OpenedList { field }` once it is shown or could not be, which runs `postOpen`.
   ADR 2784's `shown: bool` is not carried: page 392 fires the same `postOpen` either way.
3. **A `preOpen` script's `cancelAction` is the document declining its reader's filling-in**, so it
   is `Restriction::OpeningCancelled` against `Operation::FillInForm` (§7.6.4.1's "\[f]illing in
   forms"), asked once in `Viewer::opening_list` as ADR 2777 asks a print's: `off` (the default)
   grants and says nothing, `warn` grants and says why, `on` withholds and says why, `ask` holds the
   opening (`Held::OpeningCancelled`) and `Command::Answer` grants on a `yes`. An opening not
   granted fires the `postOpen` page 400's cleanup owes, and no `OpenedList` follows.
4. **The wire and the ABI**: command kinds 50 and 51 and event kind 29 on the confined protocol;
   `QUORRA_EVENT_LIST_OPENS` (30), `QUORRA_OWED_LIST_OPENS` (17), and four entry points —
   `quorra_opening_list`, `quorra_opened_list` on a viewer and `quorra_window_opening_list`,
   `quorra_window_opened_list` on a window — where ADR 2784 counted two, because the ABI has both
   surfaces and the WinUI host draws through the window's.

## Consequences

- `$host.openList()`, the third trigger page 392 names, is not built: page 392 says it "is described
  in the Adobe XML Scripting Object Reference", not in XFA 3.3, as `addItem()` is, and this
  program's `$host` has no such method; a script calling it is refused as any unknown method is.
  `Event::ListOpens` is the message it would use.
- No script here changes a field's options (ADR 2784 decision 2), so a `preOpen` script changes
  values and properties only; a host reads the options after the grant all the same.
