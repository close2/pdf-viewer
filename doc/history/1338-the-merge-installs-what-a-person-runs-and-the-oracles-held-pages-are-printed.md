# 1338 — The merge installs what a person runs, and the oracle's held pages are printed

Instruments slot of batch fifty-two. ADRs 1511, 1512. No ledger row moved; no question written.

**What a person ran (ADR 1511).** The main checkout's `target/` held all ten programs and both C
libraries from 2026-09-11 15:16 (`main` then `44c77ef5`, 79 commits ago) and `safedocs` from
2026-08-18; nothing said which commit; `quorra --version` opens a file called `--version`. The
documented step copied from `/home/AI/cargo-target/quorra/`, which is gone, and a worktree round
cannot write the main checkout anyway. Now `tools/batch.sh install`, after the fast-forward and
before `close`: refuses a dirty worktree and a HEAD that is not `main`'s, builds the ten programs in
one `--release` invocation and the libraries in a second, in the directory Cargo names in the
worktree, installs them (mode 775) into the main checkout's `target/`, and writes
`target/installed-from` (commit, subject, SHA-256 per file). Ten and two, not the brief's six:
six would leave `quorrafs`, `pdf-vfs-worker`, `quorra-retrieve`, `quorra-transform` and the
libraries from September. `tests/batch.rs` runs it on a throwaway workspace with stand-in targets
(both refusals, then `git status --ignored` shows only `target/`) and holds the names to the
manifests. `state.sh binaries` reads the main checkout's record; `round.sh` fails while `main` is
past it (today: no record). `todo/02` section 5 rewritten (a round installs nothing, builds
`--release` before a measurement), section 8 step 5 gains the step; `running-the-viewer.md`,
`environment.md`'s prune comment and `verify.md` follow.

**Held pages (ADR 1512).** `--bin held` / `state.sh oracle-held` (in `all` and `quick`; `oracle`
was already the walk): 47 contradicted in 15 groups, 810 ambiguous in 68, 46 not comparable in 9,
9 no render in 4, 2 reference geometry; contradicted 2 ours / 12 references' / 33 choice;
candidates `issue4436r.pdf` and `issue7891_bc1.pdf`. The run's 836 ambiguous includes 26
incomplete pages no group holds by construction.

**Documents.** state-of-play: word gap in text space (1490), zoom step's fill (1493), walk-only rows
(1497). PLAN: third ratchet, `install`, `oracle-held`. crate-map: field value (1489), fill record
(1493), `install`, `held.rs`. HANDOVER: robustness and run rows. Navigation before/after: 14
absent, 28 undefined, 22 overtaken, history 0 — unchanged. `todo/65`: curves re-searched
2026-10-02, versions unchanged, five unjudged Ed448 names listed; map gate passes. Records
1327–1332 all pass the gates rule. `batches`: 9f544eb4 sums 139036 s over 7 figures, 6 rounds, no
remainder.

**Gates.** `rustfmt --check` on my five Rust files exit 0; `RUSTFLAGS=-D warnings cargo clippy -p
conformance --all-targets` exit 0; `bash -n` on `state.sh`, `batch.sh`, `round.sh` exit 0; `cargo
test -p conformance` exit 0, 373 passed (`batch` 8, `read_only` 2, `state_sections` 2, `commands` 3,
`the_frontier_map` 1, `records` 3); `tools/batch.sh check` exit 1, only `cargo fmt --all --check` over siblings' in-flight files.
