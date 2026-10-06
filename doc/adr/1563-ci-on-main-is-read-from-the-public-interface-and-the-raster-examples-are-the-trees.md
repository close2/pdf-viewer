# 1563 — CI on `main` is read from the public interface, and the raster examples' failure was the tree's

Status: **accepted**. Amends nothing; supersedes nothing.
Context: `CLAUDE.md` principle 1 (warnings are errors in CI) and principle 4; ADRs 0060 of
`raster/` (every example's `--check`), 0450, 0463, 1517 and `raster/`'s 0026; trap 10.
Code: `tools/state.sh` (`ci_on_main`, called from `main-checkout`), `tools/round.sh` (its CI line's
hint), `raster/crates/raster-gpu/examples/retained.rs` (the signature is the first frame's),
`raster/crates/raster-gpu/examples/outline_upload.rs` (`WITNESS_SEGMENTS`),
`crates/pdf-render/src/shading.rs` (`rasterise`; two tests' `cfg_attr(miri, ignore)`),
`crates/pdf-render/src/collapsed/stroke_image.rs` (one test's dash array).

## 1. How a round learns CI's state

`tools/round.sh` asks `gh run list` with a token it reads from `github-token.txt` beside the main
checkout (gitignored, the owner's). A round's own shell has no `gh` login, no `~/.config/gh` and no
`GH_TOKEN`, and reading the owner's token for anything beyond that one line is not a round's to do.
So the line said *failure* and named `gh run view … --log-failed`, which a round cannot run.

**The repository is public**, and GitHub's REST interface answers four questions with no token:
the last run on `main`, its jobs, each job's failed steps, and each check run's annotations. Only
the step's log needs a token. The annotation is enough to tell the two kinds of failure apart —
`Process completed with exit code N` is a step that ran and failed; `The job was not acquired by
Runner of type hosted even after multiple attempts` is a job GitHub never ran. **Decision:**
`tools/state.sh main-checkout` prints that table (`ci_on_main`), and `round.sh`'s failing line
points at it. A report, never a failure of either script: no network prints "not asked".

## 2. What the failing runs were

Workflow `CI` (`.github/workflows/ci.yml`), on pushes to `main`; the last green run was on
`93f5e320`, and every run from `4adfa689` on has failed:

- **`raster-examples`, step "Every raster example's assertions execute (--check)", exit 134** — an
  assertion's abort. Reproduced here on the device and on llvmpipe: two examples fail, and both are
  the examples' premises going stale rather than the library regressing.
  - `outline_upload --check`: its witness asserts the processor lane and the device lane draw one
    picture, which holds only while `raster/`'s ADR 0026 triangle test declines every mark. A
    128-pixel box is 16 384 bytes of coverage; the witness's 32-cubic wiggles now convert to fewer
    triangles than that costs, so they take the device lane and 2 272 bytes differ. The witness
    now draws section B's 120-cubic marks, which the test declines.
  - `retained --check`: its signature compared a *warm* frame's counters with the recorded row,
    which is a cold device's first frame (`tests/archetypes.rs`). Since ADR 1517 a later frame
    reuses the residue meets the frame before it kept, so it rasterises none and counts 0 where the
    row says 2. The signature is now the first warm-up frame's.
- **`nightly`, step "Miri on the pure-Rust core", exit 1** — `continue-on-error: true`, so advisory
  and not what fails the run. Reproduced here, two defects of the tree's:
  `pdf-render`'s mesh raster asked `rayon::current_num_threads` on its serial path too, which
  brings the global pool up and lets Miri meet crossbeam-epoch's retag — ADR 0450's known
  dependency, now reached by a 32-pixel raster that divides nothing; the question moved inside the
  divided branch, and the two tests that do divide decline under Miri as `paint.rs`'s does. And
  `a_closed_subpath_is_joined_at_its_start` set a dash gap ending exactly at the path's start, so
  the answer sat on the last bit of four `hypot`s, which Miri perturbs on purpose (seeds 2 and 5 of
  eight failed); the gap is now one unit past it. With both, the step passes here: 253 and 121
  tests, exit 0.
- **`check`, `test`, `build (macOS)` on the newest run: not acquired by a hosted runner** — GitHub's
  capacity or the account's minutes. **The owner's**, not the tree's; the run before had all three
  green.

## 3. Which gate would have caught it here

None. Miri runs only in CI's advisory job. `cargo test` builds no example, and the merge recipe
runs no example's `--check`: the loop
exists only in CI, which is the gap ADR 0060 of `raster/` closed there and nobody closed here. A
change under `raster/crates/raster-gpu/src/` is a change to what those assertions read, so the loop
belongs in `doc/todo/02` section 2's change-to-gate map for `raster` — proposed for that file's
owner rather than edited here: `xvfb-run -a cargo run --release -p raster-gpu --example <e> --
--check` for each example `ci.yml` names, behind the lock.
