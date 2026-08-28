# 794 — The rule the writer is named by

The errata selection rule's twelfth use. Both rankings topped out at four annotations, with the
one live row at that height *inside* the settled plateau — eight rows tied over every row, seven
of them settled — so step 4's preference took a settled head and the third use's tie-break chose
between seven: §14.8.5.4.2, whose two issues rewrite table cells where the rest of the plateau
moves typography, a URL, a step letter and a version marker. It confirmed its rows and paid
nothing, for the fourth consecutive use, and placing the rectangles showed that both of its
cell-rewriting pairs belong to §14.8.5.4.1 one clause above. **The walk downward paid on the live
head.** Issue #90 is a caret with no strikeout: it inserts *providing the Contents key, if* after
the opening *When* of §12.5.6.2's paragraph rule, so a sentence written in the passive now names
the act of writing the entry — and the line-feed tolerance this tree argued as an inference about
whom the `shall` binds becomes the clause's own scoping, in the popup window and in a free text
annotation's layout alike. Issue #297 re-dates the same row's grouping sentence to PDF 1.5 and
marks `/RT` as the PDF 1.6 part, under `check`'s four-word floor, quoted in two blockquotes and
the row. Twelve issues left the population; no code moved.

Date: 2026-08-28.
ADR: [0728](../adr/0728-the-rule-the-writer-is-named-by.md), the number the briefing reserved.

Touched: `crates/pdf-model/src/markup.rs`, `crates/pdf-model/examples/annotation_group_census.rs`,
`crates/viewer-ui/src/chrome.rs`, `crates/pdf-model/src/variable_text.rs`,
`crates/pdf-model/src/content/marked.rs`, `crates/pdf-model/src/destination.rs`,
`crates/pdf-syntax/src/crypt.rs` (one erratum annotation apiece — no executable line changed),
`doc/conformance/ledger.toml` (§7.6.4.4.2, §7.10.5.2, §12.5.5, §12.5.6.2, §12.10.3, §12.11.2,
§14.7.5.3, §14.7.5.4, §14.8.5.4.1, §14.8.5.4.2, reformatted by its own binary),
`doc/errata-read.md` (twelfth-use section), `doc/todo/01`, `tools/worktree.sh` (the gitlink guard
— see below), the ADR and this file.

## What the rule gave

Under the recipe's own single-issue line parse, 302 issue numbers in
`doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret and **85 were named nowhere** at
this round's base — the eleventh use's closing arithmetic (99 less its fourteen verdicts)
reproduced by the greps, the third consecutive use at which base and derived closing figure
agree. The multi-issue parse counts 310 and 87, the eleventh's own figures less the same
fourteen. Twelve issues gain verdicts this round; the closing population is 73, re-derived by the
same greps after the records were written.

Over every row the head is an eight-way tie at four — §7.6.4.4.3, §7.10.5.3, §12.5.6.1,
§12.5.6.2, §12.10.4, §12.11.2, §14.7.5.4, §14.8.5.4.2 — and over live rows §12.5.6.2 alone.

## What paid

- **Issue #90 (§12.5.6.2, the paragraph rule)**: a single caret, no strikeout. The amended
  sentence opens *When providing the Contents key, if separating text into paragraphs*, which
  names the writer the `shall` was always addressed to. `viewer_ui::chrome`'s popup and
  `pdf_model::variable_text::encode` both accept a CARRIAGE RETURN, a LINE FEED and the pair as
  one break already; the warrant for the second and third stops being ours. Nothing to test:
  the behaviour is unchanged and both places are pinned by tests that predate the round.
- **Issue #297 (§12.5.6.2, the grouping sentence)**: *PDF 1.6* struck for *PDF 1.5*, with
  *(PDF 1.6)* inserted after the `RT` instead — two words, under `check`'s floor, and the
  sentence is a rustdoc blockquote in `markup.rs`, the same blockquote in
  `examples/annotation_group_census.rs`, and a quotation in the row. All three keep the published
  wording the quotation gate verifies and carry the amendment beside it.
- **Issue #643 (§7.6.4.4.2)**: Algorithm 3's two cross-references into Algorithm 2 named step
  (b), which initialises the MD5 hash function, for the padding string step (a) prints. Both `b`s
  struck for *a*; `crypt.rs` has cited step (a) since `PAD` was written, so the erratum vindicates
  the reading. Recorded on `unwrap_owner_entry`, which is what runs those steps.
- **Issue #339 (§14.7.5.3)**: a second erratum over the run Issue #431 replaces wholesale, both
  landing on *required*. `destination.rs` credited #431 with both halves; each is now named.
  **Two accepted errata over one sentence, agreeing** — Table 161's shape without the
  contradiction.
- **Issue #343 (§14.7.5.4)**: three words struck under `check`'s floor, in a blockquote in
  `content/marked.rs` and in the row; the sentence's second half is what makes the route per
  stream, so nothing moves.
- **Issue #321 (§12.10.3)**: the published paragraph said the coordinate system "shall be
  described in either or both of two well-established standards" while each of Table 270's rows
  forbids the other entry — **the clause contradicted its own table**, and the erratum takes the
  *both* out. `CoordinateSystem::is_stated` reads the table.
- **Six more to verdicts, none moving code**: #223 and #189 rewriting Table 377's and Table 378's
  cells for attributes nothing here reads, #195 demoting a `shall` inside clause 13's ground,
  #269 and #669 correcting this standard's own printing of `if` and of `7Dh`, and #422
  italicising two `f`s in an EXAMPLE.

## Gates

Full §2 sequence, all green. `fmt` clean; `clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"` silent apart from gcc's `viewer-qt` bridge warnings on the cold build,
which §2 names. nextest 2740 passed, 18 skipped — the launch test passed inside the workspace
run. Doctests clean; the fuzz `check` clean; both trap-10 binaries built before the gates that
need them. Corpus: 974 documents in 2.9 s — 0 unopenable, 8 locked, 2 encrypted beyond us,
6 pageless, 67 incomplete, 0 slow. Oracle: 1945 pages in 161.1 s — 983 agree, 61 contradicted,
836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render; exit 0, no
ratchet moved. **The reference cache was cold**: this worktree's build directory has its own
`tmp/pdfref-cache`, so 6698 renders were produced at a 0.1% hit rate and the run cost 161 s where
a warm one costs a third of that — a cost, not a result; every verdict count matches the previous
round's exactly. Text extraction: 4 passed in 33.3 s, 10971/11163 words in bounds (98.28%), 487
of 508 documents fully in bounds. Selection census 5.4 s; accessibility census 21.9 s, no ratchet
moved, 0 untagged pages given structure. Dates, XMP, JPEG 2000 green. Quorra: 957 pages in 41.3 s
— 932 agree, 22 differ, 3 refused, 17 not comparable. `fixed_documents`: 41 checked, 0 absent.
`cargo test -p conformance`: 23 result lines, all ok — re-run after every document edit.

## Sweeps

Fourteen sweeps plus the three errata ones, against the pristine main checkout before any edit
and again in the worktree after them; `quoted` and `unpriced` not run, no page-list note touched.
Every delta is the round's own work. `blockers`, `callers`, `entries`, `unread`, `overstated`,
`parts` and `overtaken` identical. `capabilities` unchanged but for line numbers.
`pointers` +3 paths, absent unchanged at 98 and undefined at 13. `tables` +12 sentences and +3
key citations, **all agreeing**, absent unchanged at 101 and denials at 6. `counts` +6 sentences
and +2 attributed counts, both in the "clause with no rows below it" bucket, agreeing count
unchanged at 151 and contradictions at 4. `owed` two rows gain terms with every one named, and
its three population figures — 182 unnamed terms, 112 rows, the 110-row reading list — are
unchanged. `quotations` +2 ledger quotations, **both verbatim**, diverging unchanged at 2 and 38.
`inapplicable` §14.8.5.4.1 gains shared vocabulary that sorts to the noise end, no new cousin.
`applied` +14 places naming an erratum and +10 comparisons, with its read-first list unchanged at
10 — the round's own sentences quote the *replacement* in italics, which is the convention that
keeps them off it. `check` unchanged but for line numbers, after one repair: the #643 note was
first written on `PAD`'s doc comment, where its §7.6.4.4.2 citation re-labelled `AES_BLOCK`'s
neighbouring §7.6.3.1 quotation in `check`'s output; moved to `unwrap_owner_entry`, which is
where the algorithm is, and the relabelling is gone. `moved` +1 source citation of §7.6.4.4.

One pre-existing environment difference, not this round's: `pointers` resolves
`file.rs::a_test` — a metavariable in three ADRs — against `tmp/hayro/hayro-jbig2/src/file.rs` in
the main checkout and against nothing in the worktree, which has no `tmp/`. Same three lines,
same rung, different witness.

## What else the round touched, and why

**The gitlink guard covered four of six submodules, and this round tripped the other two.**
`tools/worktree.sh` replaces `doc/corpora/*`, `doc/pdf.js` and `doc/arlington-pdf-model` with
symlinks into the main checkout, and set `--skip-worktree` on the corpora alone — the guard was
one line inside the loop that links them. A `git add -A crates doc` here therefore staged mode
120000 over the gitlinks of the two the loop does not reach, and the commit had to be amended to
put them back (`git restore --source=HEAD~1 --staged`, then `--amend`). Nothing was lost, and it
is the third time this footgun has fired in this tree.

The guard now derives its own population — mode 160000 in the index, a symlink on disk — so it
covers whatever the script has linked rather than a list written by hand, which is exactly how
this one went stale. `list` counts the same way and its report is sharper for it: it printed
`(4/4 skip-worktree)` for a worktree that was guarded on four of six paths, and now prints
`GITLINK GUARD OFF (4/6)` for the three live siblings, which were opened before the repair. They
are **not** touched from here: another round is working in each, and `update-index` on somebody
else's checkout is the same class of act as the one this fixes.

## What contradicts the briefing

- Nothing does. The briefing's population figure — 85 after the eleventh use's fourteen verdicts
  — is confirmed by the greps rather than trusted, and the multi-issue parse reproduces the same
  way at 310 and 87.
- The worktree existed at `main`'s tip and was clean, as the briefing said; the round reused it.
- The launch test passed inside the workspace run; per the briefing a failure would have been
  news, and there is none.
- `round.sh` flags the pre-existing CI failure on `main` the briefing names, and `target/pdf-viewer`
  older than `HEAD` — §5's rebuild is owed before a *measurement*, and this round measured
  nothing outside the gates.
