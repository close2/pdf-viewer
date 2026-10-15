# 1559 — The window converts to PDF/A under a profile a person names

Slot 2 of batch eighty-nine, 2026-10-15, a host round. ADR 1954; no Q file.

**The premise held** (no `Archive` in `command.rs`; the CLI converts under `--profile`; Command 36
of 44 reach the ABI). **The hypothesis held in part**: print's shape carries the request out, but
the report cannot come back through the core — `pdf-transform` depends on `viewer-core` — so the
grant carries the file and the report comes from the host's runner. The profiles lived in the binary.

**Built.** `Command::Archive(Archiving { target, profile })` asks Table 22's bit 11 at the reader's
level (all four levels tested on `bug1815476.pdf`, `/P −1084`, bit 11 clear) and `Event::Archiving`
carries the opened `FileBytes`. `pdf_transform::profiles` holds `SHIPPED` (moved out of the binary,
which now reads it) and `Contribution`, the one folding of a configuration into a plan, used by the
binary's `--config`/`--profile` too. `viewer_host::archiving` is the runner for all hosts: profile,
plan, passes through `executor::execute`, report lines, the file beside the document as
`<name>.pdfa-2b.pdf`, on an `archive` thread collected on the windows' answer timer. `--archive
<target>:<profile>` is a reader word; Shift+A asks (unshifted `A` still selects all). The confined
window's wire carries neither message and its Shift+A says it converts nothing. The C ABI gained
seven entry points (`quorra_archive`, `quorra_archived{,_copy,_line,_json}`,
`quorra_archive_profile{s,}`) and `QUORRA_EVENT_ARCHIVING` 29; the C test program converts. The
JSON conversion report has `profile`, set by the face that read the configuration.

**The window's file is the CLI's**: `quorra-transform archive … --to 2b --profile as-if-printed` on
the drive's one-rectangle fixture writes the same bytes (`cmp`), and veraPDF 1.30.3 calls all three
windows' outputs `isCompliant="true"`. The drive's own fixture is refused by name
(`fonts/font-programs-embedded`: a standard 14 font with no descriptor) and each window says so.

**Gates.** rustfmt clean on every touched file. `clippy --all-targets -- -D warnings` on
pdf-transform, viewer-core, viewer-host, viewer-gtk, viewer-qt, viewer-ui, viewer-ffi,
viewer-confined: exit 0 (both toolkit `react` matches now `#[expect(too_many_lines)]` with a reason:
slot 5's `Media` arm and this one took them past 100). `nextest` on seven: 1 439 passed, 0 failed;
viewer-confined 767 of 768 — `the_two_deferred_producers_reach_the_raster_arm_by_name`, an in-process
interpretation test this round did not touch. `cargo test -p conformance`: 410 passed, 2 failed
(slot 5's record). `tools/state.sh docs`: exit 0. `tools/state.sh hosts`: Command 37 of 45
reach the ABI (the gap stays 8). Drive (dev binaries, `:159`): step 88 in four windows, 8 works,
0 wrong; `drive`+`reader_words` in the three, 125 works, 0 wrong; `confined`+`reader_words`, 5
works, 0 wrong.

**Owed**: a menu or card choosing target and profile after launch; a password-protected document
converts only with an empty user password (the core keeps none).
