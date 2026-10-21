# 2153 — The WinUI window draws the marks in Windows' colours and places a control over each delegated widget

Session 1658, RFC 0010 section 6's step N3a, second part. Status: **accepted**; written in
`hosts/windows/Quorra.App/` (`PageArea.cs`, `Controls.cs`, `QuorraWindow.cs`, `MainWindow.xaml.cs`)
and **compiled by CI only** (ADR 2136): this machine has no Windows and no .NET SDK. Context: ADRs
1604, 2088, 2104, 2136, 2137, 2152; `doc/ui-boundary.md` (chrome crosses as shapes).

## 1. What was missing, and the part of it that was a loss

The first step drew the pictures and nothing over them. Two things followed. The marks — the
selection, the find bar's occurrences, Annex O's highlight, §12.5.1's focus — were not shown. And,
found reading `Window::open` this round, **the window opens with `WidgetAppearances::Delegated`**:
the page is interpreted without the widgets of the fields a host is handed a control for
(`pdf_model::form::delegated_widgets`), so in the first step a text field's value, a check box's
state and a choice's selection were on no layer at all. Trap 5's failure, inside a partly built
window: nothing said it, because the record that says "build the controls" was logged and dropped.

## 2. Decisions

1. **Three layers in the page area**, in drawing order: the pictures, the marks (not hit-testable,
   so a press reaches the page), the controls. A key or a press whose source is inside the control
   layer is the control's and is not passed to `quorra_window` (`PageArea.IsPage`).
2. **The marks are read on every `REPAINT`**, after the pictures, through the four handles quorra.h
   names, and drawn as polygons in **Windows' own colours**: `UISettings.UIElementColor(Highlight)`
   — the highlight brush `doc/ui-boundary.md` names — at `viewer-qt`'s three weights (Annex O 0.18,
   other occurrences 0.12, the selection 0.35), and §12.5.1's ring a two-pixel stroke in the accent
   (`UIColorType.Accent`). The weights are `viewer-qt`'s choice, so the windows draw the three kinds
   alike; the standard states no look for any of them.
3. **`CONTROLS` rebuilds one platform control per widget** of `quorra_window_fields_read`: a
   `TextBox` (multi-line by Table 231 bit 13, read-only by Table 227 bit 1, `/MaxLen` as its length),
   a `PasswordBox` for bit 14, a `CheckBox`, a `RadioButton` each in a group of its own (§12.7.5.2.4's
   set is a field's, and Table 229 bit 15 is the library's to apply), a `ComboBox` editable by Table
   233 bit 19, and a `ListBox` multiple by bit 22. A push-button, a signature and a field stating no
   `/FT` get none: their widgets keep their appearance on the page.
4. **What a person does is committed as the toolkit windows commit it**: typing as
   `quorra_window_set_field_text`, Enter or the keyboard leaving a single-line or password field as
   `quorra_window_commit_field`, a click as `quorra_window_toggle_field` towards the opposite of the
   field's state, a choice as `quorra_window_set_field_options`, an editable combo box's submitted
   text as both. After each, the controls are read again and show what the field took (ADR 1604:
   the value while a control holds the keyboard, Table 199's `/F` display otherwise; a password
   control is never written to).
5. **The controls move with the page**: on a `REPAINT` whose pictures moved or changed size, the
   fields are read again and each control placed over its widget's rectangle, sized by the
   rectangle and not by its style's minimum. A repaint that moved nothing reads nothing.
6. **Control and the wheel** is `quorra_window_zoom_wheel` in lines, a notch being 120 of WinUI's
   `MouseWheelDelta` (ADR 2152).

ADR 2137's table is unchanged: none of its five declined records is answered by this step.
`CONTROLS` leaves the list of records named in the log; the panels, the popups, the strip, the
chrome, a placement and the system sound stay on it.

## 3. Considered and refused

- **Moving the controls on every repaint**: a drag of the selection owes a repaint per pointer move,
  and each read of the fields walks §12.7.4.1's tree, which quorra.h says not to do on a click.
- **One XAML radio group per field**: XAML groups by parent or by name and enforces exclusivity
  itself, which would take Table 229 bit 15's decision away from the library.
- **A `writing` flag alone around the writes**: WinUI raises `TextChanged` after the call that set
  the text, so a text control is told apart from this window's own write by its text; the flag
  stays for the selection events, which XAML raises inside the write.

## 4. What is not built

Table 231 bit 25's comb cells, bit 21's file chooser (RFC 0010 step N4), bit 26's rich text, the
measured fit of a control whose style's minimum exceeds its rectangle (`viewer_host::ControlFit`,
which `viewer-qt` reports), §12.9's traced path, and the popups. The pixels are still copied on every
`REPAINT`, a drag included; the ABI says "again" without saying whether the frames changed.
