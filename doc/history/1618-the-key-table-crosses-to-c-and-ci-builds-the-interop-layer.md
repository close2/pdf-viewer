# 1618 — The key table crosses to C, and CI builds the interop layer

Session 1618, slot 4 of batch ninety-seven, a "host" round; 2026-10-20. RFC 0010 section 6's N1
and N0's owed CI job; ADRs 2072 and 2073. No Q file: no owner's word was needed. No ledger row moved.

**Built.** `crates/viewer-ffi/src/keyboard.rs`: `QUORRA_KEY_*` (33, with `QUORRA_KEY_COUNT`) and
`QUORRA_ACT_*` (22, with `QUORRA_ACT_COUNT`), each numbered by this ABI and matched exhaustively
against `viewer_host::Key` and `WindowAct`; `decide` asks as the windows ask — a focused
push-button, then `viewer_host::meaning`, then the selection check before a markup.
`quorra_key_press` sends what the table means and hands back the act only a window can do;
`quorra_key_said`, `quorra_key_count`, `quorra_act_count`, `quorra_act_name`; a submission's owed
sentences through `quorra_event_submit_owed_count` and `_owed`. 261 to 268 entry points. The C#
for all seven, `Native.Keyboard.cs` new. CI's `interop` job runs `dotnet build` of
`hosts/windows/Quorra.Interop/` on `ubuntu-24.04` under a 15-minute ceiling, priced in its comment.

**The premise and the hypothesis.** The brief's N1 was "the `quorra_window` layer … or whatever N1
names": RFC 0010 section 6 names the key table and its count for N1 and `quorra_window` for N2 (two
rounds), so N1 was built as the RFC states it. The hypothesis held for the file (one new module)
and not for the count (seven entry points, not a dozen) or the cache: the project references no
package, so there is nothing to cache. `ubuntu-latest` was written `ubuntu-24.04`, every job's image.

**Calibrated.** A `nuint*` planted for `float* by` in the C# and `QUORRA_ACT_SAY` changed to 2:
`header_and_interop_agree` named both; reversed. The C program presses five keys and checks each
act, reads the markup sentence, and Tabs to a push-button whose Space submits a form with Table 240
bit 11 set and bit 8 clear, reading the owed sentence (the fixture gained that button, object 11).

**Unfinished.** No `dotnet` here: the C# has never been compiled, and the job is written and its
YAML parsed, not run. `doc/todo/30` lines 262–265 still say the key table is owed; that file was
not this slot's. The markup sentence is in four places (three windows and `keyboard.rs`); it
belongs in `viewer-host`. N2's `quorra_window` is not started.

**Gates.** `rustfmt --check --edition 2024` on every touched Rust file and `cargo fmt --check -p
viewer-ffi`: 0. `RUSTFLAGS="-D warnings" cargo clippy -p viewer-ffi --all-targets`: 0. `cargo
nextest run -p viewer-ffi`: 0, 50 of 50, the C program (`-Werror`) and `header_and_interop_agree`
(4 of 4) among them. `cargo test -p conformance --no-fail-fast`: 0. `tools/state.sh --round 1618
hosts`: 0, Command 45 of 45, Query 36 of 36, 268 entry points. `tools/state.sh --round 1618 docs`:
0. Rustdoc `-D warnings` on `viewer-ffi`: 43 located errors, as before, none in a line written here.
The workflow: parsed by PyYAML, `interop` among ten jobs. `cargo tree -i viewer-ffi --workspace`
names no dependant, so no window's graph moved (habit 119). Duration: about 2 000 s.
