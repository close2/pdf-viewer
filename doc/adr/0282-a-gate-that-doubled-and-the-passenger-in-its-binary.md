# 0282 — A gate that doubled, and the passenger sharing its binary

Status: accepted, four-hundred-and-forty-seventh session.

## The question

The project owner asked whether a new quorra release fixes the performance regression the
four-hundred-and-forty-fifth session found: three of `doc/todo/02` §2's ten gate timings roughly
doubled between the three-hundred-and-ninety-eighth session and that one.

| gate | 398 | 445 |
|---|---:|---:|
| oracle, 1794 pages | 51.5 s | 102.0 s |
| quorra vs the CPU oracle, 957 pages | 25.1 s | 39.0 s |
| corpus, 974 documents | 3.2 s | 5.0 s |

Principle 2 decides how to answer it, and `doc/todo/43` had already written down the instrument:
run the sequence at both ends and bisect between them, so that a commit is named rather than a
suspicion.

## 1. Two of the three cannot involve quorra, and this is structural

`crates/pdf-model` has **no quorra dependency, direct or transitive**, and both the corpus and the
oracle gates are `crates/pdf-model/tests/`. That is not a reading of the manifest but the output of
a program:

```
$ cargo tree -p pdf-model --edges normal,dev | grep -c quorra
0
```

The quorra gate is the third one and it is `render-quorra`'s, which compares quorra *against*
`render-cpu` and therefore runs both. So a quorra release can move exactly one of the three
numbers, whatever it does. **The owner's question could not have been answered "yes" by any
release**, and the useful part of the answer had to come from somewhere else.

## 2. Only one of the three moved at all, and the other two were the day rather than the tree

Session 398's own commit is `244b86a`. Checked out beside `351bfed` on one idle-ish machine, in one
sitting, with the reference cache at the same 99.7% hit rate at both ends:

| gate | 398 reported | 445 reported | `244b86a` today | `351bfed` today |
|---|---:|---:|---|---|
| oracle | 51.5 s | 102.0 s | **50.3, 52.0** | **94.4, 96.1, 98.4** |
| quorra | 25.1 s | 39.0 s | **34.6, 34.5** | **34.1, 35.0** |
| corpus | 3.2 s | 5.0 s | **4.6, 4.6, 4.7** | **3.9, 4.0, 4.0** |

**The corpus gate is faster at HEAD than at 398, and the quorra gate is level.** Neither 398's
figure nor 445's reproduces for those two; what reproduces is the *difference between them being
zero*. The oracle is the one real finding, and it reproduces to the second at both ends.

That kills the hypothesis `doc/todo/43` handed over — that §11.4.7's page group has been drawn as
two rasters since ADR 0262 and quorra's pair since ADR 0275. The file already doubted it on
arithmetic (7 documents of 974, nowhere near a factor of two); it is now excluded by measurement,
because the gates those ADRs touched are the two that did not move.

**The corpus gate was therefore the wrong bisect probe**, and `doc/todo/43` named it as the right
one because it is cheap. Cheap and *sensitive to the thing being looked for* are different
properties, and a probe that shows the effect backwards is worse than an expensive one. The oracle
at 50–100 s a probe is about two minutes a bisect step, which over 53 commits is six steps and a
quarter of an hour — affordable, and it was simply not checked whether the cheap probe moved.

## 3. The bisect names `92579c2`

`git bisect` over `244b86a..351bfed`, 53 commits, probe = the oracle's own `1794 pages in ...s`
line, "good" below 70 s — a threshold with 30 s of clearance on either side of a ~2 s spread. Five
probes:

```
2e0541d  82.3s   bad
265b800  94.6s   bad
db1d7ef  50.1s   good
92579c2  87.2s   bad     <- first bad
744472e  46.5s   good    <- its parent
```

`744472e` **46.5 s** against its own child `92579c2` **87.2 s**. One commit, 40.7 s, twenty times
the spread.

`92579c2` is the four-hundred-and-seventh session, ADR 0243, and its own commit message says *"No
rendering code, no pixel, no verdict"* — which is true. It added a second test to
`crates/pdf-model/tests/oracle.rs`, `the_fixed_bounds_against_the_references_own_spread`, which
re-derives the eight fixed tolerance bounds from 9898 reference pairs over the same 1794 pages.

**That round saw the symptom and read it as bookkeeping.** Its history row ends: "Only the
*skipped*-test count moved, 9 → 10." The skipped count moving is precisely the observation that a
new `#[ignore]` had appeared in a binary the round invokes with `--ignored`, and it went down as a
number rather than as a question.

## 4. What it actually is: `--ignored` runs the whole binary

The added test carries `#[ignore]` and a doc comment headed *"Why this is a separate run and not
part of the gate"*, ending:

> A number in this table is evidence for changing a bound; it is not itself a gate, because a bound
> that moved whenever a reference renderer was upgraded would be the curve-fitting `CLAUDE.md`
> forbids wearing a schedule.

It is, in its own words, not a gate. And `doc/todo/02` §2 invokes the oracle as

```sh
cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture
```

`--ignored` is not a filter on one test; it un-ignores **every** ignored test in the binary. So the
derivation ran beside the gate, in the same process, on the same 24 cores, both of them walking the
corpus under `rayon`. `doc/todo/02` already knew this mechanism and had written it down for the
*other* binary where it applies — "[t]he `text_extraction` line is two gates and gains no line" —
and there it is correct, because both of `text_extraction`'s tests are gates. Nobody checked the
oracle's for the case where one of the two is not.

The arithmetic closes. Same session, same machine, one after another:

| | |
|---|---:|
| the gate test alone (`--exact`) | **47.9, 47.8 s** |
| the derivation alone (`--exact`) | **40.2 s** |
| both, which is what the round ran | **94.4, 96.1, 98.4 s** |

47.9 + 40.2 = 88.1 against 94–98 measured, and the excess is the contention between them.

**And the second cost is worse than the first**, because it is silent. The gate's report is built
from per-page `Instant` spans, so a passenger saturating the machine inflates them too. The same
run printed `22060_A1_01_Plans.pdf page 1` at **93.2 s** where the gate alone puts it at 18.0–19.7 s
and `PDFVIEWER_ORACLE_ONLY` on its own at 5.4–6.7 s; the `processor time: ... ours` row read
**281 s** against **146–147 s** for the same work. `doc/todo/43` quoted that inflated row as
corroborating evidence — "it prints 271 s ours against 90 s in the three reference renderers" — so
the instrument had already misled one round about the thing it was measuring.

## 5. The decision: the guard goes in the test, not in the invocation

`the_fixed_bounds_against_the_references_own_spread` now returns early, printing why, unless
`PDFVIEWER_ORACLE_SPREAD` is set.

The alternative was to change `doc/todo/02` §2's command to name the gate test with `--exact`. It
is rejected for one reason: **an invocation can be copied without its guard, and a test cannot be
run without itself.** `tools/state.sh`, `doc/todo/02` §2, CI and anybody typing the obvious command
all get the gate and only the gate, with no line to keep in step. That the same rule was already
learned one file over — `doc/todo/02` §6's "the instrument that says a change happened is not the
change" — is the shape of the argument rather than a coincidence.

An `#[ignore]` attribute cannot express this. It says *run explicitly*, and `--ignored` overrides it
for the binary as a whole, so a test that must not run inside a gate has no attribute available to
say so. The environment variable is a second mechanism doing what the first cannot, and it is
documented as that rather than left to look like belt and braces.

**Nothing is lost.** The derivation is one environment variable away, its output is unchanged, and
it was never a verdict: it asserts only that it had a population to print. What is gained is
measured, interleaved, three samples each, alternating so that a drift in machine load cannot be
read as an effect:

| | |
|---|---|
| oracle gate **with** the passenger | 47.0, 46.3, 55.5 s |
| oracle gate **without** | 24.4, 25.1, 30.2 s |

The bands do not overlap and the gap is about twice the worse of the two spreads. The whole of
`doc/todo/02` §2, run end to end through `tools/state.sh`, is **2 m 37 s**.

**An independent check that the passenger was the whole of it**: ADR 0222 measured this gate at
**24.5–25.7 s** in the three-hundred-and-eighty-fifth session, before `92579c2` existed.
`tools/state.sh` now prints **25.3 s**. Sixty-two rounds of rendering work later, the oracle costs
what it cost, which is the strongest available evidence that nothing this tree draws got slower.

**This is not a correctness fix being undone to buy back seconds**, which is the trade `CLAUDE.md`
would require written down. No verdict, no bound and no pixel moves: the oracle's 905 / 68 / 786 /
1 / 2 / 14 / 18 and every other gate's counts are identical before and after. What changed is which
of two tests a command runs.

## 6. The quorra release, measured separately, which is the answer to the question asked

Upstream moved twice while the round ran: `595d8c87` — the release the question was about, a
murmur-style avalanche in the key hasher's `finish`, fixing a regression **quorra** introduced in
`89d7dd77`, the revision we were pinned to — and then `c1f6e2f4`, which chooses the GPU coverage
lane per command by comparing what its coverage would cost against what its triangles would.

All three were run through `render-quorra`'s corpus gate in an **A/B/A alternation**, six samples
each, rebuilt between arms, because the gate's own wall clock had varied 23.9–39.2 s at a *fixed*
pin earlier the same day and cannot resolve anything read off unalternated runs.

| pin | gate wall clock | serial rasterisation through quorra | verdicts |
|---|---|---|---|
| `89d7dd77` (was) | 26.3–27.1 s | 6.17–6.35 s | 917 / 35 / 5 / 17 |
| `595d8c87` | 26.2–27.3 s | 6.12–6.44 s | 917 / 35 / 5 / 17 |
| `c1f6e2f4` (now) | 26.9–29.0 s | 6.26–6.78 s | 917 / 35 / 5 / 17 |

**Every band overlaps every other band.** The difference in means between the first two arms is
0.02 s of wall clock on a spread of about 0.9 — so the honest statement is the one
`doc/QUORRA_FEEDBACK.md` has already had to make once about this gate: **it cannot resolve a change
at quorra's `encode` grain**, and this is the second time that has been established rather than
assumed. Upstream's own numbers for the hasher fix are 468 → 397 ms on one page and 46 → 43 on
another; against 957 pages summing to 6.3 s, that is under the noise by construction.

The pin is moved to `c1f6e2f4` rather than to the `595d8c87` the question named, because it is the
branch head, because the pin has tracked the head at every previous move, and because every gate is
green on it — including `render-quorra::real_pages`, which is the one that gates the *window's*
resolution and the glyph-phase quantum, and which is where a lane-selection change would show if it
showed anywhere.

## Consequences

- **The three-gate regression is one gate**, and that one is not a rendering slowdown. The corpus
  and quorra figures in `doc/todo/43`'s handover table are a day's difference between two machines'
  states, not a change in this tree. `doc/todo/43` is corrected accordingly.
- **A gate binary may hold at most one kind of test**, or the ones that are not gates must decline
  by themselves. The oracle binary is the only one in this tree that had the problem —
  checked by `cargo test --test <name> -- --list --ignored` over all seven gate binaries, which is
  how the claim is made rather than assumed. `xmp` and `text_extraction` each hold two, and in both
  cases both are gates.
- **A per-page span printed by a gate is only as good as what else is in its process.** The
  oracle's `processor time` and `slowest pages` rows were wrong by a factor of two for
  **thirty-nine rounds**, the four-hundred-and-seventh to the four-hundred-and-forty-sixth, and
  nothing could see it because there was no second reading to disagree with.
- `cargo deny` clean on all four checks with the pin moved; both cross-target checks build under
  `-D warnings`; no ledger row is owed, because the commit the bisect named implements no normative
  requirement — it is the oracle's tolerance instrument — and the conformance gate reports 0 cited
  clauses owing a review either way.
