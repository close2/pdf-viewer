# 1642 — A window's settings cross once, and its three dialogues are records

Session 1642, slot 4 of batch one hundred, a "host" round; 2026-10-20. RFC 0010 section 5's policy
flags and section 3.2a's three dialogues; ADRs 2120 and 2121. No Q file. No ledger row moved.

**Built.** ADR 2120: `viewer_host::Settings::take` reads the seven settings words for all four
binaries (about 200 lines of their parsing gone) and `Settings::set` puts six `Policy`s on one
`Level` scale; `quorra-confined` refuses by name the four it has no level for. `quorra_settings_*`
(six) and `quorra_window_open_under`, with `QUORRA_POLICY_*` and `QUORRA_LEVEL_*`. ADR 2121:
`QUORRA_OWED_ARCHIVE`, `SCRIPT`, `SCRIPT_WITHDRAWN`, `PRINT` (17 kinds), answered by
`quorra_window_archive`, `_answer_script`, `_print_finish`, read by four `quorra_owed_*` and
`quorra_archive_targets`/`_target`; the window carries out the print key as `viewer-qt` does. 324
to 340 entry points, the C# for all. `drive_a_window.c` opens the form under stated levels (its
submission refused unasked), closes the chooser, ends a print, and answers a validation's
`app.alert` through the real worker, a second withdrawn after a 2 s wait.

**The premise and the hypothesis.** The premise held but for `--scripts=`, which does not exist:
`--scripts` takes the next word and stays `ReaderWords`'. The hypothesis held for the parsers (the
same seven arms) and not for the requests: four kinds, not three, since a withdrawn question closes
its dialogue — and the print grant was unreachable from C, the key handed back with no entry point.

**Calibrated.** `QUORRA_LEVEL_PROCEED` as 4 and a `byte*` as `uint*` in the C#, `QUORRA_OWED_PRINT`
as 15 in the header: the agreement tests named all three; reversed, `cmp` identical.

**Unfinished.** The reader's other words (anchors, target documents, audience) for a window; the
print key's decision is in three windows (ADR 2121); `RestrictKind::On`'s doc says "The default",
which ADR 1144 made `off` — found, not fixed.

**Gates.** `rustfmt --check` on every touched file, `cargo fmt --check -p` on the five crates: 0.
`RUSTFLAGS="-D warnings" cargo clippy --all-targets` on the five: 0 in `tools/batch.sh export 1642
--as strict` (HEAD plus this round's 22 paths); the tree's failed on siblings' crates mid-edit.
`cargo nextest run` on the five in the export: 0, 526 of 526; in the tree viewer-ffi 55 of 55, both
C programs (`-Werror`), `header_and_interop_agree` 4 of 4. `cargo test -p conformance`: 101,
`round_numbers` alone, on slot 2's untracked `examples/r1640_isolate.rs`. `tools/state.sh --round
1642 hosts`: 0, Command 45 of 45, Query 35 of 36, 340 entry points. Rustdoc: no warning in a line
written here. The drive at `:142`, release binaries in scratch: `link_levels`, `submission_levels`,
`scripts_levels`, `reader_words` in four windows 89 works, 0 wrong; `drive`, `script_asks`,
`script_withdrawn` 125 works, 1 wrong (GTK's `25-find-arabic`, the shot's needle typed without its
ya), 5 not offered; GTK's `drive` again 38 works, 0 wrong. `tools/state.sh --round 1642 docs` in
the export (HEAD's six crates, viewer-ui among them): 0. Duration: 4350 s.
