# 1260 — A push-button is its own appearance, and a launch opens a PDF

The HOST-UI round, batch forty. ADRs 1357, 1358. §12.6.4.6 `reported` → `departed`; its heads
§12.6.4 and §12.6 `partial` → `implemented` (ADR 1035's rule). §12.5.5 and §12.7.5.2 unchanged.

## Push-buttons (§6.3.2.2, §12.7.5.2.2)

- The page's delegated widgets are the fields a host places a control over: `Control::is_delegable`
  and `ControlKind::is_placed`, one set. Push-button, signature and unstated keep their `/AP`; GTK's controls signature skips the unplaced.
- GTK and Qt place no toolkit button. The click is `Command::Pointer`'s; Space or Enter at §12.5.1's
  focus is `viewer_host::pressed` (`Key::Enter` added, three windows). GTK hands its keyboard to
  the walk after each Tab; Qt's page declines Qt's focus chain so Tab walks `/Tabs`.

## Launch (§12.6.4.6, handed over from 1265)

- `/F` gives `Action::Launch`, asked under `Purpose::LaunchDocument` (wire 5, C ABI 5) at the
  remote-documents level. §7.5.2's header decides after arrival: a PDF opens, beside where
  `/NewWindow` says; anything else is named and withheld by the sandbox.

## Driven under Xvfb (`scratchpad/r1260/`)

- `form.pdf`: before, GTK and Qt showed buttons "go", "mk", "press"; after, the producer's art, the
  `/MK`-constructed caption, and after the import the named page from `library.pdf`. Click, Tab +
  Enter and Tab Tab + Space each fired `/A` in both windows; a text field still takes typing.
- AT-SPI (GTK, `form-tagged.pdf`): the `Form` element is a push button and `DoAction` pressed it.
- `launch.pdf`: the card, then `file1.pdf` opened beside in a tab; `tool.sh` named and refused.
  `issue17846.pdf`'s `/UF` is a path into subdirectories and the path rule refuses it by name.

## Tests

`viewer-host/tests/push_buttons.rs` (4), `viewer-host/tests/launch_documents.rs`,
`pdf-model/tests/delegated_widgets.rs::a_field_with_no_value_a_control_shows_keeps_its_appearance`,
four launch tests in `viewer-core/tests/remote_go_to.rs`; `actions.rs`, `headless.rs` and the
`action.rs` launch test changed with the reading.

## Left

- An untagged page's push-button is announced on AT-SPI by no window (ADR 1357 section 4).
- GTK sends `Moved` right after a press, which drops `/D` before release; this was already so.
- quorra's Space/Enter press was built and not driven.
