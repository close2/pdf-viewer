# 1770 — The Windows window is released, and a superseded run is cancelled

A "host" round beside batch one hundred and sixteen, 2026-10-26; ADR 2374; ADR 2375 unused; Q636.
I worked in `scratchpad/r1770/`. No ledger row moves; it changes no code in `crates/`.
**The request.** The owner's of 2026-10-26: improve the CI, which is red, and build and release the
new Windows UI.
**The red.** Run 40286139348 failed `deny` alone, on RUSTSEC-2026-0387 and a yank of `crypto-bigint`
0.7.5. 0.7.6 to 0.7.8 are yanked as well, so both lockfiles take 0.7.10, with `ctutils` 0.4.6
(ADR 1327's update, `deny.toml` untouched).
**Built.** `window` publishes `Quorra.App` self-contained (`dotnet publish --self-contained`, the
project's own `WindowsAppSDKSelfContained`). It checks that the exe, `viewer_ffi.dll` and
`coreclr.dll` are in the folder, and starts the exe once on `type4_pi.pdf` to read its title,
advisory. `snapshot-window` zips that folder with the Windows leg's `pdf-sandbox-worker.exe`,
LICENSE, NOTICE and a README as `quorra-winui-x64-<commit>.zip`. The new `snapshot.yml` publishes
the pre-release on `workflow_run`, only from a completed, all-green push run. `ci.yml` cancels a
superseded run. The snapshot builds lose `needs`. The fuzz job caches `fuzz/target` and builds once.
Ceilings are added or restated. The csproj publishes the dll; `round.sh` and `state.sh` read
`ci.yml`'s runs by name. Docs: `doc/todo/02`'s map, `doc/todo/30`, `doc/PLAN.md`,
`doc/state-of-play.md`, `doc/running-the-viewer.md`.
**Measured.** Five green runs: the longest job is `miri (pdf-render)`, 1186–2179 s, and the longest
gate step `cargo test --workspace`, 1176–1459 s. Published 37–60 min after the push, against
27–36 min for the longest job. The fuzz job compiled its graph twice: 588 `Compiling` lines, a
cache of an unused `target/`. 0 of 59 pushes were records-only, so there is no `paths-ignore`.
`deny` already reports at 51–68 s, so it is not gated first. The NuGet restore is 20 s, so it gets
no cache. apt stalls of 1250–1410 s are named and left.
**Premises.** `tools/round.sh host` refuses `ROUND_SESSION=1770` (no slot in the batch's brief);
I read its unnumbered `host` list. Deny's red was an advisory, as the brief guessed, and the fix
it named was itself yanked. The brief's "the dll exists in `window`" held; `platforms` and
`snapshot` build no `viewer_ffi.dll`, so the dll is not built twice.
**Unverified here** (no runner, no .NET): the publish, the folder's contents, the title read,
`download-artifact` across runs, and the fuzz cache on nightly — the first run after the merge.
**Gates.** `actionlint` 1.7.12 with shellcheck 0.11.0 over `ci.yml` and `snapshot.yml`: exit 0;
PyYAML load of both: exit 0. `cargo deny --all-features check`: exit 0. Fuzz workspace `cargo
check`, `clippy -D warnings` and `fmt --check`: exit 0. `pdf-signature` fmt, clippy and nextest
(236 passed): exit 0. `bash -n` on `round.sh` and `state.sh`: exit 0. `cargo test -p conformance
--no-fail-fast`: exit 101, one red, not mine — `records`, on 1764's 47-line record and 1765's Gates
paragraph.
