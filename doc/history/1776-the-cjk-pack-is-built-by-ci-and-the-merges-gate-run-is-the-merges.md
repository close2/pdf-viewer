# 1776 — The CJK pack is built by CI and published beside the binaries, and the merge's gate run is the merge's

Slot 6 of batch one hundred and seventeen, 2026-10-26, a "host" round on A596 and trap 270; ADRs 2386,
2387; Q642 unused. I worked in `scratchpad/r1776/`. No ledger row moves.
**Built.** (1) `ci.yml`'s new `cjk-pack` job (ADR 2386): fontTools 4.66.1's `py3-none-any` wheel
fetched by SHA-256 into a venv; `tools/cjk-pack.py --fetch` downloads Source Han Sans 2.005R's four
subset programs and licence from the `2.005R` tag's commit and the JP sequences from `master`'s
"Version 2.005 sources" commit, each refused unless its digest is the script's (the sequences and the
licence had none); the build, `--zip` writing `quorra-cjk-pack-<n>.zip` (`cjk-pack/` inside, fixed
entry times, `SHA256SUMS`'s order); the pack's two tests on the built directory; `SHA256SUMS` and
`<zip>.sha256` beside it; upload as `snapshot-cjk-pack` on `main` only. It runs on every run, pull
requests included. (2) `snapshot.yml` publishes `./SHA256SUMS` with the archives and the notes carry
a paragraph on the pack. (3) `pack.rs`: an ignored test, `a_built_pack_holds_the_programs_whose_
digests_this_file_carries` — `SHA256SUMS` lists exactly the compiled digests and the licence's in
`Collection::ALL`'s order, every file hashes to its line, and `CjkPack::program` accepts each.
(4) `PACK_VERSION = 1` in the builder: the next number only when the output's bytes move.
(5) `tools/batch.sh gates` refuses without `--merge`, exit 2, before the log is touched (ADR 2387).
**The merge's invocation changes**: `tools/batch.sh gates --merge`; `doc/todo/02` §8 and
`doc/orchestrator-handover.md` say so. Docs: `stack.md`, `running-the-viewer.md`, `cjk-pack.md`.
**Hypotheses.** "One job under ten minutes": unmeasured here; fetch and build took 66 s to 113 s
locally, so `pdf-transform`'s test build decides it. **Premise that did not hold**: the brief's "the four Source Han files fetched by digest" —
the build reads six, and two of them were checked by no digest; the sequences file is not at the
tag's commit at all.
**What the first CI run after the merge must show**: `cjk-pack` green, its log with the six fetch
digests and five `Adobe-*` lines, both tests `1 passed`; the `snapshot` release carrying
`quorra-cjk-pack-1.zip`, `quorra-cjk-pack-1.zip.sha256` (`631d37c1…` if the runner's zlib deflates
as this machine's does) and `SHA256SUMS` beside the four archives.
**Gates.** Pack built three times — the distribution's fontTools twice, the PyPI wheel once from
`--fetch`'s files: all seven files byte-equal, the digests `pack.rs`'s; the zips of two byte-equal; the job's build and package steps run from the YAML under bash with
`RUNNER_TEMP` in my scratch: exit 0, the same zip. In an export of HEAD and my `pack.rs` hunks
(siblings' `pdf-model` mid-edit broke the shared build, habit 138): the new test against the built
pack 1 passed; planted with one byte of `Adobe-KR.otf` changed, exit 101 at that file; with
`SHA256SUMS` rewritten to match it, exit 101 at the listing (trap 13); ADR 2302's ignored conversion
test 1 passed; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform --all-targets -- -D warnings`
exit 0, `cargo fmt --check -p pdf-transform` exit 0. `cargo test -p conformance --no-fail-fast`
exit 101 on four tests: `records`, mine, until this record was cut to 40 lines; §14.8's rows (two
tests) and `media/deprecated.rs`'s quotation, siblings'. `bash -n tools/batch.sh` exit 0; `gates` refused exit 2 with
the log absent after, `gates --merge` reached `gates()`. **actionlint and shellcheck not run**: the
download of their release binaries was refused by this session's permission check; in their place
both workflows parse under PyYAML 6.0.3 and every bash `run:` block passes `bash -n`, exit 0.
