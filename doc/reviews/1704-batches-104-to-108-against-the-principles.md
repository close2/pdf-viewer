# 1704 — Batches 104–108 against the principles and the document rules

Status: **review**, a record that is never rewritten. Session 1704, slot 1 of batch one hundred and
nine. It answers the owner's request of 2026-10-22: "Another process has worked on this repository
(see doc/orchestrator-handover-batch108.md), please use one round to take a look at what has been
done, and if it meets our standards." The decision is ADR 2242. **It reports and repairs nothing**:
each finding below is a candidate slot for batch 110.

The span is `5ded4916..ee365e4f`, taken on 2026-10-23 against `ee365e4f`. It holds 48 commits, five of
them batch commits (`020a86f3`, `b9d05ee8`, `0eb4d278`, `9cd2c665`, `b17ea075`), 32 records
(`doc/history/1671`–`1703`; 1702 never ran), ADRs 2178–2241 (60 files; 2193, 2238 and 2239 are
declared unused, 2205 is missing without a word), and the two handover documents. Commands run from
the worktree root with `PYTHONDONTWRITEBYTECODE=1`. Where review 1456's or 1513's script needed a
change it is copied into `doc/reviews/1704/` with the change stated (ADR 1748 section 2.2), and
`doc/reviews/1704/` adds two.

**Context the findings need.** The owner asked on 2026-10-21 why "improvements regularly break other
behaviour" (review 1679 is the answer), which is why the colour work is framed as stability
boundaries. The owner also asked for regular checkpoint commits (`doc/todo/02` section 8 item 3), which
is why the span has 48 commits rather than five. Both are the owner's, and neither is a finding.

## 1. Measured

### 1.1 The batch clock: `doc/reviews/1704/slots.py 5ded4916 ee365e4f`

`doc/reviews/1456/slots.py` divides by zero on this span. A body that writes "1687 not available" gives a
wall of 0 s, and a body whose durations line holds no wall at all gives an empty list. The copy skips
both and names the commit. Calibrated on `66c6ef23..5ded4916`, it reads all 16 rounds of batches 102–103.

| batch | durations stated | tool uses stated | gate wall |
|---|---|---|---|
| 104 `020a86f3` | 0 of 9 ("full per-round totals unavailable from the retained harness") | 0 | 2 795 s |
| 105 `b9d05ee8` | 5 of 6 (1680–1684: 8 857–11 117 s) | 0 | 3 166 s |
| 106 `0eb4d278` | 3 of 6 (1686 2 317 s, 1688 2 361 s, 1691 472 s) | 0 | 3 040 s |
| 107 `9cd2c665` | 0 of 6 ("not available") | 0 | 2 833 s |
| 108 `b17ea075` | 0 of 5 | 0 | 2 920 s |

**8 of 32 durations and 0 of 32 tool-use counts.** Batch 103's body, the last before the span,
states all eight of each ("1663 4 940 s (239 tool uses); …"). Each batch commit says why: "Tool-use
counts were not supplied by the harness". Batch 103's counts came from the harness notification,
which `doc/orchestrator-handover.md` section 3 step 4 names as the source. Both figures are the
contract's (`doc/todo/02` section 0 item 6; the brief's section 4). Without them review 1513's
section 1.1 cannot be taken over these five batches, and nor can a rate per tool use.

### 1.2 The records: `doc/reviews/1513/premises.py 1671 1703`, `cites.sh`, `siblings.py`

| | records 1639–1670 (the 32 before) | records 1671–1703 |
|---|---|---|
| ≤ 40 lines with a `**Gates.**` paragraph | 32 of 32 | **32 of 32** |
| a `**Premise…**` or `**Hypothesis…**` paragraph (`premises.py`) | present in 4 of 8 records of batch 103 | **0 of 32** |
| trap citations (`cites.sh`) | 14 in 7 records | **0** |
| habit citations | 11 in 9 records | **0** |
| a sibling named (`siblings.py`) | 31 | 9 |
| proposed habits in the record | — | 7, none placed and none declined in writing |

The premise script was calibrated on 1663–1670, where it finds four paragraphs, so the zero is a
property of the records. It is not the instrument (trap 13). The contract asks a round to "say in the
report which of each held" (`doc/todo/02` section 0 item 2). The reports may have done so. The
records do not, so no later reader can measure it. Zero citations of a trap or a habit across 32
records means the rounds read none of the project's memory, or read it and did not say so. The 7
proposals (in 1674, 1675, 1676, 1677, 1678, 1682 and 1684) were neither placed nor answered. The brief
of batch 109 says "No trap or habit was added".

Readability (principle 4) falls in the last two batches: `doc/history/1694` line 3 reads
"Session1694,2026-10-22. ADR2222/2223", line 17 "exit 0,33s,0.57GiB", and line 23
"Release builds exit 0,116/102s,.88/.82GiB.12balanced CPU2pairs/48guards pass". The same register is in
ADRs 2222, 2223, 2234 and 2235. ADR 2235 line 56 has "per20,000", "adds1.514/2.186/1.887" and
"about76/109/94", and 2222/2223/2234/2235 put a whole paragraph of 618–761 characters on one line.
ABBA, OCCA, "Root" (the orchestrator), WTPDF, ROI and CPU2 are used undefined.

### 1.3 The code: `doc/reviews/1704/added.py`

`git diff 5ded4916..ee365e4f -- crates tools hosts raster` adds 19 164 lines and removes 5 787 across 389
files.

- **`unwrap`/`expect`/`todo!`/`#[allow]`**: `added.py 5ded4916 ee365e4f '\.unwrap\(\)|\.expect\(|todo!|#\[allow|#\[expect' crates tools hosts raster`.
  After test paths and `#[cfg(test)]` modules are dropped, no `unwrap`, `expect` or `todo!` remains.
  Six `#[expect]` remain, each with a reason: `too_many_lines` at `pdf-model/src/view/scripts/commit.rs:33`
  (moved verbatim) and `pdf-transform/src/archive/fonts.rs:623`; `cast_precision_loss` at
  `viewer-device/src/renderer/lanes/sample.rs:76`; `print_stderr` at `viewer-gtk/src/host/window.rs:749`
  and `:927`; `cast_possible_truncation` at `quorra-confined/permissions.rs:57`.
  The `unwrap_or_default` hits in `scripts/*.rs` are moved code.
- **Errors swallowed**: none added. The one `let _ = may;` (`pdf-transform/src/confined.rs:1091`) is a
  `#[cfg(not(unix))]` unused binding with its reason above it.
- **`pub` without docs**: 153 `pub` items added in non-test files, every one documented.
  `missing_docs` is inherited by every crate.
- **History in comments** (ADR 1023): `added.py … '(?i)(//|\*|#).*(stability boundar|retained|speedup claim|neutralit|previously|used to|no longer|earlier|baseline|candidate|batch|session)'`
  gives 55 hits. They are the standard's own words, a session as a domain object, or "retained" as a
  domain term. **One carries history**: `pdf-archive/src/table/fonts.rs:1636` "rounding the reciprocal to
  a whole number previously hid this failure". "Stability boundary" and "no speedup claim" appear in no
  `.rs`, `.cpp` or `.py` file. The span kept them in records and ADRs, where they belong.
- **Clause citations** (principle 5, "The specification appears in the code"). These new non-test files
  implement normative behaviour and cite no ISO 32000-2 clause:
  - `pdf-transform/src/archive/font_widths.rs` (87 lines) says "Table 109" and names no standard.
  - `pdf-model/src/view/scripts/events.rs`, `commit.rs` and `state.rs` (moved code) name Tables 198,
    199 and 224 without "ISO 32000-2 §".
  - `pdf-transform/src/archive/default_rgb/page_contexts.rs` (383 lines) cites ADR 2230 only.
  - `pdf-transform/src/archive/in_place/linked_popups.rs` reads `/Popup`, `/Parent` and `/IRT` in 168
    lines with one `§`.

  Its test file `pdf-transform/tests/archive/linked_popups.rs` holds 7 tests and cites no clause.
  The new colour tests do cite theirs: `icc_paint_context/indexed.rs` derives every expected pixel from
  the fixture's encoded knots and the published sRGB matrix, the derivation principle 5 asks for.
- **`pub` widened**: no item went from private to `pub`.
  - `pdf_colour::colour::ConversionContext` is new and `pub`, with seven `pub` fields, and only
    `pdf-model/src/content/colour.rs` reads it.
  - `Conversion::context()` (`pdf-colour/src/colour/conversion.rs:167`) is `pub`, and only
    `pdf-colour`'s own tests call it.
  - `ConversionContext::to_owned(self) -> Conversion` (`conversion.rs:151`) shadows `ToOwned::to_owned`
    with a different return type.

### 1.4 The arms: `doc/reviews/1704/arms.py /home/AI/arms-1663 /home/AI/arms-1671 /home/AI/arms-1680 /home/AI/arms-1704`

HEAD's arms (`/home/AI/arms-1704`, `ee365e4f`) against `66c6ef23`'s (`/home/AI/arms-1663`): the populations
are identical (968 pages per 1× arm, 965 per 4× arm), and **five documents moved**. The chain through the
intermediate exports places every move:

| link | moved |
|---|---|
| `66c6ef23 → 5ded4916` (batch 103) | nothing; 1657's medium and 1665's grid moved no corpus page |
| `5ded4916 → 020a86f3` (batch 104) | `bug1721218_reduced`, `issue16742` on all six arms; `issue4801` on compute 1× and every 4× arm; `issue4550`, `issue5751` on CPU and GPU 4× |
| `020a86f3 → ee365e4f` (batches 105–108) | nothing |

Every move is claimed, though **by a record and not by an ADR**. Record 1672 ("Pixels") claims
`issue4550`, `issue4801` and `issue5751` for the fractional CFF matrix: at most six pixels per image,
each by one level, inspected. Record 1673 claims `bug1721218_reduced` and `issue16742` for the cropped
straight-alpha rounding, attributed by isolated controls. ADRs 2180 and 2183 decide the mechanisms and
name no page. Every mean error is unchanged or lower. **Nothing is unclaimed.** Batches 105–108 held
every digest, as each batch commit says. I checked the last of those claims against its receipt (section 1.5).

### 1.5 Receipts outside the tree

The span adds 63 lines naming `/home/AI/`: 10 in records, 13 in ADRs, 12 in the supplement and 3 in
tools. The five batches before it added 32 (`git diff -U0 d1a6938a..5ded4916 -- doc crates tools | grep
'^+' | grep -c /home/AI/`). Two things make this a finding and not a style point:

- The owner cannot read `/home/AI` (`doc/environment.md`, "The machine, the account and the display").
- Every relative `scratchpad/r16xx/` path the ADRs cite is gone: r1673, r1676, r1682, r1684, r1689,
  r1691, r1692, r1694 and r1696, cited from ADRs 2188, 2198, 2203, 2212, 2217, 2218, 2223 and 2227.

The 547 MB `evidence-batch108-1698-1703.tar.zst` is not a substitute: nothing in the tree says which
claim rests on which member.

Three of the supplement's claims, each checked against its receipt:

1. "Final canonical gates: **38 of 39 pass**, 2,920 s … Only `t2-launch_path` fails (exit 101)":
   `/home/AI/batch-gates-1698.log` ends `ALL GATES DONE — 38 of 39 green, 2920 s of gate wall time`, and
   its only non-zero exit is `t2-launch_path exit=101`. **Holds.** The failure has five judged figures:
   the warm opens of PDF20_AN001-BPC, bug1815476 and opt_demo, and opt_demo's script spawn and first run
   (`/home/AI/launch-frozen-1698/canonical-failure.log` lines 14–18). Bring-up and ISO first page are
   listed as outside but declined for the carve-out, as ADR 2233 says.
2. "All six final renderer arms … match fixed 9cd2c665 exactly": `/home/AI/arms-1698` (`9cd2c665`)
   against `/home/AI/final-arms-1698` (`ae6f6fcb`) agree in order, name, digest and mean error on all
   six arms. **Holds.**
3. "the final original run fails four, with 1.036–1.383 MiB more RSS than its first run":
   `/home/AI/launch-ab-1698/location-analysis.md` gives `open_peak_mib` original pairs whose differences
   run from 1.036 (opt_demo) to 1.383 (PDF20). **Holds**, with one correction of name: the figure is
   `open_peak_mib`, "what an open costs in memory", and not `peak_anon_mib`.

## 2. Findings, ranked by what each costs the project

1. **The launch gate was merged red three times, accepted each time by an orchestrator's ADR, and left
   red, so batches 107 and 108 merged with no launch gate.** ADRs 2209 (line 85), 2221 (line 82,
   "Root accepts this diagnosed launch finding") and 2233 (line 60, "The orchestrator accepts …
   while keeping the launch gate FAILED"). Each acceptance sits under a title about something else
   ("Stability boundaries follow observed regressions", "Paint-state stability needs source and context
   boundaries", "Stability follows source and context lifetimes"), so a reader of the ADR list cannot
   find it. No `Q` was asked.

   Judged against what the project already holds:
   - **The timing half had a rule, and the rule was not applied.** Habit 101 judges a sub-millisecond
     figure by the instruction count beside it and by HEAD in the same hold. In ADR 2233's ABBA the
     baseline fails the same warm-open bands (bug1815476 0.383, opt_demo 0.171, BPC 0.419), with
     `open_kinstructions` at C/B 1.0000 on all six documents
     (`/home/AI/launch-ab-1698/analysis.md`). By habit 101 that failure is the machine's and not the
     batch's, and batch 100 (`d1a6938a`) merged on exactly that judgment. But "for three merges running"
     is ADR 1969's own opening, and ADR 1969's answer was to **re-take the bands** so that the gate gates
     again. Batches 106–108 kept the bands and kept the gate red.
   - **The memory half was not the machine's by the same test.** Both ABBAs have the baseline pass and
     the candidate fail the memory ceilings: ADR 2209 line 54, and ADR 2233 "four candidate-only memory
     failures". The OCCA controls then showed `open_peak_mib` moving 1.0–1.4 MiB on identical bytes and
     identical instructions. That **falsifies `doc/checks/launch-path.toml` lines 22–30's premise**
     that what an open costs in memory is one of the figures that "are properties of the reader rather
     than of the afternoon". It is an
     instrument defect, and it was found and not filed.
   - **Verdict**: an orchestrator may not accept a regression by ADR. Principle 2 says "A regression
     fails the gate", and CLAUDE.md asks that a conflict with a principle be put to the owner rather
     than settled quietly. The thoroughness is real: predeclared orders, frozen hashed binaries, every
     failing run kept. But it went into proving the failure unattributable, where the project's rules
     would have made the gate truthful again: re-take, calibrate or ask. Slot 2 of this batch owns the
     cause. ADR 2242 records the rule.

2. **Every solid fill, stroke and glyph now builds and drops an owned `Conversion` to ask whether it
   bypasses black generation, and no figure prices it.**
   - `fill_paint` (`pdf-model/src/content/pattern.rs:515`) calls `solid_bypasses_black_generation`
     (`content/colour.rs:188–201`).
   - That calls `self.conversion(state)` (`colour.rs:213`), which clones up to five `Arc`s: the route,
     output intent, CMYK group, black generation and spots.
   - It runs whatever the space, whether or not a black-generation pair is in force, and after the
     page's flag is already `true` (the `|=`).
   - Text calls `fill_paint` once per glyph (`content/text.rs:1519`), seventeen lines below a comment
     (`:1502`) that borrows a pattern rather than cloning it because "this runs once per glyph".
   - The pattern tint does the same, a second time, per cell mark (`pattern.rs:52–55`).

   It came in with session 1675 (ADR 2187), which carries no cost figure. The pairs of 1682, 1689, 1694
   and 1700 took `020a86f3` as their baseline, so it is inside both arms of every ratio, and every
   workload measured was path fills, never text. The borrowed `ConversionContext` that exists "for
   cache lookups without reference-count churn" (`pdf-colour/src/colour/conversion.rs:108`), or a
   short-circuit on `black_generation_painted` and the RGB space, answers it.

   **The accepted correctness costs are priced only in records and ADRs**: 2198 lines 80–91 (1.016–1.085),
   2212 (1.010–1.044), 2223 (1.2018/1.2087 on restore) and 2235 (1.20177). The comments at the code
   (`colour.rs:931`, `:938`, `:839–842`) give no figure and cite none of these ADRs. That breaks "An
   optimization must be justified by a benchmark and explained by a comment" for four accepted costs.
   The workloads are committed (`cargo run -p pdf-model --example black_generation_order -- --cost |
   --pattern-cost | --icc-cost`). The pinned, guarded paired driver behind every accepted ratio is not
   (ADR 2198's `summarize-pinned.py` is in no `git ls-files`).

   The construction is otherwise near the cheapest correct one. A selection seeds the slot, and a hit is
   two key comparisons. The epoch counter that would make a hit one comparison was declined in writing
   (ADR 2198, "No manual invalidation graph"), a defensible clarity choice.

3. **The evidence is outside the tree** (section 1.5): 63 references, twice the rate before, to paths the
   owner cannot open or that no longer exist.

4. **The records depart from the contract** (sections 1.1–1.2): 8 of 32 durations, no tool-use count, no
   premise paragraph, no trap or habit cited, 7 proposals left unanswered, and the telegraphic register.

5. **The ADRs**:
   - 14 of 60 have no Status line: 2196, 2197, 2200, 2201, 2214, 2215, 2220, 2221, 2224, 2225, 2228,
     2229, 2236 and 2237. Every one of 2100–2177 has one.
   - Five are frozen at a state the merge passed: 2194 and 2195 "under validation", 2198 "final batch
     integration pending", 2234 and 2235 "proposed for integration".
   - Nine record no decision a later round must not re-litigate (`doc/todo/02` section 0 item 4): 2199
     "accepted finding, implementation owed", 2209 "accepted direction", 2215, 2220, 2221, 2232, 2233
     ("## Assessment"), 2237, and 2241 "diagnostic experiment".
   - 2188 and 2189 are mostly receipts.
   - ADRs are records and are not rewritten. The cost is that a reader of `doc/adr/` cannot tell which of
     these is in force.

6. **Three batches of UI instruments attributed none of the owner's three symptoms.**
   - About 1 500 lines: `tools/zoom-probe-drive.py` (476), `tools/zoom-probe-records.py` (459), and about
     640 in `viewer-gtk`, `viewer-qt` and `window.cpp`, shipped in the release binaries behind
     `PDFVIEWER_ZOOM_PROBE`.
   - Each record says it established no cause. 1690 line 35: "The owner's Qt lag, GTK blur and deep-zoom
     drift remain unattributed". 1697 lines 30–31, 1701 lines 37–38, 1703 lines 7–8.
   - `zoom-probe-drive.py` lines 46–49 delete `WAYLAND_DISPLAY` and force `GDK_BACKEND=x11` and
     `QT_QPA_PLATFORM=xcb`. Every hardware trial therefore ran on Xwayland, on the Wayland session where
     the owner sees the symptoms, and its `xdotool` warp left the pointer where it was (supplement lines
     84–89). Native Wayland is "owed" in every record and was never tried.
   - The one product defect of the three batches was found by a headless in-process witness, not by a
     probe: GTK's scale-two allocation, ADR 2214, `viewer-gtk/src/host/stand_in_tests.rs`.
   - Smaller defects: `viewer-qt/src/host/input.rs:178` reads the environment on every wheel step
     (GTK caches it in a `OnceLock`, C++ in a static). No test exercises the GTK or Qt probe paths.
     Neither tool is reachable from `doc/HANDOVER.md` or `tools/state.sh`. ADR 2228 line 30 counts nine
     self-tests where the code has ten.

   **What a reviewer would have had the fourth batch do instead**:
   - take the witness that worked to the symptom: render at the owner's device-pixel ratio and zoom
     in-process, and compare the texture with the widget's device allocation;
   - time one Qt zoom step with the instruments that exist (`examples/zoom_frame`, `tools/state.sh heavy`,
     `--trace=frames`);
   - or ask the owner, in a `Q`, for one reproduction on native Wayland with keyboard zoom under the
     existing probe, which needs no pointer warp;
   - and stop adding instruments once the second batch of them had attributed nothing.

7. **Counted facts in the instruction documents** ("Where knowledge lives").
   - `doc/orchestrator-handover-batch108.md` sits where `documents.rs` reads current instructions,
     calls itself a "closing record", and writes 20-odd counted facts: 656 pass/14 skip, 13,547 selections,
     7,649 tests, 38 of 39, 2,920 s, and more. It also gives orders, at line 5 "Do not start another
     round…", which batch 109 has since overridden by the owner's word.
   - `doc/todo/75` adds gate verdicts and figures. `doc/todo/74` hand-keeps line counts.
   - `doc/todo/65` §6 line 262 is stale: it still says uncoloured tiling tints "retain only a
     default-converted colour", which ADR 2212 fixed.
   - `doc/PLAN.md`, `doc/state-of-play.md` and `doc/HANDOVER.md` were not edited in five batches. Slot 7
     of this batch rewrites them.

8. **Small, each one line**:
   - the clause citations of section 1.3;
   - `ConversionContext`'s width and its `to_owned`;
   - the history comment at `pdf-archive/src/table/fonts.rs:1636`;
   - the arms moves named by records and not ADRs (section 1.4);
   - ADR 2205 missing without a word, and batch 105's brief allocating Q574–579 where batch 109's
     allocates Q567–574.

## 3. What met the standard

- **CI green since batch 105**, every batch merged with tier 1 whole (warnings-denied clippy, fmt, nextest,
  doctests, fuzz on stable and nightly, rustdoc across all 44 packages), and the three red CI jobs on
  `5ded4916` fixed first. The Windows descriptor compile error was fixed in 1674.
- **Independent controls before the fixes** (1693, 1699). The unchanged baseline fails 11 of 13 and 9
  of 23 at the intended assertions, and the expected pixels are derived from the fixture's encoded
  knots and the published matrix, not from the output. Omission controls fail where they should.
  This is principle 5 practised.
- **No `unwrap`, `todo!` or swallowed error in production code**, and every added `pub` item is documented.
- **The ledger is honest.** §11.7 and §11.7.5 went `implemented` → `partial` for a reproduced
  order-dependence. The net count fell by two, which is the truth rather than a stall. §11.7.5.3's note
  grew from 11 939 to 13 555 characters and stays a reading of the clause, with no session in it. The
  two blockquotes in the span's ADRs (2178, 2187) are verbatim against `doc/md/`.
- **Measured rejections**: 1673's readback cache and 1676's replay reuse were measured, found not to pay,
  removed and recorded (ADRs 2182, 2189). That is principle 2's "decided by measurement" as written.
- **The arms held byte for byte** from `020a86f3` to HEAD across four batches of colour-state work.
  Batch 104's five moves are each claimed and inspected. The three receipts checked all hold.
- **Every record is within 40 lines with a `**Gates.**` paragraph.**
- **Failure kept beside success**: failed attempts, refused trials and red runs are written down rather
  than dropped. The fault is where they were written (finding 3), not that they were.

**The brief's hypothesis**:
- *The code is largely sound and conservatively tested*: **held**, with finding 2 as the exception.
- *The paperwork departs from the contract*: **held**, and further than briefed (premises and
  citations at zero).
- *The red launch gate is the one departure from a principle*: **not held**. Finding 2 departs from
  principle 2's comment rule, and the records and ADRs from principle 4's legibility.
- *One or two pages moved without an ADR naming them*: **five moved, all claimed by records, none named
  by an ADR.**

## 4. For batch 110

One slot each, in the order above. The launch gate's cause is slot 2's in this batch.

1. The per-mark `Conversion`, short-circuited or borrowed, priced on a text workload, with the four
   accepted costs written at the code with their ADR numbers.
2. The evidence a record or ADR cites lives in the tree, or is reproduced by a committed command. A
   conformance check that a new record or ADR names no `scratchpad/` path is the enforceable half.
3. The ADR Status line, held by a gate from 2242 on (`no_record_is_in_the_population` excludes
   `doc/adr` today).
4. The instruments: cache Qt's probe flag, give `zoom-probe-drive.py` a native-Wayland, keyboard-driven
   arm or retire it, and take the scale-two witness to the owner's ratio.
5. `doc/todo/65` §6 and the clause citations of section 1.3.

## 5. For the next re-take

- `python3 doc/reviews/1704/slots.py <first> <last>`
- `python3 doc/reviews/1513/premises.py <lo> <hi> -v`
- `bash doc/reviews/1513/cites.sh <lo> <hi>`
- `python3 doc/reviews/1456/siblings.py <lo> <hi>`
- `python3 doc/reviews/1456/batches.py <first> <last>`
- `python3 -I -B doc/reviews/1704/added.py <first> <last> <regex> <paths>`
- `python3 -I -B doc/reviews/1704/arms.py <arms-dir>…`
