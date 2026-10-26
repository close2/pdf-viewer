# 2386 — The CJK pack is built by CI from Source Han fetched by digest, and published beside the binaries

Session 1776, a "host" round of batch one hundred and seventeen, 2026-10-26.
Status: implemented in `.github/workflows/ci.yml` (the `cjk-pack` job), `snapshot.yml`,
`tools/cjk-pack.py` and `archive::fonts::pack`'s test, and unrun on a runner: the round never
pushes, so the first CI run after the merge is the job's first execution. Context: the owner's A596
("agreed on all"), Q596's recommendation, A569's two downloads, ADRs 2302, 2303 and 2374. Cites no
clause and moves no ledger row.

## 1. What is published, and where

**`quorra-cjk-pack-<n>.zip` on the `snapshot` release, beside the four archives**, holding one
directory `cjk-pack/` — the five programs, `LICENSE.txt` and the pack's `SHA256SUMS` — which is the
directory ADR 2303's converter looks for beside `quorra-transform`. Beside the zip, two files: the
pack's own `SHA256SUMS`, the same file as inside, readable before 22 MB are downloaded, which is the
list of digests `pack.rs` carries; and `quorra-cjk-pack-<n>.zip.sha256`, the zip's digest, the form
every other archive of the release has. The owner's words name the first; the second is the
release's convention, and without it the zip would be the one asset with nothing to check it by.

## 2. Who builds it: one CI job, on every run

`cjk-pack` fetches fontTools 4.66.1's `py3-none-any` wheel by its SHA-256 into a virtual
environment of its own — fontTools stays out of every Cargo build (`doc/stack.md`) — then runs
`tools/cjk-pack.py --fetch`, which downloads the four Source Han Sans 2.005R subset programs and the
licence from the commit the `2.005R` tag names, and `SourceHanSans_JP_sequences.txt` from `master`'s
"Version 2.005 sources" commit (the tag's commit does not carry it), refusing each whose SHA-256 is
not the script's. The two files the script left unchecked — the sequences and the licence — now
carry digests too: "fetched by digest" is every input or it is not true. It then builds, zips, and
runs the pack's two tests with `QUORRA_CJK_PACK` naming the built directory: the new
`a_built_pack_holds_the_programs_whose_digests_this_file_carries` and ADR 2302's conversion test,
which until now nothing ran. Upload happens only on a push to `main` and only after both pass, so
`snapshot.yml`, which publishes only a run whose every job is green, never ships a pack this build
refuses.

**On every run, pull requests included**, because the failure it catches is a change: a builder or
a `pack.rs` that has left the other behind. A check that ran only after the merge would report the
skew as a missing snapshot rather than as the change that made it.

**Built once in CI, not twice.** Reproducibility was the reason a runner could be trusted to build
it (A596); the test is the proof of it on the runner, because it compares the runner's bytes with
digests computed on this machine. Locally the round built three times — fontTools from the
distribution's package twice, from the PyPI wheel the job installs once — and every file and the
zip were byte for byte the same.

## 3. How a number follows a pack

`PACK_VERSION` sits beside `SOURCES` in the builder, and is the next number for a new Source Han
release or a change that moves a byte of the output — which is when `pack.rs`'s digests change. A
change to the builder that leaves the bytes alone (this round's `--fetch` and `--zip`) is the same
pack, and stays `1`. The digests stay compiled in (A596), so an older pack beside a newer binary is
refused as not this pack whatever its number says; the number is for a person, and the digests are
for the program.

## 4. What it costs, and what the first run must show

About 23 MB downloaded and one to two minutes of build on one core, measured here; the rest is
`pdf-transform`'s test build, which a cold cache makes the largest part — the job's ceiling is 30
minutes. The first CI run after the merge must show: `cjk-pack` green, its build log printing the
six `--fetch` digests and the five `Adobe-*` lines, both tests `1 passed`, and the `snapshot`
release carrying `quorra-cjk-pack-1.zip`, its `.sha256` and `SHA256SUMS` beside the archives with
the notes' new paragraph. Not built: a pack attached to a tagged release, which this repository
does not make yet; `snapshot` is the only release there is.
