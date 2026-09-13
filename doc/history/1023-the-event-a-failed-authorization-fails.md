# 1023 — The event a failed authorization fails, and the tree nobody walked

Date: 2026-09-13. Branch: `batch-1020-1025`, worktree shared with five siblings (1020–1022 in
§12.8). ADR: [1040](../adr/1040-the-event-a-failed-authorization-fails.md).

| row | was | is |
|---|---|---|
| §7.6.6 crypt filters | `partial` for `/AuthEvent` | `partial`, Table 27 alone — §7.6.5's handler |
| §14.12.4 data structures | `partial` | `implemented` |
| §14.12.4.1 general | `partial` for Table 408 | `implemented` |
| §14.13.8 files linked to DParts | `partial`, no caller | `implemented` |
| §14.13 associated files | `partial` | `implemented`, its last child settled |

**§7.6.6 — `/AuthEvent`.** Nothing read it. Table 25 decides it in the sentence ADR 0031 did not
quote: "[i]f authorization fails, the event shall fail". `crypt.rs` reads it per `/CF` entry
(`AuthEvent`, `CryptFilters::key_at_open`), holds a filter named by `/StmF` or `/StrF` to
`DocOpen` as Table 25 requires, and defaults an unknown value to `DocOpen` — the stated default,
and the direction that can only ask for a password rather than open without one. So
`encrypted-attachment.pdf` stops opening and `auth-event-ef-open.pdf` keeps opening; the two are
the same bytes but for that line, and no reference distinguishes them. Five lists moved with it:
`oracle.rs` (into `NO_RENDER_NEEDS_A_PASSWORD`), `save_round_trip.rs` (into `REFUSED_OPEN`),
`corpus.rs` (`MAX_LOCKED` 9 → 10, by argument), `collections.rs` (23 → 22 embedded files) and
`raster_golden.tsv` (that page `drawn` → `locked`). Three tests in `encryption.rs`, three planted
defects, three distinct failures (trap 13).

**§14.12.4.1 and §14.13.8 — the `/DPartRoot` walk.** `document_part::hierarchy` reads Table 408
and enumerates §14.12.2's tree depth first; `attachment::attachments` appends every part's `/AF`
files to §7.7.4's list, which gives §14.13.8 a caller — ADR 0295's shape, one family over.
§14.12.2's one-parent rule bounds the walk, and `witness_census DPartRoot DPart` reads 1479 files,
opens 1452 and answers zero at all three layers, so every fixture is hand-built (trap 4) and
calibrated against the defect it should find.

**Gates, and the launch-path question put to this round.** Tier 1 and all of tier 2 pass, plus
`text_extraction` (503 judged, 8563 cross-axis), run because the change moves a document out of
its population. **`launch_path` passes here — 4325 KiB read, 18 MiB resident, 21 figures banded,
0 outside.** The 22 945 KiB / 32.7 MiB failure reported to this round was isolated by neutralising
this round's one launch-path-reachable line and re-measuring the deterministic figure: **185 346.0
against 185 365.9 thousand instructions, 0.011 %, read and resident identical to the byte** — one
absent catalog lookup, which is what a `/DPartRoot` costs a document stating none. What does fail
in this worktree is `viewer-core/src/notes.rs`, which is a sibling's file and untouched here.
