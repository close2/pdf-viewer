# ADR 0666 — What the fat link is for, measured three hundred rounds after the question

Status: accepted, 2026-08-25. Session 752. Closes `doc/todo/43` §1 by measurement: the shipped
`[profile.release]` keeps `lto = "fat"` and `codegen-units = 1`, and both settings are shown to be
load-bearing on different paths. Corrects three stale figures for what the link costs.

## The question, and why it was cheap

`doc/todo/43` §1 has stood since ADR 0222:

> It stays until somebody measures what it is *for*. The question nobody has asked is what
> `lto = "thin"` costs the launch path and the page-turn latency; if the answer is "nothing
> measurable", `[profile.release]` could follow the gates and a round would lose another 60 s.
> That is a **measurement task, not a change**.

That was written in the three-hundred-and-eighty-fifth session and restated as owed in the
four-hundred-and-forty-seventh and the five-hundred-and-ninety-third. Nobody ran it.

**The reason it should have been run long ago is that the two arms already exist.**
`[profile.gates]` is declared `inherits = "release"` with `lto = "thin"` and
`codegen-units = 16` and nothing else — same `opt-level`, same `panic = "abort"`, no package
overrides anywhere in the workspace. So `release` against `gates` *is* the A/B this file asked
for, and it needs no edit to `Cargo.toml` at all. Two further profiles were added for the round
and removed before the commit, to separate the two settings:

| arm | `lto` | `codegen-units` |
|---|---|---|
| `release` | `fat` | 1 |
| `gates` | `thin` | 16 |
| `thin1` | `thin` | 1 |
| `fat16` | `fat` | 16 |

## The instrument, and why it is not a clock

`doc/todo/43`'s own warning binds here — "every number above was taken on a machine that may or
may not have had a neighbour on it, and none of them says which… prefer a counter to a clock where
one exists." Three parallel rounds were building throughout, with the load average between 1.7 and
27. So the program half of the question is answered with callgrind, over the three counters
`doc/verify.md` already names: `callgrind_open` (§7.5's trailer and cross-reference table, ten
opens of ISO 32000-2), `callgrind_interpret` (page 101 fifty times) and `callgrind_rasterise`.

**Instruction counts here have no spread whatever.** Every arm was run three times and every run
of a given binary agreed with the others *to the instruction* — 763,278,781 each time for
`release`'s `callgrind_open`, and so on for all four arms. There is therefore no question of
reading a difference smaller than the noise, because there is no noise.

| | `callgrind_open` | `callgrind_interpret` | `callgrind_rasterise` |
|---|---:|---:|---:|
| `release` (`fat`, 1) | 763,278,781 | 1,294,054,067 | 5,448,437,924 |
| `gates` (`thin`, 16) | 857,138,564 (**+12.30%**) | 1,315,415,619 (+1.65%) | 5,670,107,190 (+4.07%) |
| `thin1` (`thin`, 1) | 794,314,648 (+4.06%) | 1,317,897,411 (+1.84%) | 5,484,863,774 (+0.67%) |
| `fat16` (`fat`, 16) | 845,116,860 (+10.72%) | 1,293,978,158 (−0.01%) | 5,448,270,197 (−0.00%) |

Binary sizes, same order: 8.08 / 10.31 / 8.51 / 8.25 MB for `callgrind_interpret` — the shipped
profile is about a **quarter** smaller than the gates one.

## What the table says, which is more than a verdict

**The two settings buy different things, and neither alone reproduces the pair.**

- **Cross-crate inlining is what the interpreter and the rasteriser gain.** `fat16` is `release` to
  within 0.01% on both, so `codegen-units` is irrelevant once the link is fat. Symmetrically,
  `thin1` and `gates` are close to each other and both worse.
- **The single code generation unit is what §7.5's cross-reference parse gains.** `fat16` is
  +10.72% there — nearly the whole of `gates`'s penalty — while `thin1` is only +4.06%. On this
  path the partitioning matters more than the link does.
- So the obvious compromise, `thin` at `codegen-units = 1`, is still **+4.06% on the launch path's
  largest step**, and there is no cheaper combination that reproduces what ships.

Reading the per-function attribution says which boundaries move, though the rows are not
comparable across arms *because the inlining boundary is exactly what changed*: under the two fat
arms `filter::apply_predictor` is inlined into `decode_with_parms_reported` and under the two thin
ones it stands alone, and `xref::read` merges into `xref::read_section` or does not. That
observation is what ADR 0667 is about.

## What it costs, re-derived — three figures had decayed

`cargo build --timings`, §5's six binaries, after touching one file in `pdf-model`:

| | wall clock | `viewer-ui`'s unit | CPU across all units |
|---|---:|---:|---:|
| `release` | 94.5 s | **93.59 s** | 241.0 s |
| `gates` | 50.4 s | 49.48 s | 178.2 s |

**`viewer-ui`'s link is the whole critical path.** It starts 0.8 s into the build and ends 0.1 s
before it does; `viewer-gtk` (46.9 s), `viewer-qt` (45.2), `viewer-confined` (34.8) and
`pdf-retrieve` (19.5) all run underneath it and finish 45 s earlier. So the saving on offer is not
the sum of the links, it is whatever that one unit falls to.

Three written-down numbers for this were wrong, all in the same direction:

- `doc/todo/43` §1: "66.4 s of a 76.9 s `cargo build --release --bin pdf-viewer`" → **93.59 s**.
- `doc/todo/43`'s table: "§5's three release binaries, 79.3 s" → **94.5 s**, and §5 names six
  binaries and a shared library now rather than three.
- `Cargo.toml`'s own comment above `[profile.release]`: "78 s to relink `pdf-viewer`" → **93.59 s**.

The `gates` arm was measured at load average 27 and the `release` arm at 4 to 7, which biases
against the arm that won on wall clock, so the 44 s gap is a floor.

## The decision

**`[profile.release]` keeps `lto = "fat"` and `codegen-units = 1`.**

The trade actually on offer is about 44 s of wall clock against 12.30% of every `Document::open`,
4.07% of a rasterisation, 1.65% of an interpretation and a quarter of the binary size.
`CLAUDE.md` principle 2 makes startup a first-class requirement and names page-turn latency a
gate; this is a regression in both, bought with build time.

**And the prize had been priced against a cadence that no longer exists.** "A round would lose
another 60 s" was true when `doc/todo/02` §5 ran every round. ADR 0428 made it run every fifth
round and before any measurement, so the saving is ~44 s of a fifth of a round — roughly 9 s a
round — against a permanent cost to the program. That is the clearest form of the answer: the
question was framed when the denominator was five times larger.

## What this does not claim

- **It is not a launch-timeline measurement**, which is the other half `doc/todo/43` §1 named.
  That is a wall clock under `Xvfb` on a machine with three other rounds on it, and `doc/todo/42`
  already records a previous attempt ruined the same way. What can be said from the counters: at
  the ~8 M instructions per millisecond ADR 0180 established on this path, +12.30% of an open is
  roughly **+1.2 ms** on a ~110 ms launch — real, small, and dwarfed by the rasterisation figure,
  where +4.07% of 5.45 G is ~222 M instructions on a single page draw.
- **It says nothing about CI**, which `doc/todo/43` §4 already lists as unmeasured and which does
  not build the release profile at all.
- **It is a statement about this tree at this revision.** It is the same kind of claim as the one
  it corrects, and it will decay the same way — the honest thing it leaves behind is not the table
  but the fact that the A/B costs one afternoon and needs no edit, so the next round to doubt it
  can re-run it.
