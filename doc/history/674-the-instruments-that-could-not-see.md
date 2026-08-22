# 674 — The instruments that could not see

Twelfth merge round of the block, and its closing round. Four branches, no conflicts once an
untracked copy of `doc/QUORRA_GLYPH_PHASE_CARRY.md` was removed from `main` — the project owner had
dropped the file in by hand, round 673 committed a byte-identical copy, and git will not overwrite an
untracked file with a tracked one. Worth knowing: **a document handed to this tree outside git is a
merge conflict waiting for the round that commits it.**

## The sequence, whole, on a quiet machine (load 0.41)

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, the fuzz check — all silent ·
`nextest` **2437 passed, 17 skipped** · doctests, conformance (**182** + 5 + 1) · corpus **974
documents, 68 incomplete** · oracle **908 agrees, 65 contradicted, 786 ambiguous** · **`render-quorra`
957 pages at `glyph quantum 1/16` — 933 agree, 22 differ, 2 refused** · `fixed_documents` **40 checked,
0 absent** · text, both censuses, dates, XMP, JPEG 2000 · `cargo deny` all four ok. Ledger 875 rows:
436 implemented, 224 partial, 18 reported, 76 inapplicable, 8 writer-side, 113 out-of-scope, 0
unreviewed, **no `silent` row**.

That quorra line is the block's last finding in miniature. It reads `glyph quantum 1/16` because
round 673 made the gate use the setting the product ships. It had said nothing at all for its whole
life, and ran with the setting turned off.

## The four rounds

**673 — the quorra carry.** Their ADR 0073: `GlyphPlacement::of` rounded a fractional device
translation with `% q`, which reaches `q` itself for 3.1% of sub-pixel phases per axis and wrapped to
bucket 0 of the *same* pixel, drawing the mark **a whole device pixel low**. Their measurement of our
corpus reproduced to the page — 800/155 → 933/22 — and **their scope stopped at row one**: at 4×
magnification 212 pages move. No page regresses on any of four lanes, checked by name rather than by
total. `the_glyph_quantum_cost_stays_bounded` was 2.5 / 30.0 / 0.95 against a worst of 0.6865 / 20.10
/ 0.99155, so **it passed the defect on all eight cases**; re-derived to 0.80 / 4.8 / 0.9970 and
forced both ways on two adapters. And it found a fourth hiding place beside quorra's three:
`examples/quantum_diff.rs` **had printed this defect on every run since the atlas landed**, filed
under "the quantum's trade".

**671 — the same defect one instrument over.** 667 gave `witness_census` a `--crawl` scope; the
sixteenth sweep has *two* instruments, and `absence_audit` — the one that measures a construction
rather than a name — still had the curated corpora hard-coded. Ten negatives re-derived, **five
false**: §12.2's boundary entries 0 → 96, §10.7.2's `/FL` 0 → 88, §12.7.5.5's `/Lock` 0 → 90,
§12.9.2's rectilinear measure 0 → 127 of 277, §12.6.3's `/PV`/`/PI` 0 → 5. Its own first draft scored
a false zero for a thread action written inline in an `/AA` — **session 648's exact defect** — caught
by a planted witness.

**670 — the figures a note quotes.** 665 had measured 12 and 34; those were two tokens of a
five-token vocabulary, and the real count is **32 of 124 notes quoting 137 figures**. So it is a gate,
and one that rasterises nothing, since the oracle already prints every measure. **Its plant failed
before it passed**, which was the finding: restoring 665's own defect produced silence, because that
note's page list is `[&str; 0]` and a sweep anchored to list membership is blind to exactly the notes
nothing else points at. Thirteen notes corrected, contradicted figures 69 → 48. It found **a fourth
way for a note to be wrong** — forty lines diagnosing one paper sitting above the wrong `const`, two
groups from the list they open — and `freeculture.pdf` page 255 standing outside its own note's band
at mean 16.63 where nothing else in the book reaches 9, **never opened**.

**672 — the sixth criterion, and the mirror of a sentence already in the file.** `oracle.rs` held *a
number stated correctly is not a mechanism explained*; the sixth inverts it — **a mechanism explained
is not a number accounted for.** A contradicted entry is a standing exemption from a *specific failing
bound*, so does the note's mechanism account for the measurement the gate fails us on? Five of
fourteen do. It inverted the diagnosis of the one that measured nothing: removing `/VE` entirely moves
the failing differing fraction by **0.037 of the 1.35 points it is over by**, while the `DeviceCMYK`
press owns 4.371 — so the group was right about the worst tile and wrong about the fraction, and the
second belongs to a different group.

## The block summary

Appended to `doc/history.md`, which `doc/HANDOVER.md` makes the closing round's one exception to
the no-append rule.
