# 1760 — The navigational re-read: the script modules unnamed, two build claims false, two counts stale

Session 1462. Status: **accepted**. ADR 1638 rule 7's re-read of the four navigational documents,
owed since batch seventy, under `CLAUDE.md`'s *Where knowledge lives* and ADR 1023's rule. It decides
no new rule; it records what was false so that the next re-read starts from the residue.
Context: ADRs 1023, 1638, 1680.
Prose: `doc/PLAN.md`, `doc/crate-map.md`, `doc/state-of-play.md`, `doc/HANDOVER.md`; `doc/todo/02`
section 0; `doc/traps/every-round.md`; `doc/habits/every-round.md`.

## 1. How the four were checked

As ADR 1680 section 1, with one more instrument. `cargo metadata --no-deps` against the crate map's
rows: 43 members, 43 rows, none missing and none extra. `tools/state.sh navigation`: one absent
pointer in the four, `doc/questions/A170`, the owner's uncommitted answer, as last time. The same
throwaway extractor over every backticked path and identifier, calibrated first by a planted file
of three false names, all three named: every pointer resolves, the owner's uncommitted `A` files in
the main checkout. Then a census new to this re-read:
the source files added since ADR 1680's commit (`git diff --diff-filter=A fe5d66eb HEAD`), and for
each crate-map row the modules under `src/` the row does not name. Then every sentence of `PLAN.md`
and the rows and paragraphs the census pointed at were read as *what is*.

## 2. What was false, and what replaced it

| the document said | the tree says | checked by |
|---|---|---|
| "No CMake, no moc" (`PLAN.md` section 3) | `viewer-qt`'s build script runs `moc` through `cxx-qt-build`; the sentence now lists the five build scripts and names `ls crates/*/build.rs` | `crates/viewer-qt/build.rs` |
| `pdf-sandbox/build.rs` "bakes the confined worker's path" | it stamps the build identity the greeting compares (ADR 0458) | `PDF_SANDBOX_BUILD` in the script |
| §8.9.5.2 is `partial` on its general `/Decode` array (`PLAN.md` section 5a's example) | `implemented`; the paragraph now states the rule without the example | the ledger row |
| a fuzz campaign runs "behind the heavy-walk lock" | a `--long` run, the second lane only (ADR 1756); section 0's tier-2 line gains the kind too | `doc/environment.md`'s rule block |
| `raster_golden` holds a digest triple per page, nothing more | it draws undivided (ADR 1742) and a second test holds the divided draw to the division bound (ADR 1758) | the test file's header |
| `vulkan-swrast` "makes GPU output reproducible in CI" | CI installs `mesa-vulkan-drivers`; the Arch package is the same lavapipe, which is what reproduces | `.github/workflows/ci.yml` |
| `pdf-script`'s bridge is the whole host object model | five engine modules beside it, and `depth.rs` and `question.rs` unnamed | `ls crates/pdf-script/src/engine` |
| `pdf-model`'s `view/scripts.rs` is the scripts' module | four more under `view/`: the model, the sites, the annotations, the timers | `ls crates/pdf-model/src/view` |
| `viewer-host` row: no `popup`, `script_asks`, `script_timers`; `viewer-core` no `script_question` | each named with its ADR | `crates/viewer-host/src/lib.rs` |
| `pdf-colour` row: no `planckian.rs`; `pdf-font` row: no `pairs.rs` | named (ADRs 1713; 1682, 1696, 1708) | `ls src` |
| `state-of-play.md`: rich text runs not kerned; signature policies absent | the kerning clause and a policy paragraph (ADRs 1219, 1709, 1728, 1738, 1753) | the ADRs and their code |
| "the other hundred-odd are cited once each or never" (`traps/every-round.md`) | several traps outside the ten are cited ten times or more | `grep -rhoE 'traps? [0-9]+' doc/history/` |
| "about seventy habits" (`habits/every-round.md`) | the six files hold over two hundred bold-led items; the command now stands in | `grep -c '^- \*\*'` |
| `gate-ratchet` "a dev-dependency of three crates", `corpus-classes` "of two" | true today; each count is now the `grep -l` that prints it | `crates/*/Cargo.toml` |

Chronology removed: `viewer-host` "exists because the second host wanted four of `viewer-gtk`'s
eight modules" (the crate map and the state of play; `viewer-gtk` has six), `raster-pages` "until
this crate existed", "not yet a crate", the attach gesture's "yet". `PLAN.md` section 5a gains
`tests/round_numbers.rs` (ADR 1698) and `ls tools/conformance/tests/` as the population.

## 3. What the hypothesis said, and what held

Batch seventy-three's brief said the drift would be greatest where batches 67 to 72 built. The first half
held for half the table: seven of the fourteen rows are those batches' builds — the scripts, the
popups, the lock's long kind, the golden's divided draw, the Lab white point, the kerning and the
signature policy — and every one of the seven is an omission, a list of modules or capabilities that
stopped being whole without anything in it turning false. The other half did not: no sentence named a construction
those batches retired. The three false constructions were older (the build script, `moc`, §8.9.5.2's
example), and each named something no round of the last six batches touched. **The documents grew**:
`PLAN.md` 756 → 765 lines, the crate map 64 → 64, `state-of-play.md` 1 052 → 1 061, `HANDOVER.md`
88 → 88. The command for the words is `wc -w` on the four.

## 4. Left to its owner

- The rule block's first line writes `RAYON_NUM_THREADS=4` before `tools/bounded.sh`, which exports
  its own share unless the caller set one; slot 5 owns that line.
- `PLAN.md` section 5a names `tools/comment-history.py` for `tools/state.sh comments`, and slot 5 may
  retire it this batch; that sentence goes with it.
- Not checked: the crate map's "Six of the eight generators" (`pdf-vfs`), and `state-of-play.md`'s
  "four documents … with 115 beads" (ADR 0405's population example).
