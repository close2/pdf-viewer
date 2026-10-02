# 1500 — The disk is printed, and a batch's clock names what it did not sum

Session 1332. Status: accepted. Code: `tools/state.sh` (`disk`, `batches`). Prose:
`doc/environment.md` (the build-directory entries), `doc/todo/02` section 5, section 5a and
section 8 item 4.

## 1. The disk

The batch directory was 95 GB after one batch and 164 GB after two; the root it sits in was
857 GB, of which the main checkout's own directory was 637 GB (its `debug` 582 GB), and two
directories no checkout names held 57 GB. No instrument printed any of it beside the others, and
the one document that named the main checkout's directory named one nothing builds in.

`tools/state.sh disk` now prints, read-only: this tree's build directory and its reference-render
cache; the main checkout's directory, read from the configuration Cargo would read there (its own
`.cargo/config.toml`, else the user's) and never by running `cargo` in the owner's checkout; every
directory under the root with its `debug`, `release`, `gates` and `tmp` and the newest date among
its top two levels, so a directory no live tree writes is legible as one; `sccache`'s cache size
against its ceiling; the free space under `$HOME` and the main checkout; and `scratchpad/`.

**`sccache` is read off the disk, not out of `sccache --show-stats`.** The command starts a server
when none is running, which is a write a counting section may not make (ADR 1487). The cache
directory is `SCCACHE_DIR` or the default, the ceiling `SCCACHE_CACHE_SIZE` or the configuration
file's `size`. A cache at its ceiling evicts by age (ADR 1463) and is never pruned by hand.

**What to prune, and when, is the orchestrator's call, written as commands** in
`doc/environment.md`: the batch directory's `debug` at a batch boundary once it passes 100 GB; the
main checkout's `debug` and `gates` once that directory passes 100 GB; a directory under the root
that no configuration names and no live round writes. Never `tmp/`, which holds the reference
renders. The section prunes nothing.

## 2. A batch's clock

`state.sh batches` summed every `<n> s` in a commit's `Round durations` paragraph and counted
them. Batch fifty's message wrote one round as "264 s + 5 after the cut": the 5 had no unit, the
sum left it out, and nothing said so. The line now prints, per batch on one line, the sum, the
count of figures and the count of rounds the paragraph names (an entry opening with a session
number), and names any round entry with no `<n> s` and any `<n> s + <m>` whose second half has no
unit. `doc/todo/02` section 8 item 4 states the exact shape the orchestrator writes: one entry per
round, `<session> <n> s, <tool uses>`, a cut round as `<n> s + <n> s`.
