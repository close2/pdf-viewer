# 2024 — The interpreter and the fuzzer are two jobs, each with its own ceiling

Session 1594, slot 3 of batch ninety-four. Status: **accepted** and **built** in
`.github/workflows/ci.yml`; not yet proven by a run, because a round does not push (ADR 0450's
"What this round could not do" still holds). Context: ADR 0450 (every job carries a ceiling, generous
against what it takes), ADR 0463 (two jobs lost to a size nobody had measured), ADR 2010 and
`doc/history/1587` "Unfinished".

## 1. What the one job took

CI's `nightly` job ran Miri over `pdf-render` and `pdf-syntax`, then installed `cargo-fuzz`, built the
fuzz targets and ran three of them briefly, all under one `timeout-minutes: 60`. GitHub's public
`actions/runs/<id>/jobs` endpoint (the one `tools/state.sh main-checkout` reads, no token needed)
gives each step's start and end. The seven green runs on `main` between 2026-10-12 and 2026-10-14:

| run | job | Miri | fuzz build | fuzz runs |
|---|---|---|---|---|
| 38241435033 | 3 278 s | 2 502 s | 210 s | 472 s |
| 38248537857 | 3 167 s | 2 465 s | 190 s | 446 s |
| 38259888623 | 3 160 s | 2 425 s | 185 s | 469 s |
| 38283475482 | 2 528 s | 1 916 s | 163 s | 360 s |
| 38319056420 | 2 372 s | 1 802 s | 153 s | 340 s |
| 38524996755 | 3 170 s | 2 415 s | 202 s | 445 s |
| 38631706413 | 3 262 s | 2 517 s | 190 s | 463 s |

The longest is 3 278 s of the 3 600 s ceiling, 91 %. Every run since failed inside Miri within
104–666 s on the undefined behaviour ADR 2010 removed, so the next green run is the first with the
tests added since. Round 1587 ran the job's own Miri line on this machine at 2 160 s for
`pdf-render` and 681 s for `pdf-syntax`, 2 841 s together, already past CI's longest Miri step, and
with the fuzz half after it that is past 3 600 s. ADR 0450's rule is that a ceiling makes a hang
legible; one the healthy run reaches is not a ceiling but a second failure mode.

## 2. Decision

- **Two jobs.** `miri` and `fuzz`, each with the nightly toolchain it needs, `continue-on-error`
  kept as it was, and the Miri step's comment (no `--skip`, ADR 0450) moved with it unchanged.
- **Miri is one job per crate**, a matrix over `pdf-render` and `pdf-syntax` running beside each
  other, so each ceiling bounds one crate's interpretation and the run's wall is the longer of the
  two rather than their sum. The two halves share a job id on one runner image, so the cache is keyed
  by the crate (`key: miri-<crate>`); otherwise the second to finish would save over the first.
- **Each ceiling is about twice the longest healthy run of what it bounds**: `pdf-render` 75 minutes
  against 2 160 s here; `pdf-syntax` 30 minutes against 681 s; `fuzz` 30 minutes against at most
  210 + 472 s plus a minute and a half of checkout, toolchain and cache. A job that reaches one has
  hung or doubled, and either is to be read before the number is raised.
- **The fuzz build is not moved into `check`**, which was the brief's hypothesis. `check` is the
  fast-feedback job on the pinned stable toolchain; the fuzz targets need nightly for the sanitiser
  and a `cargo install cargo-fuzz` each run, and would put about twelve minutes and a second
  toolchain in front of the formatting and lint verdict.

## 3. What it costs

Two more runner jobs a push (three where there was one), each building its own Miri sysroot or fuzz
build, a few minutes of runner time each; the run's wall falls from the sum of both halves to the
longest Miri crate. The union of the two matrix lines is the old line exactly, so trap 201's "CI's
whole line" is now both lines; `cargo +nightly miri test -p pdf-render -p pdf-syntax --lib` remains
the local way to run them together.
