# 1628 — A press is decided once in `viewer-host`

Slot 6 of batch ninety-eight, 2026-10-20, a host round: ADR 2092 (2093 unused), no question, no
ledger row (a refactoring; no clause reading moved).

**Premise.** Held: the markup sentence was written four times (`viewer-gtk`, `viewer-qt`, `quorra`,
`viewer-ffi/src/keyboard.rs`), and so were the focused push-button's precedence, Table 29's mode off
`Presenting::full_screen` and `ScrollBy` times the scale. **Hypothesis.** Held: one function and one
enum; the four hosts lost 281 lines and gained 81 (session's 9 included), 200 net.

**Built.** `viewer_host::decide` returns `Decided` (`Activate { name, annotation }`, `Send`, `Window`,
`Say`, `Nothing`) with `SELECT_FIRST`, `Keyboard::{Page, Control}` (GTK's control that passed a key on),
`Mode::of` and `scroll_by`. Deleted: `viewer-ffi`'s `Decided`/`decide`/`selected`, GTK's and Qt's
`press`, `quorra`'s and `NativeWindow`'s `has_selection`. A host keeps its toolkit's half: the
tab walk's focus, its own view, presenting. GTK's scroll was `(f64(by) * f64(scale)) as f32` on an
integer scale; that product is exact in `f64`, so its one rounding is the `f32` product's (ADR 2092 §2.4).
The decide tests moved from `keyboard.rs` to `keys.rs`; `push_buttons.rs` gains one for `Keyboard`.
`doc/teaching/how-a-form-field-is-filled.md` stage 2 and its diagram name `keys.rs::decide`.

**Proof that nothing moved.** The C program's four `keyboard`/`keys` lines are byte-identical before
(HEAD) and after (H's sentence, Space on button 11, the five table presses). The drive, `--step drive
script_keys list_arrows combo_arrows archive_converted` in all four windows, from an export at HEAD
and then HEAD plus this round's paths and two `-U0` patches (one target, two builds): 151 works, 0
wrong, 7 not offered both times; all 158 verdicts identical, and what each step saw is identical once
the export's path and the loopback port are normalised. `quorra-confined` and the three workers came
out byte-identical by SHA-256; `quorra`, `quorra-gtk` and `quorra-qt` differ, as they must.

**Gates.** `rustfmt --check` on the 11 files: exit 0. `cargo fmt --check -p` viewer-host, viewer-gtk,
viewer-qt, viewer-ui: exit 0; viewer-ffi: exit 1 on slot 4's new `window.rs` and `window/asked.rs`
only. `RUSTFLAGS="-D warnings" cargo clippy --all-targets` on the five crates: exit 0 (a first run
exited 101 on slot 8's mid-split `pdf-colour`). `cargo nextest run` on the five: 518 of 519, the one
`unsafe_position` (273 entry points against 252, slot 4's). C program: exit 0 both times.
`cargo test -p conformance --no-fail-fast`: exit 101 on `names` and `teaching`, both naming slot 8's
moved items; a second run, exit 101 on `records` alone, naming slot 5's record 1627 and not this
one. `--test teaching`: exit 0, and a planted `decidez` named (exit 101), then taken out. `tools/state.sh --round 1628 docs`: exit 0. `hosts`: exit 0,
Command 45 of 45. Two release builds under the lock: exit 0, 873 s and 352 s (2 267 s queued).

**Shared files.** `crates/viewer-ffi/src/session.rs` (slot 3's: three `-U0` hunks — the import, the
`Decided` import, the `press` match), `crates/viewer-host/src/native_window.rs` (one deletion).
**Unfinished.** Nothing. 5 300 s.
