# 798 — Four clean merges, and a guard that counts its own population

Merge round, 2026-08-28, on `main` from `babb3f40`. Merged `round-794` (`70baf0f5`),
`round-795` (`bfc4c84e`), `round-796` (`dca73207`), `round-797` (`61579e50`, two
commits), in round order, each with `--no-ff`. **All four clean — no conflict in any
file, including `doc/conformance/ledger.toml`**, which 794 (ten rows across §7, §12 and
§14), 796 (§9.7.3, §9.7.5.2, §9.10.2) and 797 (§10.7.4, §8.9.5.3) touched on disjoint
rows; the ort strategy reconciled them and the ledger binary re-run on the merged file
printed no diff, so this batch has no reconciliation commit at all. The expected code
contact did not materialise either: 795's host crates, 796's `pdf-font`/`pdf-model`
tests and 797's `render-cpu`/`pdf-render` never met. Then the full §2 sequence on merged
`main`, §5's install, the §4 sweeps against a pre-merge baseline, and the worktrees
closed with the merged `tools/worktree.sh`.

## The merge order was 794's to earn

794 fixed `tools/worktree.sh`'s gitlink guard, and a merge round's close step is exactly
the hazard that fix is about, so it was merged first and the close step ran on it.
The guard's population is now **derived** — mode 160000 in the index with a symlink on
disk — rather than the hand-written list of the four corpora under `doc/corpora/`, which
covered four of the six paths the script links and let `list` print a reassuring `4/4`.
Run against the live worktrees immediately after that merge, the fixed `list` reported
what was actually there: **r794 and r796 `6/6`, r795 and r797 `4/6` — GITLINK GUARD
OFF**, the two worktrees opened before the fix. That is the script telling the truth
about a guard it does not have, which is the whole of what the change buys.

## Gitlink verification

`git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` before the first
merge, after each of the four, and before the commit of this file; every stage in this
round named its paths and no blanket `git add` was run. `cargo test -p conformance`'s
`every_declared_submodule_is_still_tracked_as_one` passed on the merged tree.

## ADR verification

`main` ended at 0727 pre-merge; the batch brought 0728 (794), 0729 (795), 0730 (796),
0731 (797) — one per round as the briefings reserved, no collision, nothing at 0732 or
above.

## Gates (full §2 sequence on merged `main`, quiet machine)

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy, `-D warnings`, all targets | clean (exit 0; the only `warning:` lines are gcc's on the cold `cxx-qt` bridge and the `proc-macro-error2` future-incompat note) |
| `cargo nextest run --workspace` | 2755 tests run, 2755 passed, 18 skipped |
| doctests | ok, 24 suites |
| fuzz `check --bins`, `-D warnings` | clean |
| pdf-model corpus | ok — 974 documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow |
| oracle | ok, exit 0 — 1945 pages, 983 agree, 61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render; 100.0% reference-cache hit rate |
| text_extraction (three gates) | ok — pdftotext 99.2%, PDFBox 99.8%, position verdict **10971/11163 (98.28%), 487 of 508** |
| selection_census | ok, 0 panics |
| accessibility_census | ok — panicked: 0, no ratchet moved |
| dates, xmp, jpeg2000 | ok |
| render-quorra corpus | ok — 957 pages, 932 agree, 22 differ, 3 refused, 17 not comparable |
| fixed_documents | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | 207 passed |

**The nextest count is the batch's exact arithmetic**: the base's 2740 plus 795's seven
tests, 796's three and 797's five; 794 added none, and it moved no executable line at
all. Everything else in the table is byte-for-byte what the branches reported in their
own worktrees. The oracle's seven-figure census is unchanged from 793's — 797's claim
that its reduction memo is byte-identical over 957 pages holds on the merged tree, where
it is measured beside 795's and 796's work rather than alone; quorra's four figures are
unchanged; the text-position ratchet held at 487 of 508 and the accessibility ratchet
did not move.

**The corpus gate now prints 796's composition**, and this is the first run of it outside
its own worktree: of 67 incomplete documents, **56 are the file's** (85 mechanisms — 25
show operators a page could not draw, 18 Identity CMaps over unembedded programs, 12
tokens §7.8.2 admits as neither operand nor operator, and eleven smaller classes),
**9 are neither one's** (a font with no glyph for any code its page shows, a bound this
program set, a transparency model this tree departs from where the two can differ) and
**2 are this reader's**. The number is the same 67 it was; what is new is that nothing
about its composition is now kept by hand.

## §5

All eight release artifacts rebuilt in one invocation and installed into `main`'s
`target/` from the directory `cargo metadata` names: `pdf-viewer`, `pdf-sandbox-worker`,
`pdf-view-worker`, `pdf-viewer-gtk`, `pdf-viewer-qt`, `pdf-viewer-confined`,
`pdf-retrieve`, `libviewer_ffi.so`. `tools/round.sh` had been reporting `target/`'s
binaries older than `HEAD` for several rounds and now reports them at least as new.

## §4 sweeps, against a pre-merge baseline

Pointers, quotations and **parts** were run on `babb3f40` before the first merge and
re-run after the merges, §5's install, the worktree closes and this file's own creation.
Parts is here rather than in 793's pair because 797 corrected four cardinals counting
this tree's own backends, which is precisely that sweep's population. Deltas, every one
accounted:

- pointers: 8960 → 9020 path pointers (+60: live 5101 → 5118, unrooted 3042 → 3075, a
  form 195 → 196, in another crate 22 → 23, not carried 502 → 510); **absent 98 → 98
  unchanged, and the listed hits are byte-identical file for file**, so nothing in the
  batch broke a pointer or retired a target; symbol pointers 158 → 158 with 13 undefined,
  unchanged. The growth is the union of what the four rounds each accounted in their own
  worktrees — ADRs 0728–0731, four history files, 794's `errata-read.md` and `doc/todo/01`
  sections, 795's trap 22 and its `doc/todo/15`, `doc/state-of-play.md` and
  `doc/HANDOVER.md` edits, 796's new `doc/todo/18` and `doc/todo/README.md` row, 797's
  `doc/performance.md` and `doc/todo/45` — plus this file.
- quotations: 6669 → 6703 quotations in 1070 → 1080 documents (+34 in +10 documents: the
  four ADRs, the four sibling history files, `doc/todo/18` and this one); verbatim 2789 →
  2801, **diverging 38 → 38 unchanged**; ledger-note quotations 1969 → 1974 over the same
  794 notes, verbatim 1505 → 1509 and **diverging 2 → 2** — the fifteen rows 794, 796 and
  797 rewrote added four verbatim quotations of the standard and no divergence.
- parts: 586 → 580 cardinals govern one of this tree's own parts (39 → 39 the workspace
  agrees with, 547 → 541 it does not) — **closest rung 52 → 48**, ledger or undated
  162 → 162, dated or inside the population 333 → 331. The whole of the rung-1 fall is
  `crates/pdf-render/src/paint.rs`, from eight hits to four, which is exactly 797's four
  corrections in the crate the backend population depends on; the other two are in the
  unlisted pool, net of what four new ADRs contributed. Four hits remain in that file and
  are the next reading, `grey_level`'s "both rasterisers offer" at their head.

## The batch, synthesised

- **794** — the errata rule's twelfth use, ADR 0728, and its fourth consecutive use that
  confirmed its rows and paid nothing on the ranking's own head. The walk downward paid
  instead: **issue #90 is a caret with no strikeout** — it inserts *providing the
  Contents key, if* after the opening *When* of §12.5.6.2's paragraph rule, so a sentence
  written in the passive names the act of writing the entry, and the line-feed tolerance
  this tree had argued as an inference about whom the `shall` binds becomes the clause's
  own scoping, in the popup window and in a free text annotation's layout alike. Issue
  #297 re-dates the same row's grouping sentence to PDF 1.5 and marks `/RT` as the PDF
  1.6 part. Twelve issues left the population, 85 → 73; ten ledger rows; no executable
  line moved. **Also `tools/worktree.sh`'s gitlink guard**, above — found because the
  round tripped the hazard itself and had to amend a commit to put two gitlinks back.
- **795** — `doc/todo/15`'s longest-standing remainder carried out, ADR 0729: the
  owner's *warn, allow an abort, do not block* reaches the three established windows
  through `viewer_host::keys`. `drawing::WARN` is a second — three times the slowest
  legitimate first page over `doc/pdf.js`'s 957, measured this round rather than quoted —
  and it decides when a *sentence* appears, never an interrupt; Escape aborts only while
  the window is saying so. **The finding on the way in: `viewer-qt` was swallowing Escape
  entirely.** A `QAction` shortcut consumed the key before `keyPressEvent` saw it, so the
  shared key table was reached only in full screen and §12.4.2's clear-selection had never
  worked in that host. That is **trap 22** — a shared key table is only as level as the
  narrowest path a key takes to reach it — and it is why the round's own new verb was the
  thing that exposed a two-year-old one.
- **796** — what `incomplete` is made of, ADR 0730: the population is many mechanisms
  dominated by one class, the great majority of it the file's own defect, and the largest
  single mechanism is the sentence of §9.7.5.2 that says an Identity CMap over an
  unembedded descendant *shall not be used*. The composition is now **printed by the
  gate**, exhaustively, instead of being kept in a comment that had drifted to three
  different figures for one quantity. `pdf-font`'s `composite::collection_gap` splits four
  facts a single message had conflated, and `FontError::NoSubstitute` says which. New
  `doc/todo/18`: Z_SYNC_FLUSH streams reported damaged that are not.
- **797** — the strip replay's image memo, ADR 0731: `render-cpu` replays the whole
  display list into every strip and `pdf_render::replay_ratio` bounds that replay by the
  *rows* a command covers, which is exact for a fill and blind to `Image::area_averaged`,
  whose cost is per source sample. A scanned page therefore reduced the same image once
  per strip and got **slower the more strips it was granted**. A memo keyed by the
  samples' pinned address plus a warm pass on the planning thread: **−42.1% program
  instructions on `issue12963.pdf` page 1, byte-identical over 957 pages.** Four cardinal
  corrections came with it — `pdf-render`'s own `Stroke` documentation said "both
  backends" twice where this workspace states three rasterisers, all three of which call
  the function. **And the rule that had to be learned rather than written: a lock may be
  taken to read or write a map and may never be held across work that itself uses the
  pool.** An `Arc<OnceLock<_>>` per key measured better and deadlocked — a worker waiting
  inside `par_chunks` can have another strip's job stolen onto its own stack — and two
  earlier hangs in that round had been written off as machine load. No gate hung in this
  round's sequence.

## Owed, standing

- CI verdict awaits the owner's push; `origin/main`'s red is pre-existing and `main` here
  is far ahead and unpushed, deliberately.
- `doc/rfc/` awaits the owner's review — untouched this batch, kept so.
- QUORRA_FEEDBACK §40 pending.
- `doc/todo/15`'s remainder after 795: breach-as-refusal, moving the established windows
  onto the confined boundary, and ADR 0725's real-adapter bring-up/present measurement,
  which is the owner's session. 790's observation stands with it and was deliberately not
  folded into 795: the confined screen's `Content::Refused` outlives a zoom, and the
  device path has no interrupt to offer.
- The owner's `git stash drop` of the known-dead entry is still owed
  (`doc/environment.md`'s standing note).
- 796's noted departure, recorded and not taken: `sci-notation.pdf`'s `1e2` is read as
  100 by the lexer's `parse::<f64>()` fallback, and §7.3.3 admits no exponent in either
  numeric form. It costs no report and no mark on that page.
- 797's named remainder: `render-gpu` still recomputes per draw, §11.6.5.2's deferred
  soft-mask image has no address that outlives a draw, and `replay_ratio` still has no
  term for a command whose cost is not in the rows it covers.

Worktrees r794–r797 closed with `tools/worktree.sh close 794 795 796 797` — the merged
script, checkouts and build directories together; verified with `list`, which reports
`main` alone.
