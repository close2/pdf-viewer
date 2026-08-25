# 742 — The fact a window had, and did not hand back

A general-improvement round, chosen rather than assigned. It took `doc/todo/41`'s remainder and
closed it (ADR 0646), and found that the "one hop" the item was priced at was one fact short.

## Why this item

Three constraints on the choice: not the `partial` ledger rows, not the oracle's rankings, not the
confinement boundary — three siblings had those. What was left is wide, and this was picked on one
argument: **it is a live regression of the tree's own making, priced by the round that made it, on
a witness that can be rebuilt to the byte.** ADR 0587 gave a bomb behind an ASCII armour a chain
pump and took its peak resident memory from 1 070 828 KB to 22 608 — and in the same table
recorded the wall clock going the *other* way, 154.98 µs to 14.62 s, because a refusal the
buffered route remembers is one a window re-reaches. That is a 25 000× cost this tree shipped
twenty-eight rounds ago, in a class of input principle 3 is specifically about.

The briefing's steer was to re-derive any price depended on, and this round's first act was to
rebuild ADR 0586's generator and confirm the arms: 4 174 537 and 12 523 517 encoded bytes, both
witnesses at 13.71 and 13.78 s on the current tree. The price held.

## What was found that the item did not say

`doc/todo/41` said the refusal a window reaches "is the *same* `FilterRefusal::TooLarge` under the
same key as the one the buffered route reaches", and that the fix was that fact travelling one hop.
The first half is true and the second is one fact short.

**A window hands over everything up to the bound and only then says it stopped.** So "too large" on
its own is not something a second read may be answered from: a stream whose prefix marked the page
owes those marks to every later read, and remembering a refusal there would make the page a
function of whether the cache still held an entry. This tree had already written that rule down for
the other half of the same memo — `Outcome::Decoded`'s `damage` field — and nobody had carried it
across.

What travels is **too large *and empty***: the bound was reached and the run added not one
operator. `Interpreter::run` is the one place both halves are visible at once, which is what made
the hop land there rather than inside `pdf-syntax`.

## What moved

| twenty pages, one hex-armoured form | before | after |
|---|---|---|
| 4 174 537 B encoded | 14.32–17.99 s, peak 22 292–22 436 kB | **186.16–287.23 µs**, peak 22 200–22 316 kB |
| 12 523 517 B encoded | 14.24–16.89 s, peak 55 108–55 248 kB | 14.18–16.47 s, peak 55 284–55 352 kB |

Three runs an arm, alternating, both built in one sitting from one patch, `RAYON_NUM_THREADS=1`,
peak sampled from `VmHWM` every 20 ms. **The load average ran 6 to 18 throughout**, from three
parallel rounds, which is why the wall clock is a range; the peaks are deterministic and they match
ADR 0587's after-column, which is the second thing saying this is the same witness. The second row
is unchanged on purpose — ADR 0586's refusal for a stream whose *encoded* bytes exceed the budget,
still standing.

`callgrind` over ISO 32000-2's 1023 pages: 37 299 983 484 → 37 301 137 814, **+0.0031%**. The sign
is honest rather than hoped-for: `Document::pumping` now hands back the chain it read, so the route
question and the memo's key cost one pair of reads instead of two, and the memo lookup plus one
chain clone are slightly more than that saves.

## The sequence

Whole, this being a change in `pdf-syntax` and `pdf-model`. `fmt` clean · `clippy --workspace
--all-targets` under `RUSTFLAGS="-D warnings"`, exit 0 · doctests · the `fuzz/` check · `nextest`
**2642 passed, 18 skipped** · both workers built first · corpus gate ok · `pdfref-hayro` built ·
oracle ok in 31.61 s against the shared warm `PDFREF_CACHE` (2.5 GB), machine at load 11.63 ·
text extraction **98.26%**, 486 of 508 documents in bounds · selection and accessibility censuses ·
dates · XMP · JPEG 2000 · `render-quorra` corpus · `fixed_documents` 40 of 40 · `cargo test -p
conformance`.

**`doc/todo/00`'s step 7 was not re-run and this says why**: no corpus page changed. The change can
only alter what is drawn for a nested content stream that reaches `Limits::max_stream_len`, which
none of the 974 does — the corpus gate, the oracle and the quorra corpus all report unchanged, and
the display lists are the same display lists.

§4's sweeps: `overstated`, `overtaken` and `tables` run; nothing they printed is this round's.

§5's six binaries and `libviewer_ffi.so` built and installed — `target/` held none of them when the
round started, so nothing a person could run was there at all.

The whole sequence was run **twice**: once for the change, and again after `nested_content_source`
was restructured to ask the memo under one write guard instead of two acquisitions. The second
run's figures are the ones above.

## Fuzzing

`page` — the target whose binary contains `pdf_model::interpret`, and therefore the content reader,
the window and the pump — twice, and the second run is where the honest sentence is.

The first was `-fork=4 -rss_limit_mb=4096 -timeout=60` for fifteen minutes over the **worktree's
own** corpus, which is 19 files: **62 170 311 executions, 0 crashes, 0 OOMs, 0 timeouts**, and
`fuzz/artifacts/page/` empty. `fuzz/corpus/` is machine-local and a worktree starts without one,
which is worth knowing before reading a coverage figure off such a run — `cov: 114` there against
261 331 counters is the corpus talking, not the target.

The second seeded that directory from the main tree (9188 files, 653 MB) and ran the same
invocation. **It spent its whole budget inside libFuzzer's fork-mode merge and never reached the
fuzzing phase**, which is `doc/todo/02` §2's own warning arriving — a `cmin` keeps the seeds with
distinct coverage, which are the large slow documents, and the rate falls as the merge goes. It
executed **8704 of the 9188 seeds** against this tree under the sanitiser before it was stopped, at
0 crashes, 0 OOMs and 0 timeouts, with no artifact written. That is a real check and it is not the
check the invocation names; a round with three hours rather than one should run it to the end.

## Ledger

§7.4, §7.4.1 and §7.8.2, with two new tests on each of the first and the last.

## Tests

`a_window_that_found_nothing_refuses_the_read_after_it` ·
`a_window_that_drew_something_is_read_again_rather_than_remembered` ·
`the_routing_question_answers_under_its_bound_and_moves_neither_tally` ·
`the_routing_question_answers_only_for_a_bound`. **Both halves planted before they were believed**
(trap 13), against a scratch copy of `run.rs`: with the hop removed the first fails on the route,
with the emptiness guard removed the second fails on the second read's display list, and the file's
other five pass under both plants.

## The one thing found beside the item

`doc/todo/README.md`'s line for item `39` described a state that ended five sessions after the item
did: it said `tools/state.sh annex-o` "says which are carried out and which are reported", and named
"a fetch … and a concept the vocabulary lacks" as what the rest needs. The command reports **none**
as reported, `doc/todo/39` has said `done` since the five-hundred-and-ninety-sixth, and the two
limits that file actually names are XFDF's XML parser and a host's import policy. Corrected. It is
that index's own warning arriving — "a summary here that restates them is a second copy to keep in
sync" — and the file it summarised was right the whole time.
