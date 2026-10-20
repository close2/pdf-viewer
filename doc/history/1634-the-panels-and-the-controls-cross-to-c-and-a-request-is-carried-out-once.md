# 1634 — The panels and the controls cross to C, and a request is carried out once

Session 1634, slot 4 of batch ninety-nine, a "host" round; 2026-10-20. RFC 0010 section 6's N2, its
second round; ADRs 2104 and 2105. No Q file. No ledger row moved; five rows' `code` lists re-pointed
from the deleted host files to `native_window/requests.rs`.

**Built.** ADR 2104: three owed kinds (`CONTROLS`, `POPUPS`, `STRIP`; 13), the controls and popups
compared after every drain; fourteen `Session` readers moved line for line into `viewer_ffi::Reads`
(199 lines, multiset equal), which both handles lend; 22 window entry points reading the panels,
pages, form, popups and marks and committing a row, a field, a toggle through `viewer_host::toggling`,
a layer and an extraction; `QUORRA_TAB_*` with `quorra_tab_count` and `quorra_tab_label` (RFC
section 5): 300 to 324 entry points, the C# for all. `drive_a_window.c` reads the tabs, the rows and
the marks, activates a row and commits typed text. ADR 2105: `native_window::Asked` and
`native_window/requests.rs` carry out what a document asks; the three windows keep
`put_the_question` and `question_waiting`.

**The premise and the hypothesis.** The premise held: three copies, line for line, GTK's spread
through closures. The hypothesis did not, twice: three record kinds, not eight — the rows are read
with the vocabulary's handles, as Qt reads getters beside its flags, and the marks ride `REPAINT`;
and the lift is one enum and eleven functions, 484 lines with their clauses, against about 1 500
removed from the three hosts, not one function of two hundred.

**Calibrated.** `QUORRA_OWED_STRIP` planted as 11 in the C#, `QUORRA_TAB_FILES` as 4 in the header
and `quorra_window_toggle_field`'s `ushort` as `uint`: `header_and_interop_agree` named all three,
reversed, `cmp` identical. Page 3's three "Black Point" and the 14 outline rows match `mutool`.

**Unfinished.** The archive chooser, a script's dialogue and a print grant are still declined aloud;
the reader's policy flags wait for RFC 0010 section 5's shared parser, not a morning's work: four
binaries parse their own words. No `dotnet` here: CI compiles the C#.

**Gates.** `rustfmt --check` on every touched file and `cargo fmt --check -p` on viewer-host,
viewer-ffi, viewer-gtk, viewer-qt: 0. `RUSTFLAGS="-D warnings" cargo clippy` on the four
`--all-targets`: 0 (a sibling's render-cpu mid-edit failed two earlier runs). `cargo nextest run`
on the four: 0, 385 of 385; viewer-ffi after the tabs: 0, 53 of 53, both C programs (`-Werror`),
`header_and_interop_agree` 4 of 4 and the two count tests among them. `cargo test -p conformance
--no-fail-fast`: 0. `tools/state.sh --round 1634 hosts`: 0, Command 45 of 45, Query 35 of 36, 324
entry points. The drive, release `quorra-gtk` and `quorra-qt` copied to scratch: four request steps
46 works, 0 wrong; `drive` 75 works, 2 wrong (GTK's 20 and 24, its controls undrawn in the shot), 3
not offered; the batch's start build 0 wrong there, and this build again 38 works, 0 wrong. Rustdoc:
no warning in a line written here. `docs` not owed. Duration: 4500 s.
