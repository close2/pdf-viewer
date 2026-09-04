# 0898 — A cost floor on the program the gate spawns

Session 929. Status: **accepted**. The first of this round's two records:
[ADR 0899](0899-three-rows-of-a-survey-that-did-not-survive-being-run.md) is what the counter
found and which of ADR 0895 §3's rows turned out not to be what the survey costed them at.

## Context

[ADR 0894](0894-a-cost-floor-that-counts-because-a-clock-here-is-a-lottery.md) gave `pdf-vfs`'s
two corpus walks a floor made of a **count** rather than a clock, on the argument that a count is
identical whatever the machine is doing — measured, not asserted: 13 793 questions from two runs
whose wall clocks differed by a factor of 1.8. [ADR 0895](0895-what-the-counter-found-and-the-walks-it-does-not-reach-yet.md)
§3 then surveyed the other seven corpus-scale gates and `doc/questions/Q27` put the ranking to the
owner. That question is open, so this round took the rows that need no decision.

Three of `doc/todo/02` §2's lines are the oracle and the two text instruments, and they share one
economic fact: **they are very largely measurements of other programs.** `pdfref::cache`'s own
module comment states it — about 1020 seconds of processor time in `pdftoppm`, `mutool` and `gs`
against 46 in our own pipeline, a ratio above twenty to one. Every one of those seconds is a
process spawned, and nothing in this tree counted them.

## The counted quantity is the spawn, and that is the whole decision

Trap 33's rule is that a property about how often something runs has to count *that thing
running*. `pdfref`'s two caches have reported `Statistics { hits, misses, remembered_timeouts }`
since they were written, and they are the wrong instrument for this by construction:

- **A lookup that never reaches the cache moves neither.** `Cache::entry_for` answers `None` when
  the cache is off, when the document cannot be hashed, or when the renderer cannot be identified,
  and every one of those falls through to `Reference::render` — spawning the program, counted
  nowhere.
- **A second miss and a first miss are the same number.** A page rendered twice by one reference
  in one run reads as two misses, indistinguishable from two pages nobody asked about twice. That
  is the exact shape of the defect ADR 0886 hid for four sessions in another crate.

So the counter is at the two places where the program is actually run, and each cache carries:

- `Runs::ran` — how many times a reference renderer or an extractor was run.
- `Runs::repeated` — how many of those were for a key this run had already run. The key is
  `(program, document path, page, dpi)`, recorded *before* the spawn so that two threads arriving
  at one key cannot both believe they are the first.
- `Runs::unstored` — how many runs produced something the cache kept nowhere. `write_entry` now
  answers **whether it stored**, which it previously did not say and nobody could ask.

`Cache::repeated_keys` names what was counted, so a failing gate is one somebody can reproduce
rather than one they have to re-derive — the same reason `pdf_vfs::Vfs::repeated_subjects` exists.

## The floor

> **A reference program runs at most once per key per run, and the only thing that may make it
> run again is the cache not having kept what it produced.**

`repeated ≤ unstored`, as `Runs::holds`. It is sound **by construction** rather than by
measurement: between two runs for one key, the first run's outcome must have been unreadable from
the cache, and every way of that happening — nothing storable, `write_entry` storing nothing —
increments `unstored`. The ceiling can therefore only be too generous, and a repeat above it is a
program spawned to answer a question that was already answered.

There is no band, no check file and no clock, so a neighbouring round's load cannot move either
side by one. That is the property this whole family of floors exists for.

**The ceiling is what makes it honest rather than merely strict.** `PDFREF_CACHE=off` is how the
oracle proves the cache changes no verdict, and under it every run is unstored; the floor then
holds with both sides moving together instead of failing a run the design asks for.

**One thing fails it without a defect of ours**: two threads missing on one key at the same
instant. That is duplicated work either way, which is why it is not excused, and the corpus walks
measure it at zero.

## Where it binds

| line in `doc/todo/02` §2 | what it now floors |
|---|---|
| `pdf-model --test oracle` | `pdftoppm`, `mutool draw` and `gs` over every judged page |
| `pdf-model --test text_extraction` | `pdftotext -bbox-layout` and `mutool draw -F stext` |
| `viewer-core --test selection_census` | the same two extractors over the selection population |

and `tools/pdfref/tests/end_to_end.rs` carries it at unit scale, which
`cargo nextest run --workspace` runs every round.

## Checked against the defect (trap 13)

A floor is not believed until it has been run against the defect it is for, and session 927's
first proof *passed* because its population could not reach the defect — so the population is
part of the proof here.

`a_page_asked_for_twice_runs_the_renderer_once` asks for one page three times **from two different
work directories**. That second directory is what makes it discriminate: `Reference::command_signature`
replaces the output path with `<out>` precisely so that where the harness puts its artefacts is
not in the key, and that substitution is the cheapest defect this floor is for — it is the dual of
the risk `pdfref::cache`'s module comment is written about, a key that omits a variable against a
key that includes one it should not.

With `argument.replace(&work_dir, "<out>")` taken out, the test fails:

```
assertion `left == right` failed: three requests for one page ran one renderer;
repeated: ["poppler:…/pdfref-runs/basic.pdf:1:72"]
  left: Runs { ran: 2, repeated: 1, unstored: 0 }
 right: Runs { ran: 1, repeated: 0, unstored: 0 }
```

`Statistics` on that same run reads one hit and two misses — a perfectly ordinary-looking line for
a run that spawned poppler twice on one page.

`a_cache_that_stores_nothing_re_runs_the_renderer_and_says_so` is the other half, and it is what
stops the floor from being a rule against a configuration the project uses: a disabled cache reads
`Runs { ran: 2, repeated: 1, unstored: 2 }` and holds, while `Statistics` reads `0, 0, 0` — the
two requests never reached the cache at all, which is the blindness this ADR is about, printed.

## What it read, cold and warm, on the run that took this round's gate sequence

| line | programs run | repeated | kept nowhere | what the cache did |
|---|---|---|---|---|
| `oracle` | 6705 | **0** | 0 | 9 hits, 6705 misses |
| `text_extraction` | 958 | **0** | 0 | 1 hit, 958 misses |
| `selection_census` | 8 | **0** | 0 | 947 hits, 8 misses, 1 remembered timeout |

The three rows are worth reading together, because they are the same floor at opposite ends of its
range. The first two are a **cold** cache — a fresh worktree's entries directory is empty, so the
oracle really did spawn `pdftoppm`, `mutool` and `gs` 6705 times — and the third is **warm**, off
the entries the line above it had just written: eight spawns against nine hundred and forty-seven
answers from disk. `repeated` is zero in both regimes, which is what a floor with no clock in it
looks like when it is holding: the two runs differ by three orders of magnitude in what they cost
and by nothing at all in what the inequality says.

The zeroes also settle the one soundness worry this ADR records. Two threads missing on one key at
the same instant would show here, and over 7671 spawns through three rayon walks it did not happen
once — so the floor is at its tightest setting, where **any** repeat at all fails it.

## Consequences

- `pdfref::Runs`, `Cache::runs`, `Cache::repeated_keys`, `ExtractionCache::runs` and
  `ExtractionCache::repeated_keys` are public, as instruments, for the reason `Vfs::questions` is.
- `cache::write_entry` and `extract::write_entry` answer whether they stored. Every early return in
  them is a program that will be run again for that page, and a caller that could not tell those
  from a stored entry would be holding a floor no defect can break.
- `Cache::at`/`Cache::disabled` and their extraction twins are one constructor with one field
  between them, which they were not.
- Three of `doc/todo/02` §2's most expensive lines now carry a cost floor with no clock in it. The
  other four are ADR 0899's subject, and three of them are not what the survey costed them at.
