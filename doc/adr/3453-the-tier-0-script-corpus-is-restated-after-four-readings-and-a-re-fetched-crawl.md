# 3453 — The Tier 0 script corpus is restated after four readings and a re-fetched crawl

Session 2188. Status: **accepted and built.** In `pdf-model`: `tests/script_corpus.tsv`, rewritten by the walk's
own `PDFVIEWER_SCRIPT_CORPUS=update`. Clauses: ISO 32000-2 §7.3.10, §12.7.5.3 (Table 232), §12.7.5.4. Builds on
ADRs 0970 (a golden held by name in both directions), 1578 and 1579 (Tier 0), 2840 (`/MaxLen` in the model),
3311 (a value of a shape the field cannot hold is no edit of it). Amends nothing.

## Context

The held file was last written on 2026-10-27 (`4ad00b04e`), on the notebook. On this machine the walk first ran
with the corpus cache absent (2026-11-16: 200 held and not on disk), and from 2026-11-18 failed with 5 moved,
51 on disk and unheld, 25 held and not on disk; at `1d73fc0f6` the moved count was 15. No round runs the walk
(record 2188 says why), so each move sat until the train's daily section.

## What moved, and the commit that moved it

Read over the 356 held documents in a private worktree, per commit, and bisected with a per-field print of the
walk's own synthetic commit:

| documents | what moved | commit | reading |
|---|---|---|---|
| `PDFBOX-4399-2`, `PDFBOX-4496-0` (one file twice) | `stpfl.kl` shows `100'000.0`, was `100'000.00` | `6dc0bc749` | correction: the field states `/MaxLen 9`; Table 232 "The maximum length of the field's text, in characters", applied to a set value by A834 and ADR 2840 |
| `poppler-95371-1`, `poppler-95391-0` | `Phone` and `N number` show nothing, were formatted | `6dc0bc749` | correction, the same rule: both fields state `/MaxLen 0`, and the table names no exception for zero |
| `poppler-22493-1` | 163 fields to 161; `Ehegatte` and `eigener_Name_Geburtsname` gone | `79d0c6c45` | correction: the update defines objects 1167 and 1170 at generations 11 and 14, and every reference names generation 0 — §7.3.10's undefined object, read as null |
| `PDFBOX-923-0`, `-2`, `-3`; `FOP-2216-4`, `-5`, `-7`, `-8`, `-9`; pdf.js `issue15815`, `issue17998` | one more value untaken, digest unchanged | `cca7b2660` | correction: each is a combo box without bit 19 whose own value (`NA`, an export value, or empty) names no `/Opt` label, which ADR 3311 refuses as typed text |

The first five moved before the 2026-11-18 train and were what it reported; `cca7b2660` added the ten.

## The population

The 25 held and not on disk are `corpus-cache/tika-issue-tracker/batch2/batch2/…` (24 under `GHOSTSCRIPT`, one under
`TIKA`): the notebook's extraction had one directory level more than this machine's re-fetch (A882). Every one is on
disk one level up, and is among the 51 unheld. The other 26 are `batch4`'s (23 under `pdf.js`, 3 under
`LIBRE_OFFICE`), a directory the notebook's held file names no document of. The whole-population walk at this round's
head moved the same 15 the per-commit reading found, added 51 and removed 25.

## Decision

The held file is rewritten by the walk's update mode at the round's head, never by hand. The `untaken` column did what
it is for: ten fields a commit no longer takes are visible as themselves. That those fields' `/K` and `/V` no longer
fire under the synthetic commit is the cost; committing a choice field by its selection (`Entered::Chosen`) instead
of its text would restore them, and is left to a round that also reruns Tier 1.
