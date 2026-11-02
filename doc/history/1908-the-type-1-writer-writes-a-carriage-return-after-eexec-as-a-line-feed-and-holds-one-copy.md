# 1908 — the Type 1 writer writes a carriage return after `eexec` as a line feed and holds one copy

Session 1908, batch 131, slot 8 (parsers, robustness: ADR 2620 section 3's two open items), in two sessions. ADR 2650;
ADR 2651 and Q784 unused.

**What changed.** `Sections::header_written` copies a Type 1 program's header byte for byte but writes a carriage
return in the white space after `eexec` as a line feed: Adobe Type 1 Font Format section 7.2 places every white space
byte there before the ciphertext, and `read-fonts` skips a carriage return only where a line feed follows somewhere in
the data, so the section's start moved with the bytes the writer re-encrypts. `MOST_CHARSTRING_BYTES` (65 535, chapter
10) refuses a given glyph's charstring past the format's limit before anything is written. The writer holds one copy:
a given glyph is its name, advance and outline choice, its body rebuilt where written and where read back, the section
written into the program and encrypted in place, the read-back's decrypted copy dropped before `read-fonts` parses.
The bound test and the carriage-return test replace the byte-bound test; the second states finding `defcb9b3`,
minimised by `cargo fuzz tmin` to 370 bytes, as its header and its decrypted section. The fuzz target's header property
admits the line feed; `doc/verify.md`'s `type1_write` line says so.

**Measured** (ADR 2650 section 4). The hostile probe, 246 006-byte `.notdef` reached by 255 codes: HEAD peaks at
1 179.7 MiB and 856.9 MiB on two runs; here it is refused by name at 12.3 MiB. At the bound (64 998 bytes): 248.6 MiB at
HEAD, 237.8 MiB here, the writer's own share about 58 MiB for a 16.8 MB program. Census without the pack against HEAD's:
no document moved.

**The plant.** `header_written` copying the white space unchanged: the carriage-return test and all five findings red,
each on the written program written again; restored, green.

**Gates.** `cargo nextest run -p pdf-font` exit 0 (355 passed); `cargo clippy --no-deps -p pdf-font --all-targets --
-D warnings` exit 0; `rustfmt --edition 2024 --check` on the three `pdf-font` files exit 0; under `fuzz/Cargo.toml`
`cargo fmt --check` exit 0 and `RUSTFLAGS=-D warnings cargo clippy --all-targets` exit 0; `cargo check --tests -p
pdf-font` in the shared worktree exit 0. Replays: five `eexec\r` findings exit 0, `c8f91bdb` exit 77. Fuzz `type1_write`,
fork mode, 4 jobs, `-ignore_crashes=1`, 614 s: 2 560 616 executions, 1 736 edges, 8 417 features, 0 out of memory,
0 timeouts, 0 crashes, peak 0.27 GiB, `bounded` exit 0. Census without the pack exit 0 (469 s, 2.51 GiB). `--bin cited`
exit 0; `--bin quotations` exit 0 (3 049 quotations, 2 851 verbatim, 3 diverging); `cargo test -p conformance
--no-fail-fast` exit 101 with eight reds, the same eight in HEAD's export (tree-shape tests reading an export).

**Found, not fixed.** `crates/pdf-font/src/type1.rs:125`, `Program::parse` reads through `read-fonts`' `Type1Font::new`
and so its rule: a stored program whose header ends `eexec\r` and whose ciphertext holds no byte 10 misreads in the
viewer itself. Finding `c8f91bdb` (a header opening a `{` it never closes, its `eexec` found only by a later copy) still
fails the written-again property; the fork-mode run did not reach its shape again. The conversion at the bound holds
about 180 MiB beyond the writer for a 16.5 MB font stream, after the writer returns.
