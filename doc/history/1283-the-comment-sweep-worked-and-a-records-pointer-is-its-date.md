# 1283 — The comment sweep worked, and a record's dead pointer is its date

Batch forty-three, instruments slot. No ledger row moved: the contract named none. ADR 1403.

## The comment sweep (`CLAUDE.md`, ADR 1023)
- `tools/comment-history.py`, `tools/state.sh comments`: the grep as written, then each hit sorted
  into code, legitimate (a viewer's or desktop's session), history, and unread.
- Before: grep 2306; history 2154, unread 19, legitimate 54, code 79; `raster/` history 5.
  After: grep 226; history 62, unread 2, legitimate 83, code 79; `raster/` 0.
- Eight forks worked disjoint slices, comment lines only, each history sentence rewritten as the
  current reason with its ADR, or deleted. About 420 `.rs` files carry comment edits.
- Left: `pdf-archive` (24 lines, 1281's crate mid-edit); `pdf-model/tests/corpus.rs`'s
  `INCOMPLETE` ratchet log (38), which ADR 0730 keeps on purpose and 1282 is editing beside.
- Legitimate shapes taught to the sweep: the session bus, the session's clipboard, a Wayland/FUSE/
  KDE session, "this session attached", `Row::session`, "session bookkeeping", and a dozen more.

## The pointer sweep
- New rungs (ADR 1403): `Reach::Historical` for a dead pointer written in a record (`doc/adr/`,
  `doc/reviews/`, the `doc/QUORRA_*`/`HAYRO_*`/`quorra-*` correspondence); a pointer read from
  its sub-project's root (`raster/`) and from beside its file before it is called absent.
- Absent 265 → 11: 217 → 221 now historical; `raster/`'s own paths live. Fixed where written:
  `doc/todo/13`, `23`, `55` (moved files), `03` and 01's fourteen `doc/todo/20` (retired, ADRs
  0791 and 0169), RFC 0001 and 0006, `viewer-core/tests/headless.rs`, `viewer-accessibility/tests/
  tree.rs`, `oracle.rs`'s pdf.js path, six `raster-gpu` comments.
- The 11 left: nine `doc/questions/A169`/`A170` pointers (owner's answers not committed; their Q
  files are not here either), two §8.9.6.1 ledger corrections (not this round's row).

## The four documents after batch forty-two
- `state-of-play`: raster's set, cap tangent and ramp bounds (ADR 1389); the hard stop (ADR 1387).
- `crate-map`: `pdf-signature`'s own curves (ADRs 1385, 1386); `render-cpu`'s and `render-gpu`'s
  hard stop; `raster-gpu`'s fill set; the pointer rungs. HANDOVER: a row for the two curves.
- `tools/state.sh navigation` before/after: pointers absent 13 → 11; overtaken 24 → 26 (rewritten
  comments changed two notes' newest ADR — a reading list); four documents' history 0 → 0.

## Todo
- `doc/todo/65`: §7.6.5 is `reported` too, now said. `cargo search bp512`/`brainpool`, `cargo info
  ed448-goldilocks` (2026-09-29): no stable reviewed crate — RustCrypto has no `bp512`,
  `ed448-goldilocks` is 0.14.0-pre.15; `aegis-crypto` and `krypteia-arcana` are new, unreviewed.
- `doc/todo/README.md`: 14, 25, 53's lines made true. No file deleted: every done file (12, 13,
  14, 25, 26, 37, 39, 53) is cited from `crates/`, a ledger note or `CLAUDE.md`.
