# 1463 — `open` warms only `dev`, the `conformance` sweeps leave the `release` profile, and a patch names its base

Session 1314. Status: accepted. Amends ADR 1451 section 2 (its reason for warming only `dev`,
replaced by the measurement below) and ADR 1440 (`main-checkout` gains a line). Code:
`tools/batch.sh` (`warm`'s comment), `tools/state.sh` (every `conformance` sweep),
`tools/main-checkout.py` (`section_signs`, `patches`, `pinned`, `patch_header`),
`doc/patches/hayro-jbig2-symbol-dictionary-bounds.patch` (its preamble). Prose:
`doc/environment.md` (*Build directory*, the `sccache` bullet, *After a merge* item 7),
`doc/HANDOVER.md` (the fork row).

## 1. What the three profiles cost in a batch, measured

ADR 1451 saw `release` and `gates` compiling dependencies beside the cold `dev` build, 24 children
each, and asked whether `open` should warm them too. Measured in this batch's directory on
2026-10-01, after batch forty-seven's merge, `time` from the shell, load 3 to 10 unless stated:

| build | wall | CPU | compiled |
|---|---|---|---|
| `--release -p render-raster --example frame_budget` | 82 s | 48 s | 9 workspace, 0 deps |
| `--release -p pdf-sandbox --bins` | 63 s (a lock wait) | 5 s | 1 workspace, 0 deps |
| `test --release -p viewer-ui --test launch_path --no-run` | 171 s | 255 s | 21 workspace, 0 deps |
| `--release -p conformance --bins`, after an edit to it | 127-130 s | 183-188 s | 1 workspace, 0 deps |
| the same under `dev` | 27 s (a lock wait) | 11 s | 1 workspace |
| `test --profile gates -p pdf-model --test corpus --no-run` | 21 s | 139 s | 14 workspace, 0 deps |
| `--profile gates --workspace --all-targets -j 8`, stopped at 25 min | 1 539 s | 25 875 s | 41 workspace, 415 deps, linking |

**The duplicated dependency builds were the first batch's alone.** The directory persists (ADR 1440),
so every `release` and `gates` build a round makes compiled no dependency at all; what a profile
pays in a batch is the workspace crates the merge changed, once, behind cargo's lock on that
profile, and again after each sibling's edit to a low crate. A warm at `open` saves the first
asker that wait and is thrown away by the first such edit — the all-targets `gates` build above
recompiled `raster` mid-run when a sibling touched it. **And all-targets is the wrong unit**: the
workspace has 369 test, 201 example and 42 binary targets, each a link of its own, and the `gates`
build had spent seven CPU-hours on them (with 415 dev-dependencies no gate had ever needed) when it
was stopped at load 150. Warming `--release --all-targets` would be dearer still, a fat link per
target. So `open` warms `dev` and nothing else, and `BATCH_WARM` keeps its two values. Disk, for
the record (`du -sh` per profile directory, after the stopped build): `debug` 78 GiB, `release`
6.5 GiB, `gates` 8.8 GiB — the warm's cost was never the disk.

## 2. The sweeps run under `dev`

The one `release` build every round makes is `conformance`'s binaries, through `tools/state.sh`
and `main-checkout`, and it is 26 fat links after any edit to that crate. Under `dev` (opt-level 1)
the same binaries run in 0.88 / 0.47 / 0.21 / 0.47 / 0.57 s against release's 0.62 / 0.36 / 0.16 /
0.27 / 0.42 s (`pointers`, `cited`, `overtaken`, `unread`, `names`), and the `dev` build is the one
`cargo test -p conformance` makes anyway. So `state.sh`, `main-checkout.py`, the binaries' own
headers, `doc/todo/02` and `doc/verify.md` name `cargo run -p conformance`. Files that still write
`--release` (`doc/todo/01`, two trap files, `doc/ledger-and-claims.md`, `doc/todo/48`) are other
owners' and run correctly, only slower to build.

## 3. `sccache` is serving, and it is full

A dependency built in two fresh target directories under `release` hit 12 of 12 the second time,
and a workspace crate with no dependencies hit on its second and third directory: the wrapper
serves across directories, as ADR 0344 says, because the directory is named in
`.cargo/config.toml`. But `--show-stats` ends with `Cache size 50 GiB` at `Max cache size 50 GiB`
(the default server's `/home/AI/.cache/sccache`): it evicts by age, and three profiles' outputs
plus every edited workspace crate's churn are what fill it — the all-targets `gates` build missed
918 times against 11 hits. Raising the ceiling is the account's configuration, not a round's;
`doc/environment.md` now says to read those two lines first.

## 4. A patch names the revision it was written against

`doc/patches/` holds fixes to a dependency pinned from a fork the owner controls. Each patch opens
with `Repository:` and `Base:` lines, which `git apply` skips as preamble, and `main-checkout`
lists a patch while the root manifest still pins its repository at its base: bumping the `rev` is
the patch applied, and it drops off. A patch without the preamble is listed as such rather than
dropped. Checked against a copy of the manifest with the `rev` changed: 0 owed, 1 no longer pinned.

## Costs

The sweeps cost a fraction of a second more per run. The decision rests on one day's figures under
loads nobody chose; the commands above re-take them.
