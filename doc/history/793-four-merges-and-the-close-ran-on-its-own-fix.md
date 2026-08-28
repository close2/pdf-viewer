# 793 — Four clean merges, and the close step ran on its own fix

Merge round, 2026-08-28, on `main` from `f81e038f`. Merged `round-789` (`9caa52df`),
`round-790` (`519af5cd`), `round-791` (`7eef2243`), `round-792` (`416eff87`), in round
order, each with `--no-ff`; all four clean, **including `doc/conformance/ledger.toml`**,
which 789 (§7.5.x/§7.7.2/§14.x families) and 792 (§9.6.3) touched on disjoint rows — the
ort strategy reconciled them, and the ledger binary re-run on the merged file printed no
diff, so this batch has no reconciliation commit at all. Then the full §2 sequence on
merged `main`, §5's install, the §4 sweeps against a pre-merge baseline, and the worktrees
closed — by the *merged* `tools/worktree.sh`, which matters this time (below).

## ADR verification

`main` ended at 0723 pre-merge; the batch brought 0724 (789), 0725 (790), 0726 (791),
0727 (792) — one per round as the briefings reserved, no collision, nothing at 0728 or
above.

## Gates (full §2 sequence on merged `main`, quiet machine)

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy, `-D warnings`, all targets | clean |
| `cargo nextest run --workspace` | 2740 passed, 18 skipped |
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

The two watched ratchets both held on the merged tree: 791's moved text-position ratchet
(487 of 508, up from 486) and 792's new `fixed-documents` row
(`7803013.pdf` p1 ink 18.469, in band, the 41st row). The nextest count is the exact
arithmetic of the batch — the base's 2728 plus 789's two tests, 790's eight and 792's
two; 791 added no workspace test. **No confined test moved beside 792's font change**:
the briefing asked for attribution if one did, and there is nothing to attribute — all of
`viewer-confined`'s and the confined window's tests passed unchanged. The launch test
ran inside nextest and passed; load-robust since 776, so a pass is the expected news.
The oracle census is byte-for-byte the branches' own (983/61/836), and quorra's four
figures match 789's and 791's runs, so 790's device work and 792's font change moved
nothing on the default lane.

## §5

All eight release artifacts rebuilt in one invocation and installed into `main`'s
`target/` from the directory `cargo metadata` names: `pdf-viewer`, `pdf-sandbox-worker`,
`pdf-view-worker`, `pdf-viewer-gtk`, `pdf-viewer-qt`, `pdf-viewer-confined`,
`pdf-retrieve`, `libviewer_ffi.so`.

## §4 sweeps, against a pre-merge baseline

Pointers and quotations were run on `f81e038f` before the first merge and re-run after
the merges, §5's install, the worktree closes and this file's own creation. Deltas,
every one accounted:

- pointers: 8925 → 8960 path pointers (+35: live 5081 → 5101, unrooted 3028 → 3042,
  a form 194 → 195); **absent 98 → 98, in another crate 22 → 22, not carried 502 → 502 —
  all unchanged; symbol pointers 157 → 158 with 13 undefined, unchanged.** The growth is
  the union of what the four rounds each accounted in their own worktrees — ADRs
  0724–0727, four history files, 789's errata-read section and `doc/todo/01` addition,
  790's `device.rs` and todo/15 rewrite, 791's classification note, 792's todo/03
  settlement — plus this file; none added or retired a pointer target.
- quotations: 6648 → 6669 quotations in 1061 → 1070 documents (+21 in +9 documents: the
  four ADRs, the four sibling history files and this one); verbatim 2783 → 2789,
  **diverging 38 → 38 unchanged**; ledger-note quotations **byte-identical at
  1969 / 1505 verbatim / 2 diverging** — 789's eight reworked rows and 792's §9.6.3
  moved no quotation count, which is the merged ledger telling the same story as the
  formatter's empty diff.

## The batch, synthesised

- **789** — the errata rule's eleventh use, ADR 0724: issue #105 inserts *or invalid* into
  Table 29's `/Lang`, so an invalid catalog language tag is unknown exactly as an absent
  one — `structure::document_language` now filters through BCP 47 well-formedness (the
  grammar, deliberately not the registry), and §14.9.2.2's conflated reason of record is
  retired. Fourteen issues left the population, 99 → 85, the rule's largest single decay.
  **Also `tools/worktree.sh`**: root resolution moved from `--show-toplevel` to the parent
  of `--git-common-dir`, because the old one, run from inside a worktree, called every
  live sibling's build directory orphaned and a `close` trusted the same wrong root — an
  `rm -rf`-wide data-loss footgun, and exactly the hazard a merge round's close step
  springs. This round's close step ran on the fixed script, from the main tree, and took
  all four checkouts and build directories cleanly.
- **790** — the device behind the confined window, ADR 0725: `pdf-viewer-confined` now
  presents through `render-quorra` (render thread, one job in flight, pooled texture
  pair), with CPU fallback on a device refusal out loud and the flagship's `--cpu` flag;
  the confinement moved nowhere — the worker still ships display lists over the pipe and
  the device is the host's, which is what ADR 0607 argued the marks cross for. **The
  finding on the way in**: the `Arc` identity two documents promised did not survive the
  pipe — every `Query::Frame` decoded a fresh `Arc`, so the documented scroll reuse fired
  only in unit tests. `protocol::HeldLists` now re-hands the same `Arc` while a page's
  bytes are unchanged, byte equality and nothing weaker, bounded by the frame on hand.
- **791** — the selection-geometry tail read as a population for the first time since ADR
  0421, ADR 0726: seven mechanisms over the out-of-bounds documents, each priced by its
  failing bound in the note above `SELECTION_BELOW_FLOOR`; the gate now prints the
  per-document failing bound and a population summary. And a pair is judged in the word's
  own reading frame — a `/Rotate 90` page swaps which axis is convention — which took
  `hello_world_rotated.pdf` off the ratchet list: 486 → 487 of 508, held here on the
  merged tree.
- **792** — the font whose bytecode is the artwork, ADR 0727: a TrueType face whose glyph
  shapes are computed by its instruction programs (DFKai-SB's stroke-component skeletons)
  was drawn from the uninstructed `glyf` data; `LoadedFont` now builds a cached
  `HintingInstance` at one pixel per design unit for the hint-reliant family, so shape
  construction runs and grid-fitting stays declined, resolution-independent for the glyph
  cache. Witness ink 16.28 → 18.51 at 8× against references at 18.52; the row is the 41st
  in `fixed-documents.toml`. The other three crawl-head rows settled as decisions already
  argued, with by-construction probes recorded. **Also `tools/state.sh`**: `find -L`, so
  symlinked corpora in worktrees are counted rather than read as 0.

## Owed, standing

- CI verdict awaits the owner's push; origin/main's red is pre-existing and `main` here
  is far ahead and unpushed, deliberately.
- `doc/rfc/` awaits the owner's review — untouched this batch, kept so.
- QUORRA_FEEDBACK §40 pending.
- `doc/todo/15`'s remainder: the warn-before-abort input for the three established
  windows, the breach-as-refusal item, moving the established windows onto the boundary,
  and the real-adapter bring-up/present measurement owed to the owner's session.
- The owner's `git stash drop` of the known-dead entry is still owed
  (`doc/environment.md`'s standing note).
- 790's observation, left for a round in that area: the confined screen's
  `Content::Refused` outlives a zoom — a refusal at one magnification is kept for any
  later list payload of the page.
- 791's named remainders: `vertical.pdf`'s quad convention (§9.7.4.3's vertical
  displacement as the reading-axis extent) and `issue6127.pdf`, the one tail document
  where both references agree against this tree.

Worktrees r789–r792 closed with `tools/worktree.sh close 789 790 791 792` — the merged
script, checkouts and build directories together; verified with `list`, which reports
`main` alone.
