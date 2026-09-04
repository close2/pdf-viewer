# 0894 — A cost floor that counts, because a clock here is a lottery

Session 927. Status: **accepted**. The first of this round's two records: the instrument and the
floors built on it. [ADR 0895](0895-what-the-counter-found-and-the-walks-it-does-not-reach-yet.md)
is what it found on its first run and which of the other walks a count fits.

## Context

`doc/todo/58` §5 has said since session 923 that the sharpest thing missing from `pdf-vfs` is a
perf floor, and the sentence that makes it sharp is the one about what its absence cost: **a
hundredfold cost regression lived in this crate for four sessions with the entire gate sequence
green.** Every question about a path under `images/NNNN/` re-ran the extraction that had produced
it, because that is how the *name* was validated (ADR 0886). Two corpus walks passed. A counter of
what the tree had produced — `Vfs::generated` — read **1** the whole time and its own sentence
stayed true, because the bytes really had been produced once. What found it was a wall clock in an
example, run by hand.

That is trap 33, and its general form is the thing this round was given: **this project gates
correctness thoroughly and cost almost not at all.** What existed before this round is the
transform suite's floor in pages a second (RFC 0002 §12) and session 922's five launch figures with
bands (ADR 0884). The eight corpus walks, the oracle and the censuses each print a wall clock that
nothing compares to anything.

## The problem with the obvious instrument

A wall clock over a corpus on this machine is the worst signal available, and session 922 proved
why rather than asserting it: this processor has **two classes of core** — four Zen 5 at 5.16 GHz
and eight Zen 5c at 3.29 — so the same fixed serial work measures 0.75 ms in one process and
1.50 ms in another at a load average under two. Unpinned, over twenty-eight runs, the spread was
100 % to 400 %. `doc/todo/02` §2 records **three** false failures from wall-clock gates, and the
worst of them read as a defect in the thing being measured rather than as contention.

ADR 0884 answered that for the launch path, at the cost of a construction with five parts: nine
fresh processes, `taskset` onto a derived core list, two calibration probes per figure taken in the
child beside the sample, a `NOT JUDGED` mode, and bands derived from forty-four runs. That is what
a duration costs to make believable here, and it is worth it for five figures nothing else can
state.

**It is the wrong shape for a corpus walk**, and not only because of the cost: a walk of a thousand
documents through a rayon pool cannot be pinned to one core class without measuring something else,
and it runs beside neighbours by design.

## Decision

**Where a cost property can be *counted*, it is counted, and this round counts questions put to the
worker.** A count does not care which core the scheduler chose, how many rounds are building, or
what the load average was. It is not an approximation of the duration — it is the thing the
duration was standing in for.

### 1. The counted quantity is `Worker::ask`, because that is the expensive call

Trap 33's rule is that a property about how often something runs has to count *that thing running*.
In this crate every question that requires looking at a PDF crosses one seam — RFC 0003 §6's
`Query`/`Answer` — and every one of them goes through `Consenting`, the wrapper that already holds
`CLAUDE.md` principle 3's consent for exactly the same reason: "there are fifteen of them and no
list of which ones matter". So the counter is there, and no call site can forget it.

- `Query::subject` says what a question is *about*: `page/7`, `render/7@150`, `images/7`,
  `attachment/NAME`, `metadata`. It is `None` for the five write verbs, because two `Attach`es are
  two files rather than one question asked twice, and for `Consult`/`Consented`, because two round
  trips per operation is ADR 0874's design rather than a cost.
- `Vfs::questions` answers `Questions { asked, repeated }` for the generation being served —
  per generation, because a worker is made per generation and a question about a document that has
  been replaced is a new question (RFC 0003 §5.4). It asks the worker nothing and builds no
  generation.
- `Cache::forgotten`, reached as `Vfs::forgotten`, counts what the cache **stopped holding** within
  a generation: an entry evicted to make room, or one larger than the whole budget and therefore
  stored nowhere (ADR 0887 §1 is why that refusal is kept). `Cache::retain`'s drops are deliberately
  not counted — they belong to a generation the document no longer has.

### 2. The floor is an inequality, and it is exact rather than banded

> **A generator runs once per subject per generation, and the only thing that may make it run
> again is the cache having stopped holding what it produced.**

`repeated ≤ forgotten`, per document. It is sound in the direction that matters: between two
questions about one subject there must have been a removal of that subject's entry, so every honest
repeat has a forgetting behind it and the ceiling can only be too generous. A repeat *above* it is
work done to answer a question rather than to produce bytes — which is precisely the class of defect
`Vfs::generated` is blind to, and precisely where an accidental quadratic hides, because it looks
like nothing at all: no allocation, no output, no error.

There is no band, no check file, no calibration probe and no `NOT JUDGED` mode, because there is no
clock. **A neighbouring round's load cannot move either side of the inequality by one.** That is
the whole argument for preferring a count, and it is why this floor is one a round will not switch
off.

### 3. It is enforced at two levels, and the cheaper one runs every round

- `tests/a_face.rs::the_whole_layout_walked_twice_asks_no_generator_twice` walks a document's whole
  tree — every directory listed, every file `stat`ed and read — and then does all of it again, on
  three documents. `cargo nextest run --workspace` runs it, so it is in the four lines
  `doc/todo/02` §2 calls the core and **every round runs it**, whatever the round is about. Under
  ten seconds.
- `tests/read_corpus.rs` carries the same inequality per document over its whole population, which
  is where documents too large for the byte budget make `forgotten` non-zero and where a
  pathological document meets the floor at all. That line runs when `pdf-vfs` changes and every
  fifth round.

The unit test asserts one thing more, and it is the reason to believe the other two: **the crate's
own counter is checked against an independent count taken one level lower**, by a decorator over
the transport that records every subject the worker was actually asked about. A counter checked
only against itself is how trap 33 happened in the first place.

### 4. Checked against the defect, twice, and the second one moved the population

Trap 13: a floor is not believed until it has been run against the defect it is for.

- **ADR 0886's own defect, restored** (`locate_in` validating an image's name by re-running the
  extraction): `a_directorys_names_are_asked_of_the_extraction_once` fails **3 against 1**, and the
  whole-layout floor fails **24 of 97 questions repeated**, naming the five pages —
  `["images/63", "images/60", "images/51", "images/36", "images/35"]`.
- **This round's own finding, restored** (§14.3.2's stream fetched to validate the name
  `meta/xmp.xml`; ADR 0895 §1): the metadata test fails **11 against 1** and the whole-layout floor
  fails **7 of 37 questions repeated**, naming `["metadata"]`.

**The first of those two proofs failed the first time it was run, and that is the finding worth
keeping.** With ADR 0886's defect restored the whole-layout floor **passed**, because its two
documents place no image between them and the walk therefore never entered `images/NNNN/`. A floor
whose population cannot reach the defect is trap 25 with a counter on it — a clean answer to a
question nobody asked. The third document (`/images` of the seventy-two-page guide) is in that test
because of it, and the comment there says so.

### 5. What it reads, and the same numbers under two loads

Over the whole population, on the run that took this round's gate sequence:

| | `read_corpus` | `write_corpus` |
|---|---|---|
| questions put to the workers | 13 793 | 16 217 |
| about a subject already answered | **0** | **0** |
| outputs the caches forgot | 0 | 0 |
| asked again after a refusal | 51 | — |
| documents | 1132 | the pdf.js corpus, five verb mounts each |

**And the A/B that makes the case for a count came for free.** The read walk was run twice on the
same tree: once beside a neighbour's gate sequence and once inside this round's own, and the wall
clock was **200.3 s and 109.9 s** — a factor of 1.8 — while the questions were **13 793 both
times**, with every column above identical. A duration gate would have needed a band wide enough
to hold both; the count needed nothing. That is what "a neighbour's load cannot move it" means, and
it is measured rather than argued.

The 51 in the last column are ADR 0895 §2's shortfall and not repeats: a refusal produces no bytes,
so there is nothing to cache and the next question runs the generator again.

## Consequences

- Two public items on this crate that a face can also use: `Vfs::questions` and `Vfs::forgotten`.
  They are instruments rather than plumbing, and they are public for the same reason
  `Vfs::generated` is.
- **`Vfs::generated` keeps its job and loses its claim.** It is still the right instrument for ADR
  0865 §3's size notes — "a second `stat` produces nothing" — and `tests/read_corpus.rs` still
  asserts on it. What it is no longer is the answer to "did this run twice".
- A defect found on the counter's first run, in a row nobody had looked at since session 899:
  ADR 0895 §1.
- The other seven walks are unfloored and ADR 0895 §3 says what each would need. This round floored
  the one whose absence had been paid for.
