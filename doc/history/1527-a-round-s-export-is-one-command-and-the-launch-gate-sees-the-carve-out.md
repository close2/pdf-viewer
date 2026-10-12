# 1527 — A round's export is one command, and the launch gate sees the carve-out

Slot 6 of batch eighty-three, 2026-10-12, an instruments round under the owner's performance target;
the partial-row word yields to it. ADRs 1890 and 1891; no question. No ledger row moves.

**Premises, held.** `grep fdinfo launch_path.rs` and `grep "export)" tools/batch.sh` printed nothing;
`mem_info_vram_used` read 371–428 MiB as this user. **Hypothesis (b), held**: three bring-up children
asked 197 472–197 476 KiB of the carve-out and were placed 864–868, ADR 1872's 197 476 / 868 again.

**Built (a), ADR 1890.** `tools/batch.sh export <session> [--as <name>] [--patch <file>]... [<path>...]`:
HEAD's archive at today's time, every submodule and every `git status --ignored` path linked (not
`.cargo/`, `target`, `__pycache__`), named paths whole, patches checked against HEAD, its own target
in its own `.cargo/config.toml`, a record beside the tree; `export-sums` hashes every executable the
target holds. Replaced by rename after the arms' `done` (trap 130). Here: 0.6 s, 43 ignored paths and
7 submodules linked, and of 5 035 tracked files compared blob by blob one differs, the one named.

**Built (b), ADR 1891.** Bring-up and first-page children print what the device asked of the
carve-out, what was placed there, and the adapter's total and used; it had room when every kibibyte
asked was placed. Only `bring_up_ms` and `first_page_ms` are held to it (`Declined::TheCarveOut`,
counted in the summary), and the reading is printed beneath both on every run.

**The figure moved (headless gate).** Before, the merge's 16:45 run on HEAD failed `bring_up_ms`
24.039 and one `first_page_ms` with the carve-out full. After, one clock hold, desktop at 423–428 MiB:
`bring_up_ms` 27.110 and six `first_page_ms` 34.4–39.8 printed and declined, "(0 of them for load,
7 for the carve-out)"; HEAD's arm beside it, `bring_up_ms` 27.798, 9 declined for load at 15.17.

**(c), read.** `gates-cost` on batch 82: 35 small walks, 17.4 s queued with lane 2 free (longest
1.7 s), ADR 1869's poll a walk; clock runs queued 7 210 s against `sidelane.py`'s 8 026 s built-rule
replay. The re-ask holds. **Not this round's**: `peak_anon_mib` under two floors (PDF20 18.70 < 19.5,
WTPDF 22.43 < 23.0); interleaved children, same frames, read PDF20 HEAD 17.76–20.79 MiB, this
18.09–18.59, WTPDF HEAD 21.54–24.40, this 21.58–21.96 — trap 34's third dimension again.

**Files.** `tools/batch.sh`, `tools/conformance/tests/batch.rs`, `crates/viewer-ui/tests/launch_path.rs`, `doc/checks/launch-path.toml`, `doc/traps/{README,instruments-and-reports}.md`, ADRs 1890, 1891, this record.

**Gates.** `rustfmt --check` on both Rust files, `bash -n tools/batch.sh`: 0. Clippy `-D warnings`,
`conformance` and (both exports) `viewer-ui`: 0. `cargo nextest run -p viewer-ui` (export): 183 passed,
2 skipped. `cargo test -p conformance`: exit 101, 396 passed, 1 failed (`names`, slot 3's `encode/instance.rs:258`). The export tests fail with `tar -m` or the `git apply`
ceiling planted out. `tools/batch.sh check`: exit 1, slot 3's `raster-gpu` formatting only. Walks
queued 861/294/378/0 s, held 451/160/567/17 s. Clock holds queued 958 and 115 s, held 64 and 56 s,
`launch_path` exit 101 both (the first showed the script stage held to the carve-out; fixed). Duration 5507 s.
