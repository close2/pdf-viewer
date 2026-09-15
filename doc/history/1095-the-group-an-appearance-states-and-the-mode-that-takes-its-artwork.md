# 1095 — the group an appearance states, and the mode that takes its artwork away

Date: 2026-09-15. No ADR — both halves are what the clauses say. Two §12.5 residues, neither
Q63-shaped; `content/annotations.rs`, `content/transparency.rs`, `annotation.rs`, the annotations
tests, `push_button_census.rs`, and two ledger rows whose status did not move.

**Censuses first (trap 8), and both negatives are the corpus's alone.** Of the curated 1452
documents' 48 191 annotations, **5 appearance streams state a `/Group` — 0 isolated, 0 knockout —
and none states a non-Normal `/BM`**, against 143 groups, **95 isolated, 1 knockout** and 8
`/BM /Multiply` over `CC-MAIN-2021-31`'s 65 720. Of the corpus's 833 widgets 7 state an `/H` and of
the crawl's 65 410, 9172 do; **not one in either states a `/D` that a mode other than `P`
overrides**, against 13 268 stating a `/D`.

**§12.5.5, built as one construction rather than two.** The clause makes *every* stored appearance a
group and lets the entry decide only which, so `transparency::appearance_group` answers with the
stated group or the non-isolated, non-knockout one the clause names in its own words, and
`draw_appearance` enters `run_transparency_group` with `state` as both inner state and composite:
§11.6.6 resets the blend, both alphas and the mask on the inner copy, and the outer carries `/BM`,
the opacity 1.0 Table 166 and §12.5.2 leave a stored stream, and a soft mask of `None`. §11.4.4's
NOTE 5 then decides whether a group is built at all, on its own two conditions rather than on this
path's assumption — so the no-`/Group` case flattens as before and the non-Normal `/BM` case, which
used to blend per element, composites once; `note_appearance_group` gives way to
`note_group_departures`.

**§12.5.6.19's residue was stale and the row's own boast was the defect.** The dispatch it called
missing is §12.6.3's, `implemented` since session 257; what was actually broken is Table 191's "[a]
highlighting mode other than P shall override any down appearance", which the row claimed and the
code did not do — `highlight` decided the *mark* while `stored_appearance` went on selecting `/AP`
`/D` from `Appearance::Down`, so `/H /I` beside a `/D` drew the down artwork **and** inverted it.
`annotation::down_overridden` is the reading now, widgets only (Table 176 gives a link four modes
with no such sentence) and `press_changes` asks it too. That row's Table 190 is the *screen*'s.

**Calibration (trap 13)**: both fixtures are rasters, both run with the change planted back — the
knockout overlap read (64, 128, 64) for (0, 128, 128), the press the `/D`'s blue for the `/N`'s red.
**Trap 1 by eye** at 4× on all four appearance-group renders. **Gates**: fmt (my files,
`rustfmt --check`) 0; clippy `-D warnings` on `pdf-model` 0; `nextest -p pdf-model` 1492/1492 (0);
doc 0; `raster_golden` **held 974, moved 0** (0); `--test corpus` 0, every ratchet at its ceiling;
`--test actions` 2/2 (0); oracle 3/3 (0) at load 7.02; `viewer-core` `headless` 121/121 (0).
`-p conformance` cannot pass here — a sibling's §12.7.6.2 note has an unescaped `"` at line 4204 and
the ledger will not parse — and the quotation gate names no file of mine.
