# 1647 — `DeviceCMYK` takes the operator's press, a lost `Trans` is removed, and the other corpora are swept

Slot 1 of batch one hundred and one, 2026-10-20, an archive round. ADRs 2130, 2131; no question.

**Built.** (1) `6-6-1-t01-fail-m`: the widget's `/AA` edit removing a `Trans` was tagged with the
`/AA` rule nobody failed, so the rewriter skipped it; it is now the type rule's (test red on HEAD).
The hypothesis (two visits) did not hold: `action_sites` deduplicates. (2) ADR 2130: a CMYK
`--output-intent-profile` beside the file's own intent of another family was silently unused; it is
an `ICCBased` `/DefaultCMYK` now, the operator's statement in report and history, for part 4's CMYK
row, part 2's ahead of the `DeviceN` default, and the group, base and alternate rows; unsupplied,
part 4's row names the flag (giving the group rows that sentence where the survey saw `DeviceCMYK`
refused 7 real 2b files the intent converts, and was taken back). (3) ADR 2131:
`xmp::remove_items`; complete descriptions stay. (4) The real-producers census, run in parallel.
The PostScript row is corrected.

**veraPDF, refused (as-if-printed / as-shown).** 2b 87/89 → 87/89; 4 94/97 → 93/96 (the `Trans`
witness). Group 1 (colour family) 7 at 2b, 15 at 4 before and after; with the shipped press profile
supplied: 0 and 0 (2b 82, 4 80 refused); two `6-2-3-t0?-fail-b` refuse only with it (ADR 2007).

**Other corpora, as-if-printed** (converted / conforming / refused / unreadable / source declined):

| corpus | 2b | 4 |
|---|---|---|
| pdf.js (976) | 618 / 2 / 336 / 11 / 9 | 570 / 0 / 386 / 11 / 9 |
| `doc/corpora` (503) | 179 / 6 / 290 / 16 / 12 | 157 / 0 / 318 / 16 / 12 |

Site-document pairs at 2b (pdf.js, corpora): a standard font stating no `FontDescriptor` 192, 168;
other embedding 48, 68; fence 69, 79; losses at `stop` 26, 24; header, 6.1.13 14, 26; colour 3, 0
(4: 9, 22). Declined: files only §C.4's recovery reads; no panic. Hypothesis wrong: unreadable plus
declined is 20 and 28 — the missing descriptor is the mass.

**Gates.** In the `mine` export (habit 138): `cargo fmt --check` and `-D warnings` clippy
`--all-targets` over pdf-model, pdf-archive, pdf-transform exit 0; nextest 3001/0 failed. Worktree
`cargo test -p conformance --no-fail-fast`: all ok but `batch` and `worker_features` (slots 5's and
6's worker). Censuses: one hold, exit 0, 249 s. `tools/state.sh --round 1647 archive` exit 0:
`--remedy-sites` 2b 23 of 68, 4 17 of 56; validator `missed` 0, `over` 0; `archive_corpus` 2b
525/86, 4 266/93; `cross_check` 0 disagreements. `docs` exit 0. `t3-foreign_corpus` `--clock` 0, 147 s.

**Unfinished.** A standard font stating no descriptor, the real corpora's largest refusal; without
a supply, `6-2-4-3-t04-fail-n` (`doc/todo/72` item 1). 5 900 s.
