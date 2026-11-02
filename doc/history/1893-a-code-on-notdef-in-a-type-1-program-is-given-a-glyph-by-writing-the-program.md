# 1893 — a code on `.notdef` in a Type 1 program is given a glyph by writing the program

Session 1893, batch 130, slot 3 (archive, R03a for Type 1 programs), in two sessions. ADR 2620; ADR 2621 and Q769
unused.

**What changed.** `pdf_font::type1::with_glyphs_given` (new module `type1_glyphs_given.rs`) decrypts a `/FontFile`
program's `eexec` section, grows its `CharStrings` dictionary by a copy of `.notdef`'s charstring per width (or a
glyph that draws nothing) under charstring encryption at the program's `lenIV`, writes the section again with
`/Length1`–`/Length3`, and reads it back byte for byte and through `read-fonts` before returning it.
`LoadedFont::draws_notdef_in_place` names a code §9.6.5.2 sends to `.notdef`. The archive's CFF route
(`cff_notdef_glyph.rs`) takes a simple Type 1 font through the same proof; `truetype_cmap.rs` states the Type 1
programs in the history; `notdef_format.rs` drops the Type 1 refusal. A fuzz target `type1_write` with its seed recipe
(`fuzz/seed_streams.py`, `fuzz/seeds.sh`) and its `doc/verify.md` line; R03a's lines in `doc/pdfa-blocking-issues.md`
and `doc/pdfa-remedies.md`.

**Measured.** Against HEAD's census, with and without the pack: `issue11477_reduced.pdf`, `issue6721_reduced.pdf` and
`issue6901.pdf` convert at every line (as-if-printed 2b 884 → 887 with the pack), the Type 1 rows are gone, and
`504076.pdf` stays refused on rows it held. veraPDF passes all six outputs. The hostile probe and the fuzz figures are
ADR 2620 section 3's.

**The plant.** `lenIV` 1 for 4 in the writer's charstring encryption: five of eight writer tests red on "does not read
back", no program returned; restored.

**Found, not fixed.** The fuzz target's last property (a written program written again) fails on six inputs: five
headers end `eexec\r`, where `read-fonts` 0.44's `find_eexec_data` (`src/ps/type1.rs`) skips the `\r` only if a `\n`
follows anywhere in the data, so the section's start moves with the re-encrypted bytes; one header opens a `{` it
never closes. The read-back refuses both, so nothing wrong is written; Adobe Type 1 Font Format section 7.2 against
FreeType's rule is a question for a later batch. The probe's 796 MiB for a 251 KB input is the 64 MiB bound times the
copies the read-back and the proof hold; a lower bound is a later batch's.

**Gates.** `cargo nextest run -p pdf-font` exit 0 (349 passed). Clippy `--no-deps -D warnings -p pdf-font -p
pdf-transform --all-targets` exit 0; `cargo fmt --check -p pdf-font` exit 0; under `fuzz/Cargo.toml` `cargo fmt
--check` exit 0 and `RUSTFLAGS=-D warnings cargo clippy --all-targets` exit 0. `cargo test -p pdf-transform --test
archive --test profiles` exit 0 (524 and 13 passed). `cargo test -p conformance --test fuzz_workspace --test
fuzz_seeds` exit 0. Census two runs exit 0 (1 337 s together). Fuzz 602 s exit 0: 21 338 199 executions, 1 501
edges, 6 crashes (one property, above), peak 0.08 GiB. Probe: hostile 796 MiB, 4.1 s (HEAD refuses, 13 MiB); 256
distinct glyphs 372 MiB here, 373 MiB at HEAD. veraPDF 7 runs: six outputs exit 0, the planted source fails.
`--bin cited` exit 0, `--bin quotations` exit 0. `cargo test -p conformance --no-fail-fast` exit 0 (69
binaries).
