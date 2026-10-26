# 2374 — The Windows window ships as a self-contained zip, and the snapshot is published from a completed run

Session 1770, a "host" round beside batch one hundred and sixteen, 2026-10-26.
Status: implemented in `.github/workflows/`, `hosts/windows/Quorra.App/Quorra.App.csproj` and both
lockfiles, and unrun: no runner and no .NET SDK exist on this machine, so the first CI run after the
merge is each change's first execution; the signing question is Q636. Context: the owner, 2026-10-26:
"the repository is now pushed regularily (every minute). Use one round, to improve the CI (it's red
now anyways) and also build and release the new windows UI." ADRs 0450, 1327, 2024, 2136, 2301.
Cites no clause and moves no ledger row.

## 1. The red

Run 40286139348 on `ecb6bea0` failed `deny` alone: RUSTSEC-2026-0387 ("Multiple timing variability
issues in operations intended to be constant-time") and a yank, both on `crypto-bigint` 0.7.5, which
`elliptic-curve` brings to `pdf-signature`. The advisory names 0.7.6; 0.7.6 to 0.7.8 were yanked in
their turn, so both lockfiles take 0.7.10, the newest unyanked, with `ctutils` 0.4.6 beside it. That
is ADR 1327's rule: an advisory is taken by updating the crate, and `deny.toml` is not touched.

## 2. What the Windows release is

**An unpackaged, self-contained folder in a zip**, `quorra-winui-x64-<commit>.zip`, beside the three
archives in the `snapshot` pre-release. The project already is unpackaged with the Windows App SDK
beside the executable (ADR 2136); `dotnet publish --self-contained` adds .NET's runtime, which the
Windows App SDK's deployment guide says a .NET app needs as well. Unzipped, `Quorra.App.exe` runs on
a double click with nothing installed first. The folder carries `viewer_ffi.dll`, built once by the
`window` job, and `pdf-sandbox-worker.exe` from the Windows snapshot leg — the library looks for it
beside the running executable — with LICENSE, NOTICE and a README. `snapshot-window` assembles it
on Linux from the two jobs' artifacts, so no Rust is built twice for one target.

**Not an MSIX**: a package installs only when it is signed by a certificate the machine trusts, the
repository holds none: an unsigned package does not install outside developer mode, and a
self-signed one only once its certificate is imported into the machine's trusted store. The zip
runs today; the unsigned executable meets
SmartScreen's unknown-publisher prompt, which the README names. Signing, and with it an MSIX, is
the owner's to decide (Q636); nothing waits on it.

## 3. What CI does now

- **A push** starts one run of `ci.yml`. The snapshot builds start beside the gates instead of after
  `check`, `test` and `platforms`.
- **A burst of pushes**: a newer push cancels the run before it (`concurrency`, keyed by workflow,
  event and ref). The last push of a burst has nothing to cancel it and runs to its end. A pull
  request's runs cancel in the same way, keyed by their own ref.
- **The snapshot** is published by `snapshot.yml`, on `workflow_run`, only for a run that was a push
  to main in this repository and completed with conclusion `success`, so never from a cancelled run
  and never by a run cut halfway through deleting the release. Its group never cancels a publish in
  progress, and a re-run older than the commit the release names is not published over it.
- **The tree's own readers of CI**, `tools/round.sh`'s CI line and `tools/state.sh main-checkout`,
  ask for `ci.yml`'s runs by name: the newest run on main is otherwise `snapshot.yml`'s.

**What changes for a user**: the snapshot names a commit whose every job passed. That now includes
`deny`, `interop`, `raster-examples` and `window`, where it used to be `check`, `test` and
`platforms` alone. Run 40286139348's snapshot was published with `deny` red, and would not be now.
The latency to a release, for the five green runs of 2026-10-20 to 22: 37 to 60 minutes as
published, about 29 to 38 by the longest job of each, which is now `miri (pdf-render)` (20 to 36
minutes, advisory). That job is the next lever on the time to a release.

## 4. Measured, changed, rejected

From the five green runs 39850609005, 39827417634, 39734821882, 39707587946 and 39414723133. The
longest job is `miri (pdf-render)` (1186–2179 s), and its step the longest step (1104–2097 s). Of
the gates, `cargo test --workspace` is the longest step (1176–1459 s).

| change | the count that decided it |
|---|---|
| `concurrency`, cancel in progress | a full run is 15 jobs and 37–60 min; 3 of 89 runs since 2026-07-26 overlapped the one before, and the owner's cadence is now a minute |
| snapshot builds lose `needs` | `test`'s 23–26 min stood in front of the builds' longest leg, 12–34 min |
| fuzz caches `fuzz/target` | the cache held 471 MB of the root `target/`, which `cargo fuzz` never writes; 588 `Compiling` lines a run |
| fuzz builds once, in release | `build --dev` 3 min 16 s, then the first `run` built release in 7 min 43 s (run 39850609005) |
| `timeout-minutes` on `snapshot` (70), `snapshot-window` (10), publish (10); `window` 45 → 30 | ADR 2024's twice the longest healthy run: 2016 s for a snapshot leg, 610 s for `window` plus the priced new steps |
| **rejected**: `deny` first, the rest after it | it already reports at 51–68 s with no `needs`; gating on it masks a test failure behind an advisory, which its own comment refuses, and adds its minute to every run |
| **rejected**: `paths-ignore` for records | 0 of 59 pushes touched only `doc/history/`, `doc/adr/` or `doc/reviews/`; `cargo test --workspace` runs `records`, `documents` and `adr_status`, which read them |
| **rejected**: a NuGet cache | the restore is 20.4 s of the window's 43 s build, and a cache of the Windows App SDK's packages costs much of that to fetch |
| **already so**: `fail-fast: false` | all three matrices have it |
| **left**: apt's stalls | 2 of 5 runs spent 1250 and 1410 s in `check`'s apt step, and one snapshot leg 1295 s, all in slow downloads from the Azure mirror; a cache of the `.deb` files is the remedy, and unrunnable here |

## 5. §12.2, and the one run of the window

§12.2's row stays as ADR 2301 has it. The window is now built and released, and still no gate runs
the placement the row owes. The `window` job does now start the published executable once, on
`doc/corpora-own/type4_pi.pdf`, and reads its title until it names the document. That shows that
the runtimes start, the library loads and a document opens; it does not show a page drawn or a
placement made. The step is advisory (`continue-on-error`) because it has never run. The round that
reads its first answer makes it a gate or says why not. What would move the row is the placement
driven on Windows: a document with `/FitWindow`, `/CenterWindow` and `FullScreen`, its window's
rectangle read back, as `drive_a_window.c` reads the library's half (ADR 2308). That is the next
slot's contract, written in `doc/todo/30`.

## 6. What only the first run after the merge can tell

- whether `dotnet publish` takes these properties and stays clean under `TreatWarningsAsErrors`;
- the folder's size and file list;
- whether `MainWindowTitle` reads WinUI's window on `windows-latest`;
- whether `download-artifact` with `run-id` finds the CI run's artifacts in `snapshot.yml`;
- whether rust-cache's `fuzz -> target` saves anything on nightly, whose cache key changes daily.
