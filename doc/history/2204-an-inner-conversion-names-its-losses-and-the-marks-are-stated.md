# 2204 — An inner conversion names its losses, and the marks outside the tree are stated

Session 2204, an archive round in the `lane3` worktree: round 2202's leftovers. ADRs 3517, 3518,
3519. Ledger rows unchanged (14.8.2.2.1 and 14.8.2.2.2 `implemented`; no 6.1.13 row touched).

**Inner losses (ADR 3517).** `Conversion::losses` unions a conversion's authorised decisions' losses
with those rows record beside them. A PDF drawn or appended as pages carries its own run's losses
on `AttachmentOutcome::lost`, a derived file on `ConvertedTo::lost`; the report sentence and
`xmpMM:History` add "; this converter converted it first, losing …", the JSON
`losses_of_its_conversion`. The inner plan's authorisations are the outer plan's, so the one answer
decides both runs; an inner refusal now carries each refusal's sentence, naming the missing loss.
Witness: `digitally_signed_3D_Portfolio`'s outputs name `interactive-behaviour,
tagging-of-preserved-marks` for its inner PDFs.

**Statement page (ADR 3518).** `[profile] tagging-of-preserved-marks = "append-statement"`
(`Authorisations::tagging_of_preserved_marks_stated`) lists each annotation by row, loss, object,
subtype and page; `state-every-loss` now takes it so, `as-shown` keeps `discard`.

**R16 and 6.1.13 (ADR 3519, a reading).** `poppler-395-0-fuzzed`: `stand-in-face` under `as-shown`,
the signature under `state-every-loss` — refusals by name that stand. Of the thirteen 6.1.13
documents at 2b, `475121`'s forty 34-digit `m`/`l` operands can be written as reals of the same value
(owed as a build); the other twelve's values are damaged runs, `040669` and `427669` showing none past a
limit to a lexical scan of the decoded streams.

**Census.** With and without the pack, every line as round 2202's; `rank.py` prints the same table,
so sections 2 and 9 keep their figures (section 2's R06b and R16 rows gain the readings). Kept
outputs differ from 2202's for three documents only, each by the history's new clause
(`digitally_signed_3D_Portfolio`, `preEnc_20141025_105451`, `issue17056`). veraPDF: 5533 written as
conforming, 47 failing outputs of 15 documents, as before.

**Gates.** Over `-p pdf-archive -p pdf-transform -p viewer-qt`: fmt 0; clippy 0 (after two findings
fixed); rustdoc 0; `fuzz` 0; nextest 1748 passed, 18 failed, the manifest's 18; conformance 478 passed, 0 failed
(after this record and ADR 3519 stopped naming a scratch path).
