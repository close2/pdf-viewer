# 1576 — `open_document` is the trait's, and `xfa` is not there

Slot 6 of batch ninety-one, 2026-10-19, a script refactoring round. ADRs 1988, 1989; no question.

**Premise.** It held: both hosts' `open_document` named `restrictions`, `widget_appearances` and
`separations`, and the reference's XFA page (fetched again at RFC 0008's pinned commit, SHA-256
`eccaeca0…`) says "The xfa object is undefined in an Acrobat form". **Hypothesis.** Both halves held;
the second against RFC 0008 §4.3's "never as `undefined`", which ADR 1989 §2 answers for one name.

**Built.** The three fields move into `WindowState`, built by `WindowState::new` from the window's
`WidgetAppearances` and `Settings`; `standing` and `opening_commands` move from both `host.rs` into
`native_window`; `NativeWindow::open_document` is provided, both hosts' copies and forwards gone (ADR
1988). The Annex K row of `surface::EXCLUDED` keeps `XFA`; `xfa` is in `surface::ABSENT` with its
reason, so `typeof xfa` answers `undefined` (ADR 1989). Tested: `ABSENT`'s names are not there
(planted back into `EXCLUDED`, the test fails); the reference's example stops at its first line's
`this.documentFileName` (`REFUSED`), and from its second statement prints the Acrobat form's line.

**Proof that nothing moved.** The move's token multiset over this round's `-U0` hunks alone (slot
4's three chooser hunks in the same files left out), comments set aside, `self.<field>` read as
`self.state.<field>`: GTK's net removal is the trait's net addition but for `pub`, `#[must_use]`,
`let commands = …; self.drain(commands)` for `self.pump(opening_commands(…))`, `WindowState::new`'s
two arguments, one `use` and the deleted forwards. Drives of HEAD's export and of HEAD plus this
patch: GTK 93 works, 0 wrong, 4 not offered; Qt 96 works, 0 wrong, 1 not offered; verdict columns
identical. 175 of 221 photographs pixel-identical; of the other 46, 31 differ in a status strip
naming the drive's output directory, a loopback port or an answer file, 5 in a caret's blink, 3 in a
window not up at the shot (two of them HEAD's), 5 in a loading bar's or "Reading the outline."'s
moment, GTK's 22 in a focus ring's six pixels, Qt's 30 in its machine face not loaded at 0.4 s
(ADR 1406; that launch's next shot draws the row). `tools/state.sh hosts`, both: 37/45, 36/36.

**Files.** `viewer-host`'s `native_window.rs`; `host.rs` and `host/{input,opening,react,requests,
restrictions,saving,tabs}.rs` in `viewer-gtk` and `viewer-qt`; `pdf-script`'s `surface.rs`,
`tests/fixtures.rs`; `doc/todo/70`; ADRs 1988, 1989.

**Gates.** rustfmt `--check` on the 19 Rust files clean. clippy `-D warnings --all-targets` on
`viewer-host`, `-gtk`, `-qt`, `pdf-script` with and without `engine`: exit 0. nextest over the four,
`engine` on: 505 passed, 4 skipped, exit 0. `cargo test -p conformance`: exit 0. In the export: t2
`pdf-script --features engine --test script_corpus` exit 0, 21 101 runs, every column at its
ceiling, no ratchet, no `xfa` among the names thrown; t2 `pdf-model --test script_corpus` exit 0,
356 held, 0 moved. rustdoc `-W rustdoc::all`: nothing on a line written here. Drives exit 0, held
350 s (GTK) and 325 s (Qt). About 6 100 s.
