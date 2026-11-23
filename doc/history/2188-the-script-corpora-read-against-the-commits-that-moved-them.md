# Round 2188 — the script corpora read against the commits that moved them

A round in the `gwg` worktree, on the train's two field-script failures at `1d73fc0f6`. No code changed; Tier 0's held
file is restated by the walk's own update mode, and Tier 1's held figures stay.

## Tier 0 (ADR 3453)
- The 15 moved, each read per commit over the 356 held documents in a private worktree and bisected with a
  per-field print: `6dc0bc749` (ADR 2840's `/MaxLen` cut) moved four — `PDFBOX-4399-2`, `PDFBOX-4496-0` to
  `/MaxLen 9`, `poppler-95371-1`, `poppler-95391-0` to `/MaxLen 0`; `79d0c6c45` moved `poppler-22493-1` (two
  widgets defined at generations 11 and 14, referenced at 0: §7.3.10's null); `cca7b2660` (ADR 3311) moved ten,
  each one more value untaken: combo boxes whose own value names no `/Opt` label. All corrections; none fixed.
- 25 held and not on disk: `batch2/batch2/` on the notebook is `batch2/` in this machine's re-fetch. 51 unheld:
  those 25, and 26 under `batch4`, which the old held file names nothing of.

## Tier 1 (ADR 3454)

- The failure was the floor (21 096 runs against 21 101), not the ceiling. Fifteen runs went with `cca7b2660`'s
  untaken fields. The thirty throws went with no commit: all of the variance is `5712688.pdf`'s calculations,
  which ADR 1603's one-second calculation-order budget cuts shorter on a loaded machine (9 315 and 9 310 throws
  over the same 356 documents at two commits that read fields alike). At the round's head the whole population ran
  21 121 times, threw 9 315 and passed; neither held figure moves, and its `raise the floor` is not followed.

## Why the train saw it

Rounds run only the fast gates (owner, 2026-11-16), and `doc/todo/02` section 2 maps `script_corpus` to `pdf-script`
alone: the three commits changed `pdf-model` and `pdf-syntax`, and the train runs the scripts once a day. A round that
changes how a field's value is read, set or committed should run both walks — whole, 1.5 to 2.5 hours, so not a gate;
over the 356 held documents a minute and five, cheap enough if the walks could take that population (not built).

## Left, in order

1. Tier 1's figures move with the load through one document: a calculation-order budget counted in work, or the
   stopped documents counted apart from the floor (ADR 3454).
2. The synthetic commit sends a choice field its text; `Entered::Chosen` of its selection would run the ten
   fields' scripts again (ADR 3453). 3. A held-documents-only mode for both walks.

**Gates.** Over `-p pdf-model -p pdf-script -p viewer-qt`: fmt exit 0; clippy `-D warnings --all-targets` exit 0;
`pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` 3035 passed, 0 failed, `audit-diff.py` "failed 0,
expected 0"; `cargo test -p conformance` 478 passed, 0 failed. Both walks exit 0 at the head. No Rust changed.
Trap or habit: a `ratchet:` line is a measurement under the load it ran at; read it against the commits first.
