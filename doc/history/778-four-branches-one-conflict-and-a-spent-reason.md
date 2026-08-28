# 778 — Four branches, one conflict, and a spent reason

Merge round: rounds 774–777, all based on `03316582`, merged into `main` in round order.
One conflict, called out in advance and resolved by argument; every other contact
auto-merged and was then read rather than trusted. Date: 2026-08-28.

## The merges

`round-774` and `round-775` merged clean. `round-776` conflicted exactly where the briefing
said it would: `crates/viewer-host/src/drawing.rs`, where 775 had made `Drawing` generic over
a new `DrawRequest` trait and moved `POLL`/`SETTLE` to module constants while 776 rewrote the
launch test in the same lines. **Resolved by argument: the merged tree keeps 776's structural
test running against 775's generic type.** Concretely, 776's test body was taken whole — the
expensive page (`amplified(&request, 5_000)`), the single `settle` call with the give-up bound
as budget, no loop, no sleep, no wall clock in any assertion, and the final
`drawing.spent > Duration::ZERO` check guarded on `link.is_some()` — and its doc comment's two
`[`Drawing::SETTLE`]` links became `[`SETTLE`]`, because 775 moved the constant out of the type
(an associated constant on a generic type cannot be named without choosing an `R`, and the
number is the same for every one). 776's three calibration properties were re-read against the
generic type before committing and all hold: one settle call is the requirement, the page
provably cannot finish in the `ask`-to-`settle` gap, and nothing in the assertions reads a
clock.

`round-777` then auto-merged everywhere the briefing worried — `pdf-model/src/structure.rs`
(774's §14.7.6.2 work and 777's §14.8.5.7 additions are disjoint regions),
`doc/conformance/ledger.toml` (disjoint rows; the ledger binary was run afterwards as the
formatter it is and changed nothing), and `doc/state-of-play.md`. 775's and 777's confined-wire
edits were disjoint as predicted: 775 built the host consuming the protocol, 777 added the two
accessibility fields, and the fuzz `confined_wire` target compiles against the union.

**ADR numbers**: 0712–0715 present, one per round as pre-reserved, no duplicate numbers
anywhere in `doc/adr/`, nothing at 0716 or above.

## The sequence, whole, on a quiet machine, on merged `main`

| | |
|---|---|
| `fmt --check` | clean |
| `clippy --workspace --all-targets`, `-D warnings` | silent, exit 0 |
| `nextest --workspace` | **2717 run: 2717 passed, 18 skipped** — no failure at all |
| doctests | ok (the one doctest nextest does not run) |
| fuzz `check`, `-D warnings` | clean — `confined_wire` among the bins |
| corpus | 974 documents in 2.6 s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow |
| oracle | 1945 pages in 53.3 s: **983 agree, 61 contradicted, 836 ambiguous, 42 not comparable**, ratchets held |
| text extraction | **98.26%** (10969/11163 words), 486 of 508 documents; all three gates green |
| selection census | clean, 0 panics |
| accessibility census | ratchet holds — 102853 elements reached, 0 disagreeing lines, 876/876 honest empty |
| dates, XMP, JPEG 2000 | clean |
| `render-quorra` corpus | 957 pages in 26.8 s: **932 agree, 22 differ, 3 refused, 17 not comparable** |
| `fixed_documents` | **40 checked, 0 absent, 40 rows** |
| `cargo test -p conformance` | 11586 citations, 1089 quotations verbatim, 0 unreviewed |

**The launch test passed inside the full parallel run, which is the news this table carries.**
`viewer-host drawing::tests::a_launch_waits_for_page_one_instead_of_polling_for_it` failed
beside the loaded suite in rounds 770, 772 and 773 — the old shape asserted that 16.7 ms of
wall clock sufficed, which measured the machine. 776's rewrite asserts the waiting
structurally, and on this round's full `nextest` run it passed in place (and again alone,
0.6 s). That is the first merged full-sequence run since the rewrite, and it behaved exactly
as ADR 0714 designed: the flake is not load-shaped any more because there is no clock left to
lose.

## §5, owed to this round by 775 and 777 both

Rebuilt and installed in one invocation into the main tree's `target/`, with the build
directory taken from `cargo metadata` as §5 requires (it answered
`/home/AI/cargo-target/pdf-viewer/release`): `pdf-viewer`, `pdf-sandbox-worker`,
`pdf-view-worker`, `pdf-viewer-gtk`, `pdf-viewer-qt`, **`pdf-viewer-confined`** — the set's
new member, 775's binary, installed for the first time — `pdf-retrieve`, and
`libviewer_ffi.so`.

## Sweeps, with the deltas accounted

Run after the sequence, never beside it, and diffed against a pre-merge baseline (r776's
checkout, base plus one test edit) so that every moved figure attributes:

- **`owed`**: 223 → 222 `partial` rows and 3939 → 3928 terms — 777 moving §14.8.5.7 to
  `implemented`. 180 → 182 named-by-no-source: the two are `DefaultCryptFilter` and
  `DefEmbeddedFile`, named by 774's Errata #89 sentence in §7.6.4.1's note and deliberately
  reached by nothing ("behind §7.6.5's refusal"), which is the sweep's own definition of a debt
  named in a word. §7.6.4.1 accordingly left the fully-named list (111 → 110).
- **`entries`**: 177 → 176 reported entries over 51 → 50 rows — the same §14.8.5.7 move.
- **`inapplicable`**: 288 → 292 stated terms, all four named with non-inapplicable cousins —
  774's Errata #69 paragraph added to §14.5's note (Table 350's widened key column), the
  documented noise shape and not a defect.
- **`tables`**: 6771 → 6805 sentences naming a table, attributed citations 2491 → 2514, **all
  23 new ones agreeing** — 774's §14.7.6.2 and 777's Table 371 citations; absent stayed 101.
- **`unread`**: identical headline (68 rows, 179 keys); the §14.12.4.1 `/Lang` hit predates the
  batch.
- **`parts`**: identical — 52 on the closest rung, 162, 333; workspace still 3 backends,
  25 crates, 3 hosts, 6 submodules, 2 workers. 775's window is a binary inside `viewer-ui`, so
  no part population moved, which 775 had already established by diffing on its own branch.
- **`callers`**: 194 → 196 names asked by a dependent crate — the confined host consuming
  entry points; nothing came off the bottom rungs.
- **`pointers`**: 8672 → 8712, growth all live or unrooted prose; absent (98) and undefined
  symbols (13) unchanged.
- **`quotations`**: 6510 → 6550, verbatim 2752 → 2765, diverging unchanged at 38.
- **`counts`, `blockers`, `capabilities`, `overstated`, `overtaken`**: headline-identical to
  baseline apart from prose-population growth from the batch's own ADRs and histories.
- **`quoted` / `unpriced`** over this run's oracle log: no contradicted page unaccounted —
  93 failing bounds over 61 pages, 93 named by the note that holds the page; `issue6069.pdf`
  page 1 remains the one page whose printed line rounds inside every bound, as documented.
- **`spec-errata check`** after 774's row work: 0 struck passages current, 0 quotations of
  struck text.
- **`retired`** over the batch's nouns (`/Summary`, `/Short`, `six consumers`, `SETTLE`): found
  the finding below.

## The spent reason, deleted — and the count 775's arrival aged

775 reported that `tools/state.sh windows` prints `Collection SPENT — every window reaches it
now` against a reason row ADR 0711 had rewritten rather than removed. Read against ADR 0603
§4, the rule plainly applies: the reading table explains variants some window does not reach,
its checker's SPENT direction exists precisely for a row that outlived its variant, and the
precedent in the ADR's own text (`Command::Restrict`) is deletion — "its reason had to be
deleted rather than left to describe a debt somebody closed". The script's comment above the
table says the same in one sentence. **The `Query:Collection` row is deleted**;
`tools/state.sh windows` now prints no SPENT and no UNREAD.

The `retired` sweep then found the same decay one layer up: 775 updated
`doc/ui-boundary.md`'s consumer count to seven with the new window named, and two other
current claims still said six. **`doc/state-of-play.md`'s "Six consumers on that boundary" and
`doc/todo/30`'s "six consumers have never asked for a new message" now say seven** — ADR 0713
confirms the seventh added no message, so the rest of each sentence survives. The dated
records ("Six consumers failed to compile", ADR 0509's survey, todo/16's "proved on six
consumers") stay as the history they are.

After these edits the conformance gate was re-run and is green; no Rust source changed after
the sequence ran.

## What 774–777 delivered, as one batch

- **774** — the errata selection rule's ninth use (ADR 0712): Errata Collection 3's strikes
  read where `emit` files them rather than where their headings sit — §7.6.4.1's crypt-filter
  singulars, §14.4's `/ID` loosening (with a quotation and a blockquote that had sat on struck
  words below the sweep's four-word floor), §14.5's second-class-name widening — plus a new
  §14.7.6.2 precedence test and `write.rs` warrant re-anchors.
- **775** — the first host on the confined boundary (ADR 0713): `pdf-viewer-confined`, a
  window whose viewer is on the far side of `pdf-view-worker`'s pipe; `viewer_host::drawing`
  made generic over `DrawRequest` so the one arrangement serves both kinds of host, GTK/Qt
  call sites updated, no new message on any boundary.
- **776** — the launch test made load-robust (ADR 0714): waiting asserted structurally instead
  of timed, the pre-settle polling shape failing deterministically under a planted defect,
  and the flake that cost three merge rounds a re-run retired with the clock.
- **777** — §14.8.5.7's two sentences reach a reader (ADR 0715): Table 371's `/Summary` and
  `/Short` through `pdf-model`, across the confined wire, onto AT-SPI; the ledger row
  `implemented`, todo/31's bullets gone.

The batch shape held again: the two findings of this round — the SPENT row and the aged
consumer count — were each visible only from a tree holding 775's work beside the documents
it aged, which no branch was.

## Closed

Worktrees r774–r777 and their build directories removed as one act
(`tools/worktree.sh close 774 775 776 777` — bare numbers; the `close r769`-style no-op that
prints success is 773's finding and was not repeated). `tools/worktree.sh list` shows `main`
alone.

## Owed

- **CI's verdict on this merged `main`** — the failing run is pre-existing on the owner-arc
  commit `48bb1167`; local `main` is ahead and unpushed, the lints do not reproduce here, and
  the token is read-only for contents, so CI's verdict awaits the owner's next push.
- **The upstream answer to `doc/QUORRA_FEEDBACK.md` section 40** (carried from 773) — the
  device-refused `issue1905.pdf` stays drawn by the CPU backend, loudly, until it comes.
