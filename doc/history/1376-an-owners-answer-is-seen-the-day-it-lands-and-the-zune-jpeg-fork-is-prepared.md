# 1376 — An owner's answer is seen the day it lands, and the zune-jpeg fork is prepared

Instruments slot of batch fifty-eight. ADRs 1588 and 1589; no row, no question.

**Answers.** `tools/main-checkout.py` prints first every untracked or modified `A` file in the main
checkout's `doc/questions/`, newest first, dated by when it landed, then the questions open once
they are counted and those a list of tracked files alone still calls open; `--answers` prints only
those, and `tools/batch.sh check` repeats them. On the day: 19 answered, uncommitted, five of them
2026-10-05; one question open, Q254; 17 more the tracked files alone call open. The old count-only
`doc/questions` kind is gone; `doc/environment.md`'s *After a merge* has both new entries, and
`tests/owner_section.rs` holds them first and the check's call (ADR 1588).

**The fork.** Upstream tags no 0.5.15; the crate's `.cargo_vcs_info.json` names `31d81fed`, whose
`crates/zune-jpeg/src/` is the published `src/` byte for byte. Both patches apply there, alone and
together, with `--directory=crates/zune-jpeg`; neither had drifted. Their preambles now state
`Base:` as that commit, `Directory:` and `Fork: https://github.com/close2/zune-image`. A scratch
crate pinning the repository by `rev` resolves and builds, `zune-core` coming from the same commit.
The stanza and the owner's sentence are a comment above `zune-jpeg` in `Cargo.toml`; `main-checkout`
prints the owner's step while A227 is on the disk and the manifest pins no fork, and counts both
patches applied once it does. The artefact was gone, so the DC reproducer is the patch's hexadecimal
(846 bytes, not the 847 the preamble said): `banded_decodes.rs`'s ignored
`a_dc_prediction_past_i32_is_decoded_rather_than_aborting` panics at `bitstream.rs:400` today.
Against a scratch export patched to the clone, `cargo test -p pdf-model --no-fail-fast` passed
1784 and failed only the grey-row guard ADR 1520 wrote to fail that day. The two upstream reports
are `doc/patches/zune-jpeg-*.md` (ADR 1589). A `cargo new` probe briefly entered the workspace's
`members`; removed within minutes.

**Documents.** HANDOVER, PLAN and state-of-play read against ADRs 1565–1577: eleven sentences made
true, crate-map unchanged; HANDOVER has no rows keyed by answer date, so none was added. Ordinals
(`spelled_ordinals`): todo 23 54 → 0, 11 25 → 0, `_scan-conversion` 23 → 0, 48 21 → 0; all of
`doc/todo/` 375 → 252. Records 1365–1370: 38–40 lines, each with Gates. `doc/todo/65`, re-read after 1375's record: Q192's
sentence made A192's, the 2026-10-05 answers placed; 1375's §12.10 bullet kept. State-of-play
already held 1371's Tier 0 and 1375's census, in their rounds' words.

**Gates.** `rustfmt --check` on `banded_decodes.rs` and `owner_section.rs`: 0. `bash -n` on
`batch.sh`, `state.sh`: 0. `RUSTFLAGS=-D warnings cargo clippy -p pdf-model -p conformance
--all-targets`: 0. `cargo nextest run -p pdf-model`: 0, 1862 passed; `--ignored` DC test: 101,
the panic. `cargo test -p conformance`: 0, 383 passed, once `tools/state.sh scripts` named
`script_corpus`'s gate line, which `state_sections` had failed on.
