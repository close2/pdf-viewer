# 773 — The numbers the arc had spent, and the page it priced

Merge round: four branches, all based on `2f6deace`, merged into a `main` that had moved under
them — the owner-merged GPU arc (`2f6deace` → `14eafaaf`), which consumed ADR numbers the
briefings had assigned, left CI red on two lints, and priced one corpus page past a budget. Each
of those three is this round's subject; none was any branch's fault, and every attribution below
is by measurement rather than inference.

Date: 2026-08-28.

## Bare main first, so post-merge failures attribute correctly

Run at `14eafaaf` before any merge: `fmt` clean; `nextest --workspace` **2690 of 2690 green**
(the load-sensitive `viewer-host` launch test among them); `clippy` under `-D warnings` **red on
exactly two lints** — `float_cmp` in the arc's `stale.rs` test
`a_scroll_stand_in_lands_on_whole_pixels`, asserting that a scroll stand-in's placement snaps to
whole device pixels. The comparison is exact by construction — snapping is the behaviour under
test — so the fix is the `#[expect(clippy::float_cmp, reason = …)]` pattern `access.rs` already
uses, applied as `7525df24` before the first merge. That was the whole of the red round 770 had
reported on the new `main`, on the lint side; the other half was the quorra pin, below.

## The ADR numbers, renumbered on the branches before merging

The arc carried `main`'s ADRs through 0707, so three of the four branches collided and one had
already dodged:

| round | wrote | collides with | renumbered to |
|---|---|---|---|
| 769 | 0698 | the arc's compute-rasteriser candidate | **0709** |
| 770 | 0701 | the arc's stroke-width decision | **0710** |
| 771 | 0708 | nothing — it resumed after the arc and took the first free number itself | kept |
| 772 | 0707 | the arc's zoom-is-a-view-change | **0711** |

Each renumber is a commit on the round's own branch, with every in-branch reference moved — ADR,
history file, ledger notes, todo files, `tools/state.sh`, and doc comments. The one that would
have survived a naive `grep "ADR 0707"` was a citation split across a comment line break in
`viewer_host::panel` (`(ADR` at one line's end, `0707)` at the next line's start); grep the bare
number, not the phrase. The briefing predicted only 770's and 772's collisions — 769's ADR was
not in its own briefing line, and the tree won.

## The merges: four, in round order, zero conflicts

`round-769` through `round-772` merged clean — including `doc/conformance/ledger.toml`, which
three branches edited in disjoint rows and git resolved without a conflict marker. No decision of
the kind 629 had to make arose; the batch's subjects (a sweep, a cache, a lexer rule, a panel)
share no line. `git log` on `main` now reads 528eb077, 17a34c26, d9d7046d, 122129ea.

## The gate the merge could not green, and what decided it

The full §2 sequence on merged `main` failed one line: the quorra corpus gate's refusal ratchet,
on **`issue1905.pdf` arriving refused at 1×** — *frame needs 272158852 scene-derived bytes, over
the stated budget of 268435456*, 1.4% over. Attribution by run, not by reading: the same gate at
`7525df24` (the arc's head plus the lint fix, the four rounds absent, the tree rebuilt there)
prints the identical refusal byte for byte. The arc took quorra's `5fb011a` and built the scene
in page space (ADR 0702); the corpus's heaviest page — already device-refused at 4× for the
16384² coverage-sheet ceiling — crossed the frame-byte budget at its own resolution.

The pin's own doc prescribes what a device-stage arrival means and what the finder does, and
that is what was done (`a89e9ab3`): the name is in `REFUSED_BY_THE_DEVICE` with the run's
message, the report is upstream as `doc/QUORRA_FEEDBACK.md` section 40 — which deliberately asks
for no budget increase, *a budget raised to admit one page is a budget chosen by that page* —
and the CPU backend draws the page, reported out loud, which is `CLAUDE.md`'s rule for a budget
refusal. The arc's quorra release also owed §2's second coverage lane, run here as the survey it
is: `COVERAGE=gpu` at 4× — **939 agree, 10 differ, 3 refused, 22 not comparable**, no byte
refusal on that lane.

## The sequence, whole, on a quiet machine

| | |
|---|---|
| `fmt --check` | clean |
| `clippy --workspace --all-targets`, `-D warnings` | silent (the documented cold-build gcc lines from `viewer-qt` only) |
| `nextest --workspace` | **2706 run: 2705 passed, 1 failed, 18 skipped** — the failure below |
| doctests | clean |
| fuzz `check`, `-D warnings` | clean |
| corpus | 974 documents: 0 unopenable, **67 incomplete**, 0 slow |
| oracle | 1945 pages — **983 agree, 61 contradicted, 836 ambiguous**, ratchets held, 54 s |
| text extraction | **98.26%** (10969/11163 words), 486 of 508 documents; PDFBox frozen 99.8% |
| selection census | clean, 0 panics |
| accessibility census | ratchet holds — 57116 caret elements, 0 disagreeing lines, 876/876 honest empty |
| dates, XMP, JPEG 2000 | clean |
| `render-quorra` corpus | 957 pages — **932 agree, 22 differ, 3 refused, 17 not comparable** (green after the pin) |
| `fixed_documents` | **40 checked, 0 absent, 40 rows** — first run over a tree holding all four rounds |
| `cargo test -p conformance` | 11560 citations, 1087 quotations verbatim, 0 unreviewed |

**The one failure is the known launch flake, and both halves of its A/B are recorded.**
`viewer-host drawing::tests::a_launch_waits_for_page_one_instead_of_polling_for_it` (16.7 ms
settle budget) failed beside the full parallel suite and **passed 4 of 4 runs alone**; it had
also passed inside bare `main`'s full run the same day. Same shape 770 and 772 recorded; the
772 history file carries the diff-reverted A/B. Load-shaped, not tree-shaped.

The citation checker earned its keep once during the round: the pin's first doc comment wrote
`QUORRA_FEEDBACK.md §40` and the checker refused the `§` — a section sign here is checked
against ISO 32000-2's clauses and would pass by landing on one. It reads `section 40` now.

## Sweeps, with the deltas accounted

Run after the sequence, never beside it. `--bin parts` (769's own, its second run ever): the
closest rung — the one that gates by argument — is **unchanged at 52**; the lower rungs grew
159 → 162 and 323 → 333, which is the arc's five todo files and this batch's own ADRs and
histories entering the prose population, exactly the growth the sweep expects of new dated
records. `pointers`' loudest paragraph is a correction quoting the pointer it retired
(`policy.rs`'s own doc comment records the fix), the sweep's documented oldest false positive.
`quoted` and `unpriced` over this run's oracle log: no contradicted page unaccounted.
`spec-errata check` after 771's row moves: 0 struck passages current, 0 quotations of struck
text. `ledger`, `tables`, `counts`, `entries`, `unread`, `blockers`, `capabilities`,
`inapplicable`, `owed`, `overstated`, `overtaken`: at their documented noise levels, nothing
naming this batch's work.

## What 769–772 delivered, as one batch

- **769** — `--bin parts`, the twenty-second sweep: a cardinal governing one of this tree's own
  parts (backends, crates, hosts, submodules, workers) checked against the workspace's own
  membership, with the place-rung derived from the manifests. Built one of 767's two proposed
  instruments and declined the other with numbers. Tools only; no pixel could move.
- **770** — a font outlives the page that loaded it (ADR 0710): `FontCache` beside the document
  in `viewer_core::Open`, keyed by `ObjectId` bound to the document's bytes, 2 MiB LRU budget
  derived by `examples/font_cache_budget`; **−14.86% of interpretation instructions** on
  multi-page documents by `examples/callgrind_pages`, built because `callgrind_interpret`
  repeats one page and so contains the repetition a cross-page cache removes.
- **771** — §7.3.4.2's end-of-line rule in literal strings (ADR 0708): an unescaped CR, LF or
  CRLF inside a literal string is **one byte, 0Ah** — unimplemented under an `implemented` row
  whose note enumerated everything else, found through Errata Collection 3's Issue #276 making
  the literal string one of a byte string's two written forms. One lexer serves file body and
  content streams, so this could move a pixel, and that round ran §2 whole.
- **772** — §12.3.5's collections reach all three windows (ADR 0711):
  `viewer_host::panel::collection_rows` toolkit-free, both native hosts asking
  `Query::Collection` before `Query::Attachments`; and a defect on no list — `viewer_ui::chrome`
  had dropped every `<n>`-tagged file of a collection with no `/Folders`, and any file whose key
  names a folder the tree does not state, since ADR 0202. Two sentences of the clause decide
  both; fixed in both copies.

Not a merge decision among them — but the batch shape 629 predicted held again: the finding of
this round (the pinned page) came from one tree's work meeting another's, visible from no branch.

## Closed

Worktrees r769–r772 and their build directories removed as one act (`tools/worktree.sh close
769 770 771 772` — the script prefixes the `r` itself; `close r769` is a silent no-op that
still prints success, which is worth a trap-shaped sentence somewhere if it bites twice).
`tools/worktree.sh list` shows `main` alone. §5's binaries: not owed — not a fifth round, no
measurement taken. Build directory at 80 GB, under §5a's line.

## Owed

- **The upstream answer to `doc/QUORRA_FEEDBACK.md` section 40** — whether 272 MB of
  scene-derived bytes for `issue1905.pdf` at 1× is the page-space encode spending as intended,
  or more than it means to spend. Until then the page is drawn by the CPU backend, loudly.
- **CI on the merged `main`** — the two local reds (lints, pin) are fixed here; whether the
  runner agrees is the next push's to see.
