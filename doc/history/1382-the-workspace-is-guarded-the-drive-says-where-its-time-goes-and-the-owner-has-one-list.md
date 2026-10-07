# 1382 — The workspace is guarded, the drive says where its time goes, and the owner has one list

Instruments slot of batch fifty-nine. ADRs 1600 and 1601; no row, no question.
**Workspace.** `tools/batch.sh check` names a `members` entry (globs expanded) whose `Cargo.toml`
git does not track or that is under `scratchpad/`, and any `__pycache__` under `tools/`, `crates/`,
`raster/`. `tests/batch.rs` plants each, sees the line, removes it, sees `none`; a second test holds
every `python3` in `tools/*.sh` to `PYTHONDONTWRITEBYTECODE=1` (`drive-windows.sh` exports it,
`bounded.sh` sets it). On the day it named a sibling's untracked `crates/pdf-script` (ADR 1600).

**Gate log.** Its summary had overwritten `build-sandbox`'s line (`gates`'s output was the log);
`gates()` no longer prints it there, and `gates-cost` names all 34, the missing one too (ADR 1600).

**Drive.** `verdict` stamps a fifth column, the seconds since the verdict before. `state.sh drive`
sums it per window, per step group and per step. Driven behind the lock: 133 works, 0 wrong,
3 not offered, 995 s wall (990 s over 136 verdicts); quorra 329 s, qt 304, gtk 299, confined 59.
Slowest steps: `23-push-button` 27.6 s in each window, almost all fixed sleeps — `launch`'s
`sleep 5`, then about 16 inputs each followed by `key`/`type_in`/`click`'s 1.2–1.3 s.
`33-aimed-field` 25.0 s: two launches at 5 s, each with 1.5 + 1.5 + 1.3 + 1.5 + 1 s of sleeps
around one AT-SPI click and one save. `27-processor-fallback` 18.5 s: a launch, a poll for the
refusal line every 5 s, then `sleep 3`. `launch`'s `sleep 5` is in every group; a wait for the
window's first trace line is the change to price. Largest group: `25`, find, 151.5 s.

**Owner.** `main-checkout.py` ends in one numbered list, 10 items on the day, in the order a person
does them: Q169/Q170's `§`, commit 19 answers, answer Q254/Q270/Q271, the zune-image fork, two
upstream reports (owed until a `Filed:` line), three hayro patches, re-seed 9 corpora, remove 5
artefacts, prune the 636 GiB build directory, decide `sccache` at 50 of 50 GiB. Each file is named
once. `owner_section.rs` holds the list's shape against a planted checkout; `doc/environment.md`
follows (ADR 1601).

**Documents.** Read against ADRs 1578–1589. crate-map: `aform/`, `view/scripts.rs`, `geospatial/`,
`reader`, `post.rs`; state-of-play: the software driver's 3.5 ms (ADR 1585); PLAN: JavaScript in
RFC 0008's tiers; HANDOVER: the drive's times, the owner's list.
Ordinals: todo 13 21 → 0, 34 20 → 0, 21 18 → 0, 49 17 → 0; all of `doc/todo/` 252 → 176.
Records 1371–1376: 35–40 lines, each with Gates. `doc/todo/65` re-read after 1381's aggregate edits
(§7.4, §7.6.5, §12.10): true, the frontier gate green; nothing changed.

**Gates.** `rustfmt --check` on `batch.rs`, `owner_section.rs`: 0. `bash -n`, four scripts: 0.
`RUSTFLAGS=-D warnings cargo clippy -p conformance --all-targets`: 0. `cargo test -p conformance`:
101, only `fuzz_workspace` (a sibling's lock); `--test batch` 0, 12 passed; `owner_section` 0, 4.
The drive behind the lock: 0, 133 works.
