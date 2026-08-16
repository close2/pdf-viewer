# 553 — The seventh sweep becomes a program, and the two ADRs a correction stopped short of

**Finding.** A sweep round under `doc/todo/01`'s binding rule, pointed at a hazard rather than at a
population: **all eight rounds since the last sweep corrected an earlier ADR in place**, and the
sweep before this one had found that six of nine document defects were a round's own correction
that stopped at the code. So the fourth sweep was run over those rounds' own nouns, and **two ADRs
were carrying a claim a later round had already disproved**. ADR 0382 §6 still said "[t]he escape
hatch is complete and needs nothing from upstream" — a presenter rendering into a `Target::Texture`
it owns — five rounds after ADR 0383 established that presenting that texture needs the surface,
that `quorra_gpu::Device` keeps it private, and that a host configuring one of its own needs a
format only a `&wgpu::Adapter` gives. ADR 0384 §6 still said an atlas repack's lost reprojection
"cannot be worked around from here", three rounds after ADR 0385 showed every clause of that is
true about *capturing* and a non-sequitur about *drawing*. The first is the sharper because the
sentence is a decision's own conclusion: a reader of 0382 who never opens 0383 comes away believing
this tree has a working escape hatch it does not have. Both are struck and re-argued in place.

**Second finding, the sweep that became a program.** The seventh — the only one that reads the
`inapplicable` rows, and the population that let five wrong reasons sit undisturbed until ADR 0205
— is `conformance --bin inapplicable` now. Its nine hand-runs printed 25, 64, 72, 71, 27, 66, 43,
61, 57 and 47 of about 80 while the population barely moved, because each session wrote its own
stop-list; the program counts the **naming files** instead, which is a property of the ledger and
the tree, and prints each named term with any **cousin row** that is not `inapplicable` and says
the same word — the pair all five of this sweep's defects have been. Two derived rules replace the
word list in the extraction (an inner capital makes an identifier anywhere; a plain `Capitalised`
word counts only where it does not open a sentence), which took the first run from 440 stated terms
to 305. **Its one defect is §14.5**, `inapplicable` because "[n]othing here writes a PDF" — the
exclusion wording `CLAUDE.md` amended in session 137 and ADR 0345 corrected in the ledger's own
generated header — found because `tests/saving.rs` names `/PieceInfo` under it.

**Third, from the blame band, and it is one count in two rows.** §12.4 and §12.6.4.15 both said
five of Table 164's twelve transition styles are reported by name. Four are: `R` is the cut, and
`transition::note`'s own doc comment says "`R` is the one style with nothing to report". **ADR 0230
says both** — "[t]he other five are named and **reported by name**", and twelve lines later "`R` is
therefore not reported and the other four are" — so the two rows were written from the wrong half
of a document that had already corrected itself.

**Date.** 2026-08-16.
**ADR.** [0388](../adr/0388-the-seventh-sweep-becomes-a-program-and-the-two-adrs-a-correction-stopped-short-of.md).

**Sweep results, verbatim from the runs.**

- `inapplicable` (new): **80 `inapplicable` rows stating 305 terms — 60 named by no source, 245
  named over 72 rows, 231 of them carrying a cousin row.** 1 defect (§14.5), plus one by-catch
  about the file rather than a claim: §Q's note read "one question — does this page contain
  transparency — and The annex states", a sentence spliced by an append, which is what the
  sentence-position rule printed.
- `retired`, over the wave's fourteen nouns (`Stale`, `Base`, `composed`, `Plan::Reproject`,
  `Cadence`, `approximated`, `device_pixels`, `Path::walked`, `last_phases`, `bytes_uploaded`,
  `readback`, `detach_presenter`, `Presenter`, `RasterCache`): 2036 mentions, 10 carrying both
  shapes, 0 defects — 1445 of the 2036 are `Base` and `readback`. **Run again over the corrections'
  own nouns** (`SHARE`, `ASSUMED`, `TooDear`, `present_texture`, `capture_presented`,
  `atlas_repacked`, `Settled`): 1264 mentions, 5 carrying both shapes, **2 defects** — ADR 0382 §6
  and ADR 0384 §6.
- `tables`: 409 tables captioned, 305 stating entries; 4698 sentences name a table; 1849 attributed
  key citations — 1710 the table agrees with, 89 absent, 4 a denial the table contradicts, 46 under
  a table that states no entries, 0 under no such table. **0 defects**; the absent are up from 75
  because the five-hundred-and-forty-fifth wrote its own record of every number it retired.
- `pointers`: 5013 path pointers — 2791 live, 97 absent, 14 in another crate, 1738 unrooted, 118 a
  form, 255 not carried; 52 symbol pointers, 12 undefined. 0 defects.
- `blockers`: ledger 22 — 6 expired, 10 holding, 6 naming no clause; source 27 — 10, 10, 7. 0
  defects; the extra ledger sentence is 545's own "[r]ead and kept" line on §12.5.6.22.
- `capabilities`: ledger 52 — 37 witnessed, 45 program, 7 crate; source 151 — 124, 84, 67. 0
  defects.
- `unread`: 62 rows claim, 173 keys; 49 confirmed, 124 quoted over 52 rows, 54 by the row's own
  code — **the five-hundred-and-forty-fifth's five numbers exactly**. 0 defects.
- `entries`: 244 rows explain themselves by an arrival and name code, 1 names none; 756 entries
  stated, 174 reported over 46 rows — 41 named nowhere, 133 only elsewhere, 39 not named by the
  row's own note. Known populations.
- `quotations`: 3574 quotations in 571 documents, 1613 verbatim, 24 diverging; 1394 in 794 ledger
  notes, 1089 verbatim, 1 diverging. 0 defects.
- `callers`: 299 distinct `pub fn` names in `pdf-model` (296 in the five-hundred-and-thirty-seventh),
  122 that no crate under `crates/` asks, 177 named by a dependent crate, 80 only inside
  `pdf-model`, 21 only by a test or an example, **1 by nothing at all** — down from two.
- Arithmetic (6): §7.9.2 and §O, read and kept before. Clean. Parent counts (10): 41 counted claims,
  6 matching neither the children nor the descendants, 0 defects. Sweep 14: 19 hits under a
  session-local vocabulary, every one naming its debt in other words. Errata (12): "151 struck
  passage(s) of 4 words or more that doc/md/ still carries as current text" over all fourteen PDFs,
  unchanged, and 71 quotations quoting struck text — the same 71 as the five-hundred-and-forty-fifth.

**Rows corrected in this commit.** §12.4 and §12.6.4.15 (four transition styles reported, not five,
and why `R` is not one), §14.5 (the retired exclusion wording replaced by the clause's own narrower
reason), §Q (the spliced sentence). Kept with evidence recorded: §12.8.3.4.3, §12.8.4, §12.8.4.4,
§12.8.5.2, §12.1, §12.3.5, §12.3.5.1, §12.6, §12.6.4 — nine of the eleven the band from commit 534
to 536 held.

**Documents corrected.** ADR 0382 §6 and ADR 0384 §6 (struck and re-argued, each naming the ADR that
disproved it), ADR 0230 (the sentence its own table contradicts twelve lines later, and the two
ledger rows written from it).

**Code.** `tools/conformance/src/inapplicable.rs` and `src/bin/inapplicable.rs` (new, nine unit
tests); `tools/conformance/src/lib.rs` (the module).

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's record,
the band pointer advanced to commit 541, eleven commands now, the fourteenth sweep named as the next
one a program should take), `doc/todo/02-every-round.md` §4 (the new command's line),
`doc/ledger-and-claims.md` (ten programs → eleven, and the sweep named beside the shape it found),
`doc/adr/0388-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of lints
(the `viewer-qt@0.1.0:` lines are gcc's on a cold build, which `doc/todo/02` §2 names);
`cargo nextest run --workspace` **2049 tests run: 2049 passed, 15 skipped**, against 2040 before —
the nine are the new sweep's unit tests; `cargo test --workspace --doc` green;
`cargo test -p conformance -- --nocapture` 5 passed over 875 subclauses. No corpus or oracle run:
`git diff` shows no line under `crates/` at all, so nothing this round changed can reach a raster.
