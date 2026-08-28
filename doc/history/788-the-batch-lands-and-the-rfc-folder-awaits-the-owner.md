# 788 — The batch lands, and the RFC folder awaits the owner

Merge round, 2026-08-28, on `main` from `b8f44a0c`. Merged `round-784` (`d34d4eae`),
`round-785` (`5534402d`), `round-786` (`5d59ac94`), `round-787` (`9f93ab38`), in round
order, each with `--no-ff`; all four clean, including 784's deliberately older base
(`28ed2239` — it branched before 783 landed, and `doc/rfc/` was new territory so the ort
strategy had nothing to reconcile). Then one reconciliation commit (`48576e86`), the full
§2 sequence, §5's install, the §4 sweeps against a pre-merge baseline, and the worktrees
closed.

**The round's headline outcome: `doc/rfc/` now holds the finished five-RFC set, and it
awaits the OWNER'S REVIEW.** Nothing in it is decided; every status is `draft` except
0001's `proposed`, and the README gives the owner alone the word that moves one.

## The reconciliation

784 wrote the RFC index before its siblings finished and reserved 0003 for KIO/FUSE and
0004 for print-plus-editing as one document; 786 wrote three documents instead (0003
file-system faces, 0004 print and preview, 0005 text editing). Resolved by letting the
**index bend to reality**: the files keep their numbers, the README's index now lists the
five RFCs that exist with each row's title taken from the file's own title line. Also
settled, as the headers themselves anticipated: 0001's arc sentence names 0003–0005 as
three documents; 0002's provisional header note (written before 784's conventions existed,
addressed to this merge round) replaced by the standard header shape — no number collided;
0003/0004/0005's "numbering may be reconciled by the merge round" sentences now record
that it was; and 0003's citation of 0004's 300 dpi print-grade default read "§5" where
0004 states its DPI policy in §3 — corrected. Cross-references between the five files were
walked after the merge: every number, title and section reference now resolves.

## The ADR decision

0720–0722 were reserved for 784/785/786; none wrote an ADR, correctly — an RFC proposes
and decides nothing. 787 took 0723. `doc/adr/`'s sequence is **not** dense — the listing
shows gaps throughout (0448→0450, 0694→0697, and dozens more), so reserved-but-unused
numbers are the tree's existing convention and **0723 stays 0723**. Verified: `main`
ended at 0719 pre-merge, and 0723 is the only new number — no collision.

## Gates (full §2 sequence on merged `main`, quiet machine)

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy, `-D warnings`, all targets | clean |
| `cargo nextest run --workspace` | 2728 passed, 18 skipped — ≥ 787's 2728, as expected |
| doctests | ok |
| fuzz `check`, `-D warnings` | ok |
| pdf-model corpus | ok |
| oracle | ok — 61 contradicted, unchanged from 787's branch |
| text_extraction (three gates) | ok — 10969/11163 words in bounds (98.26%), 486/508 documents |
| selection_census | ok |
| accessibility_census | ok — panicked: 0 |
| dates, xmp, jpeg2000 | ok |
| render-quorra corpus | ok |
| fixed_documents | 40 checked, 0 absent, 40 rows |
| `cargo test -p conformance` | 200 passed |

The launch test ran inside nextest and passed; load-robust since 776, so a pass is the
expected news.

## §5

All eight release artifacts rebuilt and installed into `main`'s `target/`: `pdf-viewer`,
`pdf-sandbox-worker`, `pdf-view-worker`, `pdf-viewer-gtk`, `pdf-viewer-qt`,
`pdf-viewer-confined`, `pdf-retrieve`, `libviewer_ffi.so`, from the build directory
`cargo metadata` names.

## §4 sweeps, against a pre-merge baseline

Pointers and quotations were run on `b8f44a0c` before the first merge and re-run after
the reconciliation commit, §5's install and this file's own creation, so the post
figures include everything the round adds. Deltas, every one accounted:

- pointers: 8740 → 8925 path pointers (+185: live 5052 → 5081, unrooted 2877 → 3028,
  a form 189 → 194); **absent 98 → 98, in another crate 22 → 22, not carried 502 → 502,
  symbol pointers 157 with 13 undefined — all unchanged.** The growth is the six
  `doc/rfc/` files, ADR 0723, the amended `doc/todo/41`, and the five history files —
  the union of what 784/785/786 each accounted in their own worktrees, plus the
  reconciliation's edits, none of which added or retired a pointer target.
- quotations: 6580 → 6648 quotations in 1049 → 1061 documents (+68 in +12 documents:
  five RFCs, the RFC README, four sibling history files, ADR 0723, and this file);
  verbatim 2772 → 2783, **diverging 38 → 38 unchanged**; ledger-note quotations
  1969/1505 verbatim/2 diverging — byte-identical, as they must be: the reconciliation
  touched no ledger row.

## The batch, synthesised

- **784** — the survey (RFC 0001, `proposed`), owner-commissioned, and `doc/rfc/` born
  with its conventions. Headline findings: the demand centre is document transforms, and
  nearly every easy-or-moderate gap is gated on one missing piece — a whole-file writer;
  printing is demand-backed pain in both major Linux viewers, and a good print preview is
  this tree's existing core competence; every surveyed viewer is growing into an editor
  under user pressure, annotations first; KIO/FUSE have no tracker demand and real
  integration value, argued on that register honestly.
- **785** — RFC 0002, the transform suite: one CLI over one library crate whose public
  API — the transform seam — is what 786's designs sit on. Writer recommendation: a
  structure-preserving whole-file serializer in `pdf-syntax` (the qpdf shape;
  re-distillation rejected on fidelity grounds), with `pdf_syntax::Document` staying
  immutable — a transform builds a new file from immutable sources, so the oracle-purity
  argument survives the whole suite. §11 of the RFC is the ratifiable amendment.
- **786** — three interface RFCs: 0003 file-system faces (KIO worker and FUSE filesystem
  over one `pdf-vfs` core; FUSE first, KIO second), 0004 print and preview (Route B: the
  CPU oracle renders the printed page, banded; print as an interpretation intent), 0005
  text editing without reflow (per-glyph editability decided from the file, named
  refusals, save through the transform layer).
- **787** — the batch's general-improvement round (code): `annotation::appearance_damage`
  takes the decided `Content`, so deciding an annotation no longer pumps a windowed
  appearance stream the draw will read anyway — ADR 0586's bomb −6.63% instructions, a
  benign 5.24 MiB appearance −28.2% per interpretation, ISO 32000-2's 1023 pages +0.0025%
  with identical command totals. ADR 0723; §12.5.5's ledger row amended; two new tests.

## Owed, standing

- **The RFC folder awaits the owner's review** — the round's headline outcome, said
  plainly: five documents, nothing decided, the owner is the decider.
- CI verdict awaits the owner's push; origin/main's red is pre-existing (owner-arc
  commit) and `main` here is far ahead and unpushed, deliberately.
- QUORRA_FEEDBACK §40 pending.
- `doc/todo/15`'s remainder.

Worktrees r784–r787 closed with `tools/worktree.sh close 784 785 786 787`, checkouts and
build directories together; verified with `list`.
