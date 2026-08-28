# 786 — Three RFCs: file-system faces, print, and editing in place

Design round, owner-commissioned, in worktree `r786` on branch `round-786` from
`b8f44a0c`. Date: 2026-08-28. Docs only — no implementation, no stubs, no crate touched.

## What the round wrote

Three proposals in `doc/rfc/`, all `Status: draft`, all carrying the owner's directive
that RFCs are not bound by current project rules (each names the relevant rule as a
current restriction with its rationale, then proposes the unconstrained design):

- **`0003-file-system-faces.md`** — a KIO worker and a FUSE filesystem over one shared
  `pdf-vfs` core: the virtual tree (`pages/`, `renders/`, `images/`, `text/`,
  `attachments/`, `meta/`), five write verbs mapped onto the transform layer (RFC 0002's
  seam, named abstractly as briefed), stated refusals for the rest, the ffmpegfs
  stat-size lesson, commit-on-flush, and the broker/confined-parser split (ADR 0713's
  pattern) as the sandbox posture. KIO shim in C++ over our C ABI (no Rust KIO binding
  exists); FUSE via `fuser`, pure Rust. Recommendation: core + FUSE first, KIO second.
- **`0004-print-and-print-preview.md`** — Route B recommended: `render-cpu` (the oracle)
  renders the printed page, banded, at the printer's resolution clamped to a stated
  budget; print as an interpretation *intent* that finally asks Table 167 bit 3,
  §8.11.4.5's Print event and §12.5.6.22's FixedPrint — with a table of what the ledger
  already says about each row this makes live. Preview as a view mode of the existing
  window in all hosts; entry per host (GtkPrintOperation / QPrinter via new bridge code /
  a winit chrome panel + IPP / worker-renders-host-spools in the confined window).
- **`0005-text-editing-without-reflow.md`** — the honest v1: an edit mode, per-glyph
  editability decided from the file (embedded-subset glyph presence, standard 14),
  named refusals for what a font cannot spell, overwrite/extend/delete within the line's
  box (recommendation at the boundary: refuse, not clip), save as a byte-spliced
  replacement content stream through the transform layer, and the font-subset-extension
  cost priced and excluded from v1.

Web research was done for prior art (KIO/kio-fuse/fuser/ffmpegfs; GTK/Qt/CUPS/portal
print paths; Okular/Evince behaviour) and is cited by URL inside the RFCs, in the
prior-art register, kept separate from the spec-truth register throughout.

## Two things the briefing got wrong, and the tree won

- The briefing cited the annotation Print flag as "Table 165's bit 3" — that is ISO
  32000-1's number. ISO 32000-2 §12.5.3 puts the flags in **Table 167**; the RFC cites
  the tree's own standard.
- The briefing allocated RFCs 0003/0004/0005 to this round; round 784's parallel
  `doc/rfc/README.md` reserved 0003 and 0004 only, with print and editing combined in
  0004. This branch follows its briefing (three files); the split and the index are the
  merge round's to reconcile, and every cross-reference in the three RFCs names
  companions by title as well as number so a renumbering is one-line-per-reference.
  This branch deliberately adds no `doc/rfc/README.md` of its own.

## Gates (docs-only map: core + conformance)

Run in this worktree after the final edit, quiet machine:

- `cargo fmt --all --check` — clean.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — clean.
- `cargo nextest run --workspace` — 2726 passed, 18 skipped.
- `cargo test --workspace --doc` — ok.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — clean.
- `cargo test -p conformance -- --nocapture` — all green (11629 citations, 1092
  quotations verbatim).

## Sweeps, before/after a pristine baseline (deltas accounted)

Fourteen argument-less sweep binaries run before any edit and again after
(`tmp/sweeps-{baseline,after}-786.log`, not committed). Deltas, all from the three new
documents:

- **pointers**: +66 unrooted, +3 form — the RFCs' virtual-tree examples
  (`pages/0001.pdf` and kin), the sweep's benign categories. One **absent** hit appeared
  (`fuzz/validation`, a slash in prose parsed as a path) and was reworded to zero; the
  absent count ended at the baseline's 98.
- **quotations** (documents): +11 verbatim, +16 "sharing too little to be a quotation"
  (ordinary quoted phrases: UI messages, other tools' option names), **0 new
  diverging**. Every specification quote in the RFCs was verified verbatim against
  `doc/md/` before writing.
- **tables**: +2 attributed key citations, both mine — Table 100's `Print` and
  `PrintState` in RFC 0004 — landing in "under a table that states no entries", the
  sweep's documented conversion-noise class (Table 100 nests those entries inside a
  cell). Both checked against the standard's own text at the clause; both correct.
- All other sweeps: byte-identical summaries.

ADR 0722 was reserved for this round and not used: a design round that decides nothing
writes proposals, not decisions.
