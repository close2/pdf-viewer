# 1660 — The reader's other words reach a window, and the print key is decided once

Session 1660, slot 6 of batch one hundred and two, a "host" round; 2026-10-21. RFC 0010 section 5's
reader's words; ADRs 2156 and 2157. No Q file. No ledger row moved.

**Built.** ADR 2156: `viewer_host::policy::Words` is the one reading of a command line's words —
`ReaderWords::take`, then `Settings::take`, answering `Taken::Reader` or `Taken::Setting` — with the
scripts level in one place; the four binaries call it (`quorra-gtk` and `quorra-qt` no longer copy
the level across; `quorra-confined` still refuses its four uncarried words by name). On the handle:
`quorra_settings_reader` with `QUORRA_READER_*` (6), `_accept_unknown_revocation`, `_archive`,
`_take_next`; `quorra_settings_take` reads every word. `quorra_window_open_under` sends the reader's
commands on the document thread and says each refused file as a `SAID` record; the archive key
starts on the stated conversion. 341 to 345 entry points with slot 4's; the C# for all. ADR 2157:
`viewer_host::print_key(Printer)` beside `decide`, `Printer::Preview { standing }` or
`Printer::Dialogue(sheet)`; `quorra`, `viewer-qt`, `quorra_window` and `viewer-gtk` call it.

**The premise and the hypothesis.** The premise held for the ABI, not for the binaries:
`ReaderWords::take` was already the one function the four call; what was copied was the second call
beside it and the scripts level carried between the halves. The hypothesis did not hold as priced:
one setter with six constants rather than one per word (ADR 2120 section 3), and three more entry
points. The print decision held: four call sites, one sentence.

**Unfinished.** The audience reaching a C window is shown by unit equality and by the directory's
refusals, not by a layer drawn for a named reader; the drive presses print once, so the key's end
is judged by the C program and the unit test. The opening sequence (reader commands, refusals,
opening commands, anticipate) is written in GTK, Qt and the C window — found, not moved.

**Gates.** `rustfmt --check` on every touched file: 0. In `tools/batch.sh export 1660 --as strict`
(HEAD, this round's paths, `viewer-ffi` and the interop layer whole with slot 4's edits): `cargo fmt
--check` and `RUSTFLAGS="-D warnings" cargo clippy --all-targets` on `viewer-host`, `viewer-ffi`,
`viewer-gtk`, `viewer-qt`, `viewer-ui` 0 (the tree's failed on `raster-gpu` mid-edit); `cargo nextest
run` on the five 0, 537 of 537; `tools/state.sh --round 1660 docs` 0; no rustdoc warning in a line
written here. The C program's new line planted wrong ("starts on 1 and 1") failed naming it; restored,
`cmp` identical. In the tree: `viewer-ffi` 58 of 58, both C programs (`-Werror`) and
`header_and_interop_agree` among them; `cargo test -p conformance` 0 (101 earlier, `round_numbers`
on slots 1's and 3's scratch examples, since removed); `tools/state.sh --round 1660 hosts` 0,
Command 45 of 45, Query 35 of 36, 345 entry points. The drive at `:1660`, HEAD's release binaries
before and the export's after: `drive` in three windows 116 works, 0 wrong, 4 not offered, both;
`reader_words`, `link_levels`, `submission_levels`, `scripts_levels` in four windows 89 works, 0
wrong, both; every verdict identical. Duration: 6100 s.
