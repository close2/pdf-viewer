# 1557 — Where Doom in a PDF stands, and a script this program will not read is said

Slot 6 of batch eighty-eight, 2026-10-14/15, a script round under the owner's switch to code
improvement. ADR 1950; `doc/todo/70`; no question; no ledger row moves.

**Premise.** The raw counts held for `/Widget`, `/Btn`, `/JavaScript` and `app.setInterval`. They did
not hold for `/AA` (7 096 of 7 326 are in the WAD's base64; the file has ten) or for `/Tx` (51 in it).
**Hypothesis.** It did not hold. The rows are written as they change, but nothing runs today: the
widgets are direct, there is no `/AcroForm` and no `/DA`, and the `/O` is 6 415 756 bytes. Past those,
a `Date` that stands still inside a run keeps Doom's start-up from ever ending.

**Built.** The view state dropped an ECMAScript action past `MAX_SCRIPT_BYTES`, or with an unreadable
`/JS`, and said nothing. Hosts drop the action path's refusal at every level but `off`, so nobody
did. It is now said, with its length and the bound, at every chain site (`read_script`,
`Walked::unread`), and Table 200's "not ECMAScript" count leaves it out. `tests/script_unread.rs`'s
first test fails with the two source hunks reversed (trap 13). Its control passes either way.
Instrument: `examples/script_cost.rs`.

**Measured.** One clock hold, load 3.7, 85 s, output `scratchpad/r1557/hold.txt`. doom.pdf opens in
1.4 ms and says the new sentence. Clocked copy: decode 35.9 s, runtime 0.93 s, start-up 0.87 s with
926 edits, ticks 182–201 ms (mean 191 ms over ten), high water 240 MiB. Framebuffer copy, as means:
timer run 6.2–6.5 ms, the page interpreted after a tick 98.8–105.0 ms. quorra at 0 ms: 258 ticks and
0 of 257 frames presented in 20 s; at 250 ms, 79 ticks and 97 presented. Outside the hold: start-up
stopped at 600 s and 3 600 s on a still clock, 128.5 MiB after the decode, and callgrind's 42 % in
`LoadedFont::load` (668 loads).

**Files.** `crates/pdf-model/src/view/{scripts,script_sites}.rs`, `crates/pdf-model/tests/
script_unread.rs`, `crates/pdf-script-worker/examples/script_cost.rs`, `crates/pdf-script-worker/
Cargo.toml`, ADR 1950, `doc/todo/70-doom-in-a-pdf.md`, `doc/todo/README.md` (one line), this
record. The repaired copies are written from doom.pdf with qpdf by scripts the round left in its scratch.

**Gates.** rustfmt clean on the four Rust files. clippy `-D warnings` exit 0 on `-p pdf-model --lib
--tests` and on `-p pdf-script-worker --features engine --all-targets`; pdf-model's `--all-targets`
stops on slot 1's `examples/r1552_probe.rs`. nextest exit 0: `-p pdf-model` 2 064 passed, 21
skipped; `-p pdf-script-worker --features engine` 26 passed, 1 skipped. t2 `pdf-model --test
script_corpus` exit 0, 356 held, 0 moved; t2 `pdf-script --features engine --test script_corpus`
exit 0, 3 passed. `tools/state.sh docs` exit 0. `cargo test -p conformance --no-fail-fast` exit 101,
434 passed, 2 failed on siblings' files (slot 1's probe, slot 3's 45-line record). Clock hold exit
0. Drive: `quorra` under Xvfb on doom.pdf at `--scripts on` (the new sentence) and `off` (Tier 0's,
as at HEAD); the toolkit windows were not driven, and `quorra-confined` is pinned `off`.
