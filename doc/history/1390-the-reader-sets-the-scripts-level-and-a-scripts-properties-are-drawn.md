# 1390 — The reader sets the scripts level, and what a script sets on a field is drawn

**Contract.** RFC 0008 section 11 item 3 (e): the `Scripts` level in `viewer_host::policy`, every
window's menu and `--scripts off|ask|warn|on`, `quorra-confined` pinned to `off`, the host supplying
`pdf-script-worker` as the runner; `Q286` for the owner's amendment; the properties ADR 1603 kept
and did not draw, drawn and saved; the whole drive behind the lock.

**Built (ADR 1616).** `viewer_host::Scripts` (`off` by default, A193), read by `ReaderWords` and by a
third machine act in all three menus; `Command::Scripts` hands the viewer a maker of runners, one per
document; *ask* holds a `Withheld` runner that puts `Event::AskingToRunScripts` once, at the first
script, with its first line, and a `yes` reruns the open sequence and `/CO`. Every script's sentence
now reaches the window after every command. Table 200's five moments (round 1389's ADR 1614) are
called at the close, the save and the print, and `setFocus` (ADR 1615) is carried out. The host
crates take `pdf-script-worker` without `engine`; round 1394's `install_featured` installs the
worker beside the windows. `quorra`'s question card now takes keys before a field.

**Built (ADR 1617).** `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `alignment`,
`charLimit` are drawn: `appearance::scripted_entries` reads each as the entry §12.5.6.19's Table 191
and §12.7.4.3's Table 228 draw a widget from, a scripted widget is constructed anew (check boxes and
radios excepted, their states being the file's), and a save writes `/MK` and `/BS` on the widget and
`/DA`, `/Q`, `/MaxLen`, `/Ff` (`required`) on the field. `AccessibilityNode::value` carries the
displayed value; the editable combo box commits like a text field in GTK and Qt.

**Premises that did not hold.** `/MK` is Table 192 (Table 191's row), not Table 189.

**Smallest edits in files other rounds own**, named: `crates/pdf-model/src/view/scripts.rs`
(`recalculate_with_runner`, the per-widget `drawn` record and its arm, `scripted`),
`crates/pdf-model/src/view.rs` (`AnnotationView::scripted`, `write_scripted`, `named_field`),
`crates/pdf-model/src/annotation.rs` (`stored_set_aside`), root `Cargo.toml` (one dependency).

**Measured.** The new steps: 16 verdicts `works`, about 6.5 s each. The whole drive behind the lock:
158 works / 1 wrong / 3 not offered in 543 s — the wrong `38-located` in `quorra`, its `m` taken on
and off before the press, `works` on both reruns. `ps -u AI -o nlwp=` summed to 187 at the drive.

**Gates.** `rustfmt --check` clean on every file touched. Clippy pedantic over the eight crates:
exit 101, only `viewer-ui/tests/launch_path.rs` (a sibling's, in flight). `cargo nextest run` over
the same eight: exit 0, 2738 passed. `cargo test -p conformance`: 377 passed, the two failing are
the records tests on round 1394's record.

**Question.** `doc/questions/Q286`: the exclusion's sentence, §12.6.4.17 to `implemented` (A100).
