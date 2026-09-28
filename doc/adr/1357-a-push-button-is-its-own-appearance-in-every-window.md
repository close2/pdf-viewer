# 1357 — A push-button is its own appearance in every window

Session 1260. Status: **accepted** and built.
Context: `crates/pdf-model/src/form.rs` (`Control::is_delegable`, `delegated_widgets`),
`crates/viewer-host/src/form.rs` (`ControlKind::is_placed`, `Pressed`, `pressed`),
`crates/viewer-host/src/keys.rs` (`Key::Enter`), `crates/viewer-gtk/src/controls.rs`,
`crates/viewer-gtk/src/host.rs` (`follow_focus`, `press`), `crates/viewer-qt/src/host.rs`
(`placement`, `focused_control`, `press`), `crates/viewer-qt/cpp/window.cpp`
(`PageArea::focusNextPrevChild`, `MainWindow::eventFilter`, `keyPressEvent`),
`crates/viewer-ui/src/bin/quorra/window.rs`.
Amends: ADR 0245 (which widgets `WidgetAppearances::Delegated` takes off the page), ADR 0244 (the
native push button). Clauses: ISO 32000-2 §6.3.2.2, §12.5.1, §12.5.5, §12.7.5.2.2, Table 166, Table 227.

## 1. The finding

In `quorra-gtk` and `quorra-qt` a push-button was a toolkit button labelled with the field's name,
placed over a page drawn without the widget's appearance. Neither the producer's `/AP`, nor the
caption and icon Table 192's `/MK` constructs one from, nor an imported §12.7.8.3.3 named page
(ADR 1335) could be seen there. `quorra` drew all three. The same rule also took the appearance of a
§12.7.5.5 signature and of a field stating no `/FT` off the page, and no control replaced either.

## 2. The clause

§6.3.2.2: "A PDF processor shall also render the appropriate appearance stream for all annotations
(12.5.5, "Appearance streams") which have appearance streams designated for this purpose as indicated
by the annotation flags (see 12.5.3, "Annotation flags"), unless otherwise instructed." A host's
instruction is honest where the host draws the field itself. That holds for a field whose value a
person gives it: text, choice, check box, radio button. A push-button "responds immediately to user
input without retaining a permanent value" (§12.7.5.2.2), so its appearance is all it shows. A
toolkit button in its place shows marks the producer did not draw.

## 3. Decisions

- **The delegated set is the fields a host places a control over.** `Control::is_delegable` in the
  model and `ControlKind::is_placed` in the host are one set; `tests/push_buttons.rs` holds them
  together for every variant. Push-button, signature and unstated keep their appearance.
- **No host control for a push-button, with or without `/AP`.** Table 166 requires every widget to
  have an appearance dictionary. One with `/MK` only has its appearance constructed by the model, as
  in `quorra`. One with neither states nothing, and a toolkit button would invent its marks.
- **The click is the pointer's**, as on every page-drawn widget: `Command::Pointer` gives Table 197's
  triggers, `/D`'s down appearance and `/A`. `Clicked` is unchanged.
- **The key is `viewer_host::pressed`.** §12.5.1 names only the tab key, so pressing a widget is a
  choice: Space and Enter at §12.5.1's focus send `Command::Activate`; Table 227 bit 1 refuses by
  name. `Key::Enter` joins the table and means nothing on the page otherwise.
- **The keyboard follows the walk in both native windows.** GTK clears its focus, or gives it to the
  placed control, after a `Command::Focused`. Qt's page declines Qt's focus chain, so Tab reaches the
  key table, and Enter from a placed control is not passed on to the page.

## 4. Costs

- On an **untagged** page a push-button is announced on AT-SPI by no window. GTK's and Qt's own
  widget trees used to announce the toolkit button. §14.7's route announces it as a `Button` where a
  `Form` element names it, and `DoAction` presses it (measured in GTK). Publishing untagged widgets
  as nodes would be a change to `viewer-accessibility`'s untagged-page rule, argued on its own.
- A GTK press on a push-button is followed by a spurious `Moved`, which clears `/D` before release.
  That was so before this change.
