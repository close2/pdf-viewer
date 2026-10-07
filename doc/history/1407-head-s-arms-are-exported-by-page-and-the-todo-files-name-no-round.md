# 1407 — HEAD's arms are exported by page, and the todo files name no round in words

Slot 6 of batch sixty-three, 2026-10-07, an instruments round. ADR 1650. No ledger row moved; no
question written.

**Premise.** `TODO_CEILING = 2` and `TODO_NUMBERED_CEILING = 119` held, and so did the two spelled
lines in `doc/todo/56`. The seventeen files with digit-form lines include `56`, which carries three
of them. The brief gave this round only `56`'s two spelled lines, so the numbered ceiling is 3, not
0: the three are its §12.11.5 row (lines 732, 747) and RFC 0008's commissioning line (1177).
Records 1395–1401 were already at most 40 lines with `**Gates.**`, and `records.rs` holds that.

**The hand export, read before building on it.** Three defects. It wrote no page's digest: the
gate's output names only the pages that differ, and the digests are in the `PDFVIEWER_RASTER_TIMES`
file. It was not one commit: each arm was its own `cargo test` in the shared worktree, and
`gpu-4x.txt` opens with `Compiling pdf-model`, a sibling's edit. And its `cpu-4x` passed but was
logged `exit 1`, because round 1405's `tools/bounded.sh` was mid-edit
(`line 422: dren: command not found`). The orchestrator's corrected re-run went into the same
directory at the same time as this round's first export and deleted that export's copied binaries
(every arm `exit 127`). The orchestrator stopped its run and handed the directory over.

**Built.** `tools/batch.sh arms [<dir>]`, and `open` starts it detached after `warm`. It refuses a
worktree that is not clean, a branch whose name gives no first session, and a directory holding
another commit's export. It builds the test binary and the worker once, copies them and records
their SHA-256, and runs all six arms on the copies under one hold of the lock, two files per arm.
`tests/batch.rs` covers the four refusals and the keep (calibrated: a skipped dirty-tree check fails
it). Its gate-line reader skips `--no-run`. `/home/AI/arms-1402/` holds the export, taken from a
detached checkout of `0f9c12cf`: six arms, all `ok`, 968 page lines per 1x arm and 963–964 per 4x
arm. The lock was held 1 864 s. `cpu-1x` and `gpu-1x` differ on 125 pages.
`doc/todo/02` section 8 step 1 names the export.

**Rewritten.** 114 digit-form lines in sixteen todo files, and `56`'s two spelled lines, now state
what is, each with its ADR. `TODO_CEILING` is 0 and `TODO_NUMBERED_CEILING` is 3.

**Gates.** `bash -n tools/batch.sh`: exit 0; rustfmt on both test files: exit 0;
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`: exit 0;
`cargo test -p conformance --test batch`: 14 passed; `--test spelled_ordinals`: 0 spelled and 3
numbered lines under `doc/todo`; `cargo test -p conformance`: exit 0, 53 test binaries ok, the
frontier map green after slot 5's moves (`doc/todo/65` read against the five rows' notes: current);
`tools/batch.sh check`: exit 0, every line `none`, `all 7` or `clean`.
