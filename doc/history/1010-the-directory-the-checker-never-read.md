# 1010 — The directory the checker never read

Branch `batch-1006-1011`, in the shared worktree `/home/AI/pdf-viewer-rounds` alongside sessions
1006–1011. ADR **1029**. Topic: ADR 1024 §4's finding — `tools/conformance`'s `SOURCE_ROOTS` is a
hand-written list beside a manifest glob — and the gate population it implies.

## What was done

1. **The roots are derived** (`tools/conformance/src/roots.rs`, new). `SOURCE_ROOTS` and
   `pointers::ROOTED_HEADS` are gone; the scan's population comes from the workspace manifest's
   `members` globs plus the crates at the top of the tree, and the pointer heads from the walk
   `pointers::Tree` already does. `raster/crates/*`'s five crates and `kio/` entered every sweep.
2. **1,884 citations entered the gate**, producing 259 citation findings and 60 quotation findings.
   All 259 were a `§` naming a section of a document that is not ISO 32000-2 — the brief 204, the
   caller's documents 26, WGSL 26, a research note 3 — and each was rewritten to the tree's own
   spelling for another document's section, the word `section`. Of the 60 quotation findings, six
   were **wrong clause numbers on correct quotations of the standard**; the rest were the
   instrument's (see ADR 1029 §3) or writing the gate could not classify.
3. **1004's check 7 moved out of `tools/round.sh`** into
   `tools/conformance/tests/workspaces.rs::every_workspace_member_is_scanned`, with its two sides
   derived differently on purpose — the scan's from the manifest, the gate's from git.
4. **Five of `--bin quotations`' forty-nine diverging quotations read** (ADR 1029 §6). Two were
   real and are fixed; three were the instrument's, in three different ways — a matcher that takes
   the first candidate rather than the best, a correction quoting the quotation it retired, and
   `doc/errata-read.md` quoting the errata's *replacement* text against a conversion of the text
   the errata amend.
5. **The population question, asked of the sweeps.** One of twenty holds a classification the
   standard states itself: the ledger's normative/informative annex lists.
   `every_annex_is_classified_as_the_standard_classifies_it` now reads all 17 annex title lines
   against them. It agrees today, and it is calibrated.

## Movements, each explained

`tools/state.sh` and the sweeps print the numbers; what follows is only what moved and why.

- **`cargo test -p conformance`**: 6 tests → 7, all passing. The citation gate reads 17,278
  citations where it read fewer, the quotation gate 1,454 quotations of the standard, plus 16
  blockquotes attributed elsewhere and 8 equations under clauses whose formulas the conversion
  dropped — three counts that did not exist before this round.
- **`--bin pointers`**: 15,785 → **16,623** path pointers. Attributed by running the sweep with this
  crate's sources at `HEAD` against the same tree: **15,954 of that is the tree** (sessions
  1006–1011 editing the same worktree while this ran), and the remainder is this round —
  `raster/`'s comments joining the population and `raster`/`kio` becoming rooted heads, so pointers
  that were dropped as unrooted now resolve. The absent count rose from 211 to 226: a reading list,
  not a verdict, and most of it is `raster/fill.rs` where the path is now
  `raster/crates/raster/src/fill.rs`. The sweep exits 0.
- **`--bin quotations`, documents**: 9,279 → 9,350 in 1,659 → 1,670, diverging 49 → **51**.
  **None of it is the typography folds**: with `quote.rs` swapped back to its `HEAD` version the
  sweep prints the identical line, measured rather than assumed. One of the two new divergences is
  mine and is the finding restating itself — ADR 1029 §6 quotes Errata Collection 3's *replacement*
  sentence while explaining that `doc/errata-read.md`'s quotations of replacement sentences diverge
  from `doc/md/` by construction. The other is a sibling's.
- **`--bin quotations`, the ledger**: 1,940 → **1,943** verbatim, 5 → **3** diverging. Two of the
  five were read and fixed (ADR 1029 §6): a table caption abbreviated inside the quotation marks,
  and a parenthetical truncated at a semicolon with the bracket supplied. Both sweeps exit 0.

## What a later round should know

- **`raster/` writes a bare `§` for two different documents**, interleaved line by line —
  `"§4.5 places §8.5.3.2's disc upstream"` is one brief section and one clause in one sentence. The
  259 corrected here are only those whose number ISO 32000-2 does not have. The rest resolve, so no
  gate can see them; ADR 1029 §5 says what is left.
- **The shared worktree moves under you.** Two gate runs of `--bin pointers` twenty minutes apart
  differed by 35 pointers with none of my files touched between them. Any figure taken here needs
  the tree's state named beside it.
